//! TOML persistence for the execution graph.
//!
//! Implements `docs/design/execution-graph/persistence-and-concurrency.md`.
//! The machine fact source is `docs/plan/execution-graph.toml`; the generated
//! view is `docs/plan/execution-graph.md` and is never a mutation target.
//!
//! Write sequence for [`TomlStore::save_with_revision`]:
//! 1. acquire the exclusive lock on `.mine/locks/execution-graph.lock`;
//! 2. reload the on-disk TOML (state may have changed while waiting);
//! 3. recheck `expected_revision` against the reloaded revision;
//! 4. run a caller-provided mutation that produces the new workspace;
//! 5. write the TOML atomically;
//! 6. render the Markdown view from the committed TOML;
//! 7. release the lock.
//!
//! If the TOML write succeeds but Markdown rendering fails, the TOML remains
//! the fact source and a partial-success error is returned advising
//! `mine graph render` repair.

use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::domain::error::{MineError, MineResult};
use crate::domain::graph::PlanWorkspace;
use crate::domain::validation;

use super::atomic_write;
use super::file_lock;

/// Default lock timeout in milliseconds.
const DEFAULT_LOCK_TIMEOUT_MS: u64 = 5000;

/// The execution-graph lock file name.
pub const GRAPH_LOCK_NAME: &str = "execution-graph.lock";

/// Persistence adapter for the execution-graph TOML and generated Markdown.
pub struct TomlStore {
    /// Repository root.
    repo_root: PathBuf,
    /// Path to `docs/plan/execution-graph.toml`.
    toml_path: PathBuf,
    /// Path to `docs/plan/execution-graph.md`.
    md_path: PathBuf,
    /// Path to `.mine/locks/execution-graph.lock`.
    lock_path: PathBuf,
    /// Lock wait timeout.
    lock_timeout: Duration,
}

impl TomlStore {
    /// Creates a store rooted at `repo_root` using conventional paths.
    #[must_use]
    pub fn new(repo_root: &Path) -> Self {
        let toml_path = repo_root
            .join("docs")
            .join("plan")
            .join("execution-graph.toml");
        let md_path = repo_root
            .join("docs")
            .join("plan")
            .join("execution-graph.md");
        let lock_path = repo_root.join(".mine").join("locks").join(GRAPH_LOCK_NAME);
        Self {
            repo_root: repo_root.to_path_buf(),
            toml_path,
            md_path,
            lock_path,
            lock_timeout: Duration::from_millis(DEFAULT_LOCK_TIMEOUT_MS),
        }
    }

    /// Returns a store with a custom lock timeout (mainly for tests).
    #[must_use]
    pub fn with_lock_timeout(mut self, timeout: Duration) -> Self {
        self.lock_timeout = timeout;
        self
    }

    /// Returns the TOML fact-source path.
    #[must_use]
    pub fn toml_path(&self) -> &Path {
        &self.toml_path
    }

    /// Returns the generated Markdown view path.
    #[must_use]
    pub fn md_path(&self) -> &Path {
        &self.md_path
    }

    /// Loads the workspace from the TOML fact source.
    ///
    /// # Errors
    /// - [`MineError::GraphNotInitialized`] if the file is absent.
    /// - [`MineError::GraphInvalid`] if the TOML is unparseable or fails
    ///   structural validation.
    pub fn load(&self) -> MineResult<PlanWorkspace> {
        if !self.toml_path.exists() {
            return Err(MineError::GraphNotInitialized {
                path: self.toml_path.clone(),
            });
        }
        let content = std::fs::read_to_string(&self.toml_path)?;
        let ws: PlanWorkspace = toml::from_str(&content).map_err(|e| MineError::GraphInvalid {
            detail: format!("could not parse execution-graph TOML: {e}"),
        })?;
        validation::validate(&ws)?;
        Ok(ws)
    }

    /// Persists a workspace under the lock with revision checking.
    ///
    /// `expected_revision` is checked against the on-disk revision *after* the
    /// lock is acquired and the file reloaded. The caller's `mutate` closure
    /// receives the reloaded workspace and returns the new one to persist; it
    /// must set `revision = expected_revision + 1` and update timestamps.
    ///
    /// # Errors
    /// - [`MineError::RevisionConflict`] if the on-disk revision does not match
    ///   `expected_revision`.
    /// - [`MineError::LockTimeout`] if the lock cannot be acquired.
    /// - [`MineError::GraphInvalid`] if the mutated workspace fails validation.
    /// - Partial success ([`MineError::GraphInvalid`] with a render-repair hint)
    ///   if the TOML writes but Markdown rendering fails.
    pub fn save_with_revision(
        &self,
        expected_revision: u64,
        mutate: impl FnOnce(PlanWorkspace) -> MineResult<PlanWorkspace>,
    ) -> MineResult<PlanWorkspace> {
        let _lock = file_lock::acquire_exclusive(&self.lock_path, self.lock_timeout)?;

        // Reload under the lock.
        let reloaded = if self.toml_path.exists() {
            let content = std::fs::read_to_string(&self.toml_path)?;
            toml::from_str::<PlanWorkspace>(&content).map_err(|e| MineError::GraphInvalid {
                detail: format!("could not parse execution-graph TOML: {e}"),
            })?
        } else {
            return Err(MineError::GraphNotInitialized {
                path: self.toml_path.clone(),
            });
        };

        if reloaded.revision != expected_revision {
            return Err(MineError::RevisionConflict {
                expected: expected_revision,
                actual: reloaded.revision,
            });
        }

        let new_ws = mutate(reloaded)?;
        validation::validate(&new_ws)?;

        let toml_content = toml::to_string(&new_ws).map_err(|e| MineError::GraphInvalid {
            detail: format!("could not serialize execution-graph TOML: {e}"),
        })?;
        atomic_write::write(&self.toml_path, toml_content.as_bytes())?;

        // Render the Markdown view from the committed TOML. If this fails, the
        // TOML is still the fact source.
        let reloaded_for_render = self.load()?;
        match render_markdown(&reloaded_for_render) {
            Ok(md) => {
                atomic_write::write(&self.md_path, md.as_bytes())?;
            }
            Err(e) => {
                return Err(MineError::GraphInvalid {
                    detail: format!(
                        "TOML written but Markdown render failed (run `mine graph render` to repair): {e}"
                    ),
                });
            }
        }

        Ok(reloaded_for_render)
    }

    /// Re-renders the Markdown view from the current TOML without mutating
    /// the TOML. Used by `mine graph render` repair.
    ///
    /// # Errors
    /// Returns [`MineError::GraphInvalid`] if the TOML cannot be loaded or
    /// rendered.
    pub fn render(&self) -> MineResult<()> {
        let ws = self.load()?;
        let md = render_markdown(&ws)?;
        atomic_write::write(&self.md_path, md.as_bytes())?;
        Ok(())
    }

    /// Returns the repository root.
    #[must_use]
    pub fn repo_root(&self) -> &Path {
        &self.repo_root
    }
}

/// Renders the Markdown view for a workspace.
///
/// The view is a stable, human- and agent-readable summary. The format is
/// deterministic: the same TOML always renders to the same Markdown.
pub fn render_markdown(ws: &PlanWorkspace) -> MineResult<String> {
    use std::fmt::Write;
    let mut out = String::new();
    writeln!(
        out,
        "# Execution Graph\n\n> GENERATED VIEW. Do not edit directly. The machine fact source is `execution-graph.toml`.\n>\n> This directory is ephemeral and must be purged before stable release integration.\n\n- Workspace: `{}`\n- Stable branch: `{}`\n- Integration branch: `{}`\n- Revision: `{}`\n",
        ws.workspace_id, ws.stable_branch, ws.integration_branch, ws.revision
    )
    .map_err(|e| MineError::GraphInvalid {
        detail: format!("markdown write failed: {e}"),
    })?;

    writeln!(out, "| Plan | Title | Status | Hard predecessors |").map_err(|e| {
        MineError::GraphInvalid {
            detail: format!("markdown write failed: {e}"),
        }
    })?;
    writeln!(out, "|---|---|---|---|").map_err(|e| MineError::GraphInvalid {
        detail: format!("markdown write failed: {e}"),
    })?;
    for p in &ws.plans {
        let preds = if p.hard_predecessors.is_empty() {
            "-".to_string()
        } else {
            p.hard_predecessors.join(", ")
        };
        writeln!(out, "| {} | {} | {} | {} |", p.id, p.title, p.status, preds).map_err(|e| {
            MineError::GraphInvalid {
                detail: format!("markdown write failed: {e}"),
            }
        })?;
    }

    // Topology section.
    let order = validation::topological_sort(ws)?;
    out.push_str("\n## Topology\n\n```text\n");
    if order.is_empty() {
        out.push_str("(no plans)\n");
    } else {
        writeln!(out, "{}", order.join(" -> ")).map_err(|e| MineError::GraphInvalid {
            detail: format!("markdown write failed: {e}"),
        })?;
    }
    out.push_str("```\n");

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::graph::PlanNode;
    use crate::domain::status::PlanStatus;

    fn sample_workspace(rev: u64) -> PlanWorkspace {
        PlanWorkspace {
            schema_version: 1,
            revision: rev,
            project_id: "mine-is-not-everyones".to_string(),
            workspace_id: "test-ws".to_string(),
            stable_branch: "master".to_string(),
            integration_branch: "dev".to_string(),
            stable_baseline_commit: "abc".to_string(),
            design_root: "docs/design/index.md".to_string(),
            ephemeral_workspace: true,
            purge_before_stable_release: true,
            plans: vec![PlanNode {
                id: "01".to_string(),
                path: "docs/plan/01.md".to_string(),
                title: "First".to_string(),
                status: PlanStatus::Ready,
                hard_predecessors: vec![],
                soft_predecessors: vec![],
                design_references: vec!["docs/design/principles.md".to_string()],
                exclusive_write_paths: vec!["src/a/".to_string()],
                read_only_paths: vec![],
                reserved_shared_paths: vec![],
                implementation_report: String::new(),
                review_report: String::new(),
                implementation_commits: vec![],
                owner: String::new(),
                run_id: String::new(),
                started_at: String::new(),
                updated_at: String::new(),
                rejection_reason: String::new(),
                compensating_plan: String::new(),
            }],
        }
    }

    #[test]
    fn load_returns_not_initialized_when_absent() {
        let dir = tempfile::tempdir().unwrap();
        let store = TomlStore::new(dir.path());
        let err = store.load().unwrap_err();
        assert_eq!(err.code(), "MINE_GRAPH_NOT_INITIALIZED");
    }

    #[test]
    fn save_then_load_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let store = TomlStore::new(dir.path());
        // Seed the TOML directly (simulating an existing graph).
        let ws = sample_workspace(0);
        std::fs::create_dir_all(store.toml_path.parent().unwrap()).unwrap();
        std::fs::write(&store.toml_path, toml::to_string(&ws).unwrap()).unwrap();

        let loaded = store.load().unwrap();
        assert_eq!(loaded, ws);
        assert!(!store.md_path.exists()); // save_with_revision renders; load does not.
    }

    #[test]
    fn save_with_revision_increments_and_renders() {
        let dir = tempfile::tempdir().unwrap();
        let store = TomlStore::new(dir.path());
        let ws = sample_workspace(0);
        std::fs::create_dir_all(store.toml_path.parent().unwrap()).unwrap();
        std::fs::write(&store.toml_path, toml::to_string(&ws).unwrap()).unwrap();

        let result = store
            .save_with_revision(0, |mut w| {
                w.revision = 1;
                w.plans[0].status = PlanStatus::InProgress;
                w.plans[0].owner = "tester".to_string();
                Ok(w)
            })
            .unwrap();
        assert_eq!(result.revision, 1);
        assert_eq!(result.plans[0].status, PlanStatus::InProgress);

        // Markdown rendered with the new revision.
        let md = std::fs::read_to_string(store.md_path).unwrap();
        assert!(md.contains("Revision: `1`"));
        assert!(md.contains("IN_PROGRESS"));
    }

    #[test]
    fn save_with_revision_conflict_detected() {
        let dir = tempfile::tempdir().unwrap();
        let store = TomlStore::new(dir.path());
        let ws = sample_workspace(5);
        std::fs::create_dir_all(store.toml_path.parent().unwrap()).unwrap();
        std::fs::write(&store.toml_path, toml::to_string(&ws).unwrap()).unwrap();

        let err = store
            .save_with_revision(4, |w| {
                // caller expected revision 4 but on disk it is 5
                Ok(w)
            })
            .unwrap_err();
        assert_eq!(err.code(), "MINE_REVISION_CONFLICT");
        // TOML unchanged.
        assert_eq!(store.load().unwrap().revision, 5);
    }

    #[test]
    fn render_repair_regenerates_markdown() {
        let dir = tempfile::tempdir().unwrap();
        let store = TomlStore::new(dir.path());
        let ws = sample_workspace(7);
        std::fs::create_dir_all(store.toml_path.parent().unwrap()).unwrap();
        std::fs::write(&store.toml_path, toml::to_string(&ws).unwrap()).unwrap();
        // Stale / absent markdown.
        store.render().unwrap();
        let md = std::fs::read_to_string(store.md_path).unwrap();
        assert!(md.contains("Revision: `7`"));
    }

    #[test]
    fn render_markdown_is_deterministic() {
        let ws = sample_workspace(3);
        let a = render_markdown(&ws).unwrap();
        let b = render_markdown(&ws).unwrap();
        assert_eq!(a, b);
    }
}

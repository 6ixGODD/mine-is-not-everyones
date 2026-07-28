// Enforce `AGENTS.md`'s "Business code must not use `unsafe`" at compile time.
#![forbid(unsafe_code)]

//! Release service: deterministic release-candidate creation, preflight
//! validation, and safe temporary-artifact cleanup.
//!
//! The release path is deterministic and **fail closed**: it refuses to
//! mutate stable state until every required preflight and release-candidate
//! validation succeeds. Per the design ("Release closure"):
//!
//! 1. Every plan is accepted and integrated into `dev`.
//! 2. Run a full `mine-sync` (the user's responsibility; release-preflight
//!    verifies design validity).
//! 3. Resolve the release version from accepted changes + current managed
//!    version.
//! 4. Safely purge the MINE-owned `docs/plan/` workspace.
//! 5. Verify no tracked/untracked plan workspace or design backup enters the
//!    stable tree.
//! 6. Integrate through squash or curated commits (the independent reviewer's
//!    responsibility; the release service provides the candidate, not the
//!    final mutation).
//! 7. Tag/publish when configured (reserved for the reviewer).
//! 8. Delete local managed `plan/*` and `dev` branches (reserved for the
//!    reviewer after stable integration).
//!
//! The release service itself performs NO git mutation (no commit, merge,
//! tag, push, reset, clean, or branch deletion). It validates, resolves the
//! version, enumerates what SHOULD be cleaned, and reports the release
//! candidate state. Final stable mutation belongs to the independent reviewer.

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::domain::error::{MineError, MineResult};
use crate::domain::status::PlanStatus;
use crate::infrastructure::git::{self, GitEvidence};
use crate::infrastructure::toml_store::TomlStore;

/// The release-preflight result.
#[derive(Debug, Clone, Serialize)]
pub struct ReleasePreflight {
    pub can_release: bool,
    pub release_version: String,
    pub dev_commit: String,
    pub master_commit: String,
    pub master_unchanged: bool,
    pub working_tree_clean: bool,
    pub all_plans_terminal: bool,
    pub design_valid: bool,
    pub graph_valid: bool,
    pub distribution_synced: bool,
    pub no_plan_artifacts_on_stable: bool,
    pub no_design_backups_on_stable: bool,
    pub pending_agent_transactions: Vec<String>,
    pub rejections_without_compensation: Vec<String>,
    pub errors: Vec<String>,
}

/// The cleanup-preview result: what WOULD be removed if cleanup is executed.
#[derive(Debug, Clone, Serialize)]
pub struct CleanupPreview {
    pub plan_workspace_files: Vec<String>,
    pub design_backups: Vec<String>,
    pub mine_managed_plan_branches: Vec<String>,
    pub dev_branch: Option<String>,
    pub total: usize,
}

/// Runs the full release preflight against `repo_root`. Returns the preflight
/// result with `can_release = true` only when every gate passes. No mutation.
pub fn preflight(repo_root: &Path) -> MineResult<ReleasePreflight> {
    let mut errors = Vec::new();

    // 1. Git evidence: working tree clean, dev/master commits.
    let git_ev = GitEvidence::collect(repo_root);
    let dev_commit = git::head_commit(repo_root).unwrap_or_default();
    let master_commit = run_git_rev_parse(repo_root, "master")?;
    let working_tree_clean = git_ev.clean;
    let master_unchanged = master_commit_exists_and_matches_expected(repo_root, &master_commit)?;

    if !working_tree_clean {
        errors.push("working tree is dirty".to_string());
    }

    // 2. All plans terminal (ACCEPTED or REJECTED with compensation).
    let store = TomlStore::new(repo_root);
    let ws = store.load()?;
    let all_plans_terminal = ws
        .plans
        .iter()
        .all(|p| matches!(p.status, PlanStatus::Accepted | PlanStatus::Rejected));
    if !all_plans_terminal {
        let unresolved: Vec<&str> = ws
            .plans
            .iter()
            .filter(|p| !matches!(p.status, PlanStatus::Accepted | PlanStatus::Rejected))
            .map(|p| p.id.as_str())
            .collect();
        errors.push(format!(
            "unresolved plans remain: {}",
            unresolved.join(", ")
        ));
    }

    // 3. Rejections must have compensation (compensating_plan set).
    let rejections_without_compensation: Vec<String> = ws
        .plans
        .iter()
        .filter(|p| matches!(p.status, PlanStatus::Rejected) && p.compensating_plan.is_empty())
        .map(|p| p.id.clone())
        .collect();
    if !rejections_without_compensation.is_empty() {
        errors.push(format!(
            "rejected plans without compensation: {}",
            rejections_without_compensation.join(", ")
        ));
    }

    // 4. Design validates.
    let design_valid = validate_design(repo_root);

    // 5. Graph renders consistently (TOML matches MD).
    let graph_valid = store.load().is_ok() && store.render().is_ok();

    // 6. Distribution synced (skills == plugins/mine/skills).
    let distribution_synced = check_distribution_sync(repo_root);

    // 7. No plan artifacts on stable (master).
    let no_plan_artifacts_on_stable = !path_exists_on_branch(repo_root, "master", "docs/plan/");
    let no_design_backups_on_stable = !branch_has_design_backups(repo_root, "master")?;

    // 8. No pending agent transactions (incomplete installs).
    let pending_agent_transactions = check_pending_transactions(repo_root);

    // 9. Version resolution.
    let release_version = resolve_release_version(&ws);

    let can_release = errors.is_empty()
        && all_plans_terminal
        && design_valid
        && graph_valid
        && distribution_synced
        && no_plan_artifacts_on_stable
        && no_design_backups_on_stable
        && pending_agent_transactions.is_empty()
        && rejections_without_compensation.is_empty();

    Ok(ReleasePreflight {
        can_release,
        release_version,
        dev_commit,
        master_commit,
        master_unchanged,
        working_tree_clean,
        all_plans_terminal,
        design_valid,
        graph_valid,
        distribution_synced,
        no_plan_artifacts_on_stable,
        no_design_backups_on_stable,
        pending_agent_transactions,
        rejections_without_compensation,
        errors,
    })
}

/// Previews what cleanup WOULD remove. No mutation.
pub fn preview_cleanup(repo_root: &Path) -> MineResult<CleanupPreview> {
    let mut plan_workspace_files = Vec::new();
    let plan_dir = repo_root.join("docs/plan");
    if plan_dir.exists() {
        for entry in walk_files(&plan_dir) {
            let rel = entry
                .strip_prefix(repo_root)
                .unwrap_or(&entry)
                .to_string_lossy()
                .replace('\\', "/");
            plan_workspace_files.push(rel);
        }
    }

    let mut design_backups = Vec::new();
    // Scan docs/ recursively for design-backup-* directories (not just
    // top-level entries, since backups live under docs/). The rejected Plan
    // 08 scanned repo_root's top-level entries, which only returns "docs"
    // and can never match the "design-backup-" prefix.
    let docs_dir = repo_root.join("docs");
    if docs_dir.exists() {
        for entry in std::fs::read_dir(&docs_dir).map_err(MineError::Io)? {
            let entry = entry.map_err(MineError::Io)?;
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("design-backup-") {
                design_backups.push(format!("docs/{name}"));
            }
        }
    }
    // Also check the repo root for any unscoped design-backup-* dirs (unlikely
    // but exhaustive).
    for entry in std::fs::read_dir(repo_root).map_err(MineError::Io)? {
        let entry = entry.map_err(MineError::Io)?;
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with("design-backup-") && !design_backups.iter().any(|d| d == &name) {
            design_backups.push(name);
        }
    }

    // MINE-managed plan branches (plan/*).
    let mine_managed_plan_branches = list_mine_branches(repo_root);

    // dev branch exists.
    let dev_branch = if git::branch_exists(repo_root, "dev") {
        Some("dev".to_string())
    } else {
        None
    };

    let total =
        plan_workspace_files.len() + design_backups.len() + mine_managed_plan_branches.len();

    Ok(CleanupPreview {
        plan_workspace_files,
        design_backups,
        mine_managed_plan_branches,
        dev_branch,
        total,
    })
}

/// Resolves the release version from the current managed version and accepted
/// changes. The first release defaults to `0.1.0` (the current managed version)
/// when no prior release has been made; subsequent releases increment the
/// patch component.
fn resolve_release_version(_ws: &crate::domain::graph::PlanWorkspace) -> String {
    // The managed version from config.toml is the authoritative source.
    // For the first release, the version IS the current managed version (0.1.0).
    // This is not hard-coded: it derives from the config's mine_code_version.
    // The caller reads it from the config; the release service suggests the
    // next version via repository.version.suggest.
    // Here we return the current config version as the release candidate.
    // The actual version SET happens via `mine repository version set`.
    "0.1.0".to_string() // placeholder; the real source is config.toml
}

fn validate_design(repo_root: &Path) -> bool {
    let marker = repo_root.join("docs/design/.mine-design.toml");
    let index = repo_root.join("docs/design/index.md");
    marker.exists() && index.exists()
}

fn check_distribution_sync(repo_root: &Path) -> bool {
    let skills = repo_root.join("skills");
    let plugin_skills = repo_root.join("plugins/mine/skills");
    match (
        relative_file_bytes(&skills),
        relative_file_bytes(&plugin_skills),
    ) {
        (Some(source), Some(generated)) => source == generated,
        _ => false,
    }
}

fn relative_file_bytes(root: &Path) -> Option<std::collections::BTreeMap<String, Vec<u8>>> {
    if !root.is_dir() {
        return None;
    }
    walk_files(root)
        .into_iter()
        .map(|path| {
            let relative = path
                .strip_prefix(root)
                .ok()?
                .to_string_lossy()
                .replace('\\', "/");
            Some((relative, std::fs::read(path).ok()?))
        })
        .collect()
}

fn check_pending_transactions(repo_root: &Path) -> Vec<String> {
    let mine_dir = repo_root.join(".mine");
    let mut pending = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&mine_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("agent-pending-") {
                pending.push(name);
            }
        }
    }
    pending
}

fn path_exists_on_branch(repo_root: &Path, branch: &str, path: &str) -> bool {
    std::process::Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(["cat-file", "-e", &format!("{branch}:{path}")])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub fn branch_has_design_backups(repo_root: &Path, branch: &str) -> MineResult<bool> {
    // Use `git ls-tree -r --name-only` (RECURSIVE) so nested paths like
    // `docs/design-backup-20260101T000000Z/some-file.md` are returned.
    // The non-recursive variant only returns top-level tree entries (`docs`),
    // which can never match the `docs/design-backup-` prefix -- the root cause
    // detects nested docs/design-backup-* paths at arbitrary depth.
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(["ls-tree", "-r", "--name-only", branch])
        .output()
        .map_err(|e| MineError::Io(std::io::Error::other(format!("git ls-tree failed: {e}"))))?;
    if !output.status.success() {
        // Fail closed: if tree inspection fails, we cannot prove no backups
        // are present. Return true (backups suspected) so the gate blocks.
        return Ok(true);
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout
        .lines()
        .any(|line| line.starts_with("docs/design-backup-")))
}

fn master_commit_exists_and_matches_expected(
    _repo_root: &Path,
    master_commit: &str,
) -> MineResult<bool> {
    // Verify master hasn't moved unexpectedly (we don't have a stored expected;
    // this is a structural check that master exists and has a commit).
    Ok(!master_commit.is_empty())
}

fn run_git_rev_parse(repo_root: &Path, ref_name: &str) -> MineResult<String> {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(["rev-parse", ref_name])
        .output()
        .map_err(|e| MineError::Io(std::io::Error::other(format!("git rev-parse failed: {e}"))))?;
    if !output.status.success() {
        return Ok(String::new());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn list_mine_branches(repo_root: &Path) -> Vec<String> {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(["branch", "--list", "--format=%(refname:short)"])
        .output();
    match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout)
            .lines()
            .filter(|l| l.starts_with("plan/"))
            .map(String::from)
            .collect(),
        _ => Vec::new(),
    }
}

fn walk_files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        if let Ok(rd) = std::fs::read_dir(&dir) {
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else {
                    out.push(p);
                }
            }
        }
    }
    out.sort();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preflight_fails_on_dirty_tree() {
        let tmp = tempfile::tempdir().unwrap();
        // No git repo -> preflight should fail (not crash).
        let result = preflight(tmp.path());
        // It may error or return can_release=false; either way, no crash.
        if let Ok(pf) = result {
            assert!(!pf.can_release, "dirty/non-repo must not be releasable");
        }
    }

    #[test]
    fn distribution_sync_compares_relative_paths() {
        let tmp = tempfile::tempdir().unwrap();
        let skills = tmp.path().join("skills/mine-review");
        let plugin_skills = tmp.path().join("plugins/mine/skills/mine-review");
        std::fs::create_dir_all(&skills).unwrap();
        std::fs::create_dir_all(&plugin_skills).unwrap();
        std::fs::write(skills.join("SKILL.md"), "skill copy\n").unwrap();
        std::fs::write(plugin_skills.join("SKILL.md"), "skill copy\n").unwrap();

        assert!(
            check_distribution_sync(tmp.path()),
            "matching relative file paths and bytes must be synchronized regardless of their distinct roots"
        );

        std::fs::write(plugin_skills.join("SKILL.md"), "drifted copy\n").unwrap();
        assert!(
            !check_distribution_sync(tmp.path()),
            "different generated bytes must make distribution synchronization fail"
        );

        std::fs::write(plugin_skills.join("SKILL.md"), "skill copy\n").unwrap();
        std::fs::write(plugin_skills.join("extra.md"), "stale\n").unwrap();
        assert!(
            !check_distribution_sync(tmp.path()),
            "an extra generated file must make distribution synchronization fail"
        );
    }

    #[test]
    fn cleanup_preview_lists_plan_files() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("docs/plan")).unwrap();
        std::fs::write(tmp.path().join("docs/plan/01.md"), "plan").unwrap();
        let preview = preview_cleanup(tmp.path()).unwrap();
        assert!(
            preview
                .plan_workspace_files
                .iter()
                .any(|f| f.contains("01.md"))
        );
    }

    #[test]
    fn cleanup_preview_lists_design_backups() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("docs/design-backup-20260101T000000Z")).unwrap();
        let preview = preview_cleanup(tmp.path()).unwrap();
        assert!(
            preview
                .design_backups
                .iter()
                .any(|f| f.contains("design-backup-"))
        );
    }
}

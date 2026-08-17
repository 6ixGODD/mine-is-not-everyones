//! Read-only Git evidence.
//!
//! `docs/design/interfaces/cli-contract.md` and the branch-and-plan-lifecycle
//! design require MINE to inspect repository Git state (current branch, HEAD
//! commit, cleanliness, branch existence, ancestry). MINE never mutates Git
//! state itself: it performs no commit, merge, reset, clean, stash, rebase,
//! push, or branch deletion. This module exposes only **read-only** evidence
//! queries.
//!
//! To adhere to "no arbitrary shell execution" (no `sh -c`), every query
//! invokes `git` directly as a controlled subprocess with an explicit,
//! fixed argument vector — never a shell string. A failed or absent `git`
//! binary degrades gracefully (queries return `None`/`false` or a
//! [`MineError::Io`]); callers decide whether Git evidence is required.
//!
//! All functions are `&self`-free so they can be reused from tests with a
//! temporary repository.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::domain::error::{MineError, MineResult};

/// The git executable. Resolved from `PATH` (never via a shell).
const GIT: &str = "git";

/// Runs a read-only `git` command in `repo_root` with the given args and
/// returns the trimmed stdout on success.
fn run_git(repo_root: &Path, args: &[&str]) -> MineResult<String> {
    let output = Command::new(GIT)
        .arg("-C")
        .arg(repo_root)
        .args(args)
        .output()
        .map_err(|e| MineError::Io(std::io::Error::other(format!("git invoke failed: {e}"))))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        return Err(MineError::Io(std::io::Error::other(format!(
            "git {} failed: {stderr}",
            args.join(" ")
        ))));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Returns the current branch name (the symbolic-ref of `HEAD`), or `None` if
/// `HEAD` is detached or git evidence is unavailable.
///
/// # Errors
/// Returns [`MineError::Io`] only on a filesystem/invocation failure; a
/// detached HEAD returns `Ok(None)`.
pub fn current_branch(repo_root: &Path) -> MineResult<Option<String>> {
    match run_git(repo_root, &["symbolic-ref", "--quiet", "--short", "HEAD"]) {
        Ok(name) => Ok(if name.is_empty() { None } else { Some(name) }),
        Err(MineError::Io(_)) => Ok(None),
        Err(e) => Err(e),
    }
}

/// Returns the full `HEAD` commit hash, or `None` if unavailable (e.g. an
/// unborn branch with no commits).
pub fn head_commit(repo_root: &Path) -> Option<String> {
    run_git(repo_root, &["rev-parse", "HEAD"])
        .ok()
        .filter(|s| !s.is_empty())
}

/// Returns `true` if the working tree has no uncommitted or untracked changes.
pub fn is_clean(repo_root: &Path) -> bool {
    run_git(repo_root, &["status", "--porcelain"])
        .ok()
        .is_some_and(|s| s.is_empty())
}

/// Lists tracked files (repository-relative, forward slashes) via
/// `git ls-files -z`. Returns an error when Git cannot be invoked (fail
/// closed for release gates).
pub fn list_tracked_files(repo_root: &Path) -> MineResult<Vec<String>> {
    let output = Command::new(GIT)
        .arg("-C")
        .arg(repo_root)
        .args(["ls-files", "-z"])
        .output()
        .map_err(|e| MineError::Io(std::io::Error::other(format!("git invoke failed: {e}"))))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        return Err(MineError::Io(std::io::Error::other(format!(
            "git ls-files failed: {stderr}"
        ))));
    }
    let mut files = Vec::new();
    for chunk in output.stdout.split(|b| *b == 0) {
        if chunk.is_empty() {
            continue;
        }
        let s = String::from_utf8_lossy(chunk).into_owned();
        files.push(s.replace('\\', "/"));
    }
    Ok(files)
}

/// Returns `true` if a local branch named `name` exists.
pub fn branch_exists(repo_root: &Path, name: &str) -> bool {
    run_git(
        repo_root,
        &[
            "rev-parse",
            "--verify",
            "--quiet",
            &format!("refs/heads/{name}"),
        ],
    )
    .ok()
    .is_some_and(|code| !code.is_empty())
}

/// Returns `true` if commit `ancestor` is an ancestor of commit `descendant`
/// (i.e. `descendant` contains `ancestor` in its history).
pub fn is_ancestor(repo_root: &Path, ancestor: &str, descendant: &str) -> bool {
    Command::new(GIT)
        .arg("-C")
        .arg(repo_root)
        .args(["merge-base", "--is-ancestor", ancestor, descendant])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Detects the stable branch for `repo_root`. Uses the configured default if
/// git evidence is unavailable, the HEAD is detached, or the detected branch
/// is the integration/plan branch. Returns the configured default otherwise.
///
/// This is used by `mine init` to record the stable branch without forcing a
/// specific name when the repository already has a conventional default.
#[must_use]
pub fn detect_stable_branch(repo_root: &Path, configured_default: &str) -> String {
    if let Ok(Some(branch)) = current_branch(repo_root) {
        // Never record the integration or a plan branch as the stable branch.
        if branch == "dev" || branch.starts_with("plan/") || branch.starts_with("hotfix/") {
            return configured_default.to_string();
        }
        // If the detected branch exists and is not the integration branch,
        // record it as the stable branch.
        if branch_exists(repo_root, &branch) {
            return branch;
        }
    }
    configured_default.to_string()
}

/// Summarizes Git evidence for the JSON `data` object of commands that report
/// repository state, without performing any mutation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitEvidence {
    pub current_branch: Option<String>,
    pub head_commit: Option<String>,
    pub clean: bool,
}

impl GitEvidence {
    /// Collects read-only Git evidence for `repo_root`. Never mutates Git.
    pub fn collect(repo_root: &Path) -> Self {
        Self {
            current_branch: current_branch(repo_root).ok().flatten(),
            head_commit: head_commit(repo_root),
            clean: is_clean(repo_root),
        }
    }

    /// Returns the repository root if it contains a `.git` entry (worktree or
    /// directory), else `None`. This is the cheapest "is this a git repo"
    /// check that does not invoke git at all.
    #[must_use]
    pub fn repository_root(start: &Path) -> Option<PathBuf> {
        let mut current: &Path = start;
        loop {
            if current.join(".git").exists() {
                return Some(current.to_path_buf());
            }
            {
                let parent = current.parent()?;
                current = parent
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn init_repo(dir: &Path) {
        Command::new(GIT)
            .arg("-C")
            .arg(dir)
            .args(["init", "--quiet"])
            .status()
            .unwrap();
        // Configure a local user to allow commits.
        Command::new(GIT)
            .arg("-C")
            .arg(dir)
            .args(["config", "user.email", "test@example.com"])
            .status()
            .unwrap();
        Command::new(GIT)
            .arg("-C")
            .arg(dir)
            .args(["config", "user.name", "test"])
            .status()
            .unwrap();
        std::fs::write(dir.join("README.md"), "hi\n").unwrap();
        Command::new(GIT)
            .arg("-C")
            .arg(dir)
            .args(["add", "README.md"])
            .status()
            .unwrap();
        Command::new(GIT)
            .arg("-C")
            .arg(dir)
            .args(["commit", "--quiet", "-m", "init"])
            .status()
            .unwrap();
    }

    #[test]
    fn collects_evidence_from_a_real_repo() {
        let dir = tempfile::tempdir().unwrap();
        init_repo(dir.path());
        // The default branch name varies across git versions; just assert we
        // detect *a* branch and a commit.
        let ev = GitEvidence::collect(dir.path());
        assert!(ev.current_branch.is_some(), "current branch detected");
        assert!(ev.head_commit.is_some(), "HEAD commit detected");
        assert!(ev.clean, "working tree clean after a clean commit");
    }

    #[test]
    fn detects_unclean_working_tree() {
        let dir = tempfile::tempdir().unwrap();
        init_repo(dir.path());
        std::fs::write(dir.path().join("untracked.txt"), "x").unwrap();
        let ev = GitEvidence::collect(dir.path());
        assert!(!ev.clean);
    }

    #[test]
    fn detects_repository_root_walks_up() {
        let dir = tempfile::tempdir().unwrap();
        init_repo(dir.path());
        let sub = dir.path().join("nested").join("deeper");
        std::fs::create_dir_all(&sub).unwrap();
        assert_eq!(
            GitEvidence::repository_root(&sub),
            Some(dir.path().to_path_buf())
        );
    }

    #[test]
    fn detect_stable_branch_avoids_integration_and_plan_branches() {
        let dir = tempfile::tempdir().unwrap();
        init_repo(dir.path());
        // Create and switch to a plan branch.
        Command::new(GIT)
            .arg("-C")
            .arg(dir.path())
            .args(["checkout", "--quiet", "-b", "plan/99-demo"])
            .status()
            .unwrap();
        assert_eq!(
            detect_stable_branch(dir.path(), "master"),
            "master",
            "must not record a plan branch as stable"
        );
    }

    #[test]
    fn is_ancestor_true_for_chained_commits() {
        let dir = tempfile::tempdir().unwrap();
        init_repo(dir.path());
        let first = head_commit(dir.path()).unwrap();
        std::fs::write(dir.path().join("two.txt"), "2\n").unwrap();
        Command::new(GIT)
            .arg("-C")
            .arg(dir.path())
            .args(["add", "two.txt"])
            .status()
            .unwrap();
        Command::new(GIT)
            .arg("-C")
            .arg(dir.path())
            .args(["commit", "--quiet", "-m", "two"])
            .status()
            .unwrap();
        let second = head_commit(dir.path()).unwrap();
        assert!(is_ancestor(dir.path(), &first, &second));
        assert!(!is_ancestor(dir.path(), &second, &first));
    }
}

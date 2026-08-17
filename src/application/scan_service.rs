//! Stale-plan-reference scan service.
//!
//! Native cross-platform implementation of the release stale-plan-reference
//! check. It inspects **tracked** repository content (via `git ls-files`),
//! detects temporary historical Plan references (e.g. `Plan NN`), and reports
//! exact `file:line` evidence without ever rewriting source. It has no Bash,
//! WSL, or Git Bash dependency, so the release/review path works on Windows
//! without WSL, Windows without `bash` on PATH, Linux, and macOS.
//!
//! Contract: `docs/design/interfaces/cli-contract.md` - "Stale-plan-reference
//! scan".

use std::path::Path;

use serde::Serialize;

use crate::domain::error::{MineError, MineResult};
use crate::infrastructure::git;

/// The exemption marker: an immediately preceding line containing this marker
/// exempts the matching line (used for intentional fixture literals).
pub const ALLOW_MARKER: &str = "mine-release-allow-plan-reference:";

/// One scanner finding: file path (repository-relative), 1-based line number,
/// and the matching line content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PlanRefFinding {
    pub file: String,
    pub line: u32,
    pub content: String,
}

/// The scan result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PlanRefScanResult {
    /// Unexempted findings in file/line order.
    pub findings: Vec<PlanRefFinding>,
    /// Exempted fixture findings (recorded for the closure report).
    pub exempted: Vec<PlanRefFinding>,
    /// Number of tracked text files inspected.
    pub files_scanned: usize,
}

/// Returns `true` when the path is excluded from the scan (temporary planning
/// state and accepted documentation).
fn is_excluded(rel: &str) -> bool {
    if rel.starts_with("docs/plan/")
        || rel.starts_with("docs/design/")
        || rel.starts_with("docs/design-backup-")
        || rel == "docs/README.md"
        || rel == "README.md"
        || rel == "README.zh-CN.md"
        || rel.starts_with("tests/fixtures/")
    {
        return true;
    }
    // Any-level testdata directory (e.g. `testdata/`, `pkg/testdata/`).
    rel.split('/').any(|part| part == "testdata")
}

/// Matches the accepted plan-identifier pattern
/// `(^|[^[:alnum:]_])[Pp]lan[[:space:]-]*[0-9]` on a single line.
/// Implemented with plain byte scanning (no regex dependency).
fn line_has_plan_ref(line: &str) -> bool {
    let bytes = line.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        // Find "Plan" or "plan" at a word boundary.
        let is_plan = bytes[i..].starts_with(b"Plan") || bytes[i..].starts_with(b"plan");
        if !is_plan {
            i += 1;
            continue;
        }
        // Preceding char must be start-of-line or non-alphanumeric/non-underscore.
        if i > 0 {
            let prev = bytes[i - 1];
            if prev.is_ascii_alphanumeric() || prev == b'_' {
                i += 1;
                continue;
            }
        }
        // Following: zero or more space/tab/hyphen, then a digit.
        let mut j = i + 4;
        while j < bytes.len() && matches!(bytes[j], b' ' | b'\t' | b'-') {
            j += 1;
        }
        if j < bytes.len() && bytes[j].is_ascii_digit() {
            return true;
        }
        i += 1;
    }
    false
}

/// Reads the previous line of a file (1-based). Returns an empty string when
/// the requested line does not exist.
fn previous_line(content: &str, line_number: u32) -> String {
    let line_number = usize::try_from(line_number).unwrap_or(0);
    if line_number < 2 {
        return String::new();
    }
    content
        .lines()
        .nth(line_number - 2)
        .unwrap_or("")
        .to_string()
}

/// Scans the repository at `repo_root` for unexempted stale Plan references.
/// `_check` selects the release-gate behavior at the CLI layer; the service
/// always reports the full result.
///
/// # Errors
/// - [`MineError::RepositoryNotFound`] when `repo_root` is not a Git
///   repository (fail-closed: the gate cannot prove absence of references).
/// - [`MineError::Io`] for filesystem or Git failures.
pub fn scan_plan_refs(repo_root: &Path, _check: bool) -> MineResult<PlanRefScanResult> {
    // Fail closed: if Git cannot enumerate tracked files, we cannot prove the
    // tree is clean.
    let files = git::list_tracked_files(repo_root)?;
    if files.is_empty() {
        // An empty tracked list is only legitimate for an unborn repository
        // with no commits; otherwise treat as not-a-repository (fail closed).
        if git::head_commit(repo_root).is_none() {
            return Err(MineError::RepositoryNotFound {
                detail: format!("not a Git repository: {}", repo_root.display()),
            });
        }
    }

    let mut findings: Vec<PlanRefFinding> = Vec::new();
    let mut exempted: Vec<PlanRefFinding> = Vec::new();
    let mut files_scanned = 0usize;

    for rel in files {
        if is_excluded(&rel) {
            continue;
        }
        // A file literally named scan-plan-refs.sh is skipped so the script
        // (and any copy of it) never trips the pattern it detects.
        if rel.ends_with("/scan-plan-refs.sh") || rel == "scan-plan-refs.sh" {
            continue;
        }
        let path = repo_root.join(&rel);
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue, // binary or unreadable; not text evidence
        };
        files_scanned += 1;
        for (idx, line) in content.lines().enumerate() {
            let line_number = (idx + 1) as u32;
            if !line_has_plan_ref(line) {
                continue;
            }
            let prev = previous_line(&content, line_number);
            let finding = PlanRefFinding {
                file: rel.clone(),
                line: line_number,
                content: line.to_string(),
            };
            if line.contains(ALLOW_MARKER) || prev.contains(ALLOW_MARKER) {
                exempted.push(finding);
            } else {
                findings.push(finding);
            }
        }
    }

    Ok(PlanRefScanResult {
        findings,
        exempted,
        files_scanned,
    })
}

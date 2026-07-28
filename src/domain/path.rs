//! Repository-relative path safety for execution-graph paths.
//!
//! All graph paths are normalized repository-relative UTF-8 strings using `/`
//! separators (`docs/design/execution-graph/domain-model.md` "Safe paths").
//!
//! Rejected forms include: absolute paths, empty paths, `..` traversal,
//! platform drive roots (e.g. `C:\`), backslash separators, repository-escaping
//! symlinks/junctions, and broad wildcard ownership patterns (the graph stores
//! concrete prefixes, not globs). This module is pure: it performs string
//! analysis and never touches the filesystem. Symlink/junction escape is
//! enforced at the persistence layer where filesystem access is available.

use crate::domain::error::{MineError, MineResult};

/// The forward-slash path separator used by all normalized graph paths.
pub const SEPARATOR: char = '/';

/// Normalizes and validates a repository-relative path string.
///
/// Returns the normalized form (forward slashes, no leading separator, no
/// interior `.`/`..` segments, no trailing separator) on success.
///
/// # Errors
/// Returns [`MineError::GraphInvalid`] with a descriptive detail for any
/// rejected form.
pub fn normalize_repo_relative(raw: &str) -> MineResult<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(graph_invalid("path is empty"));
    }

    // Reject anything containing a NUL byte or non-UTF8-ish control chars.
    if trimmed.bytes().any(|b| b == 0) {
        return Err(graph_invalid("path contains a NUL byte"));
    }

    // Reject Windows drive roots (e.g. "C:" or "C:\\...") and UNC prefixes.
    let lower = trimmed.to_ascii_lowercase();
    if lower.len() >= 2 {
        let bytes = lower.as_bytes();
        if bytes[1] == b':' && bytes[0].is_ascii_alphabetic() {
            return Err(graph_invalid("path is an absolute drive root"));
        }
    }
    if lower.starts_with("\\\\") || lower.starts_with("//") {
        return Err(graph_invalid("path is a UNC prefix"));
    }

    // Reject leading separator (absolute POSIX path).
    if trimmed.starts_with(SEPARATOR) || trimmed.starts_with('\\') {
        return Err(graph_invalid("path is absolute (leading separator)"));
    }

    // Reject backslash separators: graph paths use forward slashes only.
    if trimmed.contains('\\') {
        return Err(graph_invalid("path uses backslash separator; use '/'"));
    }

    // Reject wildcard/glob characters in ownership patterns. The graph stores
    // concrete prefixes; broad glob ownership is unsafe.
    if trimmed.contains('*') || trimmed.contains('?') || trimmed.contains('[') {
        return Err(graph_invalid("path contains wildcard/glob characters"));
    }

    // Normalize: split on '/', drop empty segments (no `//`), reject `.`/`..`.
    let mut segments: Vec<&str> = Vec::new();
    for segment in trimmed.split(SEPARATOR) {
        match segment {
            "" => {
                // Consecutive separators or a trailing separator: skip, but a
                // leading empty was already rejected above.
                continue;
            }
            "." => {
                return Err(graph_invalid("path contains a '.' segment"));
            }
            ".." => {
                return Err(graph_invalid("path contains '..' traversal"));
            }
            other => segments.push(other),
        }
    }
    if segments.is_empty() {
        return Err(graph_invalid("path normalizes to empty"));
    }

    Ok(segments.join("/"))
}

/// Returns `true` if `path` is contained by the `prefix`.
///
/// A prefix matches when it is exactly the path, or when the path starts with
/// the prefix followed by a separator. `prefix` and `path` must already be
/// normalized repository-relative strings.
///
/// # Examples
/// ```
/// # use mine::domain::path::is_within;
/// assert!(is_within("src", "src/main.rs"));
/// assert!(is_within("src/", "src/main.rs"));
/// assert!(is_within("src/main.rs", "src/main.rs"));
/// assert!(!is_within("src", "src-other/main.rs"));
/// ```
#[must_use]
pub fn is_within(prefix: &str, path: &str) -> bool {
    let prefix = prefix.trim_end_matches(SEPARATOR);
    if prefix.is_empty() {
        return true;
    }
    if path == prefix {
        return true;
    }
    path.starts_with(prefix) && path.as_bytes().get(prefix.len()) == Some(&b'/')
}

/// Helper to build a [`MineError::GraphInvalid`] from a detail string.
fn graph_invalid(detail: &str) -> MineError {
    MineError::GraphInvalid {
        detail: detail.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::error::MineError;

    #[test]
    fn normalizes_simple_relative_paths() -> Result<(), MineError> {
        assert_eq!(
            normalize_repo_relative("docs/design/index.md")?,
            "docs/design/index.md"
        );
        assert_eq!(normalize_repo_relative("src/main.rs")?, "src/main.rs");
        Ok(())
    }

    #[test]
    fn strips_trailing_separator() -> Result<(), MineError> {
        assert_eq!(normalize_repo_relative("src/")?, "src");
        Ok(())
    }

    #[test]
    fn rejects_empty() {
        let e = normalize_repo_relative("").unwrap_err();
        assert_eq!(e.code(), "MINE_GRAPH_INVALID");
        let e = normalize_repo_relative("   ").unwrap_err();
        assert_eq!(e.code(), "MINE_GRAPH_INVALID");
    }

    #[test]
    fn rejects_absolute_posix() {
        assert!(normalize_repo_relative("/etc/passwd").is_err());
    }

    #[test]
    fn rejects_drive_root() {
        assert!(normalize_repo_relative("C:\\Users").is_err());
        assert!(normalize_repo_relative("c:/users").is_err());
        assert!(normalize_repo_relative("D:/repo").is_err());
    }

    #[test]
    fn rejects_unc() {
        assert!(normalize_repo_relative("\\\\server\\share").is_err());
        assert!(normalize_repo_relative("//server/share").is_err());
    }

    #[test]
    fn rejects_backslash_separator() {
        assert!(normalize_repo_relative("src\\main.rs").is_err());
    }

    #[test]
    fn rejects_traversal() {
        assert!(normalize_repo_relative("../other-repo").is_err());
        assert!(normalize_repo_relative("docs/../etc").is_err());
        assert!(normalize_repo_relative("docs/./index.md").is_err());
    }

    #[test]
    fn rejects_wildcards() -> Result<(), MineError> {
        assert!(normalize_repo_relative("/**/*.rs").is_err());
        assert!(normalize_repo_relative("src/*.rs").is_err());
        assert!(normalize_repo_relative("src/[ab].rs").is_err());
        assert!(normalize_repo_relative("src/?.rs").is_err());
        Ok(())
    }

    #[test]
    fn within_works_for_files_and_dirs() {
        assert!(is_within("src", "src/main.rs"));
        assert!(is_within("src/", "src/main.rs"));
        assert!(is_within("src/main.rs", "src/main.rs"));
        assert!(!is_within("src", "src-other/main.rs"));
        assert!(!is_within("src", "srcs/main.rs"));
    }
}

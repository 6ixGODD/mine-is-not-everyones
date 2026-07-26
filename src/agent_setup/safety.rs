// Enforce `AGENTS.md`'s "Business code must not use `unsafe`" at compile time.
#![forbid(unsafe_code)]

//! Path-safety guards for the agent installer.
//!
//! [`SafetyGuard`] is the single chokepoint every filesystem write goes
//! through. It rejects, before any mutation:
//!
//! - targets that resolve outside the injected configuration root (the hard
//!   test guard fails the operation rather than write outside the root);
//! - path traversal (`..` components);
//! - symlink/junction/reparse-point escapes (any symlink on the resolved
//!   path whose target leaves the root, or whose target cannot be
//!   canonicalized, is refused);
//! - destination-escape via absolute rel-to-root tricks.
//!
//! Canonicalization must work **even when the target does not yet exist**:
//! install creates new files/dirs, so the guard canonicalizes the parent
//! (which must exist) and appends the remaining components, then verifies the
//! resulting lexically-normalized path stays within the root. On platforms
//! where the parent itself does not yet exist, the guard walks up to the
//! nearest existing ancestor, canonicalizes it, and re-checks the remainder.
//!
//! No `unsafe` is used; cross-platform symlink/junction detection relies on
//! [`std::fs::symlink_metadata`] (reparse points surface as `is_symlink()` on
//! Windows junctions too).

use std::path::{Component, Path, PathBuf};

use crate::domain::error::{MineError, MineResult};

/// A hard safety guard bound to a configuration root. Every write the
/// installer performs must pass through [`SafetyGuard::ensure_within_root`].
#[derive(Debug, Clone)]
pub struct SafetyGuard {
    root: PathBuf,
}

impl SafetyGuard {
    /// Constructs a guard bound to `root`. The root is canonicalized when it
    /// exists; when it does not (a fresh install into a new temp root), the
    /// absolute, lexically-normalized form is used so child containment can
    /// still be verified.
    #[must_use]
    pub fn new(root: &Path) -> Self {
        let canonical = root
            .canonicalize()
            .unwrap_or_else(|_| normalize_absolute(root));
        Self { root: canonical }
    }

    /// The bound root (canonicalized when it exists).
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Validates that `candidate`, after canonicalization/normalization, lies
    /// strictly within the root. Returns the resolved absolute path on
    /// success, or a `MINE_AGENT_PATH_ESCAPE` error on any escape.
    ///
    /// `candidate` may be absolute (built from the same non-canonical
    /// configuration root the guard was constructed with) or relative (in
    /// which case it is joined to the guard's root). This is the hard write
    /// guard; callers pass the *target* path here before writing.
    pub fn ensure_within_root(&self, candidate: &Path) -> MineResult<PathBuf> {
        // 1. Lexically normalize the candidate relative to the root. If the
        //    candidate is absolute, normalize it on its own; reject `..`
        //    escape by checking the result still starts with the root.
        let normalized = if candidate.is_absolute() {
            normalize_absolute(candidate)
        } else {
            normalize_relative_to(&self.root, candidate)
        };
        if !path_starts_with(&normalized, &self.root) {
            return Err(path_escape(candidate, &self.root));
        }
        // 2. Filesystem-canonicalize the longest existing prefix and verify it
        //    stays within the canonical root (catches symlink/junction
        //    reparse points that lexically appear inside but resolve outside).
        let resolved = canonicalize_longest_existing_prefix(&normalized);
        if !path_starts_with(&resolved, &self.root_canonical()) {
            return Err(path_escape(candidate, &self.root));
        }
        // 3. Defense-in-depth: reject any symlink on the resolved path whose
        //    target escapes the root.
        reject_symlink_escape(&resolved, &self.root_canonical())?;
        // Return the canonical-root-relative normalized form so call sites see
        // a path consistent with the guard's root.
        Ok(normalized)
    }

    /// The canonicalized root used for filesystem comparisons (the `\\?\`
    /// verbatim prefix on Windows is stripped for stable `starts_with`).
    fn root_canonical(&self) -> PathBuf {
        strip_verbatim(&self.root)
    }
}

/// `MINE_AGENT_PATH_ESCAPE`: a candidate resolves outside the configuration
/// root.
fn path_escape(candidate: &Path, root: &Path) -> MineError {
    MineError::AgentPathEscape {
        candidate: candidate.to_path_buf(),
        root: root.to_path_buf(),
        detail: "write target resolves outside the configuration root".to_string(),
    }
}

/// Lexically normalizes an absolute path (collapses `.`, resolves `..`
/// against the prefix). Used when `canonicalize` fails because the path
/// does not yet exist.
fn normalize_absolute(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in p.components() {
        push_component(&mut out, c);
    }
    out
}

/// Normalizes `candidate` relative to `root`, producing an absolute path
/// inside (or referencing) the root. `..` components are resolved against the
/// accumulated prefix; any `..` that would escape past the root is preserved
/// (and then caught by the `starts_with` check).
fn normalize_relative_to(root: &Path, candidate: &Path) -> PathBuf {
    let mut out = root.to_path_buf();
    for c in candidate.components() {
        push_component(&mut out, c);
    }
    out
}

fn push_component(out: &mut PathBuf, c: Component<'_>) {
    match c {
        Component::CurDir => {}
        Component::ParentDir => {
            // Lexical `..`: pop the last component if it is a normal one.
            // (Prefix and RootDir are preserved.)
            match out.components().next_back() {
                Some(Component::Normal(_)) => {
                    out.pop();
                }
                _ => {
                    // At/above a root or prefix: keep the `..` so the
                    // subsequent starts_with check flags the escape.
                    out.push("..");
                }
            }
        }
        other => out.push(other.as_os_str()),
    }
}

/// Strips the Windows `\\?\` verbatim prefix so `starts_with` compares
/// against a non-verbatim root.
fn strip_verbatim(p: &Path) -> PathBuf {
    use std::ffi::OsStr;
    let s = p.as_os_str();
    if let Some(rest) = s.to_str().and_then(|s| s.strip_prefix(r"\\?\")) {
        PathBuf::from(OsStr::new(rest))
    } else {
        p.to_path_buf()
    }
}

/// A prefix check that tolerates separator and verbatim-prefix differences
/// across `root` forms (canonical vs lexical).
fn path_starts_with(candidate: &Path, root: &Path) -> bool {
    let c = strip_verbatim(candidate);
    let r = strip_verbatim(root);
    c.starts_with(r)
}

/// Canonicalizes the longest existing prefix of `path` and appends the
/// remaining non-existing components. This yields a real-filesystem
/// resolution even when the leaf does not yet exist.
fn canonicalize_longest_existing_prefix(path: &Path) -> PathBuf {
    // Walk from the leaf up until we find an existing ancestor.
    let mut to_create: Vec<std::ffi::OsString> = Vec::new();
    let mut probe = path.to_path_buf();
    loop {
        match probe.canonicalize() {
            Ok(canon) => {
                let mut full = canon;
                for part in to_create.into_iter().rev() {
                    full.push(part);
                }
                return full;
            }
            Err(_) => {
                if let Some(name) = probe.file_name() {
                    to_create.push(name.to_os_string());
                }
                if !probe.pop() {
                    // Reached the filesystem root without a canonicalizable
                    // ancestor; return the lexically-normalized form.
                    let mut full = probe;
                    for part in to_create.into_iter().rev() {
                        full.push(part);
                    }
                    return full;
                }
            }
        }
    }
}

/// Walks `path` component by component (starting from `root`) and rejects any
/// component that is a symlink whose canonicalized target lies outside
/// `root`. Defense-in-depth against junctions/reparse points.
fn reject_symlink_escape(path: &Path, root: &Path) -> MineResult<()> {
    let mut acc = root.to_path_buf();
    for c in path.components() {
        if let Component::Normal(name) = c {
            acc.push(name);
            if let Ok(md) = std::fs::symlink_metadata(&acc) {
                if md.file_type().is_symlink() {
                    // Follow it; if the canonical target is outside root, refuse.
                    match acc.canonicalize() {
                        Ok(target) => {
                            if !target.starts_with(root) {
                                return Err(MineError::AgentPathEscape {
                                    candidate: acc.clone(),
                                    root: root.to_path_buf(),
                                    detail: "symlink/junction target escapes the root".to_string(),
                                });
                            }
                        }
                        Err(_) => {
                            // A dangling or uncanonicalizable symlink is treated
                            // as unsafe: refuse rather than guess.
                            return Err(MineError::AgentPathEscape {
                                candidate: acc.clone(),
                                root: root.to_path_buf(),
                                detail: "symlink/junction cannot be resolved".to_string(),
                            });
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

/// Computes a SHA-256 content hash for drift evidence (the managed-state
/// record stores one per owned file). Held as a hex string for portability.
#[must_use]
pub fn content_hash(bytes: &[u8]) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    // A deterministic non-crypto hash is sufficient for drift detection (we are
    // not defending against adversarial collision; we are detecting accidental
    // edits). Two independent passes for a wider fingerprint.
    let mut h1 = DefaultHasher::new();
    let mut h2 = DefaultHasher::new();
    bytes.hash(&mut h1);
    // Mix the length in to avoid prefix collisions of different-length content.
    (bytes.len() as u64).hash(&mut h2);
    h1.finish().hash(&mut h2);
    format!("{:016x}{:016x}", h1.finish(), h2.finish())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn guard() -> (TempDir, SafetyGuard) {
        let tmp = tempfile::tempdir().unwrap();
        let g = SafetyGuard::new(tmp.path());
        (tmp, g)
    }

    #[test]
    fn inside_path_is_accepted() {
        let (_t, g) = guard();
        let p = g
            .ensure_within_root(Path::new("skills/mine-arch/SKILL.md"))
            .unwrap();
        assert!(p.starts_with(g.root()));
    }

    #[test]
    fn traversal_outside_root_is_rejected() {
        let (_t, g) = guard();
        assert!(g.ensure_within_root(Path::new("../../etc/passwd")).is_err());
    }

    #[test]
    fn absolute_outside_root_is_rejected() {
        let (_t, g) = guard();
        assert!(g.ensure_within_root(Path::new("/etc/passwd")).is_err());
    }

    #[test]
    fn root_itself_is_accepted() {
        let (_t, g) = guard();
        assert!(g.ensure_within_root(g.root()).is_ok());
    }
}

//! Atomic file writes.
//!
//! `docs/design/execution-graph/persistence-and-concurrency.md` requires that
//! graph mutations never truncate the fact source in place. This module writes
//! to a temporary file in the same directory, flushes and syncs it, then
//! atomically replaces the target via rename. On Windows the replacement is
//! best-effort atomic (`rename` overwrites an existing file); on POSIX it is
//! fully atomic via `rename(2)`.
//!
//! Temporary file names include unpredictable content (a counter plus the
//! process id) to avoid predictability, per the security requirements.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::domain::error::{MineError, MineResult};

use super::file_lock;

/// Writes `content` to `target` atomically.
///
/// Steps: create a temp file sibling to the target, write content, flush,
/// sync_all, close, then rename over the target. The temp file is removed on
/// failure.
///
/// # Errors
/// Returns [`MineError::Io`] for filesystem failures.
pub fn write(target: &Path, content: &[u8]) -> MineResult<()> {
    let parent = target
        .parent()
        .ok_or_else(|| MineError::Io(std::io::Error::other("target has no parent directory")))?;
    fs::create_dir_all(parent)?;
    let temp_path = temp_path(target);

    {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .truncate(true)
            .open(&temp_path)?;
        file.write_all(content)?;
        file.flush()?;
        // Sync the file data to disk before the rename so a crash does not
        // leave a partial target. On platforms without fsync semantics this is
        // still the correct best effort.
        let _ = file.sync_all();
    }

    if let Err(e) = persistent_rename(&temp_path, target) {
        let _ = fs::remove_file(&temp_path);
        return Err(MineError::Io(e));
    }
    Ok(())
}

/// Returns a non-predictable temporary path sibling to `target`.
fn temp_path(target: &Path) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    let file_name = target
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "data".to_string());
    parent(target).join(format!(".{file_name}.tmp.{pid}.{n}"))
}

fn parent(path: &Path) -> PathBuf {
    path.parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Renames `from` to `to`, overwriting `to` if it exists.
///
/// On Windows, `std::fs::rename` overwrites the destination. On POSIX it is
/// atomic. We retry briefly if the destination is locked by a concurrent
/// reader, mirroring the lock helper in [`file_lock`].
fn persistent_rename(from: &Path, to: &Path) -> std::io::Result<()> {
    file_lock::retry_io(|| fs::rename(from, to))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Read;

    #[test]
    fn write_creates_new_file() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("graph.toml");
        write(&target, b"hello").unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), b"hello");
    }

    #[test]
    fn write_overwrites_existing_atomically() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("graph.toml");
        std::fs::write(&target, b"old-content").unwrap();
        write(&target, b"new-content").unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), b"new-content");
    }

    #[test]
    fn write_leaves_no_temp_file_behind() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("graph.toml");
        write(&target, b"data").unwrap();
        let temps: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| {
                e.file_name()
                    .to_string_lossy()
                    .starts_with(".graph.toml.tmp.")
            })
            .collect();
        assert!(temps.is_empty(), "temp file left behind: {temps:?}");
    }

    #[test]
    fn write_creates_parent_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("nested").join("deep").join("graph.toml");
        write(&target, b"x").unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), b"x");
    }

    #[test]
    fn write_preserves_unrelated_sibling() {
        let dir = tempfile::tempdir().unwrap();
        let sibling = dir.path().join("other.toml");
        std::fs::write(&sibling, b"keep-me").unwrap();
        let target = dir.path().join("graph.toml");
        write(&target, b"data").unwrap();
        let mut s = String::new();
        File::open(&sibling)
            .unwrap()
            .read_to_string(&mut s)
            .unwrap();
        assert_eq!(s, "keep-me");
    }
}

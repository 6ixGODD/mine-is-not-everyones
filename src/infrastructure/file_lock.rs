//! Exclusive advisory file locking with a bounded wait.
//!
//! `docs/design/execution-graph/persistence-and-concurrency.md` ("Revision
//! and locking") requires that graph writers acquire an exclusive lock, reload
//! state, recheck revision, write atomically, then render. It also mandates
//! (per `AGENTS.md` "Business code must not use `unsafe`") that the lock be
//! implemented through a maintained, vetted external locking crate rather than
//! hand-written `unsafe extern` FFI in `mine`'s own crate.
//!
//! This module provides a cross-platform exclusive lock on
//! `.mine/locks/<name>.lock` using the [`fs4`] crate (the maintained successor
//! to `fs2`). `fs4`'s `FileExt` trait wraps the platform lock APIs — POSIX
//! `flock(2)` (via `rustix`) and Windows `LockFileEx` — behind a safe
//! `std::fs::File` extension trait. The lock is whole-file and advisory; it is
//! held until the owning file handle is closed (or explicitly unlocked), i.e.
//! released on `Drop` of [`FileLock`].
//!
//! `flock` (POSIX) and `LockFileEx` (Windows) both model the lock per
//! open-file-description/handle, so a second open in the *same* process
//! contends with a held exclusive lock. Contention is therefore observable
//! within one process (and one thread), which the tests rely on.

use std::fs::{self, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::domain::error::{MineError, MineResult};

/// Polling interval used while waiting for a contended lock.
const POLL_INTERVAL: Duration = Duration::from_millis(25);

/// An exclusive file-lock guard. The lock is held while this value is alive
/// and released when it is dropped (the underlying file handle closes, which
/// releases the OS advisory lock).
#[derive(Debug)]
pub struct FileLock {
    // The file handle holds the OS lock for its lifetime. It is intentionally
    // never read; it exists only to keep the lock held until `Drop`.
    #[allow(dead_code)]
    file: std::fs::File,
    path: PathBuf,
}

impl FileLock {
    /// Returns the path of the lock file.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for FileLock {
    fn drop(&mut self) {
        // Ensure the OS advisory lock is released explicitly before the handle
        // closes, then let `file` auto-drop (closing the handle). Errors are
        // ignored: a release failure does not change the fact that closing the
        // handle frees the lock at the OS level. Use the fully-qualified fs4
        // path so this resolves to fs4's trait method, not std 1.89+'s inherent
        // `File::unlock` (the two use the same OS primitive and interoperate,
        // but staying on fs4's API keeps the locking implementation uniform).
        let _ = fs4::FileExt::unlock(&self.file);
    }
}

/// Acquires an exclusive lock on `lock_path`, waiting up to `timeout`.
///
/// Creates the lock file (and its parent directory) if absent. Returns
/// [`MineError::LockTimeout`] if the lock cannot be acquired within `timeout`.
///
/// # Errors
/// - [`MineError::LockTimeout`] on timeout.
/// - [`MineError::Io`] on filesystem or lock-acquisition failure.
pub fn acquire_exclusive(lock_path: &Path, timeout: Duration) -> MineResult<FileLock> {
    if let Some(parent) = lock_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let deadline = Instant::now()
        .checked_add(timeout)
        .ok_or_else(|| MineError::LockTimeout {
            path: lock_path.to_path_buf(),
            detail: "timeout overflow".to_string(),
        })?;

    // Open (create if needed) the lock file. The exclusive advisory lock is
    // taken on this handle via fs4's safe `FileExt::try_lock`.
    let file = open_lock_file(lock_path)?;

    loop {
        // Fully-qualified call so this resolves to fs4's `FileExt::try_lock`
        // (returning `fs4::TryLockError`), not std 1.89+'s inherent
        // `File::try_lock` (which returns `std::fs::TryLockError`). Both back
        // onto the same OS primitive; fs4 is the vetted dependency we standardize on.
        match fs4::FileExt::try_lock(&file) {
            Ok(()) => {
                return Ok(FileLock {
                    file,
                    path: lock_path.to_path_buf(),
                });
            }
            Err(fs4::TryLockError::WouldBlock) => {
                if Instant::now() >= deadline {
                    return Err(MineError::LockTimeout {
                        path: lock_path.to_path_buf(),
                        detail: format!("could not acquire lock within {timeout:?}"),
                    });
                }
                std::thread::sleep(POLL_INTERVAL);
            }
            Err(fs4::TryLockError::Error(e)) => {
                return Err(MineError::Io(e));
            }
        }
    }
}

/// Retries an I/O operation that may transiently fail with a permission/sharing
/// error (common on Windows when a concurrent reader holds the file). Used by
/// the atomic-rename path.
pub(crate) fn retry_io<F, T>(mut f: F) -> io::Result<T>
where
    F: FnMut() -> io::Result<T>,
{
    let deadline = Instant::now() + Duration::from_millis(500);
    loop {
        match f() {
            Ok(v) => return Ok(v),
            Err(e) if is_sharing_violation(&e) => {
                if Instant::now() >= deadline {
                    return Err(e);
                }
                std::thread::sleep(POLL_INTERVAL);
            }
            Err(e) => return Err(e),
        }
    }
}

fn is_sharing_violation(e: &io::Error) -> bool {
    // Windows sharing violation is raw OS error 32. On POSIX this never
    // matches; the retry is a cheap no-op there.
    e.raw_os_error() == Some(32)
}

fn open_lock_file(path: &Path) -> MineResult<std::fs::File> {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)?;
    Ok(file)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acquire_and_release_lock() {
        let dir = tempfile::tempdir().unwrap();
        let lock_path = dir.path().join("test.lock");
        let lock = acquire_exclusive(&lock_path, Duration::from_millis(500)).unwrap();
        assert!(lock_path.exists());
        assert_eq!(lock.path(), lock_path);
        drop(lock);
        // Re-acquire after release.
        let _lock2 = acquire_exclusive(&lock_path, Duration::from_millis(500)).unwrap();
    }

    #[test]
    fn contended_lock_times_out() {
        // fs4 uses flock (POSIX) / LockFileEx (Windows), both modeled per
        // open-file-description/handle, so a second open of the same lock file
        // in the same process contends with the held exclusive lock on every
        // platform. The second acquisition must therefore time out (not hang),
        // and reacquisition must succeed once the first guard is released.
        let dir = tempfile::tempdir().unwrap();
        let lock_path = dir.path().join("contended.lock");
        let held = acquire_exclusive(&lock_path, Duration::from_millis(500)).unwrap();
        let err = acquire_exclusive(&lock_path, Duration::from_millis(200))
            .expect_err("second acquire must contend with the held lock");
        assert_eq!(err.code(), "MINE_LOCK_TIMEOUT");
        // Releasing the first guard lets a fresh acquisition succeed.
        drop(held);
        let _again = acquire_exclusive(&lock_path, Duration::from_millis(500)).unwrap();
    }

    #[test]
    fn lock_file_parent_created() {
        let dir = tempfile::tempdir().unwrap();
        let lock_path = dir.path().join("locks").join("nested.lock");
        let lock = acquire_exclusive(&lock_path, Duration::from_millis(500)).unwrap();
        assert!(lock_path.exists());
        drop(lock);
    }
}

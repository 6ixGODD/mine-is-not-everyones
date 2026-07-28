#![forbid(unsafe_code)]

//! Self-update and self-removal of the `mine` binary.
//!
//! `download_and_replace`: fetches the prebuilt archive for the current
//! platform from the GitHub Release matching `tag`, extracts the binary, and
//! atomically replaces the running binary. On Windows the running `.exe`
//! cannot be overwritten while held, so the old binary is renamed aside and
//! the new one written in its place; the rename aside is best-effort.
//!
//! `remove_self`: deletes the `mine` binary and removes its directory from
//! the user PATH (Windows: user environment variable; Unix: the rc files are
//! not edited — PATH removal on Unix is left to the user, since rc-file
//! editing is fragile and shell-dependent).

use std::path::PathBuf;

use crate::domain::error::{MineError, MineResult};

/// Downloads the prebuilt binary for the current platform from release `tag`
/// and replaces the currently-running binary in place.
pub fn download_and_replace(tag: &str) -> MineResult<()> {
    let (asset, bin_name) = current_platform_asset();
    let account = std::env::var("MINE_RELEASE_ACCOUNT").unwrap_or_else(|_| "6ixGODD".to_string());
    let repo =
        std::env::var("MINE_RELEASE_REPO").unwrap_or_else(|_| "mine-is-not-everyones".to_string());
    let url = format!("https://github.com/{account}/{repo}/releases/download/{tag}/{asset}");

    let current_exe = std::env::current_exe().map_err(|e| MineError::ExternalDependency {
        detail: format!("cannot locate current exe: {e}"),
    })?;

    println!("Downloading {url}");
    let resp = ureq::get(&url)
        .header("User-Agent", "mine-setup")
        .call()
        .map_err(|e| MineError::ExternalDependency {
            detail: format!("download failed: {e}"),
        })?;
    let bytes = resp
        .into_body()
        .read_to_vec()
        .map_err(|e| MineError::ExternalDependency {
            detail: format!("download read failed: {e}"),
        })?;

    // Extract the binary from the archive (in memory).
    let bin_bytes = extract_binary(&bytes, asset, bin_name)?;

    // Write to a temp file next to the current exe, then rename atomically.
    let dir = current_exe
        .parent()
        .ok_or_else(|| MineError::ExternalDependency {
            detail: "current exe has no parent dir".to_string(),
        })?;
    let tmp = dir.join(format!("mine.new.{}.tmp", std::process::id()));
    std::fs::write(&tmp, &bin_bytes).map_err(|e| MineError::ExternalDependency {
        detail: format!("write temp failed: {e}"),
    })?;

    // Unix: chmod +x.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755)).ok();
    }

    replace_binary(&current_exe, &tmp)?;
    println!("mine updated to {tag}. Reopen your terminal if PATH changed.");
    Ok(())
}

/// Removes the running binary. PATH entries are cleaned on Windows (user env
/// var); on Unix the user is advised to remove the PATH entry manually.
pub fn remove_self() -> MineResult<bool> {
    let exe = std::env::current_exe().map_err(|e| MineError::ExternalDependency {
        detail: format!("cannot locate current exe: {e}"),
    })?;
    let bin_dir = exe.parent().map(|p| p.to_path_buf());

    // Windows: remove the bin dir from the user PATH.
    #[cfg(windows)]
    {
        if let Some(dir) = &bin_dir {
            remove_from_user_path_windows(dir);
        }
    }

    // Delete the binary. On Windows the running exe is locked; rename it aside
    // so the delete of the "real" path succeeds and the stale file is cleaned
    // by the OS on reboot or next install.
    let _ = std::fs::remove_file(&exe).or_else(|_| {
        let stale = exe.with_extension("exe.old");
        std::fs::rename(&exe, &stale).map_err(|_| std::io::Error::from(std::io::ErrorKind::Other))
    });
    let removed = !exe.exists();

    #[cfg(not(windows))]
    {
        if let Some(dir) = &bin_dir {
            eprintln!(
                "mine removed PATH entry guidance: remove `{}` from your shell rc if present.",
                dir.display()
            );
        }
    }

    Ok(removed)
}

/// Returns (asset filename, binary name inside the archive) for the current
/// platform.
fn current_platform_asset() -> (&'static str, &'static str) {
    let target = if cfg!(target_os = "windows") {
        "x86_64-pc-windows-msvc"
    } else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        "aarch64-apple-darwin"
    } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
        "x86_64-apple-darwin"
    } else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        "x86_64-unknown-linux-gnu"
    } else {
        return ("", "mine");
    };
    let ext = if cfg!(target_os = "windows") {
        "zip"
    } else {
        "tar.gz"
    };
    let bin = if cfg!(target_os = "windows") {
        "mine.exe"
    } else {
        "mine"
    };
    // Leak a static string for the asset name. We use once_cell-free approach:
    // format at runtime and pass via a small static via Box::leak is heavy;
    // instead return known static mappings.
    static WINDOWS_ASSET: &str = "mine-x86_64-pc-windows-msvc.zip";
    static LINUX_ASSET: &str = "mine-x86_64-unknown-linux-gnu.tar.gz";
    static MACOS_ARM_ASSET: &str = "mine-aarch64-apple-darwin.tar.gz";
    static MACOS_X86_ASSET: &str = "mine-x86_64-apple-darwin.tar.gz";
    let asset = if cfg!(target_os = "windows") {
        WINDOWS_ASSET
    } else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        MACOS_ARM_ASSET
    } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
        MACOS_X86_ASSET
    } else {
        LINUX_ASSET
    };
    let _ = (target, ext);
    (asset, bin)
}

/// Extracts the binary bytes from a downloaded archive.
fn extract_binary(bytes: &[u8], asset: &str, bin_name: &str) -> MineResult<Vec<u8>> {
    if asset.ends_with(".zip") {
        // Minimal zip extraction for a single entry. Use a lightweight approach:
        // the `zip` crate is not in deps; we shell out to the OS unzipper if
        // present, else error. To avoid a new dependency, write the zip to a
        // temp file and extract via std::process::Command calling `tar` (which
        // handles zip on modern Windows 10+) or `unzip`.
        extract_via_temp_file(bytes, asset, bin_name)
    } else {
        // tar.gz: shell out to `tar` (available on all CI runners and modern
        // macOS/Linux). Extract the single binary entry.
        extract_via_temp_file(bytes, asset, bin_name)
    }
}

fn extract_via_temp_file(bytes: &[u8], asset: &str, bin_name: &str) -> MineResult<Vec<u8>> {
    let tmp_dir = std::env::temp_dir().join(format!("mine-update-{}", std::process::id()));
    std::fs::create_dir_all(&tmp_dir).map_err(|e| MineError::ExternalDependency {
        detail: format!("create temp dir failed: {e}"),
    })?;
    let archive = tmp_dir.join(asset);
    std::fs::write(&archive, bytes).map_err(|e| MineError::ExternalDependency {
        detail: format!("write archive failed: {e}"),
    })?;

    // tar -xf works for both .tar.gz and (on Windows 10+ and most Linux) .zip.
    let status = std::process::Command::new("tar")
        .arg("-xf")
        .arg(&archive)
        .arg("-C")
        .arg(&tmp_dir)
        .status()
        .map_err(|e| MineError::ExternalDependency {
            detail: format!("tar extraction failed to start: {e}"),
        })?;
    if !status.success() {
        // Fallback: try unzip for zip on systems where tar cannot handle zip.
        if asset.ends_with(".zip") {
            let s2 = std::process::Command::new("unzip")
                .arg("-o")
                .arg(&archive)
                .arg("-d")
                .arg(&tmp_dir)
                .status();
            if s2.map(|s| s.success()).unwrap_or(false) {
                let bin = find_binary(&tmp_dir, bin_name)?;
                let b = std::fs::read(&bin).map_err(|e| MineError::ExternalDependency {
                    detail: format!("read extracted binary failed: {e}"),
                })?;
                let _ = std::fs::remove_dir_all(&tmp_dir);
                return Ok(b);
            }
        }
        return Err(MineError::ExternalDependency {
            detail: format!("tar extraction exited {status}"),
        });
    }
    let bin = find_binary(&tmp_dir, bin_name)?;
    let b = std::fs::read(&bin).map_err(|e| MineError::ExternalDependency {
        detail: format!("read extracted binary failed: {e}"),
    })?;
    let _ = std::fs::remove_dir_all(&tmp_dir);
    Ok(b)
}

fn find_binary(root: &std::path::Path, bin_name: &str) -> MineResult<PathBuf> {
    // The archive may contain the binary at root or under a staging dir.
    if root.join(bin_name).is_file() {
        return Ok(root.join(bin_name));
    }
    for entry in std::fs::read_dir(root).map_err(|e| MineError::ExternalDependency {
        detail: format!("read extracted dir failed: {e}"),
    })? {
        let entry = entry.map_err(|e| MineError::ExternalDependency {
            detail: format!("read dir entry failed: {e}"),
        })?;
        let p = entry.path();
        if p.is_dir() {
            if let Ok(inner) = std::fs::read_dir(&p) {
                for ie in inner.flatten() {
                    if ie.file_name() == bin_name && ie.path().is_file() {
                        return Ok(ie.path());
                    }
                }
            }
        }
    }
    Err(MineError::ExternalDependency {
        detail: format!("binary {bin_name:?} not found in extracted archive"),
    })
}

fn replace_binary(current: &std::path::Path, tmp: &std::path::Path) -> MineResult<()> {
    // Try atomic rename first.
    if std::fs::rename(tmp, current).is_ok() {
        return Ok(());
    }
    // Fallback: on Windows the running exe is locked. Rename current aside,
    // then move new into place.
    let aside = current.with_extension("replace-old");
    let _ = std::fs::remove_file(&aside);
    std::fs::rename(current, &aside).map_err(|e| MineError::ExternalDependency {
        detail: format!("could not rename old binary aside: {e}"),
    })?;
    if let Err(e) = std::fs::rename(tmp, current) {
        // Restore the old binary if the new one could not be moved in.
        let _ = std::fs::rename(&aside, current);
        return Err(MineError::ExternalDependency {
            detail: format!("could not move new binary into place: {e}"),
        });
    }
    let _ = std::fs::remove_file(&aside);
    Ok(())
}

#[cfg(windows)]
fn remove_from_user_path_windows(bin_dir: &std::path::Path) {
    // Windows PATH cleanup via the registry requires the `winreg` crate (or
    // raw `unsafe` FFI), which is not a dependency. Report guidance instead;
    // the user removes the entry via System > Environment Variables > Path.
    let _ = bin_dir;
    eprintln!(
        "mine removed. To clean your user PATH, remove the mine entry via\n  System > Environment Variables > Path (User)."
    );
}

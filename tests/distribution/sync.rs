// Enforce no `unsafe` in MINE-owned test crates.
#![forbid(unsafe_code)]

//! Synchronization tests: verify the sync algorithm is deterministic,
//! idempotent, removes stale MINE-owned generated files, preserves unrelated
//! files, and detects drift. Algorithm tests run in isolated temp directories;
//! the real script is exercised via subprocess with `--root <temp>` so no real
//! repository files are ever modified.

use super::common::*;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Builds an isolated temp source tree with two skill files.
fn temp_source() -> (TempDir, PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("skills");
    std::fs::create_dir_all(src.join("mine-arch/references")).unwrap();
    std::fs::write(src.join("mine-arch/SKILL.md"), "arch\n").unwrap();
    std::fs::write(src.join("mine-arch/references/outline.md"), "outline\n").unwrap();
    std::fs::create_dir_all(src.join("mine-sync")).unwrap();
    std::fs::write(src.join("mine-sync/SKILL.md"), "sync\n").unwrap();
    (tmp, src)
}

#[test]
fn sync_copies_all_files_byte_for_byte() {
    let (tmp, src) = temp_source();
    let dst = tmp.path().join("plugins/mine/skills");
    sync_to(&src, &dst);
    assert!(
        check_sync(&src, &dst).is_ok(),
        "after sync, dst must match src"
    );
    // Verify content (binary-faithful, stable line endings).
    assert_eq!(
        std::fs::read(dst.join("mine-arch/SKILL.md")).unwrap(),
        b"arch\n"
    );
}

#[test]
fn sync_is_idempotent() {
    let (tmp, src) = temp_source();
    let dst = tmp.path().join("plugins/mine/skills");
    sync_to(&src, &dst);
    // Snapshot the destination after the first sync.
    let snap1: std::collections::HashMap<String, Vec<u8>> = rel_files(&dst)
        .into_iter()
        .map(|r| (r.clone(), std::fs::read(dst.join(&r)).unwrap()))
        .collect();
    // Sync again - no changes expected.
    sync_to(&src, &dst);
    let snap2: std::collections::HashMap<String, Vec<u8>> = rel_files(&dst)
        .into_iter()
        .map(|r| (r.clone(), std::fs::read(dst.join(&r)).unwrap()))
        .collect();
    assert_eq!(
        snap1, snap2,
        "second sync must produce identical output (idempotent)"
    );
}

#[test]
fn sync_removes_stale_generated_files() {
    let (tmp, src) = temp_source();
    let dst = tmp.path().join("plugins/mine/skills");
    sync_to(&src, &dst);
    // Add a stale file that no longer exists in the source.
    std::fs::create_dir_all(dst.join("mine-obsolete")).unwrap();
    std::fs::write(dst.join("mine-obsolete/SKILL.md"), "stale\n").unwrap();
    // Re-sync: the stale file must be removed.
    sync_to(&src, &dst);
    assert!(
        !dst.join("mine-obsolete/SKILL.md").exists(),
        "stale generated file must be removed"
    );
    assert!(check_sync(&src, &dst).is_ok());
}

#[test]
fn sync_preserves_unrelated_files_outside_skills_tree() {
    let (tmp, src) = temp_source();
    let plugin_root = tmp.path().join("plugins/mine");
    let dst = plugin_root.join("skills");
    // Place an unrelated file in the plugin root (not under skills/).
    std::fs::create_dir_all(&plugin_root).unwrap();
    std::fs::write(plugin_root.join("plugin.json"), "{}\n").unwrap();
    std::fs::write(plugin_root.join("GENERATED.md"), "generated\n").unwrap();
    sync_to(&src, &dst);
    // The unrelated files must survive the sync.
    assert!(
        plugin_root.join("plugin.json").exists(),
        "unrelated plugin.json must be preserved"
    );
    assert!(
        plugin_root.join("GENERATED.md").exists(),
        "unrelated GENERATED.md must be preserved"
    );
}

#[test]
fn check_detects_drift_when_generated_differs() {
    let (tmp, src) = temp_source();
    let dst = tmp.path().join("plugins/mine/skills");
    sync_to(&src, &dst);
    // Introduce drift: modify a generated file.
    std::fs::write(dst.join("mine-arch/SKILL.md"), "TAMPERED\n").unwrap();
    assert!(
        check_sync(&src, &dst).is_err(),
        "check must fail when a generated file differs from source"
    );
}

#[test]
fn check_detects_missing_generated_file() {
    let (tmp, src) = temp_source();
    let dst = tmp.path().join("plugins/mine/skills");
    sync_to(&src, &dst);
    // Remove a generated file -> drift.
    std::fs::remove_file(dst.join("mine-sync/SKILL.md")).unwrap();
    assert!(
        check_sync(&src, &dst).is_err(),
        "check must fail when a generated file is missing"
    );
}

#[test]
fn check_detects_stale_extra_generated_file() {
    let (tmp, src) = temp_source();
    let dst = tmp.path().join("plugins/mine/skills");
    sync_to(&src, &dst);
    // Add an extra file not in source -> drift.
    std::fs::write(dst.join("extra.txt"), "extra\n").unwrap();
    assert!(
        check_sync(&src, &dst).is_err(),
        "check must fail when a stale extra file is present"
    );
}

/// Runs the real sync script against a temp root (write mode).
fn run_sync_script(root: &Path) -> std::process::Output {
    std::process::Command::new("python")
        .arg(repo_root().join("scripts/sync-plugin-assets.py"))
        .arg("--root")
        .arg(root)
        .current_dir(repo_root())
        .output()
        .expect("run sync script")
}

/// Runs the real sync script against a temp root (check mode).
fn run_check_script(root: &Path) -> std::process::Output {
    std::process::Command::new("python")
        .arg(repo_root().join("scripts/sync-plugin-assets.py"))
        .arg("--check")
        .arg("--root")
        .arg(root)
        .current_dir(repo_root())
        .output()
        .expect("run sync script")
}

#[test]
fn real_sync_script_copies_files_in_temp_root() {
    let (tmp, src) = temp_source();
    // The script expects skills/ under the root.
    let root = tmp.path();
    let dst = root.join("plugins/mine/skills");
    let out = run_sync_script(root);
    assert!(
        out.status.success(),
        "sync must succeed; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(dst.join("mine-arch/SKILL.md").exists());
    assert!(dst.join("mine-sync/SKILL.md").exists());
    // Byte-equivalence.
    assert_eq!(
        std::fs::read(dst.join("mine-arch/SKILL.md")).unwrap(),
        std::fs::read(src.join("mine-arch/SKILL.md")).unwrap()
    );
}

#[test]
fn real_sync_script_check_passes_when_in_sync_in_temp_root() {
    let (tmp, _src) = temp_source();
    let root = tmp.path();
    run_sync_script(root);
    let out = run_check_script(root);
    assert!(
        out.status.success(),
        "check must pass when in sync; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn real_sync_script_check_detects_drift_in_temp_root() {
    let (tmp, _src) = temp_source();
    let root = tmp.path();
    run_sync_script(root);
    // Introduce drift in the isolated temp copy (never the real repo).
    let target = root.join("plugins/mine/skills/mine-sync/SKILL.md");
    std::fs::write(&target, "TAMPERED\n").unwrap();
    let out = run_check_script(root);
    assert!(
        !out.status.success(),
        "check must fail on drift; stdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}

#[test]
fn real_sync_script_removes_stale_in_temp_root() {
    let (tmp, _src) = temp_source();
    let root = tmp.path();
    run_sync_script(root);
    let dst = root.join("plugins/mine/skills");
    // Add a stale file.
    std::fs::create_dir_all(dst.join("mine-obsolete")).unwrap();
    std::fs::write(dst.join("mine-obsolete/SKILL.md"), "stale\n").unwrap();
    // Re-sync.
    run_sync_script(root);
    assert!(
        !dst.join("mine-obsolete/SKILL.md").exists(),
        "sync must remove stale generated files"
    );
}

#[test]
fn real_sync_script_preserves_unrelated_files_in_temp_root() {
    let (tmp, _src) = temp_source();
    let root = tmp.path();
    let plugin_root = root.join("plugins/mine");
    std::fs::create_dir_all(&plugin_root).unwrap();
    std::fs::write(plugin_root.join("plugin.json"), "{}\n").unwrap();
    run_sync_script(root);
    assert!(
        plugin_root.join("plugin.json").exists(),
        "sync must preserve unrelated files outside skills/"
    );
}

#[test]
fn real_sync_script_reports_in_sync_for_repository() {
    // Exercise the real scripts/sync-plugin-assets.py --check against the
    // repository (read-only). This must exit 0 (generated copies are in sync).
    let script = repo_root().join("scripts/sync-plugin-assets.py");
    assert!(script.exists(), "sync script must exist");
    let output = std::process::Command::new("python")
        .arg(&script)
        .arg("--check")
        .current_dir(repo_root())
        .output()
        .expect("run sync script");
    assert!(
        output.status.success(),
        "sync --check must pass for the repository; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

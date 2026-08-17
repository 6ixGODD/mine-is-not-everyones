// Enforce no `unsafe` in MINE-owned test crates.
#![forbid(unsafe_code)]

//! Skill-refresh-on-update and Pi skill-deduplication integration tests.
//!
//! Every test drives the real CLI against an isolated temporary
//! configuration root; no test touches the real user HOME.

use std::path::Path;

use mine::agent_setup::targets::Env;
use mine::application::agent_service::refresh_all_installed;

use super::common::*;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

use std::path::PathBuf;

/// Reads the managed-state JSON for `agent` under the isolated root.
fn record_of(root: &Path, agent: &str) -> serde_json::Value {
    let raw = std::fs::read_to_string(root.join(".mine/agent-installs.json"))
        .unwrap_or_else(|_| panic!("managed state missing"));
    let state: serde_json::Value = serde_json::from_str(&raw).expect("valid managed state");
    state["installs"]
        .as_array()
        .and_then(|a| a.iter().find(|r| r["agent"] == agent).cloned())
        .unwrap_or_else(|| panic!("no record for {agent}"))
}

#[test]
fn refresh_all_installed_rewrites_skills_and_updates_record() {
    let repo = repo_root();
    let tmp = tempfile::tempdir().unwrap();
    // Install claude-code (isolated root).
    dispatch_agent(&repo, tmp.path(), &["agent", "install", "claude-code"]);
    let rec_before = record_of(tmp.path(), "claude-code");
    assert_eq!(rec_before["mine_version"], env!("CARGO_PKG_VERSION"));

    // Corrupt one managed skill file to simulate drift.
    let skill_path = tmp.path().join(".claude/skills/mine-arch/SKILL.md");
    std::fs::write(&skill_path, "corrupted\n").unwrap();

    // Refresh: files must be rewritten from the embedded payload and the
    // record must remain current.
    let env = Env::isolated(tmp.path().to_path_buf());
    let report = refresh_all_installed(&env, env!("CARGO_PKG_VERSION"));
    assert_eq!(report.refreshed, vec!["claude-code".to_string()]);
    assert!(report.errors.is_empty(), "errors: {:?}", report.errors);

    let restored = std::fs::read_to_string(&skill_path).unwrap();
    // The rewritten file must match the current embedded payload byte-for-byte.
    let embedded = mine::infrastructure::embedded_skills::EMBEDDED_SKILL_FILES
        .iter()
        .find(|f| f.path.ends_with("mine-arch/SKILL.md"))
        .map(|f| f.content)
        .unwrap();
    assert_eq!(
        restored, embedded,
        "skill must be rewritten from the embedded payload"
    );
    let rec_after = record_of(tmp.path(), "claude-code");
    assert_eq!(rec_after["mine_version"], env!("CARGO_PKG_VERSION"));
}

#[test]
fn refresh_with_no_managed_records_is_noop() {
    let tmp = tempfile::tempdir().unwrap();
    let env = Env::isolated(tmp.path().to_path_buf());
    let report = refresh_all_installed(&env, env!("CARGO_PKG_VERSION"));
    assert!(report.refreshed.is_empty());
    assert!(report.errors.is_empty(), "errors: {:?}", report.errors);
}

#[test]
fn refresh_failure_is_reported_and_leaves_record_unchanged() {
    let repo = repo_root();
    let tmp = tempfile::tempdir().unwrap();
    dispatch_agent(&repo, tmp.path(), &["agent", "install", "opencode"]);

    // Replace a managed skill file with a non-MINE-owned file (simulate a
    // user file that took over the path): refresh must refuse the collision.
    let skill_path = tmp
        .path()
        .join(".config/opencode/skills/mine-sync/SKILL.md");
    std::fs::write(&skill_path, "user file\n").unwrap();
    // Make the managed-state record NOT list this file so it is treated as
    // not-MINE-owned -> collision.
    let state_path = tmp.path().join(".mine/agent-installs.json");
    let mut state: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&state_path).unwrap()).unwrap();
    for inst in state["installs"].as_array_mut().unwrap() {
        if inst["agent"] == "opencode" {
            inst["files"] = serde_json::json!([]);
        }
    }
    std::fs::write(&state_path, serde_json::to_string_pretty(&state).unwrap()).unwrap();
    // Baseline is the post-modification record: a failed refresh must leave it
    // exactly as it was after the modification.
    let rec_before = record_of(tmp.path(), "opencode");

    let env = Env::isolated(tmp.path().to_path_buf());
    let report = refresh_all_installed(&env, env!("CARGO_PKG_VERSION"));
    assert!(
        report.errors.iter().any(|e| e.contains("opencode")),
        "refresh must report the failure: {:?}",
        report.errors
    );
    assert!(report.refreshed.is_empty());
    let rec_after = record_of(tmp.path(), "opencode");
    assert_eq!(
        rec_after["files"], rec_before["files"],
        "failed refresh must not change the record"
    );
}

#[test]
fn pi_uses_shared_skills_when_codex_installed_first() {
    let repo = repo_root();
    let tmp = tempfile::tempdir().unwrap();
    // Codex first: shared ~/.agents/skills gets the complete MINE set.
    dispatch_agent(&repo, tmp.path(), &["agent", "install", "codex"]);
    assert!(
        tmp.path()
            .join(".agents/skills/mine-arch/SKILL.md")
            .is_file()
    );

    // Pi second: must use the shared set, not ~/.pi/agent/skills.
    dispatch_agent(&repo, tmp.path(), &["agent", "install", "pi"]);
    let rec = record_of(tmp.path(), "pi");
    let files: Vec<String> = rec["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["path"].as_str().unwrap().to_string())
        .collect();
    assert!(
        files.iter().all(|f| f.starts_with(".agents/skills/")),
        "pi record must point at shared skills, got: {files:?}"
    );
    assert!(
        !tmp.path()
            .join(".pi/agent/skills/mine-arch/SKILL.md")
            .exists(),
        "pi must not install its own copy when the shared set exists"
    );
}

#[test]
fn pi_install_removes_legacy_own_skills_when_shared_set_appears() {
    let repo = repo_root();
    let tmp = tempfile::tempdir().unwrap();
    // Pi first, no shared set: installs into ~/.pi/agent/skills.
    dispatch_agent(&repo, tmp.path(), &["agent", "install", "pi"]);
    assert!(
        tmp.path()
            .join(".pi/agent/skills/mine-arch/SKILL.md")
            .is_file()
    );

    // Codex second: shared set appears.
    dispatch_agent(&repo, tmp.path(), &["agent", "install", "codex"]);
    assert!(
        tmp.path()
            .join(".agents/skills/mine-arch/SKILL.md")
            .is_file()
    );

    // Re-running setup for pi must move it to the shared set and remove the
    // legacy own copy.
    dispatch_agent(&repo, tmp.path(), &["agent", "install", "pi"]);
    let rec = record_of(tmp.path(), "pi");
    let files: Vec<String> = rec["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["path"].as_str().unwrap().to_string())
        .collect();
    assert!(
        files.iter().all(|f| f.starts_with(".agents/skills/")),
        "pi record must point at shared skills after reinstall: {files:?}"
    );
    assert!(
        !tmp.path()
            .join(".pi/agent/skills/mine-arch/SKILL.md")
            .exists(),
        "legacy pi skill copy must be removed"
    );
}

#[test]
fn pi_install_without_shared_set_uses_own_dir() {
    let repo = repo_root();
    let tmp = tempfile::tempdir().unwrap();
    dispatch_agent(&repo, tmp.path(), &["agent", "install", "pi"]);
    let rec = record_of(tmp.path(), "pi");
    let files: Vec<String> = rec["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["path"].as_str().unwrap().to_string())
        .collect();
    assert!(
        files.iter().all(|f| f.starts_with(".pi/agent/skills/")),
        "pi must use its own dir when no shared set exists: {files:?}"
    );
}

#[test]
fn pi_uninstall_does_not_break_codex_shared_skills() {
    let repo = repo_root();
    let tmp = tempfile::tempdir().unwrap();
    dispatch_agent(&repo, tmp.path(), &["agent", "install", "codex"]);
    dispatch_agent(&repo, tmp.path(), &["agent", "install", "pi"]);

    // Uninstall pi: shared files must survive (codex still owns them).
    dispatch_agent(&repo, tmp.path(), &["agent", "uninstall", "pi"]);
    assert!(
        tmp.path()
            .join(".agents/skills/mine-arch/SKILL.md")
            .is_file(),
        "codex's shared skill must survive pi uninstall"
    );
    // Pi record must be gone.
    let raw = std::fs::read_to_string(tmp.path().join(".mine/agent-installs.json")).unwrap();
    let state: serde_json::Value = serde_json::from_str(&raw).unwrap();
    assert!(
        !state["installs"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["agent"] == "pi"),
        "pi record must be removed"
    );
}

#[test]
fn doctor_healthy_for_pi_with_shared_skills() {
    let repo = repo_root();
    let tmp = tempfile::tempdir().unwrap();
    dispatch_agent(&repo, tmp.path(), &["agent", "install", "codex"]);
    dispatch_agent(&repo, tmp.path(), &["agent", "install", "pi"]);
    let env = dispatch_doctor(&repo, tmp.path(), "all");
    let diags = env["data"]["agents"]["diagnostics"].as_array().unwrap();
    let pi = diags.iter().find(|d| d["agent"] == "pi").unwrap();
    assert_eq!(
        pi["status"], "healthy",
        "pi with shared skills must be healthy: {pi}"
    );
}

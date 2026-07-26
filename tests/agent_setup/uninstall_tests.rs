// Enforce no `unsafe` in MINE-owned test crates.
#![forbid(unsafe_code)]

//! Uninstall integration tests (ported + reworked for isolated Env).

use super::common::*;

#[test]
fn clean_uninstall_removes_owned_preserves_unrelated() {
    let repo = repo_root();
    let tmp = tempfile::tempdir().unwrap();
    dispatch_agent(&repo, tmp.path(), &["agent", "install", "claude-code"]);
    std::fs::write(
        tmp.path().join(".claude/skills/user-notes.md"),
        "user owned",
    )
    .unwrap();
    let (outcome, env) = dispatch_agent(&repo, tmp.path(), &["agent", "uninstall", "claude-code"]);
    assert_eq!(outcome.exit_code, 0, "uninstall failed: {env}");
    assert!(
        !tmp.path()
            .join(".claude/skills/mine-arch/SKILL.md")
            .exists()
    );
    assert!(
        tmp.path().join(".claude/skills/user-notes.md").exists(),
        "unrelated file preserved"
    );
    let cfg = serde_json::from_str::<serde_json::Value>(
        &std::fs::read_to_string(tmp.path().join(".claude.json")).unwrap(),
    )
    .unwrap();
    assert!(
        cfg.get("mcpServers").and_then(|m| m.get("mine")).is_none(),
        "MINE entry removed"
    );
}

#[test]
fn uninstall_refuses_without_managed_record() {
    let repo = repo_root();
    let tmp = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(tmp.path().join(".pi/agent/skills/mine-sync")).unwrap();
    std::fs::write(
        tmp.path().join(".pi/agent/skills/mine-sync/SKILL.md"),
        "fake",
    )
    .unwrap();
    let (outcome, env) = dispatch_agent(&repo, tmp.path(), &["agent", "uninstall", "pi"]);
    assert_ne!(outcome.exit_code, 0, "must refuse without managed record");
    assert_eq!(env["error"]["code"], "MINE_AGENT_MANAGED_STATE_INVALID");
    assert!(
        tmp.path()
            .join(".pi/agent/skills/mine-sync/SKILL.md")
            .exists(),
        "fake preserved"
    );
}

#[test]
fn uninstall_preserves_drifted_files() {
    let repo = repo_root();
    let tmp = tempfile::tempdir().unwrap();
    dispatch_agent(&repo, tmp.path(), &["agent", "install", "codex"]);
    let f = tmp.path().join(".agents/skills/mine-arch/SKILL.md");
    std::fs::write(&f, "USER EDITED").unwrap();
    let (outcome, env) = dispatch_agent(&repo, tmp.path(), &["agent", "uninstall", "codex"]);
    assert_eq!(outcome.exit_code, 0);
    assert!(
        env["data"]["drifted_files"]
            .as_array()
            .is_some_and(|d| !d.is_empty()),
        "drifted files reported, not deleted"
    );
    assert!(f.exists(), "drifted file preserved");
}

#[test]
fn uninstall_handles_missing_files() {
    let repo = repo_root();
    let tmp = tempfile::tempdir().unwrap();
    dispatch_agent(&repo, tmp.path(), &["agent", "install", "opencode"]);
    std::fs::remove_file(
        tmp.path()
            .join(".config/opencode/skills/mine-sync/SKILL.md"),
    )
    .unwrap();
    let (outcome, _e) = dispatch_agent(&repo, tmp.path(), &["agent", "uninstall", "opencode"]);
    assert_eq!(outcome.exit_code, 0);
}

#[test]
fn dry_run_uninstall_removes_nothing() {
    let repo = repo_root();
    let tmp = tempfile::tempdir().unwrap();
    dispatch_agent(&repo, tmp.path(), &["agent", "install", "pi"]);
    let (outcome, _e) = dispatch_agent(
        &repo,
        tmp.path(),
        &["agent", "uninstall", "pi", "--dry-run"],
    );
    assert_eq!(outcome.exit_code, 0);
    assert!(
        tmp.path()
            .join(".pi/agent/skills/mine-sync/SKILL.md")
            .exists()
    );
}

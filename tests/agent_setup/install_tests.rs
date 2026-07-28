// Enforce no `unsafe` in MINE-owned test crates.
#![forbid(unsafe_code)]

//! Fix 1 tests: mandatory exact-byte backup before config mutation;
//! comment/formatting preservation for Codex TOML; idempotency; collision.

use super::common::*;

#[test]
fn clean_install_for_all_four_agents() {
    let repo = repo_root();
    for slug in FOUR_AGENTS {
        let tmp = tempfile::tempdir().unwrap();
        let (outcome, env) = dispatch_agent(&repo, tmp.path(), &["agent", "install", slug]);
        assert_eq!(outcome.exit_code, 0, "install {slug} failed: {env}");
        assert_eq!(env["ok"], true);
        assert_eq!(env["data"]["agent"].as_str(), Some(*slug));
        assert_eq!(
            env["data"]["skills_installed"].as_u64(),
            Some(5),
            "{slug}: all 5 skills"
        );
    }
}

#[test]
fn codex_backup_before_mutation_exact_bytes() {
    let repo = repo_root();
    let tmp = tempfile::tempdir().unwrap();
    let cfg = tmp.path().join(".codex/config.toml");
    std::fs::create_dir_all(cfg.parent().unwrap()).unwrap();
    let original = b"# comment\n[mcp_servers]\nexisting = true\n";
    std::fs::write(&cfg, original).unwrap();
    let (outcome, env) = dispatch_agent(&repo, tmp.path(), &["agent", "install", "codex"]);
    assert_eq!(outcome.exit_code, 0, "{env}");
    // Backup exists and matches the original bytes.
    let backup_path = env["data"]["backup"]["backup_path"]
        .as_str()
        .expect("backup_path");
    let backup_bytes = std::fs::read(backup_path).unwrap();
    assert_eq!(
        backup_bytes, original,
        "backup is exact bytes, not reserialized"
    );
}

#[test]
fn codex_preserves_comments_and_unrelated_keys() {
    let repo = repo_root();
    let tmp = tempfile::tempdir().unwrap();
    let cfg = tmp.path().join(".codex/config.toml");
    std::fs::create_dir_all(cfg.parent().unwrap()).unwrap();
    let original = "# user comment\n[mcp_servers]\n# another\nexisting = true\n[to_keep]\nx = 1\n";
    std::fs::write(&cfg, original).unwrap();
    dispatch_agent(&repo, tmp.path(), &["agent", "install", "codex"]);
    let after = std::fs::read_to_string(&cfg).unwrap();
    assert!(after.contains("# user comment"), "comment preserved");
    assert!(after.contains("[to_keep]"), "unrelated table preserved");
    assert!(after.contains("existing = true"), "unrelated key preserved");
    assert!(after.contains("[mcp_servers.mine]"), "MINE table added");
    assert!(after.contains("command = \"mine\""));
    assert!(after.contains("enabled = true"));
}

#[test]
fn claude_json_backup_preserves_unrelated_keys() {
    let repo = repo_root();
    let tmp = tempfile::tempdir().unwrap();
    let cfg = tmp.path().join(".claude.json");
    let original = r#"{"user-theme":"dark","mcpServers":{"other":{"command":"keepme"}}}"#;
    std::fs::write(&cfg, original).unwrap();
    let (outcome, env) = dispatch_agent(&repo, tmp.path(), &["agent", "install", "claude-code"]);
    assert_eq!(outcome.exit_code, 0, "{env}");
    let after =
        serde_json::from_str::<serde_json::Value>(&std::fs::read_to_string(&cfg).unwrap()).unwrap();
    assert_eq!(after["user-theme"], "dark", "unrelated key preserved");
    assert_eq!(
        after["mcpServers"]["other"]["command"], "keepme",
        "unrelated MCP server preserved"
    );
    assert_eq!(
        after["mcpServers"]["mine"]["command"], "mine",
        "MINE entry merged"
    );
    // Backup matches the original.
    let backup_path = env["data"]["backup"]["backup_path"]
        .as_str()
        .expect("backup_path");
    assert_eq!(
        std::fs::read(backup_path).unwrap(),
        original.as_bytes(),
        "exact-byte backup"
    );
}

#[test]
fn install_is_idempotent() {
    let repo = repo_root();
    let tmp = tempfile::tempdir().unwrap();
    dispatch_agent(&repo, tmp.path(), &["agent", "install", "codex"]);
    // Exclude `.mine/agent-backups/` (legitimately created on update when a
    // previously-MINE-created config exists and is mutated again).
    let snap: Vec<String> = files_under(tmp.path())
        .into_iter()
        .filter(|f| !f.starts_with(".mine/agent-backups/"))
        .collect();
    let (outcome, _e) = dispatch_agent(&repo, tmp.path(), &["agent", "install", "codex"]);
    assert_eq!(outcome.exit_code, 0);
    let snap2: Vec<String> = files_under(tmp.path())
        .into_iter()
        .filter(|f| !f.starts_with(".mine/agent-backups/"))
        .collect();
    assert_eq!(
        snap, snap2,
        "idempotent install changes no files (excluding backups)"
    );
}

#[test]
fn collision_refused_no_partial_managed_state() {
    let repo = repo_root();
    let tmp = tempfile::tempdir().unwrap();
    let dest = tmp.path().join(".claude/skills/mine-arch/SKILL.md");
    std::fs::create_dir_all(dest.parent().unwrap()).unwrap();
    std::fs::write(&dest, "USER OWNED").unwrap();
    let (outcome, env) = dispatch_agent(&repo, tmp.path(), &["agent", "install", "claude-code"]);
    assert_ne!(outcome.exit_code, 0, "must refuse collision");
    assert_eq!(env["error"]["code"], "MINE_AGENT_COLLISION");
    assert_eq!(
        std::fs::read_to_string(&dest).unwrap(),
        "USER OWNED",
        "user file preserved"
    );
    assert!(
        !tmp.path().join(".mine/agent-installs.json").exists(),
        "no managed state on failure"
    );
}

#[test]
fn dry_run_writes_nothing() {
    let repo = repo_root();
    let tmp = tempfile::tempdir().unwrap();
    let (outcome, env) =
        dispatch_agent(&repo, tmp.path(), &["agent", "install", "pi", "--dry-run"]);
    assert_eq!(outcome.exit_code, 0);
    assert_eq!(env["data"]["skills_installed"].as_u64(), Some(5));
    assert!(!tmp.path().join(".pi/agent/skills").exists());
    assert!(!tmp.path().join(".mine/agent-installs.json").exists());
}

#[test]
fn mcp_entry_uses_real_mine_mcp_serve() {
    let repo = repo_root();
    let tmp = tempfile::tempdir().unwrap();
    dispatch_agent(&repo, tmp.path(), &["agent", "install", "opencode"]);
    let cfg = serde_json::from_str::<serde_json::Value>(
        &std::fs::read_to_string(tmp.path().join(".config/opencode/opencode.json")).unwrap(),
    )
    .unwrap();
    let entry = &cfg["mcp"]["mine"];
    assert_eq!(entry["type"], "local");
    assert_eq!(entry["enabled"], true);
    let cmd = entry["command"].as_array().unwrap();
    assert_eq!(cmd[0], "mine");
    assert_eq!(cmd[1], "mcp");
    assert_eq!(cmd[2], "serve");
}

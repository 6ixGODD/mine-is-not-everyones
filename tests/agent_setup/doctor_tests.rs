// Enforce no `unsafe` in MINE-owned test crates.
#![forbid(unsafe_code)]

//! Doctor diagnostics integration tests (ported + reworked for isolated Env).

use super::common::*;

#[test]
fn doctor_reports_not_installed_when_absent() {
    let repo = repo_root();
    let tmp = tempfile::tempdir().unwrap();
    let env = dispatch_doctor(&repo, tmp.path(), "all");
    let diags = env["data"]["agents"]["diagnostics"].as_array().unwrap();
    for d in diags {
        assert_eq!(
            d["status"], "agent_not_detected",
            "absent agent must be not-detected"
        );
    }
}

#[test]
fn doctor_healthy_after_install() {
    let repo = repo_root();
    let tmp = tempfile::tempdir().unwrap();
    dispatch_agent(&repo, tmp.path(), &["agent", "install", "claude-code"]);
    let env = dispatch_doctor(&repo, tmp.path(), "all");
    let diags = env["data"]["agents"]["diagnostics"].as_array().unwrap();
    let claude = diags.iter().find(|d| d["agent"] == "claude-code").unwrap();
    assert_eq!(claude["status"], "healthy");
    assert_eq!(claude["mcp_registered"], true);
}

#[test]
fn doctor_detects_drifted_files() {
    let repo = repo_root();
    let tmp = tempfile::tempdir().unwrap();
    dispatch_agent(&repo, tmp.path(), &["agent", "install", "pi"]);
    std::fs::write(
        tmp.path().join(".pi/agent/skills/mine-sync/SKILL.md"),
        "EDITED",
    )
    .unwrap();
    let env = dispatch_doctor(&repo, tmp.path(), "pi");
    let d = &env["data"]["agents"]["diagnostics"][0];
    assert_eq!(d["status"], "drifted_files");
}

#[test]
fn doctor_detects_missing_files() {
    let repo = repo_root();
    let tmp = tempfile::tempdir().unwrap();
    dispatch_agent(&repo, tmp.path(), &["agent", "install", "codex"]);
    std::fs::remove_file(tmp.path().join(".agents/skills/mine-arch/SKILL.md")).unwrap();
    let env = dispatch_doctor(&repo, tmp.path(), "codex");
    let d = &env["data"]["agents"]["diagnostics"][0];
    assert_eq!(d["status"], "missing_files");
}

#[test]
fn doctor_detects_stale_version() {
    let repo = repo_root();
    let tmp = tempfile::tempdir().unwrap();
    dispatch_agent(&repo, tmp.path(), &["agent", "install", "opencode"]);
    let p = tmp.path().join(".mine/agent-installs.json");
    let raw = std::fs::read_to_string(&p).unwrap();
    let current_ver = env!("CARGO_PKG_VERSION");
    let needle = format!(r#""mine_version":"{current_ver}""#);
    let stale = raw.replace(&needle, r#""mine_version":"0.0.0""#);
    std::fs::write(&p, stale).unwrap();
    let env = dispatch_doctor(&repo, tmp.path(), "opencode");
    let d = &env["data"]["agents"]["diagnostics"][0];
    assert_eq!(d["status"], "stale_version");
}

#[test]
fn doctor_detects_incorrect_mcp_entry() {
    let repo = repo_root();
    let tmp = tempfile::tempdir().unwrap();
    dispatch_agent(&repo, tmp.path(), &["agent", "install", "claude-code"]);
    std::fs::write(
        tmp.path().join(".claude.json"),
        r#"{"mcpServers":{"mine":{"command":"wrong","args":["x"]}}}"#,
    )
    .unwrap();
    let env = dispatch_doctor(&repo, tmp.path(), "claude-code");
    let d = &env["data"]["agents"]["diagnostics"][0];
    assert_eq!(d["status"], "mcp_registration_missing_or_incorrect");
}

#[test]
fn doctor_reports_malformed_managed_state() {
    let repo = repo_root();
    let tmp = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(tmp.path().join(".mine")).unwrap();
    std::fs::write(
        tmp.path().join(".mine/agent-installs.json"),
        "{not valid json",
    )
    .unwrap();
    let env = dispatch_doctor(&repo, tmp.path(), "all");
    assert_eq!(env["data"]["agents"]["malformed_state"], true);
}

#[test]
fn doctor_single_vs_all_scope() {
    let repo = repo_root();
    let tmp = tempfile::tempdir().unwrap();
    dispatch_agent(&repo, tmp.path(), &["agent", "install", "pi"]);
    let single = dispatch_doctor(&repo, tmp.path(), "pi");
    assert_eq!(
        single["data"]["agents"]["diagnostics"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let all = dispatch_doctor(&repo, tmp.path(), "all");
    assert_eq!(
        all["data"]["agents"]["diagnostics"]
            .as_array()
            .unwrap()
            .len(),
        4
    );
}

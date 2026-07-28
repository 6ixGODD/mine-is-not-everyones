// Enforce no `unsafe` in MINE-owned test crates.
#![forbid(unsafe_code)]

//! Four-Agent isolated E2E: install all four Agents into isolated temp roots,
//! run doctor, verify no real HOME mutation.

use mine::cli;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn run_agent(
    repo: &std::path::Path,
    config_root: &std::path::Path,
    rest: &[&str],
) -> (i32, serde_json::Value) {
    let mut argv = vec![
        "mine".to_string(),
        "--repo".to_string(),
        repo.to_string_lossy().to_string(),
    ];
    for t in rest {
        argv.push(t.to_string());
    }
    argv.push("--config-root".to_string());
    argv.push(config_root.to_string_lossy().to_string());
    let outcome = cli::dispatch(&argv, "mine");
    let (stdout, stderr) = cli::render(&outcome, true, false);
    let body = if outcome.exit_code == 0 {
        stdout
    } else {
        stderr
    };
    (
        outcome.exit_code,
        serde_json::from_str(&body).unwrap_or(serde_json::Value::Null),
    )
}

fn run_doctor(
    repo: &std::path::Path,
    config_root: &std::path::Path,
    agents: &str,
) -> serde_json::Value {
    let argv = vec![
        "mine".to_string(),
        "--repo".to_string(),
        repo.to_string_lossy().to_string(),
        "doctor".to_string(),
        "--agents".to_string(),
        agents.to_string(),
        "--config-root".to_string(),
        config_root.to_string_lossy().to_string(),
    ];
    let outcome = cli::dispatch(&argv, "mine");
    let (stdout, stderr) = cli::render(&outcome, true, false);
    let body = if outcome.exit_code == 0 {
        stdout
    } else {
        stderr
    };
    let mut envelope: serde_json::Value =
        serde_json::from_str(&body).unwrap_or(serde_json::Value::Null);
    if envelope["data"]["agents"].is_null() {
        let agents = envelope["error"]["details"]["agents"].clone();
        if !agents.is_null() {
            envelope["data"]["agents"] = agents;
        }
    }
    envelope
}

const FOUR_AGENTS: &[&str] = &["claude-code", "codex", "pi", "opencode"];

#[test]
fn all_four_agents_install_and_doctor_healthy() {
    let repo = repo_root();
    let tmp = tempfile::tempdir().unwrap();
    for slug in FOUR_AGENTS {
        let (exit, env) = run_agent(&repo, tmp.path(), &["agent", "install", slug]);
        assert_eq!(exit, 0, "install {slug} failed: {env}");
        assert_eq!(
            env["data"]["skills_installed"].as_u64(),
            Some(5),
            "{slug}: 5 skills"
        );
    }
    // Doctor reports all four healthy.
    let doc = run_doctor(&repo, tmp.path(), "all");
    let diags = doc["data"]["agents"]["diagnostics"]
        .as_array()
        .expect("diagnostics array");
    assert_eq!(diags.len(), 4, "all four agents diagnosed");
    for d in diags {
        assert_eq!(d["status"], "healthy", "agent healthy: {}", d["agent"]);
    }
}

#[test]
fn real_home_and_agent_dirs_unchanged() {
    let repo = repo_root();
    let home = real_homedir();
    let dirs = [".claude", ".codex", ".pi", ".config/opencode"];
    let before: Vec<(String, Vec<String>)> = dirs
        .iter()
        .map(|d| {
            let p = home.join(d);
            let files = if p.exists() {
                files_under(&p)
            } else {
                Vec::new()
            };
            (d.to_string(), files)
        })
        .collect();

    let tmp = tempfile::tempdir().unwrap();
    for slug in FOUR_AGENTS {
        run_agent(&repo, tmp.path(), &["agent", "install", slug]);
    }

    let after: Vec<(String, Vec<String>)> = dirs
        .iter()
        .map(|d| {
            let p = home.join(d);
            let files = if p.exists() {
                files_under(&p)
            } else {
                Vec::new()
            };
            (d.to_string(), files)
        })
        .collect();
    assert_eq!(before, after, "real HOME Agent dirs unchanged");
}

fn real_homedir() -> PathBuf {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn files_under(root: &std::path::Path) -> Vec<String> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        if let Ok(rd) = std::fs::read_dir(&dir) {
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else {
                    out.push(
                        p.strip_prefix(root)
                            .unwrap()
                            .to_string_lossy()
                            .replace('\\', "/"),
                    );
                }
            }
        }
    }
    out.sort();
    out
}

// Enforce no `unsafe` in MINE-owned test crates.
#![forbid(unsafe_code)]

//! Shared helpers for the Plan 07 agent-setup test suite.
//!
//! Every test drives the **real `mine` binary** via the accepted CLI
//! (`cli::dispatch`) against an isolated temporary configuration root. No test
//! touches the real user HOME or real Agent configuration.

use std::path::{Path, PathBuf};

use mine::cli;
use mine::domain::status::PlanStatus;

/// The five first-class Skill names.
#[allow(dead_code)]
pub const FIVE_SKILLS: &[&str] = &[
    "mine-arch",
    "mine-plan-create",
    "mine-plan-exec",
    "mine-plan-review",
    "mine-sync",
];

/// The four governed Agents.
pub const FOUR_AGENTS: &[&str] = &["claude-code", "codex", "pi", "opencode"];

/// Builds an argv that runs `mine --repo <repo> agent <verb> [<slug>] ... --config-root <root>`.
/// The `--config-root` flag is appended at the END so it does not become the
/// agent subcommand (the dispatcher parses `tokens[1]` as the subcommand).
pub fn agent_argv(repo: &str, config_root: &Path, rest: &[&str]) -> Vec<String> {
    let mut v = vec!["mine".to_string(), "--repo".to_string(), repo.to_string()];
    for &t in rest {
        v.push(t.to_string());
    }
    v.push("--config-root".to_string());
    v.push(config_root.to_string_lossy().to_string());
    v
}

/// Dispatches a `mine agent ...` call and returns the outcome + parsed JSON
/// envelope.
pub fn dispatch_agent(
    repo: &Path,
    config_root: &Path,
    rest: &[&str],
) -> (cli::Outcome, serde_json::Value) {
    let argv = agent_argv(repo.to_str().unwrap(), config_root, rest);
    let outcome = cli::dispatch(&argv, "mine");
    let (stdout, stderr) = cli::render(&outcome, true, false);
    let body = if outcome.exit_code == 0 {
        stdout
    } else {
        stderr
    };
    (
        outcome,
        serde_json::from_str(&body).expect("envelope is valid JSON"),
    )
}

/// Dispatches `mine doctor --agents all --config-root <root>`.
pub fn dispatch_doctor(repo: &Path, config_root: &Path, agents: &str) -> serde_json::Value {
    let argv = vec![
        "mine".to_string(),
        "--repo".to_string(),
        repo.to_str().unwrap().to_string(),
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
    serde_json::from_str(&body).expect("doctor envelope is valid JSON")
}

/// A node helper for the seed fixture (reused shape; the agent tests don't
/// mutate the graph).
#[allow(dead_code)]
pub fn blank_plan_node() -> (&'static str, PlanStatus) {
    ("unused", PlanStatus::Ready)
}

/// Reads a file relative to `root` as a string.
#[allow(dead_code)]
pub fn read(root: &Path, rel: &str) -> String {
    std::fs::read_to_string(root.join(rel))
        .unwrap_or_else(|e| panic!("read {}: {e}", root.join(rel).display()))
}

/// Lists files under `root` (relative, forward-slash).
pub fn files_under(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for e in std::fs::read_dir(&dir).unwrap().flatten() {
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
    out.sort();
    out
}

/// The repository root for `--repo` (the live repo; tests don't mutate it).
pub fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

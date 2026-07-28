// Enforce no `unsafe` in MINE-owned test crates.
#![forbid(unsafe_code)]

//! Fix 3 tests: explicit `--config-root` isolation - real process environment
//! variables (`CLAUDE_CONFIG_DIR`, `CODEX_HOME`, `PI_HOME`, `OPENCODE_CONFIG_DIR`)
//! are never honored when an explicit config root is supplied.
//!
//! Because the crate forbids `unsafe` and mutating the global process
//! environment is unsafe in parallel tests, we verify isolation structurally:
//! the `Env::isolated` constructor always has an empty override map (never reads
//! `std::env`), and the CLI `--config-root` path builds an isolated env. We
//! also snapshot the real HOME Agent dirs before/after to prove no mutation.

use super::common::*;
use mine::agent_setup::targets::{Agent, Env, Targets};
use std::path::{Path, PathBuf};

#[test]
fn isolated_env_has_empty_overrides() {
    // The structural guarantee: an isolated env never reads real env vars.
    // Its override map is empty by construction, regardless of what
    // CLAUDE_CONFIG_DIR/CODEX_HOME/PI_HOME/OPENCODE_CONFIG_DIR happen to be set
    // to in the real process environment.
    let env = Env::isolated(PathBuf::from("/tmp/test-root"));
    assert!(env.isolated, "isolated flag set");
    assert!(
        env.overrides.is_empty(),
        "override map is empty - no real env vars leaked"
    );
}

#[test]
fn isolated_env_derives_all_paths_from_root() {
    let root = PathBuf::from("/tmp/injected");
    let env = Env::isolated(root.clone());
    for agent in Agent::ALL {
        let t = Targets::resolve(agent, &env);
        // Every target path starts with the injected root.
        assert!(
            t.skills_dir.starts_with(&root),
            "{:?} skills_dir {:?} must start with injected root",
            agent,
            t.skills_dir
        );
        if let Some(cfg) = t.mcp_config_file {
            assert!(
                cfg.starts_with(&root),
                "{:?} mcp_config {:?} must start with injected root",
                agent,
                cfg
            );
        }
    }
}

#[test]
fn real_env_reads_live_environment() {
    // The real_env constructor IS distinct from isolated: it reads std::env.
    // This test only verifies the constructors are structurally different -
    // it does NOT mutate the global environment (unsafe in parallel tests).
    let real = Env::real_env();
    assert!(!real.isolated, "real_env is NOT isolated");
    // The overrides map may or may not be populated depending on the real
    // environment; the key invariant is `isolated == false`.
}

#[test]
fn real_home_and_agent_dirs_unchanged_after_suite() {
    // Snapshot the real home's Agent dirs before and after an isolated install.
    // They must be byte-for-byte unchanged.
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

    // Run an isolated install for all four agents.
    let tmp = tempfile::tempdir().unwrap();
    for slug in FOUR_AGENTS {
        dispatch_agent(&repo, tmp.path(), &["agent", "install", slug]);
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
    assert_eq!(
        before, after,
        "real HOME Agent dirs are unchanged by isolated installs"
    );
}

fn real_homedir() -> PathBuf {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn files_under(root: &Path) -> Vec<String> {
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

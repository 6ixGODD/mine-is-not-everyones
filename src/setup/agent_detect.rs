#![forbid(unsafe_code)]

//! Detection of installed coding agents.
//!
//! An agent counts as "detected" only when BOTH:
//!   1. its CLI is on PATH (e.g. `claude`, `codex`, `pi`, `opencode`); AND
//!   2. its user configuration directory exists.
//!
//! Requiring both avoids false positives from leftover config dirs after the
//! app was uninstalled, and from CLI shims without a real install.

use std::path::PathBuf;

use crate::agent_setup::targets::Agent;

#[derive(Debug, Clone, serde::Serialize)]
pub struct Detection {
    pub slug: String,
    pub display_name: String,
    pub cli_on_path: bool,
    pub config_dir_exists: bool,
    pub detected: bool,
    pub config_dir: String,
}

/// Returns true if `slug` is one of the four supported agent slugs.
pub fn slug_is_supported(slug: &str) -> bool {
    Agent::from_slug(slug).is_some()
}

/// Detects all four supported agents on this machine.
pub fn detect_all() -> Vec<Detection> {
    Agent::ALL.iter().map(|a| detect_one(*a)).collect()
}

fn detect_one(agent: Agent) -> Detection {
    let (cli_name, config_dir) = agent_cli_and_config_dir(agent);
    let cli_on_path = which(cli_name);
    let config_dir_path = config_dir.unwrap_or_else(PathBuf::new);
    let config_dir_exists = !config_dir_path.as_os_str().is_empty() && config_dir_path.exists();
    let detected = cli_on_path && config_dir_exists;
    Detection {
        slug: agent.slug().to_string(),
        display_name: display_name(agent),
        cli_on_path,
        config_dir_exists,
        detected,
        config_dir: if config_dir_path.as_os_str().is_empty() {
            String::new()
        } else {
            config_dir_path.display().to_string()
        },
    }
}

fn display_name(agent: Agent) -> String {
    match agent {
        Agent::ClaudeCode => "Claude Code".to_string(),
        Agent::Codex => "Codex".to_string(),
        Agent::Pi => "Pi".to_string(),
        Agent::OpenCode => "OpenCode".to_string(),
    }
}

/// Returns (cli binary basename, user config dir) for an agent.
fn agent_cli_and_config_dir(agent: Agent) -> (&'static str, Option<PathBuf>) {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from);
    match agent {
        Agent::ClaudeCode => ("claude", home.as_ref().map(|h| h.join(".claude"))),
        Agent::Codex => ("codex", home.as_ref().map(|h| h.join(".codex"))),
        Agent::Pi => ("pi", home.as_ref().map(|h| h.join(".pi"))),
        Agent::OpenCode => {
            // OpenCode: ~/.config/opencode on Unix, %APPDATA%\opencode on Windows.
            if cfg!(windows) {
                (
                    "opencode",
                    std::env::var_os("APPDATA")
                        .map(PathBuf::from)
                        .map(|p| p.join("opencode")),
                )
            } else {
                let xdg = std::env::var_os("XDG_CONFIG_HOME")
                    .map(PathBuf::from)
                    .or_else(|| home.as_ref().map(|h| h.join(".config")));
                ("opencode", xdg.map(|p| p.join("opencode")))
            }
        }
    }
}

/// A minimal `which`: searches PATH for an executable. On Windows, tries
/// `.exe` and bare name.
fn which(name: &str) -> bool {
    let path = match std::env::var_os("PATH") {
        Some(p) => p,
        None => return false,
    };
    for dir in std::env::split_paths(&path) {
        if dir.as_os_str().is_empty() {
            continue;
        }
        let candidates = if cfg!(windows) {
            vec![dir.join(format!("{name}.exe")), dir.join(name)]
        } else {
            vec![dir.join(name)]
        };
        for c in candidates {
            if is_executable(&c) {
                return true;
            }
        }
    }
    false
}

fn is_executable(p: &std::path::Path) -> bool {
    // std::fs::metadata follows symlinks; fine for PATH lookup.
    p.is_file()
}

/// Prints a one-line summary of each agent's detection state.
pub fn print_summary(detections: &[Detection]) {
    println!("Detected coding agents:");
    for d in detections {
        let mark = if d.detected { "✓" } else { "—" };
        let suffix = if d.detected {
            String::new()
        } else {
            " (not detected)".to_string()
        };
        println!("  {mark} {}{}", d.display_name, suffix);
    }
}

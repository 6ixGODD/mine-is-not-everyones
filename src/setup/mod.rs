#![forbid(unsafe_code)]

//! `mine setup` / `mine update` / `mine uninstall` — the end-user installation
//! lifecycle.
//!
//! These commands are the user-facing surface that the remote bootstrap scripts
//! delegate to. They own:
//!
//! - the MINE ASCII banner;
//! - latest-release version comparison against GitHub Releases;
//! - detection of installed coding agents (Claude Code, Codex, Pi, OpenCode)
//!   by requiring BOTH the agent CLI on PATH AND its config directory;
//! - an interactive TUI selector (crossterm) for choosing which detected
//!   agents to install/uninstall MINE into, with a non-TTY fallback that
//!   installs into every detected agent;
//! - delegation to the existing transactional `agent_service::install` /
//!   `agent_service::uninstall` for the actual MCP + Skill wiring;
//! - self-update (`mine update`) and full local removal (`mine uninstall`).
//!
//! Design notes:
//! - `mine setup` is the single entry point the bootstrap downloads `mine` for
//!   and then runs. It never assumes a repository context (it is invoked
//!   before `mine init`); it operates on the user machine, not a repo.
//! - Network access is best-effort and reported honestly: if the latest
//!   release cannot be resolved, version comparison is skipped with a clear
//!   note rather than fabricated.
//! - The TUI is gated on stdin being a TTY. When it is not (CI, `curl|sh`
//!   pipes), the selector is skipped and the `--agents` flag (or "all
//!   detected") drives installation non-interactively.

pub mod agent_detect;
pub mod banner;
pub mod release_meta;
pub mod selector;
pub mod self_update;

use crate::application::agent_service;
use crate::domain::error::{MineError, MineResult};
use std::path::PathBuf;

/// Which agents to act on, resolved from CLI flags or the TUI.
#[derive(Debug, Clone)]
pub struct SetupPlan {
    /// Agents to install MINE into.
    pub install: Vec<String>,
    /// Agents to uninstall MINE from (were installed, user deselected).
    pub uninstall: Vec<String>,
}

/// Runs `mine setup`.
///
/// Steps:
/// 1. Print the banner.
/// 2. Resolve the latest published release tag; if the running binary is
///    older and stdin is a TTY, prompt to update first.
/// 3. Detect installed agents.
/// 4. Resolve the install/uninstall plan (TUI if TTY, else `--agents` or all
///    detected).
/// 5. Execute: install for `install`, uninstall for `uninstall`.
pub fn run_setup(args: &SetupArgs) -> MineResult<SetupReport> {
    banner::print();
    let current = env!("CARGO_PKG_VERSION");

    // Version check against latest release (best-effort).
    let latest = release_meta::latest_tag();
    let version_note = match &latest {
        Ok(tag) if release_meta::is_newer(tag, current) => {
            let msg = format!("mine {} is available (you are running {})", tag, current);
            if args.yes {
                println!("{msg}");
                println!("Run `mine update` to upgrade.");
            } else {
                println!("{msg}");
            }
            msg
        }
        Ok(tag) => format!("mine {} is up to date (latest release: {})", current, tag),
        Err(e) => format!("could not check for updates: {e}"),
    };
    println!("{version_note}");
    println!();

    // Detect installed agents.
    let detections = agent_detect::detect_all();
    agent_detect::print_summary(&detections);
    println!();

    // Build the environment once: isolated when --config-root is given
    // (CI/tests), otherwise the real-env (reads live process env + home).
    let env = match &args.config_root {
        Some(p) => agent_service::isolated_env(p.clone()),
        None => agent_service::real_env(),
    };

    // Resolve the plan.
    let plan = if let Some(list) = &args.agents {
        // Explicit --agents a,b,c (non-interactive override).
        let requested: Vec<String> = list.split(',').map(|s| s.trim().to_string()).collect();
        // Validate slugs; ignore unknowns with a warning.
        let mut install = Vec::new();
        for slug in &requested {
            if agent_detect::slug_is_supported(slug) {
                install.push(slug.clone());
            } else {
                eprintln!("warning: unknown agent {slug:?}, skipping");
            }
        }
        SetupPlan {
            install,
            uninstall: Vec::new(),
        }
    } else {
        selector::resolve_plan(&detections, args.yes, &env)?
    };

    // Execute.
    let mut installed = Vec::new();
    let mut uninstalled = Vec::new();
    let mut errors = Vec::new();

    for slug in &plan.install {
        // When the plan came from an explicit --agents list (non-interactive),
        // honor it even if the agent is not detected on this machine (CI
        // installs into an isolated config root without the agent CLI). The
        // TUI/non-TTY fallback path only includes detected agents, so this
        // filter only relaxes the explicit-list case.
        if args.agents.is_none() {
            let det = detections.iter().find(|d| d.slug == *slug);
            if let Some(d) = det {
                if !d.detected {
                    eprintln!("skipping {slug}: agent not detected on this machine");
                    continue;
                }
            }
        }
        match agent_service::install(
            slug,
            &env,
            current,
            false,
            crate::agent_setup::install::FailPhase::None,
        ) {
            Ok(_) => installed.push(slug.clone()),
            Err(e) => errors.push(format!("install {slug}: {e}")),
        }
    }
    for slug in &plan.uninstall {
        match agent_service::uninstall(slug, &env, false) {
            Ok(_) => uninstalled.push(slug.clone()),
            Err(e) => errors.push(format!("uninstall {slug}: {e}")),
        }
    }

    Ok(SetupReport {
        version_note,
        detected: detections,
        installed,
        uninstalled,
        errors,
    })
}

/// Runs `mine update`: compare to latest, optionally download+replace.
pub fn run_update(args: &UpdateArgs) -> MineResult<UpdateReport> {
    let current = env!("CARGO_PKG_VERSION");
    let latest = release_meta::latest_tag();
    match latest {
        Ok(tag) if release_meta::is_newer(&tag, current) => {
            println!("mine {} is available (you are running {}).", tag, current);
            if !args.yes {
                print!("Update now? [y/N] ");
                use std::io::Write;
                std::io::stdout().flush().ok();
                let mut line = String::new();
                std::io::stdin().read_line(&mut line).ok();
                if !line.trim().eq_ignore_ascii_case("y") {
                    return Ok(UpdateReport {
                        from: current.to_string(),
                        to: tag,
                        updated: false,
                        note: "update declined".to_string(),
                    });
                }
            }
            let to = tag.clone();
            self_update::download_and_replace(&to)?;
            Ok(UpdateReport {
                from: current.to_string(),
                to,
                updated: true,
                note: "updated".to_string(),
            })
        }
        Ok(tag) => Ok(UpdateReport {
            from: current.to_string(),
            to: tag,
            updated: false,
            note: "already up to date".to_string(),
        }),
        Err(e) => Err(MineError::ExternalDependency {
            detail: format!("could not resolve latest release: {e}"),
        }),
    }
}

/// Runs `mine uninstall`: remove MINE from every agent, then remove the
/// binary and PATH entries.
pub fn run_uninstall(args: &UninstallArgs) -> MineResult<UninstallReport> {
    if !args.yes {
        println!("This will remove MINE from every coding agent and delete the");
        println!("`mine` binary and its PATH entries. This cannot be undone.");
        print!("Proceed? [y/N] ");
        use std::io::Write;
        std::io::stdout().flush().ok();
        let mut line = String::new();
        std::io::stdin().read_line(&mut line).ok();
        if !line.trim().eq_ignore_ascii_case("y") {
            return Ok(UninstallReport {
                agents_cleared: Vec::new(),
                binary_removed: false,
                note: "uninstall declined".to_string(),
            });
        }
    }

    let env = agent_service::real_env();
    let state = crate::agent_setup::managed_state::ManagedState::load(&env.config_root)
        .unwrap_or_else(|_| crate::agent_setup::managed_state::ManagedState::new());
    let mut cleared = Vec::new();
    for agent in crate::agent_setup::targets::Agent::ALL {
        if state.record(agent.slug()).is_some() {
            match agent_service::uninstall(agent.slug(), &env, false) {
                Ok(_) => cleared.push(agent.slug().to_string()),
                Err(e) => eprintln!("warning: uninstall {} failed: {}", agent.slug(), e),
            }
        }
    }
    let binary_removed = self_update::remove_self()?;
    Ok(UninstallReport {
        agents_cleared: cleared,
        binary_removed,
        note: "mine removed".to_string(),
    })
}

/// CLI flags for `mine setup`.
#[derive(Debug, Clone, Default)]
pub struct SetupArgs {
    /// `--agents claude,codex` non-interactive override.
    pub agents: Option<String>,
    /// `--yes` / `-y`: skip interactive prompts (version update prompt,
    /// non-TTY selector fallback uses all detected).
    pub yes: bool,
    /// `--config-root <path>`: isolated config root (CI/tests). When set,
    /// setup uses [`Env::isolated`] and never touches the real environment.
    pub config_root: Option<PathBuf>,
}

/// CLI flags for `mine update`.
#[derive(Debug, Clone, Default)]
pub struct UpdateArgs {
    pub yes: bool,
}

/// CLI flags for `mine uninstall`.
#[derive(Debug, Clone, Default)]
pub struct UninstallArgs {
    pub yes: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SetupReport {
    pub version_note: String,
    pub detected: Vec<agent_detect::Detection>,
    pub installed: Vec<String>,
    pub uninstalled: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct UpdateReport {
    pub from: String,
    pub to: String,
    pub updated: bool,
    pub note: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct UninstallReport {
    pub agents_cleared: Vec<String>,
    pub binary_removed: bool,
    pub note: String,
}

// Re-export Env for submodules.
pub use crate::agent_setup::targets::Env as SetupEnv;

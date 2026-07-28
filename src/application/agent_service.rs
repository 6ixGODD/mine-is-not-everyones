// Enforce `AGENTS.md`'s "Business code must not use `unsafe`" at compile time.
#![forbid(unsafe_code)]

//! Application service for agent install/uninstall/status/config — Plan 07-1.
//!
//! Orchestration behind the CLI `mine agent` group. The critical Fix 3
//! (isolation): there are two distinct entry points — [`install`] /
//! [`uninstall`] / [`doctor`] / [`status`] all take an explicit [`Env`]
//! built by the caller (isolated when `--config-root` supplied, real-env only
//! when not). The service NEVER reads `std::env` itself; the caller chooses the
//! env constructor. This structural separation is what the rejected Plan 07's
//! mixed `real_env()`/`agent_env()` violated.

use std::path::PathBuf;

use crate::agent_setup;
use crate::agent_setup::doctor::{AgentDiagnostic, AgentStatus};
use crate::agent_setup::install::{FailPhase, InstallOutcome};
use crate::agent_setup::targets::{Agent, Env};
use crate::agent_setup::uninstall::UninstallOutcome;
use crate::domain::error::{MineError, MineResult};

/// Resolves an [`Agent`] from a slug.
pub fn resolve_agent(slug: &str) -> MineResult<Agent> {
    Agent::from_slug(slug).ok_or_else(|| MineError::AgentUnsupported {
        detail: format!("unknown agent {slug:?}; supported: claude-code, codex, pi, opencode"),
    })
}

/// Installs/updates MINE for `slug` under the explicitly-provided `env`.
/// `mine_version` is the live MINE version; `dry_run` performs no mutation.
/// Tests inject `fail_phase` to exercise rollback; production passes
/// `FailPhase::None`.
pub fn install(
    slug: &str,
    env: &Env,
    mine_version: &str,
    dry_run: bool,
    fail_phase: FailPhase,
) -> MineResult<InstallOutcome> {
    let agent = resolve_agent(slug)?;
    agent_setup::install::install(agent, env, mine_version, dry_run, fail_phase)
}

/// Uninstalls MINE for `slug` under the explicitly-provided `env`.
pub fn uninstall(slug: &str, env: &Env, dry_run: bool) -> MineResult<UninstallOutcome> {
    let agent = resolve_agent(slug)?;
    agent_setup::uninstall::uninstall(agent, env, dry_run)
}

/// Doctor diagnostics for one agent (single slug) or all four (`"all"`).
pub fn doctor(slug: &str, env: &Env, current_mine_version: &str) -> MineResult<DoctorReport> {
    let state_result = agent_setup::managed_state::ManagedState::load(&env.config_root);
    let malformed = state_result.is_err();
    let state = state_result.unwrap_or_else(|_| agent_setup::managed_state::ManagedState::new());
    if slug == "all" {
        let (diags, all_healthy, malformed_flag) =
            agent_setup::doctor::doctor_all(env, current_mine_version);
        return Ok(DoctorReport {
            diagnostics: diags,
            all_healthy,
            malformed_state: malformed_flag,
        });
    }
    let agent = resolve_agent(slug)?;
    let d = agent_setup::doctor::doctor(agent, env, &state, current_mine_version);
    Ok(DoctorReport {
        diagnostics: vec![d.clone()],
        all_healthy: d.status == AgentStatus::Healthy,
        malformed_state: malformed,
    })
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct DoctorReport {
    pub diagnostics: Vec<AgentDiagnostic>,
    pub all_healthy: bool,
    pub malformed_state: bool,
}

/// `mine agent status`: list MINE-managed agent installations under `env`.
pub fn status(env: &Env) -> MineResult<Vec<AgentInstallSummary>> {
    let state = agent_setup::managed_state::ManagedState::load(&env.config_root)?;
    let mut out: Vec<AgentInstallSummary> = Vec::new();
    for agent in Agent::ALL {
        if let Some(rec) = state.record(agent.slug()) {
            let targets = agent_setup::targets::Targets::resolve(agent, env);
            let mcp_ok = if agent.supports_mcp() {
                agent_setup::doctor::doctor(agent, env, &state, "-").mcp_registered
            } else {
                false
            };
            out.push(AgentInstallSummary {
                agent: rec.agent.clone(),
                mine_version: rec.mine_version.clone(),
                destination: rec.destination.clone(),
                files: rec.files.len(),
                config_entries: rec.config_entries.len(),
                previous_version: rec.previous_version.clone(),
                mcp_registered: agent.supports_mcp() && mcp_ok,
                skills_dir: targets.skills_dir.to_string_lossy().to_string(),
            });
        }
    }
    out.sort_by(|a, b| a.agent.cmp(&b.agent));
    Ok(out)
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AgentInstallSummary {
    pub agent: String,
    pub mine_version: String,
    pub destination: String,
    pub files: usize,
    pub config_entries: usize,
    pub previous_version: Option<String>,
    pub mcp_registered: bool,
    pub skills_dir: String,
}

/// `mine agent config`: read-only preview of the MCP entry MINE would merge.
/// No file mutation.
pub fn config_preview(slug: &str) -> MineResult<ConfigPreview> {
    let agent = resolve_agent(slug)?;
    match agent {
        Agent::ClaudeCode => Ok(ConfigPreview {
            agent: agent.slug().to_string(),
            target_file: "~/.claude.json (or $CLAUDE_CONFIG_DIR)".to_string(),
            json_pointer: "/mcpServers/mine".to_string(),
            entry: serde_json::json!({"command":"mine","args":["mcp","serve"]}),
            supports_mcp: true,
        }),
        Agent::Codex => Ok(ConfigPreview {
            agent: agent.slug().to_string(),
            target_file: "~/.codex/config.toml (or $CODEX_HOME)".to_string(),
            json_pointer: "/mcp_servers/mine".to_string(),
            entry: serde_json::json!({"command":"mine","args":["mcp","serve"],"enabled":true}),
            supports_mcp: true,
        }),
        Agent::OpenCode => Ok(ConfigPreview {
            agent: agent.slug().to_string(),
            target_file: "~/.config/opencode/opencode.json (or $OPENCODE_CONFIG_DIR)".to_string(),
            json_pointer: "/mcp/mine".to_string(),
            entry: serde_json::json!({"type":"local","command":["mine","mcp","serve"],"enabled":true}),
            supports_mcp: true,
        }),
        Agent::Pi => Ok(ConfigPreview {
            agent: agent.slug().to_string(),
            target_file: "(none - Pi has no MCP)".to_string(),
            json_pointer: String::new(),
            entry: serde_json::Value::Null,
            supports_mcp: false,
        }),
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ConfigPreview {
    pub agent: String,
    pub target_file: String,
    pub json_pointer: String,
    pub entry: serde_json::Value,
    pub supports_mcp: bool,
}

/// The production entry point used only when `--config-root` is NOT supplied:
/// builds the real-env [`Env`] (reads the live process environment and the
/// platform home dir). The CLI chooses this vs [`Env::isolated`]; this
/// function never runs under an explicit `--config-root`.
#[must_use]
pub fn real_env() -> Env {
    Env::real_env()
}

/// Re-export so the CLI can build an isolated env without leaking `std::env`.
#[must_use]
pub fn isolated_env(root: PathBuf) -> Env {
    Env::isolated(root)
}

// Enforce `AGENTS.md`'s "Business code must not use `unsafe`" at compile time.
#![forbid(unsafe_code)]

//! Doctor service for agent diagnostics: the bridge between the existing
//! `mine doctor` repository checks and the agent diagnostics. It
//! produces a combined machine-readable report; the CLI `mine doctor` handler
//! calls this to append the agent section.
//!
//! This service only orchestrates [`crate::agent_setup::doctor`]; it performs no
//! filesystem mutation.

use serde::Serialize;

use crate::agent_setup::doctor::AgentDiagnostic;
use crate::agent_setup::targets::Env;
use crate::application::agent_service;

/// The agent portion of the doctor report.
#[derive(Debug, Clone, Serialize)]
pub struct AgentDoctorSection {
    pub all_healthy: bool,
    pub malformed_state: bool,
    pub diagnostics: Vec<AgentDiagnostic>,
}

/// Runs the agent diagnostics against an injected `env` and current MINE
/// version. `agents` selects which agents to report (`"all"` or a single slug).
pub fn run(agents: &str, env: &Env, current_mine_version: &str) -> AgentDoctorSection {
    match agent_service::doctor(agents, env, current_mine_version) {
        Ok(r) => AgentDoctorSection {
            all_healthy: r.all_healthy,
            malformed_state: r.malformed_state,
            diagnostics: r.diagnostics,
        },
        Err(_) => AgentDoctorSection {
            // On error (e.g. malformed state), synthesize a malformed section.
            all_healthy: false,
            malformed_state: true,
            diagnostics: Vec::new(),
        },
    }
}

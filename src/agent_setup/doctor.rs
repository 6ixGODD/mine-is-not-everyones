// Enforce `AGENTS.md`'s "Business code must not use `unsafe`" at compile time.
#![forbid(unsafe_code)]

//! Agent diagnostics — truthful inspection of actual Agent installation state.
//! Agent doctor diagnostics, reworked for isolated `--config-root` operation and
//! [`Env`] (it derives paths only from `env.config_root`, never real env vars).
//! Adds an incomplete-transaction status (Fix 2): doctor reports a pending
//! transaction so the user sees an actionable recovery state.

use serde::Serialize;

use crate::agent_setup::managed_state::{ManagedState, OwnedFile};
use crate::agent_setup::safety::{SafetyGuard, content_hash};
use crate::agent_setup::targets::{Agent, Env, Targets};
use crate::agent_setup::transaction;
use crate::agent_setup::transaction::PendingTransaction;
use crate::infrastructure::embedded_skills;

#[derive(Debug, Clone, Serialize)]
pub struct AgentDiagnostic {
    pub agent: String,
    pub status: AgentStatus,
    pub note: String,
    pub managed_files: usize,
    pub found_files: usize,
    pub drifted_files: usize,
    pub mcp_registered: bool,
    pub incomplete_transaction: bool,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatus {
    AgentNotDetected,
    AgentDetectedMineNotInstalled,
    Healthy,
    MissingFiles,
    DriftedFiles,
    StaleVersion,
    MalformedManagedState,
    McpRegistrationMissingOrIncorrect,
    Unsupported,
    IncompleteTransaction,
}

pub fn doctor_all(env: &Env, current_mine_version: &str) -> (Vec<AgentDiagnostic>, bool, bool) {
    let state_result = ManagedState::load(&env.config_root);
    let malformed = state_result.is_err();
    let state = state_result.unwrap_or_else(|_| ManagedState::new());
    let mut diags = Vec::new();
    let mut healthy_all = true;
    for agent in Agent::ALL {
        if malformed && state.installs.is_empty() {
            diags.push(AgentDiagnostic {
                agent: agent.slug().to_string(),
                status: AgentStatus::MalformedManagedState,
                note: "managed installation state is malformed or foreign".to_string(),
                managed_files: 0,
                found_files: 0,
                drifted_files: 0,
                mcp_registered: false,
                incomplete_transaction: false,
            });
            healthy_all = false;
            continue;
        }
        let d = doctor(agent, env, &state, current_mine_version);
        if d.status != AgentStatus::Healthy {
            healthy_all = false;
        }
        diags.push(d);
    }
    (diags, healthy_all, malformed)
}

pub fn doctor(
    agent: Agent,
    env: &Env,
    state: &ManagedState,
    current_mine_version: &str,
) -> AgentDiagnostic {
    let guard = SafetyGuard::new(&env.config_root);
    let targets = Targets::resolve(agent, env);

    // Incomplete-transaction detection (Fix 2): a pending record means an
    // interrupted install requiring recovery.
    let incomplete = transaction::is_pending(agent.slug(), &env.config_root);
    if incomplete {
        // If the pending record carries rollback-failure evidence, surface it.
        let note = match PendingTransaction::load(agent.slug(), &env.config_root) {
            Ok(Some(p)) if p.rollback_failure.is_some() => {
                format!(
                    "an incomplete installation transaction exists with a prior rollback failure ({}); recovery is needed",
                    p.rollback_failure.unwrap_or_default()
                )
            }
            _ => "an incomplete installation transaction exists; the next install recovers or reports it".to_string(),
        };
        return AgentDiagnostic {
            agent: agent.slug().to_string(),
            status: AgentStatus::IncompleteTransaction,
            note,
            managed_files: 0,
            found_files: 0,
            drifted_files: 0,
            mcp_registered: false,
            incomplete_transaction: true,
        };
    }

    let Some(record) = state.record(agent.slug()) else {
        let detected = targets.skills_dir.exists()
            || targets.mcp_config_file.as_ref().is_some_and(|p| p.exists());
        let (status, note) = if detected {
            (
                AgentStatus::AgentDetectedMineNotInstalled,
                "Agent detected but MINE is not installed for it".to_string(),
            )
        } else if agent_caps_dir_exists(agent, env) {
            (
                AgentStatus::AgentDetectedMineNotInstalled,
                "Agent detected elsewhere but MINE has no managed record".to_string(),
            )
        } else {
            (
                AgentStatus::AgentNotDetected,
                "Agent not detected".to_string(),
            )
        };
        return AgentDiagnostic {
            agent: agent.slug().to_string(),
            status,
            note,
            managed_files: 0,
            found_files: 0,
            drifted_files: 0,
            mcp_registered: false,
            incomplete_transaction: false,
        };
    };

    let mut found = 0usize;
    let mut drifted = 0usize;
    let mut missing: Vec<String> = Vec::new();
    let mut drifted_files: Vec<String> = Vec::new();
    for f in &record.files {
        let abs = match guard.ensure_within_root(&env.config_root.join(&f.path)) {
            Ok(p) => p,
            Err(_) => {
                missing.push(f.path.clone());
                continue;
            }
        };
        if !abs.exists() {
            missing.push(f.path.clone());
            continue;
        }
        found += 1;
        if let Ok(cur) = std::fs::read(&abs) {
            if content_hash(&cur) != f.hash {
                drifted += 1;
                drifted_files.push(f.path.clone());
            }
        }
    }

    let skills_dir_rel = targets
        .skill_rel_path(&env.config_root, "")
        .unwrap_or_default()
        .replace('\\', "/");
    let skills_prefix = if skills_dir_rel.is_empty() {
        String::new()
    } else {
        format!("{skills_dir_rel}/")
    };
    let payload_matches = payload_matches_record(&record.files, &skills_prefix);
    let mcp_ok = if agent.supports_mcp() {
        mcp_entry_correct(agent, env)
    } else {
        true
    };

    let status = if !vertex_payload_present(&record.files, &skills_prefix) {
        AgentStatus::MissingFiles
    } else if drifted > 0 {
        AgentStatus::DriftedFiles
    } else if !missing.is_empty() {
        AgentStatus::MissingFiles
    } else if record.mine_version != current_mine_version || !payload_matches {
        AgentStatus::StaleVersion
    } else if !mcp_ok {
        AgentStatus::McpRegistrationMissingOrIncorrect
    } else {
        AgentStatus::Healthy
    };
    let note = match status {
        AgentStatus::Healthy => "healthy managed installation".to_string(),
        AgentStatus::MissingFiles => format!("{} managed file(s) missing", missing.len()),
        AgentStatus::DriftedFiles => format!(
            "{} file(s) drifted from install hashes: {}",
            drifted,
            drifted_files.join(", ")
        ),
        AgentStatus::StaleVersion => format!(
            "managed version {} != current {}",
            record.mine_version, current_mine_version
        ),
        AgentStatus::McpRegistrationMissingOrIncorrect => {
            "MCP registration missing or does not match the standard mine mcp serve".to_string()
        }
        _ => status_note(status),
    };
    AgentDiagnostic {
        agent: agent.slug().to_string(),
        status,
        note,
        managed_files: record.files.len(),
        found_files: found,
        drifted_files: drifted,
        mcp_registered: mcp_ok,
        incomplete_transaction: false,
    }
}

fn status_note(s: AgentStatus) -> String {
    match s {
        AgentStatus::AgentNotDetected => "Agent not detected".into(),
        AgentStatus::AgentDetectedMineNotInstalled => {
            "Agent detected but MINE not installed".into()
        }
        AgentStatus::Healthy => "healthy".into(),
        AgentStatus::MissingFiles => "managed files missing".into(),
        AgentStatus::DriftedFiles => "managed files drifted".into(),
        AgentStatus::StaleVersion => "managed version is stale".into(),
        AgentStatus::MalformedManagedState => "managed state malformed/foreign".into(),
        AgentStatus::McpRegistrationMissingOrIncorrect => {
            "MCP registration missing/incorrect".into()
        }
        AgentStatus::Unsupported => "unsupported agent".into(),
        AgentStatus::IncompleteTransaction => "incomplete transaction".into(),
    }
}

fn recorded_to_embedded(
    files: &[OwnedFile],
    skills_dir_rel: &str,
) -> std::collections::HashSet<String> {
    files
        .iter()
        .filter_map(|f| {
            f.path
                .strip_prefix(skills_dir_rel)
                .map(|rest| format!("skills/{rest}"))
        })
        .collect()
}

fn payload_matches_record(files: &[OwnedFile], skills_dir_rel: &str) -> bool {
    let recorded = recorded_to_embedded(files, skills_dir_rel);
    embedded_skills::EMBEDDED_SKILL_FILES
        .iter()
        .all(|f| recorded.contains(f.path))
}

fn vertex_payload_present(files: &[OwnedFile], skills_dir_rel: &str) -> bool {
    let recorded = recorded_to_embedded(files, skills_dir_rel);
    embedded_skills::EMBEDDED_SKILL_FILES
        .iter()
        .filter(|f| f.path.ends_with("SKILL.md"))
        .all(|f| recorded.contains(f.path))
}

/// Verifies the on-disk MCP entry matches the standard `mine mcp serve`.
fn mcp_entry_correct(agent: Agent, env: &Env) -> bool {
    let targets = Targets::resolve(agent, env);
    let Some(cfg) = targets.mcp_config_file else {
        return true;
    };
    if !cfg.exists() {
        return false;
    }
    let raw = match std::fs::read_to_string(&cfg) {
        Ok(s) => s,
        Err(_) => return false,
    };
    match agent {
        Agent::ClaudeCode => {
            let doc: serde_json::Value =
                serde_json::from_str(&raw).unwrap_or(serde_json::Value::Null);
            let entry = &doc["mcpServers"]["mine"];
            entry.get("command").and_then(|v| v.as_str()) == Some("mine")
                && entry
                    .get("args")
                    .and_then(|v| v.as_array())
                    .is_some_and(|a| {
                        a.len() == 2
                            && a[0].as_str() == Some("mcp")
                            && a[1].as_str() == Some("serve")
                    })
        }
        Agent::OpenCode => {
            let doc: serde_json::Value =
                serde_json::from_str(&raw).unwrap_or(serde_json::Value::Null);
            let entry = &doc["mcp"]["mine"];
            entry.get("type").and_then(|v| v.as_str()) == Some("local")
                && entry
                    .get("command")
                    .and_then(|v| v.as_array())
                    .is_some_and(|a| {
                        a.len() == 3
                            && a[0].as_str() == Some("mine")
                            && a[1].as_str() == Some("mcp")
                            && a[2].as_str() == Some("serve")
                    })
                && entry.get("enabled").and_then(|v| v.as_bool()) == Some(true)
        }
        Agent::Codex => {
            // Use toml_edit to inspect the codex config preserving it (read-only).
            use toml_edit::DocumentMut;
            let doc = raw
                .parse::<DocumentMut>()
                .unwrap_or_else(|_| DocumentMut::new());
            let entry = &doc["mcp_servers"]["mine"];
            entry.get("command").and_then(|v| v.as_str()) == Some("mine")
                && entry.get("enabled").and_then(|v| v.as_bool()) == Some(true)
        }
        Agent::Pi => true,
    }
}

fn agent_caps_dir_exists(agent: Agent, env: &Env) -> bool {
    match agent {
        Agent::ClaudeCode => env.config_root.join(".claude").exists(),
        Agent::Codex => env.config_root.join(".codex").exists(),
        Agent::Pi => env.config_root.join(".pi").exists(),
        Agent::OpenCode => env.config_root.join(".config/opencode").exists(),
    }
}

/// Diagnoses an arbitrary slug (`Unsupported` for unknown agents).
pub fn doctor_slug(slug: &str, env: &Env, state: &ManagedState, current: &str) -> AgentDiagnostic {
    match Agent::from_slug(slug) {
        Some(a) => doctor(a, env, state, current),
        None => AgentDiagnostic {
            agent: slug.to_string(),
            status: AgentStatus::Unsupported,
            note: format!(
                "unknown agent slug {slug:?}; supported: claude-code, codex, pi, opencode"
            ),
            managed_files: 0,
            found_files: 0,
            drifted_files: 0,
            mcp_registered: false,
            incomplete_transaction: false,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_setup::install::{FailPhase, install};
    use crate::agent_setup::targets::Env;

    fn env(tmp: &tempfile::TempDir) -> Env {
        Env::isolated(tmp.path().to_path_buf())
    }

    #[test]
    fn healthy_after_install() {
        let tmp = tempfile::tempdir().unwrap();
        install(
            Agent::ClaudeCode,
            &env(&tmp),
            "0.1.0",
            false,
            FailPhase::None,
        )
        .unwrap();
        let state = ManagedState::load(tmp.path()).unwrap();
        let d = doctor(Agent::ClaudeCode, &env(&tmp), &state, "0.1.0");
        assert_eq!(d.status, AgentStatus::Healthy);
        assert!(d.mcp_registered);
    }

    #[test]
    fn not_installed_when_absent() {
        let tmp = tempfile::tempdir().unwrap();
        let state = ManagedState::new();
        let d = doctor(Agent::Codex, &env(&tmp), &state, "0.1.0");
        assert_eq!(d.status, AgentStatus::AgentNotDetected);
    }

    #[test]
    fn drifted_after_edit() {
        let tmp = tempfile::tempdir().unwrap();
        install(Agent::Pi, &env(&tmp), "0.1.0", false, FailPhase::None).unwrap();
        std::fs::write(
            tmp.path().join(".pi/agent/skills/mine-sync/SKILL.md"),
            "EDITED",
        )
        .unwrap();
        let state = ManagedState::load(tmp.path()).unwrap();
        let d = doctor(Agent::Pi, &env(&tmp), &state, "0.1.0");
        assert_eq!(d.status, AgentStatus::DriftedFiles);
    }

    #[test]
    fn stale_version_detected() {
        let tmp = tempfile::tempdir().unwrap();
        install(Agent::OpenCode, &env(&tmp), "0.1.0", false, FailPhase::None).unwrap();
        let state = ManagedState::load(tmp.path()).unwrap();
        let d = doctor(Agent::OpenCode, &env(&tmp), &state, "0.2.0");
        assert_eq!(d.status, AgentStatus::StaleVersion);
    }
}

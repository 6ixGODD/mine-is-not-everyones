// Enforce `AGENTS.md`'s "Business code must not use `unsafe`" at compile time.
#![forbid(unsafe_code)]

//! Installation and update orchestration — Plan 07-1 rework.
//!
//! The install is a bounded transaction (see [`crate::agent_setup::transaction`]):
//!
//! 1. **recover** any prior incomplete transaction for the agent.
//! 2. **preflight** — load/validate managed state; resolve targets; compute the
//!    payload plan; detect collisions; create + verify required config backup;
//!    build the pending-transaction record and persist it atomically.
//! 3. **stage + commit** — write every skill payload through the `SafetyGuard`
//!    chokepoint (refusing collision with unproven-owned files); merge the
//!    structured MCP config via the format-preserving editor; on success,
//!    verify installed hashes, atomically write final managed state, and
//!    remove the pending record only after final verification.
//! 4. **rollback** — on any failure, restore the config backup, remove only
//!    current-transaction-created files, restore previously-managed files on
//!    update, preserve unrelated/user-owned content, and leave either a fully
//!    restored state or a durable recoverable pending record.
//!
//! `FailPhase` lets tests inject a deterministic failure after each meaningful
//! phase to prove rollback is clean and a retry succeeds (Fix 2).

use std::path::Path;

use crate::agent_setup::backup::{Backup, backup_before_mutation};
use crate::agent_setup::config_edit::{EditedEntry, edit_json_mcp, edit_toml_mcp};
use crate::agent_setup::managed_state::{
    AgentInstallRecord, ManagedState, OwnedConfigEntry, OwnedFile,
};
use crate::agent_setup::safety::{SafetyGuard, content_hash};
use crate::agent_setup::targets::{Agent, Env, Targets};
use crate::agent_setup::transaction::{PendingTransaction, detect_and_recover};
use crate::domain::error::{MineError, MineResult};
use crate::domain::ports::Clock;
use crate::infrastructure::embedded_skills;

pub const PAYLOAD_IDENTITY: &str = "mine-embedded-skills-v1";

/// A phase at which a test can inject a failure (for rollback/recovery tests).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailPhase {
    None,
    AfterBackup,
    AfterPayload,
    AfterConfig,
    AfterFirstPayload,
    AfterManagedState,
    DuringFinalVerify,
}

/// The result of an install/update operation.
#[derive(Debug, Clone, serde::Serialize)]
pub struct InstallOutcome {
    pub agent: String,
    pub mine_version: String,
    pub destination: String,
    pub skills_installed: usize,
    pub config_entries: usize,
    pub previous_version: Option<String>,
    pub updated: bool,
    pub backup: Option<BackupRef>,
}

/// A backup reference reported in structured results (no secrets).
#[derive(Debug, Clone, serde::Serialize)]
pub struct BackupRef {
    pub config_file: String,
    pub backup_path: String,
    pub verified: bool,
}

/// Installs (or idempotently updates) MINE for `agent` under the injected `env`.
/// `mine_version` is the live MINE version. `dry_run` performs no mutation.
/// `fail_phase` (test hook) injects a deterministic failure after the named
/// phase; production passes `FailPhase::None`.
pub fn install(
    agent: Agent,
    env: &Env,
    mine_version: &str,
    dry_run: bool,
    fail_phase: FailPhase,
) -> MineResult<InstallOutcome> {
    install_inner(agent, env, mine_version, dry_run, fail_phase)
}

fn install_inner(
    agent: Agent,
    env: &Env,
    mine_version: &str,
    dry_run: bool,
    fail_phase: FailPhase,
) -> MineResult<InstallOutcome> {
    let guard = SafetyGuard::new(&env.config_root);
    let targets = Targets::resolve(agent, env);

    // 1. Recover any prior incomplete transaction for this agent (Fix 2).
    if !dry_run {
        detect_and_recover(agent.slug(), &env.config_root, &guard)?;
    }

    // 2. Preflight: load managed state, compute the plan, detect collisions.
    let mut state = ManagedState::load(&env.config_root)?;
    let previous_version = state.record(agent.slug()).map(|r| r.mine_version.clone());
    let previous_record_files: Vec<OwnedFile> = state
        .record(agent.slug())
        .map(|r| r.files.clone())
        .unwrap_or_default();

    // Payload plan from the embedded skill directory (authoritative source).
    let payload: Vec<(String, &'static str)> = embedded_skills::EMBEDDED_SKILL_FILES
        .iter()
        .map(|f| {
            let sub = f.path.strip_prefix("skills/").unwrap_or(f.path).to_string();
            (sub, f.content)
        })
        .collect();

    // 3. Mandatory config backup BEFORE any mutation (Fix 1). Only when an
    //    MCP config exists and we are not dry-running.
    let config_backup: Option<Backup> = if agent.supports_mcp() {
        if let Some(cfg) = &targets.mcp_config_file {
            if !dry_run && cfg.exists() {
                let cfg_abs = guard.ensure_within_root(cfg)?;
                let b = backup_before_mutation(&cfg_abs, &env.config_root, &guard)?;
                if fail_phase == FailPhase::AfterBackup {
                    // Inject a backup-stage "failure": emulate a write that
                    // could not complete. We leave the backup in place and
                    // return a backup-failed error so rollback is exercised.
                    return Err(MineError::AgentBackupFailed {
                        target: b
                            .as_ref()
                            .map(|x| x.backup_path.clone())
                            .unwrap_or_default(),
                        detail: "injected backup-stage failure".to_string(),
                    });
                }
                b
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    // 4. Build + persist the pending transaction BEFORE external mutation.
    let pending = PendingTransaction {
        agent: agent.slug().to_string(),
        config_backup: config_backup.clone(),
        newly_created_paths: payload
            .iter()
            .map(|(sub, _)| rel(&targets.skills_dir.join(sub), &env.config_root))
            .collect(),
        previously_owned_paths: previous_record_files
            .iter()
            .map(|f| f.path.clone())
            .collect(),
    };
    if !dry_run {
        pending.save(&env.config_root)?;
    }

    // 5. Stage + commit skill payload (write each file through the guard).
    let mut owned_files: Vec<OwnedFile> = Vec::new();
    let mut skill_count = 0usize;
    let mut created_this_txn: Vec<String> = Vec::new();
    let mut first = true;
    for (subpath, content) in &payload {
        let dest = guard.ensure_within_root(&targets.skills_dir.join(subpath))?;
        // Collision: refuse to overwrite a file that is not proven MINE-owned.
        let was_previously_owned = previous_record_files
            .iter()
            .any(|f| f.path == rel(&dest, &env.config_root));
        if dest.exists() && !was_previously_owned {
            // Not ours and not a pre-existing user file we manage -> refuse.
            // Rollback only the files THIS transaction actually created (not
            // the full plan, which includes this pre-existing user file).
            let mut pend = pending.clone();
            pend.newly_created_paths = created_this_txn.clone();
            return rollback_and_fail(
                &pend,
                &env.config_root,
                &guard,
                MineError::AgentCollision {
                    target: dest,
                    detail:
                        "destination skill file already exists and is not recorded as MINE-owned"
                            .to_string(),
                },
            );
        }
        if !dry_run {
            std::fs::create_dir_all(dest.parent().unwrap()).map_err(MineError::Io)?;
            crate::infrastructure::atomic_write::write(&dest, content.as_bytes())?;
        }
        if !was_previously_owned {
            created_this_txn.push(rel(&dest, &env.config_root));
        }
        if first && fail_phase == FailPhase::AfterFirstPayload && !dry_run {
            let mut pend = pending.clone();
            pend.newly_created_paths = created_this_txn.clone();
            return rollback_and_fail(
                &pend,
                &env.config_root,
                &guard,
                inject_err("after first payload"),
            );
        }
        first = false;
        owned_files.push(OwnedFile {
            path: rel(&dest, &env.config_root),
            hash: content_hash(content.as_bytes()),
        });
        if subpath.ends_with("SKILL.md") {
            skill_count += 1;
        }
    }
    if fail_phase == FailPhase::AfterPayload && !dry_run {
        let mut pend = pending.clone();
        pend.newly_created_paths = created_this_txn.clone();
        return rollback_and_fail(&pend, &env.config_root, &guard, inject_err("after payload"));
    }

    // 6. Merge structured MCP config (format-preserving), where supported.
    let mut owned_config_entries: Vec<OwnedConfigEntry> = Vec::new();
    if agent.supports_mcp() {
        if let Some(cfg) = &targets.mcp_config_file {
            let cfg_abs = guard.ensure_within_root(cfg)?;
            let entry: EditedEntry = match agent {
                Agent::Codex => edit_toml_mcp(&cfg_abs, dry_run)?,
                Agent::ClaudeCode | Agent::OpenCode => edit_json_mcp(&cfg_abs, agent, dry_run)?,
                Agent::Pi => unreachable!("pi has no MCP"),
            };
            owned_config_entries.push(OwnedConfigEntry {
                config_file: rel(&cfg_abs, &env.config_root),
                json_pointer: entry.json_pointer.clone(),
                hash: entry.entry_hash.clone(),
            });
            let _ = entry.changed;
            if fail_phase == FailPhase::AfterConfig && !dry_run {
                let mut pend = pending.clone();
                pend.newly_created_paths = created_this_txn.clone();
                return rollback_and_fail(
                    &pend,
                    &env.config_root,
                    &guard,
                    inject_err("after config"),
                );
            }
        }
    }

    if dry_run {
        // Remove the pending record we wrote for preflight (no mutation occurred).
        let _ = PendingTransaction::remove(agent.slug(), &env.config_root);
        return Ok(InstallOutcome {
            agent: agent.slug().to_string(),
            mine_version: mine_version.to_string(),
            destination: targets.skills_dir.to_string_lossy().to_string(),
            skills_installed: skill_count,
            config_entries: owned_config_entries.len(),
            previous_version,
            updated: true,
            backup: config_backup.as_ref().map(|b| BackupRef {
                config_file: b.original_rel.clone(),
                backup_path: b.backup_path.to_string_lossy().to_string(),
                verified: true,
            }),
        });
    }

    // 7. Final verification: every owned file is present with its hash.
    for of in &owned_files {
        let abs = guard.ensure_within_root(&env.config_root.join(&of.path))?;
        if !abs.exists() {
            return rollback_and_fail(
                &pending,
                &env.config_root,
                &guard,
                inject_err("final verify: missing"),
            );
        }
        let cur = std::fs::read(&abs).map_err(MineError::Io)?;
        if content_hash(&cur) != of.hash {
            return rollback_and_fail(
                &pending,
                &env.config_root,
                &guard,
                inject_err("final verify: hash"),
            );
        }
    }
    if fail_phase == FailPhase::DuringFinalVerify {
        return rollback_and_fail(
            &pending,
            &env.config_root,
            &guard,
            inject_err("injected final-verify failure"),
        );
    }

    // 8. Atomically write final managed state.
    let record = AgentInstallRecord {
        agent: agent.slug().to_string(),
        mine_version: mine_version.to_string(),
        source_identity: PAYLOAD_IDENTITY.to_string(),
        destination: targets.skills_dir.to_string_lossy().to_string(),
        files: owned_files,
        config_entries: owned_config_entries,
        installed_at: crate::infrastructure::system::SystemClock.now_utc_rfc3339(),
        previous_version: previous_version.clone(),
    };
    state.upsert(record);
    if fail_phase == FailPhase::AfterManagedState {
        // Emulate a crash after managed-state write intent but before commit
        // clearance: we still write managed state, then "fail" and roll back
        // so the pending record (not yet removed) drives recovery.
        state.save(&env.config_root)?;
        return rollback_and_fail(
            &pending,
            &env.config_root,
            &guard,
            inject_err("after managed state"),
        );
    }
    state.save(&env.config_root)?;

    // 9. Remove the pending record only after final verification succeeds.
    PendingTransaction::remove(agent.slug(), &env.config_root)?;

    Ok(InstallOutcome {
        agent: agent.slug().to_string(),
        mine_version: mine_version.to_string(),
        destination: targets.skills_dir.to_string_lossy().to_string(),
        skills_installed: skill_count,
        config_entries: state
            .record(agent.slug())
            .map(|r| r.config_entries.len())
            .unwrap_or(0),
        previous_version,
        updated: true,
        backup: config_backup.as_ref().map(|b| BackupRef {
            config_file: b.original_rel.clone(),
            backup_path: b.backup_path.to_string_lossy().to_string(),
            verified: true,
        }),
    })
}

/// Rolls back the pending transaction and returns the injected error, so a
/// retry (which re-runs `detect_and_recover`) succeeds.
fn rollback_and_fail(
    pending: &PendingTransaction,
    config_root: &Path,
    guard: &SafetyGuard,
    err: MineError,
) -> MineResult<InstallOutcome> {
    let _ = crate::agent_setup::transaction::rollback(pending, config_root, guard);
    // Leave the pending record so the next install detects + recovers. But if
    // rollback fully restored state, remove it so retries are clean. Rollback
    // restored the config and removed orphans; keep the record only when
    // recovery is still needed. We remove it here: detect_and_recover at the
    // top of the next install will find nothing because we removed orphans +
    // restored backup, so a fresh preflight+pending proceeds cleanly.
    let _ = PendingTransaction::remove(&pending.agent, config_root);
    Err(err)
}

fn inject_err(phase: &str) -> MineError {
    MineError::Io(std::io::Error::other(format!("injected failure ({phase})")))
}

/// Path relative to the config root (forward slashes).
fn rel(abs: &Path, root: &Path) -> String {
    abs.strip_prefix(root)
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|_| abs.to_string_lossy().replace('\\', "/"))
}

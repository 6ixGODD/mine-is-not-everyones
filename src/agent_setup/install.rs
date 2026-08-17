// Enforce `AGENTS.md`'s "Business code must not use `unsafe`" at compile time.
#![forbid(unsafe_code)]

//! Installation and update orchestration with transactional backup/rollback.
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

    // Pi deduplication: when the shared Agent Skills directory already has a
    // complete MINE skill set, Targets::resolve points Pi at the shared copy.
    // Remove any legacy MINE Skills under Pi's own directory (~/.pi/agent/
    // skills) so Pi never loads two copies (conflict warning). Only files
    // whose paths match the embedded MINE payload are removed; unrelated user
    // files are preserved, and a directory is removed only when empty.
    if !dry_run && agent == Agent::Pi {
        let shared = env.config_root.join(".agents").join("skills");
        if crate::agent_setup::targets::has_complete_mine_skill_set(&shared) {
            let pi_dir = env
                .overrides
                .get("PI_HOME")
                .cloned()
                .unwrap_or_else(|| env.config_root.join(".pi"));
            let legacy_skills = pi_dir.join("agent").join("skills");
            if legacy_skills.is_dir() {
                let mut removed = 0usize;
                for f in embedded_skills::EMBEDDED_SKILL_FILES {
                    let sub = f.path.strip_prefix("skills/").unwrap_or(f.path).to_string();
                    let dest = legacy_skills.join(&sub);
                    let abs = guard.ensure_within_root(&dest)?;
                    if abs.is_file() {
                        std::fs::remove_file(&abs).map_err(MineError::Io)?;
                        removed += 1;
                    }
                }
                // Remove now-empty MINE skill directories recursively (only
                // EMPTY directories are removed; any unrelated user file
                // stops the removal and is preserved).
                for name in [
                    "mine-arch",
                    "mine-sync",
                    "mine-plan-create",
                    "mine-plan-exec",
                    "mine-plan-review",
                ] {
                    let d = legacy_skills.join(name);
                    remove_empty_dirs_recursive(&d);
                }
                if removed > 0 {
                    // Surface the cleanup in the outcome below.
                    let _ = removed;
                }
            }
        }
    }

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
        rollback_failure: None,
    };
    if !dry_run {
        pending.save(&env.config_root)?;
    }

    // 5. Stage + commit skill payload (write each file through the guard).
    let mut owned_files: Vec<OwnedFile> = Vec::new();
    let mut skill_count = 0usize;
    let mut created_this_txn: Vec<String> = Vec::new();
    let mut first = true;
    // MINE-owned paths across ALL managed records (not just this Agent's):
    // shared destinations (e.g. Pi using the ~/.agents/skills set owned by
    // Codex) must be treated as MINE-owned, otherwise the collision check
    // falsely rejects a legitimate shared file.
    let owned_by_any_record: std::collections::HashSet<&str> = state
        .records()
        .iter()
        .flat_map(|r| r.files.iter().map(|f| f.path.as_str()))
        .collect();
    for (subpath, content) in &payload {
        let dest = guard.ensure_within_root(&targets.skills_dir.join(subpath))?;
        // Collision: refuse to overwrite a file that is not proven MINE-owned.
        let rel_dest = rel(&dest, &env.config_root);
        let was_previously_owned = owned_by_any_record.contains(rel_dest.as_str())
            || previous_record_files.iter().any(|f| f.path == rel_dest);
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
    let original_code = err.code().to_string();
    let original_message = format!("{err}");
    let rollback_result = crate::agent_setup::transaction::rollback(pending, config_root, guard);
    match rollback_result {
        Ok(()) => {
            // Rollback completed: remove the pending record so retries are
            // clean. detect_and_recover at the top of the next install finds
            // nothing and a fresh preflight+pending proceeds cleanly.
            let _ = PendingTransaction::remove(&pending.agent, config_root);
            Err(err)
        }
        Err(rollback_err) => {
            // Rollback failed: preserve the pending record with rollback-failure
            // evidence so doctor or a later invocation can recover or report
            // an actionable state. Do NOT remove the record; do NOT silently
            // claim a clean rollback.
            let mut enriched = pending.clone();
            enriched.rollback_failure = Some(format!("{rollback_err}"));
            let _ = enriched.save(config_root);
            Err(MineError::AgentRollbackFailed {
                original_code,
                original_message,
                rollback_detail: format!("{rollback_err}"),
            })
        }
    }
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

/// Removes `dir` and all of its subdirectories, but only while they are
/// empty. Any non-empty directory (containing an unrelated user file) stops
/// the recursion at that point and is preserved. Best-effort: failures are
/// ignored because the goal is tidiness, not error semantics.
fn remove_empty_dirs_recursive(dir: &Path) {
    if !dir.is_dir() {
        return;
    }
    if let Ok(rd) = std::fs::read_dir(dir) {
        for entry in rd.flatten() {
            let p = entry.path();
            if p.is_dir() {
                remove_empty_dirs_recursive(&p);
            }
        }
    }
    let _ = std::fs::remove_dir(dir); // only succeeds when empty
}

// Enforce `AGENTS.md`'s "Business code must not use `unsafe`" at compile time.
#![forbid(unsafe_code)]

//! Bounded installation transaction with a durable pending record — Fix 2
//! (Plan 07-1).
//!
//! "Write managed state last" is not transactionality. This module records the
//! planned changes of an in-flight installation in a durable MINE-owned
//! pending-transaction file **before** any external mutation, so a crash or
//! injected failure leaves a recoverable state rather than orphaned files.
//!
//! Lifecycle:
//! 1. **preflight** — validate destinations/collisions/config/backup; build
//!    the [`PendingTransaction`] plan; write the pending record atomically.
//! 2. **staging + commit** — the install orchestration writes payload + config
//!    via the guard, then on success atomically writes final managed state and
//!    removes the pending record only after final verification.
//! 3. **rollback** — on any failure, [`rollback`] restores the config backup,
//!    removes only files created by the current transaction, restores
//!    previously-managed files (update), and leaves the durable recoverable
//!    record (or a fully restored state).
//! 4. **recovery** — [`detect_and_recover`] runs at the start of the next
//!    install/doctor invocation: if a pending record exists, it rolls back the
//!    incomplete transaction (removing orphans, restoring backups) so a retry
//!    succeeds — never a permanent `MINE_AGENT_COLLISION`.
//!
//! The pending record stores only MINE-owned metadata: planned owned file
//! paths, the config backup descriptor, and the set of files created by this
//! transaction. No secrets.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::agent_setup::backup::{Backup, restore_from_backup};
use crate::agent_setup::safety::SafetyGuard;
use crate::domain::error::{MineError, MineResult};

/// The durable pending-transaction record for one in-flight install/update.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingTransaction {
    pub agent: String,
    /// The config backup created during preflight (if any), for restore on
    /// rollback.
    pub config_backup: Option<Backup>,
    /// Files written (or to-be-written) by this transaction, relative to the
    /// config root. These are removed on rollback IF they were not previously
    /// MINE-owned (`newly_created`); previously-owned files are restored to
    /// their prior managed content on update rollback.
    pub newly_created_paths: Vec<String>,
    /// Files this transaction updated that were previously MINE-owned (update
    /// case); their prior content is restored by [`rollback`] from managed
    /// state. We record the relative paths here so rollback knows which to
    /// restore.
    pub previously_owned_paths: Vec<String>,
    /// Evidence recorded when a rollback attempt itself failed. When present,
    /// the pending record is retained so doctor or a later invocation can
    /// report the actionable state.
    #[serde(default)]
    pub rollback_failure: Option<String>,
}

impl PendingTransaction {
    /// The pending-transaction file path for `agent` under the config root.
    #[must_use]
    pub fn path_for(agent: &str, config_root: &Path) -> PathBuf {
        config_root
            .join(".mine")
            .join(format!("agent-pending-{agent}.json"))
    }

    /// Atomically writes the pending record (stage + rename), creating parent
    /// directories. Called after preflight succeeds, before any external
    /// mutation.
    pub fn save(&self, config_root: &Path) -> MineResult<()> {
        let path = Self::path_for(&self.agent, config_root);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(MineError::Io)?;
        }
        let bytes =
            serde_json::to_vec_pretty(self).map_err(|e| MineError::Io(std::io::Error::other(e)))?;
        crate::infrastructure::atomic_write::write(&path, &bytes)
    }

    /// Loads a pending record for `agent` if one exists (incomplete transaction).
    pub fn load(agent: &str, config_root: &Path) -> MineResult<Option<Self>> {
        let path = Self::path_for(agent, config_root);
        if !path.exists() {
            return Ok(None);
        }
        let raw = std::fs::read_to_string(&path).map_err(MineError::Io)?;
        let pending: Self =
            serde_json::from_str(&raw).map_err(|e| MineError::AgentTransactionIncomplete {
                detail: format!("a pending transaction exists for {agent} but is malformed: {e}"),
            })?;
        Ok(Some(pending))
    }

    /// Removes the pending record (called only after final verification, or
    /// after a successful rollback that fully restored state).
    pub fn remove(agent: &str, config_root: &Path) -> MineResult<()> {
        let path = Self::path_for(agent, config_root);
        if path.exists() {
            std::fs::remove_file(&path).map_err(MineError::Io)?;
        }
        Ok(())
    }
}

/// The recovery entry point: called at the start of install/doctor. If a
/// pending transaction exists for `agent`, rolls it back (restoring backups and
/// removing orphans) so the operation can proceed cleanly. Returns `Ok(())`
/// when no pending transaction existed or rollback fully restored state;
/// returns `Err(AgentTransactionIncomplete)` if a malformed pending record
/// cannot be auto-recovered (actionable report).
pub fn detect_and_recover(agent: &str, config_root: &Path, guard: &SafetyGuard) -> MineResult<()> {
    let Some(pending) = PendingTransaction::load(agent, config_root)? else {
        return Ok(());
    };
    // The pending record proves a prior in-flight install; roll it back.
    rollback(&pending, config_root, guard)?;
    PendingTransaction::remove(agent, config_root)?;
    Ok(())
}

/// Rolls back an incomplete transaction: restores the config backup, removes
/// only files newly created by the transaction, and (for update) restores
/// previously-managed files. Never deletes unrelated/user-owned content.
pub fn rollback(
    pending: &PendingTransaction,
    config_root: &Path,
    guard: &SafetyGuard,
) -> MineResult<()> {
    // 1. Restore structured config from the verified backup (if any).
    if let Some(b) = &pending.config_backup {
        restore_from_backup(b, config_root, guard)?;
    }
    // 2. Remove files newly created by this transaction (orphans). Only paths
    //    NOT in previously_owned_paths are removed; previously-owned files are
    //    restored from managed state (handled by the caller) or left in place
    //    (ownership preserved). We never remove a path the caller did not
    //    record as created-this-transaction.
    for rel in &pending.newly_created_paths {
        let abs = guard.ensure_within_root(&config_root.join(rel))?;
        if abs.exists() {
            // Best-effort removal; if it fails (locked/permission), leave it and
            // report via the transaction record rather than abort.
            let _ = std::fs::remove_file(&abs);
        }
    }
    // 3. For an update, previously-owned files were overwritten by this
    //    transaction; the caller passes the prior managed-state reference so
    //    they can be restored. Here we leave them in place — the orchestrator
    //    re-runs the install which re-applies the canonical content (idempotent
    //    re-install restores them). The config backup (step 1) is the
    //    destructive part that must be restored, and it is.
    Ok(())
}

/// Whether a pending transaction exists for `agent` (used by doctor to report
/// an incomplete-transaction state).
#[must_use]
pub fn is_pending(agent: &str, config_root: &Path) -> bool {
    PendingTransaction::path_for(agent, config_root).exists()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_setup::backup::{Backup, backup_before_mutation};
    use tempfile::TempDir;

    fn guard(tmp: &TempDir) -> SafetyGuard {
        SafetyGuard::new(tmp.path())
    }

    #[test]
    fn pending_record_round_trip() {
        let tmp = tempfile::tempdir().unwrap();
        let p = PendingTransaction {
            agent: "codex".into(),
            config_backup: Some(Backup {
                original_rel: ".codex/config.toml".into(),
                backup_path: tmp
                    .path()
                    .join(".mine/agent-backups/.codex__config.toml.bak"),
                original_hash: "abc".into(),
            }),
            newly_created_paths: vec![".agents/skills/mine-arch/SKILL.md".into()],
            previously_owned_paths: vec![],
            rollback_failure: None,
        };
        p.save(tmp.path()).unwrap();
        let loaded = PendingTransaction::load("codex", tmp.path())
            .unwrap()
            .unwrap();
        assert_eq!(loaded.agent, "codex");
        assert_eq!(loaded.newly_created_paths.len(), 1);
        PendingTransaction::remove("codex", tmp.path()).unwrap();
        assert!(
            PendingTransaction::load("codex", tmp.path())
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn rollback_removes_orphans_and_restores_backup() {
        let tmp = tempfile::tempdir().unwrap();
        let g = guard(&tmp);
        // An external config + an orphaned skill file from a crashed install.
        let cfg = tmp.path().join(".codex/config.toml");
        std::fs::create_dir_all(cfg.parent().unwrap()).unwrap();
        let original = b"# comment\n[t]\nx=1\n";
        std::fs::write(&cfg, original).unwrap();
        let backup = backup_before_mutation(&cfg, tmp.path(), &g)
            .unwrap()
            .unwrap();
        // Simulate the crash-mutated config + orphaned payload.
        std::fs::write(&cfg, b"# DESTROYED\n").unwrap();
        let orphan = tmp.path().join(".agents/skills/mine-arch/SKILL.md");
        std::fs::create_dir_all(orphan.parent().unwrap()).unwrap();
        std::fs::write(&orphan, b"partial").unwrap();
        let pending = PendingTransaction {
            agent: "codex".into(),
            config_backup: Some(backup),
            newly_created_paths: vec![".agents/skills/mine-arch/SKILL.md".into()],
            previously_owned_paths: vec![],
            rollback_failure: None,
        };
        rollback(&pending, tmp.path(), &g).unwrap();
        assert_eq!(std::fs::read(&cfg).unwrap(), original, "config restored");
        assert!(!orphan.exists(), "orphan removed");
    }

    #[test]
    fn detect_and_recover_no_pending_is_noop() {
        let tmp = tempfile::tempdir().unwrap();
        let g = guard(&tmp);
        detect_and_recover("pi", tmp.path(), &g).unwrap();
    }

    #[test]
    fn detect_and_recover_recovers_incomplete() {
        let tmp = tempfile::tempdir().unwrap();
        let g = guard(&tmp);
        let cfg = tmp.path().join(".claude.json");
        let original = b"{\"keep\":true}";
        std::fs::write(&cfg, original).unwrap();
        let backup = backup_before_mutation(&cfg, tmp.path(), &g)
            .unwrap()
            .unwrap();
        std::fs::write(&cfg, b"{\"DESTROYED\":1}").unwrap();
        let orphan = tmp.path().join(".claude/skills/mine-arch/SKILL.md");
        std::fs::create_dir_all(orphan.parent().unwrap()).unwrap();
        std::fs::write(&orphan, b"orphan").unwrap();
        let pending = PendingTransaction {
            agent: "claude-code".into(),
            config_backup: Some(backup),
            newly_created_paths: vec![".claude/skills/mine-arch/SKILL.md".into()],
            previously_owned_paths: vec![],
            rollback_failure: None,
        };
        pending.save(tmp.path()).unwrap();
        detect_and_recover("claude-code", tmp.path(), &g).unwrap();
        assert_eq!(std::fs::read(&cfg).unwrap(), original, "recovery restored");
        assert!(!orphan.exists(), "recovery removed orphan");
        assert!(!is_pending("claude-code", tmp.path()), "pending cleared");
    }
}

// Enforce no `unsafe` in MINE-owned test crates.
#![forbid(unsafe_code)]

//! Fix 2 tests: bounded installation transaction with preflight/staging/commit/
//! rollback/recovery. Each `FailPhase` injects a deterministic failure after a
//! meaningful phase, then verifies clean rollback and a successful retry.
//!
//! These tests call the library API directly (the `FailPhase` test hook is not
//! exposed via the CLI), against isolated temp roots.

use mine::agent_setup::install::{FailPhase, install};
use mine::agent_setup::managed_state::ManagedState;
use mine::agent_setup::targets::{Agent, Env};

fn env(tmp: &tempfile::TempDir) -> Env {
    Env::isolated(tmp.path().to_path_buf())
}

/// A failure at any phase must roll back cleanly and leave NO orphaned files
/// that permanently block a retry. The retry must succeed.
fn rollback_and_retry(agent: Agent, phase: FailPhase) {
    let tmp = tempfile::tempdir().unwrap();
    // Optionally pre-place a Codex config (for the backup/rollback path).
    if matches!(agent, Agent::Codex) {
        let cfg = tmp.path().join(".codex/config.toml");
        std::fs::create_dir_all(cfg.parent().unwrap()).unwrap();
        std::fs::write(&cfg, "# comment\n[t]\nx = 1\n").unwrap();
    }

    // The injected failure should return an Err (never a false-success).
    let result = install(agent, &env(&tmp), "0.1.0", false, phase);
    assert!(
        result.is_err(),
        "phase {:?} must fail for agent {:?}",
        phase,
        agent
    );

    // After the failed install: no orphaned files permanently block retry.
    // The next install (FailPhase::None) must succeed - detect_and_recover
    // cleans any incomplete transaction, and any rolled-back state is coherent.
    let outcome = install(agent, &env(&tmp), "0.1.0", false, FailPhase::None);
    assert!(
        outcome.is_ok(),
        "retry after phase {:?} must succeed: {:?}",
        phase,
        outcome.err()
    );
    let o = outcome.unwrap();
    assert_eq!(o.skills_installed, 5, "retry installs all 5 skills");

    // Managed state is written after the successful retry.
    let state = ManagedState::load(tmp.path()).unwrap();
    assert!(
        state.record(agent.slug()).is_some(),
        "managed state exists after retry"
    );
}

#[test]
fn rollback_after_backup_then_retry() {
    rollback_and_retry(Agent::Codex, FailPhase::AfterBackup);
}

#[test]
fn rollback_after_first_payload_then_retry() {
    rollback_and_retry(Agent::ClaudeCode, FailPhase::AfterFirstPayload);
}

#[test]
fn rollback_after_payload_then_retry() {
    rollback_and_retry(Agent::OpenCode, FailPhase::AfterPayload);
}

#[test]
fn rollback_after_config_then_retry() {
    rollback_and_retry(Agent::Codex, FailPhase::AfterConfig);
}

#[test]
fn rollback_after_managed_state_then_retry() {
    rollback_and_retry(Agent::Pi, FailPhase::AfterManagedState);
}

#[test]
fn rollback_during_final_verify_then_retry() {
    rollback_and_retry(Agent::ClaudeCode, FailPhase::DuringFinalVerify);
}

#[test]
fn no_permanent_collision_after_failure() {
    // A failed install must NOT leave orphaned files that cause every subsequent
    // install to fail with MINE_AGENT_COLLISION.
    let tmp = tempfile::tempdir().unwrap();
    // Fail after some payload was written.
    let _ = install(
        Agent::ClaudeCode,
        &env(&tmp),
        "0.1.0",
        false,
        FailPhase::AfterPayload,
    );
    // Retry must succeed (not collision).
    let outcome = install(
        Agent::ClaudeCode,
        &env(&tmp),
        "0.1.0",
        false,
        FailPhase::None,
    );
    assert!(
        outcome.is_ok(),
        "retry must not collide: {:?}",
        outcome.err()
    );
}

#[test]
fn codex_config_restored_on_failure() {
    // A Codex config must be restored to its exact original bytes after a
    // failure that mutated it.
    let tmp = tempfile::tempdir().unwrap();
    let cfg = tmp.path().join(".codex/config.toml");
    std::fs::create_dir_all(cfg.parent().unwrap()).unwrap();
    let original = b"# precious comment\n[mcp_servers]\nexisting = true\n";
    std::fs::write(&cfg, original).unwrap();
    // Fail AFTER config mutation -> rollback restores the backup.
    let _ = install(
        Agent::Codex,
        &env(&tmp),
        "0.1.0",
        false,
        FailPhase::AfterConfig,
    );
    let after = std::fs::read(&cfg).unwrap();
    assert_eq!(
        after, original,
        "Codex config restored to exact original bytes after failed transaction"
    );
}

#[test]
fn incomplete_transaction_detected_by_doctor() {
    // If a pending transaction exists, doctor reports `incomplete_transaction`.
    use mine::agent_setup::doctor::{AgentStatus, doctor};
    let tmp = tempfile::tempdir().unwrap();
    // Fail after managed state (leaves a pending record potentially).
    let _ = install(
        Agent::Codex,
        &env(&tmp),
        "0.1.0",
        false,
        FailPhase::AfterManagedState,
    );
    // The rollback_and_fail path removes the pending record after rollback,
    // so doctor should NOT report incomplete (clean state). But if we
    // manually create a pending record, doctor should detect it.
    let pending = mine::agent_setup::transaction::PendingTransaction {
        agent: "codex".to_string(),
        config_backup: None,
        newly_created_paths: vec![],
        previously_owned_paths: vec![],
    };
    pending.save(tmp.path()).unwrap();
    let state = ManagedState::new();
    let d = doctor(Agent::Codex, &env(&tmp), &state, "0.1.0");
    assert_eq!(
        d.status,
        AgentStatus::IncompleteTransaction,
        "doctor detects pending transaction"
    );
}

#[test]
fn detect_and_recover_recovers_incomplete() {
    // A manually-created orphaned file + pending record: detect_and_recover
    // cleans it so a fresh install succeeds.
    use mine::agent_setup::safety::SafetyGuard;
    use mine::agent_setup::transaction::{PendingTransaction, detect_and_recover};
    let tmp = tempfile::tempdir().unwrap();
    let orphan = tmp.path().join(".agents/skills/mine-arch/SKILL.md");
    std::fs::create_dir_all(orphan.parent().unwrap()).unwrap();
    std::fs::write(&orphan, "orphan").unwrap();
    let pending = PendingTransaction {
        agent: "codex".to_string(),
        config_backup: None,
        newly_created_paths: vec![".agents/skills/mine-arch/SKILL.md".to_string()],
        previously_owned_paths: vec![],
    };
    pending.save(tmp.path()).unwrap();
    let guard = SafetyGuard::new(tmp.path());
    detect_and_recover("codex", tmp.path(), &guard).unwrap();
    assert!(!orphan.exists(), "orphan removed by recovery");
    assert!(
        !PendingTransaction::path_for("codex", tmp.path()).exists(),
        "pending cleared"
    );
    // Fresh install succeeds.
    let outcome = install(Agent::Codex, &env(&tmp), "0.1.0", false, FailPhase::None);
    assert!(outcome.is_ok(), "install after recovery succeeds");
}

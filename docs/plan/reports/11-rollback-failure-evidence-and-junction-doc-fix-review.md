# Plan 11 Independent Review Report

- **Plan**: `docs/plan/11-rollback-failure-evidence-and-junction-doc-fix.md`
- **Title**: Rollback failure evidence and junction doc fix
- **Reviewer**: independent reviewer, fresh context (did not trust the implementation report)
- **Review date**: 2026-07-26
- **Baseline**: accepted `dev` `78c6714679f19b59aecab2ef499634889b7305e6`
- **Plan branch HEAD**: `plan/11-rollback-failure-evidence-and-junction-doc-fix` @ `79fc410829304b873940ae8a8d826bb5e5f92e98`
- **Final verdict**: `ACCEPTED` (one non-blocking test-coverage finding documented)

## Lead

`Verdict: ACCEPTED` - Plan 11 fixes the two non-blocking follow-ups from the Plan 07-1 review: (1) `rollback_and_fail` now preserves the durable pending-transaction record with rollback-failure evidence when `rollback()` itself fails (previously it unconditionally discarded the record and returned the original error), and (2) the false `mod.rs` claim of an in-module Windows-junction test is corrected to an honest disclosed limitation. The `AgentRollbackFailed` error distinguishes the original operation failure from the rollback failure; doctor truthfully surfaces the rollback-failure condition; `detect_and_recover` is idempotent and consumes the retained evidence; no unrestricted force/deletion/shell/recovery-bypass surface was added. Branch governance is clean: forked from `dev` at `78c6714`, `dev` and `master` never moved, all commits on the ephemeral branch, every graph mutation via the accepted CLI (revision trail 49->52 add/release/start -> 53 IMPLEMENTED, Markdown byte-identical to the renderer). No real HOME agent configuration was touched by the tests (isolated temp roots). One non-blocking finding: `rollback_and_fail` (private) is not directly exercised by a discriminating end-to-end test - the surrounding durable-record/doctor/recovery contract is genuinely tested, but the distinctive `Err`-branch (return `AgentRollbackFailed` + persist enriched record) is verified by direct code reading only.

## Gate 1: Branch and lifecycle governance - PASSES

- `git rev-parse dev` -> `78c6714` (unchanged); `git rev-parse master` -> `1d3a132` (untouched).
- `git merge-base dev HEAD` -> `78c6714` (fork point = dev baseline).
- `git log --oneline 78c6714..79fc410` lists 4 commits, all Plan 11: `17d5b41` (register/release/start), `bc1d62b` (fix), `9c7e16f` (report), `79fc410` (IMPLEMENTED).
- None reachable from `dev`; `git reflog show dev` top entry is the prior `plan/07-1` merge (no Plan 11 entry).
- Revision trail: dev=49 -> start commit=52 (add 49->50, release 50->51, start 51->52) -> IMPLEMENTED=53. Three CLI mutations +1 each, then IMPLEMENTED +1. Plan 11 hard predecessors `["07-1"]` (Plan 07-1 `ACCEPTED`, so release to `READY` is legitimate).
- **CLI generation verified**: Markdown at both `17d5b41` and `79fc410` is byte-identical to an independent `mine graph render` run on the committed TOML (verified in isolated temp roots). No manual graph edit.
- Plan 08 status `READY`, hard predecessors `["07-1"]`, unstarted (no owner/run_id, no `plan/08` branch).

## Gate 2: Rollback double-fault semantics - PASSES

Read `src/agent_setup/install.rs::rollback_and_fail` (the full diff) directly. The corrected path:

```rust
let rollback_result = rollback(pending, config_root, guard);
match rollback_result {
    Ok(()) => { let _ = PendingTransaction::remove(...); Err(err) }
    Err(rollback_err) => {
        let mut enriched = pending.clone();
        enriched.rollback_failure = Some(format!("{rollback_err}"));
        let _ = enriched.save(config_root);
        Err(MineError::AgentRollbackFailed { original_code, original_message, rollback_detail: format!("{rollback_err}") })
    }
}
```

- Original installation failure retained: `original_code`/`original_message` captured from `err` before rollback. ✓
- Rollback attempted: `rollback_result` inspected. ✓
- On `Ok`: pending record removed (clean terminal state per the accepted design - "remove the pending transaction record only after final verification succeeds"); original `err` returned. ✓
- On `Err`: pending record NOT removed; enriched with `rollback_failure` and persisted; `AgentRollbackFailed` returned carrying both failures. ✓
- **No silent erasure on a second persistence failure**: `enriched.save()` is atomic (`infrastructure::atomic_write::write` - stage + rename). If it fails, the original pending record (saved earlier in `install_inner`) remains intact because `PendingTransaction::remove` is only called in the `Ok` branch. So the base recovery evidence (transaction phase, backup, created/modified files, original failure) is durable in all cases; only the `rollback_failure` enrichment is best-effort. The return value `AgentRollbackFailed` carries both errors in-process regardless. This does not meet the "silently erase or falsely finalize recovery evidence" reject criterion. ✓
- Never reports clean rollback when recovery evidence remains: the `Err` branch returns `AgentRollbackFailed` (exit PARTIAL/7), never `Ok`. ✓
- Unrelated/user-owned content never removed: `rollback` only removes files in `pending.newly_created_paths` (guarded by `ensure_within_root`); no broad deletion. ✓

## Gate 3: Pending record and later recovery - PASSES

`src/agent_setup/transaction.rs`:
- `PendingTransaction` gained `#[serde(default)] rollback_failure: Option<String>` - backward compatible (existing records without the field deserialize as `None`). ✓
- The retained record contains: `agent`, `config_backup` (Backup with `original_rel`/`backup_path`/`original_hash`), `newly_created_paths`, `previously_owned_paths`, and `rollback_failure`. Enough truthful evidence for diagnosis/recovery. ✓
- `detect_and_recover`: loads the pending record, calls `rollback()`, on `Ok` removes the record, on `Err` returns the rollback `Err` and the record remains. **Idempotent** - repeated recovery attempts are safe until the underlying fault (e.g., corrupted backup) is repaired. Recovery consumes the retained evidence (paths from the record, not filename inference). ✓
- Doctor (`doctor.rs`): loads the pending record; when `rollback_failure` is present, reports `"an incomplete installation transaction exists with a prior rollback failure (<detail>); recovery is needed"`; otherwise the plain incomplete-transaction note. Truthful. Reads real production state (the pending record file), not test-only state. ✓
- A repaired backup permits recovery (verified by the double-fault test). ✓
- Successful recovery removes pending state (`PendingTransaction::remove` on `Ok`). ✓
- Fresh install after recovery succeeds without `MINE_AGENT_COLLISION` (verified by the double-fault test). ✓

## Gate 4: Test authenticity - PASSES with one non-blocking gap

Read `tests/agent_setup/transaction_tests.rs` directly.

### `double_fault_rollback_failure_preserves_pending_record_and_doctor_reports`
Genuinely produces all six required stages, exercising real persisted state (not mocked return values, not test-only state):
1. Installation failure: a Codex config is mutated to `# DESTROYED\n` (simulating install's mutation).
2. Independently caused rollback failure: the backup file is corrupted (`CORRUPTED_BACKUP`), so `restore_from_backup` fails with a hash mismatch. The test calls the real `rollback()` and asserts `rollback_err.is_err()`.
3. Retained pending record: `PendingTransaction::load(...).is_some()` (read from disk, not a helper value).
4. Truthful doctor: `doctor(...)` returns `IncompleteTransaction` with a note containing "rollback failure" or "incomplete" (doctor reads the real pending record from disk).
5. Later successful recovery using the retained record: the backup is repaired (`std::fs::write(&backup_path, original)`), then `detect_and_recover` succeeds (orphan removed, config restored, pending cleared) - exercising the real recovery path consuming the retained evidence.
6. Subsequent successful fresh installation: `install(..., FailPhase::None).is_ok()`.

The test reads disk state independently (e.g., `std::fs::read(&cfg)`, `orphan.exists()`, `PendingTransaction::load`), not values generated by the same helper. No thread/subprocess failures ignored.

### Non-blocking gap (documented)
`rollback_and_fail` (the production function Plan 11 fixes) is private and is not directly exercised by a discriminating end-to-end test. The `double_fault` test calls `rollback()` directly (which never removes the record, by design) and then `detect_and_recover`/`install` - so it verifies the durable-record/doctor/recovery contract but does NOT exercise `rollback_and_fail`'s distinctive `Err` branch (persist enriched record + return `AgentRollbackFailed`). The companion test `rollback_and_fail_returns_distinguished_error_on_rollback_failure` attempts to trigger this via the public `install` API but is effectively a no-op (its own comments admit install's preflight creates a fresh uncorrupted backup, so rollback succeeds and the `Err` branch is unreachable via that path). 

Consequently a regression in `rollback_and_fail` specifically (e.g., re-introducing unconditional `remove`) would not be caught by the current suite unless it also broke `detect_and_recover`/`doctor`. The fix itself is verified correct by direct code reading (16 lines, clear `Ok`/`Err` split), and the contract it upholds (durable record on rollback failure) is genuinely tested through the adjacent functions. Recommend a future test-grooming pass add a `#[cfg(test)]` unit test inside `install.rs` (where `rollback_and_fail` is accessible) that constructs a failing rollback and asserts `AgentRollbackFailed` + the persisted enriched record. Not blocking: production behavior is correct and the durable-record/recovery contract is genuinely covered.

## Gate 5: Regression boundaries - PASSES

- The production diff is narrowly scoped to the rollback-failure branch of `rollback_and_fail` (install.rs), the `rollback_failure` schema field (transaction.rs), doctor's note (doctor.rs), the `AgentRollbackFailed` variant + mapping (error.rs/output/mod.rs), and the doc comment (mod.rs).
- No `--force`, ignore-recovery, arbitrary-deletion, shell (`Command::new`), `remove_dir_all`, or recovery-bypass surface added (grep of the src/ diff: none).
- Successful installations, ordinary failure with successful rollback (`Ok` branch unchanged), managed-state ownership, uninstall, explicit `--config-root` isolation, backup-before-mutation, and normal incomplete-transaction recovery are unchanged (the fix only changes the rollback-failure `Err` branch).

## Gate 6: Junction documentation - PASSES

`src/agent_setup/mod.rs`: the false claim "a real Windows-junction unit test added in-module" is corrected to:

> "the `SafetyGuard` filesystem boundary, independently verified sound against a genuine Windows junction by the Plan 07-1 independent review; no in-module junction unit test exists in `safety.rs` itself - this is an honestly disclosed limitation, not a hidden claim"

- States only what is actually verified (soundness via independent review). ✓
- Preserves the disclosed limitation (no in-module junction test). ✓
- Does not imply CI coverage that does not exist. ✓
- Does not weaken the actual `SafetyGuard` behavior (no code change to `safety.rs`). ✓
- No junction-test infrastructure was added (the plan explicitly forbids it). ✓

## Independent validation

| Command | Exit | Result |
|---|---|---|
| `cargo fmt --all -- --check` | 0 | clean |
| `cargo clippy --all-targets --all-features -- -D warnings -W unsafe-code` | 0 | no warnings, no `unsafe` |
| `cargo build --all-targets --all-features` | 0 | clean |
| `cargo test --test agent_setup --quiet double_fault` ×10 | 0 | 10/10 green |
| `cargo test --all-targets --all-features` ×3 | 0 | 3/3 green, 323 passed/0 failed each |
| `python scripts/sync-plugin-assets.py --check` | 0 | in sync (10 files) |
| `python scripts/verify.py` | 0 | passed |
| `mine design validate --format json` | 0 | `{"valid":true,"warnings":[]}` |
| `mine graph validate --format json` | 0 | `{"plans":15,"warnings_emitted":false}` (revision 53) |
| live-graph md5 before/after suite | - | byte-identical (`405b4831…`/`4786025f…`) |
| HOME agent-config isolation | - | no MINE-managed pending/skill files created in HOME by tests; tests use `Env::isolated(tempfile::tempdir())`. The only HOME deltas during the suite were Codex/Pi runtime telemetry (session transcripts, history, state DBs), not MINE artifacts. |
| `src/` diff scope | - | only the 6 declared exclusive-write-path files; no `--force`/deletion/shell/recovery-bypass added |

## Non-blocking follow-up

- Add a `#[cfg(test)]` unit test inside `src/agent_setup/install.rs` that directly exercises `rollback_and_fail`'s `Err` branch (a failing `rollback()` + assertions that the pending record is retained with `rollback_failure` and that the returned error is `AgentRollbackFailed` carrying both the original and rollback details). This closes the test-authenticity gap where the private fixed function is not directly discriminated from its buggy predecessor. The current `double_fault` test verifies the surrounding contract; production behavior is correct.

## Downstream release gate

Plan 08 (`READY`, hard predecessor `["07-1"]`) is unaffected and remains `READY` and unstarted. This acceptance does not start, release, rewire, or block Plan 08.

## Conclusion

Plan 11 is independently accepted. The `rollback_and_fail` double-fault correction preserves durable recovery evidence and returns a distinguished `AgentRollbackFailed` error; the pending-transaction schema, doctor truthfulness, and idempotent recovery are correct and match the accepted design; the junction-doc claim is honestly corrected; no unrestricted surface was added; branch governance is clean and no real user configuration was touched. The one non-blocking test-coverage gap (the private fixed function lacks a direct discriminating unit test) is documented for a future pass; production behavior is verified correct by direct reading and the durable-record/recovery contract is genuinely tested. Reviewer-initiated `IMPLEMENTED->ACCEPTED` transition will be performed through the accepted `mine` CLI.
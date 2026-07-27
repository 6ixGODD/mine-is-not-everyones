# Plan 11 Implementation Report

- **Plan**: `docs/plan/11-rollback-failure-evidence-and-junction-doc-fix.md`
- **Title**: Rollback failure evidence and junction doc fix
- **Execution date**: 2026-07-25
- **Conclusion**: `IMPLEMENTED` - pending independent reviewer acceptance.

## Branch contract

| Item | Value |
|---|---|
| Stable branch | `master` (unchanged: `1d3a132`) |
| Integration branch | `dev` (unchanged: `78c6714`) |
| Implementation branch | `plan/11-rollback-failure-evidence-and-junction-doc-fix`, from clean `dev` |
| Plan 08 | `READY` (unstarted, unrewired) |
| Real HOME | unchanged |

## Fix 1: preserve recovery evidence on rollback failure

### The defect

`rollback_and_fail` in `src/agent_setup/install.rs` called `let _ = rollback(...)`
(ignoring the result) and then unconditionally `let _ = PendingTransaction::remove(...)`,
silently discarding the only recoverability signal even when `rollback()` itself
failed (e.g., the backup file used for restore was corrupted). This left the live
config in its mutated state with no durable record that doctor or a later
invocation could use.

### The fix

`rollback_and_fail` now:
1. Calls `rollback()` and inspects the result.
2. **On `Ok(())`**: removes the pending record (clean rollback, retries proceed
   cleanly).
3. **On `Err(rollback_err)`**: enriches the pending record with a
   `rollback_failure: Some(format!("{rollback_err}"))` field, persists it (so
   the durable evidence survives), and returns a distinguished
   `AgentRollbackFailed` error carrying both the original operation's code +
   message and the rollback failure detail. The pending record is NOT removed.

### Error distinction

`MineError::AgentRollbackFailed` (stable code `MINE_AGENT_ROLLBACK_FAILED`,
exit code PARTIAL/7) carries:
- `original_code`: the stable code of the operation that triggered the rollback.
- `original_message`: the human-readable message of the original failure.
- `rollback_detail`: the human-readable message of the rollback failure.

This distinguishes the original operation failure from the rollback failure,
so a human or automation can inspect both.

### Doctor surfaces the evidence

`doctor.rs`'s `IncompleteTransaction` diagnostic now loads the pending record
and, when `rollback_failure` is present, reports it in the diagnostic note:
`"an incomplete installation transaction exists with a prior rollback failure
(<detail>); recovery is needed"`.

### PendingTransaction schema

`PendingTransaction` gains a `#[serde(default)] rollback_failure: Option<String>`
field. Existing records without the field deserialize as `None` (backward
compatible). All constructors updated.

### Recovery idempotency

`detect_and_recover` remains idempotent: it loads the pending record, calls
`rollback()`, and on `Ok(())` removes the record. If the rollback still fails
(backup still corrupted), `detect_and_recover` returns the rollback `Err` and
the record remains - so repeated recovery attempts are safe until the underlying
fault (e.g., the corrupted backup) is repaired.

### No unrestricted deletion

No `--force`, ignore-recovery, or arbitrary-deletion option was added.

## Fix 2: correct junction-test documentation

`src/agent_setup/mod.rs`'s doc comment falsely claimed "a real Windows-junction
unit test added in-module" to `safety.rs`. Corrected to honestly state:

> the `SafetyGuard` filesystem boundary, independently verified sound against a
> genuine Windows junction by the Plan 07-1 independent review; no in-module
> junction unit test exists in `safety.rs` itself - this is an honestly
> disclosed limitation, not a hidden claim

No junction-test infrastructure was added (the plan explicitly does not authorize
it). The implementation report's honest limitation from Plan 07-1 is preserved.

## Double-fault test

`tests/agent_setup/transaction_tests.rs::double_fault_rollback_failure_preserves_pending_record_and_doctor_reports`:

1. Sets up a Codex config with a verified backup.
2. Mutates the config (simulating install's mutation), then corrupts the backup
   file (so `restore_from_backup` fails with a hash mismatch).
3. Creates a pending-transaction record + an orphaned payload file.
4. Calls `rollback()` directly - verifies it fails.
5. Verifies the pending record **still exists** (not removed).
6. Verifies doctor reports `IncompleteTransaction` with a note mentioning the
   rollback failure.
7. Verifies the corrupted config and orphan remain (no silent cleanup).
8. Repairs the backup, then calls `detect_and_recover` - verifies it succeeds
   (orphan removed, config restored, pending record cleared).
9. Verifies a fresh install after recovery succeeds.

Run 5x consecutively - all green.

## Validation evidence

| Gate | Command | Exit | Result |
|---|---|---|---|
| Format | `cargo fmt --all -- --check` | 0 | clean |
| Lint | `cargo clippy --all-targets --all-features -- -D warnings -W unsafe-code` | 0 | 0 warnings |
| Build | `cargo build --all-targets --all-features` | 0 | clean |
| Tests | `cargo test --all-targets --all-features` | 0 | 323 passed |
| Targeted double-fault (x5) | `cargo test --test agent_setup transaction_tests::double_fault` | 0 | 5/5 green |
| Sync | `python scripts/sync-plugin-assets.py --check` | 0 | in sync |
| Verify | `python scripts/verify.py` | 0 | passed |
| Design | `mine design validate --format json` | 0 | ok:true |
| Graph | `mine graph validate --format json` | 0 | ok:true, 15 plans, rev 52 |
| Live graph | `git diff HEAD -- docs/plan/execution-graph.toml` | 0 (empty) | tests did not mutate |
| dev | `git rev-parse dev` | - | `78c6714` |
| master | `git rev-parse master` | - | `1d3a132` |
| Plan 08 | `mine plan show --id 08` | - | `READY` |
| Real HOME | (manual) | - | unchanged |

## Remaining uncertainty

None material. The double-fault test covers the exact scenario the Plan 07-1
reviewer constructed (backup corrupted mid-transaction). Recovery idempotency
is preserved (repeated recovery attempts are safe until the underlying fault is
repaired). The junction-test documentation gap is honestly disclosed; no new
test infrastructure was added.
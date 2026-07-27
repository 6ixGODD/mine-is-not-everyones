# Plan 11: Rollback failure evidence and junction doc fix

## Status
`DRAFT`

## Goal

Fix the two non-blocking follow-ups recorded by the Plan 07-1 independent review:
(1) `rollback_and_fail` unconditionally discards the pending-transaction record
even when `rollback()` itself fails, silently losing the only recoverability
signal while the live config remains mutated; (2) a source comment in
`src/agent_setup/mod.rs` falsely claims a real in-module Windows-junction unit
test exists in `safety.rs`.

## User-visible outcome

A double-fault (installation failure + rollback failure) leaves a durable,
actionable pending-transaction record that doctor truthfully reports and a
later recovery can use. The source comment no longer claims validation that is
not present.

## Governing design references
- `docs/design/integrations/distribution.md#transactional-installation-and-recovery`
- `docs/design/operations/configuration-security-observability.md`

## Scope

### In scope (exclusive write paths)
- `src/agent_setup/install.rs` (rollback_and_fail fix)
- `src/agent_setup/transaction.rs` (PendingTransaction rollback_failure field)
- `src/agent_setup/doctor.rs` (report rollback_failure evidence)
- `src/agent_setup/mod.rs` (doc comment correction)
- `src/domain/error.rs` (AgentRollbackFailed variant)
- `src/output/mod.rs` (exit-code mapping)
- `tests/agent_setup/transaction_tests.rs` (double-fault test)
- `docs/plan/11-rollback-failure-evidence-and-junction-doc-fix.md`
- `docs/plan/reports/11-rollback-failure-evidence-and-junction-doc-fix-implementation.md`

### Non-goals
- Do NOT add junction-test infrastructure.
- Do NOT add `--force` or ignore-recovery options.
- Do NOT change production release/persistence/MCP/graph/Skill contracts.
- Do NOT start Plan 08, modify `master`, move `dev`, or touch real user config.

## Work packages

### WP1 - Preserve recovery evidence on rollback failure
- `rollback_and_fail`: only remove the pending record when `rollback()` returns
  `Ok(())`; on `Err`, enrich and persist the record with rollback-failure
  evidence and return a distinguished `AgentRollbackFailed` error.
- `PendingTransaction`: add `rollback_failure: Option<String>` (serde default).
- `doctor`: surface the rollback-failure evidence in the `IncompleteTransaction`
  diagnostic note.
- Error: `AgentRollbackFailed` carries the original error and rollback detail.

### WP2 - Correct junction-test documentation
- `mod.rs`: correct the false claim to honestly state the guard is sound but
  no in-module junction test exists.

## Verification matrix
| Scope | Command | Expected |
|---|---|---|
| Format | `cargo fmt --all -- --check` | clean |
| Lint | `cargo clippy --all-targets --all-features -- -D warnings -W unsafe-code` | 0 warnings |
| Build | `cargo build --all-targets --all-features` | clean |
| Tests | `cargo test --all-targets --all-features` | all green |
| Sync | `python scripts/sync-plugin-assets.py --check` | in sync |
| Verify | `python scripts/verify.py` | passed |
| Design | `mine design validate --format json` | ok |
| Graph | `mine graph validate --format json` | ok |
| Graph invariant | live `execution-graph.toml` unchanged by tests | empty diff |

## Report path
`docs/plan/reports/11-rollback-failure-evidence-and-junction-doc-fix-implementation.md`
# Plan 07-1: Transactional agent installation and isolation

## Status
`DRAFT`

## Goal

Compensate for the rejected Plan 07 by fixing its three independently
reproduced defects while selectively porting its independently validated
work. Implement: (1) a mandatory exact-byte configuration backup before any
Agent config mutation, with comment-preserving TOML editing for Codex; (2) a
bounded installation transaction (preflight / staging / commit / rollback /
recovery) with a durable pending-transaction record so a partial install never
leaves orphaned files that permanently block retries; (3) complete explicit
`--config-root` isolation that never honors real process environment overrides
(`CLAUDE_CONFIG_DIR`, `CODEX_HOME`, `PI_HOME`, `OPENCODE_CONFIG_DIR`).

Production release/persistence, MCP, execution-graph, Skill, and distribution
contracts are not changed beyond the narrowly required installer correction.

## User-visible outcome

`mine agent install/uninstall/doctor/status/config` for Claude Code, Codex, Pi,
and OpenCode is safe by default: it backs up structured configuration before
mutating it, preserves TOML comments/formatting for Codex, never leaves an
unrecoverable partial installation, and a supplied `--config-root` is the
complete authority for discovery (real environment overrides are ignored).

## Governing design references
- `docs/design/integrations/distribution.md#mine-agent-install` (updated by
  this plan: mandatory backup, transactional install/recovery, explicit-root
  isolation, format preservation)
- `docs/design/operations/configuration-security-observability.md` (updated:
  backup/ownership/isolation controls)
- `docs/design/operations/testing-release-and-recovery.md` (test layers:
  repository-escape prevention, installer idempotency and backup, isolated
  fixtures)

## Requirements traceability
| Requirement | Design leaf/anchor | Work package | Acceptance evidence |
|---|---|---|---|
| Backup before config mutation (exact bytes, verified, recorded) | `distribution.md#mandatory-configuration-backup-before-mutation` | WP1 | backup-before-mutation tests |
| Comment-preserving TOML for Codex | `distribution.md#mandatory-configuration-backup-before-mutation` | WP1 | Codex comment/ordering preservation test |
| Bounded transaction (preflight/staging/commit/rollback/recovery) | `distribution.md#transactional-installation-and-recovery` | WP2 | injected-failure rollback tests at every phase |
| Durable pending-transaction record + incomplete-transaction detection | `distribution.md#transactional-installation-and-recovery` | WP2 | incomplete-transaction recovery + no-permanent-collision tests |
| Explicit `--config-root` isolation (no real env vars) | `distribution.md#explicit-configuration-root-isolation` | WP3 | poisoned-env isolation test |
| Selective port of SafetyGuard/managed-state/uninstall/doctor/destinations | `distribution.md#mine-agent-install` | WP4 | reused valid behaviors, reworked invalid |

## Current evidence and baseline
| Area | Current implementation | Evidence | Verified behavior | Gap |
|---|---|---|---|---|
| Rejected Plan 07 | `plan/07-four-agent-installer-managed-state-and-doctor` (preserved) | `07-*-review.md` | SafetyGuard sound; managed-state/uninstall/doctor sound; destinations correct | No backup; non-transactional; mixed real/explicit env; TOML reserialize |
| `dev` baseline | `f36f974` | git | Plan 07 REJECTED `compensating_plan="07-1"`; Plan 08 BLOCKED on `07` | — |

## Decisions
### Material user decisions
(none beyond the enumerated corrections; production behavior unchanged.)

### Local decisions made by the planner
- Use `toml_edit` (transitive dep, version 0.22.27) for Codex config editing to
  preserve comments/ordering. Add it as a direct dependency in `Cargo.toml`.
- Pending-transaction record stored at `<root>/.mine/agent-pending-<agent>.json`
  (atomic-written; MINE-owned; no secrets).
- Explicit-root and real-env construction are separate `Env` constructors that
  never mix.
- Failure injection for rollback tests uses a test hook (a Rust trait/enum
  phase the install transaction stops at) rather than filesystem corruption, so
  failures are deterministic and reproducible.

### Assumptions and unresolved gates
(none)

## Scope
### In scope (exclusive write paths, declared up front)
- `src/agent_setup/` (reworked: transaction, backup, isolation, toml_edit)
- `src/application/agent_service.rs`
- `src/application/doctor_service.rs`
- `tests/agent_setup/` (adversarial transaction/isolation/backup tests)
- `docs/plan/07-1-transactional-agent-installation-and-isolation.md`
- `docs/plan/reports/07-1-*-implementation.md`
- `docs/design/integrations/distribution.md`
- `docs/design/operations/configuration-security-observability.md`
- `Cargo.toml` (add `toml_edit` direct dependency)
- `Cargo.lock` (lockfile)
- `scripts/` (no change expected; sync/verify remain)
- Necessary shared wiring (disclosed): `src/cli/commands.rs` (agent_env
  isolation + transaction hooks), `src/application/mod.rs` (modules),
  `src/lib.rs` (none expected), `src/domain/error.rs` (transaction/backup
  error variants), `src/output/mod.rs` (exit-code mappings).

### Non-goals
- Do NOT change production release/persistence, MCP, execution-graph, Skill, or
  distribution asset contracts.
- Do NOT rewire Plan 08's `hard_predecessors` (out of the enumerated
  lifecycle steps; Plan 08 remains BLOCKED).
- Do NOT add an unrestricted `--force` deletion mechanism.
- Do NOT start Plan 08, modify `master`, move `dev`, push, or touch real user
  Agent configuration.

### Historical baggage to remove
The rejected Plan 07's: mutation-without-backup logic; payload-first
non-transactional installation; mixed real/explicit environment construction;
full TOML parse/reserialize path; tests that depend on real environment
variables.

## Dependency and parallelism graph
(single work package sequence; no parallelism)

| Work package | Depends on | Parallel group | Exclusive write scope | Start gate | Join gate |
|---|---|---|---|---|---|
| WP1 backup+format | Plan 06 accepted | — | `src/agent_setup/backup.rs`, `src/agent_setup/config_edit.rs`, `Cargo.toml` | this plan READY | — |
| WP2 transaction | WP1 | — | `src/agent_setup/transaction.rs`, `install.rs` (reworked) | WP1 | — |
| WP3 isolation | — | — | `targets.rs`, `agent_service.rs` | this plan READY | — |
| WP4 port+wire | WP1-3 | — | `safety.rs`(port+test), `managed_state.rs`(port), `uninstall.rs`(port), `doctor.rs`(port), `commands.rs` | WP1-3 | — |

## Work packages

### WP1 — Backup and format-preserving config edit
- Purpose: mandatory exact-byte backup before any Agent config mutation;
  comment-preserving TOML editing for Codex.
- Inputs: rejected Plan 07's `targets.rs` (destinations), `Cargo.toml`.
- Exact files: `src/agent_setup/backup.rs` (new), `src/agent_setup/config_edit.rs`
  (new, uses `toml_edit`), `Cargo.toml`.
- Required final behavior: `config_backup()` copies exact original bytes to a
  deterministic MINE-owned backup path, verifies the copy matches, never
  overwrites an existing backup silently, and returns the backup path + hash.
  `toml_edit` inserts/overwrites `[mcp_servers.mine]` preserving comments and
  unrelated formatting.
- Edge cases: backup target exists (reuse/verify, not clobber); backup write or
  verification fails (no mutation); config absent (no backup needed, create).
- Tests: exact-byte backup; backup-failure-blocks-mutation; Codex comment/
  ordering/unrelated-key preservation.
- Suggested commit: `feat(agent-setup): mandatory config backup and toml_edit format preservation`.

### WP2 — Transactional installation and recovery
- Purpose: bounded install transaction with preflight/staging/commit/rollback/
  recovery and a durable pending-transaction record.
- Exact files: `src/agent_setup/transaction.rs` (new: PendingTransaction record,
  durable write/detect/recover), `src/agent_setup/install.rs` (reworked to run
  the transaction phases).
- Required final behavior: preflight validates all destinations/collisions/
  config/backup; staging writes payload + staged config without exposing
  partial final state; commit verifies hashes then atomically writes managed
  state then removes the pending record; rollback restores backup, removes only
  current-transaction files, restores previously-managed files (update);
  incomplete-transaction detection on next install/doctor recovers or reports
  an actionable state. No orphaned file permanently causes collision.
- Edge cases: failure after backup creation; after payload staging; after
  config staging; after first committed payload; after config replacement;
  after managed-state write; after final verification — each rolls back cleanly
  and a retry succeeds.
- Tests: injected-failure rollback at each phase; retry-after-every-failure
  succeeds; incomplete-transaction detection+recovery; no-permanent-collision.
- Suggested commit: `feat(agent-setup): transactional installation with backup-rollback recovery`.

### WP3 — Complete explicit-root isolation
- Purpose: `--config-root` is the complete authority; real env overrides never
  honored.
- Exact files: `src/agent_setup/targets.rs` (separate `Env::isolated` vs
  `Env::real_env` constructors), `src/application/agent_service.rs`,
  `src/cli/commands.rs` (`agent_env` uses `Env::isolated` when `--config-root`
  supplied).
- Required final behavior: `Env::isolated(root)` builds an env map of empty
  overrides (or None) so no real env var is consulted; `Env::real_env()` reads
  real env only when no explicit root. Paths derive only from the injected
  root + deterministic subpaths.
- Edge cases: poisoned `CODEX_HOME`/`PI_HOME`/`CLAUDE_CONFIG_DIR`/
  `OPENCODE_CONFIG_DIR` ignored under explicit root; tested via child process
  (no global env mutation in parallel tests).
- Tests: explicit-root ignores poisoned env; real HOME/Agent dirs unchanged.
- Suggested commit: `feat(agent-setup): complete explicit config-root isolation`.

### WP4 — Selective port and wiring
- Purpose: port independently validated work from rejected Plan 07 (SafetyGuard,
  managed state, uninstall, doctor, destinations), discard invalid, wire CLI.
- Ported as-is: `safety.rs` (add a real Windows-junction unit test in-module);
  `managed_state.rs` (ownership record); `uninstall.rs` (ownership-proven
  removal); `doctor.rs` (truthful diagnostics); `targets.rs` destination shapes.
- Reworked: `install.rs` (transaction); `targets.rs` (isolated/real split);
  `agent_service.rs`/`commands.rs` (isolated env).
- Discarded: mutation-without-backup; payload-first non-transactional; mixed
  env; full TOML reserialize; env-dependent tests.
- Tests: clean install/update/doctor/uninstall for all four Agents; idempotency;
  ownership uncertainty refusal; symlink/junction escape; concurrent
  install/config mutation locked or rejected.

## Verification matrix
| Scope | Command | Expected evidence | Owner |
|---|---|---|---|
| Format | `cargo fmt --all -- --check` | clean | WP1-4 |
| Lint | `cargo clippy --all-targets --all-features -- -D warnings -W unsafe-code` | no warnings, no unsafe | WP1-4 |
| Build | `cargo build --all-targets --all-features` | clean | WP1-4 |
| Tests | `cargo test --all-targets --all-features` | all green | WP1-4 |
| Sync | `python scripts/sync-plugin-assets.py --check` | in sync | WP4 |
| Verify | `python scripts/verify.py` | passed | WP4 |
| Design | `mine design validate --format json` | ok:true | WP4 |
| Graph | `mine graph validate --format json` | ok:true | WP4 |
| Isolation | real HOME/Agent dirs unchanged after suite | byte-identical | WP3 |
| Graph invariant | live `execution-graph.toml` unchanged by tests | empty diff | WP4 |

## Acceptance checklist
- [ ] Every requirement is traced to design, implementation, and evidence.
- [ ] All writes stay within declared ownership (cross-scope declared up front).
- [ ] The three rejection defects are fixed and adversarially tested.
- [ ] Selectively ported work is documented; invalid work discarded.
- [ ] `dev` unmoved; `master` untouched; Plan 08 BLOCKED; live graph unchanged.
- [ ] No unrestricted deletion/force/shell/arbitrary-replacement surface.
- [ ] The node reaches `IMPLEMENTED`, never self-granted `ACCEPTED`.

## Report path
`docs/plan/reports/07-1-transactional-agent-installation-and-isolation-implementation.md`

## Suggested commits
- `feat(agent-setup): mandatory config backup and toml_edit format preservation`
- `feat(agent-setup): transactional installation with backup-rollback recovery`
- `feat(agent-setup): complete explicit config-root isolation`
- `test(agent-setup): adversarial transaction/isolation/backup tests`
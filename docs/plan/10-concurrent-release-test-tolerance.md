# Plan 10: Concurrent release test tolerance

## Status
`DRAFT`

## Goal

Fix the known-flaky concurrency test
`tests/release.rs::concurrent_release_is_resolved_by_revision_conflict` so it
tolerates both honest loser outcomes of a concurrent `mine plan release` race,
without altering any production release or persistence semantics. The test
currently over-constrains the loser to `MINE_REVISION_CONFLICT` only; a
documented second loser path (`MINE_INVALID_TRANSITION`) makes the assertion
flaky and intermittently red. Only the test assertion (and directly related
test documentation/report files) change.

## User-visible outcome

The release concurrency test is deterministic and green under repeated
parallel execution. No public CLI, MCP, persistence, or lifecycle behavior
changes. The accepted `mine plan release` semantics are exactly as already
independently accepted; only the test's tolerance for the raced loser is
relaxed and its invariants are strengthened to prove the race is still safe.

## Governing design references
- `docs/design/execution-graph/state-machine-and-algorithms.md#plan-release` —
  documents `MINE_INVALID_TRANSITION` and `MINE_REVISION_CONFLICT` as the two
  valid stability errors of `mine plan release`, and explicitly states that
  racing automation may treat `MINE_INVALID_TRANSITION` as "already released".
- `docs/design/execution-graph/persistence-and-concurrency.md` — the shared
  `lock -> reload -> revision check -> semantic check -> mutation ->
  atomic write -> render -> release lock` transaction that the test exercises.

## Requirements traceability
| Requirement | Design leaf/anchor | Work package | Acceptance evidence |
|---|---|---|---|
| Loser may be `MINE_REVISION_CONFLICT` or `MINE_INVALID_TRANSITION` | `state-machine-and-algorithms.md#plan-release` (Stability errors) + Idempotency and re-run behavior | WP1 | Tolerant assertion + repeated-run stability evidence |
| Exactly one real DRAFT release; revision +1; final status correct | `state-machine-and-algorithms.md#plan-release` (Mutation) | WP1 | Strengthened invariants in the test |
| No stale writer overwrites the winner; no unrelated data changes; TOML/MD consistent | `persistence-and-concurrency.md` (atomic transaction) | WP1 | Unrelated-plan and TOML/MD consistency assertions |

## Current evidence and baseline
| Area | Current implementation | Evidence path/commit | Verified behavior | Gap |
|---|---|---|---|---|
| Flaky test | `tests/release.rs::concurrent_release_is_resolved_by_revision_conflict` | `dev` `f8b5768` | Loser asserted to be only `MINE_REVISION_CONFLICT` (exit 5) | Over-constrained: the `MINE_INVALID_TRANSITION` loser path (loser reloads after winner's `DRAFT->READY` transition) intermittently fails the assertion |
| Release prod behavior | `src/application/plan_service.rs::release` + `src/domain/plan_release.rs::release_plan` | accepted Plan 09-1 | Loser path 1: `save_with_revision` `RevisionConflict` (`MINE_REVISION_CONFLICT`). Loser path 2: closure re-reloads graph where status is now `READY`, so `release_plan` returns `InvalidTransition` (`MINE_INVALID_TRANSITION`). Both honest; both mutate nothing. | None — production is correct; only the test assertion is wrong |

## Research source register
| Source title | Organization/version | URL | Accessed | Verified claim | Plan implication |
|---|---|---|---|---|---|
| Plan release state machine | MINE design | `docs/design/execution-graph/state-machine-and-algorithms.md#plan-release` | 2026-07-24 | Both `MINE_INVALID_TRANSITION` and `MINE_REVISION_CONFLICT` are documented stability errors; "automation that may race should … treat `MINE_INVALID_TRANSITION` as 'already released'" | Test must accept either loser code |

## Decisions
### Material user decisions
(none — the user authorized relaxing the loser assertion to both documented outcomes and strengthening the safety invariants; production behavior is unchanged.)

### Local decisions made by the planner
- Accept loser error code from the set `{MINE_REVISION_CONFLICT, MINE_INVALID_TRANSITION}`.
- Strengthen the test by seeding an unrelated `ACCEPTED` plan and asserting it
  survives byte-unchanged, to prove "no unrelated graph data changes".
- Assert TOML and generated Markdown stay mutually consistent (same revision,
  same final status) after the race.

### Assumptions and unresolved gates
(none)

## Scope
### In scope
- `tests/release.rs::concurrent_release_is_resolved_by_revision_conflict` (the
  single test body — relax loser assertion + strengthen invariants).
- Directly related test documentation: this plan document and its
  implementation report.

### Non-goals
- Do NOT alter `src/application/plan_service.rs`, `src/domain/plan_release.rs`,
  `src/infrastructure/toml_store.rs`, or any production path.
- Do NOT change the release error codes, the optimistic-concurrency check, or
  the lock/render lifecycle.
- Do NOT touch Plan 07, `master`, `dev`, or any other test.

### Historical baggage to remove
The stale comment/docstring that states the loser "gets
`MINE_REVISION_CONFLICT`" as if it were the only outcome.

## Dependency and parallelism graph
(single work package; no parallelism)

| Work package | Depends on | Parallel group | Exclusive write scope | Shared-file requests | Start gate | Join gate |
|---|---|---|---|---|---|---|
| WP1 | Plan 06 accepted | — | `tests/release.rs`, `docs/plan/10-*.md`, `docs/plan/reports/10-*-implementation.md` | none | this plan `READY` | single WP |

## Work packages

### WP1 — Relax and strengthen the concurrent-release test
- Purpose: make the concurrency test deterministic across both loser paths.
- Inputs and predecessors: accepted `dev` `f8b5768` (Plan 06 accepted).
- Exact files/symbols/contracts: `tests/release.rs` — the
  `concurrent_release_is_resolved_by_revision_conflict` test only (its docstring
  and body). No production files.
- Current behavior: asserts `loser_env["error"]["code"] == "MINE_REVISION_CONFLICT"` only.
- Required final behavior: the loser's error code must be in
  `{MINE_REVISION_CONFLICT, MINE_INVALID_TRANSITION}`; keep exactly-one-winner,
  revision `n -> n+1`, final status `READY`, live-graph-unchanged invariants;
  ADD an unrelated `ACCEPTED` plan to the seed and assert it is unchanged; ADD
  TOML/Markdown mutual-consistency assertions.
- Input/output/error/lifecycle semantics: unchanged production semantics; test
  only widens tolerated loser codes and tightens invariants.
- Transactions, concurrency, retries, timeouts: the test spawns two threads
  racing `cli::dispatch` for `plan release`; both observe pre-mutation revision
  `n`. Keep the spawn/join structure.
- Security/privacy considerations: none (isolated temp repo).
- Configuration/dependencies: none added.
- Cleanup/removals: remove the over-constrained single-code assertion and the
  wording implying `MINE_REVISION_CONFLICT` is the only outcome.
- Edge and failure cases: path 1 (`MINE_REVISION_CONFLICT`) and path 2
  (`MINE_INVALID_TRANSITION`) are both expected; the test must pass for either.
- Tests and fixtures: reuse `seeded_repo`, `node`, `load_graph`,
  `live_graph_bytes`, `assert_live_unchanged`.
- Narrow verification commands and expected evidence:
  - `cargo test --test release --quiet concurrent_release_is_resolved_by_revision_conflict -- --test-threads=4` repeated ≥10×: all green.
  - `cargo test --all-targets --all-features --quiet` repeated ≥3×: all green.
  - `cargo fmt --all -- --check`; `cargo clippy --all-targets --all-features -- -D warnings -W unsafe-code`; `cargo build --all-targets --all-features`; `mine design validate --format json`; `mine graph validate --format json`.
- Downstream artifact: the green, deterministic concurrency test.
- Suggested commit: `test(release): tolerate both loser outcomes in concurrent release race`.

## Integration and join procedure
(single work package; no join)

## Verification matrix
| Scope | Command | Preconditions | Expected evidence | Owner |
|---|---|---|---|---|
| Release concurrency test | `cargo test --test release --quiet concurrent_release_is_resolved_by_revision_conflict -- --test-threads=4` (×10) | build | 10× green, both loser paths tolerated | WP1 |
| Full suite (×3) | `cargo test --all-targets --all-features --quiet` | build | 3× green, no flakes | WP1 |
| Format | `cargo fmt --all -- --check` | — | clean | WP1 |
| Lint | `cargo clippy --all-targets --all-features -- -D warnings -W unsafe-code` | — | no warnings, no `unsafe` | WP1 |
| Build | `cargo build --all-targets --all-features` | — | clean | WP1 |
| Design | `mine design validate --format json` | — | `ok:true` | WP1 |
| Graph | `mine graph validate --format json` | — | `ok:true`, stable | WP1 |
| Live graph unchanged | `git diff HEAD -- docs/plan/execution-graph.toml` after full suite | tests run | empty | WP1 |
| Production untouched | `git diff dev -- src/` | before commit | empty | WP1 |

## Acceptance checklist
- [ ] Every requirement is traced to architecture, implementation, and evidence.
- [ ] Required quality gates pass or failures are accurately recorded.
- [ ] No production release/persistence semantics changed (`git diff dev -- src/` empty).
- [ ] The concurrency test is deterministic across repeated runs.
- [ ] The strengthened invariants hold: exactly one winner, revision +1, final `READY`, unrelated plan unchanged, TOML/MD consistent.
- [ ] No unapproved external mutation occurred.

## Report path
`docs/plan/reports/10-concurrent-release-test-tolerance-implementation.md`

## Suggested commits
- `test(release): tolerate both loser outcomes in concurrent release race`
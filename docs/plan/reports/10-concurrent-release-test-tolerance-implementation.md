# Plan 10 Implementation Report

- **Plan**: `docs/plan/10-concurrent-release-test-tolerance.md`
- **Title**: Concurrent release test tolerance
- **Execution date**: 2026-07-24
- **Conclusion**: `IMPLEMENTED` - pending independent reviewer acceptance. The
  accepted MINE CLI performed registration, release, and `start`; the agent did
  not self-accept, did not merge into `dev`, did not touch `master`, did not
  start Plan 07, and did not alter any production release or persistence
  semantics.

## Branch contract honored

| Item | Value |
|---|---|
| Stable branch | `master` (unchanged throughout: `1d3a132`) |
| Integration branch | `dev` (unchanged throughout: `f8b5768` - never moved during this plan) |
| Implementation branch | `plan/maintenance-concurrent-release-test-tolerance`, created from clean accepted `dev` (`f8b5768`) before registration |
| Fork point verification | `git merge-base dev HEAD == f8b5768` and `git rev-parse dev == f8b5768` for the entire plan - `dev` never moved |
| Plan 07 | remains `READY` (unstarted); not started, not released, not rewired |
| Production untouched | `git diff dev -- src/` is empty - no release/persistence/domain semantics changed |
| Remotes | none; nothing pushed |

The lifecycle transitions for Plan 10 itself (`plan add` -> DRAFT at rev 36,
`plan release` -> READY at rev 37, `plan start` -> IN_PROGRESS at rev 38) were
performed through the accepted `mine` CLI and committed as `6d1b954` before
implementation began. The `IMPLEMENTED` transition (performed via the accepted
CLI after this report) is committed in a separate lifecycle record.

## Commits on the Plan branch (f8b5768..HEAD)

| Hash | Kind | Notes |
|---|---|---|
| `6d1b954` | `chore(graph)` | Register (`plan add`, rev 35->36), release (`plan release`, rev 36->37), and start (`plan start`, rev 37->38) Plan 10 via accepted `mine` CLI. Performed before implementation; the only graph mutations on this branch. |
| `cf7df45` | `test(release)` | Tolerate both loser outcomes in the concurrent release race; strengthen safety invariants. |
| (report) | `docs(plan-10)` | This implementation report. |

## Root cause of the flaky test

`tests/release.rs::concurrent_release_is_resolved_by_revision_conflict` spawns
two threads racing `cli::dispatch` for `mine plan release --id 09-1` against one
isolated temp repo, where both readers observe the pre-mutation revision `n`.
The shared transaction is
`lock -> reload -> revision check -> semantic check -> mutation -> atomic write
-> render -> release lock` (`src/infrastructure/toml_store.rs::save_with_revision`
+ `src/application/graph_service.rs::mutate`).

`PlanService::release` reads `expected = self.graph.validate()?.revision`
**outside** the mutation closure, then calls `self.graph.mutate(expected, ...)`.
This gives the racing loser two honest outcomes, both documented in
`docs/design/execution-graph/state-machine-and-algorithms.md#plan-release`:

1. **`MINE_REVISION_CONFLICT`** — the loser reads `expected = n`, the winner
   writes `n+1`, and the loser's `save_with_revision(expected=n)` rejects because
   the on-disk revision is now `n+1 != n` (`MineError::RevisionConflict`).
2. **`MINE_INVALID_TRANSITION`** — the loser reads `expected` *after* the winner
   committed (so `expected = n+1` matches on-disk), the closure proceeds against
   the reloaded graph where the node is now `READY` (not `DRAFT`), and
   `release_plan` returns `MineError::InvalidTransition` ("current status is
   not `DRAFT`"). The design's idempotency section explicitly says automation
   that races "should read `mine plan show` first or treat
   `MINE_INVALID_TRANSITION` as 'already released'".

The test asserted `loser_env["error"]["code"] == "MINE_REVISION_CONFLICT"` as
the only allowed outcome, so path 2 intermittently turned the test red. The
common factor in both tests (checking this assertion twice) was the same
over-constrained single-code check.

## Change made (narrow, in-scope)

Only `tests/release.rs::concurrent_release_is_resolved_by_revision_conflict`
(its docstring and body) changed. No production file changed
(`git diff dev -- src/` is empty).

### Relaxed loser assertion
The loser's error code is now accepted from the documented set
`{MINE_REVISION_CONFLICT, MINE_INVALID_TRANSITION}`:

```rust
let loser_code = loser_env["error"]["code"].as_str().unwrap_or("(none)");
assert!(
    loser_code == "MINE_REVISION_CONFLICT" || loser_code == "MINE_INVALID_TRANSITION",
    "loser must be a documented stability error, got {loser_code:?}; loser_env={loser_env}"
);
```

The duplicate single-code assertion at the end of the test was removed.

### Strengthened safety invariants
The test now also proves the race is safe regardless of which loser path occurs:

- **Exactly one writer performs the real DRAFT release** (unchanged:
  `assert!(ok_a ^ ok_b, ...)`).
- **Graph revision increases exactly once**: `ws.revision == n + 1` (not
  higher), so no stale writer overwrote/doubled the winner.
- **Final Plan status is correct**: `ws.get("09-1").status == READY`.
- **Winner transitioned DRAFT -> READY off the pre-mutation revision `n`**:
  `winner_env["data"]["status_before"] == "DRAFT"`,
  `winner_env["data"]["status_after"] == "READY"`,
  `winner_env["revision_before"] == n`,
  `winner_env["revision_after"] == n + 1`.
- **No unrelated graph data changes**: the seed now includes an unrelated
  `ACCEPTED` plan `01`; the snapshot of that node before the race is compared
  byte-for-byte to the post-race node, proving the concurrent release did not
  touch it.
- **TOML and generated Markdown remain consistent**: the TOML is re-parsed and
  its revision asserted `== n + 1`, and the Markdown view is asserted to contain
  `READY` and the released plan's row `| 09-1 |`.
- **Live repository graph unchanged** (`assert_live_unchanged(&live)`,
  unchanged).

## Validation evidence

| Gate | Command | Exit | Result |
|---|---|---|---|
| Format | `cargo fmt --all -- --check` | 0 | clean |
| Lint | `cargo clippy --all-targets --all-features -- -D warnings -W unsafe-code` | 0 | no warnings, no errors, no `unsafe` in business/test code |
| Build | `cargo build --all-targets --all-features` | 0 | clean |
| Targeted test (×12) | `cargo test --test release --quiet concurrent_release_is_resolved_by_revision_conflict` (repeated 12×) | 0 | 12/12 green (both loser paths tolerated) |
| Full suite (×3) | `cargo test --all-targets --all-features --quiet` (repeated 3×) | 0 | 3/3 green; 246 tests each, zero flakes |
| Design validate | `mine design validate --format json` | 0 | `ok:true, valid:true` |
| Graph validate | `mine graph validate --format json` | 0 | `ok:true, plans:13, rev:38` |
| Production unchanged | `git diff dev -- src/` | 0 (empty) | no release/persistence/domain code changed |
| Live graph unchanged | `git diff HEAD -- docs/plan/execution-graph.toml` after full suite | 0 (empty) | test suite did not mutate the live graph |
| `dev` unmoved | `git rev-parse dev` | - | `f8b5768` |
| `master` untouched | `git rev-parse master` | - | `1d3a132` |
| Plan 07 unstarted | `mine plan show --id 07` | - | status `READY` |

### Repeated-run stability evidence

The targeted concurrency test was run 12 times consecutively; all 12 passed
(both loser paths occur and are tolerated across runs). The full suite was run 3
times consecutively; all 3 passed with zero flakes (each run: 108 lib unit +
138 integration = 246 total). This demonstrates the relaxation removed the
intermittent red without weakening the safety guarantees.

## Scope discipline

- In scope: `tests/release.rs` (the single test body + docstring), this plan
  document, and this report.
- Not touched: `src/application/plan_service.rs`, `src/domain/plan_release.rs`,
  `src/infrastructure/toml_store.rs`, `src/application/graph_service.rs`, and
  every other production file (`git diff dev -- src/` empty).
- Not touched: Plan 07 (still `READY`), `master`, `dev`, any other test, any
  design document.

## Deviations and unresolved uncertainties

None. The production `mine plan release` behavior was already independently
accepted (Plan 09-1); only the test assertion was over-constrained. No
validator was unavailable.

## Remaining risks

None material. The test now faithfully matches the documented concurrency
contract (both `MINE_REVISION_CONFLICT` and `MINE_INVALID_TRANSITION` are
accepted loser outcomes) while proving the safety invariants that matter
(exactly one winner, exactly one revision bump, correct final status, no
unrelated change, no stale overwrite, TOML/Markdown consistency).
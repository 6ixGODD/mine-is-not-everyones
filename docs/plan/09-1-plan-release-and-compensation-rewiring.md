# Plan 09-1: Plan release and compensation rewiring (compensation for rejected Plan 09)

## Status
`DRAFT` (to be released to `READY` by the one-time break-glass exception in
§Break-glass, then started via the accepted `mine plan start` CLI).

## Branch contract

- Stable branch: `master` (detected by `mine init`).
- Integration branch: managed `dev`.
- Implementation branch: `plan/09-1-plan-release-and-compensation-rewiring`,
  created from the current accepted `dev` tip (`88affc0`).
- Never implement directly on the stable branch or `dev`. Registration,
  break-glass, start, implementation, report, and `IMPLEMENTED` bookkeeping
  remain on this ephemeral Plan branch and nowhere else.
- Only an independent reviewer may accept and merge it into `dev`.

> This section is explicitly present because the rejected Plan 09's own
> document omitted it — an omission the independent review (Gate 2) cited as
> corroborating evidence of lifework/lifecycle bookkeeping landing on `dev`
> instead of an ephemeral branch. This plan port-forwards the technically
> accepted substance of Plan 09 **strictly on its own branch**.

## Goal

Port forward the technically accepted substance of the rejected Plan 09 — the
two lifecycle-closure operations and their tests — onto its own ephemeral
branch, executed strictly under branch governance. Plan 09 was rejected by
independent review only for lifecycle and branch-governance violations (its
design, registration, break-glass, and `start` bookkeeping were committed
directly to `dev` before acceptance). Its release/rewire-compensation domain
and CLI implementation was independently verified sound (Gates 1, 3, 4, 5 of
`docs/plan/reports/09-execution-graph-compensation-rewiring-review.md`: 185/185
tests, fmt/clippy clean, no arbitrary graph-edit API, no `unsafe`, exact design
match). This plan ports that sound substance forward and adds the two
review-flagged follow-ups (dead-code removal, stale/concurrent-revision test).

Two first-class, deterministic, CLI-managed operations:

1. **`mine plan release --id <id>`** — the explicit gate that moves a newly
   registered `DRAFT` plan to `READY` (every hard predecessor `ACCEPTED`,
   including no-predecessor plans) or `BLOCKED` (one or more unsatisfied).
   `mine plan add` registers plans as `DRAFT`; `mine plan start` requires
   `READY`; automatic successor release happens only inside `mine plan accept`;
   so a standalone newly added `DRAFT` plan could not previously enter
   execution. This plan itself could not be started without this capability.
2. **`mine plan rewire-compensation --id <rejected-id>`** — reroutes a rejected
   plan's downstream dependencies onto its **registered compensating plan**
   (derived from `compensating_plan`), through the accepted persistence path.
   Closes the gap exposed when Plan 05 was rejected and 05-1 registered:
   downstream Plan 06 still names `05`.

Both go through the shared application/persistence transaction
(`lock → reload → revision check → semantic validation → mutation → atomic
persistence → deterministic render`). Neither introduces an arbitrary
graph-editing or state-setting API. Manual editing of
`docs/plan/execution-graph.{toml,md}` is prohibited (bootstrap exception
ended), except the single one-time break-glass mutation recorded below that
makes Plan 09-1 itself startable.

## User-visible outcome

After this plan is accepted and merged into `dev`, an authorized reviewer can
close the Plan 05 rejection and resume implementation of Plan 05-1, reading
the authoritative current revision immediately before each operation:

```
mine plan rewire-compensation --id 05 --format json   # 06: hard preds 05 -> 05-1
mine plan release --id 05-1 --format json             # 05-1: DRAFT -> READY
mine plan start --id 05-1 --owner <owner> --run-id <run> --format json
```

Each mutation satisfies `revision_after == revision_before + 1`. Until these
commands exist and are accepted, no agent may hand-edit the graph to repair
dependencies or release plans.

## Governing design references

- `docs/design/execution-graph/state-machine-and-algorithms.md#plan-release` (algorithm, preconditions, idempotency, result)
- `docs/design/execution-graph/state-machine-and-algorithms.md#compensation-rewiring`
- `docs/design/execution-graph/domain-model.md` (`compensating_plan` as single source of truth)
- `docs/design/execution-graph/persistence-and-concurrency.md#revision-and-locking`
- `docs/design/interfaces/cli-contract.md#plan-release`
- `docs/design/interfaces/cli-contract.md#compensation-rewiring`
- `docs/design/governance/branch-and-plan-lifecycle.md#registration-and-release`
- `docs/design/governance/branch-and-plan-lifecycle.md#compensation-and-downstream-rewiring`
- `docs/design/system/component-architecture.md`
- `docs/plan/reports/09-execution-graph-compensation-rewiring-review.md` (the review that rejected Plan 09 on governance and authorized porting the sound substance forward)

## Requirements traceability

| Requirement | Design leaf/anchor | Work package | Acceptance evidence |
|---|---|---|---|
| Plan release: DRAFT-only → READY/BLOCKED | state-machine #plan-release | WP1 | release tests |
| Plan release: deterministic result {plan, status_before, status_after, hard_predecessors, unsatisfied_predecessors, revisions}; revision +1 exactly once | cli-contract #plan-release | WP3 | golden envelope test |
| Plan release: never alters non-DRAFT plans; no arbitrary state-editing | #plan-release; branch-and-plan #registration-and-release | WP1, WP3 | non-DRAFT tests + grep gate |
| Plan release: reads current revision immediately before; runtime invariant `revision_after == revision_before + 1` | #plan-release | WP3 | concurrent-release test |
| Rewire: only via registered compensating_plan; derived, not caller-supplied | domain-model `compensating_plan`; #compensation-rewiring | WP1, WP3 | empty/caller test |
| Rewire: original REJECTED; replacement exists & not REJECTED;.mutable successors only; no cycle; unrelated unchanged | #compensation-rewiring | WP1 | rewire domain tests |
| Rewire: deterministic result; revision +1 on real mutation, no-op writes nothing | cli-contract #compensation-rewiring | WP3 | idempotent + concurrent tests |
| Both: shared persistence path (lock→reload→validate→mutate→atomic write→render); atomic TOML+MD | persistence #revision-and-locking | WP3 | byte-unchanged-on-failure + MD regen tests |
| Stale/concurrent revision protection for the two new mutation commands | (added test) | WP3 | concurrent_release/concurrent_rewire tests |
| Dead-code cleanup flagged by Plan 09 review | review Gate 4 note | WP1 | duplicate `references` call removed |
| All tests on isolated temp repos; live graph byte-unchanged | branch-and-plan | WP3 | md5 guards |
| No arbitrary edit/state API; no new `unsafe`; quality gates | `#![forbid(unsafe_code)]`; grep gate | WP1–WP3 | grep gates |

## Current evidence and baseline

| Area | Current implementation | Evidence | Gap |
|---|---|---|---|
| Rejected Plan 09 reviewer-verified source | `plan/09-plan-release-and-compensation-rewiring` commits `e4c416d`/`888b382`/`33496be` | review Gates 3,4,5 pass; tests 185/185 | port forward on own branch |
| Design on `dev` | `state-machine-and-algorithms.md`/`cli-contract.md`/`governance` already carry release + rewire (commits `8fe9ab4`/`8eff7df` on `dev`) | `mine design validate` passes | usable directly |
| Accepted CLI on `dev` | `mine plan add` (creates DRAFT), `mine plan start` (requires READY), `save_with_revision`, `topological_sort`, `MineError::GraphCycle` | dev `88affc0` | release gap; rewire gap |
| Authoritative graph | `docs/plan/execution-graph.toml` rev 22 (`dev` `88affc0`): Plan 09 `REJECTED` `compensating_plan="09-1"`; 05 `REJECTED` `compensating_plan="05-1"`; 06 `BLOCKED hard=[04,05]`; 05-1 `DRAFT` | dev graph | 09-1 unreachable without release; 06 needs rewire (live reroute is reviewer's post-acceptance step, not this plan) |

## Research source register

No new external technology. Both operations reuse the accepted in-repository
execution-graph engine and CLI/envelope contracts. Sources are repository
design documents (cited above) and the accepted implementation on `dev`. No
web research required (internal graph-lifecycle operations, no external
protocol/library/standard dependency).

## Decisions

### Material user decisions

- Two operations in one compensating plan (user directive: port forward the
  rejected Plan 09 substance, owning both related lifecycle closures).
- Branch-local one-time break-glass for Plan 09-1's own `DRAFT→READY`, on the
  Plan branch only, never on `dev`.
- Preserve registration/release distinction: do NOT change `mine plan add`.

### Local decisions made by the planner

- Selectively port only the production+test source represented by Plan 09's
  sound commits (`e4c416d`, `888b382`, `33496be`); do NOT merge or cherry-pick
  the whole rejected branch, its lifecycle bookkeeping, its old report, its
  IMPLEMENTED transition, or the break-glass/start history already on `dev`.
- Remove the dead duplicate `references(p, rejected_id);` call flagged in the
  review (Gate 4 note).
- Add the dedicated stale/concurrent-revision test for both new mutation
  commands required by the review (Gate 5 gap): two concurrent `dispatch`
  invocations against the same isolated temp repo, asserting one winner
  (revision +1, real mutation) and one loser (`MINE_REVISION_CONFLICT`, or —
  the honest alternative outcome for rewire — an idempotent no-op that does
  not overwrite the winner). Race-free: both readers read the same
  pre-mutation revision `n`, the winner commits `n+1` inside the transaction
  lock, the loser's expected_revision is now stale.
- `rewire_compensation`'s idempotent no-op leaves the workspace unchanged and
  does not bump the revision (the store re-serializes deterministically, so
  byte-identical TOML/MD result). No new store API needed.
- Release is not idempotent-success (DRAFT-only); rewire is idempotent-success.
- Remove the dead `(Rejected, Blocked)` state-machine edge + its existing test
  assertion (no operation performs it; design removed the row in `8fe9ab4`).
- `RewireSuccessorLocked` → exit `3` (GATE). Release reuses
  `MINE_PLAN_NOT_FOUND`/`MINE_INVALID_TRANSITION`.

### Assumptions and unresolved gates

- Assumes dev presents the accepted CLI/store/domain surface as verified on
  `88affc0` (repository evidence, not assumption).

## Scope

### In scope

- Ported: `src/domain/plan_release.rs`, `src/domain/rewire.rs`, the
  `RewireSuccessorLocked` error variant + exit map, the CLI dispatch/handler
  code, `tests/release.rs`, `tests/rewire.rs`, the `tests/common/mod.rs`
  helper, the dead-edge removal in `status.rs` + its existing test update, and
  the module registration in `src/domain/mod.rs`.
- Added: dead `references` call removal; concurrent-revision tests for both
  commands.
- Break-glass: Plan 09-1 `DRAFT→READY` on this branch only.

### Non-goals

- Do NOT change `mine plan add` (registration stays DRAFT).
- Do NOT change automatic successor release inside `mine plan accept`.
- Do NOT create any generic graph-editing or state-setting API.
- Do NOT perform the live `06` → `05-1` rewiring, the live `05-1` release, or
  start Plan 05-1 — those are the reviewer's post-acceptance steps.
- Do NOT merge or cherry-pick the rejected `plan/09-*` branch; do NOT rewrite
  `dev` history.
- Do NOT modify `dev` or `master` during implementation; everything stays on
  this branch until independent acceptance.
- Do NOT add an MCP tool (CLI-only).

### Historical baggage to remove

- The dead `(Rejected, Blocked)` `validate_transition` arm + its `allowed_edges_pass`
  assertion (matching the design row removal in `8fe9ab4`).
- The dead duplicate `references(p, rejected_id);` call in `rewire.rs`.

## Break-glass exception (branch-local, one-time)

Because the accepted CLI still cannot release a newly registered `DRAFT` plan
(the gap this plan closes), authorize one narrowly scoped manual mutation on
the Plan 09-1 branch only:

- Plan 09-1: `DRAFT → READY`;
- graph revision increments exactly once;
- regenerate `docs/plan/execution-graph.md` using the accepted renderer;
- record pre/post hashes in a dedicated break-glass commit;
- no dependency changes, no other Plan state changes; Plan 05-1 remains `DRAFT`;
  Plan 06 remains `BLOCKED` and depends on `05`.

Never perform this mutation on `dev`. After Plan 09-1 becomes `READY`, start
it via the accepted `mine plan start --id 09-1 ...` and commit that lifecycle
transition separately on this branch.

## Work packages

### WP1 — Domain (release + rewire + dead-edge removal)

Port `src/domain/plan_release.rs::release_plan` (DRAFT-only; READY/BLOCKED
decision; `unsatisfied_predecessors`), `src/domain/rewire.rs::rewire_compensation`
(derives replacement from `compensating_plan`; exact-id in-place replacement
in mutable successors; post-rewire `topological_sort` cycle check on a staged
clone; idempotent no-op), `MineError::RewireSuccessorLocked` + code, register
both modules in `src/domain/mod.rs`, map the variant to `GATE` in
`src/output/mod.rs`, and remove the dead `(Rejected, Blocked)` edge +
its test assertion. Remove the dead duplicate `references` call flagged by the
Plan 09 review. Domain unit tests ported unchanged.

### WP2 — CLI handlers + dispatch

Port `plan_release` and `plan_rewire_compensation` handlers in
`src/cli/commands.rs` (route through `save_with_revision`; release bumps +1
always; rewire bumps +1 only on a real mutation; idempotent no-op writes
byte-identical TOML/MD), the `command_name`/dispatch arms in `src/cli/mod.rs`,
and the deterministic result envelopes (`plan.release`, `plan.rewire-compensation`).

### WP3 — Tests (release + rewire + concurrent revision) + live invariants

Port `tests/common/mod.rs`, `tests/release.rs`, `tests/rewire.rs`. Add the
dedicated stale/concurrent-revision tests required by the Plan 09 review:
`concurrent_release_is_resolved_by_revision_conflict` and
`concurrent_rewire_is_resolved_by_revision_conflict` (two threads
`cli::dispatch` against one isolated temp repo; one winner with revision `+1`
and the real mutation; one loser with `MINE_REVISION_CONFLICT` — or, for
rewire honestly, an idempotent no-op that does not overwrite the winner).
Update the existing `tests/domain.rs::reject_path_requires_review_then_compensation`
assertion to REJECTED-terminal. Live-graph md5 guards in both files; full
byte-unchanged-on-failure invariant suites.

## Integration and join procedure

WP1 → WP2 → WP3 sequential (CLI depends on domain; integration tests depend on
both). Final join: `cargo fmt --check`, `cargo clippy -D warnings -W unsafe-code`,
`cargo test --all-targets --all-features`, `mine graph validate --format json`
(`ok:true`), `mine design validate --format json` (`valid:true`), live-graph
md5-unchanged across the test suite, edit-API grep gate, structural-`unsafe`
grep clean.

## Verification matrix

| Scope | Command | Expected evidence |
|---|---|---|
| Format | `cargo fmt --all -- --check` | no diff, exit 0 |
| Lint + unsafe | `cargo clippy --all-targets --all-features -- -D warnings -W unsafe-code` | no warnings, exit 0 |
| Build | `cargo build --all-targets --all-features` | exit 0 |
| Tests | `cargo test --all-targets --all-features` | 187 passed, 0 failed |
| Design | `mine design validate --format json` | `valid:true` |
| Graph | `mine graph validate --format json` | `ok:true` (11 plans) |
| No arbitrary edit API | `grep -rnE "set_predecessors\|edit_graph\|move_plan\|set_status" src/` | no matches |
| Live graph unchanged | `md5sum docs/plan/execution-graph.toml` before/after suite | identical |
| 05-1 unchanged | graph TOML node 05-1 `status = "DRAFT"` | unchanged |
| 06 unchanged | graph TOML node 06 `hard_predecessors = ["04", "05"]` | unchanged |
| dev unmoved | `git rev-parse dev` | `88affc0` throughout |

## Acceptance checklist

- [ ] Every requirement traced to architecture, implementation, evidence.
- [ ] Cited design leaves/anchors exist (already on `dev`).
- [ ] `## Branch contract` present; all lifecycle bookkeeping on the Plan branch.
- [ ] No generic graph-editing/state-setting API (grep gate).
- [ ] Dead `REJECTED -> BLOCKED` code arm + redundant `references` call removed.
- [ ] Dedicated stale/concurrent-revision test for both new mutation commands.
- [ ] All mutating tests use isolated temp repos; live graph byte-unchanged.
- [ ] Required quality gates pass.
- [ ] Plan 05-1 remains `DRAFT`; Plan 06 remains `BLOCKED` deps `[04,05]`; `dev`
      has not moved since branch creation; `master` untouched.
- [ ] Break-glass executed exactly once on this branch (Plan 09-1 DRAFT→READY),
      recorded; no other manual graph mutation.
- [ ] Live rewiring/release/start NOT performed; Plan 05-1 NOT begun.
- [ ] No self-accept; this agent concludes `IMPLEMENTED` only.

## Post-acceptance reviewer handoff (NOT part of this plan's implementation)

After independent `ACCEPTED` + merge into `dev`:

```
mine plan rewire-compensation --id 05 --format json     # 06: hard preds 05 -> 05-1
mine plan release --id 05-1 --format json               # 05-1: DRAFT -> READY
mine plan start --id 05-1 --owner <owner> --run-id <run> --format json
```
Each reads the current revision immediately before mutating.

## Report path
`docs/plan/reports/09-1-plan-release-and-compensation-rewiring-implementation.md`

## Suggested commits

- `feat(domain): plan release and compensation rewiring operations`
- `feat(cli): mine plan release and rewire-compensation`
- `test(plan-09-1): release and rewire-compensation tests incl. concurrent revision`
- `docs(plan-09-1): implementation report`
- break-glass: `chore(graph): one-time break-glass release Plan 09-1 DRAFT -> READY`
- lifecycle: `mine plan start` and `mine plan implemented` CLI-generated commits
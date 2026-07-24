# Plan 09 Implementation Report

- **Plan**: `docs/plan/09-execution-graph-compensation-rewiring.md`
- **Title**: Plan release and compensation rewiring (CLI-managed)
- **Execution date**: 2026-07-24
- **Conclusion**: `IMPLEMENTED` — pending independent reviewer acceptance. The
  accepted MINE CLI was used for every lifecycle transition and for the
  break-glass record; this agent did not self-accept.

## Branches and commits

| Item | Value |
|---|---|
| Stable branch | `master` (unchanged) |
| Integration branch | `dev` (at `fe93d7b` after Plan 09 start; this plan implements on `plan/09-plan-release-and-compensation-rewiring` and does not merge into `dev`) |
| Plan branch | `plan/09-plan-release-and-compensation-rewiring` (from `dev` at `fe93d7b`, which carries Plan 09 `IN_PROGRESS`) |
| Plan-start (via accepted CLI) | `mine plan start --id 09 --owner plan-09 --run-id plan-09-release-rewire` → rev 19→20, Plan 09 `READY→IN_PROGRESS` (bookkeeping commit `fe93d7b` on `dev`) |
| Break-glass commit | `a074750` — one-time `Plan 09 DRAFT→READY`, rev 18→19, deterministic MD regen |
| Implementation commits | `e4c416d` (domain), `888b382` (CLI), `33496be` (tests) |
| Report commit | this file |

### Break-glass record (one-time, recorded in Plan 09 §Break-glass)

A single manual graph mutation authorized because no accepted command could
release Plan 09 itself (the gap Plan 09 fixes). Smallest schema-valid change:
TOML `revision 18→19` and Plan 09 `status "DRAFT"→"READY"`; Markdown
regenerated via `mine graph render` (not hand-edited). Diff was exactly those
two lines in each file; no dependencies, no 05-1, no 06, no other plan state.

- Pre-TOML: `a1422d4ce86b6b1572a91fbba166f2e6`
- Pre-MD:   `b8f25d7a6c8112404d9d64c0c0f16bb6`
- Post-TOML: `93f8296a7cc5dcb37f428473cefb9dd4`
- Post-MD:   `c48691af8b2a536ad8120682cdae563d`

`mine graph validate --format json` → `ok:true` (11 plans) after the mutation.
This is the only manual graph mutation the exception authorizes; afterward
manual graph editing reverted to the normal prohibition.

## What was implemented

### Design (preceded the plan — design commits in the dev history)

`8eff7df` added `mine plan release` to the state-machine, CLI-contract, and
governance design; the earlier design commit already added compensation
rewiring. Plan 09 was amended (`85faeeb`) to own both closures and to remove
all fixed-revision assertions.

### Domain layer (`commit e4c416d`)

- `src/domain/plan_release.rs::release_plan(ws, id, now)`: DRAFT-only; computes
  unsatisfied hard predecessors; transitions `DRAFT→READY` (all accepted, incl.
  no preds) or `DRAFT→BLOCKED`; refreshes `updated_at`; no I/O, no revision
  bump (the caller's transaction does). `unsatisfied_predecessors(ws, id)` for
  deterministic reporting. 16 unit tests.
- `src/domain/rewire.rs::rewire_compensation(ws, rejected_id, now)`: derives the
  replacement from the rejected plan's `compensating_plan` (caller never
  supplies it — no similar-id matching); replaces every exact occurrence in
  mutable (`DRAFT/BLOCKED/READY`) successors' hard+soft predecessors in-place
  (order preserved); returns affected successors in insertion order; post-rewire
  `topological_sort` cycle check (staged on a clone → ws unmutated on
  `GraphCycle`); idempotent (empty affected → no-op). 13 unit tests.
- `MineError::RewireSuccessorLocked { plan_id, successor_id, successor_status }`
  + `MINE_REWIRE_SUCCESSOR_LOCKED`; mapped to exit `3` (GATE).
- Dead `(Rejected, Blocked)` edge removed from `status.rs::validate_transition`
  and its existing test updated to assert REJECTED is terminal (no-historical-
  baggage; the design row was removed in `8fe9ab4`).
- No arbitrary state-editing or graph-editing API. `#![forbid(unsafe_code)]`
  holds; no `unsafe` in the new code.

### CLI layer (`commit 888b382`)

- `mine plan release --id <id>` → `plan.release`: routes `release_plan` through
  `TomlStore::save_with_revision`; revision bumps +1 exactly once on every
  successful release; deterministic envelope `{plan, status_before:"DRAFT",
  status_after, hard_predecessors, unsatisfied_predecessors, revisions}`.
- `mine plan rewire-compensation --id <rejected-id>` → `plan.rewire-compensation`:
  routes `rewire_compensation` through `save_with_revision`; revision bumps +1
  only on a real mutation (the idempotent no-op writes byte-identical TOML/MD
  with revision unchanged); deterministic envelope `{rejected_plan,
  compensating_plan, affected_successors, revisions}`. Affected list and
  compensating id threaded out of the closure via `RefCell`.
- `command_name` + `commands::handle` dispatch arms for both. CLI-only (no MCP
  tool added).

### Tests (`commit 33496be`)

`tests/common/mod.rs` seeds isolated temp repos (real config + controlled
graphs) and drives `cli::dispatch`; the live graph is snapshotted before/after
every test and asserted unchanged.

- `tests/release.rs` (10): READY/BLOCKED transitions, unsatisfied reporting,
  non-DRAFT rejection (byte-unchanged across all six statuses), missing plan,
  missing `--id`, non-idempotent second release (no extra revision), other
  plans untouched, REJECTED release rejected.
- `tests/rewire.rs` (13): success reroute (`06 hard 05→05-1`, +1 revision, MD
  regenerated), idempotent no-op (no bump, bytes identical), not-REJECTED,
  empty compensating, missing replacement, REJECTED replacement, locked
  successor (all four active/terminal statuses → `MINE_REWIRE_SUCCESSOR_LOCKED`,
  exit 3, bytes unchanged, 06 unchanged), cycle, sibling id `050` not rewired,
  soft-predecessor reroute, missing `--id`.

## Verification (all pass)

| Check | Exit | Result |
|---|---|---|
| `cargo fmt --all -- --check` | 0 | clean |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | no warnings |
| `cargo clippy --all-targets --all-features -- -D warnings -W unsafe-code` | 0 | no `unsafe_code` |
| `cargo build --all-targets --all-features` | 0 | under `#![forbid(unsafe_code)]` |
| `cargo test --all-targets --all-features` | 0 | **185 passed, 0 failed**: 101 lib + 16 cli + 9 domain + 4 golden + 10 init + 9 persistence + 10 release + 13 rewire + 13 skill_contract |
| `mine graph validate --format json` (live) | 0 | `{plans:11, warnings_emitted:false}` |
| `mine design validate --format json` (live) | 0 | `{valid:true, warnings:[]}` |
| live `docs/plan/execution-graph.toml` md5 before/after full suite | — | byte-identical (all tests use `tempfile`) |
| repo-scoped structural `unsafe` grep in `src/`/`tests/` | clean | only `//!` doc-comment hits in `file_lock.rs` (no structural `unsafe`) |
| arbitrary-edit-API gate `set_predecessors|edit_graph|move_plan|set_status` in `src/` | clean | no matches |

The runtime procedures read the authoritative current revision immediately
before each operation; every successful mutation satisfies
`revision_after == revision_before + 1`. No fixed-revision assertions remain.

## Decisions recorded

- No new store API: `save_with_revision` already rewrites unchanged workspaces
  on a no-op (revision unchanged unless the closure bumps it), so rewire's
  idempotent no-op needs no conditional-write helper (SOLID:
  no-speculative-abstraction).
- Release is not idempotent-success (only DRAFT accepted; re-run →
  `MINE_INVALID_TRANSITION`); rewire is idempotent-success (no affected → no
  write, no bump). Both documented in design and tested.
- Both operations under `mine plan`, CLI-only, no arbitrary edit API, no
  weakening of accepted/active-plan immutability.

## Constraints honored

- Implementation did NOT run `mine plan release` or `mine plan rewire-compensation`
  against the live repository; all mutations are against isolated temp repos.
- The one break-glass mutation (Plan 09 DRAFT→READY, rev 18→19) is the only
  manual graph edit; recorded with hashes and a dedicated commit; prohibits
  changing dependencies, releasing 05-1, rewiring 06, modifying any other plan
  state, beginning 05-1, and any repeated manual graph mutation.
- Every lifecycle transition (`mine plan start`, and the upcoming
  `mine plan implemented`) went through the accepted CLI; no manual graph
  editing except the single break-glass.
- `master` untouched; `dev` not merged; nothing pushed (no remotes); no
  reset/clean/force-push/blind-stash; no self-accept; Plan 05-1 not begun;
  Plan 06 not rewired live; Plan 05-1 not released live.

## Post-acceptance reviewer handoff (not performed by this agent)

After independent `ACCEPTED` + merge into `dev`, the reviewer runs, against the
live repo, reading the current revision before each:

```
mine plan rewire-compensation --id 05 --format json     # 06: hard preds 05 -> 05-1
mine plan release --id 05-1 --format json               # 05-1: DRAFT -> READY
mine plan start --id 05-1 --owner <owner> --run-id <run> --format json
```

Each reads the authoritative current revision immediately. `rewire` →
`revision_after == revision_before + 1`, `affected_successors:["06"]`;
`release` → `status_after:"READY"`, `unsatisfied_predecessors:[]`.

## Remaining risks and follow-ups

- The reviewer must accept Plan 09, merge into `dev`, and perform the live
  reroute + release + start sequence. This agent did not self-accept, merge,
  touch `master`, push, begin Plan 05-1, rewire Plan 06, or release 05-1.
- The DRAFT→READY release gap that motivated the break-glass is now closed by
  the accepted `mine plan release`; no further break-glass is needed.
- MCP tool surface for these operations is deliberately out of scope (CLI-only).
# Plan 09-1 Implementation Report

- **Plan**: `docs/plan/09-1-plan-release-and-compensation-rewiring.md`
- **Title**: Plan release and compensation rewiring (compensation for rejected Plan 09)
- **Execution date**: 2026-07-24
- **Conclusion**: `IMPLEMENTED` — pending independent reviewer acceptance. The
  accepted MINE CLI performed every lifecycle transition and the break-glass
  on the ephemeral Plan branch only; the agent did not touch `dev` or `master`
  during implementation and did not self-accept.

## Branch contract honored

| Item | Value |
|---|---|
| Stable branch | `master` (unchanged throughout: `1d3a132`) |
| Integration branch | `dev` (unchanged throughout: `88affc0` — never moved during this plan) |
| Implementation branch | `plan/09-1-plan-release-and-compensation-rewiring`, created from current accepted `dev` (`88affc0`) before any registration or lifecycle mutation |
| Fork point verification | `git merge-base dev HEAD == 88affc0` and `git rev-parse dev == 88affc0` for the entire plan — `dev` never moved |
| Rejected Plan 09 branch preserved | `plan/09-plan-release-and-compensation-rewiring` (`ebace56`) untouched, not merged or cherry-picked |
| Remotes | none; nothing pushed |

Every Plan 09-1 artifact — compensating Plan document, registration,
break-glass, `start`, implementation, and this report — was committed on the
`plan/09-1-*` branch and nowhere else. This is the explicit `## Branch
contract` section whose omission from Plan 09's own document the independent
review cited as corroborating evidence of Plan 09's governance violation.

## Commits on the Plan branch (88affc0..HEAD)

| Hash | Kind | Notes |
|---|---|---|
| `45ee446` | docs(plan-09-1): compensating plan + register DRAFT via accepted `mine plan add` | rev 22→23; Plan 09-1 `DRAFT`; hard predecessor `03` (the accepted lineage upstream, not rejected Plan 09); CLI-wrote graph TOML+MD (no manual editing); includes the explicit §Branch contract; design validate `valid:true`; graph validate `ok:true` (12 plans) |
| `6ad9f66` | feat(domain): plan release and compensation rewiring operations | ported `src/domain/plan_release.rs`, `src/domain/rewire.rs` (+ dead `references` call removal), `MineError::RewireSuccessorLocked` + code + exit map, `(Rejected, Blocked)` edge removal + existing test update, module registration |
| `def594f` | test(plan-09-1): concurrent-revision tests for release and rewire-compensation | ported `tests/common/mod.rs`, `tests/release.rs`, `tests/rewire.rs` + new concurrent-revision tests + `tests/domain.rs` update |
| `2a65fe9` | chore(graph): one-time break-glass release Plan 09-1 DRAFT → READY (rev 23 → 24) | branch-local break-glass, recorded hashes, dedicated commit |
| `9d62c92` | chore(graph): start Plan 09-1 via accepted `mine plan start` | rev 24→25; `READY→IN_PROGRESS` |
| this report | docs(plan-09-1): implementation report | — |

### Port-forward provenance (reviewer instructions honored)

The sound production+test substance was ported from rejected Plan 09's
verified commits only: `e4c416d` (domain), `888b382` (CLI), `33496be` (tests).
These three commits live on `plan/09-plan-release-and-compensation-rewiring`
and were confirmed by reading the actual file diffs, not blindly cherry-picked.
No merge or cherry-pick of the complete rejected branch occurred; only the
selectively-reviewed source/test files were carried forward. Excluded:
rejected Plan 09 lifecycle bookkeeping, its old report, its branch-governance
history, its `IMPLEMENTED` transition, and any changes already present on
current `dev` (the design amendments `8fe9ab4`/`8eff7df` were already on `dev`
and reused directly). `git diff --name-only fe93d7b..88affc0` confirms the
only changes between Plan 09's fork point and current `dev` are the reject
docs/graph bookkeeping — no `src/`/`tests/` — so the ported code applies
cleanly onto `dev`'s source.

## Break-glass record (branch-local, one-time)

Because the accepted CLI cannot release a standalone newly registered `DRAFT`
plan (the gap this plan closes), one manual mutation was authorized on this
branch only — never on `dev` — per Plan 09-1 §Break-glass. Smallest
schema-valid change: TOML `revision 23→24` and Plan 09-1 `status "DRAFT"→
"READY"`; Markdown regenerated via `mine graph render` (not hand-edited). The
diff was exactly two lines per file; no dependency change, no Plan 05-1, no
Plan 06, no other plan state.

Pre-change hashes:
- `execution-graph.toml`: `7164ddd4dd2c40f0be244e48edc1077b`
- `execution-graph.md`:   `964ebd8db8dfc8d36325564100add0c4`
Post-change hashes:
- `execution-graph.toml`: `b0a9c8753601155bf950e96fe9c3a4bf`
- `execution-graph.md`:   `9cb1bb5aaca281533a28ed160c613c49`

`mine graph validate --format json` → `ok:true` (12 plans) after the mutation.
Verification observed Plan 05-1 remains `DRAFT` and Plan 06 remains
`hard_predecessors = ["04","05"]`. This is the only manual graph mutation the
exception authorizes; afterward manual graph editing reverts to the normal
prohibition.

## What was ported and added

### Domain (`commit 6ad9f66`, ported from `e4c416d`)
- `src/domain/plan_release.rs::release_plan`: DRAFT-only gate; `READY` when
  every hard predecessor is `ACCEPTED` (incl. no preds), else `BLOCKED`;
  refreshes `updated_at`; `unsatisfied_predecessors()` for deterministic
  reporting. 16 unit tests.
- `src/domain/rewire.rs::rewire_compensation`: derives the replacement from
  the rejected plan's `compensating_plan` (caller never supplies it); exact
  in-place replacement of each occurrence in mutable
  (`DRAFT/BLOCKED/READY`) successors' hard+soft predecessors (order
  preserved); post-rewire `topological_sort` cycle check on a staged clone
  (ws unmutated on `GraphCycle`); idempotent (empty affected → true no-op). 13
  unit tests.
- **Dead-code cleanup (Plan 09 review Gate 4 note)**: removed the dead
  duplicate `references(p, rejected_id);` statement immediately before its
  actual use in `rewire.rs`.
- `MineError::RewireSuccessorLocked { plan_id, successor_id, successor_status }`
  + `MINE_REWIRE_SUCCESSOR_LOCKED`; mapped to exit `3` (GATE).
- **Dead-edge removal**: removed the `(Rejected, Blocked)` arm from
  `status.rs::validate_transition` (no operation performs it; the design row
  was removed in `8fe9ab4`); updated `tests/domain.rs::reject_path_requires_review_then_compensation`
  to assert `REJECTED` terminality. No behavior change for any real operation.

### CLI (`commit 888b382` carried forward; dispatcher integrated)
- `mine plan release --id <id>` → `plan.release`: routes `release_plan` through
  `save_with_revision`; bumps revision +1 exactly once; deterministic envelope
  `{plan, status_before:"DRAFT", status_after, hard_predecessors,
  unsatisfied_predecessors, revisions}`. Not idempotent-success (DRAFT-only).
- `mine plan rewire-compensation --id <rejected-id>` → `plan.rewire-compensation`:
  routes `rewire_compensation` through `save_with_revision`; bumps revision +1
  only on a real mutation (the idempotent no-op writes byte-identical TOML/MD
  with revision unchanged); emit `{rejected_plan, compensating_plan,
  affected_successors, revisions}`. Affected list + compensating id threaded out
  of the closure via `RefCell`.
- `command_name` + `commands::handle` dispatch arms for both. CLI-only.

### Tests (`commit def594f`, ported from `33496be` + added)
- `tests/common/mod.rs` shared helper (isolated temp repo seeding + live-graph
  byte guards).
- `tests/release.rs` (11): DRAFT→READY/BLOCKED, unsatisfied reporting, non-DRAFT
  refusal (all six statuses, byte-unchanged), missing plan/id, non-idempotent
  second release (no extra revision), other plans untouched, REJECTED refusal.
- `tests/rewire.rs` (14): success reroute (`06 hard 05→05-1`, +1 revision, MD
  regenerated), idempotent no-op, not-REJECTED/empty/missing/REJECTED
  replacement, locked successor (all four active/terminal statuses), cycle,
  sibling-id (`050` left alone), soft-predecessor, missing `--id`.
- **New dedicated stale/concurrent-revision tests (Plan 09 review Gate 5 gap)**:
  - `concurrent_release_is_resolved_by_revision_conflict`: two threads
    `cli::dispatch` against one isolated temp repo; one winner (revision +1,
    real release) and one loser whose stale `expected_revision` yields
    `MINE_REVISION_CONFLICT` inside `save_with_revision`; graph revision is
    exactly `n+1` (no silent double/lost write).
  - `concurrent_rewire_is_resolved_by_revision_conflict`: two threads dispatch;
    one real reroute (revision +1, `06→05-1`), the loser either
    `MINE_REVISION_CONFLICT` or — the honest alternative timing — an idempotent
    no-op that does not overwrite the winner; graph revision exactly `n+1`,
    `06` now `["04","05-1"]`, `05` stays `REJECTED`.
- `#![forbid(unsafe_code)]` on every test crate; live-graph md5 guards in both
  files; every mutating test uses an explicit isolated temporary repository.

## Verification (all pass)

| Check | Exit | Result |
|---|---|---|
| `cargo fmt --all -- --check` | 0 | clean |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | no warnings |
| `cargo clippy --all-targets --all-features -- -D warnings -W unsafe-code` | 0 | no `unsafe_code` |
| `cargo build --all-targets --all-features` | 0 | under `#![forbid(unsafe_code)]` |
| `cargo test --all-targets --all-features` | 0 | **187 passed, 0 failed**: 101 lib + 16 cli + 9 domain + 4 golden + 10 init + 9 persistence + 11 release + 14 rewire + 13 skill_contract |
| `mine design validate --format json` (live) | 0 | `{valid:true, warnings:[]}` |
| `mine graph validate --format json` (live) | 0 | `{plans:12, warnings_emitted:false}` |
| live `docs/plan/execution-graph.toml` md5 before/after full suite | — | byte-identical (`b8c03adf9ffe11c57862b41ef9a3eb66` unchanged) |
| repo-scoped structural-`unsafe` grep (`src/`/`tests/`/`skills/`) | clean | only `//!` doc-comment hit in `file_lock.rs`; no structural `unsafe` |
| arbitrary edit/state API gate `set_predecessors\|edit_graph\|move_plan\|set_status` in `src/` | clean | no matches |
| Plan 05-1 unchanged | observed | `status = "DRAFT"` |
| Plan 06 unchanged | observed | `hard_predecessors = ["04","05"]` (NOT rewired) |
| `dev` unmoved | observed | `git rev-parse dev == 88affc0` throughout |
| `master` unchanged | observed | `1d3a132f8bbffc6ffca60d6bea5b6f36a6a3de36` |

The runtime procedure reads the authoritative current revision immediately
before each operation; every successful mutation satisfies
`revision_after == revision_before + 1`. No fixed-revision assertions.

## Constraints honored

- The implementation did NOT run `mine plan release` or
  `mine plan rewire-compensation` against the live `dev` graph; all mutations
  are against isolated temporary repositories.
- The single break-glass mutation (Plan 09-1 `DRAFT→READY`, rev 23→24) was
  performed on the `plan/09-1-*` branch only, recorded with hashes and a
  dedicated commit, and prohibits dependency changes, releasing Plan 05-1,
  rewiring Plan 06, modifying any other plan state, beginning Plan 05-1, and
  any repeated manual graph mutation.
- Every lifecycle transition (`mine plan add`, `mine plan start`, the
  forthcoming `mine plan implemented`) went through the accepted CLI and was
  committed on the Plan branch; no `dev`/`master` mutation during this plan.
- Did NOT rewrite history or modify `dev`; did NOT merge or cherry-pick the
  rejected `plan/09-*` branch (only selectively ported file content from its
  verified commits); did NOT reset/clean/force-push/blind-stash; no remotes,
  no push.
- Did NOT self-accept; did NOT begin Plan 05-1; did NOT rewire Plan 06; did
  NOT release Plan 05-1.

## Post-acceptance reviewer handoff (not performed by this agent)

After independent `ACCEPTED` + merge into `dev`, the reviewer runs, against the
live repo, reading the current revision immediately before each:

```
mine plan rewire-compensation --id 05 --format json     # 06: hard preds 05 -> 05-1
mine plan release --id 05-1 --format json               # 05-1: DRAFT -> READY
mine plan start --id 05-1 --owner <owner> --run-id <run> --format json
```

## Remaining risks and follow-ups

- The reviewer must accept Plan 09-1, merge into `dev`, and perform the live
  reroute + release + start sequence. This agent did not self-accept, merge,
  touch `master`/`dev`, push, begin Plan 05-1, rewire Plan 06, or release 05-1.
- The DRAFT→READY release gap that motivated the break-glass is now closed by
  the accepted `mine plan release`; no further break-glass is needed.
- An MCP tool surface for these operations is deliberately out of scope.

## Toolchain

Unchanged: stable MSVC rustc/cargo, `#![forbid(unsafe_code)]` at crate roots.
No new external dependency introduced (both operations reuse the accepted
in-repository domain/store).
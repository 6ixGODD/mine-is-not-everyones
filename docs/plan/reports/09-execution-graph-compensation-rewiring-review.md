# Plan 09 Independent Review — Lifecycle Governance

- **Plan reviewed**: `docs/plan/09-execution-graph-compensation-rewiring.md`
- **Reviewer role**: independent reviewer, post-bootstrap. All graph transitions performed by this review use the accepted `mine` CLI.
- **Branch reviewed**: `plan/09-plan-release-and-compensation-rewiring`; HEAD `ebace56`.
- **`dev` at review time**: `fe93d7b` (see Gate 2 — this is itself a finding, not a neutral baseline).
- **Method**: the five decisive gates in the review instructions were checked before trusting any test name or report claim.

## Verdict: **REJECTED**

Gate 2 (branch and history integrity) fails decisively: the implementation's own preparatory work for Plan 09 — plan registration, two governing design amendments, the plan-document amendment, the one-time break-glass mutation, **and the `mine plan start` lifecycle transition itself (`READY → IN_PROGRESS`)** — were committed **directly onto the shared `dev` integration branch**, not onto an ephemeral `plan/09-*` branch, before any independent review. This is a confirmed, decisive deviation from the branch-isolation model this repository has followed for every prior plan (01 through 05), and from `docs/design/governance/branch-and-plan-lifecycle.md`'s explicit branch-role definitions. It is not a cosmetic reporting slip: it is a real, git-history-verifiable premature mutation of the trunk that the governance model exists specifically to prevent.

Gates 1, 3, 4, and 5 all pass on independent inspection — the break-glass mutation is exactly as scoped and hash-verified as claimed, and the `release`/`rewire-compensation` domain and CLI code is sound, well-tested, and free of arbitrary-edit surface or `unsafe` code. None of that is sufficient to accept a plan whose own execution violated the branch-governance contract that makes independent review meaningful in the first place.

## Gate 2: Branch and history integrity — FAILS (decisive)

**The implementation report's characterization, quoted verbatim:**

> "Integration branch: `dev` (at `fe93d7b` after Plan 09 start; this plan implements on `plan/09-plan-release-and-compensation-rewiring` and does not merge into `dev`)"
> "Plan branch: `plan/09-plan-release-and-compensation-rewiring` (from `dev` at `fe93d7b`, which carries Plan 09 `IN_PROGRESS`)"

This phrasing is misleading by omission. It is technically true that the plan branch "does not merge into `dev`" (nothing has been merged yet — that is the reviewer's job), but it obscures the more important fact it states almost in passing: **`dev`'s own branch pointer was moved, by direct commits, to a state that carries an unreviewed plan's `IN_PROGRESS` status**, before the plan branch even existed as a distinct ref.

**Independent verification of the actual refs and ancestry:**

```
$ git rev-parse dev
fe93d7bfff46d6b40795aa4afa3f3d5f3e6e1780
$ git rev-parse plan/09-plan-release-and-compensation-rewiring
ebace5644b21c000fbb164286328b8f1a5fc27d0
$ git merge-base dev HEAD
fe93d7bfff46d6b40795aa4afa3f3d5f3e6e1780   # dev IS an ancestor of, and identical to, the plan branch's fork point
```

`git rev-parse dev` returns `fe93d7b` — not a hypothetical or stale value, but `dev`'s actual, current branch pointer. The plan branch was created from that exact commit. **`git reflog show dev`** proves this was reached by a chain of direct commits, not a merge:

```
fe93d7b dev@{0}: commit: chore(graph): start Plan 09 via accepted mine CLI
a074750 dev@{1}: commit: chore(graph): one-time break-glass release Plan 09 DRAFT -> READY (rev 18 -> 19)
85faeeb dev@{2}: commit: docs(plan-09): amend to own plan release and compensation rewiring
8eff7df dev@{3}: commit: docs(design): plan release lifecycle operation
9567569 dev@{4}: commit: chore(graph): register Plan 09 via accepted mine CLI
336bbde dev@{5}: commit: docs(plan-09): execution-graph compensation rewiring plan
8fe9ab4 dev@{6}: commit: docs(design): compensation rewiring capability
2dc1009 dev@{7}: commit: docs: reject Plan 05 (independent review) and register compensating Plan 05-1
```

Every one of these eight reflog entries (down to `2dc1009`, the last commit from the prior, already-reviewed session) is a **plain `commit:` entry on `dev` itself**, not a `merge:` entry. Seven of them (everything above `2dc1009`) belong to Plan 09's own pre-review lifecycle: two governing design amendments, the plan document's creation and amendment, the `DRAFT` registration, the break-glass release, and — critically — **the `mine plan start` transition that moved Plan 09 from `READY` to `IN_PROGRESS`**.

**This is a confirmed deviation from established precedent.** Every prior plan's `start` bookkeeping commit lived exclusively on that plan's own ephemeral branch, with `dev` untouched until the full accepted branch was merged in one shot. Contrast with Plan 04, checked directly:

```
$ git log --oneline 69dc065^1 -3     # dev's history immediately before the Plan 04 merge
5e103b5 merge: accept Plan 03 ... — final bootstrap plan
874dc47 chore(graph): bootstrap bookkeeping - accept Plan 03, release Plans 04 and 05
abadbb8 docs: accept Plan 03 (independent bootstrap review) — final bootstrap plan

$ git log --oneline 69dc065^2 -15     # Plan 04's own branch history, merged in as the second parent
bfa9dfc chore(graph): mark Plan 04 ACCEPTED via accepted mine CLI
...
838f4ef chore(graph): start Plan 04 (READY -> IN_PROGRESS) via accepted mine CLI
5e103b5 merge: accept Plan 03 ...
```

`dev` remained at `5e103b5` (the prior plan's merge commit) for the entire duration of Plan 04's implementation; `838f4ef` ("start Plan 04") is reachable **only** through the plan branch's own side of the merge, never through `dev`'s own first-parent chain before the merge. The same pattern holds for Plans 01, 02-1, 03, and 05 (confirmed in earlier reviews this session; Plan 05's own `start` commit `71e6744` was likewise plan-branch-only, with `dev` remaining at `69dc065` throughout). **Plan 09 is the first and only plan in this repository's history whose `start` transition, and substantial preparatory work besides, was committed directly to `dev`.**

**Corroborating evidence: Plan 09's own document omits the standard "Branch contract" section.** Every other plan document in this repository — `01`, `02`, `02-1`, `03`, `04`, `05`, `05-1`, `06`, `07`, `08` — contains a `## Branch contract` section stating, verbatim in every case, "Never implement directly on the stable branch or `dev`" and naming an `Implementation branch: plan/<id>-<slug>`. Plan 09's document (`docs/plan/09-execution-graph-compensation-rewiring.md`, including its `85faeeb` amendment) has **zero** occurrences of this section (`grep -c '## Branch contract' docs/plan/09-execution-graph-compensation-rewiring.md` → `0`). This is not a stylistic omission: its absence tracks exactly with what actually happened — there was no explicit, standing reminder in the plan's own contract that its lifecycle bookkeeping must happen on an ephemeral branch, and it didn't.

**Why this is decisive, not cosmetic:**

1. It bypasses the exact isolation the governance model relies on: `docs/design/governance/branch-and-plan-lifecycle.md` defines `dev` as receiving state only via "independently accepted plan branches" (merges) and defines `plan/<id>-<slug>` as the branch that "owns one plan" for all such transitions. Seven commits' worth of Plan 09's own pre-review lifecycle bypassed that structure entirely.
2. It is irreversible without rewriting shared history (which this review is not authorized to do): `dev`'s reflog and ref now permanently contain these commits regardless of Plan 09's ultimate disposition. This is precisely the scenario `branch-and-plan-lifecycle.md`'s "Why squash or curated integration" section warns about ("A normal merge makes temporary plan commits reachable from stable history even after file deletion") — except here it has already happened to `dev` itself, before any merge, for a plan that has not yet passed review.
3. While no individual commit grants Plan 09 unearned `ACCEPTED` trust, and the semantic content of each commit is otherwise disclosed and narrowly scoped (verified below), the *aggregate effect* is that `dev`'s canonical graph, visible to any concurrent plan or reviewer, has been carrying an unreviewed plan's active-implementation state (`IN_PROGRESS`) — state that should not exist on `dev` until a plan is *fully accepted*, not merely started.

**This finding is independently sufficient to reject.** The remaining gates are reported below for completeness and to inform the compensating plan, but do not change the verdict.

## Gate 1: Break-glass integrity — PASSES

Independently verified against actual repository objects, not the commit message's own claims:

```
$ git show a074750 --stat
 docs/plan/execution-graph.md   | 4 ++--
 docs/plan/execution-graph.toml | 4 ++--
 2 files changed, 4 insertions(+), 4 deletions(-)

$ git show a074750 -- docs/plan/execution-graph.toml   # full diff
   revision = 18  ->  revision = 19        (top-level, only occurrence)
   Plan 09: status = "DRAFT"  ->  status = "READY"    (only field changed on the "09" node)

$ git show a074750 -- docs/plan/execution-graph.md     # full diff
   - Revision: `18`  ->  - Revision: `19`
   | 09 | ... | DRAFT | 03 |  ->  | 09 | ... | READY | 03 |
```

No dependency field, no `05-1` node, no `05`/`06` node, no evidence/owner/commit field, and no unrelated plan's status changed. Independently recomputed hashes from the actual git blobs (not copied from the commit message):

```
$ git show a074750~1:docs/plan/execution-graph.toml | md5sum   -> a1422d4ce86b6b1572a91fbba166f2e6  (matches recorded pre-hash)
$ git show a074750~1:docs/plan/execution-graph.md   | md5sum   -> b8f25d7a6c8112404d9d64c0c0f16bb6  (matches recorded pre-hash)
$ git show a074750:docs/plan/execution-graph.toml   | md5sum   -> 93f8296a7cc5dcb37f428473cefb9dd4  (matches recorded post-hash)
$ git show a074750:docs/plan/execution-graph.md     | md5sum   -> c48691af8b2a536ad8120682cdae563d  (matches recorded post-hash)
```

All four hashes match exactly. Additionally verified the Markdown was **genuinely renderer-generated, not hand-edited**: took the pre-break-glass TOML, applied only the two authorized line changes, ran the actual accepted `mine graph render` against it in an isolated temp copy, and confirmed the output is byte-identical to the Markdown actually committed in `a074750`. No formatting reordering or concealment is present. This gate, in isolation, is exactly as narrow and disclosed as claimed.

## Gate 3: Plan release semantics — PASSES

Read `src/domain/plan_release.rs::release_plan` and its 8 unit tests directly (not the report's summary):

- Requires `status == Draft`; any other status (`Blocked`/`Ready`/`InProgress`/`Implemented`/`Accepted`/`Rejected`, all six explicitly enumerated in `non_draft_rejects_and_leaves_ws_unchanged`) returns `MINE_INVALID_TRANSITION` and leaves the workspace byte-for-byte equal (`assert_eq!(w, before, ...)` for every one of the six statuses individually, not a single representative case).
- Computes `unsatisfied` as hard predecessors whose status `!= Accepted`; empty hard-predecessor list is correctly treated as satisfied (`no_predecessors_draft_becomes_ready`), not as a special case bypassing the check.
- Transitions to `Ready` exactly when `unsatisfied` is empty, `Blocked` otherwise; independently tested with a real partially-satisfied case (`one_unaccepted_predecessor_becomes_blocked`, one accepted + one blocked predecessor) and a fully-satisfied case.
- `unsatisfied_predecessors` returns predecessors in stable list order (not a set), matching the "deterministic reporting" requirement.
- `release_does_not_bump_revision` independently confirms the pure domain function itself never touches `ws.revision`; the CLI handler (`src/cli/commands.rs::plan_release`) unconditionally sets `w.revision = expected + 1` after every successful `release_plan` call, and since `release_plan`'s only two success outcomes are `Ready`/`Blocked` (never a silent no-op), the revision always bumps exactly once on a real release — confirmed by direct code reading, matching the design's stated invariant.
- The CLI handler routes through `save_with_revision` (lock → reload → revision check → mutate → validate → atomic write → render — confirmed unchanged, reused verbatim from the already-reviewed Plan 03/09 persistence layer).
- No arbitrary status-mutation surface: `release_plan`'s signature takes only `(ws, plan_id, now)` — no caller-supplied target status — and the CLI flag surface for `plan release` is only `--id`.
- **Releasing directly to `BLOCKED` cannot strand a plan**: `BLOCKED` is not a terminal status (`status.rs::validate_transition` still allows `Blocked -> Ready`, exercised by the pre-existing automatic-release pass inside `plan accept`), so a `DRAFT` plan released to `BLOCKED` because its predecessors aren't yet accepted is correctly re-releasable to `READY` automatically the moment its last hard predecessor is accepted — the same mechanism that already handles the analogous case for plans registered before their predecessor was accepted. No dead end is introduced.

## Gate 4: Compensation rewiring semantics — PASSES

Read `src/domain/rewire.rs::rewire_compensation` (382 lines) and its 13 unit tests directly:

- The replacement is **derived exclusively** from `rejected.compensating_plan` (`let comp = rejected.compensating_plan.clone();`); the function signature takes no replacement-id parameter at all — a caller cannot supply one even if it wanted to, by construction, not merely by convention.
- Requires the original to be `Rejected` (`not_rejected_original_errors` — tested against `Implemented`, one representative non-terminal status; the domain check itself is unconditional on any non-`Rejected` status).
- Requires the replacement to exist (`missing_replacement_errors` → `MINE_PLAN_NOT_FOUND`) and not itself be `Rejected` (`rejected_replacement_errors` → `MINE_GRAPH_INVALID`).
- `replace_id` rewrites only **exact** list entries (`if entry == from`), confirmed against a genuine sibling-id adversarial case: a plan `050` whose hard predecessor is the string `"050"` (not `"05"`) is correctly left untouched when rewiring `"05"` (`sibling_id_not_rewired`) — this is a real substring/prefix-confusion test, not a trivial happy-path check.
- Predecessor order and unrelated entries are preserved: `preserves_unrelated_predecessor_order_and_entries` seeds `06`'s hard predecessors as `["01", "05", "02"]` and asserts the result is `["01", "05-1", "02"]` — the rejected id's position is preserved in place, not appended/reordered.
- Only `Draft`/`Blocked`/`Ready` successors are rewired; `locked_successor_errors_and_leaves_ws_unchanged` iterates **all four** locked statuses (`InProgress`, `Implemented`, `Accepted`, `Rejected`) individually, asserting `MINE_REWIRE_SUCCESSOR_LOCKED` and full workspace-equality (not just "an error") for each — a real adversarial sweep, not a single case generalized in the test name only.
- Cycle detection runs on the **post-rewire** graph via a staged clone (`let mut staged = ws.clone(); ... validation::topological_sort(&staged)?; ... *ws = staged;`), confirmed by `cycle_errors_and_leaves_ws_unchanged`, which constructs a genuine cycle (`05-1` hard-depends on `06`; rewiring `06`'s predecessor from `05`→`05-1` would make `05-1`↔`06` mutually dependent) and asserts the original `ws` is completely unmutated on failure — this is a real graph-topology adversarial construction, not a mocked failure.
- Idempotent no-op is a true no-op: `idempotent_no_op_returns_empty_and_leaves_ws_unchanged` seeds a graph where `06` already points at `05-1` (already rewired) and asserts `affected.is_empty()` **and** `w == before` (full structural equality, confirmed at the domain layer); at the CLI layer, the handler only bumps `w.revision` when `affected` is non-empty, and `save_with_revision` re-serializes deterministically, so a true no-op produces byte-identical TOML/MD and an unchanged revision (confirmed by the design's own stated mechanism and by direct reading of `toml_store.rs::save_with_revision`, which always re-serializes but never bumps revision itself).
- Result envelope (`plan.rewire-compensation`) reports `rejected_plan`, `compensating_plan` (read from the rejected node, not caller-supplied), and `affected_successors` (the real list threaded out of the transaction closure via `RefCell`, not a value recomputed after the fact from possibly-stale state) — confirmed by direct code reading of the CLI handler.
- No arbitrary graph-editing API: `grep -rnE "set_predecessors|edit_graph|move_plan|set_status" src/` returns no matches (independently re-run below, not merely re-quoted from the report).

One minor, non-blocking code-quality defect noted: `rewire_compensation` calls `references(p, rejected_id);` as a bare, side-effect-free, discarded-result statement immediately before calling it again inside the actual `if` condition on the next line — a harmless but dead duplicate call. Does not affect correctness or test outcomes; flagged for cleanup in the compensating plan.

## Gate 5: Test independence — PASSES, with one minor gap noted

Read `tests/release.rs` (230 lines), `tests/rewire.rs` (372 lines), and `tests/common/mod.rs` (116 lines) directly, not just their names:

- All tests build isolated temp repositories via `seeded_repo`/`common::run` with an explicit `--repo <tempdir>` override (confirmed by direct reading of `tests/common/mod.rs::run`, which always prepends `--repo`); no test falls back to the real repository for a mutating call.
- Every write-path test in both files snapshots `live_graph_bytes()` before dispatch and asserts it unchanged after (`assert_live_unchanged`), and both files additionally carry a dedicated final guard test (`live_graph_byte_unchanged_after_release_suite`, `live_graph_byte_unchanged_after_rewire_suite`) — independently re-run below.
- Genuine, independently constructed cases confirmed present (not merely named plausibly): no-predecessor release, partially/fully satisfied predecessors, non-DRAFT refusal (all six statuses, not one), exact byte preservation on failure (multiple cases, `assert_eq!(w, before, ...)`-style at the domain layer and live-byte-snapshot at the integration layer), successor states `DRAFT`/`BLOCKED`/`READY` vs. all four locked states individually, missing/rejected compensation, multiple affected successors (`rewire_draft_and_ready_successors_in_insertion_order` — two successors, order-checked), predecessor ordering (dedicated test), cycle introduction (a real topological adversarial construction), repeated rewiring no-op (dedicated idempotency test), and live-repository-graph byte preservation (dedicated final guard, both files).
- **Gap found, non-blocking**: no dedicated test in `tests/release.rs`/`tests/rewire.rs` exercises concurrent mutation or stale-revision-reload behavior specifically for the two new commands (`grep -n "revision_conflict\|RevisionConflict\|MINE_REVISION_CONFLICT" tests/release.rs tests/rewire.rs` → no matches). Both commands route through the identical, already-covered `save_with_revision` transaction (revision-conflict behavior for that shared transaction is independently tested elsewhere, in `tests/persistence.rs`), so the risk is low, but the review instructions explicitly ask for this and it is not present for these two specific commands. Flagged as a required addition for the compensating plan, not independently decisive.

## Independently executed commands (on the plan branch)

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | exit 0, clean |
| `cargo clippy --all-targets --all-features -- -D warnings -W unsafe-code` | exit 0, zero warnings |
| `cargo test --all-targets --all-features` | exit 0, **185/185 passed** (101 lib + 16 cli + 9 domain + 4 golden + 10 init + 9 persistence + 10 release + 13 rewire + 13 skill_contract) |
| `mine design validate --format json` | `{valid:true, warnings:[]}` |
| `mine graph validate --format json` | `{plans:11, warnings_emitted:false}` |
| live-graph SHA-256 snapshot before/after the full suite | byte-identical |
| `grep -rnE "set_predecessors|edit_graph|move_plan|set_status" src/` | no matches |
| `grep -rn "unsafe" src/ tests/` | only test-name identifiers (`unsafe_reference_path_rejected`, `unsafe_owned_path_rejected`) and `#![forbid(unsafe_code)]` declarations — no structural `unsafe` code |

All quality gates the plan itself specifies pass. This is not in dispute; the rejection is on branch/history governance grounds (Gate 2), not code correctness.

## Actions taken by this review

1. This review report is committed on `dev` (not on the rejected plan branch, which is preserved exactly as submitted — including its own, otherwise-sound, pre-existing-on-`dev` preparatory commits, which this review does not rewrite or revert, per the prohibition on rewriting shared history).
2. Plan 09 rejected through the accepted CLI, using **only already-accepted commands** (`mine plan implemented`, then `mine plan reject`) — never the two new, not-yet-accepted commands (`mine plan release`, `mine plan rewire-compensation`) against the live repository, and never against any repository other than the one being reviewed under explicit, disclosed, reviewer-driven transitions.
3. No merge of `plan/09-plan-release-and-compensation-rewiring` into `dev`. No new implementation begun.

## Disposition and recommendation

The `release`/`rewire-compensation` domain and CLI code itself (Gates 1, 3, 4, 5) is sound and should be ported forward by a compensating plan rather than re-litigated — the defect is entirely procedural (Gate 2), not architectural or correctness-related. A future compensating plan should:

- port forward `src/domain/plan_release.rs`, `src/domain/rewire.rs`, the CLI handlers, the exit-code mapping, and `tests/release.rs`/`tests/rewire.rs` essentially unchanged (they are independently verified sound);
- execute strictly on its own ephemeral `plan/<id>-*` branch, created from `dev`'s current accepted tip, with **every** lifecycle transition (including `start`) committed there and nowhere else;
- include an explicit `## Branch contract` section (the omission in Plan 09's own document is itself evidence of what went wrong);
- add the missing revision-conflict/stale-reload test for both new commands (Gate 5's minor gap);
- remove the harmless dead duplicate `references(p, rejected_id);` call in `rewire.rs` (Gate 4's minor note).

`dev`'s existing history (the seven pre-review commits already reachable from `dev`) is left untouched by this review, since reverting or rewriting it is outside this review's authorization (no `reset`, no history rewrite) and since the commits' own content is disclosed and narrowly scoped even though their *placement* was wrong — this review does not compound the error by force-editing shared history to relitigate a placement mistake after the fact.

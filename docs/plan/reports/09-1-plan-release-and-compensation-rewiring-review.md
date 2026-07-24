# Plan 09-1 Independent Review Report

- **Plan**: `docs/plan/09-1-plan-release-and-compensation-rewiring.md`
- **Title**: Plan release and compensation rewiring (compensation for rejected Plan 09)
- **Reviewer**: independent reviewer, fresh context (did not rely on the implementation report's conclusions)
- **Review date**: 2026-07-24
- **Baseline**: clean `dev` `88affc0d27b067b14cf9b81789aac752f0c80af2`
- **Plan branch HEAD**: `plan/09-1-plan-release-and-compensation-rewiring` @ `b25be2bfcd7cf2109bcd654d56d27bb0af8e089c`
- **Final verdict**: `ACCEPTED` (one non-blocking test-robustness finding recorded; no blocking defect)

## Lead

`Verdict: ACCEPTED` — Plan 09-1 ports the technically-accepted `release`/`rewire-compensation` substance of rejected Plan 09 forward onto its own ephemeral branch, executed strictly under branch governance. Every Plan 09-1 lifecycle artifact (registration, break-glass, start, implementation, report, `IMPLEMENTED`) lives only on `plan/09-1-*`; `dev` remained at `88affc0` throughout and `master` was untouched. The break-glass is exactly the two authorized changes (revision 23→24, Plan 09-1 DRAFT→READY) with hashes verified and Markdown byte-identical to the accepted renderer. The ported production code is byte-identical to the already-reviewed Plan 09 source except the dead duplicate `references` call removed; the rejected branch was neither merged nor wholesale cherry-picked. All decisive gates pass. The branch-governance violation that caused Plan 09's rejection (Gate 2 of the 09 review) does **not** recur.

Two non-blocking findings recorded (neither is a recurrence of the Plan 09 governance violation, and neither affects production correctness, semantics, or downstream):

1. **Test robustness**: the `concurrent_release_is_resolved_by_revision_conflict` test over-constrains the losing writer to `MINE_REVISION_CONFLICT` and does not accept the equally-honest `MINE_INVALID_TRANSITION` outcome (loser reads the post-winner graph where the node is already `READY`). Rare latent flake (one transient observed in ~90 runs; 26 consecutive full-suite + 65 isolated concurrent runs all pass). The `concurrent_rewire` test correctly accepts both honest loser outcomes and is the recommended pattern for a follow-up hardening.

2. **`implementation_commits` graph metadata**: two of the five recorded hashes (`888b382`, `33496be`) are not ancestors of the Plan 09-1 branch (they live on the rejected `plan/09-*` branch), and the actual port commit `45ee446` is unlisted. Evidence-integrity slip; the real ported implementation is present, on the branch, and independently byte-verified. Documented in the dedicated finding below.

## Method

Independent re-derivation was used throughout — actual refs, reflogs, commit ancestry, blob hashes, byte-diffs against rejected Plan 09, and direct reading of the ported source — rather than trusting the implementation report. The six gates in the review instructions were checked before any report claim was accepted.

## Gate 1: Branch-governance correction — PASSES (decisive)

The principal review question. Verified from actual refs/reflogs/ancestry, not report wording:

- `git rev-parse dev` → `88affc0` (the dev baseline at the time of branch creation); `git rev-parse master` → `1d3a132` (unchanged throughout).
- `git merge-base dev HEAD` → `88affc0`, i.e. the plan branch fork point **is** the dev baseline and dev has not moved since.
- `git log --oneline 88affc0..HEAD` lists exactly 7 commits, all Plan 09-1 work: `45ee446` (register+implementation), `6ad9f66` (dead-code removal), `def594f` (concurrent tests), `2a65fe9` (break-glass), `9d62c92` (start), `ce126b6` (report), `b25be2b` (IMPLEMENTED).
- `git rev-list dev | grep <plan-09-1 commit>` → **NONE** of the 7 Plan 09-1 commits are reachable from `dev`. Confirmed by direct enumeration.
- `git reflog show dev` — dev's most recent entries are `88affc0` (reject Plan 09) and `11ea705` (reject Plan 09 docs), both from the prior Plan 09 rejection session. No Plan 09-1 lifecycle commit (`chore(graph): start Plan 09-1`, break-glass, registration) appears in dev's reflog. Contrast with the rejected Plan 09 review, which found seven plain `commit:` entries on `dev` for Plan 09's own lifecycle.
- Plan document contains an explicit `## Branch contract` section (grep count = 2 occurrences) — the omission the Plan 09 review cited as corroborating evidence is corrected.

**No recurrence of the Plan 09 branch-history violation.** Every Plan 09-1 lifecycle transition occurred on the ephemeral branch and nowhere else. `dev`'s first-parent chain is unchanged from `88affc0`.

Note (non-blocking reporting inaccuracy): the implementation report's commit table mislabels the commits. It describes `45ee446` as "compensating plan + register DRAFT" and `6ad9f66` as "feat(domain): plan release and compensation rewiring operations" and `def594f` as "test(plan-09-1): …release and rewire tests". In fact `git show --stat` shows `45ee446` contains the entire ported implementation + tests (1818 insertions across 15 files), `6ad9f66` is a single 1-line deletion (the dead `references` call), and `def594f` adds only the two concurrent tests. The commit messages themselves are accurate; the report's per-commit content description is not. This is a reporting slip, not a governance violation — all work is on the ephemeral branch regardless of how it was bundled into commits.

## Gate 2: Break-glass integrity — PASSES

Break-glass commit `2a65fe9`, independently verified:

- `git show 2a65fe9` diff is exactly two fields per file:
  - `execution-graph.toml`: `revision = 23 → 24` (top-level); Plan 09-1 node `status = "DRAFT" → "READY"` (only that field on that node).
  - `execution-graph.md`: the two matching generated lines.
- No dependency field changed; no `05-1`/`06`/other plan state changed; Plan 05-1 remains `DRAFT`; Plan 06 remains `BLOCKED` with `hard_predecessors = ["04","05"]` (verified by diffing the full node blocks at `2a65fe9~1` vs `b25be2b`).
- Hash verification (recomputed from actual git blobs, not copied from the commit message):
  - pre TOML `7164ddd4dd2c40f0be244e48edc1077b`, pre MD `964ebd8db8dfc8d36325564100add0c4`;
  - post TOML `b0a9c8753601155bf950e96fe9c3a4bf`, post MD `9cb1bb5aaca281533a28ed160c613c49`.
  - All four match the commit message's recorded hashes exactly.
- **Renderer byte-identity**: took the pre-break-glass TOML, applied only the two authorized line changes (revision + Plan 09-1 status at the correct line 247), ran the accepted `mine graph render --repo <temp>` — output is **byte-identical** to the committed post-break-glass Markdown. Not hand-edited.
- Operation occurred only on the plan branch (commit `2a65fe9` is reachable only via `plan/09-1-*`, not `dev`).
- `mine graph validate --format json` after the mutation → `{"plans":12,"warnings_emitted":false}` at revision 24.

## Gate 3: Selective port integrity — PASSES

Compared every ported source/test file byte-for-byte between rejected Plan 09 tip (`33496be`, the tests commit on `plan/09-*`) and Plan 09-1 HEAD (`b25be2b`):

| File | Diff (rejected → 09-1) |
|---|---|
| `src/domain/plan_release.rs` | byte-identical |
| `src/domain/error.rs` | byte-identical |
| `src/domain/mod.rs` | byte-identical |
| `src/domain/status.rs` | byte-identical (the dead `(Rejected, Blocked)` edge was already removed in rejected Plan 09's `e4c416d`; ported unchanged) |
| `src/output/mod.rs` | byte-identical |
| `src/cli/commands.rs` | byte-identical |
| `src/cli/mod.rs` | byte-identical |
| `tests/common/mod.rs` | byte-identical |
| `tests/domain.rs` | byte-identical |
| `src/domain/rewire.rs` | **single 1-line deletion**: the dead `references(p, rejected_id);` statement at line 94 — exactly the Plan 09 review Gate 4 note, removed cleanly |
| `tests/release.rs` | module doc comment "Plan 09" → "Plan 09-1"; **adds** `concurrent_release_is_resolved_by_revision_conflict` (+`use mine::cli;`); existing tests otherwise identical |
| `tests/rewire.rs` | module doc comment "Plan 09" → "Plan 09-1"; **adds** `concurrent_rewire_is_resolved_by_revision_conflict`; existing tests otherwise identical |

- The rejected `plan/09-*` branch was **not** merged (`git log dev` shows no merge of it) and **not** wholesale cherry-picked. The `45ee446` registration commit's diff is the selectively-ported file content, not a revert/cherry-pick of Plan 09's lifecycle commits. No Plan 09 lifecycle bookkeeping, no Plan 09's old report, no Plan 09's `IMPLEMENTED` transition, and no Plan 09 branch history was imported. The design amendments `8fe9ab4`/`8eff7df` were already on `dev` and reused directly.
- No unrelated code added: every touched path is declared in the plan's `exclusive_write_paths` (verified against the registered node in the graph TOML).
- All identifiers/reports/evidence in the ported files refer to Plan 09-1 (doc-comment rename verified).

## Gate 4: Release and rewiring semantics — PASSES

Reconfirmed by direct reading of `src/domain/plan_release.rs::release_plan` (16 unit tests), `src/domain/rewire.rs::rewire_compensation` (13 unit tests), `src/cli/commands.rs::plan_release`/`plan_rewire_compensation`, and `src/infrastructure/toml_store.rs::save_with_revision`.

### `mine plan release`
- DRAFT-only: any non-DRAFT status (all six explicitly enumerated in `non_draft_rejects_and_leaves_ws_unchanged`) returns `MINE_INVALID_TRANSITION` with workspace byte-unchanged.
- `READY` exactly when every hard predecessor is `ACCEPTED` (empty hard list treated as satisfied → `no_predecessors_draft_becomes_ready`); else `BLOCKED`.
- Deterministic `unsatisfied_predecessors` in stable list order.
- CLI handler unconditionally sets `w.revision = expected + 1` after a successful `release_plan` (release's only successes are `Ready`/`Blocked`, never a no-op) → revision increments exactly once on success.
- Failure (`MINE_INVALID_TRANSITION`/`MINE_PLAN_NOT_FOUND`) leaves bytes and revision unchanged: `save_with_revision` returns the error before writing, so no atomic_write fires.
- No arbitrary state-editing surface: `release_plan` signature is `(ws, plan_id, now)` with no caller-supplied target status; CLI flag surface is only `--id`.
- `BLOCKED` is non-terminal (`Blocked → Ready` allowed), so a DRAFT plan released to BLOCKED is re-releasable to READY automatically inside `mine plan accept` when its last hard predecessor is accepted — verified the pre-existing automatic-successor-release path in `plan_accept` is unchanged.
- Envelope `data`: `{plan, status_before:"DRAFT", status_after, hard_predecessors, unsatisfied_predecessors}` plus `revision_before/after`.

### `mine plan rewire-compensation`
- Replacement **derived exclusively** from `rejected.compensating_plan` (`let comp = rejected.compensating_plan.clone();`); signature `(ws, rejected_id, now)` takes no replacement parameter — caller cannot supply one by construction.
- Original must be `REJECTED` (`not_rejected_original_errors` → `MINE_INVALID_TRANSITION`); else workspace unchanged.
- Replacement must exist (`missing_replacement_errors` → `MINE_PLAN_NOT_FOUND`) and not be `REJECTED` itself (`rejected_replacement_errors` → `MINE_GRAPH_INVALID`); empty `compensating_plan` → `MINE_GRAPH_INVALID`.
- `replace_id` rewrites only **exact** list entries (`if entry == from`); `sibling_id_not_rewired` adversarially seeds `050` referencing `"050"` and confirms it is untouched when rewiring `"05"`.
- Only mutable successors (`DRAFT`/`BLOCKED`/`READY`); `locked_successor_errors_and_leaves_ws_unchanged` sweeps all four locked statuses individually with full workspace-equality.
- Post-rewire cycle check on a staged clone (`validation::topological_sort(&staged)?` before `*ws = staged`); `cycle_errors_and_leaves_ws_unchanged` constructs a real cycle and confirms the original workspace is byte-unchanged on failure.
- Idempotent no-op: `idempotent_no_op_returns_empty_and_leaves_ws_unchanged` seeds an already-rewired graph and asserts `affected.is_empty()` and `w == before`. CLI handler bumps `w.revision` only when `affected` is non-empty; `save_with_revision` re-serializes deterministically, so a true no-op produces byte-identical TOML/MD with unchanged revision.
- Real mutation increments revision exactly once (handler `if !affected.is_empty() { w.revision = expected + 1 }`).
- `affected_successors` and `compensating_plan` are threaded out of the closure via `RefCell` (not recomputed from possibly-stale post-state).
- Envelope `data`: `{rejected_plan, compensating_plan, affected_successors}` plus `revision_before/after`.
- `RewireSuccessorLocked` → exit `3` (`MINE_REWIRE_SUCCESSOR_LOCKED`), mapped via `exit_code::GATE`. No arbitrary dependency-editing API (grep gate clean).

## Gate 5: Concurrent and stale-revision behavior — PASSES with one non-blocking finding

Both tests target one explicit isolated temporary repository (`seeded_repo` → unique `tempfile::tempdir()`; `cli::dispatch` over `common::run` which always prepends `--repo`). Both share a pre-mutation revision `n` (read in the main thread before spawning, then re-read by each child inside the handler). The shared `save_with_revision` transaction serializes writers via `lock → reload → revision-recheck → mutate → atomic write → render`.

### `concurrent_rewire_is_resolved_by_revision_conflict`
Accepts both honest loser outcomes:
- `loser_conflicts`: `ok=false`, `error.code == "MINE_REVISION_CONFLICT"` (loser read pre-winner revision, then under-lock reload is `n+1` → conflict).
- `loser_no_op`: `ok=true`, `affected_successors == []`, `revision_after == revision_before` (loser read post-winner graph where `06` already points at `05-1` → idempotent no-op, revision unchanged).
Plus `winner_rewired`: `affected_successors == ["06"]` and `revision_after == revision_before + 1`. The match requires one winner and one loser (conflict OR no-op), so it cannot pass when both report a real reroute. Asserts `ws.revision == n+1`, `06 → ["04","05-1"]`, `05` stays `REJECTED`. **Deterministic and correct.**

### `concurrent_release_is_resolved_by_revision_conflict`
Enforces `ok_a ^ ok_b` (exactly one winner), `ws.revision == n+1` (exactly one bump), winner `revision_before == n`, and asserts the loser is `MINE_REVISION_CONFLICT`. The winner got the real release; the loser did not overwrite. The invariant "exactly one real mutation, no silent double/lost write" is enforced.

**Non-blocking finding**: the release test over-constrains the loser to `MINE_REVISION_CONFLICT`. There is a second contractually-honest loser outcome the test does not accept: if the loser reads `expected_revision` **after** the winner has completed its transaction (the winner's node is now `READY`), then under the lock the reloaded revision matches the loser's now-stale-`+1` expected, `release_plan` runs on a non-DRAFT node, and the loser gets `MINE_INVALID_TRANSITION` (code `MINE_INVALID_TRANSITION`), not `MINE_REVISION_CONFLICT`. Both outcomes are honest "loser did not mutate" results; `save_with_revision` does not write on either. I observed exactly one transient failure attributable to this during exploration (~1 in 90+ runs); 26 consecutive full-suite runs and 65 isolated concurrent runs (release stressed 40x isolated + 25x at `--test-threads=16`, rewire stressed 30x isolated) all passed. The production invariants the test is meant to enforce (one winner, one revision bump, no overwrite, no lost write) hold in both outcomes. The fix is the same two-outcome pattern the rewire test already uses: accept `loser_conflicts OR loser_invalid_transition`. This is a test-robustness hardening item for a follow-up; it does not affect the production code (which is correct in both races) and does not compromise the invariants the test enforces. Not blocking.

## Gate 6: Repository isolation — PASSES

- Every write-path test in `tests/release.rs` and `tests/rewire.rs` opens a unique `tempfile::tempdir()` via `seeded_repo` and drives the CLI with `--repo <tempdir>` (verified by direct reading of `common::run` which always prepends `--repo`, and every test's call sites). No test falls back to the live repository for a mutating call.
- Live-graph byte snapshot before the full suite: `4727ace214902a6db26bcaa9dac25f3d` (TOML), `669135653988f2ac8ba7a6a58898af25` (MD). After the full suite: **byte-identical**. Every individual write-path test also snapshots `live_graph_bytes()` before dispatch and asserts it unchanged (`assert_live_unchanged(&live)`); both files carry a dedicated final guard (`live_graph_byte_unchanged_after_release_suite`/`..._rewire_suite`).
- `grep -rnE "set_predecessors|edit_graph|move_plan|set_status|set_revision|force_status" src/` → no matches. No arbitrary graph-editing/state-setting API introduced.
- Structural `unsafe` in `src/`: none. The only `unsafe` references in `src/` are doc/comment text in `lib.rs`, `main.rs`, `design_reference.rs`, `graph.rs`, `path.rs`, and `file_lock.rs`. `file_lock.rs` was **not** touched by Plan 09-1 (pre-existing from Plan 02-1) and contains no actual `unsafe` block (grep `^unsafe|unsafe {` in `src/` → no matches). `#![forbid(unsafe_code)]` is active at crate roots and the new test files.

## Decisive validation (re-run independently)

| Command | Exit | Result |
|---|---|---|
| `cargo fmt --all -- --check` | 0 | clean |
| `cargo clippy --all-targets --all-features -- -D warnings -W unsafe-code` | 0 | no warnings, no `unsafe_code` |
| `cargo build --all-targets --all-features` | 0 | builds under `#![forbid(unsafe_code)]` |
| `cargo test --all-targets --all-features` | 0 | **187 passed, 0 failed** (101 lib + 16 cli + 9 domain + 4 golden + 10 init + 9 persistence + 11 release + 14 rewire + 13 skill_contract); verified stable across 26 consecutive full-suite runs |
| `mine design validate --format json` | 0 | `{"valid":true,"warnings":[]}` |
| `mine graph validate --format json` | 0 | `{"plans":12,"warnings_emitted":false}` (revision 26) |
| live-graph md5 before/after full suite | — | byte-identical |

## Acceptance-criteria mapping

| Criterion | Evidence |
|---|---|
| branch `plan/09-1-*` created from `dev` `88affc0` before registration | `git merge-base dev HEAD == 88affc0`; `45ee446` is the first commit after the fork |
| `dev` remained at `88affc0` throughout | `git rev-parse dev == 88affc0`; no Plan 09-1 commit reachable from `dev` |
| all unreviewed lifecycle/implementation commits only on the ephemeral branch | `git rev-list dev` excludes all 7 Plan 09-1 commits |
| explicit Branch contract present | `grep -c '## Branch contract' == 2` in the plan doc |
| no merge/direct commit placed Plan 09-1 work on `dev` | dev reflog shows no Plan 09-1 entries; no merge commit |
| `master` untouched | `git rev-parse master == 1d3a132` throughout |
| break-glass: only revision 23→24 + Plan 09-1 DRAFT→READY + MD regen | verified by `git show 2a65fe9` (2 fields/file) |
| break-glass MD byte-identical to renderer | independent `mine graph render` matches committed MD |
| Plan 05-1 remained DRAFT; Plan 06 BLOCKED `["04","05"]` | full-node diff `2a65fe9~1` → `b25be2b` shows no change to those nodes |
| ported behavior accurate; rejected branch not merged/cherry-picked | byte-diffs vs `33496be`; `dev` history clean |
| dead duplicate `references` call removed | `src/domain/rewire.rs` line 94 deletion in `6ad9f66` |
| identifiers/reports refer to Plan 09-1 | doc-comment rename verified in test files |
| release semantics (DRAFT-only, READY/BLOCKED, +1 once, no edit API) | direct code reading + 16 unit tests + 11 integration tests |
| rewire semantics (derived replacement, exact replace, cycle, idempotent, +1 once) | direct code reading + 13 unit tests + 14 integration tests |
| concurrent tests against isolated temp repos, one winner, +1 revision, no overwrite | both tests; release test has one non-blocking robustness gap (documented above) |
| all write-path tests on isolated repos; live graph byte-unchanged | `seeded_repo` + `--repo`; md5 snapshots byte-identical |
| no arbitrary edit/state API; no structural `unsafe` | grep gates clean |

## Evidence-integrity finding (non-blocking) — `implementation_commits` misrecord

The graph TOML's recorded `implementation_commits` for Plan 09-1 (verified via `mine plan show --id 09-1 --format json`) is:

```
["6ad9f660...", "888b382...", "33496be...", "def594fe...", "ce126b6..."]
```

Independently checked against actual commit ancestry (`git merge-base --is-ancestor <hash> b25be2b`):

- `6ad9f66`, `def594f`, `ce126b6` — **on** the Plan 09-1 branch (correct).
- `888b382` (rejected Plan 09's `feat(cli): mine plan release and rewire-compensation`) and `33496be` (rejected Plan 09's `test(plan-09): release and rewire-compensation integration tests`) — **NOT ancestors** of the Plan 09-1 branch; they live only on the rejected `plan/09-plan-release-and-compensation-rewiring` branch.
- `45ee446` (the Plan 09-1 commit that actually contains the bulk of the ported domain+CLI+test source — 1818 insertions) is **on** this branch but **unlisted** in `implementation_commits`.

So two of the five recorded hashes are not reachable from the branch under review, and the actual port commit `45ee446` is omitted. This is an evidence-integrity slip in graph metadata made by the implementing agent's `mine plan implemented --commit …` invocation (it appears the agent recorded the rejected Plan 09 *source-of-port* hashes alongside its own follow-up commits, rather than `45ee446` the real port commit on this branch). The actual ported implementation IS present and verified on this branch (independently byte-diffed above); reviewability is preserved because the true commit `45ee446` is reachable and named in this report. The misrecord cannot be repaired through the accepted CLI (no edit-commits API exists; `plan implemented` is not idempotent once `IMPLEMENTED`). It is metadata-only and does not affect production, semantics, governance, or downstream correctness; documented here for the permanent record. Not blocking.

## Non-blocking follow-ups (recorded; not acceptance blockers)

1. **Harden `concurrent_release_is_resolved_by_revision_conflict`** to accept the second honest loser outcome (`MINE_INVALID_TRANSITION` when the loser reads the post-winner graph), mirroring the rewire test's `loser_conflicts OR loser_no_op` pattern. Latent rare flake (~1% observed); production invariants unaffected.
2. **Report commit-table accuracy**: the implementation report mislabels `45ee446`/`6ad9f66`/`def594f`'s contents. Cosmetic; commit messages themselves are accurate.
3. **`implementation_commits` graph metadata**: see the dedicated finding above. Two of the five recorded hashes are not ancestors of this branch and the real port commit `45ee446` is unlisted. Evidence slip, not a correctness/governance defect.

## Downstream release gate

Plan 06 (`BLOCKED`, hard predecessors `04, 05`) is not released by this acceptance until the reviewer performs the post-acceptance live rewiring of `06`'s hard predecessor `05 → 05-1` via `mine plan rewire-compensation --id 05`, then releases `05-1` to `READY` via `mine plan release --id 05-1`. Plan 05 itself remains `REJECTED`. This review performs those live steps next, per the review instructions, after merging into `dev`.

## Conclusion

Plan 09-1 is independently accepted. The principal review question — whether the previously-verified implementation was ported forward through a fully compliant ephemeral-branch lifecycle — is answered affirmatively by actual refs, reflogs, ancestry, and hashes. The Plan 09 branch-governance violation does not recur. The break-glass is exactly scoped and renderer-verified. The selective port is byte-faithful minus the flagged dead-code removal. All decisive gates pass; the one test-robustness gap is non-blocking, production-correct, and documented for a follow-up.
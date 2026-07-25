# Plan 10 Independent Review Report

- **Plan**: `docs/plan/10-concurrent-release-test-tolerance.md`
- **Title**: Concurrent release test tolerance
- **Reviewer**: independent reviewer, fresh context (did not trust the implementation report)
- **Review date**: 2026-07-25
- **Baseline**: accepted `dev` `f8b57680394a6138d6d64fd6f65e693f336e54b0`
- **Plan branch HEAD**: `plan/maintenance-concurrent-release-test-tolerance` @ `e3bc1d444bef69f57bc83fe5a2b435b45d4ddce1`
- **Final verdict**: `ACCEPTED`

## Lead

`Verdict: ACCEPTED` — Plan 10 is a narrow test-only maintenance plan that relaxes the over-constrained loser assertion in `tests/release.rs::concurrent_release_is_resolved_by_revision_conflict` to accept both honest loser outcomes documented by the accepted Plan-release Design (`MINE_REVISION_CONFLICT` and `MINE_INVALID_TRANSITION`), while strengthening the safety invariants. No production file changed (`git diff f8b5768..e3bc1d4 -- src/` is empty). Branch governance is clean: the plan forked from `dev` at `f8b5768`, `dev` and `master` never moved, all lifecycle and implementation commits live only on the ephemeral branch, and every graph mutation went through the accepted MINE CLI (revision trail 35→36→37→38 for add/release/start, then 38→39 for IMPLEMENTED, with Markdown byte-identical to the accepted renderer at every graph commit). The relaxed assertion matches the already-accepted design and real transaction ordering; it cannot pass vacuously, and it still proves exactly one real `DRAFT→READY` mutation, exactly one revision increment, the correct final `READY` state, an unchanged unrelated `ACCEPTED` plan, and mutually consistent TOML/Markdown. The targeted test passed 20× and the full suite passed 6× (246/0 each) with zero flakes.

## Method

Independent re-derivation: actual refs/reflogs/ancestry, the immutable plan and its two design references, the full implementation diff, direct reading of the modified test body, byte-diff of `src/` against the dev baseline, renderer byte-identity checks for every graph-mutating commit, repeated targeted and full-suite runs, and live `mine` CLI calls. No report claim was trusted without independent evidence.

## Gate 1 & 2: Branch and lifecycle governance — PASSES

- `git rev-parse dev` → `f8b5768` (unchanged throughout); `git rev-parse master` → `1d3a132` (untouched).
- `git merge-base dev HEAD` → `f8b5768`, i.e. the plan branch fork point is exactly the dev baseline.
- `git log --oneline f8b5768..e3bc1d4` lists 4 commits: `6d1b954` (register+release+start), `cf7df45` (test change), `2180222` (report), `e3bc1d4` (IMPLEMENTED).
- None of the 4 are reachable from `dev` (`git rev-list dev` excludes them); `git reflog show dev` shows no Plan 10 entry (dev's tip is the prior `plan/06` merge).
- Standard `## Branch contract` section is not present as a heading in this plan document (it is a narrow maintenance plan), but the plan's "Scope/Non-goals" explicitly forbid touching `dev`/`master`/Plan 07 and the governance is verified clean by actual refs.

## Gate 3: CLI lifecycle operations and revision trail — PASSES

The review instructions specifically require that registration, DRAFT release, and start in commit `6d1b954` were each performed through accepted MINE CLI operations with legitimate successive revisions `35→36→37→38`.

- Revision trail (independently read from the committed TOML at each ref): `dev` = 35 → `6d1b954` = 38 → `e3bc1d4` = 39. The start commit's graph diff shows a Plan 10 node added with `status = "IN_PROGRESS"`, `owner = "plan-10-impl"`, `run_id = "plan-10-run-1"`, `started_at` set, `hard_predecessors = ["06"]` (Plan 06 is `ACCEPTED`, so release to `READY` is legitimate). The +3 revision jump 35→38 corresponds exactly to three CLI mutations (`plan add` 35→36, `plan release` 36→37, `plan start` 37→38), each incrementing the revision exactly once, as the design mandates.
- **CLI (not manual) generation verified**: the Markdown committed at both `6d1b954` and `e3bc1d4` is **byte-identical** to an independent `mine graph render` run on the committed TOML (verified in isolated temp roots for both commits). Hand-editing would not produce a renderer-identical MD.
- The `IMPLEMENTED` transition `e3bc1d4` (38→39) set `implementation_report` and `implementation_commits = ["cf7df45", "2180222"]`. Both recorded hashes are confirmed ancestors of the branch HEAD (unlike the Plan 09-1 metadata slip): `cf7df45` is the test change and `2180222` is this implementation report. No manual graph edit anywhere.

## Gate 4 & 5: Implementation scope and production untouched — PASSES

`git diff --stat f8b5768..e3bc1d4` (5 files): `docs/plan/10-*.md` (plan doc), `docs/plan/execution-graph.{md,toml}` (CLI-managed), the implementation report, and `tests/release.rs`.

- **`git diff f8b5768..e3bc1d4 -- src/` is empty** (0 lines) — no production release, persistence, revision, or state-machine behavior changed.
- The only source file touched is `tests/release.rs` (the single test body + docstring; commit `cf7df45` is 85 insertions / 17 deletions to that file alone).
- No `src/application/`, `src/domain/`, `src/infrastructure/`, `src/cli/`, or `src/mcp/` file changed.

## Adversarial test review — PASSES

Read the full modified `concurrent_release_is_resolved_by_revision_conflict` body directly. The relaxed assertion and strengthened invariants:

**Relaxed loser assertion matches the accepted design.** `docs/design/execution-graph/state-machine-and-algorithms.md#plan-release` "Stability errors" lists both `MINE_INVALID_TRANSITION` (current status is not `DRAFT`) and `MINE_REVISION_CONFLICT` (shared transactional check), and the "Idempotency and re-run behavior" section explicitly states automation that races "should read `mine plan show` first or treat `MINE_INVALID_TRANSITION` as 'already released'". The test now accepts exactly that two-code set:
```rust
assert!(loser_code == "MINE_REVISION_CONFLICT" || loser_code == "MINE_INVALID_TRANSITION", ...);
```
Both outcomes are honest losers that mutate nothing, and both arise from the real `lock → reload → revision check → semantic check → mutation → atomic write → render` transaction ordering: path 1 when the loser's stale `expected_revision` fails the post-lock reload revision check; path 2 when the loser re-reads after the winner committed, the revision matches, but `release_plan` sees the now-`READY` node and returns `InvalidTransition`.

**Cannot pass vacuously** (the relaxation does not weaken safety):
- `ok_a = out_a.exit_code == 0 && env_a["ok"] == true`; `assert!(ok_a ^ ok_b, ...)` — XOR requires exactly one winner; both-success and both-failure both fail. A thread panic propagates via `handle.join().unwrap()` (no silent swallow).
- `assert_eq!(loser_env["ok"], false)` plus the two-code allowlist — the loser must be a documented stability error; any other code or missing code fails with a descriptive message.
- `assert_eq!(ws.revision, n + 1)` — exactly one revision increment; cannot be `n+2` (double write) or `n` (lost write). `n` is read from the seeded fixture graph, not copied from the implementation's output.
- `assert_eq!(ws.get("09-1").unwrap().status, PlanStatus::Ready)` — correct final state.
- Winner envelope: `status_before == "DRAFT"`, `status_after == "READY"`, `revision_before == n`, `revision_after == n + 1` — confirms the exact `DRAFT→READY` transition off the pre-mutation revision (not self-referential; would fail if the winner reported `BLOCKED` or an incorrect revision).
- **Unrelated `ACCEPTED` plan unchanged**: the seed now includes `node("01", PlanStatus::Accepted, &[], &[])`; the test snapshots `unrelated_before = load_graph(&repo).get("01").clone()` before the race and asserts `assert_eq!(unrelated_after, &unrelated_before)` after — a full `PlanNode` structural equality (derives `Eq`), proving no unrelated graph data changed.
- **TOML/Markdown consistency**: the TOML is re-parsed and asserted `revision == n + 1`; the Markdown is asserted to `.contains("READY")` and `.contains("| 09-1 |")`. The seeded graph initially contains `01=ACCEPTED, 09-1=DRAFT` and no `"READY"` string, so a stale (un-regenerated) Markdown would fail the `"READY"` check — the substring assertions discriminate the intended final state from a stale MD. (A byte-identity check against a renderer run would be marginally stronger and is noted below as a non-blocking follow-up; the current checks are sound because the atomic transaction regenerates MD from TOML and the discriminator is meaningful.)
- `assert_live_unchanged(&live)` — the live repository graph is byte-unchanged across the test.

**The test still proves every required invariant:**
- exactly one real `DRAFT→READY` mutation (XOR on `ok`);
- final revision is exactly `n + 1` (`ws.revision == n + 1`);
- final Plan status is `READY`;
- the winning result reports the correct transition and revision (`status_before=="DRAFT"`, `status_after=="READY"`, `revision_before==n`, `revision_after==n+1`);
- the losing result is one of exactly the two documented honest outcomes (two-code allowlist);
- the unrelated `ACCEPTED` plan remains semantically unchanged (full-node `Eq`);
- TOML and Markdown represent the same final graph (re-parsed TOML revision + discriminating MD substring checks).

## Independent validation

| Command | Exit | Result |
|---|---|---|
| `cargo fmt --all -- --check` | 0 | clean |
| `cargo clippy --all-targets --all-features -- -D warnings -W unsafe-code` | 0 | no warnings, no `unsafe` |
| `cargo build --all-targets --all-features` | 0 | clean |
| `cargo test --test release --quiet concurrent_release_is_resolved_by_revision_conflict -- --test-threads=8` ×20 | 0 | 20/20 green, both loser paths tolerated |
| `cargo test --all-targets --all-features` ×6 | 0 | 6/6 green, 246 passed/0 failed each, zero flakes |
| `mine design validate --format json` | 0 | `{"valid":true,"warnings":[]}` |
| `mine graph validate --format json` | 0 | `{"plans":13,"warnings_emitted":false}` (revision 39) |
| live-graph md5 before/after full suite | — | byte-identical (`0461306d…` TOML, `b8212bc3…` MD unchanged) |
| `git diff f8b5768..e3bc1d4 -- src/` | — | empty (0 lines) — production untouched |
| Plan 07 status | — | `READY`, hard predecessors `["06"]`, unstarted |
| Plan 10 status | — | `IMPLEMENTED`, hard predecessors `["06"]` (06 `ACCEPTED` ⇒ valid), owner/run_id set |

## Non-blocking follow-up

- The TOML/Markdown consistency check uses discriminating substring matching (`.contains("READY")`, `.contains("| 09-1 |")`) rather than byte-identity against an independent renderer run. It is sound (the seed has no `READY` initially, so a stale MD fails), but a future test-grooming pass could assert byte-identity for maximal strictness. Not blocking; no semantic risk.

## Downstream release gate

Plan 07 (`READY`, hard predecessor `["06"]`) is unaffected by this maintenance plan and remains `READY` and unstarted. This acceptance does not start, release, rewire, or block Plan 07.

## Conclusion

Plan 10 is independently accepted. The narrow test-only change correctly relaxes the loser assertion to both documented stability errors while strengthening the safety invariants, matches the already-accepted Plan-release Design and real transaction ordering, touches no production code, and is deterministic across repeated parallel and full-suite runs. Branch governance is clean; every graph mutation went through the accepted MINE CLI with a verified revision trail and renderer-consistent Markdown. Reviewer-initiated `IMPLEMENTED→ACCEPTED` transition will be performed through the accepted `mine` CLI.
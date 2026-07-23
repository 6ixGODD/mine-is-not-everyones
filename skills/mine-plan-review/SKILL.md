---
name: mine-plan-review
description: Strictly review one implemented repository plan against AGENTS.md, architecture, the immutable plan, implementation commits, real runtime behavior, verification evidence, and downstream contracts. Use when the user invokes the host-specific mine-plan-review skill with a plan path, asks whether a plan can be accepted, requests an acceptance review, or wants review failures classified into direct small fixes versus a new compensating plan. Work in the current workspace; never trust an implementation report or green tests without independent evidence.
---

# MINE Plan Review

MINE Is Not Everyone's. Act as an independent acceptance reviewer, not as the implementing agent's advocate. Try to falsify the implementation's claims. Accept only
when the plan's actual contract is demonstrated end to end.

## Resolve the review target

Treat the invocation argument as exactly one immutable plan path:

```text
mine-plan-review docs/plan/02-card-and-observation-encoding.md
mine-plan-review docs/plan/02-card-and-observation-encoding.md
```

Strip only the invocation-level `@`, resolve from repository root, and require the file to exist. Do not silently review another plan. If no
path is supplied, ask for it before mutation.

## Read the governing evidence

Read completely, in order:

1. Root `AGENTS.md`.
2. The architecture source named by `AGENTS.md`.
3. Query the target node and graph revision through `mine_plan_get` / `mine_graph_status` or JSON CLI; use the generated Markdown only as a readable view.
4. The target plan.
5. Every hard-predecessor acceptance report and referenced commit.
6. The target implementation report, implementation commits, diff from its accepted baseline, and suggested downstream consumer plan.
7. Fetch and read the official sources and best-practice references registered by the plan; do not rely on the plan's paraphrase alone.
8. Relevant production code, schemas, tests, CLI/configuration and generated-artifact contracts.

Govern review in this order: `AGENTS.md` → architecture → registered authoritative sources → immutable plan. Treat implementation comments,
test names and the implementation report as claims to verify, not authority.

## Establish a clean review subject

Review the committed implementation named by its report. Inspect current branch, worktree list, `git status`, staged state and concurrent
changes before running commands.

- Work in the current workspace; do not create a worktree or branch.
- Never stash, reset, clean, checkout, restore or discard user/agent changes.
- Separate target-plan changes from unrelated concurrent changes using commit diffs and blobs.
- Do not attribute an unrelated dirty-worktree failure to the reviewed plan.
- If concurrent changes make a command unreliable, run the closest valid check against the managed environment or committed artifact and
  document the deviation. Do not call it a pass if equivalence is unproven.

## Build a traceability matrix

For every goal, implementation step, deliverable and acceptance checkbox, record:

- governing architecture section;
- implementation file/interface and commit;
- independent test or runtime probe;
- observed result;
- status: `PASS`, `FAIL`, `UNVERIFIED`, or `NOT_APPLICABLE` with reason.

One failed hard contract is sufficient to reject. Do not average severe defects against many passing cosmetic checks.

## Review adversarially

Use this evidence priority:

1. Real boundary behavior and independently inspected artifacts/data.
2. Production code and serialized/runtime round-trip behavior.
3. Independent tests that would fail for a plausible wrong implementation.
4. Existing tests.
5. Implementation report prose.

At minimum:

- Re-run the plan's exact commands when the environment permits.
- Inspect error/fallback paths, not only happy paths.
- Check empty, malformed, unknown, boundary-size, timeout, corruption and partial-failure cases applicable to the plan.
- Verify persisted data through write → load → consumer round-trips, including identity, ordering, hashes, dtype and shape.
- Compare real source counts/content when the plan makes data-specific claims.
- Check privacy/information boundaries by changing hidden inputs and observing all outputs.
- Check both symmetric roles/seats for game or multi-party logic.
- Run the focused tests, integration/smoke checks, formatter, linter, static analysis, type checks, build checks, and other repository gates required by `AGENTS.md`, the architecture, and the plan.
- Inspect whether downstream code can consume the produced interface without decoding guessed semantics or adding a compatibility shim.
- Confirm reports state exact commands/results and do not mark skips, timeouts, missing tools or warnings as passes.

Do not accept because “tests pass.” Determine whether the tests assert the intended semantics rather than restating the implementation.
Write a small independent probe when a key acceptance claim lacks discriminating evidence.

## Classify findings

Classify by impact, not line count.

### Fix directly during review

Apply a direct reviewer fix only when all are true:

- architecture and plan already specify the correct behavior unambiguously;
- the defect is local and has no effect on persisted schema, tensor/model I/O, public API, security/privacy boundary, reward/label semantics,
  process lifecycle, dependency graph or downstream design;
- no product/design decision is required;
- the fix and regression test are cohesive and fully verifiable in the current session;
- fixing it does not hide a false implementation claim or require rewriting an immutable plan.

Examples: typo, wrong import, missing local guard, incorrect error message, small off-by-one with an existing clear contract, missing focused test
for behavior already correctly implemented elsewhere.

Patch it, add a regression test, run all affected gates, commit it separately as `fix: ...`, then re-run the complete acceptance matrix. Record
the reviewer-authored change in the review report.

### Require a compensating plan

Reject and create a compensating plan when any finding changes or repairs:

- methodology or training target;
- persistent data/catalog/checkpoint schema, provenance or migration/rebuild behavior;
- model/tensor input-output contract or feature semantics;
- privacy, authorization, secret or hidden-information boundary;
- cross-component ownership/lifecycle, concurrency or process cleanup;
- external runtime/package/submission behavior;
- multiple coupled modules or a downstream dependency;
- missing real integration evidence central to the plan;
- a report that claims a hard acceptance criterion without evidence.

When uncertain, prefer a compensating plan. A short diff can still be architecturally large.

## Accept a correct implementation

Accept only when every hard acceptance item is `PASS`, required verification is reproducible, artifacts/reports are accurate, no unresolved
finding can invalidate downstream work, and all implementation/report commits exist on the current branch.

Then:

1. Create a separate review report under `docs/plan/reports/<plan-name>-review.md` rather than overwriting implementation evidence.
2. Record inspected commits, traceability matrix, independent commands/results, reviewer fixes and remaining non-blocking risks.
3. Call `mine_plan_accept` or the final `mine plan accept --format json` command with the review report and expected revision; MINE releases eligible downstream nodes.
4. Stage explicit review files only and commit with `docs: accept Plan NN ...`.
5. Verify the commit and state that no worktree merge is required.

## Reject and compensate

For a material failure:

1. Do not edit the immutable target plan or rewrite its implementation report/history.
2. Create `docs/plan/reports/<plan-name>-review.md` with `REJECTED`, exact findings, commands, evidence and affected downstream nodes.
3. Call `mine_plan_reject` or the final JSON CLI equivalent with the review report, rejection reason, expected revision, and compensating-plan relationship; MINE keeps downstream nodes blocked.
4. Update the architecture source first with corrected target behavior and verified failure evidence.
5. Create the repository's next compensating plan number/name (for example `02-1-...`) that identifies the rejected implementation and
   changes target code directly—no alias, shim or migration solely to preserve the rejected behavior.
6. Give the compensation node hard predecessors, deliverables, concrete steps, verification, acceptance criteria and graph edges. Route
   downstream release through the compensating plan.
7. Do not implement the compensating plan during review unless the user separately asks for implementation.
8. Stage only architecture, review, and compensation-plan files, inspect the cached diff, and commit with a Conventional Commit. Never stage hand-edited graph files; MINE owns graph writes.

If material product decisions remain unresolved, write the rejection report and ask the user before changing architecture/creating the plan.
Do not invent the decision merely to finish review.

## Review report requirements

Every review report must include:

- target plan, baseline, implementation/report commits and current branch;
- review scope and any concurrent-worktree caveat;
- findings ordered by severity with file/interface evidence;
- acceptance traceability matrix;
- exact commands, exit codes and concise outputs;
- direct reviewer fixes, if any;
- passed, failed, skipped, timed-out and unavailable checks;
- security/data handling statement;
- final `ACCEPTED` or `REJECTED` decision;
- downstream nodes released or kept blocked;
- remaining risks and required user/external actions.

Never claim independent acceptance when the same unresolved defects remain, and never soften `FAIL` into “remaining risk” to release a
downstream plan.

## Finish the handoff

Lead with `ACCEPTED` or `REJECTED`. Summarize decisive evidence, reviewer fixes, report/plan paths, commit hash, graph effect, verification
results, unrelated changes preserved and next action. Say explicitly whether downstream work is released.

---
name: mine-plan-review
description: Strictly review one implemented repository plan against AGENTS.md, architecture, the immutable plan, implementation commits, real runtime behavior, verification evidence, and downstream contracts. Use when the user invokes the host-specific mine-plan-review skill with a plan path, asks whether a plan can be accepted, requests an acceptance review, or wants review failures classified into direct small fixes versus a new compensating plan. Work in the current workspace; never trust an implementation report or green tests without independent evidence.
---

# MINE Plan Review

MINE Is Not Everyone's. Act as an independent acceptance reviewer, not as the implementing agent's advocate. Try to falsify the implementation's claims. Accept only
when the plan's actual contract is demonstrated end to end.

## Integration: MCP tools and CLI fallback

`mine-plan-review` queries and transitions plan state through two paths, in
this order of preference:

1. **MCP tools (preferred)** - when the current Agent runtime exposes the
   MINE MCP server (`mine mcp serve`), call the typed MCP tools. They return
   the same DTOs as the JSON CLI and never touch the execution-graph files.
2. **JSON CLI (deterministic fallback)** - when MCP is unavailable, call
   `mine --format json` commands. Never parse human output.

Never invent an MCP tool, CLI command, flag, JSON field, or lifecycle
transition that the current binary does not expose. Never edit
`docs/plan/execution-graph.toml` or `docs/plan/execution-graph.md` directly.

The accepted MCP tools `mine-plan-review` may use:

- `mine_graph_status` (no arguments) - read the current revision.
- `mine_plan_show` (`id`) - read the target node, its predecessors, and status.
- `mine_plan_accept` (`id`, `review`) - transition `IMPLEMENTED` -> `ACCEPTED`
  and record the review report path.
- `mine_plan_reject` (`id`, `reason`, `compensating_plan`) - transition
  `IMPLEMENTED` -> `REJECTED` and register the compensating plan id.
- `mine_graph_validate` (no arguments) - validate the graph after transitions.
- `mine_design_validate` (no arguments) - confirm design references.

Operations `mine-plan-review` needs that are intentionally **CLI-only** (no
MCP tool exposes them, because they rewire downstream dependencies outside
the single-node accept/reject path):

- `mine plan rewire-compensation --id <rejected-plan-id> --format json` -
  reroute downstream dependencies from a rejected plan onto its registered
  compensating plan. There is **no MCP tool for rewiring**; after
  `mine_plan_reject`, rewiring is a mandatory CLI fallback when downstream
  nodes must be rerouted.
- `mine plan release --id <id> --format json` - release a compensating plan
  after it is registered (CLI only).

When a required operation has no MCP tool, fall back to the JSON CLI and state
the fallback explicitly.

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
2. The design knowledge base rooted at `docs/design/index.md` (and the relevant leaves named by `AGENTS.md`).
3. Query the target node and graph revision through `mine_plan_show` (MCP) or `mine plan show --id <id> --format json` (CLI fallback), and `mine_graph_status` (MCP) or `mine graph status --format json` (CLI fallback); use the generated Markdown only as a readable view.

Every reviewer-initiated transition of graph state must go through the accepted MINE CLI (`mine plan accept` / `mine plan reject` with `--format json`); never edit `docs/plan/execution-graph.toml` or `docs/plan/execution-graph.md` directly (`AGENTS.md` documents this rule; the bootstrap exception has ended).
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

### Remove temporary-plan references before release closure

Before the final stable-candidate integration, run the scanner bundled with this Skill from the root of the repository under review:

```text
bash references/scan-plan-refs.sh --check
```

The scanner lives next to this `SKILL.md` under `references/scan-plan-refs.sh` and is copied into every installer-managed Skill directory (Claude Code, Codex, Pi, OpenCode). It must never be invoked as `scripts/scan-plan-refs.sh` — that path does not exist in the repository being reviewed and would be a dangling reference. Run it with an absolute path if the agent cannot resolve the relative one, for example `bash ~/.claude/skills/mine-plan-review/references/scan-plan-refs.sh --check`.

This scans tracked implementation, test, workflow, Skill, and distribution assets while excluding temporary `docs/plan/` and design documentation. It rejects stale `Plan NN` references because stable behavior must be intelligible without the ephemeral planning history. Rewrite a historical comment as an enduring contract; for example:

```rust
// Bad: A stale plan-number comment attributing behavior to a historical plan.
// Good: Checksums prove artifact integrity only; binary reproducibility is not claimed.
```

An intentional fixture literal may be exempted only by an immediately preceding line with a concise reason:

```rust
// mine-release-allow-plan-reference: protocol fixture
let input = "Plan 08-2";
```

Never use exemptions for implementation comments, workflow behavior, public diagnostics, or prose that can be expressed as a durable contract. Review every exemption, record its path/line/reason in the closure report, and rerun the scan after every correction. The scanner is a release-closure gate, not a substitute for semantic review.

Do not accept because “tests pass.” Determine whether the tests assert the intended semantics rather than restating the implementation.
Write a small independent probe when a key acceptance claim lacks discriminating evidence.

## Classify findings

Classify by impact, not line count. **A reviewer is responsible for bringing submitted work to an acceptable, mergeable state, not merely for issuing an immediate binary verdict.** Independence means independently inspecting and validating the implementation end to end — it does not mean refusing to correct a defect once it is found. Do not spawn a compensating plan, and do not consume another full implementation/review cycle, merely to preserve reviewer/implementer role purity for a narrow, well-understood correction.

### Fix directly during review (the normal path for narrow findings)

Apply a direct reviewer fix whenever all are true:

- architecture and plan already specify the correct behavior unambiguously (or the fix is a release/workflow/manifest/documentation/test correction that does not change any product or design decision);
- the defect is local and has no effect on persisted schema, a public API/tool contract, a security/privacy boundary, cross-component ownership/lifecycle, or a downstream design decision;
- the fix and its regression coverage (a new/strengthened test, a corrected workflow step, a corrected manifest/doc line) are cohesive and fully verifiable in the current review session;
- fixing it does not hide a false implementation claim, rewrite an immutable plan, or silently weaken a gate the plan was supposed to satisfy.

This explicitly includes, without needing a second reviewer or a compensating plan: typos, wrong imports, missing local guards, incorrect error messages, small off-by-ones against an existing clear contract, missing focused tests for behavior already correctly implemented elsewhere, incorrect or ineffective CI/CD workflow steps (for example a masked failure condition, a missing platform in a required matrix, an incorrect path/flag), stale or contradictory documentation/Skill/template wording, a narrow release-blocking defect discovered during release-closure validation (for example a diagnostic command that discards data it should preserve, an ineffective detection gate, a version-resolution edge case), and coherent updates to generated distribution copies performed only through the accepted synchronization mechanism (never by hand-editing a generated copy directly).

Patch it, add or strengthen the regression test, run every affected gate (not just the one you touched), commit the correction **separately** from the plan's own commits (a clearly labeled `fix:`/`docs:`/`chore:` commit authored by the reviewer), then re-run the complete acceptance matrix against the corrected HEAD. Record the exact reviewer-authored change, its rationale, and its revalidation evidence in the review report. Never conceal a reviewer-authored change, fold it silently into the implementer's own commits, or accept without rerunning every gate the change could affect.

### Require a compensating plan (reserved for substantial issues)

Reject and create a compensating plan only when a finding is genuinely substantial — not merely inconvenient to fix inline. Reserve this path for findings that:

- require a material Design change or replace the plan's core approach/methodology;
- change a persistent data/catalog/checkpoint schema, provenance, or migration/rebuild behavior;
- change a public API/tool input-output contract or a security/privacy/secret/hidden-information boundary;
- introduce or require a substantial independent work package, a major scope expansion, or coordinated changes across multiple coupled modules and a downstream dependency;
- reveal missing real integration evidence central to the plan's own claim, such that no bounded, same-session correction could responsibly close the gap;
- cannot be safely and fully verified within the current review session (for example, the fix itself would need its own design decision, its own dependency-aware work-package sequencing, or realistically exceeds what one reviewer can independently validate in one pass).

When genuinely uncertain whether a finding is narrow or substantial, weigh it against the criteria above rather than defaulting to rejection: a change that is small in line count can still be architecturally substantial (create a compensating plan), and a change that touches many files can still be narrow and mechanical (fix it directly) — for example, a corrected CI workflow, a batch of strengthened Skill-contract tests, or a synchronized set of generated distribution copies produced by the one accepted sync mechanism are narrow even when they touch several files, because they carry no new design decision and are each independently, fully verifiable in the same session.

## Accept a correct implementation

Accept only when every hard acceptance item is `PASS` (after any direct reviewer fixes are applied and revalidated), required verification is reproducible, artifacts/reports are accurate, no unresolved finding can invalidate downstream work, and all implementation/report/reviewer-fix commits exist on the current branch.

Then:

1. Create a separate review report under `docs/plan/reports/<plan-name>-review.md` rather than overwriting implementation evidence. Include any reviewer-authored fixes: what changed, why, and its revalidation evidence.
2. Record inspected commits, traceability matrix, independent commands/results, reviewer fixes and remaining non-blocking risks.
3. Call `mine plan accept --id <id> --review <review report path> --format json` (MCP: `mine_plan_accept` with `id` and `review`); the accepted MINE CLI and MCP tool read the current revision under the lock, transitions `IMPLEMENTED`→`ACCEPTED`, records the review report, and (when combined with the released status) releases eligible downstream `BLOCKED`→`READY` successors whose hard predecessors are all now accepted. The envelope reports `revision_before`/`revision_after`.
4. Stage explicit review and reviewer-fix files only and commit with `docs: accept Plan NN ...` (reviewer-fix commits use their own `fix:`/`chore:` subject, kept separate from this bookkeeping commit).
5. Verify the commit and state that no worktree merge is required.

## Bring release closure to completion

When the accepted plan graph reaches (or, after this acceptance, will reach) an all-terminal state (`docs/design/governance/branch-and-plan-lifecycle.md` → "Release closure"; every plan `ACCEPTED` or `REJECTED` with an accepted compensation chain), the reviewer who performs the final acceptance is also responsible for carrying the release to local closure in the same session, not for spawning another plan merely to perform the remaining mechanical steps:

1. Merge the just-accepted branch into `dev` with `--no-ff` and re-run the complete decisive validation suite (`cargo fmt`, `cargo clippy -D warnings -W unsafe-code`, `cargo build --all-targets --all-features`, `cargo test --all-targets --all-features` at least twice with a fresh `CARGO_TARGET_DIR`, `python scripts/sync-plugin-assets.py --check`, `python scripts/verify.py`, `mine design validate --format json`, `mine graph validate --format json`) directly on `dev`.
2. Call `mine release --format json` (CLI-only; no MCP tool exposes release preflight) as a diagnostic before candidate construction. Its development-tree gates are decisive immediately: terminal plan state, accepted compensation for every rejected plan, valid graph/render, valid design, synchronized distribution assets, no dirty tree, no pending Agent transaction, and the authoritative resolved version from `.mine/config.toml`. Before stable integration, `can_release:false` is expected solely when the existing stable branch still contains its old `docs/plan/` workspace; do not misreport that pre-integration stable-tree fact as a failed `dev` validation. After curated stable integration, run the preflight again from `dev` and require every gate, including no `docs/plan/` or `docs/design-backup-*` on the stable branch, to pass. `mine release` is validation-only; it must never itself claim that `master`, tags, publication, or cleanup occurred.
3. If `mine release` (or any other decisive check) fails on a narrow, release-scoped defect — not a new design decision — fix it directly in this same session exactly as in "Fix directly during review" above (own commit, own regression coverage, full revalidation), rather than opening a compensating plan solely to perform the closure.
4. Run the bundled `references/scan-plan-refs.sh --check` from the root of the repository under review (never `scripts/scan-plan-refs.sh` — that path does not exist in the target repository), correct every unexempted temporary-plan reference, and record every line-local fixture exemption before candidate construction.
5. In an isolated clone or Git worktree (never the reviewer's own live checkout), construct the exact stable candidate tree per `docs/design/governance/branch-and-plan-lifecycle.md` and `docs/design/integrations/distribution.md`: the accepted `dev` tree with the ephemeral `docs/plan/` workspace removed and no tracked `docs/design-backup-*` path. Build, test, run `mine design validate` and distribution verification against the candidate; do **not** run `mine graph validate` there because the graph-less stable tree intentionally has no `docs/plan/` workspace. Graph/render validation is instead a decisive gate on `dev` before candidate construction. Install all four Agents (`claude-code`, `codex`, `pi`, `opencode`) into an explicit isolated `--config-root`, confirm `mine doctor --agents all` reports every Agent healthy on this graph-less stable candidate (a positively-identified stable branch legitimately has no `docs/plan/`; this must not be confused with an unhealthy development repository), and confirm `mine mcp serve` exposes exactly the twelve accepted MCP tools.
6. Only after every candidate check passes, perform the Design-authorized local stable-branch integration (squash or curated commit so temporary plan history is not imported into the stable branch — never a plain merge of `dev`), determine and record the resolved release version from `.mine/config.toml`, and perform only the MINE-owned local cleanup the Design authorizes: delete local `plan/*` branches that are merged/accepted (never a branch with unexpected ancestry, an unmerged branch, or a checked-out worktree), and delete the local `dev` branch only after stable integration succeeds.
7. Never push, create a remote release, publish a package, force-update history, or delete a remote or unrelated/user branch. Report the exact final local stable-branch commit, the resolved version, the final stable-tree contents, and every local branch or artifact removed.

## Reject and compensate

For a material failure:

1. Do not edit the immutable target plan or rewrite its implementation report/history.
2. Create `docs/plan/reports/<plan-name>-review.md` with `REJECTED`, exact findings, commands, evidence and affected downstream nodes.
3. Call `mine plan reject --id <id> --reason <rejection summary> --compensating-plan <comp-id> --format json` (MCP: `mine_plan_reject` with `id`, `reason`, `compensating_plan`); the accepted MINE CLI and MCP tool transition `IMPLEMENTED`→`REJECTED`, records the `rejection_reason` and `compensating_plan`, reads the current revision under the lock, and emits `revision_before`/`revision_after`. Downstream nodes stay blocked; rerouting of the downstream edge to the compensation node is done when the compensation plan is registered (`mine plan add --hard <comp-id>`, CLI only) and then rewired (`mine plan rewire-compensation --id <rejected-id> --format json`, CLI only - no MCP tool).
4. Update the `docs/design/` knowledge base first with corrected target behavior and verified failure evidence.
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

Lead with `ACCEPTED` or `REJECTED`. Summarize decisive evidence, reviewer-authored fixes and their revalidation, report/plan paths, commit
hash(es), graph effect, verification results, unrelated changes preserved, whether local release closure was carried out in this session, and
next action. Say explicitly whether downstream work is released.

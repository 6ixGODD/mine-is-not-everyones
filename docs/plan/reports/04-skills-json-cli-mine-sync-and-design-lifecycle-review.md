# Plan 04 Independent Review Report

- **Plan**: `docs/plan/04-skills-json-cli-mine-sync-and-design-lifecycle.md`
- **Title**: Skills JSON-CLI integration, mine-sync, and design lifecycle
- **Reviewer**: independent acceptance reviewer (fresh; did not rely on the implementation report's conclusions)
- **Review date**: 2026-07-24
- **Baseline**: clean `dev` `5e103b5c95547baefe8093161b47aa590d52a56d`
- **Plan branch HEAD**: `plan/04-skills-json-cli-mine-sync-and-design-lifecycle` @ `134c0db`
- **Final verdict**: `ACCEPTED`

## Lead

`Verdict: ACCEPTED` — Plan 04 migrated all five root Skills and the user guide to the accepted JSON CLI contract and the progressive `docs/design/index.md` knowledge base, never edits the execution graph directly, keeps `mine-arch` requirement-first, keeps `mine-sync` code-authoritative with mandatory pre-mutation backup, and adds 13 contract tests that fail on Skill drift. All required gates are green against the built binary and the real graph (revision 12). No `mine` Rust production code, MCP (Plan 05) scope, or out-of-scope file was modified. Two non-blocking cleanup notes are recorded below; neither violates an acceptance criterion.

## Evidence reviewed (independent re-derivation)

1. The plan document and its six governing design references: `docs/design/integrations/skills.md`, `docs/design/governance/design-knowledge-base.md`, `docs/design/governance/design-sync.md`, `docs/design/interfaces/cli-contract.md`, ADR-0005, ADR-0006.
2. The implementation report (`docs/plan/reports/04-...-implementation.md`) — read but not trusted; its claims were re-derived.
3. `git diff --stat 5e103b5..134c0db` and full diff of `docs/plan/execution-graph.{toml,md}`.
4. The five root Skill files: `skills/mine-{arch,sync,plan-create,plan-exec,plan-review}/SKILL.md`, plus `skills/mine-arch/references/AGENTS.template.md` and `skills/mine-plan-create/references/plan-template.md`.
5. `docs/user-guide.md`.
6. `tests/skill_contract.rs` (13 tests).
7. Accepted CLI help/JSON for every command referenced by the Skills, plus `src/cli/commands.rs` flag parsing for exact contract verification.

## Scope and immutability

`git diff --stat 5e103b5..134c0db` (12 files):

```
docs/plan/execution-graph.{md,toml}          CLI-managed (revision 10→12; start + implemented)
docs/plan/reports/04-...-implementation.md  implementation report (in-scope)
docs/user-guide.md                            exclusive write path
skills/mine-*/SKILL.md + 2 references         exclusive write path
tests/skill_contract.rs                       exclusive write path
```

- No `src/`, `Cargo.toml`, `Cargo.lock`, or any `mine` Rust crate file was modified. Confirmed: `git diff --name-only 5e103b5..134c0db | grep -E '^(src|Cargo)'` returns nothing.
- No Plan 05 (stdio MCP server / typed tools / `src/output` MCP plumbing / `mine mcp serve`) scope change.
- `master` untouched; `dev` not merged by the implementer; nothing pushed (no remotes).

## Execution-graph discipline

The graph TOML/MD diff is exactly what the accepted CLI produces for `READY→IN_PROGRESS` (start) then `IN_PROGRESS→IMPLEMENTED`:

- `revision = 10 → 12`;
- node `04`: `status = READY → IMPLEMENTED`; `owner = "plan-04"`; `run_id = "plan-04-skills"`; `started_at` / `updated_at` set; `implementation_commits` populated;
- no field outside the `plan.start` / `plan.implemented` write set changed.

This is consistent with CLI invocation, not manual editing. No hand-edit of `execution-graph.toml` or `execution-graph.md` occurred. `mine graph validate --format json` reports `{"plans":9,"warnings_emitted":false}` at revision 12.

## CLI command contract verification (re-derived from `src/cli/commands.rs` + live calls)

Every `mine …` invocation shown in the Skills was verified against the accepted binary and source:

| Skill citation | Verified accepted contract | Match |
|---|---|---|
| `mine init --format json` | accepted; no subcommand flags required | ✓ |
| `mine plan add --id --path --title --design-ref [--write] [--hard] --format json` | `plan_add` reads `--id/--path/--title` (required), `--design-ref` (repeatable, ≥1, non-empty), `--write`, `--hard` (repeatable); revision read internally | ✓ |
| `mine plan show --id <id> --format json` | `plan_show` requires `--id`; envelope `data.plan` returns `status`, `owner`, `run_id`, `implementation_commits`, etc. | ✓ |
| `mine plan start --id --owner --run-id --format json` | `plan_start` requires `--id`; `--owner` default `default`, `--run-id` default `default-run`; no `--expected-revision` flag accepted; revision read under lock; emits `revision_before/after` | ✓ |
| `mine plan implemented --id --report --commit --format json` | `plan_implemented` requires `--id`, `--report`, `--commit` (repeatable, ≥1) | ✓ |
| `mine plan accept --id --review --format json` | `plan_accept` requires `--id`, `--review` (non-empty); releases `BLOCKED→READY` successors whose hard preds are all accepted | ✓ |
| `mine plan reject --id --reason --compensating-plan --format json` | `plan_reject` requires `--id`, `--reason`, `--compensating-plan` (non-empty) | ✓ |
| `mine graph status --format json` | envelope `data.revision` present (=12) | ✓ |
| `mine graph validate --format json` | accepted, returns `plans` count and `warnings_emitted` | ✓ |
| `mine design backup --format json` | envelope `data` = `{backup_path, file_count, total_bytes}` — matches the `mine-sync` claim | ✓ |
| `mine design validate --format json` | envelope `data.valid` + `data.warnings` | ✓ |
| `mine design status --format json` | accepted subcommand | ✓ |

Live JSON re-run on the real repository:

- `mine design validate --format json` → `{"ok":true,"data":{"valid":true,"warnings":[]}}`, exit 0.
- `mine graph validate --format json` → `{"ok":true,"data":{"plans":9,"warnings_emitted":false}}`, exit 0.
- `mine graph status --format json` → `data.revision=12`, `ready=["05"]`.
- `mine plan show --id 04 --format json` → status `IMPLEMENTED`, owner `plan-04`, run_id `plan-04-skills`, three implementation commits recorded.

No Skill instructs `--expected-revision`, `--agents`, `--run`, a user-supplied release/cycle version, or any flag the accepted CLI does not accept. The CLI does not implement per-subcommand `--help` (it prints top-level usage and exits 2 for `--help`); this was re-derived from `src/cli/commands.rs` rather than `--help`.

## Stale-name and obsolete-path searches (repository-scoped)

`rg --glob '!target/**' --glob '!.git/**'`:

- `mine-design-sync`: **not present** in `skills/`, `tests/skill_contract.rs`, or `docs/user-guide.md`. Appears only in the pre-existing orphan draft `docs/plan/04-skills-json-cli-and-design-lifecycle.md` and `docs/plan/08-...md` — both predate the Plan 04 branch (`git cat-file -e 5e103b5:<orphan>` succeeds) and were **not** touched by this branch. Orphan plan drafts are not in Plan 04's write scope; they are a pre-existing repository condition for a future cleanup plan.
- snake_case MCP tools (`mine_plan_*`, `mine_graph_*`, `mine_design_*`, etc.): **not present** in root `skills/` or `docs/user-guide.md`. Appears only in `REQUIREMENTS.md` (read-only context), `tests/skill_contract.rs` (as the negative assertion), and `plugins/mine/skills/` (generated copies owned by Plan 06 plugin-distribution scope, predating this branch, untouched by it), and `docs/design/interfaces/mcp-contract.md` (Plan 05 scope, defining the eventual MCP tool names — not fictional).
- `mine doctor --agents`: **not present** in root `skills/` or `docs/user-guide.md`. Appears only in `REQUIREMENTS.md` (read-only), `tests/skill_contract.rs` (negative assertion), and the orphan draft `docs/plan/08-...md` (pre-existing).
- `architecture-and-detailed-design.md`: in root `skills/` only as negative guidance in `skills/mine-arch/SKILL.md` ("do not … introduce a competing `architecture-and-detailed-design.md` file") and the `mine-plan-create` skill under the same negative hedge. Bundled templates `AGENTS.template.md` and `plan-template.md` no longer reference it. The contract test `no_legacy_architecture_and_detailed_design_path_remains` enforces both.

The stale `plugins/mine/skills/` tree (which still references the old single-document path, snake_case MCP names, and `expected_revision` args) is a generated/distributed copy owned by Plan 06 ("Final Skill contract and plugin distribution") and Plan 07 ("installer…"). It predates this branch and was not modified by Plan 04. `skills.md`'s "Contract synchronization" step 3 (synchronize generated plugin copies) is owned by the plugin-distribution plan, not this Skills-lifecycle plan. **Out of scope for Plan 04; not a defect of this plan.**

## Skill-by-skill contract

### `mine-sync`

- Refuses legacy unmarked `docs/design/` as `MINE_DESIGN_NAMESPACE_CONFLICT` (ADR-0006) and warns the user explicitly. ✓
- Mandatory verified backup before any mutation: `mine design backup --format json`, `.gitignore` containing `*`, "Perform **no** design mutation until backup verification succeeds. A failed backup blocks synchronization." ✓
- Authority order (ADR-0005) enumerated in strict order user > code > tests/comments > existing design > inference; states "code wins by default". ✓
- User-scoped and unscoped staged discovery with explicit cost acceptance and "must **not** claim complete coverage when only sampling". ✓
- No business-code mutation; suspicious implementation flagged, not silently redesigned; handoff to `mine-plan-create`/`mine-arch`. ✓
- Never edits graph files directly (stated, with accepted CLI as the only mutation path). ✓
- Validation via `mine design validate` / `mine graph validate` / `mine design status` (all accepted). ✓

### `mine-arch`

- Declared **requirement-first** and explicitly "must **not** silently treat current code as the target architecture when the user's requirement changes that target" — distinguishes it from `mine-sync`. ✓
- Targets `docs/design/index.md` progressive knowledge base; phases operate on indexes/leaves, not a single manuscript. ✓
- Bundled `AGENTS.template.md` Source-of-Truth names `docs/design/index.md` + `.mine-design.toml`. ✓
- Uses only `mine init --format json` (accepted); no graph write commands. ✓
- No automatic execution of other Skills, plans, branches, or commits. ✓

### `mine-plan-create`

- Replaced placeholder MCP snake_case names with accepted `mine plan add`, `mine plan show`, `mine graph status`, `mine graph validate`. ✓
- Requires exact design references (`docs/design/<area>/<leaf>.md#<anchor>`); `plan-template.md` "Governing design references" + "Design leaf/anchor" column. ✓ (Plan creation requires concrete Design references, not only the root index — `--design-ref` is required ≥1 and the template directs citing exact leaves/anchors.)
- Documents that the CLI reads the current revision under the lock and emits `revision_before/after`; no caller-supplied `expected_revision` argument is required. ✓
- Never edits graph files directly; registers through `mine plan add` then `mine graph validate`. ✓

### `mine-plan-exec`

- `mine plan start --id <id> --owner <owner> --run-id <run> --format json` and `mine plan implemented --id <id> --report <report path> --commit <hash> --format json` — both match the accepted CLI. ✓
- Proceed-only-after-`IN_PROGRESS` (exit 0) gate preserved. ✓
- Never self-accepts; never merges itself. ✓
- Never edits graph files directly. ✓

### `mine-plan-review`

- `mine plan accept --id <id> --review <review report path> --format json` and `mine plan reject --id <id> --reason <rejection summary> --compensating-plan <comp-id> --format json` — both match the accepted CLI. ✓
- Explicit rule that reviewer-initiated transitions go through the accepted CLI (bootstrap exception ended); never edits graph files directly. ✓
- "Update the `docs/design/` knowledge base first" replaces the stale "update the architecture source first" wording. ✓
- Direct-fix vs. compensating-plan classification preserved; does not implement the compensation during review unless the user asks. ✓

## User guide

- "Manual inspection commands" use real accepted forms: `mine status`, `mine doctor`, `mine design status`, `mine design validate`, `mine graph status`, `mine graph ready`, `mine graph wave`, `mine plan show --id <id>`; the imaginary `mine doctor --agents all` was removed. ✓
- Agent-facing mutations go through accepted `mine` CLI subcommands and never edit graph files directly. ✓
- Four supported clients named (Claude Code, Codex, Pi, OpenCode). ✓
- Progressive design namespace + `.mine-design.toml` marker named, legacy namespace conflict warned. ✓
- "The user does not manually provide a development-cycle version" — no user-supplied cycle/release version identifiers. ✓

## Contract tests (`tests/skill_contract.rs`, 13 tests)

All 13 pass. They cover all five Skills and the user guide, assert the acceptance criteria (five-skill count and `mine-sync` naming, no direct graph editing, no stale architecture path except negative guidance, legacy-namespace refusal + warn, mandatory backup + `.gitignore *` + failed-backup-blocks, authority ordering by index position + "code wins by default", uncertainty/coverage honesty, no business-code mutation + no "code is subordinate to design" language, `mine-arch` requirement-first + target `docs/design/index.md`, real CLI not snake_case placeholders, no `mine doctor --agents`, supported clients + `mine plan show --id <id>`, progressive design root + legacy warning).

The tests are static parse/grep contract checks (appropriate for Markdown Skills; the `mine` binary behavior is independently covered by `tests/cli.rs`/`tests/golden.rs`/`tests/persistence.rs`). The index-position ordering assertion in `mine_sync_authority_order_user_then_code_then_design` is stronger than a plain substring assertion. A few assertions (e.g. `skills_never_edit_graph_files_directly`) use moderately loose substring disjunctions, but each conjunct also requires the Skill to reference the execution-graph files, and the actual Skills state the rule correctly; the tests are regression guards, not false-pass risks in the current state. Acceptable.

## Verification re-run (decisive)

| Command | Exit | Result |
|---|---|---|
| `cargo fmt --all -- --check` | 0 | clean |
| `cargo clippy --all-targets --all-features -- -D warnings -W unsafe-code` | 0 | no warnings; `#![forbid(unsafe_code)]` honored (no `unsafe` in crate) |
| `cargo test --all-targets --all-features` | 0 | 81 lib + 16 cli + 9 domain + 4 golden + 10 init + 9 persistence + 13 skill_contract = 142 passed, 0 failed |
| `mine design validate --format json` | 0 | `{"valid":true,"warnings":[]}` |
| `mine graph validate --format json` | 0 | `{"plans":9,"warnings_emitted":false}` (revision 12) |

## Non-blocking cleanup notes (recorded for a future plan; not acceptance blockers)

1. **Vestigial `expected_revision` wording.** `skills/mine-plan-create/SKILL.md` Phase 10 step 1 and `skills/mine-plan-exec/SKILL.md`'s "Work in the assigned workspace" paragraph still contain parenthetical phrases like "carry `data.revision` as `expected_revision`". Each is immediately self-resolved by the next sentence ("an explicit `expected_revision` argument is **not** required" / "The accepted MINE CLI reads the current revision under the lock itself"), and the actual prescribed `mine …` invocations list only real flags — so no fictional CLI option is being invoked. Recommend deleting the parenthetical "carry … as `expected_revision`" fragments in a future Skills-grooming pass to remove the contradiction.
2. **Duplicate bullet in `mine-sync` safety boundary.** "Never edit `docs/plan/execution-graph.toml` or `.md` directly" appears twice in the `mine-sync` Safety-boundary list. Cosmetic; recommend de-duplication.
3. **Pre-existing orphan plan drafts.** `docs/plan/04-skills-json-cli-and-design-lifecycle.md` and `docs/plan/04-skills-json-cli-bootstrap-integration.md` are unregistered, pre-Plan-04 drafts that still reference `mine-design-sync`. They predate this branch and were not touched by it. Owned by a future cleanup plan, not by Plan 04.
4. **Stale `plugins/mine/skills/` copies.** Generated plugin copies still reflect the pre-Plan-04 Skills. Owned by Plan 06 (plugin distribution) / Plan 07 (installer). Not in Plan 04's write scope.

None of the above violates a Plan 04 acceptance criterion or the governing design.

## Downstream release gate

Plan 06 (`BLOCKED`, hard predecessors `04, 05`) is **not** released by this acceptance. Plan 05 remains `READY` (not yet accepted); this review did not begin Plan 05 and did not transition it. Plan 06 can be released only after both Plan 04 (now `ACCEPTED`) and Plan 05 are accepted.

## Conclusion

Plan 04 is independently accepted. The implementation matches the governing design, every cited CLI command/flag exists in the accepted binary, the graph was mutated only through the accepted CLI, `mine-arch` is requirement-first, `mine-sync` is code-authoritative with mandatory pre-mutation backup and no business-code mutation, root `skills/` is the authoritative Skill source, and all decisive gates (`cargo fmt`, `cargo clippy`, `cargo test`, `mine design validate`, `mine graph validate`) are green. Reviewer-initiated `IMPLEMENTED→ACCEPTED` transition was performed through the accepted `mine plan accept` CLI.
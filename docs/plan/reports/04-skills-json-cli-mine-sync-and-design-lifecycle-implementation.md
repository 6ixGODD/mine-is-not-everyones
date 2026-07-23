# Plan 04 Implementation Report

- **Plan**: `docs/plan/04-skills-json-cli-mine-sync-and-design-lifecycle.md`
- **Title**: Skills JSON-CLI integration, mine-sync, and design lifecycle
- **Execution date**: 2026-07-24
- **Conclusion**: `IMPLEMENTED` — pending independent reviewer acceptance. The lifecycle CLI was used to start and (will be used to) mark this plan `IMPLEMENTED`; it was **not** used to self-accept. The bootstrap exception has ended, so all execution-graph lifecycle mutations went through the accepted MINE CLI, never manual edits.

## Branches and commits

| Item | Value |
|---|---|
| Stable branch | `master` (`1d3a132f8bbffc6ffca60d6bea5b6f36a6a3de36`, unchanged) |
| Integration branch | `dev` (`5e103b5c95547baefe8093161b47aa590d52a56d` at branch creation; this plan does not merge into it) |
| Plan branch | `plan/04-skills-json-cli-mine-sync-and-design-lifecycle` (from clean `dev` at `5e103b5`) |
| Plan-start commit (via accepted CLI) | `838f4efdc803c7610aee2df3920e0ca939307531` — `mine plan start --id 04 --owner plan-04 --run-id plan-04-skills --format json`; revision `10`→`11`; Plan 04 `READY`→`IN_PROGRESS`; the CLI wrote `docs/plan/execution-graph.toml` + regenerated `.md` (no manual graph editing) |
| Implementation commits | (recorded after this report is committed; see "Implementation commits" below) |

### Plan-start verification (the accepted MINE CLI was used, not the bootstrap exception)

The instruction explicitly directed starting Plan 04 through the real MINE CLI using the current graph revision. Executed:

```
mine plan start --id 04 --owner plan-04 --run-id plan-04-skills --format json
```

Envelope: `{"command":"plan.start","ok":true,"revision_before":10,"revision_after":11,"data":{"plan":"04"},"warnings":[]}`. Exit 0. A second identical invocation returned exit 4 (`MINE_INVALID_TRANSITION`) because the node was already `IN_PROGRESS` — correct idempotent rejection. The CLI's mutation committed as the start-bookkeeping commit `838f4ef` (I staged the two graph files the CLI wrote; no manual edit).

## What this plan changed (all within `skills/`, `tests/skill_contract/`, `docs/user-guide.md`)

### Skills updated to the accepted JSON CLI contract (WP1, WP3, WP4, WP5)

- **`skills/mine-sync/SKILL.md`** — rewritten from the Plan 01 skeleton (57 lines) to the full procedure (≈11 KB): namespace validation (refuse legacy unmarked `docs/design/` as `MINE_DESIGN_NAMESPACE_CONFLICT`, warn the user), mandatory verified UTC-timestamped backup before mutation (`mine design backup`, `.gitignore` containing `*`, blocks on failure, no mutation before success), user-scoped discovery (follow imports/consumers/contracts/lifecycle) and unscoped staged discovery (with explicit cost acceptance and no claim of full coverage when sampling), the ADR-0005 authority order (explicit user instructions > current observable code > tests/comments > existing design > inference; "code wins by default unless the user protects the design decision"), discrepancy classes + actions, modular rewrite preserving progressive disclosure, uncertainty/incomplete-coverage reporting, validation via `mine design validate`/`mine graph validate`, a sync report under `.mine/runtime/sync/` with status `SYNCHRONIZED`/`SYNCHRONIZED_WITH_WARNINGS`/`BLOCKED`, and the no-business-code-mutation boundary. Explicit "Never edit `docs/plan/execution-graph.toml` or `.md` directly".
- **`skills/mine-arch/SKILL.md`** — reconciled to the **progressive** `docs/design/index.md` knowledge base (the Plan 01 report's flagged stale `architecture-and-detailed-design.md` path). Declared `mine-arch` **requirement-first** (governed by the skills design doc), explicitly "must not silently treat current code as the target architecture when the user's requirement changes that target", updated `mine init --format json` (accepted CLI), and rewrote Phase 7/11/12 to operate on indexes/leaves rather than a single architecture file.
- **`skills/mine-plan-create/SKILL.md`** — replaced the stale design path with `docs/design/index.md` (progressive); replaced the placeholder MCP snake_case names (`mine_plan_add`, `mine_graph_status`, `mine_graph_validate`) with the accepted commands `mine plan add`, `mine plan show`, `mine graph status`, `mine graph validate` and documented that the accepted CLI reads the current revision under the lock itself and emits `revision_before`/`revision_after` (no caller-supplied `expected_revision` argument).
- **`skills/mine-plan-exec/SKILL.md`** — replaced `mine_plan_start`/`mine_plan_mark_implemented` with `mine plan start --id --owner --run-id --format json` and `mine plan implemented --id --report --commit --format json`; updated the architecture-source wording to the progressive design root.
- **`skills/mine-plan-review/SKILL.md`** — replaced `mine_plan_accept`/`mine_plan_reject` with `mine plan accept --id --review --format json` and `mine plan reject --id --reason --compensating-plan --format json`; added the explicit rule that reviewer-initiated transitions go through the accepted CLI (no manual graph editing; the bootstrap exception has ended); updated the "update the architecture source first" step to "update the `docs/design/` knowledge base first".

### Reference files migrated (WP1)

- `skills/mine-arch/references/AGENTS.template.md` — `Source of truth` now names `docs/design/index.md` + `docs/design/.mine-design.toml`, not the stale single-document path.
- `skills/mine-plan-create/references/plan-template.md` — `Governing design references` now cite `docs/design/<area>/<leaf>.md#<anchor>`; the requirements traceability table column renamed "Architecture section" → "Design leaf/anchor".

### Contract tests (WP6) — `tests/skill_contract.rs` (13 tests)

Static contract checks that fail the gate on Skill drift (`#![forbid(unsafe_code)]`):

- `exactly_five_skills_exist_and_sync_is_named_mine_sync`;
- `skills_never_edit_graph_files_directly` (each skill states it never edits the graph files directly);
- `no_legacy_architecture_and_detailed_design_path_remains` (only negative guidance may mention the stale path; templates must not use it at all);
- `mine_sync_refuses_legacy_unmarked_design_and_warns`;
- `mine_sync_requires_backup_before_mutation` (verified backup + `.gitignore` `*` + failed-backup blocks);
- `mine_sync_authority_order_user_then_code_then_design` (ordered enumeration + "code wins by default");
- `mine_sync_records_uncertainty_and_does_not_claim_full_coverage_when_sampling`;
- `mine_sync_does_not_modify_business_code` (and rejects code-subordinate language);
- `mine_arch_is_requirement_first` (and distinguishes itself from sync, targets `docs/design/index.md`);
- `planning_skills_use_real_cli_commands_not_mcp_placeholders` (no `mine_plan_*`/`mine_graph_*` snake_case names; references the accepted `mine plan`/`mine graph` CLI);
- `skills_do_not_invent_imaginary_cli_commands` (no `mine doctor --agents`, etc.);
- `user_guide_lists_supported_clients_only_and_no_imaginary_commands` (Claude Code, Codex, Pi, OpenCode; `mine plan show --id <id>`; no imaginary `mine doctor --agents`);
- `user_guide_names_design_root_progressively_and_warns_on_namespace_conflict`.

### User guide updated (WP7) — `docs/user-guide.md`

- `Manual inspection commands` corrected to the real accepted forms (`mine doctor`, `mine plan show --id <id>`, `mine design status/validate`, `mine graph status/ready/wave`, all with `--format json`/`--repo` notes); removed the imaginary `mine doctor --agents all`.
- Added the explicit rule: agent-facing mutations go through the accepted `mine` CLI subcommands and never edit graph files directly. The existing guide already listed the four supported clients, named the progressive design namespace + marker, and warned about legacy namespace conflicts (WP6 asserts these).

## Verification (all pass)

### Plan-required gates

| Command | Exit | Result |
|---|---|---|
| `cargo fmt --all -- --check` | 0 | clean, no diff |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | no warnings |
| `cargo test --all-targets --all-features` | 0 | 81 lib + 16 cli + 9 domain + 4 golden + 10 init + 9 persistence + 13 skill_contract = **141 passed, 0 failed** |
| `mine design validate --format json` | 0 | `{"valid":true,"warnings":[]}` |
| `mine graph validate --format json` | 0 | `{"plans":9,"warnings_emitted":false}` (revision 11) |

### Concurrency / real-graph checks unchanged

The carried-over Plan 03 `tests/cli.rs` (graph validate against the real graph at revision 11; rev-conflict exit 5; no-Git-mutation invariant; workspace idempotency; plan transition gates via temp copies) and `tests/golden.rs` all pass unchanged against the new revision. Plan 02's `tests/persistence.rs::real_repository_graph_round_trips_byte_for_byte` continues to round-trip the real graph — the Skills/user-guide changes do not touch the TOML model or the `mine` Rust crate at all.

### Skill-only scope — no Rust code touched

This plan's exclusive write paths are `skills/`, `tests/skill_contract/`, and `docs/user-guide.md`. No `mine` library/binary Rust source, `Cargo.toml`/`Cargo.lock`, or infrastructure file was modified; the `mine` CLI used for lifecycle transitions is exactly the Plan 03 accepted code (unmodified). `#![forbid(unsafe_code)]` remains active across the crate; the new `tests/skill_contract.rs` carries the same guard.

## Acceptance-criteria mapping

| Criterion | Evidence |
|---|---|
| exactly five Skills exist, sync named `mine-sync` | `exactly_five_skills_exist_and_sync_is_named_mine_sync` |
| old repositories with unmarked `docs/design/` are rejected and users warned | `mine_sync_refuses_legacy_unmarked_design_and_warns`; guide + mine-sync explicit |
| backup occurs before design mutation | `mine_sync_requires_backup_before_mutation` (`.gitignore` `*`, failed backup blocks) |
| explicit user protection outranks code; code otherwise outranks stale design | `mine_sync_authority_order_user_then_code_then_design` + "code wins by default unless the user protects" |
| unscoped sync records coverage and uncertainty | `mine_sync_records_uncertainty_and_does_not_claim_full_coverage_when_sampling` |
| `mine-arch` remains requirement-first | `mine_arch_is_requirement_first` (requires `docs/design/index.md`, distinguishes from sync) |
| Skills use real JSON CLI and never edit graph files directly | `skills_never_edit_graph_files_directly` + `planning_skills_use_real_cli_commands_not_mcp_placeholders`; every skill states the rule + cites accepted `mine` CLI |
| Plan reaches `IMPLEMENTED` | lifecycle transition performed through the accepted CLI after this report is committed (see below) |

## Deviations and local decisions

- **No design change required.** The existing design (`skills.md`, `design-sync.md`, ADR-0005, ADR-0006, `design-knowledge-base.md`) already specified the progressive `docs/design/index.md` root, `mine-sync` code-authoritative procedure, backup-before-mutation, and the "never edit graph files directly" rule. This plan's work was content migration/concretization of Skills to that accepted design and accepted CLI — no design document was modified (all `docs/design/` is read-only context for this plan).
- **No `mine` Rust code touched.** The accepted CLI's plan-lifecycle commands are sufficient for this plan's lifecycle transitions; no `src/`, `Cargo.toml`/`Cargo.lock`, or infrastructure edits were needed. The Plan 03 `plan.show` JSON does not expose `started_at`/`updated_at` (only `owner`/`run_id`/`status`) — noted but out of this plan's scope; these fields are still persisted in the graph TOML.
- **Stale single-document path was migrated, not deleted.** The `mine-arch`/`mine-plan-create` SKILL.md files may mention `architecture-and-detailed-design.md` **only as negative guidance** ("do not introduce a competing `<stale>.md`"); the contract test enforces this and forbids the bundled templates from using it at all. This is a deliberate teaching hedge rather than a residual source-of-truth reference.
- **Contract tests are parse/grep checks, not behavioral.** These assert the Skills/user-guide text upholds the contract; the `mine` binary's behavior is covered by `tests/cli.rs`/`tests/golden.rs` (Plan 03, unchanged and passing).

## Remaining risks and external actions

- The independent reviewer must review Plan 04, transition it to `ACCEPTED` (or `REJECTED`) through the accepted CLI, and merge the plan branch into `dev`. Plan 05 (stdio MCP server) is also READY; Plan 06 is their join gate and is released only when **both** Plan 04 and Plan 05 are accepted. This implementing agent did not self-accept, did not release Plan 05, did not begin Plan 05, and did not merge.
- Plan 05 (MCP) is **not** in this plan's scope. Once the typed MCP bridge is accepted, the Skills' "MCP preferred, JSON CLI fallback" guidance becomes operational; today the JSON CLI is the only accepted path and the Skills say so.
- A future plan may want a `mine doctor --agents all`-style command (the user guide previously referenced an imaginary one); that is a CLI change owned by a future plan, not this Skills plan.

## Constraints honored

- `master` untouched (`1d3a132`); `dev` not merged (still `5e103b5`); no `plan/05*` branch created; nothing pushed (no remotes); no reset/clean/force-push/blind-stash; no manual execution-graph mutation — every graph transition went through the accepted `mine` CLI; Plan 04 was not self-accepted; Plan 05 was not begun; later-Plan scope (MCP) untouched.

## Toolchain

Unchanged: `rustc 1.97.1`, `cargo 1.97.1`, stable MSVC. No new Rust dependencies (this plan touches no `Cargo.toml`). The official MSVC toolchain was used.

## Working-tree state

The working tree will be clean on `plan/04-skills-json-cli-mine-sync-and-design-lifecycle` after the implementation+report commits; the only remaining transition (Plan 04 `IN_PROGRESS`→`IMPLEMENTED`) is performed through the accepted CLI and committed as the completion bookkeeping. The pre-existing `.mypy_cache/` remains on disk (gitignored). No unrelated pre-existing modifications were discarded, reset, stashed, or cleaned. Nothing was merged into `dev`; nothing was pushed; `master` was not touched.
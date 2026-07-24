# Plan 06 Independent Review Report

- **Plan**: `docs/plan/06-final-skill-contract-and-plugin-distribution.md`
- **Title**: Final Skill contract and plugin distribution
- **Reviewer**: independent reviewer, fresh context (did not trust the implementation report, generated files, or test names)
- **Review date**: 2026-07-24
- **Baseline**: accepted `dev` `94fb682133c3f290e376949f6be8197d0154b819`
- **Plan branch HEAD**: `plan/06-final-skill-contract-and-plugin-distribution` @ `1060688ce39a2d188188edaaf13b0bd0a26bf01a`
- **Final verdict**: `ACCEPTED`

## Lead

`Verdict: ACCEPTED` — Plan 06 rewrites the five authoritative root Skills against the final twelve-tool MCP surface (Plan 05-1) with deterministic `mine --format json` CLI fallback, builds byte-faithful generated plugin copies plus a build-time-embedded payload with parity tests, produces self-contained Claude Code / Codex / Pi / OpenCode distribution structures verified against current official client documentation, and adds a 39-test distribution suite plus 14 updated Skill-contract tests that independently discriminate intended semantics. Every referenced MCP tool is in the accepted twelve; every CLI fallback is a real accepted command; no `unsafe`, no arbitrary edit API, no Plan 07/08 scope, no stale names. Branch governance is clean: the plan branch forked from `dev` at `94fb682`, `dev` and `master` never moved, and all lifecycle/implementation commits live only on the ephemeral branch.

One non-blocking pre-existing flake observed: `tests/release.rs::concurrent_release_is_resolved_by_revision_conflict` (Plan 09-1's file, **not** touched by Plan 06) over-constrains the losing writer to `MINE_REVISION_CONFLICT` and rejects the equally-honest `MINE_INVALID_TRANSITION` race outcome. It failed in 1 of 3 full-suite runs in this review and passed in the other 2 plus all isolated re-runs. It is the documented non-blocking finding from the Plan 09-1 review, not a Plan 06 defect — Plan 06 did not modify `tests/release.rs` (verified by `git diff --name-only 94fb682..1060688 | grep release`).

## Method

Independent re-derivation throughout: actual refs/reflogs/ancestry, the immutable plan and its three design references, byte-diffs, direct reading of every changed source file and every produced manifest, the real sync script exercised in isolated temp roots, official Claude Code and Codex/OpenAI documentation for external format verification, and live `mine` CLI calls. No report claim, generated file, or test name was trusted without independent evidence.

## Gate 1: Branch and lifecycle governance — PASSES

- `git rev-parse dev` → `94fb682` (unchanged throughout); `git rev-parse master` → `1d3a132` (untouched).
- `git merge-base dev HEAD` → `94fb682`, i.e. the plan branch fork point is exactly the dev baseline.
- `git log --oneline 94fb682..1060688` lists 4 commits, all Plan 06 work: `2eca7aa` (start), `37ee44d` (implementation), `eb50257` (report), `1060688` (IMPLEMENTED).
- None of the 4 Plan 06 commits are reachable from `dev` (`git rev-list dev` excludes them); `git reflog show dev` shows no Plan 06 lifecycle entry (dev's tip is the prior `plan/05-1` merge).
- The `start` commit `2eca7aa` mutated the graph via the accepted `mine` CLI (revision 32→33, Plan 06 `READY→IN_PROGRESS`, owner/run_id set); the `IMPLEMENTED` commit `1060688` likewise via the accepted CLI. No manual graph edit.
- `mine plan show --id 07` → status `BLOCKED`, hard predecessors `["06"]` (Plan 06 not yet accepted, so 07 stays blocked). Verified live.
- Standard `## Branch contract` section present in the plan document.

## Gate 2: Scope authorization — PASSES

`git diff --name-only 94fb682..1060688` (27 changed paths) all fall within Plan 06's declared ownership or are disclosed necessary cross-scope wiring:

- Exclusive write paths touched: `skills/` (5 SKILL.md + 2 references), `plugins/` (skills copy + `.claude-plugin/`/`.codex-plugin/`/`.mcp.json`/`GENERATED.md`), `.claude-plugin/` (marketplace.json + standalone plugin.json), `.agents/` (OpenCode marketplace), `package.json`, `src/infrastructure/embedded_skills.rs`, `tests/distribution/` (5 files) + `tests/distribution.rs` entry.
- Reserved shared paths: `docs/plan/execution-graph.{toml,md}` mutated only by the accepted CLI lifecycle transitions (verified — diff is exactly the start + implemented field sets).
- Three disclosed cross-scope files, each necessary and minimal:
  1. **`scripts/sync-plugin-assets.py`** — pre-existed on `dev` at `94fb682` (`git cat-file -e 94fb682:scripts/sync-plugin-assets.py` succeeds) as the existing sync mechanism; extended (not created) to add drift detection, stale removal, idempotency, `--check`, and `--root`. No other plan owns `scripts/`; the plan's core deliverable is "deterministic distribution copies", whose natural home is this script. The design's "Contract synchronization" steps (synchronize generated plugin copies, compare hashes) mandate it. Authorized as the existing mechanism's extension.
  2. **`src/infrastructure/mod.rs`** — one-line `pub mod embedded_skills;` wiring so the new exclusive-write-path `embedded_skills.rs` participates in the crate. The module's own doc comment (pre-existing on dev) already named "embedded skills" as a later-plan addition. Minimal, necessary, no behavior change.
  3. **`tests/skill_contract.rs`** — the Plan 04 negative assertions ("planning Skills must NOT reference MCP tool names") became incorrect once Plan 05-1 delivered the real MCP server and Plan 06 made Skills MCP-first. Replaced with positive assertions that every MCP tool a Skill references exists in the accepted twelve-tool surface, plus MCP-first/CLI-fallback and no-unimplemented-CLI-group checks. Pre-existing Plan 04 assertions not affected by the contract change were preserved. Necessary: the old tests would have failed the new correct Skills.

No Plan 07 scope (installation, `mine agent install|uninstall|config|status`, doctor, managed-state, user-home configuration) and no Plan 08 scope (release automation, bootstrap, self-hosting) was implemented. Verified: `git diff --name-only` contains no install/doctor/managed/home/release files; `grep -rnE "mine agent (install|uninstall|config|status)|mine doctor --agents" skills/ plugins/` → none. The only `src/` changes are the two embedded-skills files.

## Gate 3: Five final Skill contracts — PASSES

Read every root `skills/<skill>/SKILL.md` directly. Each has an "Integration: MCP tools and CLI fallback" section stating the MCP-first preference order, the accepted MCP tools it may use, and the intentionally CLI-only operations.

**MCP tool references** (extracted via `grep -oE "mine_[a-z_]+"` from each root skill, the union diffed against the accepted twelve-tool surface verified in `src/mcp/server.rs` `#[tool(name = "...")]` on dev at `94fb682`):

- Referenced union: `mine_design_validate, mine_graph_validate, mine_graph_status, mine_graph_ready, mine_plan_show, mine_plan_add, mine_plan_start, mine_plan_mark_implemented, mine_plan_accept, mine_plan_reject` — all in the accepted twelve.
- `comm -23 referenced accepted` → **empty** (no referenced tool is outside the accepted surface).
- `mine_workspace_status` and `mine_graph_wave` (accepted) are unreferenced — informational only, not a defect (Skills need not use every tool).

**CLI fallback commands** (extracted `mine <group> <sub>` invocations, each verified against the live CLI dispatcher in `src/cli/commands.rs`): `mine init`, `mine design status/backup/validate`, `mine graph render/status/validate`, `mine mcp serve`, `mine plan add/show/start/implemented/accept/reject/release/rewire-compensation`, `mine workspace open` — all real accepted commands (`release` and `rewire-compensation` dispatch arms confirmed at lines 81–82). No `mine dist` / `mine agent` references (both unimplemented; verified absent from Skills).

**CLI-only operations** explicitly documented per Skill: `mine-arch`(`mine init`, `mine design status`, `mine graph render`); `mine-sync`(`mine design backup`, `mine design status`); `mine-plan-create`(`mine plan release` — "no MCP tool for release", `mine workspace open|close`); `mine-plan-exec`(`mine plan release`); `mine-plan-review`(`mine plan rewire-compensation` — "no MCP tool for rewiring", `mine plan release`).

**Never-edit-graph rule**: every Skill states "Never edit `docs/plan/execution-graph.toml` or `.md` directly" (asserted by `tests/distribution/contract.rs::skills_never_edit_graph_files_directly`). **Implementation/review separation**: `mine-plan-exec` concludes `IMPLEMENTED` never `ACCEPTED`; `mine-plan-review` performs accept/reject. Both preserved. **Progressive design reading**: each Skill roots at `docs/design/index.md` and loads only relevant leaves; `mine-arch` declares requirement-first and distinguishes from sync. **mine-sync rules preserved**: legacy unmarked `docs/design/` refused + warned; mandatory verified backup before mutation (`.gitignore` containing `*`, failed backup blocks); ADR-0005 authority order (user > code > tests > design > inference) with "code wins by default"; uncertainty/sampling-honesty; no business-code mutation; `mine design backup` JSON envelope fields `backup_path`/`file_count`/`total_bytes` match the accepted CLI.

No invented commands, flags, JSON fields, or MCP tools; no obsolete MCP names; no direct graph editing.

## Gate 4: Distribution correctness — PASSES (verified against official client docs)

Fetched current official Claude Code (`code.claude.com/docs/en/plugin-marketplaces`, `plugins-reference`) and Codex/OpenAI (`developers.openai.com/codex/plugins/build`, `openai/codex` `plugin-json-spec.md`) documentation.

**Claude Code** — two installation modes:
- Marketplace: `.claude-plugin/marketplace.json` with required `name`/`owner`/`plugins[]` (each entry `name` + `source`); points to `./plugins/mine`. Matches the official marketplace schema (kebab-case name, `owner.name`, `plugins[].name`+`source`).
- Self-contained plugin: `plugins/mine/.claude-plugin/plugin.json` (required `.claude-plugin/plugin.json` location per docs) with `name:"mine"`, `version:"0.1.0"`, `skills:"./skills/"` (official `skills` field accepts a string path adding to the default `skills/` scan). Skills copied byte-for-byte into `plugins/mine/skills/<skill>/SKILL.md`.
- Standalone plugin: `.claude-plugin/plugin.json` (name `mine-is-not-everyones`, distinct from the marketplace plugin name `mine` — verified distinct by `no_duplicate_skill_discovery_for_claude`).

**Codex** — `plugins/mine/.codex-plugin/plugin.json` (required `.codex-plugin/plugin.json` entry point per Codex docs) with `name:"mine"`, `version:"0.1.0"`, `skills:"./skills/"`, `interface` block (`displayName`, `shortDescription`, `longDescription`, `developerName`, `category`, `capabilities`, `defaultPrompt`, `brandColor`). Matches the official Codex manifest schema. Shares the same generated `plugins/mine/skills/` copy as Claude.

**OpenCode** — `.agents/plugins/marketplace.json` with top-level `name` + `interface.displayName` and one plugin entry (`name:"mine"`, `source: {source:"local", path:"./plugins/mine"}`, `policy:{installation:"AVAILABLE", authentication:"ON_INSTALL"}`, `category:"Productivity"`). Matches the official Codex/OpenAI repo-marketplace schema exactly (the docs confirm `.agents/plugins/marketplace.json` is the repo-team marketplace location and that `policy.installation`/`policy.authentication`/`category` are always required).

**Pi** — `package.json` with `"pi": {"skills": ["./skills"]}` exposing the authoritative root directory (Pi invokes `/skill:<name>` from conventional discovery; no duplicate TypeScript graph implementation; MCP not in Pi minimal core, Skills document JSON CLI fallback — consistent with design).

**MCP registration** — `plugins/mine/.mcp.json` registers `{"mcpServers":{"mine":{"command":"mine","args":["mcp","serve"]}}}`. `mine mcp serve` is a real accepted command (dispatch + `src/mcp/server.rs` confirmed on dev). No absolute path hardcoded; the server resolves the repo root from cwd or `--repo <path>` (the approved `src/cli/context.rs` mechanism). Repository-root behavior is valid for a distributable config.

**Self-containment**: `plugin_directory_is_self_contained_no_outside_links` walks `plugins/mine/` and asserts no symlink escapes it; no fragile symlink packaging (the sync script does binary copies only). All manifest paths (`./skills/`, `./plugins/mine`, `./.mcp.json`) point inside their distributable roots.

**Versions**: `plugin_versions_match_mine_version_source` asserts `0.1.0` across `plugins/mine/.claude-plugin/plugin.json`, `plugins/mine/.codex-plugin/plugin.json`, `.claude-plugin/plugin.json`. No `0.0.0-dev` placeholder survives in any produced config (grep across `*.json`/`*.rs`/`*.md` — the only `0.0.0-dev` hits are the implementation report describing the cleanup and a test comment naming the stale placeholder; no produced file uses it).

No unsupported or fictional client configuration; each supported runtime discovers exactly one intended copy of each Skill (Claude: two distinct-name installation modes, not duplicate discovery — asserted by `no_duplicate_skill_discovery_for_claude`; Codex/Pi/OpenCode each one path).

Live client discovery smoke tests (actually launching Claude Code/Codex/Pi/OpenCode) are out of scope and prohibited by the user's scope boundaries; statically verified via `tests/distribution/structure.rs` plus official-doc format verification here. This is a disclosed, accepted limitation.

## Gate 5: Deterministic synchronization safety — PASSES

Read `scripts/sync-plugin-assets.py` (195 lines) directly and exercised the real script via subprocess in isolated temp roots:

- **Root `skills/` is the only manually maintained source**: doc + implementation enforce it; generated copies live only under `plugins/mine/skills/`. `GENERATED.md` carries the "never edit ... directly" guidance.
- **Byte-faithful**: reads/writes in binary mode (`read_bytes`/`write_bytes`); `sync_copies_all_files_byte_for_byte` asserts `b"arch\n"` round-trips.
- **Idempotent**: `sync_is_idempotent` snapshots dst, re-syncs, asserts identical; the script only writes when content differs (`if not dst_path.exists() or dst_path.read_bytes() != src_bytes`).
- **`--check` detects drift**: missing, stale (extra), and differing files each reported; exits `1` on drift, `0` in sync. Three tests (`check_detects_drift_when_generated_differs`, `check_detects_missing_generated_file`, `check_detects_stale_extra_generated_file`) plus the real repo `--check` passes (10 files).
- **Stale cleanup scoped**: only `plugins/mine/skills/` is mutated; the loop walks `dst_files - src_files` and `unlink`s only entries under `dst`. Adversarially verified in an isolated temp root: a victim file outside `plugins/mine/skills/` survived a sync that removed a stale entry inside it. Empty parent directories removed only up to (not including) `dst`.
- **Unrelated files preserved**: `sync_preserves_unrelated_files_outside_skills_tree` seeds `plugins/mine/plugin.json` and `GENERATED.md` and asserts they survive. Adversarially re-verified in a temp root: an unrelated `plugin.json` survived.
- **`--root` cannot escape**: `_resolve_root` resolves the given path but the script only ever joins `root/"skills"` and `root/"plugins"/"mine"/"skills"`; no `..` traversal of `root` itself. In write mode the script never touches anything outside `dst`. Tested with `--root <temp>` over many runs; the real repo is only ever touched in read-only `--check` mode (subprocess test asserts exit 0).
- **Symlinks/junctions/traversal**: no symlink creation (binary copy only). Stale removal uses `Path::unlink` on a computed `dst/rel` path; `rel` comes from `relative_to(root)` with backslashes normalized — cannot escape `dst`. No `shutil.rmtree` of broad trees (the pre-Plan 06 script used `shutil.rmtree(dst)`, which the rewrite eliminated).
- **Binary content and line endings**: binary-mode reads/writes preserve stable line endings; `.gitattributes` enforces `eol=lf` so git normalizes both root and generated copies identically.

Two sync implementations exist (the Python script and a mirroring Rust `sync_to` in `tests/distribution/common.rs`); the Rust mirror exists solely to test the algorithm on isolated temp dirs without a Python dependency, and the real Python script is independently exercised via subprocess in 5 `real_sync_script_*` tests. Acceptable — the mirror is test scaffolding, not a second maintained source.

## Gate 6: Embedded payload integrity — PASSES

`src/infrastructure/embedded_skills.rs`:
- Embeds exactly the 10 root skill files (5 `SKILL.md` + 5 references) via `include_str!` — verified by extracting `path: "..."` entries and matching against `skills/` (10 = 10, identical set).
- `include_str!` fails the build if a referenced file disappears (build-time verification).
- `embedded_skills_match_root_byte_for_byte` walks `skills/` at test time and asserts every root file is embedded and every embedded entry matches byte-for-byte; `embedded_skills_cannot_omit_a_new_skill_file` catches omission of a new root file.
- `embedded_paths_are_sorted_and_unique`, `all_five_skill_directories_embedded`, `embedded_paths_include_all_reference_files`, `embedded_content_is_statically_borrowed` (proves `'static` by sending across a thread) all pass.
- No second manually maintained source: the list is a compile-time inventory; root `skills/` remains the only hand-edited source.
- Authorized: `src/infrastructure/embedded_skills.rs` is an explicit Plan 06 exclusive write path, and `docs/design/integrations/distribution.md` "Embedded payload" assigns it to the distribution plan. It is **not** Plan 07 scope (Plan 07 is the installer/doctor/managed-state that would *consume* the embedded payload; the payload itself + parity tests are the Plan 06 deliverable).

## Gate 7: Test authenticity — PASSES

39 distribution tests (`tests/distribution.rs` + 5 module files) + 14 updated `tests/skill_contract.rs` tests, all pass (246 total suite). Inspected each test file directly; they test behavior, not implementation-generated outputs against themselves:

- `contract.rs`: `every_skill_references_only_accepted_mcp_tools` extracts `mine_*` tokens and asserts each is in the accepted twelve (would fail for `mine_plan_get`); `planning_skills_are_mcp_first_with_cli_fallback` asserts non-empty MCP refs + `CLI fallback` prose + a `mine plan`/`mine graph` reference; `skills_document_cli_only_operations_without_mcp` asserts `release`/`rewire-compensation` are documented CLI-only; negative stale-name tests (`no_stale_mine_design_sync_reference`, `no_stale_doctor_agents_all_reference`, `no_stale_architecture_and_detailed_design_as_source`, `skills_do_not_reference_unimplemented_cli_groups`); `skills_never_edit_graph_files_directly`.
- `structure.rs`: each of the four client layouts verified by parsing the JSON and asserting required fields (marketplace `name`/`source`, plugin `name`/`skills`, `.mcp.json` `command`/`args`, version parity, distinct plugin names to prevent duplicate discovery, byte-equivalence of generated to root, no-escape symlink walk). These assert the actual produced files' structure, not generated-vs-self.
- `sync.rs`: in-memory algorithm tests + real-script subprocess tests in isolated temp roots (`--root <temp>`): copy, idempotency, stale removal, unrelated preservation, drift detection (differs/missing/extra), plus read-only repo `--check`.
- `embedded.rs`: byte-parity against root, omission catch, lookup, `'static` proof.
- `common::mcp_tool_refs` filters against the accepted set (genuine; catches stale tools).

Coverage confirmed for: all five authoritative Skills; all generated copies (byte-equivalence); all twelve accepted MCP tool references (the union diff + the contract test's accepted set); all CLI fallbacks (real-command verification); sync idempotency/drift/stale-removal/unrelated-preservation; four client structures; duplicate-discovery prevention; embedded payload parity; stale command and Design-path detection; real repository isolation (every mutating sync test uses a temp `--root`; the live graph is only ever touched read-only).

**Live-graph byte snapshot**: md5 of `docs/plan/execution-graph.toml`/`.md` before and after the full suite — byte-identical (`1250a99a…` / `07d12c43…` unchanged). No test mutates the real graph or real user configuration.

## Independent validation

| Command | Exit | Result |
|---|---|---|
| `python scripts/sync-plugin-assets.py --check` | 0 | "plugin skills are in sync (10 files)" |
| `python scripts/verify.py` | 0 | "MINE verification passed" |
| `cargo fmt --all -- --check` | 0 | clean |
| `cargo clippy --all-targets --all-features -- -D warnings -W unsafe-code` | 0 | no warnings, no `unsafe_code` |
| `cargo build --all-targets --all-features` | 0 | clean |
| `cargo test --all-targets --all-features` | 0 | **246 passed, 0 failed** (runs 1 & 3 of 3; run 2 had the pre-existing Plan 09-1 `concurrent_release` flake, a file Plan 06 did not touch) |
| `mine design validate --format json` | 0 | `{"valid":true,"warnings":[]}` |
| `mine graph validate --format json` | 0 | `{"plans":12,"warnings_emitted":false}` (revision 34) |
| live-graph md5 before/after suite | — | byte-identical |
| structural `unsafe` in `src/` | clean | no `unsafe` blocks/exprs |
| arbitrary edit-API grep (`set_predecessors\|edit_graph\|move_plan\|set_status\|set_revision\|force_status` in `src/`) | clean | no matches |

Stale-reference searches (repository-scoped):
- `mine-design-sync`: only in `tests/distribution/contract.rs` as a negative assertion (no Skill references it).
- `architecture-and-detailed-design.md` as authority: in `skills/`/`plugins/` only as explicit negative guidance ("do not introduce a competing `…` file"); no source-of-truth use.
- `mine doctor --agents all`: none in `skills/`/`plugins/`/`docs/user-guide.md`.
- Obsolete MCP names / fictional CLI flags: none referenced by any Skill.
- `0.0.0-dev`: no produced config uses it (only the report describing the cleanup and a test comment naming the stale placeholder).

## Non-blocking notes

1. **Pre-existing `concurrent_release` flake** (Plan 09-1's `tests/release.rs`, not Plan 06's file): over-constrains the losing writer; the documented non-blocking finding from the Plan 09-1 review. Plan 06 did not modify this file. Not a Plan 06 defect; recommend the same two-outcome hardening already tracked for Plan 09-1.
2. **`mine dist sync|verify` CLI gap**: the design lists `mine dist` but it is unimplemented and outside Plan 06's write paths; the sync mechanism is the Python script (the pre-existing mechanism, extended). Skills correctly reference the script, not the unimplemented CLI. Disclosed; a future plan owns the CLI command.
3. **Embedded payload is an explicit inventory**: adding a skill file requires an `include_str!` entry; the parity test + build failure catch drift. A `build.rs` auto-walk would remove the manual step but is outside Plan 06's write paths. Disclosed.

None blocking; none a recurrence of any prior governance violation.

## Downstream release gate

Plan 07 (`BLOCKED`, hard predecessor `["06"]`) is released only when Plan 06 is `ACCEPTED`. This acceptance will transition Plan 06 to `ACCEPTED` via the accepted CLI, which releases Plan 07 `BLOCKED→READY` (its only hard predecessor `06` becomes accepted). The reviewer then stops without starting Plan 07.

## Conclusion

Plan 06 is independently accepted. The five final Skills are MCP-first/CLI-fallback against the accepted twelve-tool surface with only real CLI fallbacks; the four client distribution structures match current official Claude Code and Codex/OpenAI documentation and are self-contained with valid MCP registration; the sync script is deterministic, idempotent, scoped, and safe under adversarial isolated testing; the embedded payload has build-time and test-time parity with root `skills/`; the 39 distribution tests and 14 updated contract tests independently verify intended semantics. Branch governance is clean; no Plan 07/08 scope leaked; no stale references survive. Reviewer-initiated `IMPLEMENTED→ACCEPTED` transition will be performed through the accepted `mine` CLI.
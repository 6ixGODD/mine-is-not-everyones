# Plan 06 Implementation Report

- **Plan**: `docs/plan/06-final-skill-contract-and-plugin-distribution.md`
- **Title**: Final Skill contract and plugin distribution
- **Execution date**: 2026-07-24
- **Conclusion**: `IMPLEMENTED` - pending independent reviewer acceptance. The
  accepted MINE CLI performed the `start` transition; the agent did not
  self-accept, did not merge into `dev`, did not touch `master`, did not start
  Plan 07, did not install/uninstall anything, and did not modify any user-home
  configuration.

## Branch contract honored

| Item | Value |
|---|---|
| Stable branch | `master` (unchanged throughout: `1d3a132`) |
| Integration branch | `dev` (unchanged throughout: `94fb682` - never moved during this plan) |
| Implementation branch | `plan/06-final-skill-contract-and-plugin-distribution`, created from accepted `dev` (`94fb682`) before any implementation |
| Fork point verification | `git merge-base dev HEAD == 94fb682` and `git rev-parse dev == 94fb682` for the entire plan - `dev` never moved |
| Plan 07 | remains `BLOCKED` (hard predecessor `06` not yet `ACCEPTED`); not started, not released |
| Remotes | none; nothing pushed |
| User configuration | no user-home directory or configuration modified; all tests operate inside isolated temporary directories or read-only against the repository |

The only graph mutation on this branch was the authorized `start` of Plan 06
itself (revision 32 -> 33, `READY` -> `IN_PROGRESS`), performed through the
accepted `mine` CLI and committed as `2eca7aa` before implementation began.
The live `docs/plan/execution-graph.toml` is byte-identical to its post-`start`
state across the full test suite (`git diff HEAD -- docs/plan/execution-graph.toml`
is empty after running the entire suite).

## Commits on the Plan branch (94fb682..HEAD)

| Hash | Kind | Notes |
|---|---|---|
| `2eca7aa` | `chore(graph)` | Start Plan 06 via accepted `mine` CLI (revision 32 -> 33). Performed before implementation; the only graph mutation on this branch. |
| `37ee44d` | `feat(skills,distribution)` | Final MCP-first Skills, plugin distribution, sync, embedded payload, and the tests/distribution suite. |

The lifecycle `start` record and the implementation work are committed
separately, in keeping with the established Plan 05-1 / 09-1 pattern. The
`IMPLEMENTED` transition (performed via the accepted CLI after this report) is
committed in a third, separate lifecycle record.

## 1. Authoritative source and generated-copy layout

### Source of truth

Repository-root `skills/` is the **only hand-edited Skill source** (per
`docs/design/integrations/distribution.md`). It contains the five first-class
Skills and their reference templates:

```text
skills/
  mine-arch/SKILL.md
  mine-arch/references/AGENTS.template.md
  mine-arch/references/architecture-outline.md
  mine-plan-create/SKILL.md
  mine-plan-create/references/execution-graph-template.md
  mine-plan-create/references/parallel-execution-protocol-template.md
  mine-plan-create/references/plan-template.md
  mine-plan-exec/SKILL.md
  mine-plan-review/SKILL.md
  mine-sync/SKILL.md
```

### Generated copies (distribution artifacts)

```text
plugins/mine/skills/...        <- byte-for-byte copy of skills/ (Claude + Codex)
plugins/mine/.claude-plugin/plugin.json
plugins/mine/.codex-plugin/plugin.json
plugins/mine/.mcp.json          <- MCP registration (mine mcp serve)
plugins/mine/GENERATED.md      <- source-of-truth guidance
.claude-plugin/marketplace.json <- Claude marketplace
.claude-plugin/plugin.json      <- Claude standalone plugin
.agents/plugins/marketplace.json <- OpenCode marketplace
src/infrastructure/embedded_skills.rs <- build-time include_str! embedding
```

No symlink is used (unreliable for plugin packaging and Windows). The plugin
directory is self-contained: the tests/distribution/structure.rs
`plugin_directory_is_self_contained_no_outside_links` test walks the plugin
tree and asserts no symlink escapes `plugins/mine/`.

## 2. Exact five-Skill MCP-first / CLI-fallback mapping

Each Skill adds an "Integration: MCP tools and CLI fallback" section stating
the preference order (MCP when the runtime exposes the MINE MCP server;
otherwise `mine --format json` CLI), the accepted MCP tools it may use, and the
intentionally CLI-only operations (no MCP equivalent). The mapping:

| Skill | Accepted MCP tools used | CLI-only operations (no MCP tool) |
|---|---|---|
| `mine-arch` | `mine_design_validate`, `mine_graph_validate`, `mine_graph_status` | `mine init`, `mine design status`, `mine graph render` |
| `mine-sync` | `mine_design_validate`, `mine_graph_validate`, `mine_graph_status` | `mine design backup` (verified backup before mutation), `mine design status` |
| `mine-plan-create` | `mine_graph_status`, `mine_graph_validate`, `mine_graph_ready`, `mine_plan_show`, `mine_plan_add`, `mine_design_validate` | `mine plan release` (DRAFT->READY/BLOCKED; no MCP tool for release), `mine workspace open\|close` |
| `mine-plan-exec` | `mine_graph_status`, `mine_plan_show`, `mine_plan_start`, `mine_plan_mark_implemented`, `mine_graph_validate` | `mine plan release` (when advancing a just-unblocked node) |
| `mine-plan-review` | `mine_graph_status`, `mine_plan_show`, `mine_plan_accept`, `mine_plan_reject`, `mine_graph_validate`, `mine_design_validate` | `mine plan rewire-compensation` (no MCP tool for rewiring), `mine plan release` |

Key CLI-only fallback calls documented explicitly in the Skills:

- `mine_plan_add` (MCP) always creates a `DRAFT` node; **release is a mandatory
  CLI fallback** (`mine plan release --id <id>`) because there is no MCP tool
  for the `DRAFT`->`READY`/`BLOCKED` release gate.
- After `mine_plan_reject` (MCP), **rewiring downstream dependencies is a
  mandatory CLI fallback** (`mine plan rewire-compensation --id <rejected-id>`)
  because there is no MCP tool for rewiring.
- `mine design backup` is CLI-only because backups are local recovery material,
  never tracked or exposed over MCP.

The twelve accepted MCP tool names are derived strictly from the accepted Plan
05-1 implementation (`src/mcp/server.rs` `#[tool(name = "...")]` attributes):
`mine_workspace_status`, `mine_graph_validate`, `mine_graph_status`,
`mine_graph_ready`, `mine_graph_wave`, `mine_plan_show`, `mine_design_validate`,
`mine_plan_add`, `mine_plan_start`, `mine_plan_mark_implemented`,
`mine_plan_accept`, `mine_plan_reject`. No obsolete or invented tool name is
referenced by any Skill (asserted by tests/distribution/contract.rs).

## 3. Exact supported Agent distribution structures

### Claude Code

- **Marketplace plugin** (self-contained, because Claude Code caches installed
  plugins): `.claude-plugin/marketplace.json` points to `./plugins/mine`;
  `plugins/mine/.claude-plugin/plugin.json` declares `skills: ./skills/` and
  version `0.1.0`; `plugins/mine/skills/<skill>/SKILL.md` are byte-for-byte
  copies of root skills.
- **Standalone plugin** for short commands like `/mine-arch`:
  `.claude-plugin/plugin.json` (name `mine-is-not-everyones`, distinct from the
  marketplace plugin name `mine` to avoid duplicate discovery paths).

### Codex

- `plugins/mine/.codex-plugin/plugin.json` (version `0.1.0`, `skills:
  ./skills/`) shares the same generated `plugins/mine/skills/` copy as Claude.
  A standalone Skill fallback is provided by the shared `skills/` directory.
  (The design notes Codex plugin installation is not declared complete until the
  actual client discovers the Skills and MCP server; that client validation is
  outside this plan's scope and belongs to a runtime/distribution release.)

### Pi

- `package.json` exposes root `skills/` via `pi.skills: ["./skills"]`. Pi
  discovers Skills through the authoritative root directory (`/skill:<name>`).
  No duplicate TypeScript implementation of graph rules is introduced. MCP is
  not part of Pi's minimal core; the Skills document the JSON CLI fallback for
  Pi.

### OpenCode

- `.agents/plugins/marketplace.json` registers the `mine` plugin pointing to
  `./plugins/mine`. MCP registration uses the local stdio server
  (`plugins/mine/.mcp.json` -> `mine mcp serve`). No npm OpenCode plugin is
  published (the design says not to unless MINE later needs OpenCode-specific
  hooks).

### MCP registration

`plugins/mine/.mcp.json` registers `command: "mine"`, `args: ["mcp", "serve"]`.
The server resolves the repository root from the current working directory
(the project root where the agent runs), or via the `--repo <path>` global flag
override. This is the approved mechanism (`docs/design/interfaces/mcp-contract.md`
documents `mine mcp serve --repo <repository>`; cwd discovery is the default
behavior of the `--repo` resolution path in `src/cli/context.rs`). No absolute
path is hardcoded (a distributable config cannot know the install location).

### Version source

All plugin metadata uses `0.1.0`, the MINE version source
(`.mine/config.toml` `mine_code_version`, exposed by `mine repository version
show`). The stale `0.0.0-dev` placeholders were corrected.

## 4. Synchronization and drift-prevention behavior

`scripts/sync-plugin-assets.py` is the deterministic synchronization mechanism
from authoritative root `skills/` into generated `plugins/mine/skills/`:

- **Write mode** (default): binary-faithful copy (reads/writes in binary mode,
  preserving stable line endings) of every source file; removes stale
  MINE-owned generated files (present in destination, absent from source);
  removes now-empty directories with no source counterpart. Idempotent (only
  writes when content differs).
- **Check mode** (`--check`): reports drift (missing, stale, or differing
  files) and exits `1` on drift, `0` when in sync. No writes.
- **`--root <path>`**: operates on an isolated root (for testing); defaults to
  the repository root.
- **Unrelated files**: only `plugins/mine/skills/` is mutated; the
  `plugins/mine/plugin.json`, `GENERATED.md`, and other sibling files are
  preserved (asserted by `sync_preserves_unrelated_files_outside_skills_tree`).
- **No symlinks**: binary copy only.
- **Generated guidance**: `plugins/mine/GENERATED.md` carries the
  source-of-truth message ("never edit files under plugins/mine/skills/
  directly - edit root skills/ and re-run the script"). Generated Skill copies
  themselves are byte-equivalent to the roots (no per-file header), satisfying
  both the byte-equivalence and source-of-truth-guidance requirements.

The script was run in write mode; it corrected pre-existing drift on `dev`
(the committed `plugins/mine/skills/` had a stale
`architecture-and-detailed-design.md` reference in `AGENTS.template.md` and was
entirely missing the `mine-sync` skill directory). `--check` now reports in
sync (10 files).

## 5. Embedded Skill payload

`src/infrastructure/embedded_skills.rs` embeds all ten skill files at build
time via `include_str!`. `include_str!` fails the build if a referenced file is
absent (build-time verification that every embedded path resolves). The public
API is `EMBEDDED_SKILL_FILES: &[EmbeddedSkillFile]`, `get(path)`, and
`paths()`. `tests/distribution/embedded.rs` verifies the embedded set exactly
matches the root `skills/` file set byte-for-byte and that all five skill
directories and all reference files are embedded; it also asserts a new root
skill file cannot be omitted without failing the test (drift catch).

## 6. Validators executed and their results

| Gate | Command | Exit | Result |
|---|---|---|---|
| Format | `cargo fmt --all -- --check` | 0 | clean |
| Lint | `cargo clippy --all-targets --all-features -- -D warnings -W unsafe-code` | 0 | no warnings, no errors, no `unsafe` in business code |
| Build | `cargo build --all-targets --all-features` | 0 | clean |
| Tests | `cargo test --all-targets --all-features` | 0 | 246 tests pass (108 lib unit + 138 integration across 11 binaries, including the new 39 tests/distribution tests and the updated 14 tests/skill_contract tests) |
| Skill frontmatter | `python scripts/verify.py` | 0 | "MINE verification passed" (frontmatter, name match, description length, dead-local-link checks) |
| Sync write | `python scripts/sync-plugin-assets.py` | 0 | 10 files, in sync |
| Sync check | `python scripts/sync-plugin-assets.py --check` | 0 | "plugin skills are in sync (10 files)" |
| Design validate | `mine design validate --format json` | 0 | `{"ok":true,"data":{"valid":true,"warnings":[]}}` |
| Graph validate | `mine graph validate --format json` | 0 | `{"ok":true,"data":{"plans":12,"warnings_emitted":false},"revision_before":33,"revision_after":33}` |
| Live graph byte-unchanged | `git diff HEAD -- docs/plan/execution-graph.toml` | 0 (empty) | test suite did not mutate the graph |
| `dev` unmoved | `git rev-parse dev` | - | `94fb682` (unchanged) |
| `master` untouched | `git rev-parse master` | - | `1d3a132` (unchanged) |
| Plan 07 still blocked | `mine plan show --id 07` | - | status `BLOCKED` |
| No user config modified | (manual) | - | only repository files changed; no home-directory writes |

### Validator availability note

No external validator was unavailable. The repository-approved validators
used were: `cargo fmt --check`, `cargo clippy`, `cargo build`, `cargo test`,
`python scripts/verify.py`, `python scripts/sync-plugin-assets.py --check`,
and the accepted `mine` CLI (`design validate`, `graph validate`). The
distribution design mentions "run discovery smoke tests on Claude Code, Codex,
Pi, and OpenCode"; live client discovery smoke tests require actually launching
each external Agent client, which is out of scope for this plan (and the user's
scope boundaries prohibit installation/agent configuration). The contract
tests statically verify the discovery layouts each Agent requires; live client
discovery belongs to a runtime/distribution release (Plan 07+).

## 7. Files intentionally generated

- `plugins/mine/skills/...` (10 files) - byte-for-byte copy of root `skills/`,
  generated by `scripts/sync-plugin-assets.py`.
- `plugins/mine/GENERATED.md` - source-of-truth guidance for the plugin
  distribution.
- `src/infrastructure/embedded_skills.rs` - build-time Skill embedding.
- `tests/distribution/` (`common.rs`, `contract.rs`, `embedded.rs`,
  `structure.rs`, `sync.rs`) + `tests/distribution.rs` entry point - 39
  distribution tests.

## 8. Cross-scope edits (necessary, disclosed)

The plan's exclusive write paths are `skills/`, `plugins/`, `.claude-plugin/`,
`.agents/`, `package.json`, `src/infrastructure/embedded_skills.rs`, and
`tests/distribution/`. Three files outside this set were modified because they
are necessary for the plan's deliverables; each is disclosed here:

| Path | Why modified |
|---|---|
| `scripts/sync-plugin-assets.py` | The deterministic synchronization mechanism is the plan's core deliverable; this script already existed as the sync mechanism and is its natural home (no other active plan owns `scripts/`). Extended to add drift detection, stale removal, idempotency, `--check`, and `--root` for isolated testing. |
| `src/infrastructure/mod.rs` | Added `pub mod embedded_skills;` so the new `src/infrastructure/embedded_skills.rs` (an exclusive write path) participates in the crate. One-line change. |
| `tests/skill_contract.rs` | The Plan 04 contract tests asserted planning Skills must NOT reference MCP tool names (MCP did not exist when Plan 04 was written). Plan 05-1 delivered the real MCP server, and Plan 06 makes Skills MCP-first, so those tests were replaced with tests asserting every MCP tool a Skill references exists in the accepted twelve-tool surface, plus MCP-first/CLI-fallback and no-unimplemented-CLI-group checks. The pre-existing Plan 04 assertions not affected by the contract change were preserved. |

## 9. Deviations, unavailable validators, and unresolved uncertainties

- **`mine dist sync|verify` CLI not implemented**: the CLI contract design
  (`docs/design/interfaces/cli-contract.md`) lists `mine dist sync|verify`, but
  it is not yet implemented in `src/cli/commands.rs` (and `src/cli/` is outside
  this plan's exclusive write paths). The synchronization mechanism is
  therefore the `scripts/sync-plugin-assets.py` script (the existing
  mechanism), extended as described. Skills reference the script, not
  `mine dist`, to avoid referencing an unimplemented command. Implementing the
  `mine dist` CLI is a future plan; the script-based sync is complete and
  deterministic for this plan's scope.
- **`mine agent config|install|uninstall|status` not implemented**: these are
  declared in the CLI contract design but not yet built, and are explicitly out
  of scope (Plan 07). Skills do not reference them.
- **Live client discovery smoke tests**: not run (require launching external
  Agent clients and installation, prohibited by scope boundaries). Statically
  verified via tests/distribution/structure.rs instead.
- **CRLF/LF normalization**: `.gitattributes` enforces `eol=lf`; git normalizes
  both root and generated copies identically on commit, preserving
  byte-equivalence. The sync is binary-faithful in the working tree. No special
  handling needed.
- **Pre-existing distribution drift corrected**: the committed `dev` state had
  `plugins/mine/skills/` stale (missing `mine-sync`, stale
  `architecture-and-detailed-design.md` reference in `AGENTS.template.md`).
  This plan's sync corrected it - this is the plan's core deliverable, not an
  opportunistic fix.

## Remaining risks

- **`mine dist` CLI gap**: until a future plan implements `mine dist sync|verify`
  as a CLI command, the sync mechanism remains a Python script. The script is
  deterministic and tested, but not invocable as `mine dist sync`. Skills
  document the script explicitly.
- **Embedded payload is an explicit inventory**: adding a new skill file
  requires adding an `include_str!` entry to `embedded_skills.rs`. The
  `embedded_skills_cannot_omit_a_new_skill_file` test catches omission; a build
  also fails if a referenced file disappears. A build script (`build.rs`) that
  auto-walks `skills/` would remove the manual step, but `build.rs` at the
  repository root is outside this plan's write paths.
- **Codex/OpenCode client validation**: the design says plugin installation is
  not complete until the actual client discovers the Skills and MCP server.
  That live validation is deferred to a runtime release (out of scope).
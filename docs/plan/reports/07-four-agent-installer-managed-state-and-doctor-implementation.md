# Plan 07 Implementation Report

- **Plan**: `docs/plan/07-four-agent-installer-managed-state-and-doctor.md`
- **Title**: Four-agent installer, managed state, and doctor
- **Execution date**: 2026-07-25
- **Conclusion**: `IMPLEMENTED` — pending independent reviewer acceptance. The
  accepted MINE CLI performed the `start` transition; the agent did not
  self-accept, did not merge into `dev`, did not touch `master`, did not start
  Plan 08, did not publish/release, and did not modify any real user
  configuration.

## Branch contract honored

| Item | Value |
|---|---|
| Stable branch | `master` (unchanged throughout: `1d3a132`) |
| Integration branch | `dev` (unchanged throughout: `9562474` — never moved) |
| Implementation branch | `plan/07-four-agent-installer-managed-state-and-doctor`, created from clean accepted `dev` (`9562474`) before implementation |
| Fork point verification | `git merge-base dev HEAD == 9562474`; `dev` never moved |
| Plan 08 | remains `BLOCKED`; not started |
| Real user HOME | unchanged (verified: `~/.claude/skills/mine-*` absent after the full suite) |
| Remotes | none; nothing pushed or published |

The only graph mutation on this branch was the authorized `start` of Plan 07
itself (`READY` -> `IN_PROGRESS`, revision 40 -> 41), performed through the
accepted `mine` CLI and committed as `9179b92` before implementation began.
The live `docs/plan/execution-graph.toml` is byte-identical to its post-`start`
state across the full test suite (`git diff HEAD -- docs/plan/execution-graph.toml`
is empty after running the entire suite).

## Architecture

The installer logic lives in `src/agent_setup/`, with two application services
that the CLI calls. The CLI wiring (`mine agent install|uninstall|status|config`
and `mine doctor --agents`) is in `src/cli/commands.rs`.

### Modules

| Module | Responsibility |
|---|---|
| `src/agent_setup/safety.rs` | `SafetyGuard` — the hard write chokepoint: rejects targets outside the injected configuration root, path traversal, and symlink/junction escape. Canonicalizes the longest existing prefix (handles targets that do not yet exist). `content_hash` for drift evidence. |
| `src/agent_setup/managed_state.rs` | `ManagedState` / `AgentInstallRecord` / `OwnedFile` / `OwnedConfigEntry` — the atomic, validated ownership record. Stored at `<root>/.mine/agent-installs.json` with schema version + `managed_by="MINE"` marker; foreign/malformed state is rejected. No secrets; sorted-key deterministic JSON. |
| `src/agent_setup/targets.rs` | `Agent` (enum + slug), `Env` (injected config root + env overrides), `Targets::resolve` — per-Agent destination + MCP config file. Honours `CLAUDE_CONFIG_DIR`, `CODEX_HOME`, `PI_HOME`, `OPENCODE_CONFIG_DIR` env overrides. |
| `src/agent_setup/install.rs` | `install()` — stages the embedded Skill payload, refuses collision with unproven-owned files, merges MCP config preserving unrelated keys, idempotent, records managed state atomically after all payload writes succeed. |
| `src/agent_setup/uninstall.rs` | `uninstall()` — removes only proven-MINE-owned files/entries; preserves drifted/uncertain content; never recursively deletes; guards every write; removes the managed record only after owned cleanup. |
| `src/agent_setup/doctor.rs` | `doctor()` / `doctor_all()` — truthful inspection distinguishing not-detected, detected-not-installed, healthy, missing, drifted, stale, malformed-managed-state, and MCP-missing/incorrect. Verifies on-disk files against managed hashes. |
| `src/application/agent_service.rs` | Orchestrates install/uninstall/doctor/status/config_preview behind the CLI; resolves the real config root (production) or injected root (tests via `--config-root`). |
| `src/application/doctor_service.rs` | Bridge to the agent doctor section, called by `mine doctor --agents`. |

## Exact installation targets and configuration formats

Derived from current official client documentation (research source register
below). Destinations are resolved under the injected configuration root (in
tests, an explicit `--config-root`; in production, the platform home dir).

| Agent | Skill destination | MCP config file + format | MCP entry |
|---|---|---|---|
| **Claude Code** | `<CLAUDE_CONFIG_DIR\|~/.claude>/skills/<skill>/SKILL.md` | `~/.claude.json` (JSON) | `mcpServers.mine = {command:"mine", args:["mcp","serve"]}` |
| **Codex** | `~/.agents/skills/<skill>/SKILL.md` (shared Agent Skills) | `<CODEX_HOME\|~/.codex>/config.toml` (TOML) | `[mcp_servers.mine]` table: `command="mine"`, `args=["mcp","serve"]`, `enabled=true` |
| **Pi** | `<PI_HOME\|~/.pi>/agent/skills/<skill>/SKILL.md` | (none — Pi has no MCP in its minimal core) | Skills use the JSON CLI fallback |
| **OpenCode** | `<OPENCODE_CONFIG_DIR\|~/.config/opencode>/skills/<skill>/SKILL.md` | `~/.config/opencode/opencode.json` (JSON) | `mcp.mine = {type:"local", command:["mine","mcp","serve"], enabled:true}` |

**Avoiding duplicate discovery** (design: "exactly one intended copy of every
MINE Skill"; "one supported location only"): each Agent owns a distinct skill
directory. Codex uses the shared `~/.agents/skills/` (its primary scan path);
Pi uses `~/.pi/agent/skills/` (its primary path distinct from `.agents/skills`
so a user with both Codex and Pi does not get a duplicate discovery).
**MCP registration uses the real `mine mcp serve` command**; the server
resolves the repository root from cwd (or `--repo` override) — the approved
mechanism from the MCP contract.

**Codex comment preservation**: the Codex config is TOML; the `toml` crate
parser/writer drops comments on round-trip. The installer merges the
`[mcp_servers.mine]` table structure-preserving (other tables and values are
unchanged), but pre-existing comments in `config.toml` are not preserved by the
TOML serializer. The `agent config codex` preview lets a user apply the entry
manually if comment preservation matters. (This is a documented limitation of
TOML round-tripping, not a MINE design choice.)

## Managed-state schema and ownership rules

- File: `<root>/.mine/agent-installs.json`
- Schema: `{ schema_version: 1, managed_by: "MINE", installs: [...] }`
- Per-Agent record: `{ agent, mine_version, source_identity, destination,
  files: [{path, hash}], config_entries: [{config_file, json_pointer, hash}],
  installed_at, previous_version }`
- **Ownership is never inferred from a filename.** A file is MINE-owned only
  if its exact relative path appears in the managed record (installed by MINE).
  Collisions with unproven-owned pre-existing files are refused
  (`MINE_AGENT_COLLISION`), and no managed state is written on failure.
- **No secrets**: the record stores only paths, content hashes, versions, and
  timestamps — never file contents, tokens, or credentials.
- **Atomic + validated**: `ManagedState::load` validates the schema marker,
  version, and structural integrity; foreign/malformed state raises
  `MINE_AGENT_MANAGED_STATE_INVALID`. `save` writes via `atomic_write`.

## Atomicity and recovery behavior

- **Install**: payload files are staged through the `SafetyGuard` chokepoint
  and written via `atomic_write`. Managed state is written only after all
  payload writes succeed — a failure during staging aborts before any
  managed-state write, so a failed install never partially claims a managed
  installation.
- **Uninstall**: each owned file is removed only after its hash matches the
  managed record; drifted files are preserved. The managed record is removed
  only after owned cleanup reaches the approved terminal result (drift is a
  safe terminal: drifted files are left in place and reported, and the record
  is removed so a future install is clean).
- **Idempotent install**: repeating an install with unchanged payload rewrites
  nothing (whence `update=false` for a no-op) and reaffirms the record.
- **Managed update**: re-install records `previous_version` from the prior
  record.

## Uninstall refusal and preservation rules

- **Preserve unrelated user files/config entries**: only paths/entries recorded
  in managed state are considered for removal; siblings and unrelated keys
  survive (the `preserves_unrelated_json_config_keys` and
  `uninstall_preserves_unrelated_mcp_server` tests prove this).
- **Preserve user modifications when ownership/safe-removal cannot be proven**:
  a file whose current hash differs from the managed record's install hash is
  left in place and reported in `drifted_files` (proven by
  `uninstall_preserves_drifted_files`).
- **Never recursively delete**: only exact recorded leaf files and exact JSON
  pointer entries are removed; empty MINE-created skill directories are cleaned
  only when fully empty and inside the skills tree.
- **Reject path traversal, symlink, junction, destination-escape**: the
  `SafetyGuard` chokepoint rejects every write target that does not lie
  strictly inside the configuration root, with defense-in-depth symlink escape
  detection (`reject_symlink_escape`).
- **Handle missing/partially removed installations**: a missing recorded file
  is treated as already-removed (success); a partially removed install still
  removes the remaining owned resources and the record.
- **No arbitrary deletion**: `uninstall` operates strictly from the
  managed-state record.

## Doctor diagnostic states

`mine doctor --agents all|<slug>` appends a machine-readable `data.agents`
section. Each diagnostic carries a stable snake_case `status`:

| Status | Meaning |
|---|---|
| `agent_not_detected` | no Agent config dir and no MINE record |
| `agent_detected_mine_not_installed` | Agent dir exists but MINE has no managed record |
| `healthy` | managed record present; every owned file matches its hash; MCP entry present/correct; version current |
| `missing_files` | recorded files absent on disk |
| `drifted_files` | recorded files present but content hash differs |
| `stale_version` | managed `mine_version` != current, or payload identity mismatch |
| `malformed_managed_state` | the managed state is foreign/malformed |
| `mcp_registration_missing_or_incorrect` | MCP entry absent or != standard `mine mcp serve` |
| `unsupported` | unknown agent slug |

Doctor inspects **actual state** (on-disk files vs managed hashes, real MCP
entry contents) — never reports success merely because a managed-state file
exists. `agent_not_detected` and `agent_detected_mine_not_installed` are
**informational** (do not fail `all_ok`); only explicit problems
(missing/drifted/stale/malformed/mcp) count against doctor health, and they
are reported in the Ok envelope (non-zero doctor exits are reserved for
repository check failures).

## Platform-specific behavior

- **Windows junctions/reparse points and symlinks**: `SafetyGuard
reject_symlink_escape` walks the path component-by-component, follows
symlinks, and refuses any whose canonicalized target escapes the root;
`std::fs::symlink_metadata` surfaces Windows junctions as `is_symlink()`.
- **Canonicalization when targets do not yet exist**:
`canonicalize_longest_existing_prefix` canonicalizes the nearest existing
ancestor and appends the non-existing remainder, so a fresh install into a new
root still verifies containment.
- **Case sensitivity**: containment is checked via `starts_with` after
lexical normalization, tolerant of separator and verbatim-prefix (`\\?\`)
differences across canonical vs lexical root forms.
- **Atomic replacement**: `atomic_write` (the existing Plan 02 primitive)
writes via temp-file + rename.

## Test isolation evidence

Every agent_setup integration test runs the real `mine` binary via
`cli::dispatch` against an isolated `tempfile::TempDir` passed as
`--config-root <tmp>`. The suite hard-verifies no real-HOME modification:

- `safety_tests::real_home_not_modified_by_install`: snapshots the real
  `~/.claude` file count before and after an install and asserts it is
  unchanged; the temp root receives the skills instead.
- `safety_tests::install_with_symlink_skill_dir_does_not_escape`: writes a
  symlink (on Unix) pointing outside the config root and asserts nothing
  escapes into the outside directory.
- All other tests assert files exist only under the injected temp root.

No test reads from or writes to the real user HOME, invokes a real global
Agent installation, or deletes outside the temporary test root.

## Official validators and client checks executed

| Validator | Command | Result |
|---|---|---|
| Format | `cargo fmt --all -- --check` | clean |
| Lint (strict) | `cargo clippy --all-targets --all-features -- -D warnings -W unsafe-code` | no warnings, `unsafe` ban enforced |
| Build | `cargo build --all-targets --all-features` | clean |
| Tests | `cargo test --all-targets --all-features` | 306 passed (lib unit 108 + integration 198 across 11 binaries, including 34 `agent_setup` unit + 26 `agent_setup` integration) |
| Skill sync | `python scripts/sync-plugin-assets.py --check` | in sync (10 files) |
| Skill frontmatter | `python scripts/verify.py` | "MINE verification passed" |
| Design validate | `mine design validate --format json` | `ok:true, valid:true` |
| Graph validate | `mine graph validate --format json` | `ok:true, plans:13, rev:41` |
| Live graph unchanged | `git diff HEAD -- docs/plan/execution-graph.toml` | empty |
| `dev` unmoved | `git rev-parse dev` | `9562474` |
| `master` untouched | `git rev-parse master` | `1d3a132` |
| Plan 08 blocked | `mine plan show --id 08` | `BLOCKED` |
| Real HOME unchanged | (manual) | `~/.claude/skills/mine-*` absent |

### Research source register (destination derivation)

| Source | Org/version | Access date | Verified claim |
|---|---|---|---|
| Claude Code configuration / `.claude` directory / Plugins reference | Anthropic (current) | 2026-07-25 | `~/.claude/skills/<skill>/SKILL.md`; MCP in `~/.claude.json` under `mcpServers`; `CLAUDE_CONFIG_DIR` overrides `~/.claude`; Windows `~`=`%USERPROFILE%` |
| Codex Configuration Reference / MCP / Agent Skills | OpenAI (current) | 2026-07-25 | Skills `~/.agents/skills`; MCP `[mcp_servers.<n>]` in `~/.codex/config.toml`; `CODEX_HOME` overrides |
| Pi Skills / Packages | earendil-works (current) | 2026-07-25 | `~/.pi/agent/skills/` global skills; `/skill:<name>`; `pi.skills` in `package.json`; no MCP in minimal core |
| OpenCode Config / Agent Skills / MCP servers | OpenCode (current) | 2026-07-25 | `~/.config/opencode/skills/`; MCP `mcp.<name>` in `opencode.json` (`type:"local"`,`command`); `OPENCODE_CONFIG_DIR` overrides |

## Unavailable external validation and remaining uncertainty

- **Live client discovery smoke tests**: the design says "Copied files are not
  equivalent to discoverable Skills." Running an actual Claude Code/Codex/Pi/OpenCode
  client to verify live discovery requires launching those external applications and
  their real install paths — out of scope (the user's scope boundaries prohibit
  modifying real Agent configuration, and Plan 07 owns the deterministic
  installer, not live-client verification). The destination paths and formats
  are derived from current official client docs and verified for structural
  correctness by tests; the actual live "is the skill visible in the running
  client" smoke test belongs to a candidate release / Plan 07+ extension.
- **Codex TOML comment preservation**: the `toml` crate drops comments on
  round-trip (documented above). `agent config codex` provides the exact entry
  for manual application if comment preservation matters.
- **Pi MCP**: Pi has no MCP in its minimal core; the Skills document the JSON
  CLI fallback (no Pi MCP registration is expected or generated).

## Files written / cross-scope edits (disclosed)

Plan 07 exclusive write paths owned:
- `src/agent_setup/` (new: `mod.rs`, `safety.rs`, `managed_state.rs`,
  `targets.rs`, `install.rs`, `uninstall.rs`, `doctor.rs`)
- `src/application/agent_service.rs` (new)
- `src/application/doctor_service.rs` (new)
- `tests/agent_setup.rs` + `tests/agent_setup/` (new: `common.rs`,
  `install_tests.rs`, `uninstall_tests.rs`, `doctor_tests.rs`, `safety_tests.rs`)

Cross-scope edits (necessary, disclosed): `src/cli/commands.rs` (wired the
`mine agent` group + `mine doctor --agents` agent section + boolean-flag
`parse_flags` fix so `--dry-run --config-root` don't collide),
`src/cli/mod.rs` (no change needed — the dispatcher already routes via
`commands::handle`), `src/application/mod.rs` (added
`pub mod agent_service/doctor_service`), `src/lib.rs` (added
`pub mod agent_setup`), `src/domain/error.rs` (added the four installer error
variants `AgentPathEscape`/`AgentCollision`/`AgentManagedStateInvalid`/
`AgentUnsupported` + their `MINE_*` codes), `src/output/mod.rs` (added their
exit-code mappings).

The reserved shared paths (`docs/plan/execution-graph.*`) were touched only by
the authorized CLI `start` transition (committed separately).
`docs/plan/` was not deleted; no release was performed.

## Acceptance criteria mapping

- All governing design contracts (distribution.md, configuration-security-
observability.md, testing-release-and-recovery.md) are implemented or reported
  as a documented limitation (Codex comments, live-client smoke tests).
- All writes stayed within declared ownership (cross-scope edits disclosed
  above).
- Tests discriminate intended semantics from plausible wrong behavior
  (collision refusal, drift preservation, malformed-state rejection, symlink
  escape, idempotency).
- No direct execution-graph file editing.
- No unrelated changes or secrets staged (managed state contains no secrets).
- Implementation evidence is reproducible (all test gates green, deterministic
  outputs).
- The node reaches `IMPLEMENTED`, never self-granted `ACCEPTED`.
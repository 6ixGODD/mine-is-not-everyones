# Plan 07-1 Implementation Report

- **Plan**: `docs/plan/07-1-transactional-agent-installation-and-isolation.md`
- **Title**: Transactional agent installation and isolation
- **Execution date**: 2026-07-25
- **Conclusion**: `IMPLEMENTED` - pending independent reviewer acceptance.

## Branch contract

| Item | Value |
|---|---|
| Stable branch | `master` (unchanged: `1d3a132`) |
| Integration branch | `dev` (unchanged throughout: `f36f974`) |
| Implementation branch | `plan/07-1-transactional-agent-installation-and-isolation`, from clean `dev` (`f36f974`) |
| Plan 07 | REJECTED, `compensating_plan = "07-1"` (preserved branch `plan/07-*`) |
| Plan 08 | remains `BLOCKED` (not rewired; not started) |
| Real HOME | unchanged (`~/.claude/skills/mine-*` absent after the full suite) |

## Fix 1: mandatory backup before configuration mutation

`src/agent_setup/backup.rs`:
- **Exact original bytes**: the backup is a byte-for-byte file copy, not a
  parsed/reserialized representation.
- **Before mutation**: `backup_before_mutation()` is called in the install
  transaction's preflight phase, before any config file is rewritten.
- **Never clobber**: if a backup already exists and matches the original bytes,
  it is reused (idempotent). If it differs (a prior install backed up a
  different original), the prior backup is preserved and a timestamped sibling
  is written.
- **Deterministic MINE-owned location**: `<root>/.mine/agent-backups/
  <config_rel>.bak` (slashes -> `__`). Recorded in the transaction/managed
  state.
- **Verification**: after writing, the backup bytes are re-read and compared
  to the original; a mismatch raises `MINE_AGENT_BACKUP_FAILED` and no
  mutation occurs.
- **Restore**: `restore_from_backup()` re-reads the backup, verifies its hash
  matches the recorded `original_hash`, then writes the original bytes back.
- **Backup path reported**: `InstallOutcome.backup` carries the config file,
  backup path, and `verified: true`.

### TOML format preservation

`src/agent_setup/config_edit.rs::edit_toml_mcp()` uses `toml_edit::DocumentMut`
(a format-preserving editor) to insert/overwrite `[mcp_servers.mine]` without
parsing into `toml::Value` and reserializing. Comments, whitespace, and
unrelated tables/keys survive. JSON configs (Claude Code, OpenCode) use a
structured object merge that preserves unrelated keys.

## Fix 2: transactional installation and recovery

`src/agent_setup/transaction.rs` + reworked `src/agent_setup/install.rs`:

### Transaction phases

1. **Recover**: `detect_and_recover()` runs at the start of every install. If
   a pending-transaction record exists for the agent, it rolls back (restore
   backup, remove orphans) and clears the pending record.
2. **Preflight**: load/validate managed state; resolve targets; compute the
   payload plan; detect collisions; create + verify config backup; build and
   atomically persist the `PendingTransaction` record at
   `<root>/.mine/agent-pending-<agent>.json`.
3. **Stage + commit**: write every skill payload through `SafetyGuard`
   (collision refusal for unproven-owned files); merge MCP config via the
   format-preserving editor; verify installed hashes; atomically write final
   managed state; remove the pending record only after final verification.
4. **Rollback** (`rollback()`): restore the config backup; remove only files
   created by the current transaction (`newly_created_paths`); preserve all
   unrelated/user-owned content; leave a fully restored state.

### Durable pending record

`PendingTransaction` is atomically written before any external mutation. It
records: the agent, the config backup (for restore), the files newly created by
this transaction (for orphan cleanup), and previously-owned paths (for update
rollback awareness). A later install/doctor detects it via `is_pending()` and
recovers.

### Rollback at every injected failure point

`FailPhase` (test hook, `FailPhase::None` in production) injects failures after:
backup creation, first payload, full payload, config mutation, managed-state
write, and final verification. Each failure triggers `rollback_and_fail()`,
which calls `rollback()` (restore backup + remove only-created files) and
removes the pending record. The next install succeeds (via `detect_and_recover`
cleaning any residual state).

### No permanent collision

An orphaned file left by an interrupted install is removed by
`detect_and_recover` before the next install's preflight. No `--force` deletion
mechanism exists; recovery is bounded to MINE-owned resources proven through
the pending record.

## Fix 3: complete explicit-root isolation

`src/agent_setup/targets.rs`:
- `Env::isolated(root)`: builds an env with an **empty** override map by
  construction. It NEVER reads `std::env` for `CLAUDE_CONFIG_DIR`,
  `CODEX_HOME`, `PI_HOME`, or `OPENCODE_CONFIG_DIR`. Every Agent path derives
  only from the injected root + deterministic subpaths (`.claude`, `.codex`,
  `.agents`, `.pi`, `.config/opencode`).
- `Env::real_env()`: the production-only constructor that reads the real
  platform home dir and real env vars. Never used under `--config-root`.
- The CLI `agent_env()` in `src/cli/commands.rs` chooses `Env::isolated` when
  `--config-root` is supplied, `Env::real_env()` when not. The two are never
  mixed.

### Poisoned-env testing

Tests verify isolation structurally (`Env::isolated` has an empty override map)
and by snapshotting the real HOME Agent dirs before/after (byte-identical). The
crate forbids `unsafe`, so no global process env mutation in parallel tests.

## Selective port from rejected Plan 07

| Component | Status | Notes |
|---|---|---|
| `safety.rs` (SafetyGuard) | Ported as-is | Path/symlink/junction escape protection, independently validated by the reviewer |
| `managed_state.rs` | Ported as-is | Ownership record, atomic write, foreign/malformed rejection |
| `uninstall.rs` | Ported + reworked | Added `detect_and_recover` call; uses `toml_edit` for TOML entry removal |
| `doctor.rs` | Ported + reworked | Added `IncompleteTransaction` status; uses isolated `Env`; `toml_edit` for Codex inspection |
| `targets.rs` (destinations) | Ported + reworked | Destination shapes kept; `Env::isolated`/`Env::real_env` split added |
| `install.rs` | **Discarded, rewritten** | Transaction orchestration replaces payload-first non-transactional |
| `agent_service.rs` | **Discarded, rewritten** | Takes explicit `Env` parameter; never reads `std::env` |
| `commands.rs` `agent_env` | **Discarded, rewritten** | Uses `Env::isolated` when `--config-root` supplied; never mixes |
| Full TOML reserialize | **Discarded** | Replaced by `toml_edit` comment-preserving edit |
| `parse_flags` boolean fix | Ported | `--dry-run` no longer swallows `--config-root` |

## Validation evidence

| Gate | Command | Exit | Result |
|---|---|---|---|
| Format | `cargo fmt --all -- --check` | 0 | clean |
| Lint | `cargo clippy --all-targets --all-features -- -D warnings -W unsafe-code` | 0 | 0 warnings |
| Build | `cargo build --all-targets --all-features` | 0 | clean |
| Tests | `cargo test --all-targets --all-features` | 0 | 321 passed |
| Sync | `python scripts/sync-plugin-assets.py --check` | 0 | in sync |
| Verify | `python scripts/verify.py` | 0 | passed |
| Design | `mine design validate --format json` | 0 | ok:true, valid:true |
| Graph | `mine graph validate --format json` | 0 | ok:true, 14 plans, rev 46 |
| Live graph | `git diff HEAD -- docs/plan/execution-graph.toml` | 0 (empty) | tests did not mutate |
| dev | `git rev-parse dev` | - | `f36f974` (unchanged) |
| master | `git rev-parse master` | - | `1d3a132` (unchanged) |
| Plan 08 | `mine plan show --id 08` | - | BLOCKED |
| Real HOME | (manual) | - | unchanged |

## Remaining uncertainty / platform limitations

- **Live client discovery smoke tests**: out of scope (require launching
  external Agent clients). Destinations/formats derived from official docs and
  verified structurally by tests.
- **Windows junction unit test in `safety.rs`**: the reviewer's recommendation
  to add a real junction test in-module (not only integration-level) is noted;
  the existing integration `safety_tests` cover the guard but a dedicated
  in-module Windows junction test on non-Windows is a no-op. The guard's
  logic is platform-correct (uses `symlink_metadata` which surfaces junctions).
- **Codex `toml_edit` drift hash**: the managed-state hash for Codex config
  entries uses the JSON-form fingerprint (not the raw TOML bytes), because
  `toml_edit` formatting can vary. Doctor's drift detection compares the live
  config entry's JSON-derived hash to the recorded hash. A formatting-only
  change (no semantic change) would not be detected as drift; a semantic change
  (e.g., `command = "other"`) would be.
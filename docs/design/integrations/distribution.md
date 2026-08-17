# Distribution and Installation

## Repository roles

MINE operates in two repository roles with different release gates:

### A. Generic MINE-managed repository

A user repository whose product is unrelated to MINE. It contains its own source code, Design, Plans during development, product-specific quality gates, and release candidate. It does **not** need to ship or verify MINE's plugin distribution assets, Skills, or bootstrap installers.

### B. MINE source repository

The repository that develops and distributes the MINE executable, root Skills, generated plugin copies, bootstrap installers, release assets, and MCP implementation. It has additional project-local release gates: root/generated Skill synchronization, embedded payload verification, Agent installation smoke tests, MCP discovery tests, bootstrap tests, and release artifact packaging.

These extra gates belong to MINE-local Design and governance (`AGENTS.md`, CI), not to portable Skills or generic release preflight. The generic `mine release` preflight must not require `skills/` or `plugins/mine/skills/` to exist; a repository without them is a valid generic repository, not a failed distribution.

## Source-of-truth rule

Repository-root `skills/` is the only hand-edited Skill source. Claude/Codex plugin directories and embedded binary payloads are generated or synchronized artifacts.

## Claude Code

Provide both:

- standalone installation for short commands such as `/mine-arch`;
- a versioned Claude marketplace plugin whose Skills are namespaced, for example `/mine:mine-arch`.

Marketplace layout follows current Claude Code requirements:

```text
.claude-plugin/marketplace.json
plugins/mine/.claude-plugin/plugin.json
plugins/mine/skills/<skill>/SKILL.md
```

The plugin directory is self-contained because Claude Code copies installed plugins into a cache.

## Codex

Provide:

- shared Agent Skills installation into a currently supported real directory;
- Codex plugin/marketplace metadata when the current stable Codex release validates and exposes it;
- a standalone Skill fallback because plugin support may evolve independently from Skill discovery.

Codex plugin installation is not declared complete until the actual client can discover the Skills and MCP server. Configuration presence alone is insufficient.

## Pi

The repository remains a Pi package with root `skills/` exposed through `package.json` or conventional discovery. Installation supports Git package sources. Pi invokes Skills with `/skill:<name>`.

Pi does not have MCP in its minimal core. MINE supports either:

- a user-installed MCP-capable Pi extension/adapter; or
- JSON CLI fallback from the Skills.

No duplicate TypeScript implementation of graph rules is allowed.

## OpenCode

Install Skills into one supported location only, preferring a shared Agent Skills or Claude-compatible directory. Configure the local stdio MCP server using OpenCode's current MCP configuration or CLI.

Do not publish an npm OpenCode plugin unless MINE later needs OpenCode-specific hooks or tools beyond Skills and MCP.

## `mine agent install`

The installer:

- supports only the four named harnesses;
- offers `--dry-run`;
- backs up structured configuration before mutation;
- records managed files and hashes;
- preserves unrelated configuration;
- refuses same-name conflicting entries unless forced;
- validates actual Skill discovery where possible;
- does not claim success merely because files were copied;
- never reads real process environment variables (`CLAUDE_CONFIG_DIR`,
  `CODEX_HOME`, `PI_HOME`, `OPENCODE_CONFIG_DIR`) when an explicit
  `--config-root` is supplied.

### Mandatory configuration backup before mutation

Every structured external configuration file (an Agent's MCP settings file:
  Claude Code `~/.claude.json`, Codex `~/.codex/config.toml`, OpenCode
  `~/.config/opencode/opencode.json`; Pi has no MCP configuration) must be
backed up before its first mutation by an install or update:

- back up the **exact original bytes**, not a parsed/reserialized
  representation;
- create the backup **before** replacing or rewriting the configuration;
- never overwrite an existing backup silently (deterministic MINE-owned backup
  location; a repeated install reuses/verifies the existing backup rather than
  clobbering it);
- record the backup path (and a hash of the original bytes) in the managed
  installation state / transaction record;
- preserve safe file permissions where applicable;
- if backup creation or verification fails, perform **no** external mutation;
- verify the backup can be read and matches the original bytes.

Backup does not excuse unnecessary formatting destruction: a config editor
must preserve comments, ordering, and unrelated formatting where feasible. For
  TOML configuration (Codex), use a format-preserving editor (the project-
  approved `toml_edit` path) rather than a full parse/reserialize round trip
  that drops comments and formatting.

### Transactional installation and recovery

"Write managed state last" is not transactionality. Installation is a bounded
transaction with **preflight, staging, commit, rollback, and recovery**:

**Before any external mutation (preflight):** resolve and validate every
destination; inspect collisions and ownership; parse and validate configuration;
determine every planned payload and configuration change; create and verify
required backups; create a durable MINE-owned pending transaction record.

**Stage** new payload and configuration content without exposing partial
final state (write to MINE-owned staging paths or record the planned changes
in the pending transaction, not directly into final user-visible locations
where a crash would leave orphans).

**On success (commit):** commit the approved changes; verify installed payload
hashes and configuration entries; atomically write final managed state; remove
the pending transaction record only after final verification succeeds.

**On any failure (rollback):** restore modified structured configuration from
the verified backup; remove only files created by the current transaction;
restore previously managed files when updating an existing installation;
preserve all unrelated and user-owned content; leave either a completely
restored state or a durable recoverable transaction record.

A later install/update/doctor invocation must **detect an incomplete
transaction** and deterministically recover or report an actionable recovery
state. Installation must never leave orphaned payload files that permanently
block every retry with `MINE_AGENT_COLLISION`. No unrestricted `--force`
deletion mechanism is introduced; recovery is bounded to MINE-owned resources
proven through the pending transaction record and managed state.

### Explicit configuration-root isolation

When `--config-root <path>` is supplied, it is the **complete authority** for
Agent configuration discovery. In explicit-root mode the installer:

- does **not** read or honor real `CLAUDE_CONFIG_DIR`, `CODEX_HOME`,
  `PI_HOME`, or `OPENCODE_CONFIG_DIR`;
- does **not** fall back to the real HOME or platform configuration directories;
- derives all Agent paths only from the injected root and approved
  deterministic subpaths (`.claude`, `.codex`, `.agents`, `.pi`,
  `.config/opencode`).

Real process/environment discovery and explicitly-isolated configuration roots
use **separate construction paths** that are never mixed and then partially
overridden. Tests that exercise poisoned environment variables must not mutate
the global process environment of an in-process parallel test (the crate forbids
`unsafe`); they use child processes or another isolation-safe mechanism.

## Embedded payload

Release binaries embed the five Skill directories so standalone installation does not require a Git checkout. Build-time verification ensures embedded content matches root `skills/`.

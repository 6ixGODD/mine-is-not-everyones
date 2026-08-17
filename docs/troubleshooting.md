# MINE Troubleshooting

Practical fixes for common problems. Every command below matches the current
MINE CLI. Each entry uses a consistent structure: **Symptom / Cause / Check /
Fix**.

---

## `mine: command not found`

**Symptom:** your shell cannot find the `mine` binary.

**Cause:** the binary was not installed, or the install directory is not on
`PATH`.

**Check:** `which mine` (Unix) or `Get-Command mine` (PowerShell). Confirm the
install directory exists: `%LOCALAPPDATA%\Programs\mine` (Windows) or
`~/.local/bin` (Linux/macOS).

**Fix:** re-run the bootstrap installer for your platform (see the README),
then reopen your terminal. Verify with `mine --version`.

---

## Agent not detected during `mine setup`

**Symptom:** `mine setup` does not find or install for your coding agent.

**Cause:** the agent's configuration directory is in a non-default location,
or the agent is not one of the four supported clients (Claude Code, Codex, Pi,
OpenCode).

**Check:** `mine agent status` lists managed installations. Confirm the agent
uses one of the supported config locations (e.g. `CLAUDE_CONFIG_DIR`,
`CODEX_HOME`, `PI_HOME`, `OPENCODE_CONFIG_DIR`).

**Fix:** set the relevant environment variable and re-run
`mine setup --agents <slug>` (e.g. `--agents claude-code,codex`). Use
`--config-root <path>` for an isolated install.

---

## Skill not visible after setup

**Symptom:** the MINE Skills do not appear in your agent after `mine setup`.

**Cause:** the agent needs a restart, or it caches discovered Skills.

**Check:** `mine agent status` reports the agent healthy. Confirm the Skill
directory exists under the agent's config root (e.g.
`~/.claude/skills/mine-arch/SKILL.md`).

**Fix:** restart the agent session. If Skills are still missing, re-run
`mine setup --agents <slug>` for that agent.

---

## MCP not registered / not visible

**Symptom:** MCP tools (e.g. `mine_plan_*`) are unavailable in the agent.

**Cause:** the MCP configuration was not merged, or the agent has not picked
it up.

**Check:** `mine agent config <slug>` previews the MCP entry MINE would merge.
`mine mcp serve` must run successfully on its own.

**Fix:** re-run `mine setup` for the agent, then restart the agent so it
reloads MCP configuration.

---

## `mine init` with an existing non-MINE `docs/design/`

**Symptom:** the repository already has a `docs/design/` directory that is not
MINE-managed.

**Cause:** legacy documentation occupies the MINE-owned namespace.

**Check:** look for `docs/design/.mine-design.toml`. If absent, the tree is
unmarked.

**Fix:** this is expected. `mine init` moves the legacy directory aside to
`docs/design-backup-<UTC timestamp>/` and creates a fresh managed root. The
legacy contents are preserved in the backup.

---

## Ownership mismatch / foreign Design marker

**Symptom:** `mine doctor` reports a design ownership mismatch.

**Cause:** `docs/design/.mine-design.toml` records a repository ID that does
not match the one in `.mine/config.toml` (e.g. the design tree was copied from
another repository).

**Check:** `mine doctor --format json` shows which check failed and why.

**Fix:** this is a deliberate refusal - MINE does not silently adopt a foreign
marker. Reinitialize the design namespace deliberately
(`mine design backup` first to preserve content, then re-run `mine init`), or
restore the correct marker.

---

## `mine doctor` reports graph not initialized

**Symptom:** the `graph` check fails with "graph not initialized/invalid".

**Cause:** the repository is a stable tree (no `docs/plan/` workspace - this is
correct on a stable branch), or a development tree whose graph has not been
opened yet.

**Check:** `mine graph status --format json`. If the current branch is the
stable branch, a missing `docs/plan/` is expected and reported as "not
applicable".

**Fix:** on a development branch, open the workspace:
`mine workspace open --format json`. On a stable tree, no action is needed.

---

## Why `mine agent status` differs from `mine doctor`

**Symptom:** the two commands report different things.

**Cause:** they operate at different levels.

**Check:** `mine agent status` is **machine-level** (installed Agent
integrations, independent of any repository). `mine doctor` is
**repository-aware** (checks `.mine/config.toml`, design, graph, Git branch
inside an initialized repository).

**Fix:** use `mine agent status` after a machine-level install; use
`mine doctor` inside a repository.

---

## Plan remains `BLOCKED`

**Symptom:** a Plan stays `BLOCKED` and cannot start.

**Cause:** not all hard predecessors are `ACCEPTED`, or the Plan has not been
released.

**Check:** `mine plan show --id <id> --format json` shows the status and
`hard_predecessors`.

**Fix:** accept (or reject with compensation) the outstanding predecessors;
then the Plan becomes `READY` (or is released explicitly with
`mine plan release --id <id>`).

---

## Release preflight failure

**Symptom:** `mine release --format json` reports `can_release: false`.

**Cause:** one or more release gates failed (non-terminal plans, rejected
plans without compensation, invalid design/graph, dirty tree, pending Agent
transactions, or plan artifacts/design backups on the stable branch).

**Check:** inspect the `errors` array in the preflight envelope.

**Fix:** resolve each reported gate, then re-run the preflight.

---

## Stale/missing final sync evidence

**Symptom:** release closure (Phase B) refuses to proceed.

**Cause:** the final `mine-sync prepare this repository for stable release`
(Phase A) has not run, or its evidence is stale (the dev HEAD moved after the
sync report).

**Check:** the Phase A report under `.mine/runtime/sync/`; compare the
recorded commit with the current `dev` HEAD.

**Fix:** re-run the final sync, then re-invoke
`mine-plan-review complete release closure`.

---

## How to safely rerun setup

**Symptom:** an install is incomplete or you want to add another Agent.

**Fix:** `mine setup` is idempotent and transactional: it backs up
configuration before mutation and recovers from incomplete transactions. Re-run
`mine setup --agents <list>` or without arguments for the interactive flow.
`mine doctor --agents all` inside an initialized repository shows per-Agent
health.

---

## How to update

**Fix:** `mine update` updates the binary to the latest release (skip the
prompt with `--yes`). Verify with `mine --version`.

---

## How to uninstall

**Fix:** `mine uninstall` removes MINE from all agents and this machine (skip
the prompt with `--yes`). It removes only MINE-managed files; unrelated agent
configuration is preserved.

---

## Windows scanner / shell expectations

**Symptom:** a stale-plan-reference scan fails on Windows, or error messages
mention WSL or `bash`.

**Cause (historical):** the scanner used to be a Bash helper, and Windows
could resolve `bash` to the WSL shim.

**Check:** `mine scan plan-refs --check --format json` is the native
cross-platform scanner. It has no Bash/WSL/Git Bash dependency.

**Fix:** use `mine scan plan-refs` for the release scan. The legacy Bash
helper (`references/scan-plan-refs.sh` in installed Skill directories) is a
manual Unix-only compatibility helper, not the authoritative implementation.

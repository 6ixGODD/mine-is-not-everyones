# MINE User Guide

## What the user must remember

For normal use, the repository owner needs only one CLI command and five agent Skills:

```text
mine init

mine-arch
mine-sync
mine-plan-create
mine-plan-exec
mine-plan-review
```

The remaining CLI surface exists mainly for Skills, diagnostics, and advanced inspection.

## `mine init` is setup, not execution

Run once at a repository root:

```bash
mine init
```

It performs deterministic setup only:

- discovers the Git root and stable branch;
- creates `.mine/config.toml` and ignored runtime directories;
- installs or validates the MINE design namespace marker;
- creates an empty modular `docs/design/` scaffold when absent;
- creates or updates MINE governance in `AGENTS.md`;
- configures supported agent integration where authorized;
- initializes the MINE code-repository version to the existing managed value, an explicit repository version when reliable, or `0.1.0` otherwise;
- validates the resulting setup.

It does **not** scan source code, write architecture, create `docs/plan/`, create development branches, invoke an agent, write business code, commit, merge, or publish.

## Design namespace handling

MINE owns this path:

```text
docs/design/
```

A managed tree contains:

```text
docs/design/.mine-design.toml
```

If a repository already stores unrelated documentation in `docs/design/`, `mine init` moves the existing directory aside to `docs/design-backup-<UTC timestamp>/` and creates a fresh MINE-managed design root. The legacy contents are preserved in the backup; MINE does not guess how to migrate arbitrary legacy layouts, but it does not abort the initialization either.

## New repository workflow

```bash
mine init
```

Then open Claude Code, Codex, Pi, or OpenCode and invoke:

```text
mine-arch <requirements>
```

`mine-arch` scans the relevant repository state, researches external contracts, and creates or updates modular target design. It may create the managed `dev` branch under the standing Git authorization defined by MINE governance.

When design is ready:

```text
mine-plan-create
mine-plan-exec <plan path>
mine-plan-review <plan path>
```

Repeat execution and review until every plan is accepted.

## Existing repository workflow

### 1. Resolve the namespace

If `docs/design/` contains legacy non-MINE documentation, `mine init` backs it up to `docs/design-backup-<timestamp>/` and creates a fresh managed root.

### 2. Initialize MINE

```bash
mine init
```

### 3. Build a code-accurate design baseline

Invoke:

```text
mine-sync
```

For large repositories, provide a scope:

```text
mine-sync synchronize the payment domain, order persistence, and webhook delivery paths
```

The agent begins with user-named paths, services, packages, symbols, or subsystems, then follows their direct dependencies and externally visible contracts.

With no scope, the agent is authorized to explore broadly until it can represent the repository accurately. This may be expensive. MINE treats an unscoped request as the user's acceptance of that cost.

### 4. Evolve the architecture

After the baseline reflects current code:

```text
mine-arch <new requirements>
```

`mine-arch` is requirement-first. It can intentionally make the target design differ from current implementation. `mine-plan-create` then plans the transition.

## What `mine-sync` does

When a managed design tree exists, `mine-sync`:

1. creates `docs/design-backup-<timestamp>/`;
2. copies the current design tree without following repository-external symlinks or junctions;
3. writes `*` to the backup directory's `.gitignore` so the backup remains local;
4. inventories the requested code scope, or explores freely when no scope is provided;
5. compares code, schemas, configuration, runtime behavior, tests, and public contracts with design;
6. applies this authority order:
   - explicit current user instructions and protected design decisions;
   - current observable repository behavior;
   - existing design only where code does not determine the answer;
7. updates the modular design tree and indexes;
8. reports suspicious code, uncertainty, and incomplete coverage without pretending they do not exist;
9. validates links, markers, document sizes, and design ownership.

When no meaningful design exists, `mine-sync` creates a descriptive baseline from the current codebase.

`mine-sync` does not modify business code unless the user separately requests implementation work.

## Temporary branches and plans

MINE uses:

```text
stable branch              released code and docs/design only
dev                        temporary integration branch
plan/<id>-<slug>            temporary implementation branch
docs/plan/                  temporary plan workspace
```

The repository owner grants MINE Skills standing authorization to:

- create and switch the managed `dev` and `plan/*` branches;
- commit files belonging to the active plan;
- merge an independently accepted plan branch into `dev`;
- delete an accepted, merged local `plan/*` branch;
- perform the final squash or curated release integration when every gate passes;
- delete the temporary local `dev` branch after release.

The authorization does not include arbitrary branches, force push, `reset --hard`, `git clean`, blind stash, public-history rewriting, or discarding unrelated changes.

The user does not manually provide a development-cycle version. `mine-plan-create` opens an internal workspace with a generated identifier. Repository version is decided at release time from accepted changes and existing MINE version state.

## Release closure

Release closure has two phases owned by different actors.

### Phase A - final design reconciliation (repository owner)

After all plans are accepted and integrated into `dev`, invoke a final full-repository sync:

```text
mine-sync prepare this repository for stable release
```

This reconciles accepted implementation into `docs/design/`, resolves or reports every incomplete area, and validates the complete repository. It is a separate, deliberate session.

### Phase B - mechanical release closure (mine-plan-review)

After the final sync, the reviewer performs the mechanical closure:

1. confirm the final `mine-sync` has completed (the reviewer does not run it);
2. run `mine release --format json` preflight plus the repository's own decisive validation;
3. determine the next MINE code-repository version;
4. safely purge the MINE-owned `docs/plan/` workspace;
5. verify the stable release tree contains no plan files or local backups;
6. integrate accepted state into the stable branch without importing temporary plan history;
7. delete temporary managed branches.

The reviewer never pushes, creates a remote release, or publishes a package. Remote publication remains explicitly outside MINE's authority.

The stable tree retains code and `docs/design/`, not the process used to create them.

## Installation and lifecycle

After bootstrap installation (see the README), these CLI commands manage the MINE lifecycle:

```bash
mine --version          # verify the installed binary
mine doctor --agents all  # show managed agent integrations and health
mine setup              # (re)install MINE into coding agents (interactive)
mine setup --agents claude-code,codex --yes  # non-interactive, specific agents
mine update             # update the binary to the latest release
mine uninstall         # remove MINE from all agents and this machine
```

`mine setup` is **global machine setup** (installs Skills and MCP config into agent client directories). `mine init` is **repository-local** (creates `.mine/config.toml`, the design namespace, and governance at a repository root). Run `mine setup` once per machine; run `mine init` once per repository.

Non-interactive flags: `--agents <list>` (comma-separated slugs), `--yes` (skip prompts), `--config-root <path>` (isolated install for CI/tests).

## Manual inspection commands

These are useful but not required for normal flow. All of these accept `--format json` for stable machine-consumable envelopes (and `--repo <path>` to target a different repository root); this guide shows the human form.

```bash
mine status
mine doctor
mine design status
mine design validate
mine graph status
mine graph ready
mine graph wave
mine plan show --id <id>
```

Agent-facing mutations go through the accepted `mine` CLI subcommands (`mine plan add|start|implemented|accept|reject`, `mine graph render`, `mine workspace open|status|close`, `mine design backup|validate|status`, `mine repository version show|suggest|set`), all with `--format json`. When a typed MCP bridge is accepted, prefer it and fall back to `--format json` CLI. Never edit `docs/plan/execution-graph.toml` or `docs/plan/execution-graph.md` directly.

## Supported clients

Only Claude Code, Codex, Pi, and OpenCode are supported. Other environments are intentionally excluded from MINE's compatibility burden.

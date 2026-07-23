# MINE

> **MINE Is Not Everyone’s.**

MINE is a personal, destructive-by-design engineering workflow for Claude Code, Codex, Pi, and OpenCode.

It is built for one repository owner who is willing to let coding agents make strong, explicit decisions under strict repository governance. It is not a compatibility framework, a migration assistant, or a polite layer that preserves every historical document and convention it encounters.

**You may use it. That does not mean your repository is ready for it.**

## The MINE philosophy

MINE practices document-driven development, but it does not worship stale documents.

- `docs/design/` is MINE-owned architectural state.
- `mine-sync` reconciles that state with the real repository.
- Unless the user explicitly protects a design decision, current code, schemas, configuration, and observable runtime behavior win during synchronization.
- Before rewriting design, MINE creates a local ignored backup under `docs/design-backup-<timestamp>/`.
- Temporary plans, execution graphs, implementation reports, and review reports exist only on `dev` and `plan/*` branches. They do not survive stable release integration.
- MINE does not preserve unsupported legacy design layouts, obsolete internal contracts, or accidental compatibility debt.

“Destructive” means MINE is allowed to replace inaccurate design documentation and discard temporary development process artifacts. It does **not** mean arbitrary filesystem deletion, destructive Git recovery, or unbounded shell execution.

## MINE owns `docs/design/`

A MINE-managed design tree contains:

```text
docs/design/.mine-design.toml
```

If an existing repository already uses `docs/design/` for unrelated or legacy documentation, rename or remove that directory before running `mine init`. MINE intentionally does not attempt to infer or preserve arbitrary historical layouts.

A conflicting unmarked `docs/design/` causes initialization to stop with a namespace-conflict error.

## Five Skills

```text
mine-arch
mine-sync
mine-plan-create
mine-plan-exec
mine-plan-review
```

- `mine-arch`: requirement-first architecture creation and evolution.
- `mine-sync`: code-first repository-to-design synchronization.
- `mine-plan-create`: immutable ephemeral plan creation.
- `mine-plan-exec`: governed implementation of one plan.
- `mine-plan-review`: independent acceptance or rejection.

Everything deterministic—initialization, graph state, validation, locking, installation, diagnostics, and distribution—belongs to the Rust `mine` executable.

## Supported environments

Only these environments are first-class:

- Claude Code;
- Codex;
- Pi;
- OpenCode.

Cursor, Windsurf, Cline, and other harnesses are intentionally out of scope. Their users may maintain their own forks.

## Quick start

### New repository

```bash
mine init
```

Then open a supported coding agent and invoke:

```text
mine-arch <your requirements>
```

`mine init` only establishes MINE-owned files, markers, configuration, templates, and agent integration. It does not scan the repository, write architecture, create plans, start agents, or implement code.

### Existing repository

If the repository already contains a non-MINE `docs/design/`, rename or delete it first.

```bash
mine init
```

Then establish a code-accurate baseline:

```text
mine-sync
```

For a large repository, provide a scope when possible:

```text
mine-sync the authentication, authorization, and identity persistence subsystems
```

With no scope, the agent is authorized to explore the repository broadly. The resulting token and runtime cost is accepted by the user who requested an unscoped sync.

After synchronization, evolve the system through:

```text
mine-arch <new requirement>
mine-plan-create
mine-plan-exec
mine-plan-review
```

## Durable versus temporary state

Stable branches retain:

```text
code
tests
configuration
README.md
README.zh-CN.md
docs/design/
.mine/config.toml
AGENTS.md
```

Stable branches must not retain:

```text
docs/plan/
execution graphs
implementation reports
review reports
temporary design backups
dev
plan/*
```

See [the documentation index](docs/README.md) and [the user guide](docs/user-guide.md).

## Language

- `README.md`: English source.
- `README.zh-CN.md`: Chinese translation.
- `docs/**`: English only.

## Warning

MINE assumes that repository owners accept strong conventions and are willing to let agents restructure design documentation, create managed branches, commit scoped changes, merge accepted plan branches into `dev`, and remove temporary MINE artifacts at release closure.

It refuses arbitrary `reset --hard`, `git clean`, blind stash operations, force pushes, unbounded shell deletion, and writes outside the repository.

MINE is opinionated because ambiguity is expensive.

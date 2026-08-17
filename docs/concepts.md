# MINE Concepts

This page explains the core ideas behind MINE in user-facing terms. It is
deliberately concise; readers who want implementation or design internals
should follow the [Design index](design/index.md).

## What MINE is

MINE is an opinionated, document-driven engineering workflow for coding
agents. It keeps architecture, planning, implementation, review, and release
closure in the repository itself, constrained by deterministic tools and
versionable, traceable state. One Rust binary (`mine`) handles the
deterministic parts; five agent Skills (`mine-arch`, `mine-sync`,
`mine-plan-create`, `mine-plan-exec`, `mine-plan-review`) handle workflow and
engineering judgment.

## Design as durable engineering knowledge

`docs/design/` is the durable, MINE-owned design knowledge base. It survives
releases and exists on the stable branch. It describes the accepted
architecture of the branch on which it is read. Design changes precede code
changes: you update the target design first, then plan the implementation.

## Plans as temporary execution work packages

A Plan is a temporary, executable work package: a precise contract describing
what to build, why, and how to verify it. Plans live under `docs/plan/`
during development and are tracked in an execution graph with statuses
(DRAFT, READY, IN_PROGRESS, IMPLEMENTED, ACCEPTED, REJECTED).

## Why Plans become immutable after execution

Once a Plan is handed to an implementation agent or execution begins, it
becomes immutable. Changing the contract mid-execution would invalidate the
implementation, its review, and downstream work that depends on it. If the
Plan proves wrong, the correct response is a new compensating Plan, not
editing the old one.

## Stable / dev / plan branch roles

- **Stable branch** (`main` or `master`): released code and `docs/design/`
  only. No `docs/plan/`, no temporary process state.
- **`dev`**: temporary integration branch that owns the active `docs/plan/`
  workspace and receives independently accepted Plan branches.
- **`plan/<id>-<slug>`**: short-lived implementation branch for one Plan,
  merged into `dev` only after independent acceptance.

## Why implementation and review are separated

An implementing agent commits scoped work and reports evidence but never
grants itself acceptance. An independent reviewer tries to falsify the
implementation's claims before accepting. This separation keeps acceptance
meaningful.

## Why `docs/plan/` disappears from stable releases

The planning workspace is process, not product. Stable releases contain
accepted product state and durable design - not the temporary plans, reports,
and graph that produced them. Release closure purges `docs/plan/` and
integrates the stable tree through squash or curated commits so temporary
history is not imported.

## Why MINE synchronizes before release

Before release, `mine-sync` reconciles accepted implementation into
`docs/design/` so the durable design matches the code that was actually
built. This final synchronization (Phase A) is a separate, deliberate
session; the mechanical release closure (Phase B) follows it.

## Normal engineering changes vs lightweight maintenance

MINE governs engineering change, not every repository edit.

- **Lightweight maintenance** (typos, prose cleanup, translation, broken
  links, README improvements, formatting-only changes, examples/comments
  describing already-accepted behavior) may be done directly, without a
  Plan, when the correct result is unambiguous and no durable engineering
  contract changes.
- **Normal engineering changes** (behavior, architecture, public API, CLI
  semantics, MCP contracts, Skill workflow, execution-graph/release/branch
  behavior, persistence/schema, security/privacy boundaries, deployment
  contracts, durable Design decisions) use the full
  Design → Plan → Execute → Review lifecycle.

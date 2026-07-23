# Plan 03: CLI, JSON, rendering, Git evidence, design backup, and workspace lifecycle

## Status

`BLOCKED`

## Goal

Implement the human and JSON CLI, deterministic graph rendering, read-only and managed-branch Git evidence, setup/status commands, internal workspace open/status/close, version-independent workspace identity, safe design backup, design validation, and guarded plan-workspace purge.

## Branch contract

- Stable branch: the branch detected by `mine init` (currently `master` for this repository).
- Integration branch: managed `dev`.
- Implementation branch: `plan/03-cli-json-rendering-git-and-workspace-lifecycle`.
- Never implement directly on the stable branch or `dev`.
- The user grants standing authorization to create/switch the managed branch, commit scoped files, and let an independent accepted review merge it into `dev`.
- Do not force push, reset hard, clean, blindly stash, rewrite public history, or discard unrelated changes.
- This plan and its reports are ephemeral and must not survive stable release integration.

## Hard predecessors

02-1

Note: originally `02`. `docs/plan/02-execution-graph-domain-and-persistence.md` was independently reviewed and `REJECTED` (see `docs/plan/reports/02-execution-graph-domain-and-persistence-review.md`) for an undisclosed `unsafe` file-locking implementation that violates `AGENTS.md`'s "Business code must not use `unsafe`" rule. `docs/plan/02-1-execution-graph-safe-file-locking.md` is the compensating plan; this predecessor edge was rerouted before Plan 03 execution began, per the reviewer's compensation routing. This plan had not started execution at the time of the edit, so it is not yet immutable.

## Governing design references

- [`docs/design/interfaces/cli-contract.md`](../design/interfaces/cli-contract.md)
- [`docs/design/execution-graph/state-machine-and-algorithms.md`](../design/execution-graph/state-machine-and-algorithms.md)
- [`docs/design/execution-graph/persistence-and-concurrency.md`](../design/execution-graph/persistence-and-concurrency.md)
- [`docs/design/governance/branch-and-plan-lifecycle.md`](../design/governance/branch-and-plan-lifecycle.md)
- [`docs/design/governance/design-knowledge-base.md`](../design/governance/design-knowledge-base.md)

The executor reads the exact documents before mutation. Required design change precedes implementation; immutable plans are not silently expanded.

## Scope ownership

### Exclusive write paths

- `src/cli/`
- `src/output/`
- `src/render/`
- `src/infrastructure/git.rs`
- `src/infrastructure/design_backup.rs`
- `src/application/workspace_service.rs`
- `src/application/init_service.rs`
- `tests/cli/`
- `tests/golden/`

### Reserved shared paths

- `docs/plan/execution-graph.toml`
- `docs/plan/execution-graph.md`
- files owned by other active plan branches

### Read-only context

- `REQUIREMENTS.md`
- non-target `docs/design/` documents
- predecessor reports and commits

## Required work packages

1. Freeze CLI and JSON envelope from the design contract and actual implementation.
2. Implement `mine init`, `mine status`, design marker validation, and repository version initialization.
3. Implement `mine workspace open|status|close` with generated internal workspace ID and no user release-version input.
4. Implement `mine design backup` with UTC path, `.gitignore` containing `*`, repository-bound link handling, copy verification, and no mutation before success.
5. Implement graph, plan, design validation/status, Markdown rendering, Git evidence, and safe purge.
6. Add deterministic human/JSON output, dry-run, structured errors, and exit codes.
7. Add tests for Windows paths, legacy namespace conflict, backup failure, external links, workspace identity, and release hygiene.

## Verification

```text
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
mine design validate
mine graph validate
```

Run narrower and platform-specific checks required by scope. Missing tools, skipped checks, timeouts, incomplete repository exploration, and non-zero exits are not passes.

## Acceptance criteria

- normal users can initialize without supplying a version or cycle ID;
- workspace identity is generated and distinct from repository version;
- design backup cannot escape the repository and blocks mutation on failure;
- legacy/foreign design roots return stable errors;
- generated graph output is deterministic;
- purge deletes only ownership-marked `docs/plan/`;
- Plan reaches `IMPLEMENTED`.

## Report path

`docs/plan/reports/03-cli-json-rendering-git-and-workspace-lifecycle-implementation.md`

## Downstream release

On independent acceptance, release Plans 04 and 05 in parallel.

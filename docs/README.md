# MINE Documentation

## Start here

Pick the entry point for your goal:

- **[Using MINE](user-guide.md)** — install, initialize, run the workflow day to day.
- **[Design index](design/index.md)** — understand or develop MINE itself; the durable architecture source of truth.
- **[Installation & lifecycle](user-guide.md#installation)** — `mine setup`, `mine update`, `mine uninstall`, `mine --version`.
- **[Release & governance](design/governance/branch-and-plan-lifecycle.md)** — branch roles, plan lifecycle, and release closure.

MINE documentation has two deliberately different lifecycles.

## Long-lived design knowledge

`docs/design/` is the durable, MINE-owned design knowledge base. It survives releases and is present on the stable branch. It describes the accepted architecture of the branch on which it is read.

Start with [`design/index.md`](design/index.md). Follow indexes and open only the leaf documents relevant to the current task.

A managed design tree must contain `docs/design/.mine-design.toml`. When `mine init` finds an existing unmarked `docs/design/`, it moves the legacy directory aside to a timestamped `docs/design-backup-<UTC timestamp>/` backup and creates a fresh managed root; it does not abort. `mine-sync` refuses an unmarked or foreign-owned `docs/design/` only after `mine init` has established the managed namespace.

## Local design backups

Before `mine-sync` rewrites an existing managed design tree, it creates:

```text
docs/design-backup-<UTC timestamp>/
```

The backup is local-only and ignored by Git. It exists to recover from a bad synchronization run, not as architectural history.

## Ephemeral planning workspace

`docs/plan/` is a temporary workspace created by `mine-plan-create` on `dev`. It contains immutable plans, the execution graph, temporary reports, and parallel-execution coordination.

It must not exist in the final tree or stable-history integration of the release branch. Before release:

1. all implementation plans are independently accepted;
2. `mine-sync` reconciles accepted code into durable design;
3. required release checks pass;
4. `docs/plan/` is safely purged;
5. accepted product state is integrated into the stable branch through squash or curated commits;
6. temporary `dev` and `plan/*` branches are deleted.

See [`design/governance/branch-and-plan-lifecycle.md`](design/governance/branch-and-plan-lifecycle.md).

## Language policy

All files under `docs/` are English. The repository root contains an English `README.md` and a Chinese `README.zh-CN.md`.

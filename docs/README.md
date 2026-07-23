# MINE Documentation

MINE documentation has two deliberately different lifecycles.

## Long-lived design knowledge

`docs/design/` is the durable, MINE-owned design knowledge base. It survives releases and is present on the stable branch. It describes the accepted architecture of the branch on which it is read.

Start with [`design/index.md`](design/index.md). Follow indexes and open only the leaf documents relevant to the current task.

A managed design tree must contain `docs/design/.mine-design.toml`. An existing unmarked `docs/design/` is a namespace conflict, not a migration opportunity. Rename or remove legacy content before `mine init`.

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

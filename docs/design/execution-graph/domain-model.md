# Execution Graph Domain Model

## Aggregate

`PlanWorkspace` is the aggregate root.

```text
PlanWorkspace
├── schema_version
├── revision
├── project_id
├── workspace_id
├── stable_branch
├── integration_branch
├── stable_baseline_commit
├── design_root
├── created_at
├── updated_at
└── plans[]
```

`workspace_id` is generated internally. It is not a product version and the user does not provide it.

## Plan node

Each `PlanNode` contains:

- `id`: stable within the active workspace;
- `path`: plan Markdown path under `docs/plan/`;
- `title` and status;
- hard and soft predecessors;
- exact design references with anchors;
- exclusive, read-only, and reserved shared paths;
- implementation and review report paths;
- implementation commits;
- owner, run ID, timestamps, rejection, and compensation metadata.

## Safe paths

All graph paths are normalized repository-relative UTF-8 strings using `/` separators.

Rejected forms include absolute paths, empty paths, `..` traversal, repository-escaping symlinks/junctions, broad wildcard ownership patterns, and platform drive roots.

## Design references

Every plan references at least one design leaf. Referencing only `docs/design/index.md` is insufficient unless the plan exclusively changes top-level scope.

```toml
path = "docs/design/interfaces/cli-contract.md"
anchors = ["#json-output", "#error-contract"]
reason = "Defines the public contract implemented by this plan"
```

Plan creation fails when paths or anchors do not exist.

## Release closure evidence

Before workspace closure, the graph points to final accepted states, latest full `mine-sync` report, approved exceptions, release verification, and suggested repository version. These records are temporary and are purged with `docs/plan/`.

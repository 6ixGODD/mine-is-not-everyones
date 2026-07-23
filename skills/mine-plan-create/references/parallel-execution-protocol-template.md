# Parallel Execution Protocol

Create `docs/plan/parallel-execution-protocol.md` when any plan declares parallel implementation lanes.

## Global rules
- Every lane has one owner and a disjoint primary write scope.
- Shared files are reserved to a named serial integration owner.
- A timeout or silent agent is not permission to launch a duplicate owner; inspect existing work first.
- No lane may edit the execution graph unless explicitly designated scheduler/integrator.
- No lane may stage, unstage, commit, reset, restore, clean, or discard another lane's work.

## Reserved shared files
List repository-specific files such as manifests, lockfiles, migrations, generated schemas, OpenAPI files, generated clients, central dependency registries, root lint/test configuration, execution graph, and reports.

| File/pattern | Owner | Other lanes may | Join requirement |
|---|---|---|---|
| `<path>` | `<lane>` | read only | `<evidence>` |

## Lane contract
For every lane record:
- plan path and work-package ID;
- start gate and accepted predecessors;
- exclusive write paths;
- read-only dependencies;
- forbidden paths;
- expected commits and report fragment;
- integration requests for shared files;
- narrow verification commands;
- completion signal and join artifact.

## Integration
- Merge or integrate in dependency order.
- Resolve shared-file requests only in the integration lane.
- Run schema/code generation once after all contract-defining lanes join.
- Run the full join verification matrix and record exact results.

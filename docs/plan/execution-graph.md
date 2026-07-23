# Execution Graph

> GENERATED VIEW. Do not edit directly. The machine fact source is `execution-graph.toml`.
>
> This directory is ephemeral and must be purged before stable release integration.

- Workspace: `bootstrap-mine-v1`
- Stable branch: `master`
- Integration branch: `dev`
- Revision: `11`

| Plan | Title | Status | Hard predecessors |
|---|---|---|---|
| 01 | Repository foundation, initialization, namespace, and branch governance | ACCEPTED | - |
| 02 | Execution graph domain and persistence | REJECTED | 01 |
| 02-1 | Execution graph safe file locking (compensation for rejected Plan 02) | ACCEPTED | 01 |
| 03 | CLI, JSON, rendering, Git evidence, and workspace lifecycle | ACCEPTED | 02-1 |
| 04 | Skills JSON-CLI, mine-sync, and design lifecycle | IN_PROGRESS | 03 |
| 05 | stdio MCP server and typed tools | READY | 03 |
| 06 | Final Skill contract and plugin distribution | BLOCKED | 04, 05 |
| 07 | Four-agent installer, managed state, and doctor | BLOCKED | 06 |
| 08 | Release, bootstrap, mine-sync, and self-hosting | BLOCKED | 07 |

## Topology

```text
01 -> 02 -> 02-1 -> 03 -> 04 -> 05 -> 06 -> 07 -> 08
```

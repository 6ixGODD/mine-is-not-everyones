# Execution Graph

> GENERATED VIEW. Do not edit directly. The machine fact source is `execution-graph.toml`.
>
> This directory is ephemeral and must be purged before stable release integration.

- Workspace: `bootstrap-mine-v1`
- Stable branch: `master`
- Integration branch: `dev`
- Revision: `2`

| Plan | Title | Status | Hard predecessors |
|---|---|---|---|
| 01 | Repository foundation, initialization, namespace, and branch governance | ACCEPTED | — |
| 02 | Execution graph domain and persistence | IN_PROGRESS | 01 |
| 03 | CLI, JSON, rendering, Git evidence, and workspace lifecycle | BLOCKED | 02 |
| 04 | Skills JSON-CLI, mine-sync, and design lifecycle | BLOCKED | 03 |
| 05 | stdio MCP server and typed tools | BLOCKED | 03 |
| 06 | Final Skill contract and plugin distribution | BLOCKED | 04, 05 |
| 07 | Four-agent installer, managed state, and doctor | BLOCKED | 06 |
| 08 | Release, bootstrap, mine-sync, and self-hosting | BLOCKED | 07 |

## Topology

```text
01 → 02 → 03 ─┬→ 04 ─┐
               └→ 05 ─┴→ 06 → 07 → 08 → release closure
```

Plans 04 and 05 may execute in parallel after Plan 03 is accepted. Plan 06 is their join gate.

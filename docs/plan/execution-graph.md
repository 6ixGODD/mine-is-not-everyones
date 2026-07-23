# Execution Graph

> GENERATED VIEW. Do not edit directly. The machine fact source is `execution-graph.toml`.
>
> This directory is ephemeral and must be purged before stable release integration.

- Workspace: `bootstrap-mine-v1`
- Stable branch: `master`
- Integration branch: `dev`
- Revision: `9`

| Plan | Title | Status | Hard predecessors |
|---|---|---|---|
| 01 | Repository foundation, initialization, namespace, and branch governance | ACCEPTED | — |
| 02 | Execution graph domain and persistence | REJECTED | 01 |
| 02-1 | Execution graph safe file locking (compensation for rejected Plan 02) | ACCEPTED | 01 |
| 03 | CLI, JSON, rendering, Git evidence, and workspace lifecycle | IMPLEMENTED | 02-1 |
| 04 | Skills JSON-CLI, mine-sync, and design lifecycle | BLOCKED | 03 |
| 05 | stdio MCP server and typed tools | BLOCKED | 03 |
| 06 | Final Skill contract and plugin distribution | BLOCKED | 04, 05 |
| 07 | Four-agent installer, managed state, and doctor | BLOCKED | 06 |
| 08 | Release, bootstrap, mine-sync, and self-hosting | BLOCKED | 07 |

## Topology

```text
01 → 02 (REJECTED, see review report; implementation commits preserved on the
         unmerged plan/02-execution-graph-domain-and-persistence branch)
01 → 02-1 (ACCEPTED) → 03 (READY) ─┬→ 04 ─┐
                                     └→ 05 ─┴→ 06 → 07 → 08 → release closure
```

Plan 02 was independently reviewed and `REJECTED` (`docs/plan/reports/02-execution-graph-domain-and-persistence-review.md`) for an undisclosed `unsafe` FFI file-locking implementation in `src/infrastructure/file_lock.rs`, violating `AGENTS.md`'s "Business code must not use `unsafe`" rule. Plan 02's implementation branch was not merged into `dev` and is preserved for reference and reuse. Plan 02-1 (`docs/plan/reports/02-1-execution-graph-safe-file-locking-review.md`) is the compensating plan (`compensating_plan = "02-1"` on node `02`); it replaced the unsafe locking backend with the `fs4` crate, ported forward the rest of Plan 02's independently-verified-sound work unchanged, and is independently `ACCEPTED`. Plan 03's hard-predecessor edge is rerouted to `02-1` and is now `READY`. Plans 04 and 05 may execute in parallel after Plan 03 is accepted. Plan 06 is their join gate.

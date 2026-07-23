# MCP Contract

## Transport

v1 implements stdio only using the official Rust MCP SDK. The client launches:

```text
mine mcp serve --repo <repository>
```

stdin/stdout carry protocol messages only. Diagnostics go to stderr and local files.

## Tool design

Expose a small set of focused tools, such as:

- `mine_workspace_status`;
- `mine_graph_validate`;
- `mine_graph_status`;
- `mine_graph_ready`;
- `mine_graph_wave`;
- `mine_plan_add`;
- `mine_plan_show`;
- `mine_plan_start`;
- `mine_plan_mark_implemented`;
- `mine_plan_accept`;
- `mine_plan_reject`;
- `mine_design_validate`.

Repository initialization, design backup, workspace opening/closure, installation, and release mutations remain CLI-only because they change local environment or delete owned temporary state.

## Mutation contract

Every mutating tool requires repository identity, workspace or plan identity, expected revision, and typed arguments rather than shell fragments.

The server returns the same DTOs as JSON CLI.

## Security

The MCP server exposes no arbitrary shell execution, arbitrary deletion, unrestricted Git mutation, plugin installation, unrestricted file writes, or network fetch primitive.

## Compatibility

Tool names and schemas are validated against Skill references before release. Skills never mention tools the current binary does not expose.

# Code Organization

## Initial package strategy

v1 uses one Rust package. Split crates only after stable boundaries and a real publication or compile-time need exist.

```text
src/
├── main.rs
├── cli/
├── mcp/
├── domain/
│   ├── workspace.rs
│   ├── graph.rs
│   ├── plan.rs
│   ├── status.rs
│   ├── transition.rs
│   ├── path.rs
│   ├── design_marker.rs
│   └── validation.rs
├── application/
│   ├── init_service.rs
│   ├── workspace_service.rs
│   ├── graph_service.rs
│   ├── plan_service.rs
│   ├── design_service.rs
│   ├── design_backup_service.rs
│   ├── repository_version_service.rs
│   ├── distribution_service.rs
│   ├── agent_service.rs
│   └── doctor_service.rs
├── infrastructure/
│   ├── toml_store.rs
│   ├── atomic_write.rs
│   ├── file_lock.rs
│   ├── git.rs
│   ├── repository_locator.rs
│   ├── design_index.rs
│   ├── design_backup.rs
│   ├── embedded_skills.rs
│   └── event_log.rs
├── render/
│   └── markdown.rs
├── agent_setup/
│   ├── claude.rs
│   ├── codex.rs
│   ├── pi.rs
│   └── opencode.rs
└── output/
```

## Ownership rules

- Domain modules contain pure rules and typed errors.
- Application modules define use cases and ports.
- Infrastructure performs I/O.
- Agent configuration formats stay behind one adapter per supported harness.
- Root `skills/` is the only manually maintained Skill source.
- No module parses both CLI arguments and domain state.

## Configuration source rules

Cargo, rustfmt, clippy policy, tests, and release workflow each have one authoritative configuration source.

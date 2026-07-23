# Execution Graph Design Index

The execution graph is the deterministic lifecycle model for one active temporary plan workspace.

## Documents

- [Domain model](domain-model.md)
- [Persistence and concurrency](persistence-and-concurrency.md)
- [State machine and algorithms](state-machine-and-algorithms.md)

## Scope

The graph exists only while managed `dev` work is active. Its source and generated view live under `docs/plan/` and are deleted before stable release integration.

The graph is not a historical project-management database. Stable branches retain released code and durable design, not temporary planning state.

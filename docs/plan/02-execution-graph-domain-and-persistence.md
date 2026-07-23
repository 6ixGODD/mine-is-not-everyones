# Plan 02: Execution graph domain and persistence

## Status

`BLOCKED`

## Goal

Implement the active-workspace aggregate, plan states, design references, graph validation, TOML storage, revision control, file locking, atomic writes, and safe plan-workspace ownership markers. No CLI or MCP business rules are implemented here.

## Branch contract

- Stable branch: the branch detected by `mine init` (currently `master` for this repository).
- Integration branch: managed `dev`.
- Implementation branch: `plan/02-execution-graph-domain-and-persistence`.
- Never implement directly on the stable branch or `dev`.
- The user grants standing authorization to create/switch the managed branch, commit scoped files, and let an independent accepted review merge it into `dev`.
- Do not force push, reset hard, clean, blindly stash, rewrite public history, or discard unrelated changes.
- This plan and its reports are ephemeral and must not survive stable release integration.

## Hard predecessors

01

## Governing design references

- [`docs/design/execution-graph/domain-model.md`](../design/execution-graph/domain-model.md)
- [`docs/design/execution-graph/persistence-and-concurrency.md`](../design/execution-graph/persistence-and-concurrency.md)
- [`docs/design/execution-graph/state-machine-and-algorithms.md`](../design/execution-graph/state-machine-and-algorithms.md)

The executor must resolve and read the exact referenced documents before mutation. If implementation requires a design change, update design first and create compensation rather than silently expanding this immutable plan.

## Scope ownership

### Exclusive write paths

- `src/domain/`
- `src/application/`
- `src/infrastructure/toml_store.rs`
- `src/infrastructure/file_lock.rs`
- `src/infrastructure/atomic_write.rs`
- `tests/domain/`
- `tests/persistence/`

### Reserved shared paths

- `docs/plan/execution-graph.toml`
- `docs/plan/execution-graph.md`
- files owned by other active plan branches

### Read-only context

- `REQUIREMENTS.md`
- all non-target `docs/design/` documents
- predecessor reports and commits

## Required work packages

1. **Baseline and evidence** — inspect current branch, working tree, predecessor acceptance, relevant source, tests, and official documentation.
2. **Contract extraction** — convert the governing design sections into an implementation checklist with files, types, errors, lifecycle, and verification.
3. **Implementation** — make the smallest cohesive production changes within owned paths.
4. **Focused tests** — add deterministic tests that fail for plausible wrong implementations.
5. **Integration checks** — run affected repository quality gates and inspect generated or serialized artifacts.
6. **Skill/design consistency** — confirm no public command, MCP tool, Skill instruction, or design link was made stale.
7. **Implementation report** — write exact commands, exit codes, commits, deviations, risks, and preserved unrelated changes.

## Verification

At minimum:

```text
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
mine design validate
mine graph validate
```

Run narrower and platform-specific commands required by the implemented scope. Missing tools, skipped checks, timeouts, and non-zero exits are not passes.

## Acceptance criteria

- every governing design contract is implemented or explicitly reported as blocked;
- all writes stay within declared ownership;
- tests discriminate intended semantics from plausible wrong behavior;
- stable JSON/protocol contracts are documented where applicable;
- no direct execution-graph file editing is introduced;
- no unrelated changes or secrets are staged;
- implementation evidence is reproducible;
- the node reaches `IMPLEMENTED`, never self-granted `ACCEPTED`.

## Report path

`docs/plan/reports/02-execution-graph-domain-and-persistence-implementation.md`

## Downstream release

On independent acceptance, release: 03.

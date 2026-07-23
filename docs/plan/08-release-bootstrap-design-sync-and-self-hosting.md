# Plan 08: Release, bootstrap, design sync, and self-hosting

## Status

`BLOCKED`

## Goal

Complete cross-platform release automation, bilingual root README files, bootstrap scripts, real-client smoke tests, self-hosting of the MINE workflow, final mine-design-sync evidence, cycle closure, and proof that the stable release tree contains no docs/plan workspace.

## Branch contract

- Base branch: current accepted `dev`.
- Implementation branch: `plan/08-release-bootstrap-design-sync-and-self-hosting`.
- Never implement on `main`.
- Do not create, merge, delete, or switch branches unless explicitly authorized by the user or scheduler.
- This plan and its reports are ephemeral and must not survive stable release integration.

## Hard predecessors

07

## Governing design references

- [`docs/design/governance/design-sync.md`](../design/governance/design-sync.md)
- [`docs/design/governance/branch-and-plan-lifecycle.md`](../design/governance/branch-and-plan-lifecycle.md)
- [`docs/design/operations/testing-release-and-recovery.md`](../design/operations/testing-release-and-recovery.md)
- [`docs/design/integrations/distribution.md`](../design/integrations/distribution.md)

The executor must resolve and read the exact referenced documents before mutation. If implementation requires a design change, update design first and create compensation rather than silently expanding this immutable plan.

## Scope ownership

### Exclusive write paths

- `.github/workflows/`
- `scripts/`
- `README.md`
- `README.zh-CN.md`
- `docs/design/`
- `docs/user-guide.md`
- `tests/e2e/`

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

`docs/plan/reports/08-release-bootstrap-design-sync-and-self-hosting-implementation.md`

## Downstream release

On independent acceptance, release: Release closure.

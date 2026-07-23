# Plan 01: Repository foundation, initialization, namespace, and branch governance

## Status

`READY`

## Goal

Establish the Rust repository foundation and MINE governance. Implement deterministic `mine init` behavior, MINE ownership of `docs/design/`, stable repository identity/version initialization, standing managed-branch authorization, and the initial `mine-arch` and `mine-sync` Skill skeletons without scanning or modifying business code during init.

## Branch contract

- Stable branch: the branch detected by `mine init` (currently `master` for this repository).
- Integration branch: managed `dev`.
- Implementation branch: `plan/01-repository-foundation-and-release-branch-governance`.
- Never implement directly on the stable branch or `dev`.
- The user grants standing authorization to create/switch the managed branch, commit scoped files, and let an independent accepted review merge it into `dev`.
- Do not force push, reset hard, clean, blindly stash, rewrite public history, or discard unrelated changes.
- This plan and its reports are ephemeral and must not survive stable release integration.

## Hard predecessors

None.

## Governing design references

- [`docs/design/principles.md`](../design/principles.md)
- [`docs/design/system/code-organization.md`](../design/system/code-organization.md)
- [`docs/design/governance/design-knowledge-base.md`](../design/governance/design-knowledge-base.md)
- [`docs/design/governance/branch-and-plan-lifecycle.md`](../design/governance/branch-and-plan-lifecycle.md)
- [`docs/design/decisions/0006-mine-owns-design-namespace.md`](../design/decisions/0006-mine-owns-design-namespace.md)

The executor reads the exact documents before mutation. Required design change precedes implementation; immutable plans are not silently expanded.

## Scope ownership

### Exclusive write paths

- `Cargo.toml`
- `Cargo.lock`
- `rust-toolchain.toml`
- `src/`
- `tests/`
- `.github/`
- `AGENTS.md`
- `.mine/`
- `skills/mine-arch/`
- `skills/mine-sync/`

### Reserved shared paths

- `docs/plan/execution-graph.toml`
- `docs/plan/execution-graph.md`
- files owned by other active plan branches

### Read-only context

- `REQUIREMENTS.md`
- non-target `docs/design/` documents
- predecessor reports and commits

## Required work packages

1. Baseline and research — inspect repository state and official Rust/toolchain guidance before choosing dependencies.
2. Repository identity — define project UUID and MINE code-repository version persistence; preserve existing managed values and default unmanaged repositories to `0.1.0` when no reliable root version exists.
3. Initialization service — implement setup-only init that creates configuration, runtime ignore rules, modular design scaffold, `.mine-design.toml`, governance, and validation without code scanning, plan creation, agent invocation, branch mutation, commit, or release.
4. Namespace conflict — reject unmarked or foreign-owned `docs/design/` with structured errors; do not implement legacy migration.
5. Branch governance — write durable AGENTS rules granting bounded managed-branch authority to Skills while forbidding destructive recovery and unrelated changes.
6. Skill skeletons — create/update `mine-arch` and `mine-sync` with correct high-level responsibilities and explicit no-auto-execution behavior.
7. Tests and report — cover absent, valid, legacy, and foreign design roots plus idempotent init.

## Verification

```text
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
mine design validate
mine graph validate
```

Run narrower and platform-specific checks required by scope. Missing tools, skipped checks, timeouts, incomplete repository exploration, and non-zero exits are not passes.

## Acceptance criteria

- `mine init` is idempotent and setup-only;
- legacy `docs/design/` is rejected rather than adopted;
- MINE marker and repository identity are validated;
- no branch, plan, source scan, agent run, commit, or business-code mutation occurs;
- branch authorization is explicit and bounded;
- Plan reaches `IMPLEMENTED`, never self-granted `ACCEPTED`.

## Report path

`docs/plan/reports/01-repository-foundation-and-release-branch-governance-implementation.md`

## Downstream release

On independent acceptance, release Plan 02.

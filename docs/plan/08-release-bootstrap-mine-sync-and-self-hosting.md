# Plan 08: Release, bootstrap, mine-sync, legacy onboarding, and self-hosting

## Status

`BLOCKED`

## Goal

Complete cross-platform release automation, bilingual root README files, bootstrap scripts, actual-client smoke tests, old-repository onboarding, destructive-but-bounded MINE philosophy, final full `mine-sync`, repository-version suggestion, safe workspace purge, and proof that stable output contains neither plan artifacts nor design backups.

## Branch contract

- Stable branch: the branch detected by `mine init` (currently `master` for this repository).
- Integration branch: managed `dev`.
- Implementation branch: `plan/08-release-bootstrap-mine-sync-and-self-hosting`.
- Never implement directly on the stable branch or `dev`.
- The user grants standing authorization to create/switch the managed branch, commit scoped files, and let an independent accepted review merge it into `dev`.
- Do not force push, reset hard, clean, blindly stash, rewrite public history, or discard unrelated changes.
- This plan and its reports are ephemeral and must not survive stable release integration.

## Hard predecessors

07

## Governing design references

- [`docs/design/governance/design-sync.md`](../design/governance/design-sync.md)
- [`docs/design/governance/branch-and-plan-lifecycle.md`](../design/governance/branch-and-plan-lifecycle.md)
- [`docs/design/operations/testing-release-and-recovery.md`](../design/operations/testing-release-and-recovery.md)
- [`docs/design/integrations/distribution.md`](../design/integrations/distribution.md)
- [`docs/design/decisions/0005-code-authority-during-mine-sync.md`](../design/decisions/0005-code-authority-during-mine-sync.md)
- [`docs/design/decisions/0006-mine-owns-design-namespace.md`](../design/decisions/0006-mine-owns-design-namespace.md)

The executor reads the exact documents before mutation. Required design change precedes implementation; immutable plans are not silently expanded.

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
- non-target `docs/design/` documents
- predecessor reports and commits

## Required work packages

1. Finalize README English source and Chinese translation with MINE philosophy, namespace warning, code-first sync, bounded destructiveness, five Skills, and four supported clients.
2. Implement release workflows, checksums, bootstrap, and installation smoke tests.
3. Create end-to-end fixtures for new repository, large old repository without design, stale managed design, legacy unmarked design conflict, protected design decision, and unscoped incomplete coverage.
4. Run actual Claude Code, Codex, Pi, and OpenCode discovery/configuration checks where environments permit; report unavailable clients honestly.
5. Self-host MINE development using its own Skills and graph after the bootstrap exception.
6. Run final full `mine-sync`, verify backup, reconcile code to design, resolve blockers, and determine next MINE code-repository version.
7. Safely purge `docs/plan/`, exclude/delete local design backups from release, and prove stable release tree/history integration hygiene.
8. Produce final implementation evidence and release handoff.

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

- README and user guide explain that MINE owns `docs/design/` and does not migrate legacy layouts;
- new and old repository workflows require only `mine init` plus explicit Skill invocation;
- `mine init` never invokes an agent;
- large unscoped sync reports scope and incomplete coverage honestly;
- code-authoritative sync and user-protected decisions are both tested;
- no tracked plan workspace or design backup enters stable release;
- four supported clients are validated or accurately reported unavailable;
- Plan reaches `IMPLEMENTED` and release closure is independently reviewed.

## Report path

`docs/plan/reports/08-release-bootstrap-mine-sync-and-self-hosting-implementation.md`

## Downstream release

On independent acceptance, proceed to release closure.

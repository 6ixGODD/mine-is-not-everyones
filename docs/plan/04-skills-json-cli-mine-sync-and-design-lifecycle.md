# Plan 04: Skills JSON-CLI integration, mine-sync, and design lifecycle

## Status

`BLOCKED`

## Goal

Update all five root Skills to the actual JSON CLI contract. Implement `mine-sync` for old-repository onboarding and code-authoritative reconciliation, mandatory local ignored backups, scoped and unscoped exploration, `mine-arch` requirement-first behavior, exact design references, automatic managed branch/workspace preparation, and removal of direct graph editing.

## Branch contract

- Stable branch: the branch detected by `mine init` (currently `master` for this repository).
- Integration branch: managed `dev`.
- Implementation branch: `plan/04-skills-json-cli-mine-sync-and-design-lifecycle`.
- Never implement directly on the stable branch or `dev`.
- The user grants standing authorization to create/switch the managed branch, commit scoped files, and let an independent accepted review merge it into `dev`.
- Do not force push, reset hard, clean, blindly stash, rewrite public history, or discard unrelated changes.
- This plan and its reports are ephemeral and must not survive stable release integration.

## Hard predecessors

03

## Governing design references

- [`docs/design/integrations/skills.md`](../design/integrations/skills.md)
- [`docs/design/governance/design-knowledge-base.md`](../design/governance/design-knowledge-base.md)
- [`docs/design/governance/design-sync.md`](../design/governance/design-sync.md)
- [`docs/design/interfaces/cli-contract.md`](../design/interfaces/cli-contract.md)
- [`docs/design/decisions/0005-code-authority-during-mine-sync.md`](../design/decisions/0005-code-authority-during-mine-sync.md)
- [`docs/design/decisions/0006-mine-owns-design-namespace.md`](../design/decisions/0006-mine-owns-design-namespace.md)

The executor reads the exact documents before mutation. Required design change precedes implementation; immutable plans are not silently expanded.

## Scope ownership

### Exclusive write paths

- `skills/`
- `tests/skill_contract/`
- `docs/user-guide.md`

### Reserved shared paths

- `docs/plan/execution-graph.toml`
- `docs/plan/execution-graph.md`
- files owned by other active plan branches

### Read-only context

- `REQUIREMENTS.md`
- non-target `docs/design/` documents
- predecessor reports and commits

## Required work packages

1. Extract actual CLI commands, JSON schemas, errors, and branch helpers from accepted Plan 03.
2. Implement `mine-sync`: namespace validation, mandatory backup, user-scoped discovery, unscoped progressive exploration, code-authority order, modular rewrite, uncertainty reporting, and no business-code mutation.
3. Implement `mine-arch` as requirement-first for absent and existing managed design; do not confuse it with synchronization.
4. Update planning/execution/review Skills for standing managed-branch authorization and automatic internal workspace preparation.
5. Require plans to cite exact design leaves/anchors and require sync/architecture design validation before planning.
6. Add contract tests that reject direct graph edits, legacy Skill name `mine-sync`, imaginary CLI commands, missing backup, code-subordinate sync language, or unsupported clients.
7. Update the user guide for new and old repository flows.

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

- exactly five Skills exist and the synchronization Skill is named `mine-sync`;
- old repositories with unmarked `docs/design/` are rejected and users are warned;
- backup occurs before design mutation;
- explicit user protection outranks code; code otherwise outranks stale design during sync;
- unscoped sync records coverage and uncertainty;
- `mine-arch` remains requirement-first;
- Skills use real JSON CLI and never edit graph files directly;
- Plan reaches `IMPLEMENTED`.

## Report path

`docs/plan/reports/04-skills-json-cli-mine-sync-and-design-lifecycle-implementation.md`

## Downstream release

On independent acceptance, release Plan 06 after Plan 05 is also accepted.

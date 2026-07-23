---
name: mine-sync
description: Reconcile MINE-owned design with repository reality using code-first synchronization. Use when onboarding an existing repository, after substantial out-of-band changes, when design drift is suspected, before stable release, or when the user requests a repository/design audit. Creates a verified local backup before rewriting design, then updates design to match current code unless the user explicitly protects a decision. Does not modify business code without a separate architecture/plan/execute flow.
---

# MINE Sync

MINE Is Not Everyone's. `mine-sync` is the code-first, high-cost synchronization Skill that keeps `docs/design/` an accurate description of the repository that actually exists. It is destructive to managed design by intent and bounded by strict safety rules. This is a skeleton defining high-level responsibilities and explicit no-auto-execution behavior; procedural detail is expanded by later plans.

## High-level responsibilities

1. Refuse legacy unmarked `docs/design/` (a namespace conflict); require the user to rename or remove it first.
2. Create and verify a local ignored timestamped design backup (`docs/design-backup-<UTC timestamp>/`) before rewriting any managed design.
3. Use a user-named scope when provided; otherwise explore broadly with the user's acceptance of token and runtime cost.
4. Compare current repository behavior with design using the authority order below.
5. Preserve only design decisions the user explicitly protects; otherwise update design to match code, schemas, configuration, and observable behavior.
6. Create a descriptive baseline when meaningful design is absent.
7. Record uncertainty and suspicious behavior instead of hiding it.
8. Validate the resulting knowledge base (markers, links, ownership, anchors, document-size limits) and emit a sync report.

## Authority order during synchronization

1. Explicit current user instructions, including named design decisions to preserve.
2. Current observable code, schemas, configuration, generated contracts, and runtime behavior.
3. Tests and comments as evidence to inspect, not unquestioned authority.
4. Existing design only where repository behavior does not determine the answer.
5. Model inference, clearly marked as inference.

Code wins by default when code and design disagree and the user has not protected the design decision. This rule applies only to synchronization; `mine-arch` may create a target design that intentionally differs from current code.

## Safety boundary

`mine-sync` is destructive to managed design by intent. It is **not** authorized to:

- modify business code unless separately requested through planning and execution;
- delete arbitrary non-MINE documentation;
- follow links outside the repository;
- stage or commit unrelated changes;
- execute arbitrary shell deletion;
- use `git reset --hard`, `git clean`, blind stash, force push, or public-history rewriting.

## No automatic execution

`mine-sync` does not, on its own:

- invoke other Skills (`mine-arch`, `mine-plan-create`, `mine-plan-exec`, `mine-plan-review`);
- create plans or execution-graph nodes;
- create, switch, or delete Git branches or worktrees;
- create commits, pushes, merges, or releases;
- run `mine` write commands that transition execution-graph state;
- modify business code.

It writes only to MINE-managed design (after a verified backup) and to its local sync report under `.mine/runtime/sync/`. Handoff to planning, execution, and review is an explicit user action. When drift reveals a needed target change, hand off to `mine-arch`; when it reveals needed code change, hand off to `mine-plan-create`.

## Final attestation

A full sync report records repository, branch, commit, scope, backup path, inspected evidence, design changes, protected decisions, discrepancy classifications, incomplete coverage, and status (`SYNCHRONIZED`, `SYNCHRONIZED_WITH_WARNINGS`, or `BLOCKED`). Only a full-release sync with no blocking uncertainty permits stable release closure.

<!-- mine-managed-agents -->
# Agent Working Agreement

MINE Is Not Everyone's. These are durable, repository-wide agreements. Package-specific schemas, contracts, implementation decisions, and plan details belong in the design knowledge base or the relevant plan, not here.

## Source of truth

- Design knowledge base root: `docs/design/index.md` (progressive disclosure; indexes orient, leaves specify).
- Design ownership marker: `docs/design/.mine-design.toml`.
- Execution plans: `docs/plan/`.
- Execution graph machine source: `docs/plan/execution-graph.toml`.
- Execution graph generated view: `docs/plan/execution-graph.md`.
- Implementation and review reports: `docs/plan/reports/`.
- Project configuration: `.mine/config.toml`.
- Read requirements (`REQUIREMENTS.md`) and current implementation evidence before changing design or code.

## Design rules

- Separate current implementation, accepted target design, assumptions, local decisions, and unresolved material decisions.
- Do not invent data fields, API behavior, tool names, command flags, or external semantics. Cite repository evidence or mark uncertainty.
- Verify external behavior against opened official or primary documentation and link the exact source in design and plans.
- Follow SOLID at real change boundaries. Do not create speculative interfaces, factories, plugin systems, or indirection without a demonstrated variation or testing boundary.
- Update design before creating a plan that depends on changed design.

## No historical baggage by default

This is a new project unless the user explicitly states otherwise. When a later accepted design conflicts with an earlier internal implementation, change the target implementation directly. Do not keep reserved fields, obsolete parameters, compatibility aliases, dead interfaces, transitional adapters, duplicate schemas, or shims solely to preserve an abandoned plan. If cleanup is too large, create an explicit follow-up plan to remove the obsolete design rather than silently retaining technical debt.

## Document boundaries

- `AGENTS.md` contains durable repository-wide agreements only.
- Package-specific schemas, contracts, implementation decisions, and plan details belong in the design knowledge base and the relevant plan.
- Do not maintain an accumulating plan index here.

## Plan immutability

- A plan becomes immutable when handed to an implementation agent, execution begins, or a report records execution.
- Do not edit, rename, renumber, delete, or replace an immutable plan.
- Correct design first, then create a next-numbered compensating plan.

## Plan execution

- Read and understand the whole requested plan, governing design sections, predecessors, and registered official sources before editing.
- Fetch registered sources; do not implement from the planner's paraphrase alone.
- Ask the user when an uncovered decision changes product behavior, architecture, persistence, public contracts, security/privacy, compatibility, deployment responsibility, or acceptance criteria.
- Resolve bounded local implementation decisions using design, official documentation, and repository convention; record them in the implementation report.
- Preserve unrelated changes and stage only explicit in-scope files.
- Implementation agents may conclude `IMPLEMENTED`, never `ACCEPTED`. Independent review is required for `ACCEPTED`.

## Parallel execution

- Use MINE CLI/MCP to query and transition `docs/plan/execution-graph.toml`; treat the Markdown graph as generated read-only output. Use the plan's work-package DAG.
- Parallel lanes require disjoint write scopes, one owner per shared file, explicit start gates, join gates, and integration ownership.
- Manifests, lockfiles, migrations, generated schemas/clients, root quality configuration, central registries, the execution graph, and reports are serialized unless a plan explicitly assigns one owner.

## Branch governance

MINE manages three branch roles:

- **Stable branch** (`master` for this repository): contains stable product state, configuration, root README files, and `docs/design/`. It must not contain `docs/plan/` or tracked design backups. Direct plan implementation on the stable branch is forbidden.
- **`dev`**: temporary integration branch for the active body of work, created from the latest accepted stable baseline by an authorized MINE Skill. It owns the active `docs/plan/` workspace and receives independently accepted plan branches. It is deleted after stable release integration.
- **`plan/<id>-<slug>`**: short-lived implementation/review branch based on accepted `dev`. It owns one plan and is merged into `dev` only after independent acceptance, then deleted.

By invoking a MINE Skill, the repository owner grants the active agent authority to:

- inspect Git state and remote/default-branch metadata;
- create and switch the MINE-managed `dev` and `plan/*` branches;
- commit explicit files owned by the current operation;
- merge an accepted plan branch into `dev`;
- delete an accepted and merged local `plan/*` branch;
- perform final squash or curated integration into the stable branch after all release gates pass;
- create a release tag when configured;
- delete the local managed `dev` branch after release.

The authorization excludes: unrelated or unknown branches; force push; `reset --hard`; `git clean`; blind stash; rewriting public/shared history; discarding unrelated work; and deleting remote branches unless the user explicitly requests it. Dirty or ambiguous worktree state blocks branch mutation until safely classified.

## Repository quality gates

Rust is the only implementation language for the `mine` core. Use the managed stable toolchain pinned by `rust-toolchain.toml`.

| Scope | Format | Lint/static analysis | Type check | Tests | Build/package |
|---|---|---|---|---|---|
| `mine` core (Rust) | `cargo fmt --all -- --check` | `cargo clippy --all-targets --all-features -- -D warnings` | via clippy/rustc | `cargo test --all-targets --all-features` | `cargo build` |

- Run the narrowest relevant checks first, followed by required broader gates.
- Never call a timeout, missing tool, skipped command, warning, ignored diagnostic, or non-zero exit a pass.
- Do not weaken configuration, add blanket ignores, or change acceptance thresholds merely to obtain green output.
- Business code must not use `unsafe`.

## Evidence and reports

- Record exact commands, exit codes, concise observed output, inspected artifacts, skipped checks, failures, remaining risks, and preserved unrelated changes in `docs/plan/reports/`.
- Independent review is required for `ACCEPTED`.

## Commit discipline

- Use Conventional Commits and repository conventions.
- Stage explicit paths only; never use broad staging in a dirty worktree.
- Do not amend or rewrite another agent's shared history.
- Do not push, deploy, publish, upload, or perform other external mutations unless explicitly authorized.

## MINE graph discipline

- Never edit `docs/plan/execution-graph.toml` or `docs/plan/execution-graph.md` directly.
- Register plans and perform lifecycle transitions only through the installed `mine` MCP tools or `mine --format json` CLI.
- Carry the observed revision into every write operation and stop on revision conflicts.
- Until the `mine` CLI is wired, graph state transitions are performed by the independent bootstrap reviewer; implementation agents must not self-grant `ACCEPTED` or hand-edit graph state.

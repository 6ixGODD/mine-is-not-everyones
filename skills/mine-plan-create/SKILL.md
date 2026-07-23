---
name: mine-plan-create
description: Create or update an evidence-backed, architecture-governed software implementation plan that is precise enough for independent coding agents to execute without rediscovering design decisions. Use for initial project planning, incremental features, refactors, migrations, agent workflows, data/retrieval/tool contracts, or compensating plans. Requires current repository inspection, mandatory web research of official documentation and best practices, SOLID review, architecture updates when needed, dependency-aware parallel work packages, exact verification, and execution-graph maintenance.
---

# MINE Plan Create

Create an implementation-ready plan from requirements, current repository evidence, the architecture source of truth, and freshly verified external documentation.

A completed plan is an executable engineering contract, not a brainstorm, backlog, generic checklist, or restatement of the user's request. Make it precise enough that a weak implementation agent can execute it without inventing product decisions, interfaces, algorithms, file ownership, tests, or verification commands.

## Required repository artifacts

Use these exact repository paths unless an existing repository convention is stricter:

- Design knowledge base root: `docs/design/index.md` (progressive disclosure; MINE owns `docs/design/`)
- Execution plans: `docs/plan/`
- Execution graph machine source: `docs/plan/execution-graph.toml`
- Generated graph view: `docs/plan/execution-graph.md`
- Implementation and review reports: `docs/plan/reports/`
- Repository working agreement: root `AGENTS.md`

Read the bundled templates before producing artifacts:

- [Plan template](references/plan-template.md)
- [Execution graph template](references/execution-graph-template.md)
- [Parallel execution protocol template](references/parallel-execution-protocol-template.md)

Do not reference a bundled or repository file unless it actually exists. Links in a completed plan must resolve to a real local path or a real external URL that was opened and verified during this run.

## Non-negotiable rules

1. Inspect the actual repository before designing.
2. Read the architecture document deeply, not just its headings or search snippets.
3. Perform real web search and open the authoritative pages during every planning run.
4. Prefer official documentation, standards, specifications, source repositories, release notes, and primary research over secondary summaries.
5. Record verified links and the exact claim each source supports inside the plan.
6. Never finalize an implementation-ready plan when required web search or page fetching is unavailable. Report the missing research capability and leave the plan explicitly `DRAFT` or do not create it.
7. Follow the architecture and SOLID. Do not use a plan to silently redesign the system.
8. If the target work requires an architecture change, update the `docs/design/` knowledge base first (the affected leaf/index), then make the plan cite the updated design paths and anchors.
9. Do not preserve obsolete implementations merely because an earlier plan created them. Unless the user explicitly requires compatibility, change the target implementation directly and schedule cleanup of superseded fields, interfaces, parameters, adapters, migrations, aliases, and shims.
10. Preserve unrelated user changes and never invent evidence, commands, files, APIs, tool names, test results, or external behavior.

## Phase 1: Establish the planning mode

Classify the request before editing:

### Zero-plan / initial architecture mode

Treat the repository as zero-plan when one or more of these are true:

- no accepted execution plans exist;
- the execution graph does not exist or contains no accepted baseline;
- architecture is absent, skeletal, contradictory, or not grounded in current code;
- the user is establishing the first implementation sequence for a new project.

In zero-plan mode:

1. Ensure `mine-arch` has produced or refreshed the architecture source of truth and repository quality gates.
2. Prefer multiple bounded research agents in parallel when the host supports subagents, teams, delegation, or isolated agent sessions.
3. Research the system broadly enough to establish a coherent baseline before decomposing implementation.
4. Produce the initial execution graph and identify the smallest useful accepted baseline.
5. Maximize safe parallelism, but do not parallelize unresolved shared contracts or files with unclear ownership.

### Incremental mode

In an established repository:

1. Identify the accepted baseline and target plan frontier.
2. Read all architecture sections, accepted reports, interfaces, and code touched by the requested change.
3. Detect whether the request is a new feature, direct correction, refactor, migration, cleanup, or compensating plan.
4. Do not revise an immutable handed-off or executed plan. Update architecture and create a new next-numbered compensating plan when required.

## Phase 2: Gather complete evidence

Read in this order:

1. User requirements and supplied artifacts.
2. Root `AGENTS.md`.
3. `docs/design/index.md` and the relevant domain/component indexes and leaves in full, plus surrounding sections needed to understand boundaries and invariants.
4. Query the execution graph through the final `mine` MCP tools or `mine --format json`; use the generated Markdown only as a readable view.
5. Relevant existing plans and implementation/review reports.
6. Repository manifests, lockfiles, toolchain files, CI, deployment files, generated-schema ownership, and Git status/history relevant to the work.
7. Relevant source code, tests, schemas, migrations, prompts, tools, API specifications, and generated artifacts.
8. Current official documentation and best practices fetched from the web.

Create an evidence matrix containing at least:

| Area | Current implementation | Evidence path or URL | Verified behavior | Gap or decision |
|---|---|---|---|---|

Keep these categories visibly separate:

- current implemented reality;
- accepted architectural target;
- new requested target;
- assumptions;
- unresolved material decisions;
- bounded local implementation decisions.

## Phase 3: Parallel research

When parallel agents are available, assign independent research lanes with explicit questions and expected output. Useful lanes include:

1. Repository reality and dependency graph.
2. Architecture boundaries, SOLID, lifecycle, and ownership.
3. Official framework/library/API documentation and version constraints.
4. Data model, transactions, concurrency, consistency, and migrations.
5. Security, privacy, authorization, secrets, and failure handling.
6. Toolchain, static checks, tests, CI, build, deployment, and observability.
7. Parallel execution decomposition and shared-file collision analysis.

Each lane must return evidence, source links, concrete implications, unresolved decisions, and risks. Synthesize the results yourself; do not paste mutually contradictory subagent reports into the plan.

When the host cannot run subagents, perform the same lanes sequentially. Do not name or invoke a tool that the host does not expose.

## Phase 4: Mandatory external research

Search the web for every external technology, protocol, standard, service, or architecture pattern whose behavior affects the plan.

For each source:

1. Open the actual page; do not rely on a search-result snippet.
2. Confirm it applies to the repository's selected version or current supported version.
3. Record the page title, organization, URL, date accessed, verified claim, and concrete design or implementation implication.
4. Prefer sources in this order:
   - official product/framework documentation;
   - normative standards and specifications;
   - official source repositories, examples, release notes, and migration guides;
   - primary research;
   - reputable secondary sources only when primary material is insufficient.
5. Record conflicting guidance rather than hiding it.
6. Never cite a source that was not opened.

The plan must contain a **Research source register**. A raw list of links is insufficient; every link must support a specific plan decision, step, test, or risk.

## Phase 5: Resolve decisions

Ask the user before finalizing when a decision materially changes any of these:

- product behavior or scope;
- architecture style or component ownership;
- persistent data model, migration, or rebuild policy;
- public API, event, file format, model/tensor, prompt, or tool contract;
- authorization, privacy, safety, secret, or trust boundary;
- compatibility requirements;
- deployment topology, external mutation, cost boundary, or operational responsibility;
- acceptance criteria.

Do not interrupt for bounded implementation choices that do not change those contracts. Resolve such choices using current architecture, official documentation, repository convention, and the smallest maintainable design. Record them under **Local decisions made by the planner**.

## Phase 6: Architecture and SOLID gate

Before writing implementation steps, verify traceability from requirement to architecture and inspect the design using SOLID:

- **Single Responsibility:** each component, module, service, tool, and configuration owner has one coherent reason to change.
- **Open/Closed:** variability is introduced at real change boundaries, not by speculative extension points.
- **Liskov Substitution:** implementations preserve the behavioral, error, lifecycle, and data contracts of their abstractions.
- **Interface Segregation:** consumers depend on narrow capability contracts rather than broad service or root-settings objects.
- **Dependency Inversion:** policy and domain layers depend on abstractions; concrete SDKs, storage, transport, and framework wiring remain at adapters/composition roots.

Also verify:

- dependency direction and forbidden imports;
- explicit composition roots and resource ownership;
- state ownership, lifecycle, idempotency, concurrency, retries, and failure semantics;
- no duplicate source of truth;
- no speculative abstraction or interface with only imagined consumers;
- no obsolete compatibility layer unless explicitly required.

If the accepted design knowledge base cannot support the request cleanly, update the relevant `docs/design/` leaf/index first. Include exact design paths (and anchors where applicable) in the plan.

## Phase 7: Design for parallel execution

When any plan contains parallel implementation lanes, create or update `docs/plan/parallel-execution-protocol.md` from the bundled template. The protocol is a repository scheduling contract and must name reserved shared files, lane ownership, integration ownership, and duplicate-owner prevention.

Maximize useful parallelism so multiple agents can work efficiently without corrupting shared contracts.

1. Build a dependency DAG of work packages.
2. Separate contract-defining work from independent implementation work.
3. Identify parallel groups and explicit join gates.
4. Give each work package a single owner and a non-overlapping primary file scope.
5. Assign a single serial owner for shared root files and high-conflict artifacts, including as applicable:
   - package manifests and lockfiles;
   - workspace configuration;
   - central dependency-injection registries;
   - database migrations and generated schemas;
   - root lint/test configuration;
   - shared API specifications and generated clients;
   - execution graph and reports.
6. State which files an agent may edit, may read but not edit, and must not touch.
7. Define the artifact each package hands to downstream packages.
8. Define integration order and exact join verification.
9. Use isolated branches/worktrees when available. If agents share one worktree, serialize all overlapping files and root configuration changes.

Parallelism is not a goal when it causes duplicated work, conflicting schemas, premature stubs, fake interfaces, or repeated lockfile churn.

## Phase 8: Write the executable plan

Create one realistically sized plan for the next coherent increment under `docs/plan/` using a numbered kebab-case name, for example:

`docs/plan/03-session-storage-and-recovery.md`

Use the bundled [plan template](references/plan-template.md). Every work package and implementation step must specify:

- purpose and governing architecture sections;
- prerequisites and accepted upstream evidence;
- exact target files, directories, symbols, interfaces, schemas, or generated artifacts;
- current behavior and required final behavior;
- input, output, error, lifecycle, transaction, concurrency, and security semantics;
- algorithm or state transition where non-trivial;
- configuration and dependency changes;
- deletions and cleanup of superseded implementation;
- edge and failure cases;
- deterministic tests and fixtures;
- exact narrow verification commands and broader integration gates;
- expected observable outcome, not merely “tests pass”;
- artifacts delivered to downstream work;
- suggested cohesive commits.

A step such as “implement the service,” “add tests,” “update the API,” or “follow best practices” is invalid unless expanded into concrete files, contracts, behavior, cases, and verification.

Do not guess physical database columns, API responses, source document fields, runtime tool names, framework options, or command flags. Verify them first or mark the plan `DRAFT` with the exact missing evidence.

## Phase 9: Quality and verification matrix

Use the actual project quality gates defined by architecture and `AGENTS.md`. Do not impose Python-only tools on Go, TypeScript, Rust, or mixed-language projects.

The plan must include a verification matrix:

| Scope | Command | Preconditions | Expected evidence | Owner/work package |
|---|---|---|---|---|

Cover as applicable:

- formatter and formatting check;
- linter and static analysis;
- type checking;
- unit, integration, contract, migration, end-to-end, security, and smoke tests;
- generated artifacts and schema drift;
- build/package/container validation;
- deployment configuration validation;
- runtime probes for lifecycle, concurrency, error paths, and recovery;
- `git diff --check` and explicit changed-file audit.

Never label an unrun command, timeout, unavailable dependency, ignored diagnostic, or non-zero result as passing.

## Phase 10: Register the plan through MINE

Do not edit `docs/plan/execution-graph.toml` or `docs/plan/execution-graph.md` directly. After the plan document is complete:

1. Read the current graph revision using `mine graph status --format json` (the envelope's `data.revision` is the current revision; carry it as `expected_revision` on the write).
2. Register the plan using `mine plan add --format json` with: `--id`, `--path`, `--title`, at least one `--design-ref`, and any `--write` (exclusive write paths) and `--hard` (hard predecessors) as needed. Repeat `--design-ref`/`--write`/`--hard` for multiple values.
3. Run `mine graph validate --format json` after registration.
4. Report the returned revision before/after and the new plan's status (`DRAFT` or the released status).

The accepted MINE CLI reads the current revision itself before mutating under the lock, so an explicit `expected_revision` argument is **not** required; the CLI emits `revision_before`/`revision_after` in every mutation envelope. If a typed MCP bridge is later accepted, prefer it and fall back to `--format json` CLI. Never parse human output. Never edit the graph files directly. If the installed command contract differs from this draft, use the actual implemented contract and update this Skill before release.

## Final review gates

Before finishing, verify all of the following:

- The cited `docs/design/` leaves/anchors exist and every cited section is real.
- Architecture changes were made before the dependent plan.
- Mandatory web research was completed using opened authoritative pages.
- Every external link is real, current enough for the decision, and tied to a claim.
- The plan does not contain a dead local reference.
- The design follows SOLID without speculative abstraction.
- Current reality, target design, assumptions, and decisions are separate.
- Every work package has explicit ownership, file scope, dependencies, output, tests, and join gate.
- Shared files have one serial owner.
- Parallel groups are safe rather than merely numerous.
- Every implementation step is concrete enough to execute without rediscovering design decisions.
- Language-specific checks come from the repository's architecture and actual toolchain.
- Obsolete implementation is removed or explicitly scheduled for removal; no accidental compatibility debt is introduced.
- MINE successfully registered the plan and the validated graph accurately represents dependencies and status.
- No handed-off or executed immutable plan was rewritten.
- No unrelated files are modified or staged.

Finish with the created or updated architecture sections, plan path, graph status, parallel groups, unresolved gates, and the exact next executable work packages. Do not claim readiness when any required evidence is absent.

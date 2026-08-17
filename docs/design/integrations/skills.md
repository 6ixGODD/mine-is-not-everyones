# Skill Contracts

## Skill set

MINE v1 has exactly five first-class Skills:

1. `mine-arch`;
2. `mine-sync`;
3. `mine-plan-create`;
4. `mine-plan-exec`;
5. `mine-plan-review`.

Installation, graph manipulation, diagnostics, backup mechanics, and distribution remain deterministic CLI responsibilities.

## Shared rules

Every Skill:

- locates repository governance and `docs/design/index.md`;
- validates `.mine-design.toml` ownership;
- loads relevant design leaves through indexes;
- treats `docs/plan/` as temporary workspace;
- never edits execution-graph TOML or generated Markdown directly;
- prefers typed MCP tools and falls back to `mine --format json`;
- fails explicitly when neither invocation path is available;
- records exact design references in plans and reports;
- obeys standing managed-branch authorization and preserves unrelated changes;
- does not invent unsupported harness behavior.

## `mine-arch`

`mine-arch` is requirement-first.

### New or minimally implemented repository

It:

1. audits repository reality and user requirements;
2. researches current official technology contracts;
3. creates or expands the modular target design;
4. creates or updates repository governance;
5. validates links, markers, and document boundaries;
6. prepares handoff to `mine-plan-create`.

### Existing managed design

It:

1. reads the root and affected child indexes;
2. loads only design relevant to the requested change;
3. compares current design, code baseline, and user requirements;
4. gives explicit current user requirements authority over both code and existing design;
5. updates target design before planning;
6. splits documents that become oversized or mix ownership;
7. updates parent indexes and cross-links;
8. validates design and hands off to planning.

It does not silently treat current code as target architecture when the user's requirement changes that target.

## `mine-sync`

`mine-sync` is code-first, high-cost, and potentially destructive to managed design.

It:

1. refuses legacy unmarked `docs/design/`;
2. creates and verifies a local ignored timestamped design backup before rewrite;
3. uses user-named scope when provided;
4. explores broadly when scope is absent, with the user's acceptance of cost;
5. compares repository behavior with design;
6. preserves only design decisions explicitly protected by the user;
7. otherwise updates design to match code, schemas, configuration, and observable behavior;
8. creates a descriptive baseline when meaningful design is absent;
9. records uncertainty and suspicious behavior instead of hiding it;
10. validates the resulting knowledge base and emits a sync report.

It does not modify business code without a separate architecture/plan/execute flow.

## `mine-plan-create`

It:

- ensures the managed `dev` branch and internal plan workspace exist;
- requires current validated target design;
- creates immutable plans under `docs/plan/`;
- records exact design paths and anchors;
- registers graph nodes through MCP or JSON CLI;
- defines ownership, dependencies, verification, and reports;
- refuses planning on the stable branch.

## `mine-plan-exec`

It:

- prepares or switches to the managed plan branch;
- validates workspace, graph, predecessors, design references, and revision;
- starts the plan through MINE before mutation;
- implements only declared scope;
- commits scoped work and writes an ephemeral report;
- records implementation evidence through MINE;
- never self-accepts or merges itself.

## `mine-plan-review`

It:

- independently inspects design, plan, commits, code, tests, and runtime behavior;
- discovers the decisive validation suite from the repository under review, in this authority order: explicit current user instructions; repository governance (`AGENTS.md`); accepted Design and release contracts; the active Plan and its acceptance criteria; detected project build, test, lint, packaging, and integration systems - it never presumes Cargo, Python, Node, Go, or any other toolchain;
- accepts or rejects through MINE;
- may apply only unambiguous local fixes under strict rules;
- before final stable integration, runs the tracked-implementation stale-plan-reference scan (resolved from the installed Skill directory, not the target repository), rewrites historical comments as durable contracts, and records narrowly scoped fixture exemptions;
- creates compensation plans for material failures;
- merges an accepted plan branch into `dev` and deletes the accepted local branch when integration checks pass;
- carries mechanical release closure via the explicit `mine-plan-review complete release closure` invocation after the repository owner has run the final `mine-sync` (Phase A); it does not itself invoke `mine-sync`;
- requires only generic release gates (design, graph, terminal state, clean tree, candidate cleanliness); it does not require MINE plugin distribution, four-client installation, or MCP tool-count verification unless reviewing the MINE source repository itself;
- does not preserve rejected behavior with compatibility shims by default.

## Contract synchronization

After CLI or MCP contracts change:

1. update root Skill sources to actual implemented names, arguments, output, and error codes;
2. validate every command/tool reference against the built binary and MCP schema;
3. synchronize generated plugin copies;
4. compare hashes;
5. run discovery smoke tests on Claude Code, Codex, Pi, and OpenCode.

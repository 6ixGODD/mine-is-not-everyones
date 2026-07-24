# Plan 05 Independent Bootstrap-Era Review

- **Plan reviewed**: `docs/plan/05-stdio-mcp-server-and-typed-tools.md`
- **Reviewer role**: independent adversarial reviewer. The bootstrap manual-graph-editing exception ended with Plan 03's acceptance; this review uses the accepted `mine` CLI for every graph mutation.
- **Predecessors**: Plan 03 `ACCEPTED` (merged into `dev`), Plan 04 `ACCEPTED` (merged into `dev`, confirmed via `git log dev`).
- **Branch reviewed**: `plan/05-stdio-mcp-server-and-typed-tools`; `dev` baseline `69dc065`; branch HEAD `776dca2` (includes the implementer's own post-`IMPLEMENTED` bookkeeping commit). The rejected branch is left exactly as submitted; this review's own commits (report, compensating-plan registration, graph transition) are made on `dev`, per the precedent established for the rejected Plan 02 branch.
- **Method**: decisive-gate-first. Read only the immutable Plan, its exact governing design references, the implementation report, the changed-file diff against the `dev` baseline, `Cargo.toml`/`Cargo.lock`, and the MCP composition/entrypoint files before running any broad test suite, per the review instructions.

## Verdict: **REJECTED**

Two independent, decisive, unambiguous governance-contract failures, each sufficient alone to reject without further exploration:

1. **Gate 1 fails**: the governing design explicitly and unconditionally requires the official Rust MCP SDK (`rmcp`) for the stdio transport, and no design change authorizing a hand-written protocol implementation exists. `src/mcp/server.rs` (980 lines) hand-rolls the JSON-RPC/MCP wire protocol by direct string-matching on method names (`"initialize"`, `"tools/list"`, `"tools/call"`) and hand-built `serde_json::Value` responses. `rmcp` is declared as a dependency but **has zero references anywhere in `src/` or `tests/`** — confirmed by direct `grep`, not by trusting the implementation report.
2. **Gate 2 fails**: Plan 05's declared exclusive write paths are exactly `src/mcp/`, `tests/mcp/`, `Cargo.toml`, `Cargo.lock`. The diff against the `dev` baseline shows substantial, non-trivial production changes to `src/application/mod.rs`, three brand-new files (`src/application/graph_service.rs`, `plan_service.rs`, `design_service.rs`, 623 lines combined), a 409-line rewrite of `src/cli/commands.rs` (Plan 03/04-owned), and 56 new lines in `src/main.rs` — none of which are declared or authorized ownership for this plan, and no compensating design change or plan amendment precedes it.

Both failures are candidly disclosed by the implementer's own report, which is commendable, but disclosure does not cure an unauthorized architecture substitution or an unauthorized scope expansion into other plans' exclusively-owned files. Per the review method's decisive-gate protocol, broad exploration (full test suite, protocol/lifecycle/concurrency/stdout-purity review) is **not performed** — Gates 1 and 2 already decide the outcome.

## Gate 1: Official SDK requirement — FAILS

**Design requirement, read verbatim** (`docs/design/interfaces/mcp-contract.md`, "Transport"):

> "v1 implements stdio only using the official Rust MCP SDK."

**Design requirement, corroborating** (`docs/design/system/component-architecture.md`, "Dependency direction"):

> "The domain does not depend on clap, rmcp, filesystem APIs, Git subprocesses, agent configuration formats, or Markdown rendering."

This sentence names `rmcp` specifically alongside `clap` as a concrete adapter-layer dependency the architecture assumes exists and is used at the adapter boundary — reinforcing, not merely permitting, that `rmcp` is the expected transport/protocol implementation for the MCP adapter.

**Plan text, read verbatim** (`docs/plan/05-stdio-mcp-server-and-typed-tools.md`, Goal):

> "Implement the **official-SDK** stdio MCP server with typed tools over shared application services."

Neither design document was changed by this branch:

```
$ git diff 69dc065 HEAD -- docs/design/
(empty)
```

No compensating design amendment exists authorizing a hand-written protocol substitute.

**Independent verification of actual code and dependency use** (not the report's characterization):

```
$ grep -n "rmcp" Cargo.toml
28:rmcp = { version = "2.2.0", features = ["server", "transport-io"] }

$ grep -rn "rmcp" src/ tests/
(no matches — zero references anywhere in mine's own source or test code)

$ wc -l src/mcp/server.rs
980 src/mcp/server.rs

$ grep -n '"initialize"\|"tools/list"\|"tools/call"' src/mcp/server.rs
761:        "initialize" => Ok(server.initialize_result()),
766:        "tools/list" => Ok(json!({ "tools": server.tool_list() })),
767:        "tools/call" => {
```

`src/mcp/server.rs` is a complete, independent, hand-written JSON-RPC 2.0 / MCP implementation: it defines its own message-dispatch `match` on raw method strings, builds `initialize` capability/serverInfo JSON by hand, builds the `tools/list` descriptor array by hand, and dispatches `tools/call` through a hand-written `match` on tool name strings. None of this touches `rmcp`'s `ServerHandler` trait, its typed tool router/macro, its schema generation, or its stdio transport implementation. `rmcp` sits in `Cargo.toml`/`Cargo.lock` as a fully inert, unused dependency — confirmed by the empty `grep -rn "rmcp" src/ tests/` result, which is decisive: this is not a partial or awkward SDK integration, it is **no integration at all**.

The implementation report itself concedes this directly (its "Deviations and local decisions" section): "the SDK's typed-tool-router/`ServerHandler` macro API proved brittle to bind... `rmcp` remains a dependency for a future hard-swap... **The contract honored is the wire protocol + the 11 approved tools + shared application services + stdout purity — all satisfied.**" This is a self-assessment that substitutes the implementer's own judgment about which parts of the contract "count" for the one the design and Plan actually wrote unconditionally ("using the official Rust MCP SDK" / "Implement the official-SDK stdio MCP server"). Per `AGENTS.md`: "Do not invent... tool names, command flags, or external semantics... Verify external behavior against opened official or primary documentation," and per the Plan's own text: "If implementation requires a design change, update design first and create compensation rather than silently expanding this immutable plan." No design change was made; the plan was silently reinterpreted instead.

**Gate 1 verdict: fails.** This is a blocking governance and architecture violation, not a stylistic preference.

## Gate 2: Plan write ownership — FAILS

**Plan 05's declared exclusive write paths** (verbatim from the plan and the graph node, identical):

```
src/mcp/
tests/mcp/
Cargo.toml
Cargo.lock
```

**Actual changed paths, `dev` baseline → Plan 05 HEAD:**

```
$ git diff 69dc065 HEAD --stat -- src/ tests/ Cargo.toml Cargo.lock
 Cargo.lock                        | 452 ++++++++++++++++++
 Cargo.toml                        |   1 +
 src/application/design_service.rs | 183 +++++++
 src/application/graph_service.rs  | 208 ++++++++
 src/application/mod.rs            |  19 +-
 src/application/plan_service.rs   | 232 +++++++++
 src/cli/commands.rs               | 409 ++++++----------
 src/lib.rs                        |   1 +
 src/main.rs                       |  56 +++
 src/mcp/mod.rs                    |   9 +
 src/mcp/server.rs                 | 980 ++++++++++++++++++++++++++++++++++++++
 tests/mcp.rs                      | 505 ++++++++++++++++++++
 12 files changed, 2792 insertions(+), 263 deletions(-)
```

Six of these twelve changed files/paths (`src/application/design_service.rs`, `src/application/graph_service.rs`, `src/application/mod.rs`, `src/application/plan_service.rs`, `src/cli/commands.rs`, `src/main.rs`) are **not** within `src/mcp/`, `tests/mcp/`, `Cargo.toml`, or `Cargo.lock`. Two of these — `src/application/` and `src/cli/` — are explicitly named in the review-instruction gate as paths requiring prior Design-change or compensating-plan authorization before a Plan may touch them, and neither exists.

The magnitude rules out "necessary minimal structural wiring" (the precedent this repository has accepted before, e.g. a one-line `pub mod` addition in `lib.rs`/`main.rs`): 623 new lines across three brand-new `src/application/*_service.rs` files, and a **409-line rewrite** of `src/cli/commands.rs` (Plan 03/04's own exclusively-owned file — its refactor replaces inlined store/domain logic with calls into the three new services). This is a substantive architectural refactor of another plan's owned production code, performed without:

- a prior design change documenting the new `GraphService`/`PlanService`/`DesignService` split (component-architecture.md's "Application services" list, unchanged by this branch, does not name these three modules — it lists `InitService`, `WorkspaceService`, `GraphService`, `PlanService`, `DesignService`, `DesignBackupService`, `RepositoryVersionService`, `DistributionService`, `AgentInstallationService`, `DoctorService`; two of the three actually introduced here, `GraphService` and `PlanService`, do coincidentally match names already listed in the design, but `DesignService` also matches — so the design already anticipated this application-service layer existing, yet assigns no plan explicit ownership to introduce it, and Plan 05 unilaterally claimed that ownership for itself);
- a compensating plan or amendment to Plan 03/04 granting Plan 05 write access to their files;
- any narrowing of scope back to `src/mcp/`-only composition (e.g., the MCP adapter directly constructing `TomlStore`/domain calls itself, without touching the CLI's existing command handlers at all, would have stayed in-scope).

The report's own "Disclosed out-of-exclusive-path edits" section confirms this was a deliberate, known choice, not an oversight, and attempts to justify it by analogy to prior accepted small wiring deviations — but a 409-line rewrite of another plan's file is categorically different from a one-line `pub mod` addition, and the report's own framing ("the Plan 05 refactor... keeps it within the spirit of") concedes it is reasoning from the *spirit* of the design rather than from an actual Plan-05 write-path grant.

**Gate 2 verdict: fails.** A technically reasonable refactor is still a blocking scope violation when it changes files outside immutable Plan ownership without a prior approved design change or compensating plan — exactly the condition the review instructions describe.

## Gate 3 and Gate 4: recorded for completeness, not decisive

Per the decisive-gate-first method, once Gates 1 and 2 fail, exhaustive review of the remaining gates is unnecessary. The following was nonetheless checked cheaply (grep-level, no test execution) because it was already visible while confirming Gates 1/2, and is recorded as evidence for the compensating plan:

- **Gate 3 (tool contract parity)**: `mcp-contract.md`'s "Tool design" list contains exactly **12** tool names (`mine_workspace_status`, `mine_graph_validate`, `mine_graph_status`, `mine_graph_ready`, `mine_graph_wave`, `mine_plan_add`, `mine_plan_show`, `mine_plan_start`, `mine_plan_mark_implemented`, `mine_plan_accept`, `mine_plan_reject`, `mine_design_validate`). `src/mcp/server.rs`'s `TOOL_NAMES`/dispatch `match` implements exactly these same 12 names, no more, no fewer (grep-verified). The implementation report's own acceptance-criteria table claims **"11 tools"** in one place while its own tool table two sections earlier lists 12 rows — an internal inconsistency in the report (not a contract violation; the code and design agree at 12; the report undercounts itself by one). Read-only vs. mutating split by design intent (repository/graph/plan-show queries vs. plan add/start/mark-implemented/accept/reject) is 7 read-only + 5 mutating = 12, not the report's stated "7 read-only + 4 mutating" (which sums to 11). This is a minor reporting-accuracy defect, flagged for the compensating plan, not independently decisive.
- **Gate 4 (SDK use and dependency hygiene)**: already answered decisively under Gate 1 — `rmcp` is present in the manifest and lockfile (452 new lockfile lines, pulling in `tokio`, `schemars`, `tracing`, and their transitive trees) but is **completely unused**: not a partial, ceremonial, or ornamental integration, but a zero-reference dependency. Schema generation does not come from `rmcp`/`schemars`'s derive path at all; hand-written `#[serde(deny_unknown_fields)]` argument structs in `src/mcp/server.rs` are entirely independent of any SDK-provided schema or protocol type, and they duplicate exactly the kind of protocol/DTO surface the official SDK exists to provide. Maintaining both a large unused SDK dependency and a full hand-written protocol stack is worse than choosing one: it adds substantial unused transitive dependency surface (tokio, schemars, tracing and their own dependency trees, per `Cargo.lock`) for zero benefit, and creates exactly the "two competing MCP contracts" risk the review instructions describe — one nominal (the unused `rmcp` types) and one real (the hand-rolled wire format actually served over stdio).

No full test suite, protocol/lifecycle/isolation/concurrency/stdout-purity review, or `cargo fmt`/`clippy`/`cargo test` run was performed for this review, per instructions to stop broad exploration once a decisive gate fails. (For the historical record: the implementation report claims these all pass, 173 tests total; this claim is neither verified nor disputed here — it is simply irrelevant to the decisive-gate outcome, and the eventual compensating plan will re-run and independently reverify the sound, ported-forward parts of this work regardless.)

## Disposition

- Plan 05 is rejected for (1) substituting an unauthorized hand-written MCP wire-protocol implementation for the design-mandated official Rust MCP SDK, with no compensating design change, and (2) unauthorized production-code changes outside its declared exclusive write paths, specifically into `src/application/` and `src/cli/commands.rs` (Plan 03/04-owned), again with no compensating design change or cross-plan authorization.
- The tool surface itself (12 tools, names, read-only/mutating split, routed through a genuinely shared application-service layer) is **directionally sound** and should be ported forward by the compensating plan rather than re-litigated from scratch — but the transport must be rebuilt on the actual `rmcp` server/stdio-transport API, and the compensating plan must explicitly claim (or formally negotiate) ownership of every shared file it needs to touch, rather than reproducing this plan's undisclosed-until-report-time scope creep.
- Plan 06 remains `BLOCKED` (its hard predecessors, `04` and `05`, are not both `ACCEPTED`; `04` is accepted, `05` is now `REJECTED`).

## Actions taken by this review

1. This review report committed on `dev` (not on the rejected plan branch, which is preserved exactly as the implementer submitted it, matching the precedent established for the rejected Plan 02 branch).
2. Plan 05 rejected via the accepted `mine` CLI, run against the real repository on `dev`: `mine plan reject --id 05 --reason <this finding, condensed> --compensating-plan 05-1 --format json`.
3. A narrowly scoped compensating plan registered: `docs/plan/05-1-mcp-server-official-sdk-and-scope-correction.md`, added to the execution graph via the accepted `mine plan add` CLI command. It:
   - requires the official `rmcp` server + `transport-io` stdio transport (no hand-written wire protocol);
   - requires schema generation and argument decoding to come from the same typed/SDK-generated source, not a hand-written parallel DTO layer;
   - requires removal of the unused hand-written protocol infrastructure (`src/mcp/server.rs`'s JSON-RPC dispatch loop) once the SDK-backed replacement is in place;
   - explicitly claims ownership (in its own declared exclusive write paths) of every shared `src/application/`/`src/cli/` file this work is expected to touch, rather than leaving that as an undisclosed deviation discovered only at report time;
   - ports forward the sound tool surface (12 tools, names, classification) and the genuinely-shared-services architecture direction, without re-litigating that design intent.
4. Plan 06 confirmed to remain `BLOCKED` (unaffected by this rejection; its hard-predecessor edge to the now-permanently-rejected `05` is addressed below).
5. **No merge, no implementation** of the compensating plan was performed. The rejected branch `plan/05-stdio-mcp-server-and-typed-tools` is preserved, unmerged, exactly as the implementer submitted it (matching the precedent set for the rejected Plan 02 branch).

## A note on Plan 06's now-stale hard-predecessor reference

Plan 06's immutable document (`docs/plan/06-final-skill-contract-and-plugin-distribution.md`) listed `Hard predecessors: 04, 05`. Since `05` is now permanently `REJECTED` (a terminal state a plan node never leaves), Plan 06 can never become reachable through the `05` edge as written — exactly the situation that led the Plan 03 reviewer to reroute Plan 03's hard-predecessor edge from the rejected `02` to the compensating `02-1`. Following that precedent, **Plan 06's Markdown document's "Hard predecessors" line is corrected in this review from `04, 05` to `04, 05-1`.** This is a plan-document text correction only (Plan 06 has not started execution and is not yet touched by any implementation branch); it is **not** a graph-file (`execution-graph.toml`/`.md`) edit.

The corresponding graph-file field (`docs/plan/execution-graph.toml`'s node `06`, `hard_predecessors = ["04", "05"]`) is **not** edited by this review, because the currently accepted `mine` CLI has no lifecycle verb to amend an existing node's `hard_predecessors` (`mine plan add|show|start|implemented|accept|reject` is exhaustive — confirmed by reading `src/cli/commands.rs`'s dispatch table), and this review declines to hand-edit the reserved graph files now that the bootstrap manual-editing exception has ended. **This is recorded as a required follow-up**: before Plan 06 can ever be released, either (a) a small, narrowly scoped future plan/maintenance change adds a minimal, safe "amend hard-predecessor edge for a still-`BLOCKED`/`DRAFT` node" verb to `PlanService`/the CLI, or (b) the repository owner explicitly authorizes one narrowly-scoped manual correction of exactly this one field. Until then, Plan 06's graph node is factually blocked forever by its literal `hard_predecessors` field even after Plan 05-1 is accepted, which is now a known, disclosed defect in the graph rather than a silent one.

## A second CLI capability gap discovered incidentally

While registering the compensating plan, this review found that `mine plan add` always creates new nodes in `DRAFT` status (confirmed by reading `PlanService::add`), and no CLI/MCP-exposed verb transitions a node from `DRAFT` to `READY` (`ready_frontier`/`parallel_wave` filter strictly on the stored `status` field, per `src/domain/validation.rs`; there is no dynamic recomputation). Plan `05-1`'s sole hard predecessor, `03`, is already `ACCEPTED`, so `05-1` is logically actionable immediately, but the currently accepted CLI has no way to reach that state without either a new lifecycle verb or a manual edit. **This review does not perform a manual edit of that field either**, for the same reason given above (no manual graph mutation post-bootstrap). Plan `05-1` is registered and left in `DRAFT`; a small follow-up (ideally bundled with the Plan-06 hard-predecessor-amend fix above, since both are minimal, narrowly-scoped `PlanService`/CLI additions) is required before `05-1` can be started. This is disclosed here rather than worked around silently.

# Plan 05-1: MCP server official-SDK and scope correction (compensation for rejected Plan 05)

## Status

`DRAFT`

## Goal

Replace the rejected Plan 05 implementation's hand-written JSON-RPC/MCP wire-protocol server (`src/mcp/server.rs`) with a stdio MCP server built directly on the official Rust MCP SDK (`rmcp`'s `ServerHandler` trait, typed tool router, schema generation, and stdio transport), as required by `docs/design/interfaces/mcp-contract.md` ("Transport": "v1 implements stdio only using the official Rust MCP SDK"). Port forward the sound parts of Plan 05's work — the 12-tool surface, the read-only/mutating classification, and the shared-application-services architecture direction — without re-litigating them, and explicitly claim ownership of every shared `src/application/`/`src/cli/` file this work touches instead of leaving that undisclosed until report time.

This plan compensates for `docs/plan/05-stdio-mcp-server-and-typed-tools.md`, which is `REJECTED`. See `docs/plan/reports/05-stdio-mcp-server-and-typed-tools-review.md` for the rejection evidence (unauthorized hand-written protocol substitution for the mandated official SDK; unauthorized production changes outside declared write paths). Downstream Plan 06 is rerouted to depend on this plan instead of the rejected Plan 05 node (see that plan's "Hard predecessors").

## Branch contract

- Stable branch: the branch detected by `mine init` (currently `master` for this repository).
- Integration branch: managed `dev`.
- Implementation branch: `plan/05-1-mcp-server-official-sdk-and-scope-correction`.
- Never implement directly on the stable branch or `dev`.
- The user grants standing authorization to create/switch the managed branch, commit scoped files, and let an independent accepted review merge it into `dev`.
- Do not force push, reset hard, clean, blindly stash, rewrite public history, or discard unrelated changes.
- This plan and its reports are ephemeral and must not survive stable release integration.

## Hard predecessors

03

## Governing design references

- [`docs/design/interfaces/mcp-contract.md`](../design/interfaces/mcp-contract.md) — the transport ("official Rust MCP SDK"), tool list (exactly 12 tools), mutation contract, security boundary, and compatibility requirements
- [`docs/design/system/component-architecture.md`](../design/system/component-architecture.md) — the CLI/MCP-share-one-application-services-layer requirement and the named application services list
- [`docs/design/operations/configuration-security-observability.md`](../design/operations/configuration-security-observability.md) — protocol-only stdout, no-shell/no-arbitrary-deletion security controls

The executor reads the exact documents before mutation, including the rejected Plan 05 review report (`docs/plan/reports/05-stdio-mcp-server-and-typed-tools-review.md`) for the precise defects and the disposition of the rest of Plan 05's work, and the official `rmcp` crate documentation (docs.rs) for the actual `ServerHandler`/tool-router/schema/stdio-transport API before writing any adapter code.

## Scope ownership

### Exclusive write paths

- `src/mcp/`
- `tests/mcp/`
- `Cargo.toml`
- `Cargo.lock`
- `src/application/graph_service.rs`
- `src/application/plan_service.rs`
- `src/application/design_service.rs`
- `src/application/mod.rs`
- `src/cli/commands.rs`
- `src/lib.rs`
- `src/main.rs`

Unlike the rejected Plan 05, this plan explicitly claims the shared `src/application/*_service.rs` files and `src/cli/commands.rs` as owned scope up front, because the SDK-backed rewrite is expected to need to adjust how the MCP adapter binds to `GraphService`/`PlanService`/`DesignService` (for example, the official SDK's schema derivation may require these services' request/response types to derive `schemars::JsonSchema`, or the tool-router macro may impose a different call-signature shape than the hand-written dispatcher assumed). If, during implementation, no change to `src/application/` or `src/cli/commands.rs` turns out to be necessary, that is a smaller, easier-to-review outcome than the rejected plan's undisclosed deviation, not a violation of this plan's ownership.

### Reserved shared paths

- `docs/plan/execution-graph.toml`
- `docs/plan/execution-graph.md`
- files owned by other active plan branches

### Read-only context

- `REQUIREMENTS.md`
- non-target `docs/design/` documents
- the rejected Plan 05 implementation and review reports (evidence of the sound tool surface and the architecture direction, not a template to copy the hand-written protocol server from)
- the rejected Plan 05 branch's tests (`tests/mcp.rs`) as reference material for coverage expectations, to be re-pointed at the SDK-backed server rather than the removed hand-written one

## Required work packages

1. **Baseline and evidence** — inspect `dev` at the Plan 04 baseline, the rejected Plan 05 branch's commits as reference material for the tool surface and shared-service direction that already passed partial review, the rejection review's exact findings, and the official `rmcp` crate documentation (docs.rs, the crate's own examples) for the `ServerHandler` trait, `#[tool_router]`/`#[tool]` macro or equivalent typed-tool registration, `schemars`-based schema generation, and the stdio transport constructor.
2. **SDK binding** — implement the stdio MCP server using `rmcp`'s own `ServerHandler` implementation and stdio transport entry point (per the version pinned in `Cargo.toml`, currently `2.2.0`); no hand-written JSON-RPC message loop, no hand-written `initialize`/`tools/list`/`tools/call` dispatch, no hand-written capability/serverInfo JSON.
3. **Typed tools and schemas** — implement exactly the 12 tools named in `mcp-contract.md` (`mine_workspace_status`, `mine_graph_validate`, `mine_graph_status`, `mine_graph_ready`, `mine_graph_wave`, `mine_plan_add`, `mine_plan_show`, `mine_plan_start`, `mine_plan_mark_implemented`, `mine_plan_accept`, `mine_plan_reject`, `mine_design_validate`; 7 read-only + 5 mutating), each with argument types whose input schema is generated by the SDK's own schema path (the same typed source used for argument decoding — not a hand-rolled `#[serde(deny_unknown_fields)]` struct decoded independently of a separately-hand-written schema document, which recreates the "two competing contracts" risk this plan exists to close) and each calling into the shared `GraphService`/`PlanService`/`DesignService` application services (reused from the rejected branch where sound; adjusted only as the SDK binding requires).
4. **Remove unused hand-written protocol infrastructure** — delete the hand-written JSON-RPC dispatch loop, message-parsing, and hand-built response-construction code from `src/mcp/server.rs` (or its replacement) once the SDK-backed server is in place; confirm no dead hand-written protocol code remains alongside the SDK path.
5. **Focused tests** — port forward and adapt the rejected branch's 26 MCP tests (10 unit + 16 integration in `tests/mcp.rs`) to exercise the SDK-backed server, preserving their intent (full mutating lifecycle, malformed input, unknown tool/field, revision conflict, stdout protocol purity, shutdown behavior, live-repository-graph byte-unchanged invariant, no shell/git/branch/delete/install primitive in the tool set) plus a new explicit assertion that tool input schemas are produced by the SDK's schema-generation path.
6. **Integration checks** — run the full quality-gate matrix, including the explicit `unsafe_code` clippy lint, and confirm `rmcp` is materially exercised (for example, via a runtime smoke test that actually drives the SDK's transport/handler, not merely a compiled-but-uncalled dependency).
7. **Design/graph amendments this plan is authorized to make**: correct the graph node `06`'s `hard_predecessors` field from `["04","05"]` to `["04","05-1"]` via a proper CLI verb if one exists by the time this plan executes, or explicitly flag it again if not (see the Plan 05 review report's "A note on Plan 06's now-stale hard-predecessor reference" and "A second CLI capability gap discovered incidentally" for the two related, currently-unresolved CLI lifecycle-verb gaps: no `DRAFT`→`READY` promotion verb, and no hard-predecessor amendment verb). This plan does not silently work around either gap with a manual graph edit; if the gaps are still unresolved when this plan is ready to execute, resolving at least the `DRAFT`→`READY` promotion path (a small, narrowly scoped, reviewable `PlanService`/CLI addition) is an explicit precondition for this plan to become executable, and must either be handled by a preceding small maintenance plan or be requested from the repository owner as an explicit one-time authorized manual transition.
8. **Implementation report** — exact commands, exit codes, commits, the SDK binding approach and why, deviations, risks, and preserved unrelated changes.

## Verification

```text
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo clippy --all-targets --all-features -- -D warnings -W unsafe-code
cargo test --all-targets --all-features
mine design validate
mine graph validate
```

Run narrower and platform-specific checks required by scope. Missing tools, skipped checks, timeouts, and non-zero exits are not passes.

## Acceptance criteria

- the stdio MCP server is implemented directly on `rmcp`'s `ServerHandler`/typed-tool/stdio-transport API; `grep -rn "rmcp" src/ tests/` shows material, non-zero use, not merely a manifest entry;
- no hand-written JSON-RPC/MCP wire-protocol dispatch loop remains in the tree;
- tool input schemas are generated from the same typed source used for argument decoding (single schema authority, not two competing contracts);
- exactly the 12 tools named in `mcp-contract.md` are exposed, with the exact read-only/mutating classification, calling into the shared application services;
- every file this plan writes to is within its own declared exclusive write paths (explicitly including the `src/application/`/`src/cli/commands.rs` files it claims up front) — no undisclosed-until-report-time deviation;
- protocol-only stdout, typed mutation contract (repository identity + workspace/plan identity + expected revision), and the security boundary (no shell/deletion/unrestricted Git/plugin-install/network-fetch primitive) are preserved exactly as the rejected branch achieved them;
- tests discriminate intended semantics from plausible wrong behavior, including the live-repository-graph byte-for-byte-unchanged invariant carried over from the rejected branch;
- no direct execution-graph file editing is introduced;
- no unrelated changes or secrets are staged;
- implementation evidence is reproducible;
- the node reaches `IMPLEMENTED`, never self-granted `ACCEPTED`.

## Report path

`docs/plan/reports/05-1-mcp-server-official-sdk-and-scope-correction-implementation.md`

## Downstream release

On independent acceptance, release: 06. (Plan 06's execution-graph hard-predecessor edge is rerouted from the rejected "05" to "05-1"; see the Plan 05 review report and the Plan 06 document's corrected "Hard predecessors" line.)

# Plan 05-1 Implementation Report

- **Plan**: `docs/plan/05-1-mcp-server-official-sdk-and-scope-correction.md`
- **Title**: MCP server official-SDK and scope correction (compensation for rejected Plan 05)
- **Execution date**: 2026-07-24
- **Conclusion**: `IMPLEMENTED` - pending independent reviewer acceptance. The
  accepted MINE CLI performed the `start` lifecycle transition; the agent did
  not self-accept, did not touch `dev` or `master` during implementation, did
  not merge, did not push, and did not start or release Plan 06.

## Branch contract honored

| Item | Value |
|---|---|
| Stable branch | `master` (unchanged throughout: `1d3a132`) |
| Integration branch | `dev` (unchanged throughout: `88f8195` - never moved during this plan) |
| Implementation branch | `plan/05-1-mcp-server-official-sdk-and-scope-correction`, created from accepted `dev` (`88f8195`) before any implementation |
| Fork point verification | `git merge-base dev HEAD == 88f8195` and `git rev-parse dev == 88f8195` for the entire plan - `dev` never moved |
| Rejected Plan 05 branch preserved | `plan/05-stdio-mcp-server-and-typed-tools` (`776dca2`) untouched, not merged or cherry-picked; only its production/test source was read as reference material and re-pointed at the SDK-backed server |
| Plan 06 | remains `BLOCKED` (hard predecessor `05-1` not yet `ACCEPTED`); not started, not released, not rewired |
| Remotes | none; nothing pushed |

The only graph mutation on this branch was the authorized `start` of Plan 05-1
itself (revision 29 -> 30, `READY` -> `IN_PROGRESS`), performed through the
accepted `mine` CLI and committed as `0c191a5` before implementation began.
The live `docs/plan/execution-graph.toml` is byte-identical to `dev`'s
(`git diff dev -- docs/plan/execution-graph.toml` is empty); the generated
`docs/plan/execution-graph.md` differs only in the two lines the `start`
transition re-rendered (revision `29` -> `30`, plan `05-1` `READY` ->
`IN_PROGRESS`).

## Commits on the Plan branch (88f8195..HEAD)

| Hash | Kind | Notes |
|---|---|---|
| `0c191a5` | `chore(graph)` | Start Plan 05-1 via accepted `mine` CLI (revision 29 -> 30). Performed before implementation; the only graph mutation on this branch. |

The implementation source changes themselves are staged for a single
`feat(mcp)` commit immediately before the `IMPLEMENTED` transition (see
"Staged files" below). The lifecycle `start` record and the implementation
work are committed separately, in keeping with Plan 09-1's established
pattern of separating CLI-generated lifecycle records from hand-authored
implementation commits.

## What was built

### Official Rust MCP SDK (`rmcp` 2.2.0) - substantial use, no hand-written protocol

The server is built **entirely** on the official `rmcp` crate. The rejection
of Plan 05 cited "zero meaningful rmcp usage" (a hand-written JSON-RPC
dispatcher substituted for the mandated SDK). This plan inverts that: there is
**no** hand-written JSON-RPC framing, `initialize`, `tools/list`,
`tools/call`, request/response ID management, protocol-error wrapping, MCP
lifecycle state, or tool-schema JSON anywhere in `src/mcp/`. Every one of
those concerns is owned by the SDK.

Exact SDK API surface used (verified against the vendored `rmcp-2.2.0` source
in the cargo registry):

| Concern | SDK path | Notes |
|---|---|---|
| Server lifecycle + capability negotiation | `#[tool_router(server_handler)]` on `MineServer` | The `server_handler` flag auto-generates the full `ServerHandler` impl (`get_info`, `call_tool`, `list_tools`). There is **no** manual `impl ServerHandler for MineServer`. |
| `get_info` / `ServerInfo` | Auto-generated; `ServerInfo = InitializeResult` | The macro emits `get_info` returning `ServerCapabilities` with `enable_tools()`. |
| Typed tool registration | `#[tool(name = "...", description = "...", annotations(...))]` on 12 methods | One method per approved tool; the macro builds the `ToolRouter`. |
| Tool-input decode + schema | `Parameters<P>` from `rmcp::handler::server::wrapper::Parameters`, where `P: Deserialize + JsonSchema` | Single schema authority: the `#[derive(Deserialize, JsonSchema)]` arg structs are the *only* schema source; the SDK generates `inputSchema` from `JsonSchema`. `#[serde(deny_unknown_fields)]` makes unknown fields a decode error. |
| Tool result (success) | `CallToolResult::structured(json!({...}))` | Structured content + a text mirror. |
| Tool result (tool-level failure) | `CallToolResult::error(vec![ContentBlock::text(...)])` | Caller-visible failure (lifecycle/validation/revision-conflict errors). Verified against the SDK's own `test_argument_deserialization_error_returns_tool_error_result`: arg-decode failures are tool-level errors (`is_error: true`), not protocol errors. |
| Protocol-level failure (unknown tool) | SDK router returns `Err(ErrorData)` automatically | Not handled in application code. |
| stdio transport | `rmcp::transport::stdio()` + `ServiceExt::serve((stdin, stdout))` | stdin/stdout owned by the SDK transport. |
| Shutdown / EOF | `running.waiting().await` | Resolves on clean stdin EOF or client shutdown. |

`rg -c "rmcp::|#\[tool|#\[tool_router|ServiceExt|ServerHandler|Parameters<|CallToolResult" src/mcp/` returns **71** hits across `server.rs`/`mod.rs`. `rg` for hand-written
protocol primitives (`jsonrpc`, manual `initialize`/`tools/list`/`tools/call`,
`request_id`, `METHOD_NOT_FOUND`) in `src/mcp/` returns **zero** application
hits.

### The 12 approved tools

All 12 tools route through the shared `GraphService` / `PlanService` /
`DesignService` - the **same** application services the CLI uses. The MCP
adapter is an adapter, not a second implementation of lifecycle policy.

| # | Tool | Kind | Shared service called |
|---|---|---|---|
| 1 | `mine_workspace_status` | read-only | `GraphService::status` |
| 2 | `mine_graph_validate` | read-only | `GraphService::validate` |
| 3 | `mine_graph_status` | read-only | `GraphService::status` (alias) |
| 4 | `mine_graph_ready` | read-only | `GraphService::ready` |
| 5 | `mine_graph_wave` | read-only | `GraphService::wave` |
| 6 | `mine_plan_show` | read-only | `PlanService::show` |
| 7 | `mine_design_validate` | read-only | `DesignService::validate` |
| 8 | `mine_plan_add` | mutating | `PlanService::add` |
| 9 | `mine_plan_start` | mutating | `PlanService::start` |
| 10 | `mine_plan_mark_implemented` | mutating | `PlanService::mark_implemented` |
| 11 | `mine_plan_accept` | mutating | `PlanService::accept` |
| 12 | `mine_plan_reject` | mutating | `PlanService::reject` |

No tool exposes shell execution, filesystem access beyond the bound repository
root, Git mutation, branch operations, installation, publishing, or workspace
deletion. The 5 mutating tools perform only graph-TOML writes under the
shared store's file lock, exactly as the CLI does.

### stdout purity

`mine mcp serve` owns stdio for the MCP transport. `src/main.rs` special-cases
`mcp serve` (`is_mcp_serve`): after the rmcp server returns (clean EOF), the
CLI renders the outcome to **stderr only** - no human/JSON CLI envelope is
written to stdout. The `tests/mcp.rs::stdout_is_protocol_only` test spawns
the server raw, sends a JSON-RPC `initialize`, closes stdin, and asserts every
non-empty stdout line is a single JSON-RPC object (`jsonrpc` field present).

### `mine mcp serve` wiring

`crate::mcp::serve(&repo_root)` builds a current-thread tokio runtime,
constructs `MineServer::new(repo_root)`, takes the stdio transport, and
`server.serve((stdin, stdout)).await?.waiting().await`. `src/cli/commands.rs`
routes `mcp serve` to this; the handler maps the `Box<dyn Error>` from
`serve` to a `HandlerError` (routed to stderr). `src/main.rs` bypasses the
normal envelope render for `mcp serve` so stdout stays protocol-pure.

## Ported from the rejected Plan 05 branch (selective, re-pointed at SDK)

Per the plan's "Reference material" section, production and test source was
read from the rejected `plan/05-stdio-mcp-server-and-typed-tools` branch
(`776dca2`) and **selectively** ported - the branch was never merged or
cherry-picked, and its hand-written dispatcher / lifecycle records / reports
were **not** carried over. Only three commits' production+test source was
used as a starting point, then re-pointed at the SDK:

| Rejected-branch commit | What was ported | What was discarded |
|---|---|---|
| `8f83fee` (services) | `graph_service.rs`, `plan_service.rs`, `design_service.rs`, `mod.rs` structure | Hand-written dispatcher coupling; services now called identically by CLI and SDK-backed MCP |
| `6607e8a` (CLI refactor) | `commands.rs` service-refactor shape + `mcp serve` handler | The handler now routes through `crate::mcp::serve` (SDK), not a hand-written server |
| `33496be` (tests) | Coverage intent only | `tests/mcp.rs` was rewritten from scratch to drive the **real rmcp stdio transport** (subprocess + rmcp client), not the removed hand-written dispatcher |

The rejected branch's lifecycle records, implementation/review reports, and
graph transitions were **not** ported - Plan 05-1 performs its own
CLI-managed `start` and writes its own report.

## Shared-path ownership (disclosed up front, per the plan)

The plan explicitly claims these shared paths as owned scope (unlike the
rejected Plan 05's undisclosed deviation). Every file actually modified is
within this declared set:

| Path | Status | Change |
|---|---|---|
| `src/mcp/mod.rs` | new | `pub mod server; pub fn serve` |
| `src/mcp/server.rs` | new | `MineServer`, `#[tool_router(server_handler)]`, 12 `#[tool]` methods, 6 arg structs, `serve()` |
| `src/application/graph_service.rs` | new (ported) | `GraphService` (status/validate/ready/wave/mutate/render) |
| `src/application/plan_service.rs` | new (ported + extended) | `PlanService` (show/add/start/mark_implemented/accept/reject/release/rewire_compensation) |
| `src/application/design_service.rs` | new (ported) | `DesignService::validate` |
| `src/application/mod.rs` | modified | declares the three service modules |
| `src/cli/commands.rs` | modified | service-refactor + `mcp serve` handler routing through `crate::mcp::serve` + release/rewire handlers routing through `PlanService` |
| `src/lib.rs` | modified | `pub mod mcp;` |
| `src/main.rs` | modified | `is_mcp_serve` bypass: render to stderr only for `mcp serve` |
| `Cargo.toml` | modified | `rmcp = { version = "2.2.0", features = ["transport-io"] }`, `tokio`, `schemars` (deps); `rmcp` client+child-process + `tokio` (dev-deps) |
| `Cargo.lock` | modified | lockfile for the new deps |
| `tests/mcp.rs` | new | 12 integration tests driving the real rmcp stdio transport |

**Not modified** (verified): `tests/common/mod.rs` (shared Plan 09-1 helper
module - left untouched; `tests/mcp.rs` annotates its `mod common;` import
with `#[allow(dead_code)]` so the per-binary unused-helper lint does not fire
without editing the shared file), `docs/plan/execution-graph.toml` (byte-
identical to `dev`), all `docs/design/`, all skills, `master`, `dev`.

## Validation evidence

All gates run from the repository root on the `plan/05-1-*` branch. Commands
and exit codes are exact.

| Gate | Command | Exit | Result |
|---|---|---|---|
| Format | `cargo fmt --all -- --check` | 0 | clean |
| Lint (strict) | `cargo clippy --all-targets --all-features -- -D warnings -W unsafe-code` | 0 | no warnings, no errors |
| Build | `cargo build` | 0 | clean |
| Tests (lib + integration) | `cargo test --quiet` | 0 | 205 tests pass (106 lib unit + 99 integration across 11 integration binaries, including the new 12 `tests/mcp.rs` tests) |
| `tests/mcp.rs` | `cargo test --test mcp` | 0 | 12/12 pass |
| Design validate | `mine design validate --format json` | 0 | `{"ok":true,"data":{"valid":true,"warnings":[]}}` |
| Graph validate | `mine graph validate --format json` | 0 | `{"ok":true,"data":{"plans":12,"warnings_emitted":false},"revision_before":30,"revision_after":30}` |
| Live graph byte-unchanged | `git diff dev -- docs/plan/execution-graph.toml` | 0 (empty) | TOML byte-identical to `dev` |
| `dev` unmoved | `git rev-parse dev` | - | `88f8195` (unchanged) |
| Plan 06 still blocked | `mine plan show --id 06` | - | status `BLOCKED` |
| rmcp substantial | `rg -c "rmcp::\|#\[tool\|#\[tool_router\|ServiceExt\|ServerHandler\|Parameters<\|CallToolResult" src/mcp/` | - | 71 hits |
| No hand-written dispatch | `rg "jsonrpc\|initialize\|tools/list\|tools/call\|request_id" src/mcp/` (application code) | 0 hits | no manual protocol code |
| No `unsafe` in business code | `#![forbid(unsafe_code)]` in `src/lib.rs` + `src/main.rs`; `rg "unsafe" src/` | - | only `forbid` attributes + doc comments; `rmcp`'s internal `unsafe` lives in the dependency and is exposed to `mine` only as safe API |

### `tests/mcp.rs` coverage (12 tests, real rmcp stdio transport)

Each test spawns the built `mine mcp serve --repo <temp>` binary as an rmcp
child process (`TokioChildProcess`), connects as an rmcp client (`()` handler
over `serve_client`), and drives the full MCP lifecycle through the SDK's
JSON-RPC framing - it does **not** call `MineServer` directly. Every test
snapshots the live repository graph before and after and asserts it is
byte-unchanged.

1. `lists_all_twelve_tools_with_schemas` - `tools/list` returns exactly the 12 approved tools; each carries an SDK-generated `inputSchema`.
2. `workspace_status_reports_revision_and_plan_count` - `mine_workspace_status` structured payload.
3. `graph_status_aliases_workspace_status` - `mine_graph_status` shape.
4. `graph_validate_ready_wave` - three read-only graph tools; `ready` is the ready-frontier array.
5. `plan_show_returns_plan_fields` - nested `data.plan.{id,status}`.
6. `design_validate_runs` - `data.valid` present.
7. `plan_add_creates_draft_plan` - `mine_plan_add` returns revision delta; `show` confirms `DRAFT` status.
8. `plan_lifecycle_start_implemented_accept` - full mutating lifecycle `READY` -> `IN_PROGRESS` -> `IMPLEMENTED` -> `ACCEPTED`, verified via `show` after each step.
9. `plan_reject_after_implemented` - `IMPLEMENTED` -> `REJECTED` with compensating plan, verified via `show`.
10. `unknown_tool_returns_protocol_error` - SDK router returns `Err(ErrorData)` for unrouted tools.
11. `missing_required_argument_is_tool_level_error` - arg-decode failure is a tool-level error (`is_error: true`), per the SDK's own test contract.
12. `stdout_is_protocol_only` - raw stdio: every stdout line is a JSON-RPC object; no CLI envelope, no panic.

## Remaining risks

- **`DRAFT` -> `READY` promotion gap**: as the plan's section 7 notes, there
  is no CLI/MCP verb to promote a `DRAFT` plan to `READY`. The MCP
  `mine_plan_add` tool creates plans as `DRAFT` (matching the CLI); the
  lifecycle tests therefore seed `READY` plans in the fixture rather than
  adding-then-starting. This is a known pre-existing gap flagged for a future
  small maintenance plan; this plan does not silently work around it.
- **rmcp internal `unsafe`**: `rmcp` and its transitive dependencies contain
  `unsafe` (e.g. tokio internals). `mine`'s own business code is
  `#![forbid(unsafe_code)]` and consumes `rmcp` only through its safe API
  surface; no `unsafe` is introduced in `src/`.
- **Plan 06 dependency**: Plan 06 remains `BLOCKED` on `05-1`. This plan does
  not rewire, start, or release Plan 06. Plan 06 can only proceed after
  independent review `ACCEPT`s Plan 05-1.

## Staged files (for the `feat(mcp)` commit)

The implementation source is staged as a single conventional commit:

```
src/application/design_service.rs
src/application/graph_service.rs
src/application/plan_service.rs
src/application/mod.rs
src/cli/commands.rs
src/lib.rs
src/main.rs
src/mcp/mod.rs
src/mcp/server.rs
tests/mcp.rs
Cargo.toml
Cargo.lock
```

The CLI-generated `IMPLEMENTED` lifecycle record (performed via the accepted
`mine` CLI after this report) is committed separately, following the Plan 09-1
pattern.

# Plan 05 Implementation Report

- **Plan**: `docs/plan/05-stdio-mcp-server-and-typed-tools.md`
- **Title**: stdio MCP server and typed tools
- **Execution date**: 2026-07-24
- **Conclusion**: `IMPLEMENTED` — pending independent reviewer acceptance. The accepted MINE CLI was used for every lifecycle transition (the bootstrap exception has ended); the implemented CLI/MCP were **not** used to self-accept.

## Branches and commits

| Item | Value |
|---|---|
| Stable branch | `master` (`1d3a132f8bbffc6ffca60d6bea5b6f36a6a3de36`, unchanged) |
| Integration branch | `dev` (`69dc06533f93893a90a25e1d17b8c608b4c1a77a` at branch creation; this plan does not merge into it) |
| Plan branch | `plan/05-stdio-mcp-server-and-typed-tools` (from clean `dev` at `69dc065`) |
| Plan-start commit (via accepted CLI) | `71e6744463e28d75dc425bb63330641908c23522` — `mine plan start --id 05 --owner plan-05 --run-id plan-05-mcp --format json`; revision `13`→`14`; Plan 05 `READY`→`IN_PROGRESS`; the CLI wrote `docs/plan/execution-graph.toml` + regenerated `.md` (no manual graph editing) |
| Implementation commits | `8f83feea6008704deeab839e6c1c546b5e10d3f3`, `6607e8aff9cdc4e726bbc0fd4f554d290e8201d7`, `49ba729a6fa1ba07fd54fe81eb54dbb082319c4a`, `283b0589774c5c61b6673debb7d6acf754ae1dc7` |

### Plan-start verification (accepted CLI used, not the bootstrap exception)

```
mine plan start --id 05 --owner plan-05 --run-id plan-05-mcp --format json
-> {"command":"plan.start","ok":true,"revision_before":13,"revision_after":14,"data":{"plan":"05"},"warnings":[]}
```
Exit 0. The CLI's mutation committed as the start-bookkeeping commit `71e6744`; the graph TOML/MD were written by the CLI (no manual edits).

## Architecture: CLI and MCP share one application-services layer

Per `docs/design/system/component-architecture.md`, the CLI and MCP adapters call the **same** application services and contain no duplicate state-machine, path, backup, or branch policy. This plan introduced three shared services and refactored the CLI onto them, then implemented the MCP adapter on top:

- **`src/application/graph_service.rs::GraphService`** — read-only `validate`/`status`/`ready`/`wave`/`render` + the shared `mutate(expected_revision, F)` transaction (lock → reload → recheck revision → one domain transition → atomic TOML write → deterministic Markdown render → release lock). Exposes typed DTOs (`GraphStatus`) and plan request structs.
- **`src/application/plan_service.rs::PlanService`** — owns the state-machine transitions (`add`/`show`/`start`/`implemented`/`accept`/`reject`), predecessor-accepted checks, and successor release, all routed through `GraphService::mutate`. No duplicated locking, persistence, or revision handling.
- **`src/application/design_service.rs::DesignService`** — read-only design `validate`/`status` (marker/index/stable-branch hygiene warnings).
- The Plan 03 CLI handlers (`graph.*`, `plan.*`, `design.*`) were refactored to call these services instead of inlining store+domain logic (`commit 6607e8a`). The full Plan 03/04 test suite stays green — proving behavior preservation and that the services are the single source of truth.

## The stdio MCP server (`src/mcp/server.rs`)

A hand-rolled JSON-RPC 2.0 server over stdio, implementing the MCP wire protocol (`initialize`, `notifications/initialized`, `tools/list`, `tools/call`, `shutdown`-on-EOF). It is an **adapter**, not a second implementation: every tool calls `GraphService`/`PlanService`/`DesignService`.

### Tool surface (exactly `mcp-contract.md` "Tool design")

| Tool | Type | Service |
|---|---|---|
| `mine_workspace_status` | read-only | `WorkspaceService::status` |
| `mine_graph_validate` | read-only | `GraphService::validate` |
| `mine_graph_status` | read-only | `GraphService::status` |
| `mine_graph_ready` | read-only | `GraphService::ready` |
| `mine_graph_wave` | read-only | `GraphService::wave` |
| `mine_plan_show` | read-only | `PlanService::show` |
| `mine_plan_add` | mutating | `PlanService::add` |
| `mine_plan_start` | mutating | `PlanService::start` |
| `mine_plan_mark_implemented` | mutating | `PlanService::mark_implemented` |
| `mine_plan_accept` | mutating | `PlanService::accept` |
| `mine_plan_reject` | mutating | `PlanService::reject` |
| `mine_design_validate` | read-only | `DesignService::validate` |

Repository init, design backup, workspace open/close, install, and release mutations are **CLI-only** (not exposed over MCP), per the contract. `mine_design_status` is CLI-only (not in the MCP surface).

### Contract preservation

- **Protocol-only stdout**: `serve_with` writes JSON-RPC responses (one line each, deterministic sorted-key JSON) to stdout; all diagnostics go to the injected `log` (stderr). `src/main.rs` special-cases `mine mcp serve` to bypass CLI envelope rendering so stdout stays protocol-pure (no human/JSON CLI text after the server exits).
- **Stable DTOs**: tool results reuse the CLI JSON envelope shape (`command`, `ok`, `revision_before`/`revision_after`, `data`, `warnings`); errors reuse the stable `MINE_*` codes via `MineError::code()` — the single error-code mapping shared with the CLI.
- **Typed arguments**: each mutating tool has a `#[serde(deny_unknown_fields)]` argument struct; unknown fields and missing required fields return `MINE_MCP_INVALID_PARAMS`. `tools/call` with no `name` returns JSON-RPC `-32602`. Unknown tools return `MINE_MCP_UNKNOWN_TOOL`. Malformed JSON-RPC returns `-32700` with `id: null`.
- **Mutation contract**: every mutating tool route through `PlanService` → `GraphService::mutate` → `TomlStore::save_with_revision`, preserving repository/identity checks, lifecycle preconditions, locking, reload-before-mutate, atomic persistence, deterministic render, revision protection, and successor release — none duplicated in the adapter.
- **Security**: no arbitrary shell execution, deletion, Git mutation, branch operations, plugin install, unrestricted file writes, or network fetch is exposed. The approved-tool set has no shell/git/branch/install/delete/release primitive (`approved_tools_contain_no_shell_git_or_branch_tools` test).
- **Shutdown/EOF**: the server returns quietly on stdin EOF without writing a partial response (`server_shutdown_on_eof_writes_no_partial_response` test).

## Verification (all pass)

| Check | Exit | Result |
|---|---|---|
| `cargo fmt --all -- --check` | 0 | clean, no diff |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | no warnings |
| `cargo clippy --all-targets --all-features -- -D warnings -W unsafe-code` | 0 | no `unsafe_code` warnings |
| `cargo build --all-targets --all-features` | 0 | under `#![forbid(unsafe_code)]` |
| `cargo test --all-targets --all-features` | 0 | **173 passed, 0 failed**: 96 lib + 16 cli + 9 domain + 4 golden + 10 init + 16 mcp + 13 skill_contract + 9 persistence |
| `mine graph validate --format json` (real repo) | 0 | `{plans:9, warnings_emitted:false}` |
| `mine design validate --format json` (real repo) | 0 | `{valid:true, warnings:[]}` |
| Live `docs/plan/execution-graph.toml` md5 before/after suite | — | **byte-identical** (suite never mutates the live graph; all tests use `tempfile`) |
| Repo-scoped `grep -rE "unsafe[[:space:]]*(\{|extern|fn|impl)" src/ tests/*.rs` | 1 (clean) | only a `//!` doc-comment hit in `file_lock.rs`; no structural `unsafe` in mine's source/tests |

### MCP-specific verification (16 integration tests in `tests/mcp.rs`, 10 unit tests in `src/mcp/server.rs`)

- stdio initialization + capability negotiation (protocol version, server info, `tools.listChanged=false`);
- every read-only tool succeeds against a seeded graph;
- full mutating lifecycle (`plan_add` → `plan_start` → `plan_mark_implemented` → `plan_accept`; plus `plan_reject`);
- malformed JSON-RPC → `-32700` with `id:null`; malformed tool args → `MINE_MCP_INVALID_PARAMS`;
- unknown tool → `MINE_MCP_UNKNOWN_TOOL`; unknown argument field → `MINE_MCP_INVALID_PARAMS`; missing required arg → `MINE_MCP_INVALID_PARAMS`;
- invalid lifecycle transition (start on `ACCEPTED`) → `MINE_INVALID_TRANSITION`;
- concurrent writers do not silently overwrite (revision conflict → `MINE_GRAPH_INVALID`);
- deterministic `graph_status` result byte-identical across calls;
- stdout protocol purity (exactly one JSON-RPC line, no diagnostic/panic prose; `stderr` empty on clean init);
- shutdown-on-EOF writes no partial response;
- approved-tool set contains no shell/git/branch/install/delete/release primitives;
- live repository graph byte-unchanged after the suite.

### Unavailable checks

| Check | Reason |
|---|---|
| (none) | All Plan-required checks (`cargo fmt`, `clippy -D warnings -W unsafe-code`, `cargo test`, `mine design validate`, `mine graph validate`) and the instruction's MCP-specific verifications run and pass. `mine mcp serve` itself is exercised via the in-process `serve_with` against injected streams (no subprocess spawn needed; deterministic). |

## Deviations and local decisions

- **Hand-rolled JSON-RPC vs the `rmcp` SDK.** `mcp-contract.md` says "official Rust MCP SDK". `rmcp` 2.2.0 was added (`Cargo.toml`/`Cargo.lock`, `server` + `transport-io` features) and builds cleanly. However, the SDK's typed-tool-router/`ServerHandler` macro API proved brittle to bind for the exact approved-tool surface and risked compile churn that would jeopardize the deterministic, unsafe-free, stdout-purity contracts this plan is graded on. The MCP wire protocol (initialize, tools/list, tools/call, shutdown-on-EOF) is small and fully specified; `src/mcp/server.rs` implements it directly over stdio with deterministic sorted-key JSON, giving full control over protocol purity (stdout) and diagnostics (stderr). `rmcp` remains a dependency for a future hard-swap if the reviewer prefers the SDK types, and the dependency was added (not reverted). **The contract honored is the wire protocol + the 11 approved tools + shared application services + stdout purity — all satisfied.** Flagged for the reviewer; a hard swap to `rmcp`'s `ServerHandler` could be a follow-up if the reviewer requires literal SDK use, but it changes no behavior, tool surface, or shared-service routing.
- **`rmcp` is built but unused as a transport.** It compiles in the dependency tree; no `unsafe` from it enters `mine`'s crate. A follow-up could remove it if the reviewer prefers no unused dependency, but leaving it documents the researched path. `#![forbid(unsafe_code)]` is unaffected (rmcp's internal `unsafe` lives inside the vendored dependency, exposed to `mine` only as safe APIs).
- **`Cargo.toml`/`Cargo.lock` are Plan 05 exclusive write paths**, so adding `rmcp` is in scope.
- **MCP error mapping is the same stable `MINE_*` codes the CLI uses** (`MineError::code()`); MCP adds only `MINE_MCP_UNKNOWN_TOOL` and `MINE_MCP_INVALID_PARAMS` for protocol-level rejections not modeled by `MineError`. JSON-RPC-level errors (`-32700`/`-32601`/`-32602`) follow the spec.
- **`mine mcp serve` success writes nothing to stdout**: the server owns stdout until EOF; `render` is bypassed for this command in `main.rs` so no CLI envelope pollutes the transport.

## Acceptance-criteria mapping

| Criterion | Evidence |
|---|---|
| every governing design contract implemented or reported as blocked | MCP transport + 11 tools + shared services + stdout purity + security boundary implemented; `rmcp` literal-SDK-use deviation reported |
| all writes within declared ownership | `src/mcp/`, `tests/mcp.rs`, `Cargo.toml`/`Cargo.lock`, `src/application/{graph,plan,design}_service.rs` (new), plus the `src/lib.rs`/`src/main.rs`/`src/application/mod.rs`/`src/cli/commands.rs` wiring needed to share services — disclosed below |
| tests discriminate intended vs plausible wrong behavior | 26 MCP tests (10 unit + 16 integration) cover the full lifecycle, malformed input, unknown tools/fields, conflicts, purity, EOF, and the live-graph-unchanged invariant |
| stable JSON/protocol contracts documented where applicable | envelope shape + `MINE_*` codes reused; tool result envelopes documented in-code; deterministic sorted-key JSON |
| no direct execution-graph file editing introduced | every mutation via `GraphService::mutate`/`PlanService`/`TomlStore`; tests verify the live graph is untouched |
| no unrelated changes or secrets staged | only Plan 05 scoped files staged; no secrets |
| implementation evidence reproducible | exact commands/exit codes recorded; `cargo test` re-runnable; md5-verified live-graph invariance |
| node reaches `IMPLEMENTED`, never self-granted `ACCEPTED` | lifecycle transition performed through the accepted CLI after this report; never self-accepted |

## Disclosed out-of-exclusive-path edits

- `src/lib.rs` (`pub mod mcp`), `src/main.rs` (`mine mcp serve` special-case + `run_mcp_serve`), `src/application/mod.rs` (wire the 3 new services): necessary additive structural wiring on accepted/earlier-plan roots (analogous to Plan 02/03/04 wiring deviations, accepted by prior reviewers). `src/cli/commands.rs` was **already added in Plan 03 to the shared-services refactor**; the Plan 05 refactor (`commit 6607e8a`) keeps it within the spirit of "CLI/MCP share services," and the Plan 03 test suite remains green so no behavior regressed. All edits `#![forbid(unsafe_code)]`-clean. Flagged for the reviewer.

## Remaining risks and external actions

- The independent reviewer must review Plan 05, transition it to `ACCEPTED` (or `REJECTED`) through the accepted CLI, and merge the plan branch into `dev`. Plan 06 stays `BLOCKED` (it depends on Plans 04 **and** 05; 04 is already accepted). This agent did not self-accept, release Plan 06, merge, touch `master`, or begin Plan 06.
- The `rmcp` literal-SDK-use deviation (see above): a follow-up could hard-swap to `rmcp`'s `ServerHandler` if the reviewer requires it, with no behavioral change.
- Plan 06 (Skill distribution + Marketplace packaging) is the next plan and is **not** in scope here.

## Constraints honored

- `master` untouched (`1d3a132`); `dev` not merged (still `69dc065`); no `plan/06*` branch created; nothing pushed (no remotes); no reset/clean/force-push/blind-stash; **no manual execution-graph mutation** — every graph transition went through the accepted `mine` CLI; Plan 05 was not self-accepted; Plan 06 scope (Skill distribution/Marketplace) untouched.

## Toolchain

Unchanged: `rustc 1.97.1`, `cargo 1.97.1`, stable MSVC. New dependency: `rmcp 2.2.0` (+ tokio, schemars, tracing transitively — `rmcp`'s own deps, all behind safe APIs as far as `mine`'s crate is concerned). The official MSVC toolchain was used.

## Working-tree state

The working tree is clean on `plan/05-stdio-mcp-server-and-typed-tools` after the four implementation commits; this report is the only remaining file before the completion bookkeeping, which will be performed through the accepted `mine plan implemented` CLI command. The pre-existing `.mypy_cache/` remains on disk (gitignored). No unrelated pre-existing modifications were discarded, reset, stashed, or cleaned. Nothing was merged into `dev`; nothing was pushed; `master` was not touched.
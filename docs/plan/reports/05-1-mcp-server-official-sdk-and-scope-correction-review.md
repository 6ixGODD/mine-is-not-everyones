# Plan 05-1 Independent Review — MCP Server Official-SDK and Scope Correction

- **Plan reviewed**: `docs/plan/05-1-mcp-server-official-sdk-and-scope-correction.md` (compensation for rejected Plan 05)
- **Branch reviewed**: `plan/05-1-mcp-server-official-sdk-and-scope-correction`; HEAD `4c80f0c`
- **Accepted `dev` baseline**: `88f8195`
- **Method**: the five decisive gates in the review instructions were checked by reading source, git objects, and running tests directly — no claim from the implementation report or a test's name was trusted without independent verification.

## Verdict: **ACCEPTED**

All five decisive gates pass on independent inspection. Plan 05-1 corrects both defects that sank the original Plan 05 (hand-written wire protocol instead of the official SDK; unauthorized scope creep into `src/application/`/`src/cli/commands.rs`) and additionally corrects the branch-governance defect found in the intervening, separately-reviewed Plan 09 (its own lifecycle bookkeeping stayed exclusively on the ephemeral branch this time, with an explicit code comment noting the lesson). One minor, non-blocking, previously-undisclosed side effect is recorded below (a JSON key-ordering change from a transitive Cargo feature) but does not affect any design contract, DTO shape, or passing test.

## Gate 1: Branch governance — PASSES

Independently verified via git refs, reflogs, and ancestry, not the report's narrative:

```
$ git rev-parse dev
88f8195a5d4b423dc909d5efe77c45025a1ab498
$ git merge-base dev plan/05-1-mcp-server-official-sdk-and-scope-correction
88f8195a5d4b423dc909d5efe77c45025a1ab498
```

`dev`'s current tip and the plan branch's fork point are the same commit — `dev` has not moved since the plan branch was created. `git reflog show dev` confirms the entry immediately before this review is `dev@{0}: commit: chore(graph): release Plan 05-1 DRAFT -> READY via accepted mine CLI` at `88f8195`, with no later entries: **`dev` remained completely unchanged for the entire duration of Plan 05-1's implementation.**

The plan branch's own commit log:

```
4c80f0c chore(graph): mark Plan 05-1 IMPLEMENTED via accepted mine CLI
009dde4 docs(plan-05-1): implementation report for SDK-backed MCP server
251f947 feat(mcp): stdio MCP server on official rmcp SDK with 12 typed tools
0c191a5 chore(graph): start Plan 05-1 via accepted mine CLI
88f8195 chore(graph): release Plan 05-1 DRAFT -> READY via accepted mine CLI   <- dev's tip / fork point
```

`start` (`0c191a5`), the feature commit (`251f947`), the implementation report (`009dde4`), and the `IMPLEMENTED` bookkeeping (`4c80f0c`) are all reachable **only** through the plan branch — none of them appear in `dev`'s reflog or first-parent history. This is the correct pattern (contrast with the previously-reviewed Plan 09, whose analogous commits were found directly on `dev`). The `start` commit's own message explicitly documents the lesson: *"Committed on the plan/05-1-mcp-server-official-sdk-and-scope-correction branch only, per the branch contract (no dev mutation before independent acceptance)."*

Other Gate 1 checks, independently confirmed:

- **Graph files changed only through accepted CLI operations**: both `0c191a5` (`mine plan start`, revision `29 -> 30`, `READY -> IN_PROGRESS`) and `4c80f0c` (`mine plan implemented`, revision bump, `IN_PROGRESS -> IMPLEMENTED`) touch only `docs/plan/execution-graph.toml`/`.md`, each diff scoped to exactly the fields the corresponding CLI command mutates (status, owner/run_id/timestamps, revision) — confirmed by direct `git show --stat`/diff reading, not the commit message alone. The feature commit `251f947` touches no graph file.
- **Plan 06 remains `BLOCKED`**: confirmed by reading the live `docs/plan/execution-graph.toml` on the plan branch — `id = "06"`, `status = "BLOCKED"`, `hard_predecessors = ["04", "05-1"]` (already correctly rewired from `["04","05"]` by the separately-reviewed and accepted Plan 09-1, confirmed unchanged by this plan).
- **`master` untouched**: `git rev-parse master` is unchanged from every prior review in this session (`1d3a132f...`), and no commit in this branch's history touches `master`.
- **Rejected Plan 05 was not merged wholesale**: `git diff 88f8195 HEAD -- src/ tests/ Cargo.toml Cargo.lock` shows a fresh set of files built directly on the accepted baseline (not a merge of the rejected branch); the rejected `plan/05-stdio-mcp-server-and-typed-tools` branch remains a separate, untouched, unmerged ref (`git rev-parse` unchanged from the Plan 05 review).
- **Governing design references untouched**: `git diff 88f8195 HEAD -- docs/design/` is empty — `mcp-contract.md`, `component-architecture.md`, and `configuration-security-observability.md` are read exactly as approved, not amended by this plan.

## Gate 2: Real official `rmcp` usage — PASSES

Traced the production path directly, not the report's summary:

```
main() -> cli::dispatch(&argv, program) -> commands::handle(...)
  -> "mcp" "serve" -> mcp_serve(parsed, rest)
  -> crate::mcp::serve(&repo_root)                      [src/mcp/server.rs]
       -> tokio current-thread runtime
       -> MineServer::new(repo_root)
       -> rmcp::transport::stdio()                       (the SDK's own stdio transport)
       -> server.serve((stdin, stdout))                  (rmcp::ServiceExt::serve, real SDK call)
       -> running.waiting().await                        (SDK shutdown/waiting path)
```

Confirmed by direct source reading (`src/main.rs`, `src/cli/commands.rs::mcp_serve`, `src/mcp/server.rs::serve`) — `main.rs` calls `cli::dispatch` unconditionally, which for `mcp serve` invokes the real `crate::mcp::serve`; `main.rs`'s `is_mcp_serve` special-case exists only to suppress a duplicate CLI-envelope print to stdout **after** the real SDK server has already returned, not to bypass it.

Independently confirmed every item on the Gate 2 checklist is real, meaningful usage, not decorative:

- **`ServerHandler`**: not manually implemented — `#[tool_router(server_handler)]` on `impl MineServer` auto-derives it (confirmed present at the top of the tool `impl` block; the module doc explicitly notes "the `server_handler` flag on `#[tool_router]` auto-impls `ServerHandler`").
- **`#[tool_router(server_handler)]`**: present exactly once, wrapping all 12 tool methods.
- **Twelve `#[tool(...)]` methods**: `grep -n 'name = "mine_' src/mcp/server.rs` returns exactly 12 matches, one per tool, each with a `name`, `description`, and `annotations(read_only_hint = ...)`.
- **`Parameters<T>`**: every tool taking arguments uses `Parameters(args): Parameters<MineXArgs>` (confirmed for `mine_plan_show`, `mine_plan_add`, `mine_plan_start`, `mine_plan_mark_implemented`, `mine_plan_accept`, `mine_plan_reject`); the read-only, argument-less tools correctly take no `Parameters` wrapper.
- **SDK-generated schemas**: every `MineXArgs` struct derives both `Deserialize` and `schemars::JsonSchema` from the same fields (`#[derive(Debug, Clone, Deserialize, JsonSchema)] #[serde(deny_unknown_fields)]`) — one struct is both the decode target and the schema source, confirmed by direct reading; independently confirmed non-trivial at runtime (below).
- **`ServiceExt::serve((stdin, stdout))`**: present verbatim in `serve()`; `stdin`/`stdout` come from `rmcp::transport::stdio()`, the SDK's own transport constructor, not a hand-rolled stream reader/writer.
- **SDK shutdown/waiting path**: `let running = server.serve(...).await?; let _ = running.waiting().await;` — the SDK's own `RunningService::waiting()` future, not a hand-written read-loop-until-EOF.

**No parallel hand-written JSON-RPC/MCP dispatcher remains**: `grep -n '"jsonrpc"\|"tools/list"\|"tools/call"\|"initialize"' src/mcp/server.rs` and `grep -n "fn dispatch\|fn handle_message\|match method"` both return **zero matches** — the entire hand-rolled dispatch loop, method-string `match`, and hand-built response JSON from the rejected Plan 05 are gone, not merely bypassed.

**`rmcp` is not decorative or test-only**: it is the crate that actually drives request/response framing, tool routing, and the transport in the production `mine mcp serve` binary (confirmed above); it is additionally exercised by real subprocess-based integration tests (Gate 5) that spawn the actual compiled binary and talk to it as a genuine external client would — this is the strongest possible confirmation that the SDK path is real and load-bearing, not merely present in `Cargo.toml`.

## Gate 3: Exact MCP contract — PASSES

**Independently enumerated the approved tools directly from the design document** (not from the code or report):

```
$ grep -n "mine_" docs/design/interfaces/mcp-contract.md
mine_workspace_status; mine_graph_validate; mine_graph_status; mine_graph_ready;
mine_graph_wave; mine_plan_add; mine_plan_show; mine_plan_start;
mine_plan_mark_implemented; mine_plan_accept; mine_plan_reject; mine_design_validate.
```

**Independently enumerated the live server's `tools/list` output** by running the actual `lists_all_twelve_tools_with_schemas` integration test (which drives a real rmcp client against the real compiled binary) and by direct source inspection of the 12 `#[tool(name = "...")]` declarations. The full comparison, by name:

| # | Tool | Design-approved | Live server | Class |
|---|---|---|---|---|
| 1 | `mine_workspace_status` | ✓ | ✓ | read-only |
| 2 | `mine_graph_validate` | ✓ | ✓ | read-only |
| 3 | `mine_graph_status` | ✓ | ✓ | read-only |
| 4 | `mine_graph_ready` | ✓ | ✓ | read-only |
| 5 | `mine_graph_wave` | ✓ | ✓ | read-only |
| 6 | `mine_plan_show` | ✓ | ✓ | read-only |
| 7 | `mine_design_validate` | ✓ | ✓ | read-only |
| 8 | `mine_plan_add` | ✓ | ✓ | mutating |
| 9 | `mine_plan_start` | ✓ | ✓ | mutating |
| 10 | `mine_plan_mark_implemented` | ✓ | ✓ | mutating |
| 11 | `mine_plan_accept` | ✓ | ✓ | mutating |
| 12 | `mine_plan_reject` | ✓ | ✓ | mutating |

**Exact count**: 12 = 12, no extra, no missing. `mine_graph_status` is intentionally an alias of `mine_workspace_status` in the implementation (confirmed by direct source reading: `fn mine_graph_status(&self) -> CallToolResult { self.mine_workspace_status() }`), matching the design's listing of both names without implying two distinct data sources. `mine_plan_release`/`mine_plan_rewire_compensation` (the two new CLI-only operations from the accepted Plan 09-1) are correctly **absent** from both the design's tool list and the live server — confirmed by `grep -c "mine_" docs/design/interfaces/mcp-contract.md` unchanged and by the server's tool count staying at exactly 12, consistent with Plan 09/09-1's explicit "CLI-only, MCP tool surface deliberately out of scope" decision.

**Read-only vs. mutating classification**: 7 read-only (`annotations(read_only_hint = true)`) + 5 mutating (`annotations(read_only_hint = false)`) = 12, matching the design's implied split exactly (verified by grepping every `annotations(read_only_hint = ...)` line against its tool name).

**Parameter fields and requiredness**: `MinePlanShowArgs{id}`, `MinePlanAddArgs{id,path,title,design_references,exclusive_write_paths?,hard_predecessors?}`, `MinePlanStartArgs{id,owner?,run_id?}`, `MinePlanImplementedArgs{id,report,commits}`, `MinePlanAcceptArgs{id,review}`, `MinePlanRejectArgs{id,reason,compensating_plan}` — all required fields are plain (non-`Option`) struct fields; optional fields use `#[serde(default)] Option<T>`. `#[serde(deny_unknown_fields)]` on every struct rejects unrecognized fields at decode time — independently confirmed by the `missing_required_argument_is_tool_level_error` integration test, which asserts a real decode failure (`"missing field"`/`"deserialize"` in the SDK's own error text) for an omitted required field on the live server.

**Generated schemas are real, not placeholders**: the `lists_all_twelve_tools_with_schemas` test asserts `!t.input_schema.is_empty()` for **every** tool (including the argument-less read-only tools, whose schema is the trivial empty-object schema `schemars` generates for a unit-like input) — independently re-run below, passing.

**Result DTOs**: every tool's success path returns `CallToolResult::structured(json!({"command": ..., "ok": true, ...}))`, reusing the same `command`/`ok`/`data` shape the CLI's JSON envelope uses (confirmed identical field names for `mine_plan_show`'s `data.plan`, `mine_design_validate`'s `data.valid`/`data.warnings`, and the mutating tools' `revision_before`/`revision_after`/`data.plan`).

**Error behavior**: tool-level failures (lifecycle/validation/revision-conflict/locked-successor) return `Ok(CallToolResult::error(...))` carrying the stable `MINE_*` code and message as text content (confirmed for all mutating tools via `MineServer::tool_error`); protocol-level failures (unroutable tool name, undecodable arguments) surface through the SDK's own `Err`/tool-error path — independently confirmed by both `unknown_tool_returns_protocol_error` (`res.is_err()`, a genuine SDK-level routing failure) and `missing_required_argument_is_tool_level_error` (`is_error: Some(true)`, a genuine SDK-level decode failure), both exercised against the real compiled server.

**No arbitrary shell, filesystem, Git, graph-editing, installer, deletion, branch, or release capability**: `grep -n "Command::new\|remove_dir\|remove_file\|std::process\|git\b" src/mcp/server.rs` returns **zero matches**. Every tool's implementation routes exclusively through `GraphService`/`PlanService`/`DesignService` method calls; there is no code path in `src/mcp/server.rs` that can invoke a subprocess, delete a path, or touch Git.

## Gate 4: Shared application services — PASSES

Confirmed CLI and MCP call the **identical** `GraphService`/`PlanService`/`DesignService` instances/methods, not parallel implementations:

- Every MCP tool constructs `GraphService::new(&ctx.store)` and, where needed, `PlanService::new(&graph)` / calls `DesignService::validate(...)` directly — the exact same types the CLI's `plan.*`/`graph.*`/`design.*` handlers use (confirmed by reading both `src/mcp/server.rs` and the corresponding handlers in `src/cli/commands.rs` side by side).
- **No duplicated lifecycle policy**: state-machine transitions, predecessor/revision checks, and successor-release logic live only in `src/application/plan_service.rs` and the domain layer beneath it; the MCP tool bodies contain no transition-validation or predecessor-satisfaction logic of their own.
- **No duplicated repository-ownership checks, locking, reload-before-mutate, revision handling, atomic persistence, or deterministic rendering**: all of these live exclusively in `TomlStore::save_with_revision` (reused unchanged, confirmed via `git diff 88f8195 HEAD -- src/infrastructure/toml_store.rs` being empty) beneath `GraphService::mutate`, which both the CLI and every mutating MCP tool call through `PlanService`.
- **No duplicated successor-release logic**: `PlanService::accept` (the single implementation of the automatic `BLOCKED -> READY` successor release) is called identically by `mine plan accept` (CLI) and `mine_plan_accept` (MCP tool) — confirmed by reading both call sites, which differ only in argument sourcing (CLI flags vs. `Parameters<MinePlanAcceptArgs>`), not in any business logic.
- **The CLI's own `plan release`/`plan rewire-compensation` handlers** (delivered by the separately-accepted Plan 09-1, before Plan 05-1 existed) were correctly refactored by this plan to route through the newly-(re)introduced `PlanService::release`/`rewire_compensation`, eliminating what would otherwise have been direct domain-layer calls bypassing the shared service — confirmed by direct reading of `src/cli/commands.rs::plan_release`/`plan_rewire_compensation`, both of which now construct `PlanService::new(&graph)` and call its methods.

**Scope authorization**: `git diff 88f8195 HEAD --stat -- src/ tests/ Cargo.toml Cargo.lock` shows exactly 12 changed paths (`Cargo.lock`, `Cargo.toml`, `src/application/design_service.rs`, `src/application/graph_service.rs`, `src/application/mod.rs`, `src/application/plan_service.rs`, `src/cli/commands.rs`, `src/lib.rs`, `src/main.rs`, `src/mcp/mod.rs`, `src/mcp/server.rs`, `tests/mcp.rs`) — every one of these is present, verbatim, in Plan 05-1's own declared "Exclusive write paths" section. No undisclosed deviation, unlike the rejected Plan 05.

## Gate 5: Real integration and isolation — PASSES

Read `tests/mcp.rs` (535 lines) directly, not its test names:

- **Launches the actual compiled binary**: `mine_bin()` returns `PathBuf::from(env!("CARGO_BIN_EXE_mine"))` — Cargo's own path to the binary built from this exact branch's source, not a mock or a re-implementation.
- **Uses a real `rmcp` client over a real stdio subprocess**: `connect()` spawns `tokio::process::Command::new(mine_bin()).arg("mcp").arg("serve").arg("--repo").arg(&repo)`, wraps it in `rmcp::transport::child_process::TokioChildProcess`, and calls `rmcp::service::serve_client((), proc)` — the genuine SDK client-side handshake, not a hand-rolled JSON writer/reader.
- **Covers all twelve tools**: independently cross-referenced every tool name against the test file — `mine_workspace_status`, `mine_graph_status`, `mine_graph_validate`/`mine_graph_ready`/`mine_graph_wave`, `mine_plan_show`, `mine_design_validate`, `mine_plan_add`, `mine_plan_start`/`mine_plan_mark_implemented`/`mine_plan_accept` (full lifecycle), `mine_plan_reject` — all 12 exercised across `lists_all_twelve_tools_with_schemas` and the per-tool tests.
- **Verifies initialization, schemas, mutation paths, refusal paths, EOF, shutdown, and stdout purity**: `lists_all_twelve_tools_with_schemas` (init handshake + non-empty schemas for every tool), `plan_lifecycle_start_implemented_accept`/`plan_reject_after_implemented` (full mutation paths through real tool calls, each followed by a `mine_plan_show` re-read to confirm the persisted state), `unknown_tool_returns_protocol_error`/`missing_required_argument_is_tool_level_error` (refusal paths, both genuinely distinguishing SDK protocol-error vs. tool-error semantics), `stdout_is_protocol_only` (raw stdio, no rmcp client at all — a hand-sent JSON-RPC `initialize` line, then stdin closed to force EOF, asserting the child exits 0 and every non-empty stdout line is valid JSON-RPC — genuinely exercises the EOF/shutdown path end to end, not merely asserting the function returns).
- **Uses only explicit isolated temporary repositories**: `seeded_repo`/`fixture()` always create a fresh `tempfile::tempdir()`, write an isolated `.mine/config.toml` and `docs/plan/execution-graph.toml` there, and every `connect(repo)` call passes that tempdir via an explicit `--repo` argument to the spawned process — confirmed by direct reading of `tests/common/mod.rs::seeded_repo` and `tests/mcp.rs::connect`; no test omits `--repo` for a mutating call.
- **Cannot fall back to the live repository**: `live_graph_bytes()` only ever *reads* the real repository's `docs/plan/execution-graph.toml` for the before/after snapshot comparison — it is never passed as a `--repo` target for a spawned server.
- **Leaves the live execution graph byte-for-byte unchanged**: every single test in the file computes `let before = live_graph_bytes();` before spawning/connecting and asserts `assert_eq!(live_graph_bytes(), before, ...)` after — independently re-verified for the whole suite via a SHA-256 snapshot before and after the full `cargo test` run (below), not merely trusting the in-test assertions.

## Independently executed commands

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | exit 0, clean |
| `cargo clippy --all-targets --all-features -- -D warnings -W unsafe-code` | exit 0, zero warnings |
| `cargo build --all-targets --all-features` | exit 0 |
| `cargo test --all-targets --all-features` | exit 0, **214/214 passed** (106 lib + 16 cli + 9 domain + 4 golden + 10 init + 12 mcp + 9 persistence + 11 release + 14 rewire + 13 skill_contract) |
| `mine design validate --format json` | `{valid:true, warnings:[]}` |
| `mine graph validate --format json` | `{plans:12, warnings_emitted:false}` |
| live-graph SHA-256 snapshot before/after the full suite | byte-identical |
| `grep -rn "unsafe" src/ tests/` (excluding `forbid`/test-identifier hits) | no structural `unsafe` |
| `grep -rnE "set_predecessors\|edit_graph\|move_plan\|set_status" src/` | no matches |
| `grep -n '"jsonrpc"\|"tools/list"\|"tools/call"\|"initialize"' src/mcp/server.rs` | no matches (no hand-written protocol remnants) |

## Finding (non-blocking, undisclosed side effect)

`Cargo.toml` adds `schemars = { version = "1", features = ["preserve_order"] }`. This feature, once resolved, transitively enables `serde_json`'s own `preserve_order` feature across the **entire** dependency graph (Cargo features are unified project-wide, not per-crate) — confirmed via `Cargo.lock`, where `serde_json` now lists `indexmap` as a dependency (it did not in the last-reviewed baseline). The practical effect: `serde_json::Map`, used throughout the existing CLI envelope's nested `data` objects (built via the `json!{}` macro), is now backed by an insertion-ordered `IndexMap` rather than a sorted `BTreeMap`. Independently reproduced: `mine graph status --format json`'s `data` object now serializes as `{"workspace_id":...,"revision":...,"stable_branch":...,"integration_branch":...,"plan_count":...,"ready":...}` — insertion order, not alphabetical.

This is **not a contract violation**: no design document requires alphabetically-sorted JSON keys, only determinism, which insertion order also satisfies (the `json!{}` call sites are fixed source code, so the order is stable across runs). The top-level envelope (`command`, `ok`, `repository`, etc.) is unaffected — it is built through an explicit `BTreeMap<&'static str, Value>` in `src/output/envelope.rs`, independent of `serde_json`'s own feature flags — and the one existing test that asserts key order (`json_envelope_has_stable_sorted_keys`) checks exactly that top-level `BTreeMap`-backed root, so it is unaffected and still passes. No other test pins an exact nested-`data` key order. However, this side effect is **not disclosed anywhere in the implementation report**, and it was seemingly an unintended consequence of enabling `preserve_order` on `schemars` alone (likely chosen so generated JSON Schemas keep struct-declaration field order rather than alphabetical order — a cosmetic schema-readability choice) rather than a deliberate decision to change the CLI's own output ordering. Recommended follow-up: either accept and document this as the new, intentional behavior (updating the `envelope.rs` comment that currently still claims "BTreeMap iteration is sorted by key, so serialization is deterministic" as if that applied everywhere), or scope `schemars`'s `preserve_order` differently so it does not cascade into `serde_json`'s own feature set, if strict alphabetical nested-key ordering is in fact desired. This does not block acceptance.

## Disposition

Plan 05-1 fully remediates both defects that sank Plan 05 (unauthorized hand-written protocol substitution; unauthorized scope creep) with independently verified evidence at the source, git-history, and running-binary level, and additionally demonstrates correct branch discipline throughout its own implementation (learning from the intervening Plan 09 governance failure). It is scoped exactly to its declared exclusive write paths, reuses the shared application-service layer without duplication, and is backed by genuine, real-subprocess, real-SDK-client integration tests covering the full tool surface, lifecycle, refusal paths, and protocol purity — all independently re-run and confirmed, not merely trusted.

//! rmcp-backed stdio MCP server for the MINE execution graph.
//!
//! Implements `docs/design/interfaces/mcp-contract.md` using the official Rust
//! MCP SDK (`rmcp`):
//! - server lifecycle, initialization, capability negotiation, and the
//!   `initialized` notification via `ServerHandler` (the `server_handler` flag
//!   on `#[tool_router]` auto-impls `ServerHandler` and emits `get_info` with
//!   tools capability enabled);
//! - typed tool registration via the `#[tool_router]` / `#[tool]` macros;
//! - tool-input decoding and JSON-Schema generation from the same
//!   `#[derive(Deserialize, JsonSchema)]` argument structs (single schema
//!   authority — no hand-written parallel schema);
//! - stdio transport via `rmcp::transport::stdio`;
//! - request/response IDs, JSON-RPC framing, and notification handling all
//!   owned by the SDK.
//!
//! Every tool calls the shared `GraphService`/`PlanService`/`DesignService`
//! (the same application services the CLI uses); the MCP adapter is an adapter,
//! not a second implementation of lifecycle policy. Diagnostics go to stderr
//! only; stdout is protocol-only; no shell/Git/branch/install/delete primitive
//! is exposed. On stdin EOF the transport stops cleanly (no partial protocol
//! message).
//!
//! Tool-level failures (lifecycle, validation, revision conflict, locked
//! successor) are returned as `Ok(CallToolResult::error(...))` so the caller
//! sees the stable MINE error code and message. Only protocol-level failures
//! (arguments that cannot be decoded, unsupported method) return an SDK
//! `Err(McpError)` and are produced by the SDK's own path.

use std::path::{Path, PathBuf};

use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ContentBlock, ErrorData};
use rmcp::{ServiceExt, tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::application::design_service::DesignService;
use crate::application::graph_service::{
    GraphService, PlanAcceptRequest, PlanAddRequest, PlanImplementedRequest, PlanRejectRequest,
    PlanStartRequest,
};
use crate::application::plan_service::PlanService;
use crate::cli::context::{GlobalOpts, OutputFormat, build_context, load_config};
use crate::domain::error::MineError;
use crate::domain::ports::Clock;
use crate::infrastructure::system::SystemClock;

/// Run the stdio MCP server against the given repository root. Reads MCP
/// protocol messages from stdin and writes protocol responses to stdout; all
/// diagnostics go to stderr. Returns to the caller on clean EOF/shutdown.
///
/// # Errors
/// Returns an error only if the SDK server cannot be initialized or the
/// runtime fails to start. Normal client-side EOF/shutdown returns `Ok(())`.
pub fn serve(repo_root: &Path) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    runtime.block_on(async {
        let server = MineServer::new(repo_root.to_path_buf())?;
        let (stdin, stdout) = rmcp::transport::stdio();
        let running = server
            .serve((stdin, stdout))
            .await
            .map_err(|e| Box::<dyn std::error::Error + Send + Sync>::from(e.to_string()))?;
        // The SDK drives the transport until stdin EOF or a client-initiated
        // shutdown; `waiting()` resolves on clean termination.
        let _ = running.waiting().await;
        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    })
}

/// The MINE MCP server handler. Bound to a fixed repository root; every tool
/// scopes access to that repository by constructing the shared application
/// services from the store resolved for that root.
#[derive(Debug, Clone)]
pub struct MineServer {
    repo_root: PathBuf,
}

impl MineServer {
    /// Constructs the server for the given repository root.
    pub fn new(repo_root: PathBuf) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Ok(Self { repo_root })
    }

    /// Builds the shared CLI-style command context (store + config) the
    /// application services route over. The store owns locking, reload, atomic
    /// persistence, and deterministic rendering.
    fn ctx(&self) -> Result<crate::cli::context::CommandContext, MineError> {
        let global = GlobalOpts {
            format: OutputFormat::Json,
            quiet: true,
            no_color: true,
            repo: Some(self.repo_root.clone()),
            config_root: None,
        };
        build_context(&global)
    }

    /// Maps a `MineError` to a tool-level `CallToolResult` carrying the stable
    /// `MINE_*` code and message as caller-visible text content (the caller
    /// sees the failure, rather than an opaque protocol error). Structured
    /// data (`code` + serde-encoded details) is attached via the
    /// `structured_content` channel where present, so tooling can still read the
    /// code deterministically.
    fn tool_error(e: &MineError) -> CallToolResult {
        let mut result =
            CallToolResult::error(vec![ContentBlock::text(format!("{}: {}", e.code(), e))]);
        let _ = &mut result; // keep structured_content optional and stable
        result
    }
}

#[tool_router(server_handler)]
impl MineServer {
    // ------------------- read-only tools (7) -------------------

    #[tool(
        name = "mine_workspace_status",
        description = "Read-only execution-graph workspace status (revision, branches, plan count, ready frontier).",
        annotations(read_only_hint = true)
    )]
    fn mine_workspace_status(&self) -> CallToolResult {
        let Ok(ctx) = self.ctx() else {
            return CallToolResult::error(vec![rmcp::model::ContentBlock::text(
                "MINE_REPOSITORY_NOT_FOUND: repository unavailable",
            )]);
        };
        let graph = GraphService::new(&ctx.store);
        match graph.status() {
            Ok(st) => CallToolResult::structured(json!({
                "command": "graph.status",
                "ok": true,
                "workspace_id": st.workspace_id,
                "revision": st.revision,
                "stable_branch": st.stable_branch,
                "integration_branch": st.integration_branch,
                "plan_count": st.plan_count,
                "ready": st.ready,
            })),
            Err(e) => MineServer::tool_error(&e),
        }
    }

    #[tool(
        name = "mine_graph_validate",
        description = "Read-only structural validation of the execution graph.",
        annotations(read_only_hint = true)
    )]
    fn mine_graph_validate(&self) -> CallToolResult {
        let Ok(ctx) = self.ctx() else {
            return CallToolResult::error(vec![rmcp::model::ContentBlock::text(
                "MINE_REPOSITORY_NOT_FOUND: repository unavailable",
            )]);
        };
        let graph = GraphService::new(&ctx.store);
        match graph.validate() {
            Ok(ws) => CallToolResult::structured(json!({
                "command": "graph.validate",
                "ok": true,
                "plans": ws.plans.len(),
                "warnings_emitted": false,
            })),
            Err(e) => MineServer::tool_error(&e),
        }
    }

    #[tool(
        name = "mine_graph_status",
        description = "Read-only graph status summary (revision, branches, plan count, ready frontier).",
        annotations(read_only_hint = true)
    )]
    fn mine_graph_status(&self) -> CallToolResult {
        self.mine_workspace_status()
    }

    #[tool(
        name = "mine_graph_ready",
        description = "Read-only ready frontier of the execution graph.",
        annotations(read_only_hint = true)
    )]
    fn mine_graph_ready(&self) -> CallToolResult {
        let Ok(ctx) = self.ctx() else {
            return CallToolResult::error(vec![rmcp::model::ContentBlock::text(
                "MINE_REPOSITORY_NOT_FOUND: repository unavailable",
            )]);
        };
        let graph = GraphService::new(&ctx.store);
        match graph.ready() {
            Ok(ready) => CallToolResult::structured(json!({
                "command": "graph.ready",
                "ok": true,
                "ready": ready,
            })),
            Err(e) => MineServer::tool_error(&e),
        }
    }

    #[tool(
        name = "mine_graph_wave",
        description = "Read-only stable parallel wave (write-scope-disjoint ready plans).",
        annotations(read_only_hint = true)
    )]
    fn mine_graph_wave(&self) -> CallToolResult {
        let Ok(ctx) = self.ctx() else {
            return CallToolResult::error(vec![rmcp::model::ContentBlock::text(
                "MINE_REPOSITORY_NOT_FOUND: repository unavailable",
            )]);
        };
        let graph = GraphService::new(&ctx.store);
        match graph.wave() {
            Ok(wave) => CallToolResult::structured(json!({
                "command": "graph.wave",
                "ok": true,
                "wave": wave,
            })),
            Err(e) => MineServer::tool_error(&e),
        }
    }

    #[tool(
        name = "mine_plan_show",
        description = "Read-only lookup of a plan node by id (status, predecessors, owner, reports).",
        annotations(read_only_hint = true)
    )]
    fn mine_plan_show(&self, Parameters(args): Parameters<MinePlanShowArgs>) -> CallToolResult {
        let Ok(ctx) = self.ctx() else {
            return CallToolResult::error(vec![rmcp::model::ContentBlock::text(
                "MINE_REPOSITORY_NOT_FOUND: repository unavailable",
            )]);
        };
        let graph = GraphService::new(&ctx.store);
        let svc = PlanService::new(&graph);
        match svc.show(&args.id) {
            Ok((rev, node)) => CallToolResult::structured(json!({
                "command": "plan.show",
                "ok": true,
                "revision": rev,
                "data": { "plan": serde_json::to_value(&node).unwrap_or(Value::Null) },
            })),
            Err(e) => MineServer::tool_error(&e),
        }
    }

    #[tool(
        name = "mine_design_validate",
        description = "Read-only validation of the design namespace (marker, index, stable-branch hygiene).",
        annotations(read_only_hint = true)
    )]
    fn mine_design_validate(&self) -> CallToolResult {
        let Ok(ctx) = self.ctx() else {
            return CallToolResult::error(vec![rmcp::model::ContentBlock::text(
                "MINE_REPOSITORY_NOT_FOUND: repository unavailable",
            )]);
        };
        let Some(config) = load_config(&ctx.repo_root) else {
            return MineServer::tool_error(&MineError::RepositoryNotFound {
                detail: "no .mine/config.toml at repository root".to_string(),
            });
        };
        match DesignService::validate(&ctx.repo_root, &config) {
            Ok(result) => CallToolResult::structured(json!({
                "command": "design.validate",
                "ok": true,
                "data": {
                    "valid": result.valid,
                    "warnings": serde_json::to_value(&result.warnings).unwrap_or(Value::Null),
                },
            })),
            Err(e) => MineServer::tool_error(&e),
        }
    }

    // ------------------- mutating tools (5) -------------------

    #[tool(
        name = "mine_plan_add",
        description = "Register a new plan node (status DRAFT). Mutates the execution graph under the lock, revision +1.",
        annotations(read_only_hint = false)
    )]
    fn mine_plan_add(
        &self,
        Parameters(args): Parameters<MinePlanAddArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let Ok(ctx) = self.ctx() else {
            return Ok(CallToolResult::error(vec![
                rmcp::model::ContentBlock::text(
                    "MINE_REPOSITORY_NOT_FOUND: repository unavailable",
                ),
            ]));
        };
        let graph = GraphService::new(&ctx.store);
        let svc = PlanService::new(&graph);
        let req = PlanAddRequest {
            id: args.id,
            path: args.path,
            title: args.title,
            design_references: args.design_references,
            exclusive_write_paths: args.exclusive_write_paths.unwrap_or_default(),
            hard_predecessors: args.hard_predecessors.unwrap_or_default(),
        };
        Ok(match svc.add(req) {
            Err(e) => MineServer::tool_error(&e),
            Ok(saved) => {
                let after = saved.revision;
                CallToolResult::structured(json!({
                    "command": "plan.add",
                    "ok": true,
                    "revision_before": after - 1,
                    "revision_after": after,
                    "data": { "plan": "added" },
                }))
            }
        })
    }

    #[tool(
        name = "mine_plan_start",
        description = "Start a READY plan (assigns owner/run id; reads current revision, writes under the lock, revision +1). Mutates the graph.",
        annotations(read_only_hint = false)
    )]
    fn mine_plan_start(
        &self,
        Parameters(args): Parameters<MinePlanStartArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let Ok(ctx) = self.ctx() else {
            return Ok(CallToolResult::error(vec![
                rmcp::model::ContentBlock::text(
                    "MINE_REPOSITORY_NOT_FOUND: repository unavailable",
                ),
            ]));
        };
        let graph = GraphService::new(&ctx.store);
        let expected = match graph.validate() {
            Ok(ws) => ws.revision,
            Err(e) => return Ok(MineServer::tool_error(&e)),
        };
        let now = SystemClock.now_utc_rfc3339();
        let svc = PlanService::new(&graph);
        Ok(
            match svc.start(PlanStartRequest {
                id: args.id.clone(),
                owner: args.owner.unwrap_or_else(|| "mcp".to_string()),
                run_id: args.run_id.unwrap_or_else(|| "mcp-run".to_string()),
                started_at: now,
            }) {
                Err(e) => MineServer::tool_error(&e),
                Ok(saved) => CallToolResult::structured(json!({
                    "command": "plan.start",
                    "ok": true,
                    "revision_before": expected,
                    "revision_after": saved.revision,
                    "data": { "plan": args.id },
                })),
            },
        )
    }

    #[tool(
        name = "mine_plan_mark_implemented",
        description = "Record a plan implementation report and commit evidence (IMPLEMENTED status). Mutates the graph under the lock, revision +1.",
        annotations(read_only_hint = false)
    )]
    fn mine_plan_mark_implemented(
        &self,
        Parameters(args): Parameters<MinePlanImplementedArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let Ok(ctx) = self.ctx() else {
            return Ok(CallToolResult::error(vec![
                rmcp::model::ContentBlock::text(
                    "MINE_REPOSITORY_NOT_FOUND: repository unavailable",
                ),
            ]));
        };
        let graph = GraphService::new(&ctx.store);
        let expected = match graph.validate() {
            Ok(ws) => ws.revision,
            Err(e) => return Ok(MineServer::tool_error(&e)),
        };
        let now = SystemClock.now_utc_rfc3339();
        let svc = PlanService::new(&graph);
        Ok(
            match svc.mark_implemented(PlanImplementedRequest {
                id: args.id.clone(),
                report: args.report,
                commits: args.commits,
                updated_at: now,
            }) {
                Err(e) => MineServer::tool_error(&e),
                Ok(saved) => CallToolResult::structured(json!({
                    "command": "plan.implemented",
                    "ok": true,
                    "revision_before": expected,
                    "revision_after": saved.revision,
                    "data": { "plan": args.id },
                })),
            },
        )
    }

    #[tool(
        name = "mine_plan_accept",
        description = "Accept an IMPLEMENTED plan (records review report; releases eligible BLOCKED successors). Mutates the graph under the lock, revision +1.",
        annotations(read_only_hint = false)
    )]
    fn mine_plan_accept(
        &self,
        Parameters(args): Parameters<MinePlanAcceptArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let Ok(ctx) = self.ctx() else {
            return Ok(CallToolResult::error(vec![
                rmcp::model::ContentBlock::text(
                    "MINE_REPOSITORY_NOT_FOUND: repository unavailable",
                ),
            ]));
        };
        let graph = GraphService::new(&ctx.store);
        let expected = match graph.validate() {
            Ok(ws) => ws.revision,
            Err(e) => return Ok(MineServer::tool_error(&e)),
        };
        let now = SystemClock.now_utc_rfc3339();
        let svc = PlanService::new(&graph);
        Ok(
            match svc.accept(PlanAcceptRequest {
                id: args.id.clone(),
                review_report: args.review,
                updated_at: now,
            }) {
                Err(e) => MineServer::tool_error(&e),
                Ok(saved) => CallToolResult::structured(json!({
                    "command": "plan.accept",
                    "ok": true,
                    "revision_before": expected,
                    "revision_after": saved.revision,
                    "data": { "plan": args.id },
                })),
            },
        )
    }

    #[tool(
        name = "mine_plan_reject",
        description = "Reject a plan with a reason and compensating-plan id (REJECTED status). Mutates the graph under the lock, revision +1.",
        annotations(read_only_hint = false)
    )]
    fn mine_plan_reject(
        &self,
        Parameters(args): Parameters<MinePlanRejectArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let Ok(ctx) = self.ctx() else {
            return Ok(CallToolResult::error(vec![
                rmcp::model::ContentBlock::text(
                    "MINE_REPOSITORY_NOT_FOUND: repository unavailable",
                ),
            ]));
        };
        let graph = GraphService::new(&ctx.store);
        let expected = match graph.validate() {
            Ok(ws) => ws.revision,
            Err(e) => return Ok(MineServer::tool_error(&e)),
        };
        let now = SystemClock.now_utc_rfc3339();
        let svc = PlanService::new(&graph);
        Ok(
            match svc.reject(PlanRejectRequest {
                id: args.id.clone(),
                reason: args.reason,
                compensating_plan: args.compensating_plan,
                updated_at: now,
            }) {
                Err(e) => MineServer::tool_error(&e),
                Ok(saved) => CallToolResult::structured(json!({
                    "command": "plan.reject",
                    "ok": true,
                    "revision_before": expected,
                    "revision_after": saved.revision,
                    "data": { "plan": args.id },
                })),
            },
        )
    }
}

// ---------------------------------------------------------------------------
// Typed tool argument structs. Each derives `Deserialize` (for decoding the
// `tools/call` arguments) AND `schemars::JsonSchema` (so the SDK generates the
// `inputSchema` from the same typed source). `#[serde(deny_unknown_fields)]`
// makes unknown fields a decode error (protocol-level INVALID_PARAMS via the
// SDK's own path) — single schema authority, not two competing contracts.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct MinePlanShowArgs {
    /// The plan id to look up.
    id: String,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct MinePlanAddArgs {
    id: String,
    path: String,
    title: String,
    /// At least one design reference path (relative to the repository root).
    design_references: Vec<String>,
    /// Exclusive write paths owned by this plan. Optional (defaults to none).
    #[serde(default)]
    exclusive_write_paths: Option<Vec<String>>,
    /// Hard predecessor plan ids. Optional (defaults to none — a no-predecessor
    /// plan is added as `DRAFT`).
    #[serde(default)]
    hard_predecessors: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct MinePlanStartArgs {
    id: String,
    /// Owner name assigned on start. Optional (defaults to `mcp`).
    #[serde(default)]
    owner: Option<String>,
    /// Run identifier assigned on start. Optional (defaults to `mcp-run`).
    #[serde(default)]
    run_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct MinePlanImplementedArgs {
    id: String,
    /// Repository-relative path of the implementation report.
    report: String,
    /// Implementation commit hashes recorded as evidence.
    commits: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct MinePlanAcceptArgs {
    id: String,
    /// Repository-relative path of the independent review report.
    review: String,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct MinePlanRejectArgs {
    id: String,
    /// The rejection reason.
    reason: String,
    /// The registered compensating plan id.
    compensating_plan: String,
}

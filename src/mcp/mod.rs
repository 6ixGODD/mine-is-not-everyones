//! MCP adapter over the shared Application Services.
//!
//! Implements `docs/design/interfaces/mcp-contract.md` using the official Rust
//! MCP SDK (`rmcp`'s `ServerHandler` trait, typed tool router with SDK-generated
//! JSON Schemas, and the stdio transport). The adapter contains no duplicate
//! state-machine, path, backup, or branch policy: every tool calls the shared
//! `GraphService` / `PlanService` / `DesignService` used by the CLI.
//!
//! - [`serve`] runs the stdio MCP server (used by `mine mcp serve`); diagnostics
//!   go to stderr, protocol output to stdout.
//! - [`MineServer`] is the `ServerHandler` implementation exposing exactly the
//!   approved tool surface.

pub mod server;

pub use server::{MineServer, serve};

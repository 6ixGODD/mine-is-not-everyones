//! Deterministic Markdown rendering for the execution graph.
//!
//! The graph's generated Markdown view (`docs/plan/execution-graph.md`) is a
//! stable, human- and agent-readable summary derived deterministically from
//! the TOML machine source. The same TOML always renders to the same Markdown,
//! which is what `mine graph render` guarantees and what golden tests assert.
//!
//! The renderer implementation lives in [`crate::infrastructure::toml_store`]
//! (delivered and tested by the store module). This module re-exposes it as the
//! canonical `render` surface consumed by the CLI and by `mine graph render`,
//! without duplicating the logic or modifying the store-owned module.

use crate::domain::error::MineResult;
use crate::domain::graph::PlanWorkspace;

pub use crate::infrastructure::toml_store::render_markdown;

/// Renders the workspace's Markdown view (deterministic).
///
/// # Errors
/// Returns [`crate::domain::error::MineError::GraphInvalid`] if topological
/// ordering or formatting fails.
pub fn render(ws: &PlanWorkspace) -> MineResult<String> {
    render_markdown(ws)
}

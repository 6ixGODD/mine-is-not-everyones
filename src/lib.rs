//! MINE core library.
//!
//! Plan 01 implements the deterministic initialization service, the
//! design-namespace marker validation, and repository identity/version
//! persistence. The CLI adapter, MCP server, execution-graph state machine,
//! rendering, and distribution are delivered by later plans and consume this
//! library through the application services.

pub mod application;
pub mod domain;
pub mod infrastructure;

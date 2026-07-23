//! Pure domain rules and typed errors.
//!
//! Domain modules contain no filesystem, Git, CLI, or MCP concerns. Plan 01
//! introduced the error model, the project configuration model, the
//! design-namespace marker validation, the repository identity/version logic,
//! and the side-effect ports. Plan 02 extends the domain with the
//! execution-graph aggregate, plan status and transition rules, path safety,
//! design references, and graph validation/algorithms.

pub mod config;
pub mod design_marker;
pub mod design_reference;
pub mod error;
pub mod graph;
pub mod path;
pub mod ports;
pub mod repository_identity;
pub mod status;
pub mod validation;

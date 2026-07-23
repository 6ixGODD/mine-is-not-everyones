//! Pure domain rules and typed errors.
//!
//! Domain modules contain no filesystem, Git, CLI, or MCP concerns. Plan 01
//! introduces the error model, the project configuration model, the
//! design-namespace marker validation, the repository identity/version logic,
//! and the side-effect ports used by the initialization service. Later plans
//! extend the domain with the execution graph, plan lifecycle, transition, and
//! path-conflict rules.

pub mod config;
pub mod design_marker;
pub mod error;
pub mod ports;
pub mod repository_identity;

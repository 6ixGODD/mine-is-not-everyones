//! MINE core library.
//!
//! Plan 01 implements the deterministic initialization service, the
//! design-namespace marker validation, and repository identity/version
//! persistence. The CLI adapter, MCP server, execution-graph state machine,
//! rendering, and distribution are delivered by later plans and consume this
//! library through the application services.

// `AGENTS.md` mandates "Business code must not use `unsafe`." Enforce it at
// compile time across the whole `mine` library crate so the gate cannot be
// silently regressed by a later plan (the default `cargo clippy -D warnings`
// gate does not include the `unsafe_code` lint). Any platform primitive that
// requires `unsafe` (for example file locking) must live inside a vetted
// external dependency, not in this crate.
#![forbid(unsafe_code)]

pub mod application;
pub mod domain;
pub mod infrastructure;

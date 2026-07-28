//! Application services: use cases that orchestrate domain logic and ports.
//!
//! Per `docs/design/system/component-architecture.md`, the CLI and MCP adapters
//! call the **same** application services (no duplicate state-machine, path,
//! backup, or branch policy).
//!
//! - Plan 01: [`init_service`] (setup-only initialization).
//! - Plan 02/03: [`workspace_service`] (internal workspace lifecycle).
//! - Plan 05: [`graph_service`] (read-only graph queries + shared mutation
//!   transaction), [`plan_service`] (plan lifecycle transitions), and
//!   [`design_service`] (read-only design validation/status). These are the
//!   shared services the CLI `plan.*`/`graph.*`/`design.*` handlers and the
//!   MCP `mine_plan_*`/`mine_graph_*`/`mine_design_*` tools both call.

pub mod agent_service;
pub mod design_service;
pub mod doctor_service;
pub mod graph_service;
pub mod init_service;
pub mod plan_service;
pub mod release_service;
pub mod workspace_service;

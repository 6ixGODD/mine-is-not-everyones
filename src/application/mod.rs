//! Application services: use cases that orchestrate domain logic and ports.
//!
//! Plan 01 delivered the setup-only initialization service. Plan 02 delivered
//! the execution-graph domain and safe persistence. Plan 03 adds the internal
//! workspace lifecycle service. Later plans add the agent, doctor, design
//! index, design backup orchestration beyond the infrastructure, and event
//! logging.

pub mod init_service;
pub mod workspace_service;

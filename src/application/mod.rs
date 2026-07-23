//! Application services: use cases that orchestrate domain logic and ports.
//!
//! Plan 01 implements the setup-only initialization service. Later plans add
//! the workspace, graph, plan, design, design-backup, repository-version,
//! distribution, agent, and doctor services.

pub mod init_service;

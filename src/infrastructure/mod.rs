//! Concrete adapters over domain ports.
//!
//! Plan 01 provides the system sources for repository identifiers and time.
//! Plan 02 adds the execution-graph persistence infrastructure: atomic writes,
//! exclusive file locking, and the TOML store that loads/saves the
//! [`crate::domain::graph::PlanWorkspace`] with revision checking and Markdown
//! rendering. Later plans add the Git evidence helper, the repository locator,
//! the design index, design backups, embedded skills, and the event log.

pub mod atomic_write;
pub mod file_lock;
pub mod system;
pub mod toml_store;

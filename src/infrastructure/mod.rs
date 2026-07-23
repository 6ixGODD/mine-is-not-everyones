//! Concrete adapters over domain ports.
//!
//! Plan 01 provides the system sources for repository identifiers and time.
//! Plan 02 added the execution-graph persistence infrastructure: atomic
//! writes, exclusive file locking (via the vetted `fs4` crate), and the TOML
//! store. Plan 03 adds read-only Git evidence and the safe design-backup
//! adapter. Later plans add the design index, repository locator, embedded
//! skills, and the event log.

pub mod atomic_write;
pub mod design_backup;
pub mod file_lock;
pub mod git;
pub mod system;
pub mod toml_store;

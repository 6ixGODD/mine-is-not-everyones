//! Concrete adapters over domain ports.
//!
//! Plan 01 provides the system sources for repository identifiers and time.
//! Later plans add TOML storage, atomic writes, file locks, the Git evidence
//! helper, the repository locator, the design index, design backups, embedded
//! skills, and the event log.

pub mod system;

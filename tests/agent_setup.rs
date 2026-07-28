// Enforce no `unsafe` in MINE-owned test crates.
#![forbid(unsafe_code)]

//! Integration tests: transactional installation, mandatory backup,
//! explicit-root isolation, and selective port verification.
//!
//! Every test drives the real `mine` binary via `cli::dispatch` against an
//! isolated `--config-root <tmp>`. No test reads from or writes to the real
//! user HOME or real Agent configuration.

#[path = "agent_setup/common.rs"]
mod common;
#[path = "agent_setup/doctor_tests.rs"]
mod doctor_tests;
#[path = "agent_setup/install_tests.rs"]
mod install_tests;
#[path = "agent_setup/isolation_tests.rs"]
mod isolation_tests;
#[path = "agent_setup/safety_tests.rs"]
mod safety_tests;
#[path = "agent_setup/transaction_tests.rs"]
mod transaction_tests;
#[path = "agent_setup/uninstall_tests.rs"]
mod uninstall_tests;

// Enforce no `unsafe` in MINE-owned test crates.
#![forbid(unsafe_code)]

//! Plan 08-1 end-to-end tests: release preflight, bootstrap, self-hosting,
//! four-Agent isolated installation, real MCP subprocess discovery, the six
//! Work Package 3 E2E fixtures, and adversarial nested-backup detection.

#[path = "e2e/agent_e2e_tests.rs"]
mod agent_e2e_tests;
#[path = "e2e/bootstrap_tests.rs"]
mod bootstrap_tests;
#[path = "e2e/doctor_stable_tests.rs"]
mod doctor_stable_tests;
#[path = "e2e/fixture_tests.rs"]
mod fixture_tests;
#[path = "e2e/mcp_e2e_tests.rs"]
mod mcp_e2e_tests;
#[path = "e2e/nested_backup_tests.rs"]
mod nested_backup_tests;
#[path = "e2e/release_tests.rs"]
mod release_tests;

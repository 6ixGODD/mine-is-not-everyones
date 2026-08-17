// Enforce no `unsafe` in MINE-owned test crates.
#![forbid(unsafe_code)]

//! `mine plan rewire-compensation --id` integration tests.
//!
//! Drives the CLI over isolated temp repos seeded with controlled graphs; the
//! live repository graph is snapshotted before/after and asserted unchanged.

mod common;

use mine::cli;
use mine::domain::status::PlanStatus;

use common::{dispatch_json, live_graph_bytes,
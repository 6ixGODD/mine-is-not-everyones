//! `mine` executable entry point.
//!
//! Plan 01 delivers the deterministic initialization service, design-namespace
//! marker validation, and repository identity/version persistence as the `mine`
//! library (see [`mine::application::init_service`]). The CLI command
//! dispatcher (`src/cli/`), the JSON envelope output, and the `mine init`
//! argument wiring are introduced in Plan 03. Until then the binary
//! intentionally runs no subcommand and mutates nothing; it only reports that
//! dispatch is unavailable so that no unavailable command is ever pretended to
//! have run.

fn main() -> std::process::ExitCode {
    eprintln!(
        "mine: command dispatch is not wired until Plan 03. Initialization logic is available as a library and verified via `cargo test`."
    );
    std::process::ExitCode::from(2)
}

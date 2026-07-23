//! `mine` executable entry point.
//!
//! Plan 03 wires the CLI adapter (`mine::cli`) into the binary: argument
//! parsing, command dispatch, deterministic JSON / human output, and the
//! public exit-code contract. The binary performs **no** Git mutation and
//! no automatic commit, merge, reset, clean, stash, rebase, push, or branch
//! deletion; mutations are limited to the subcommand-defined graph/config
//! writes implemented under `mine::cli`. The MCP server (`mine mcp serve`)
//! and distribution commands are delivered by later plans.

// Enforce `AGENTS.md`'s "Business code must not use `unsafe`" at compile time
// for the `mine` binary crate as well as the library crate.
#![forbid(unsafe_code)]

use std::io::Write;

use mine::cli;

fn main() -> std::process::ExitCode {
    let argv: Vec<String> = std::env::args().collect();
    let program = argv
        .first()
        .map(|s| s.as_str())
        .unwrap_or("mine")
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or("mine");

    let outcome = cli::dispatch(&argv, program);
    let json = format_is_json(&argv);
    let quiet = argv.iter().any(|a| a == "--quiet");
    let (stdout_text, stderr_text) = cli::render(&outcome, json, quiet);

    {
        let stdout = std::io::stdout();
        let mut lock = stdout.lock();
        let _ = lock.write_all(stdout_text.as_bytes());
        let _ = lock.flush();
    }
    {
        let stderr = std::io::stderr();
        let mut lock = stderr.lock();
        let _ = lock.write_all(stderr_text.as_bytes());
        let _ = lock.flush();
    }

    std::process::ExitCode::from(u8::try_from(outcome.exit_code).unwrap_or(1))
}

/// Returns true if the argument vector requests JSON output
/// (`--format json` or `--format=json`).
fn format_is_json(argv: &[String]) -> bool {
    let mut i = 0;
    while i < argv.len() {
        match argv[i].as_str() {
            "--format=json" => return true,
            "--format" => return argv.get(i + 1).map(|s| s == "json").unwrap_or(false),
            _ => {}
        }
        i += 1;
    }
    false
}

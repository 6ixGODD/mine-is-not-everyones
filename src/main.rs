//! `mine` executable entry point.
//!
//! Wires the CLI adapter (`mine::cli`) into the binary: argument
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

    // Internal refresh-only mode: `mine update` re-runs the newly replaced
    // binary with `__refresh-skills` so installed Agent Skills are refreshed
    // from the NEW binary's embedded payload without requiring `mine setup`.
    // This is an internal entry point, not a user-facing command.
    if argv.iter().any(|a| a == "__refresh-skills") {
        let json = format_is_json(&argv);
        let config_root = argv
            .windows(2)
            .find(|w| w[0] == "--config-root")
            .map(|w| std::path::PathBuf::from(&w[1]));
        let env = match config_root {
            Some(root) => mine::agent_setup::targets::Env::isolated(root),
            None => mine::agent_setup::targets::Env::real_env(),
        };
        let report = mine::application::agent_service::refresh_all_installed(
            &env,
            env!("CARGO_PKG_VERSION"),
        );
        let payload = serde_json::to_string(&report).unwrap_or_else(|_| "{}".to_string());
        let line = if json {
            // Emit the BARE RefreshReport (not an envelope): the parent
            // `mine update` process may be an OLDER binary whose parser
            // expects the report object directly. An envelope with a `data`
            // wrapper would make that older parent report a bogus
            // "refresh report unparseable" error even though the refresh
            // succeeded. Newer parents accept both forms.
            format!("{payload}\n")
        } else {
            let mut s = format!("skills refreshed for {}\n", env!("CARGO_PKG_VERSION"));
            if report.refreshed.is_empty() && report.errors.is_empty() {
                s.push_str("no managed agent installations to refresh\n");
            }
            for a in &report.refreshed {
                s.push_str(&format!("  refreshed: {a}\n"));
            }
            for e in &report.errors {
                s.push_str(&format!("  error: {e}\n"));
            }
            s
        };
        let stdout = std::io::stdout();
        let mut lock = stdout.lock();
        let _ = lock.write_all(line.as_bytes());
        let _ = lock.flush();
        return if report.errors.is_empty() {
            std::process::ExitCode::SUCCESS
        } else {
            std::process::ExitCode::from(4)
        };
    }

    let outcome = cli::dispatch(&argv, program);

    // `mine mcp serve` owns stdio for the MCP transport: the rmcp-backed
    // server writes protocol messages to stdout and reads from stdin. After
    // it returns (clean EOF/shutdown), the CLI must NOT print a human/JSON
    // envelope to stdout — that would contaminate protocol purity. So bypass
    // `render` for `mcp serve` entirely: only the exit code is emitted.
    if is_mcp_serve(&argv) {
        // `mine mcp serve` owns stdio for the MCP transport: the rmcp-backed
        // server writes protocol messages to stdout and reads from stdin.
        // After it returns (clean EOF/shutdown), the CLI must NOT print a
        // human/JSON envelope to stdout — that would contaminate protocol
        // purity. Render the outcome to stderr only (so server errors are
        // still reported) and emit just the exit code to stdout.
        let json = format_is_json(&argv);
        let quiet = argv.iter().any(|a| a == "--quiet");
        let (_stdout_text, stderr_text) = cli::render(&outcome, json, quiet);
        if !stderr_text.is_empty() {
            let stderr = std::io::stderr();
            let mut lock = stderr.lock();
            let _ = lock.write_all(stderr_text.as_bytes());
            let _ = lock.flush();
        }
        return std::process::ExitCode::from(u8::try_from(outcome.exit_code).unwrap_or(1));
    }

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

/// Returns true if the argument vector is the `mcp serve` subcommand (after any
/// global flags). `mine mcp serve` owns stdio as an MCP transport, so its
/// output handling in `main` bypasses the normal CLI envelope render to keep
/// stdout protocol-pure.
fn is_mcp_serve(argv: &[String]) -> bool {
    let mut iter = argv.iter().map(String::as_str);
    // skip program name
    let _ = iter.next();
    let mut saw_mcp = false;
    for a in iter {
        // skip global flags and their values
        match a {
            "--format" => { /* value consumed by format_is_json elsewhere */ }
            "--repo" => { /* value follows */ }
            "--quiet" | "--no-color" => continue,
            s if s.starts_with("--format=") || s.starts_with("--repo=") => continue,
            "mcp" => saw_mcp = true,
            "serve" if saw_mcp => return true,
            _ => saw_mcp = false,
        }
    }
    false
}

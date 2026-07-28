//! `mine` CLI dispatcher, argument parsing, and outcome wiring.
//!
//! Implements the human/JSON CLI surface from
//! `docs/design/interfaces/cli-contract.md`. The dispatcher parses a fixed
//! argument vector (no shell), routes to a command handler, and renders the
//! result as a deterministic JSON envelope (`--format json`) or concise human
//! text (default). It performs **no** Git mutation and supports only
//! read-only/subcommand-defined mutations; no automatic commit, merge, reset,
//! clean, stash, rebase, push, or branch deletion.

pub mod commands;
pub mod context;

use std::path::PathBuf;

use crate::output::envelope::{Envelope, EnvelopeError, ErrorEnvelope};
use crate::output::exit_code;
use crate::output::human::HumanLine;

use context::GlobalOpts;

/// A fully resolved command outcome carrying both the machine envelope and the
/// human rendering so the dispatcher can choose per `--format`.
#[derive(Debug)]
pub struct Outcome {
    pub command: &'static str,
    pub exit_code: i32,
    pub payload: OutcomePayload,
}

#[derive(Debug)]
pub enum OutcomePayload {
    Success {
        envelope: Envelope,
        human: Vec<HumanLine>,
    },
    Error {
        envelope: ErrorEnvelope,
        message: String,
    },
}

/// A handler error: a typed code + message + exit code + optional details.
/// Handlers build this from a [`crate::domain::error::MineError`] or a usage
/// failure.
#[derive(Debug)]
pub struct HandlerError {
    pub code: &'static str,
    pub message: String,
    pub exit_code: i32,
    pub details: serde_json::Value,
}

impl HandlerError {
    /// Builds a handler error from a domain error, mapping it to the public
    /// exit-code contract.
    pub fn from_mine(err: &crate::domain::error::MineError) -> Self {
        Self {
            code: err.code(),
            message: format!("{err}"),
            exit_code: crate::output::exit_code_for(err),
            details: serde_json::Value::Null,
        }
    }

    /// Builds a usage error (invalid invocation).
    pub fn usage(message: impl Into<String>) -> Self {
        Self {
            code: "MINE_USAGE",
            message: message.into(),
            exit_code: exit_code::USAGE,
            details: serde_json::Value::Null,
        }
    }
}

/// A parsed invocation: global options and the remaining command tokens.
#[derive(Debug, Clone)]
pub struct ParsedArgs {
    pub global: GlobalOpts,
    pub tokens: Vec<String>,
}

/// Parses a raw argument vector (program name expected as `argv[0]`).
///
/// Global flags (`--format`, `--quiet`, `--no-color`, `--repo`) may appear
/// before or after the subcommand for ergonomics. Returns a usage error on
/// unknown global flags or a missing subcommand.
pub fn parse(argv: &[String]) -> Result<ParsedArgs, HandlerError> {
    // Skip argv[0] (program name).
    let rest = if argv.is_empty() { &[][..] } else { &argv[1..] };

    let mut format = context::OutputFormat::Human;
    let mut quiet = false;
    let mut no_color = false;
    let mut repo: Option<PathBuf> = None;
    let mut tokens: Vec<String> = Vec::new();

    let mut i = 0;
    while i < rest.len() {
        let a = &rest[i];
        match a.as_str() {
            "--format" => {
                i += 1;
                let v = rest
                    .get(i)
                    .ok_or_else(|| HandlerError::usage("--format requires a value"))?;
                format = match v.as_str() {
                    "json" => context::OutputFormat::Json,
                    "human" => context::OutputFormat::Human,
                    other => {
                        return Err(HandlerError::usage(format!("unknown --format {other:?}")));
                    }
                };
            }
            s if s.starts_with("--format=") => {
                let v = &s["--format=".len()..];
                format = match v {
                    "json" => context::OutputFormat::Json,
                    "human" => context::OutputFormat::Human,
                    other => {
                        return Err(HandlerError::usage(format!("unknown --format {other:?}")));
                    }
                };
            }
            "--quiet" => quiet = true,
            "--no-color" => no_color = true,
            "--repo" => {
                i += 1;
                let v = rest
                    .get(i)
                    .ok_or_else(|| HandlerError::usage("--repo requires a value"))?;
                repo = Some(PathBuf::from(v));
            }
            s if s.starts_with("--repo=") => {
                repo = Some(PathBuf::from(&s["--repo=".len()..]));
            }
            "--help" | "-h" => return Err(HandlerError::usage("help requested")),
            other => tokens.push(other.to_string()),
        }
        i += 1;
    }

    let _ = no_color; // accepted for compatibility; default output is already plain

    if tokens.is_empty() {
        return Err(HandlerError::usage(
            "no subcommand given (try `mine init`, `mine status`, `mine doctor`)",
        ));
    }
    Ok(ParsedArgs {
        global: GlobalOpts {
            format,
            quiet,
            repo,
            no_color,
        },
        tokens,
    })
}

/// Runs the CLI against a raw argument vector and returns the final
/// [`Outcome`] for the caller (`main`) to render and exit on.
///
/// `program` is the binary name used in usage messages.
pub fn dispatch(argv: &[String], program: &str) -> Outcome {
    let parsed = match parse(argv) {
        Ok(p) => p,
        Err(e) => {
            if e.code == "MINE_USAGE" && e.message == "help requested" {
                return usage_outcome(program);
            }
            return error_outcome("usage", e, None, None);
        }
    };

    let tokens = &parsed.tokens;
    let group = tokens[0].as_str();
    // A group may take a subcommand as tokens[1]. If tokens[1] is a flag
    // (starts with `--`) or absent, the group has no subcommand (e.g.
    // `mine setup --agents ...`, `mine release`, `mine status`), so `sub` is
    // empty and the rest starts at index 1.
    let sub_token = tokens.get(1).map(String::as_str).unwrap_or("");
    let has_sub = !sub_token.is_empty() && !sub_token.starts_with("--");
    let sub = if has_sub { sub_token } else { "" };
    let command = command_name(group, sub);

    let rest_start = if has_sub { 2 } else { 1 };
    let rest = &tokens[rest_start..];
    match commands::handle(&parsed, group, sub, rest) {
        Ok((envelope, human)) => Outcome {
            command,
            exit_code: exit_code::SUCCESS,
            payload: OutcomePayload::Success { envelope, human },
        },
        Err(err) => {
            let repo = parsed
                .global
                .repo
                .as_ref()
                .and_then(|p| p.canonicalize().ok())
                .map(|p| p.display().to_string());
            error_outcome(command, err, repo.as_deref(), None)
        }
    }
}

/// Maps a (group, sub) pair to the dotted command identifier emitted in the
/// JSON envelope.
fn command_name(group: &str, sub: &str) -> &'static str {
    match (group, sub) {
        ("init", _) => "init",
        ("status", _) => "status",
        ("doctor", _) => "doctor",
        ("workspace", "open") => "workspace.open",
        ("workspace", "status") => "workspace.status",
        ("workspace", "close") => "workspace.close",
        ("graph", "validate") => "graph.validate",
        ("graph", "render") => "graph.render",
        ("graph", "status") => "graph.status",
        ("graph", "ready") => "graph.ready",
        ("graph", "wave") => "graph.wave",
        ("graph", "show") => "graph.show",
        ("plan", "add") => "plan.add",
        ("plan", "show") => "plan.show",
        ("plan", "start") => "plan.start",
        ("plan", "implemented") => "plan.implemented",
        ("plan", "accept") => "plan.accept",
        ("plan", "reject") => "plan.reject",
        ("plan", "release") => "plan.release",
        ("plan", "rewire-compensation") => "plan.rewire-compensation",
        ("design", "backup") => "design.backup",
        ("design", "validate") => "design.validate",
        ("design", "status") => "design.status",
        ("repository", "version") => "repository.version",
        ("repository", _) => "repository.version",
        ("setup", _) => "setup",
        ("update", _) => "update",
        ("uninstall", _) => "uninstall",
        ("agent", "install") => "agent.install",
        ("agent", "uninstall") => "agent.uninstall",
        ("agent", "status") => "agent.status",
        ("agent", "config") => "agent.config",
        ("release", _) => "release",
        _ => "usage",
    }
}

fn error_outcome(
    command: &'static str,
    err: HandlerError,
    repository: Option<&str>,
    workspace_id: Option<&str>,
) -> Outcome {
    let exit_code = err.exit_code;
    let mut envelope_err = EnvelopeError::new(err.code, err.message.clone());
    if !err.details.is_null() {
        envelope_err = envelope_err.with_details(err.details);
    }
    let mut env = ErrorEnvelope::new(command, envelope_err);
    if let Some(r) = repository {
        env = env.with_repository(r);
    }
    if let Some(w) = workspace_id {
        env = env.with_workspace_id(w);
    }
    Outcome {
        command,
        exit_code,
        payload: OutcomePayload::Error {
            envelope: env,
            message: err.message,
        },
    }
}

fn usage_outcome(program: &str) -> Outcome {
    let msg = format!(
        "usage: {program} <command> [options]\n\
         \nCommands:\n  mine init              initialize repository\n  \
         mine status             show repository and graph status\n  \
         mine doctor             diagnose MINE configuration\n  \
         mine workspace open|status|close\n  \
         mine graph validate|render|status|ready|wave|show\n  \
         mine plan add|show|start|implemented|accept|reject\n  \
         mine design backup|validate|status\n  \
         mine repository version show|suggest|set\n\nOptions:\n  \
         --format json|human     output format (default human)\n  \
         --quiet                 suppress non-error human output\n  \
         --no-color              plain text (default)\n  \
         --repo <path>           repository root override\n"
    );
    error_outcome(
        "usage",
        HandlerError {
            code: "MINE_USAGE",
            message: msg,
            exit_code: exit_code::USAGE,
            details: serde_json::Value::Null,
        },
        None,
        None,
    )
}

/// Renders an [`Outcome`] to `(stdout_text, stderr_text)`. On success, output
/// goes to stdout; on error, the human/json body goes to stderr so stdout
/// stays clean for pipelines consuming JSON only on success.
#[must_use]
pub fn render(outcome: &Outcome, json: bool, quiet: bool) -> (String, String) {
    match &outcome.payload {
        OutcomePayload::Success { envelope, human } => {
            if json {
                (envelope.to_json(), String::new())
            } else if quiet {
                (String::new(), String::new())
            } else {
                let mut out = String::new();
                for line in human {
                    out.push_str(&line.render());
                    out.push('\n');
                }
                (out, String::new())
            }
        }
        OutcomePayload::Error { envelope, message } => {
            if json {
                (String::new(), envelope.to_json())
            } else {
                (String::new(), format!("error: {message}\n"))
            }
        }
    }
}

// A helper for handlers to build a success envelope with repository context.
/// Convenience: returns a base envelope for `command` with the repository root
/// attached.
#[must_use]
pub fn envelope_for(command: &'static str, repo_root: Option<&std::path::Path>) -> Envelope {
    let mut env = Envelope::success(command);
    if let Some(r) = repo_root {
        env = env.with_repository(r.display().to_string());
    }
    env
}

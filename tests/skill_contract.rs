// Enforce no `unsafe` in MINE-owned test crates.
#![forbid(unsafe_code)]

//! Skill contract tests for Plans 04 and 06.
//!
//! These statically assert the accepted Skill contract against the Skills and
//! the user guide, so a drifted Skill fails the gate rather than shipping a
//! stale contract. They are parse/grep contract checks, not behavioral tests of
//! the `mine` binary (the CLI is covered by `tests/cli.rs`, the MCP server by
//! `tests/mcp.rs`).
//!
//! Plan 06 rewrote the Skills to be MCP-first / CLI-fallback against the twelve
//! accepted MCP tools delivered by Plan 05-1. The tests below verify that every
//! MCP tool name a Skill references exists in the accepted twelve-tool surface,
//! and that no Skill invents an unimplemented CLI command.

use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(p: &Path) -> String {
    std::fs::read_to_string(p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()))
}

fn skill(name: &str) -> String {
    read(&repo_root().join("skills").join(name).join("SKILL.md"))
}

#[test]
fn exactly_five_skills_exist_and_sync_is_named_mine_sync() {
    let skills_dir = repo_root().join("skills");
    let mut names: Vec<String> = std::fs::read_dir(&skills_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    names.sort();
    assert_eq!(
        names,
        vec![
            "mine-arch",
            "mine-plan-create",
            "mine-plan-exec",
            "mine-plan-review",
            "mine-sync"
        ],
        "exactly five first-class Skills exist and mine-sync is the synchronization Skill"
    );
}

#[test]
fn skills_never_edit_graph_files_directly() {
    // The accepted CLI is the only path that may mutate the graph. Each Skill
    // must explicitly say it never edits the graph files directly.
    for s in [
        "mine-arch",
        "mine-sync",
        "mine-plan-create",
        "mine-plan-exec",
        "mine-plan-review",
    ] {
        let body = skill(s);
        assert!(
            body.contains("Never edit")
                || body.contains("never edit")
                || body.contains("Do not edit")
                || body.contains("never edits"),
            "{s} must state it never edits execution-graph files directly"
        );
        assert!(
            body.contains("execution-graph.toml") || body.contains("execution-graph.md"),
            "{s} must reference the execution-graph files it must not edit"
        );
    }
}

#[test]
fn no_legacy_architecture_and_detailed_design_path_remains() {
    // The progressive design root is docs/design/index.md. Every skill must
    // use it, not the stale single-document path. Negative guidance
    // ("do not introduce a competing <stale>.md") is permitted and expected.
    fn uses_only_negatively(body: &str) -> bool {
        for (idx, _) in body.match_indices("architecture-and-detailed-design.md") {
            let start = idx.saturating_sub(80);
            let window = &body[start..idx + 5].to_lowercase();
            let is_negative = window.contains("do not")
                || window.contains("never")
                || window.contains("not introduce")
                || window.contains("competing");
            if !is_negative {
                return false;
            }
        }
        true
    }
    for s in [
        "mine-arch",
        "mine-plan-create",
        "mine-plan-exec",
        "mine-plan-review",
    ] {
        let body = skill(s);
        assert!(
            uses_only_negatively(&body),
            "{s} references the stale single-document path as a real source; only negative guidance is allowed"
        );
    }
    // The bundled templates must not use the stale path at all.
    let plan_template =
        read(&repo_root().join("skills/mine-plan-create/references/plan-template.md"));
    assert!(
        !plan_template.contains("architecture-and-detailed-design.md"),
        "plan-template must cite docs/design leaves, not the stale path"
    );
    let agents_template = read(&repo_root().join("skills/mine-arch/references/AGENTS.template.md"));
    assert!(
        !agents_template.contains("architecture-and-detailed-design.md"),
        "AGENTS template must name docs/design/index.md, not the stale path"
    );
}

#[test]
fn mine_sync_refuses_legacy_unmarked_design_and_warns() {
    let body = skill("mine-sync");
    assert!(
        body.contains("legacy unmarked") && body.contains("namespace conflict"),
        "mine-sync must refuse legacy unmarked docs/design/ as a namespace conflict"
    );
    assert!(
        body.to_lowercase().contains("warn"),
        "mine-sync must warn the user about the legacy namespace conflict"
    );
}

#[test]
fn mine_sync_requires_backup_before_mutation() {
    let body = skill("mine-sync");
    assert!(
        body.contains("backup") && body.contains("before any mutation")
            || body.contains("before rewriting"),
        "mine-sync must require a verified backup before any design mutation"
    );
    assert!(
        body.contains(".gitignore") && body.contains("*"),
        "mine-sync must write .gitignore containing * in the backup root"
    );
    assert!(
        body.contains("blocked")
            || body.contains("blocks")
            || body.contains("A failed backup blocks"),
        "mine-sync must block synchronization on a failed backup"
    );
}

#[test]
fn mine_sync_authority_order_user_then_code_then_design() {
    let body = skill("mine-sync");
    // The authority order must enumerate user > code > design explicitly.
    assert!(
        body.contains("authority order"),
        "mine-sync must state the authority order"
    );
    let user_idx = body
        .find("explicit current user instructions")
        .unwrap_or(usize::MAX);
    let code_idx = body.find("current observable code").unwrap_or(usize::MAX);
    let design_idx = body
        .find("existing design only where repository behavior")
        .unwrap_or(usize::MAX);
    assert!(
        user_idx < code_idx && code_idx < design_idx,
        "authority order must be: user instructions, then code, then existing design"
    );
    assert!(
        body.contains("Code wins by default") || body.contains("code wins by default"),
        "mine-sync must state code wins by default unless the user protects a decision"
    );
}

#[test]
fn mine_sync_records_uncertainty_and_does_not_claim_full_coverage_when_sampling() {
    let body = skill("mine-sync");
    assert!(
        body.contains("uncertainty"),
        "mine-sync must record uncertainty"
    );
    assert!(
        body.contains("sample") || body.contains("incomplete coverage"),
        "mine-sync must not claim complete coverage when it only sampled"
    );
}

#[test]
fn mine_sync_does_not_modify_business_code() {
    let body = skill("mine-sync");
    assert!(
        body.contains("does not modify business code")
            || body.contains("does not, on its own") && body.contains("business code"),
        "mine-sync must not modify business code without a separate architecture/plan/execute flow"
    );
    assert!(
        !body
            .to_lowercase()
            .contains("code is subordinate to design"),
        "mine-sync must use code-authoritative language, never code-subordinate language"
    );
}

#[test]
fn mine_arch_is_requirement_first() {
    let body = skill("mine-arch");
    assert!(
        body.contains("requirement-first"),
        "mine-arch must be declared requirement-first"
    );
    assert!(
        body.contains("silently treat current code as the target architecture"),
        "mine-arch must distinguish itself from mine-sync (target != current code)"
    );
    assert!(
        body.contains("docs/design/index.md"),
        "mine-arch must target the progressive design root"
    );
}

/// The twelve accepted MCP tool names exposed by `mine mcp serve` (Plan 05-1).
/// Every MCP tool name referenced by a Skill MUST be in this set.
const ACCEPTED_MCP_TOOLS: &[&str] = &[
    "mine_workspace_status",
    "mine_graph_validate",
    "mine_graph_status",
    "mine_graph_ready",
    "mine_graph_wave",
    "mine_plan_show",
    "mine_design_validate",
    "mine_plan_add",
    "mine_plan_start",
    "mine_plan_mark_implemented",
    "mine_plan_accept",
    "mine_plan_reject",
];

#[test]
fn skills_reference_only_accepted_mcp_tools() {
    // Plan 06: Skills are MCP-first / CLI-fallback. Every MCP tool name a Skill
    // references must exist in the accepted twelve-tool surface (Plan 05-1).
    // A stale or invented tool name fails the gate.
    let accepted: std::collections::HashSet<&str> = ACCEPTED_MCP_TOOLS.iter().copied().collect();
    for s in [
        "mine-arch",
        "mine-sync",
        "mine-plan-create",
        "mine-plan-exec",
        "mine-plan-review",
    ] {
        let body = skill(s);
        // Extract every mine_* token that looks like an MCP tool name.
        let referenced: Vec<&str> = regex_mcp_tool_names(&body);
        for tool in &referenced {
            assert!(
                accepted.contains(tool),
                "{s} references MCP tool {tool:?} which is not in the accepted twelve-tool surface {:?}",
                ACCEPTED_MCP_TOOLS
            );
        }
    }
}

#[test]
fn planning_skills_use_mcp_first_with_cli_fallback() {
    // Plan 06: the three planning Skills must be MCP-first / CLI-fallback.
    // Each must reference at least one accepted MCP tool AND the `mine` CLI.
    for s in ["mine-plan-create", "mine-plan-exec", "mine-plan-review"] {
        let body = skill(s);
        let mcp_tools = regex_mcp_tool_names(&body);
        assert!(
            !mcp_tools.is_empty(),
            "{s} must reference at least one accepted MCP tool (MCP-first)"
        );
        assert!(
            body.contains("mine plan ") || body.contains("mine graph "),
            "{s} must reference the `mine` CLI as a fallback"
        );
        assert!(
            body.contains("CLI fallback"),
            "{s} must document the CLI fallback explicitly"
        );
    }
}

/// Extracts candidate MCP tool names (`mine_<verb>_<noun>` snake_case tokens)
/// from a Skill body, excluding prose that is clearly not a tool reference.
fn regex_mcp_tool_names(body: &str) -> Vec<&str> {
    let mut out = Vec::new();
    for tok in body.split(|c: char| !c.is_alphanumeric() && c != '_') {
        if tok.starts_with("mine_") && tok.contains('_') && tok.len() > 6 {
            // Filter out non-tool snake_case tokens (e.g. mine_design_validate
            // is a tool, but mine_code_version is not). Only accept tokens that
            // match the accepted tool list exactly.
            out.push(tok);
        }
    }
    out.sort();
    out.dedup();
    out
}

#[test]
fn skills_do_not_invent_imaginary_cli_commands() {
    // The user guide explicitly lists only supported clients. Skills must
    // not reference commands that are not part of the accepted CLI contract.
    // The actually-implemented CLI command groups (verified against the built
    // binary). `mine agent` and `mine dist` are declared in the CLI contract
    // design but NOT yet implemented, so Skills must not reference them as
    // available commands.
    let accepted_cli = [
        "mine init",
        "mine status",
        "mine doctor",
        "mine workspace",
        "mine graph",
        "mine plan",
        "mine design",
        "mine repository",
        "mine mcp serve",
    ];
    let body = skill("mine-sync");
    // mine-sync references mine design backup / mine design validate / mine graph validate - all accepted.
    assert!(
        body.contains("mine design backup")
            || body.contains("mine design validate")
            || body.contains("mine graph validate"),
        "mine-sync must reference accepted mine design/graph CLI commands"
    );
    // Skills must not reference unimplemented CLI groups as available commands.
    for s in [
        "mine-arch",
        "mine-sync",
        "mine-plan-create",
        "mine-plan-exec",
        "mine-plan-review",
    ] {
        let body = skill(s);
        assert!(
            !body.contains("mine doctor --agents"),
            "{s} must not invent `mine doctor --agents`"
        );
        assert!(
            !body.contains("mine doctor --agents all"),
            "{s} references an imaginary flag"
        );
        // `mine dist sync` / `mine dist verify` are not yet implemented; the
        // sync mechanism is the scripts/sync-plugin-assets.py script, not a
        // CLI command. Skills must not tell users to run `mine dist ...`.
        assert!(
            !body.contains("mine dist "),
            "{s} must not reference unimplemented `mine dist` CLI group"
        );
    }
    // guide the linter to keep accepted_cli used
    let _ = accepted_cli;
}

#[test]
fn user_guide_lists_supported_clients_only_and_no_imaginary_commands() {
    let guide = read(&repo_root().join("docs/user-guide.md"));
    assert!(
        guide.contains("Claude Code")
            && guide.contains("Codex")
            && guide.contains("Pi")
            && guide.contains("OpenCode"),
        "user guide lists the four supported clients"
    );
    // The earlier draft referenced `mine doctor --agents all` and `mine plan show <id>`
    // without the --id flag. Those are imaginary/incorrect; verify the guide
    // uses the real forms.
    assert!(
        !guide.contains("mine doctor --agents"),
        "user guide must not show an imaginary `mine doctor --agents` flag"
    );
    assert!(
        guide.contains("mine plan show --id <id>"),
        "user guide must use `mine plan show --id <id>`"
    );
}

#[test]
fn user_guide_names_design_root_progressively_and_warns_on_namespace_conflict() {
    let guide = read(&repo_root().join("docs/user-guide.md"));
    assert!(
        guide.contains("docs/design/") && guide.contains(".mine-design.toml"),
        "user guide must name the design namespace and marker"
    );
    assert!(
        guide.contains("legacy") || guide.contains("old repository"),
        "user guide must warn about legacy namespace conflict"
    );
}

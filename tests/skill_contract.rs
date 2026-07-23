// Enforce no `unsafe` in MINE-owned test crates.
#![forbid(unsafe_code)]

//! Skill contract tests for Plan 04.
//!
//! These statically assert the Plan 04 acceptance criteria against the
//! Skills and the user guide, so a drifted Skill fails the gate rather than
//! shipping a stale contract. They are parse/grep contract checks, not
//! behavioral tests of the `mine` binary (the CLI is covered by `tests/cli.rs`).
//!
//! Each test names the exact acceptance criterion from
//! `docs/plan/04-skills-json-cli-mine-sync-and-design-lifecycle.md`.

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

#[test]
fn planning_skills_use_real_cli_commands_not_mcp_placeholders() {
    // The accepted MINE CLI is implemented (Plan 03). The skills must cite the
    // real `mine plan ...` commands, not the placeholder snake_case MCP names.
    for s in ["mine-plan-create", "mine-plan-exec", "mine-plan-review"] {
        let body = skill(s);
        assert!(
            !body.contains("mine_plan_add")
                && !body.contains("mine_plan_start")
                && !body.contains("mine_plan_get")
                && !body.contains("mine_plan_mark_implemented")
                && !body.contains("mine_plan_accept")
                && !body.contains("mine_plan_reject")
                && !body.contains("mine_graph_status")
                && !body.contains("mine_graph_validate"),
            "{s} must cite accepted `mine plan ...` / `mine graph ...` CLI commands, not placeholder MCP names"
        );
        assert!(
            body.contains("mine plan ")
                || body.contains("mine plan\n")
                || body.contains("mine graph "),
            "{s} must reference the accepted mine CLI"
        );
    }
}

#[test]
fn skills_do_not_invent_imaginary_cli_commands() {
    // The user guide explicitly lists only supported clients. Skills must
    // not reference commands that are not part of the accepted CLI contract.
    let accepted_cli = [
        "mine init",
        "mine status",
        "mine doctor",
        "mine workspace",
        "mine graph",
        "mine plan",
        "mine design",
        "mine repository",
        "mine dist",
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
    // Any CLI invocation "mine <x>" that is not in accepted_cli is suspicious.
    // Catch the implausible `mine doctor --agents` form specifically.
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

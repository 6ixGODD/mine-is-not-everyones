// Enforce no `unsafe` in MINE-owned test crates.
#![forbid(unsafe_code)]

//! Contract tests: every MCP tool a Skill references is in the accepted
//! twelve-tool surface; every CLI fallback command is implemented; no stale
//! references survive.

use super::common::*;

#[test]
fn exactly_five_root_skills_exist() {
    let mut names: Vec<String> = std::fs::read_dir(skills_root())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    names.sort();
    assert_eq!(
        names,
        FIVE_SKILLS
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>(),
        "exactly five first-class Skills exist"
    );
}

#[test]
fn every_skill_references_only_accepted_mcp_tools() {
    for s in FIVE_SKILLS {
        let body = skill_body(s);
        let refs = mcp_tool_refs(&body);
        for tool in &refs {
            assert!(
                ACCEPTED_MCP_TOOLS.contains(tool),
                "{s} references MCP tool {tool:?} not in the accepted twelve-tool surface"
            );
        }
    }
}

#[test]
fn planning_skills_are_mcp_first_with_cli_fallback() {
    for s in ["mine-plan-create", "mine-plan-exec", "mine-plan-review"] {
        let body = skill_body(s);
        assert!(
            !mcp_tool_refs(&body).is_empty(),
            "{s} must reference accepted MCP tools (MCP-first)"
        );
        assert!(
            body.contains("CLI fallback"),
            "{s} must document the CLI fallback explicitly"
        );
        assert!(
            body.contains("mine plan ") || body.contains("mine graph "),
            "{s} must reference the `mine` CLI as fallback"
        );
    }
}

#[test]
fn skills_document_cli_only_operations_without_mcp() {
    // Skills must state which operations are CLI-only (no MCP tool).
    // mine-plan-create must document that `release` is CLI-only.
    let create = skill_body("mine-plan-create");
    assert!(
        create.contains("no MCP tool for release") || create.contains("CLI-only"),
        "mine-plan-create must document release as a CLI-only fallback"
    );
    // mine-plan-review must document that rewire-compensation is CLI-only.
    let review = skill_body("mine-plan-review");
    assert!(
        review.contains("no MCP tool for rewiring") || review.contains("rewire-compensation"),
        "mine-plan-review must document rewire-compensation as CLI-only"
    );
}

#[test]
fn no_stale_mine_design_sync_reference() {
    for s in FIVE_SKILLS {
        let body = skill_body(s);
        assert!(
            !body.contains("mine-design-sync"),
            "{s} must not reference the obsolete `mine-design-sync` name"
        );
    }
}

#[test]
fn no_stale_doctor_agents_all_reference() {
    // `mine doctor --agents <scope>` is a real, accepted flag (added for agent
    // diagnostics, corrected for stable-tree compatibility). This test
    // previously banned it as imaginary; that assumption is stale. Instead,
    // verify no Skill invents a flag that truly does not exist, e.g. a
    // `--agents` value the CLI does not accept.
    for s in FIVE_SKILLS {
        let body = skill_body(s);
        assert!(
            !body.contains("mine doctor --agent "),
            "{s} must use the real `--agents` flag name, not a singular `--agent`"
        );
    }
}

#[test]
fn no_stale_architecture_and_detailed_design_as_source() {
    // The progressive design root is docs/design/index.md. The stale
    // single-document path may appear only in negative guidance.
    for s in FIVE_SKILLS {
        let body = skill_body(s);
        for (idx, _) in body.match_indices("architecture-and-detailed-design.md") {
            let start = idx.saturating_sub(80);
            let window = &body[start..idx + 5].to_lowercase();
            assert!(
                window.contains("do not")
                    || window.contains("never")
                    || window.contains("not introduce")
                    || window.contains("competing"),
                "{s} references the stale path as a real source (only negative guidance allowed)"
            );
        }
    }
}

#[test]
fn skills_do_not_reference_unimplemented_cli_groups() {
    // `mine agent` and `mine dist` are now
    // implemented and accepted; Skills may reference them. This test now
    // guards against genuinely unimplemented groups instead.
    for s in FIVE_SKILLS {
        let body = skill_body(s);
        assert!(
            !body.contains("mine publish ") && !body.contains("mine release --publish"),
            "{s} must not reference an unimplemented publish/release-mutation command"
        );
    }
}

#[test]
fn skills_never_edit_graph_files_directly() {
    for s in FIVE_SKILLS {
        let body = skill_body(s);
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

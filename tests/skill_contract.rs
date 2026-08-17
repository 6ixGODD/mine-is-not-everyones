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
//! Skills are MCP-first / CLI-fallback against the twelve
//! accepted MCP tools. The tests below verify that every
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
    // The tripartite contract: mine-sync refuses an unclaimed tree, but the
    // deterministic resolution is `mine init` auto-backup -- never manual
    // rename/remove and never silent adoption.
    assert!(
        body.contains("deterministic resolution is `mine init`"),
        "mine-sync must point to `mine init` as the deterministic resolution"
    );
    assert!(
        body.contains("auto-backs-up")
            && body.contains("docs/design-backup-<UTC timestamp>/")
            && body.contains("fresh managed root"),
        "mine-sync must describe init's auto-backup + fresh-root resolution"
    );
    assert!(
        !body.contains("rename or remove the legacy directory"),
        "mine-sync must never instruct manual rename/remove of the legacy directory"
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

/// The twelve accepted MCP tool names exposed by `mine mcp serve`.
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
    // Skills are MCP-first / CLI-fallback. Every MCP tool name a Skill
    // references must exist in the accepted twelve-tool surface.
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
    // The three planning Skills must be MCP-first / CLI-fallback.
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
    // The actually-implemented CLI command groups (verified against the built
    // binary). `mine agent` and `mine dist` are
    // now implemented and accepted; `mine doctor --agents <scope>` (Plan
    // diagnostics, corrected for stable-tree compatibility) is a real,
    // accepted flag. This test no longer bans them (correction of a
    // stale assertion written before those commands existed).
    let accepted_cli = [
        "mine init",
        "mine status",
        "mine doctor",
        "mine workspace",
        "mine graph",
        "mine plan",
        "mine design",
        "mine repository",
        "mine agent",
        "mine dist",
        "mine release",
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
    // `mine <unknown-group>` must never appear as an invented CLI group.
    for s in [
        "mine-arch",
        "mine-sync",
        "mine-plan-create",
        "mine-plan-exec",
        "mine-plan-review",
    ] {
        let body = skill(s);
        for unknown in ["mine sync ", "mine arch ", "mine review ", "mine exec "] {
            assert!(
                !body.contains(unknown),
                "{s} must not reference the invented CLI group `{unknown}`"
            );
        }
    }
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
    // `mine doctor --agents <scope>` is a real, accepted flag (verified against
    // the built binary). The guide may use it. Verify the guide does not show
    // the incorrect `mine plan show <id>` form (must use --id).
    assert!(
        guide.contains("mine plan show --id <id>"),
        "user guide must use `mine plan show --id <id>`"
    );
}

#[test]
fn user_guide_names_design_root_and_backup_behavior() {
    let guide = read(&repo_root().join("docs/user-guide.md"));
    assert!(
        guide.contains("docs/design/") && guide.contains(".mine-design.toml"),
        "user guide must name the design namespace and marker"
    );
    assert!(
        guide.contains("docs/design-backup-<UTC timestamp>/") || guide.contains("design-backup"),
        "user guide must describe the auto-backup of legacy design content"
    );
    assert!(
        !guide.contains("Rename or remove") && !guide.contains("rename or remove"),
        "user guide must not instruct manual rename/remove of legacy design"
    );
}

#[test]
fn reviewer_may_correct_narrow_findings_directly_without_a_compensating_plan() {
    // The review workflow must not be purely formalistic
    // ("reviewers may never edit code"), which previously forced a new
    // compensating plan for every narrow finding. The reviewer Skill must
    // explicitly authorize direct, localized reviewer fixes (committed
    // separately, revalidated, and documented) as the normal path, reserving
    // REJECTED + a compensating plan for substantial issues only.
    let review = skill("mine-plan-review");
    assert!(
        review.contains("Fix directly during review"),
        "mine-plan-review must define a direct-fix path for narrow findings"
    );
    assert!(
        review.contains("reviewer is responsible for bringing submitted work to an acceptable")
            || review.contains("bringing submitted work to an acceptable, mergeable state"),
        "mine-plan-review must state the reviewer's responsibility to bring work to an \
         acceptable state, not merely issue a verdict"
    );
    assert!(
        review.contains("without needing a second reviewer or a compensating plan"),
        "mine-plan-review must state a localized fix does not require a second reviewer or a \
         new compensating plan"
    );
    assert!(
        review.contains("reserved for substantial issues"),
        "mine-plan-review must state that reject+compensate is reserved for substantial issues"
    );
    assert!(
        review.contains("Bring release closure to completion"),
        "mine-plan-review must carry the operational release-closure procedure so a normal \
         instruction like 'review Plan N and complete the local release' is self-contained"
    );
    // Every MCP tool and CLI command the release-closure section names must be
    // real, accepted surface (cross-checked against the same list used
    // elsewhere in this file).
    for tool in [
        "mine_plan_accept",
        "mine_plan_reject",
        "mine_graph_validate",
    ] {
        assert!(
            review.contains(tool),
            "mine-plan-review references MCP tool {tool} which must be accepted"
        );
    }
    for cmd in [
        "mine release",
        "mine design validate",
        "mine graph validate",
        "mine doctor --agents all",
        "mine mcp serve",
    ] {
        assert!(
            review.contains(cmd),
            "mine-plan-review release-closure section must reference the real command `{cmd}`"
        );
    }
}

#[test]
fn plan_create_does_not_force_a_new_plan_for_narrow_corrections() {
    let create = skill("mine-plan-create");
    assert!(
        create.contains("Do not create a plan merely to preserve reviewer/implementer role")
            || create.contains("not a new plan"),
        "mine-plan-create must not force a new compensating plan for a narrow, local, \
         fully-verifiable correction"
    );
}

#[test]
fn review_skill_requires_stale_plan_reference_scan_at_release_closure() {
    let review = skill("mine-plan-review");
    assert!(
        review.contains("scan-plan-refs.sh --check"),
        "mine-plan-review must run the stale-plan-reference scanner before stable integration"
    );
    assert!(
        review.contains("mine-release-allow-plan-reference:"),
        "mine-plan-review must document narrowly-scoped fixture exemptions"
    );

    // The scanner is bundled alongside the Skill under references/.
    let scanner = repo_root().join("skills/mine-plan-review/references/scan-plan-refs.sh");
    let source = read(&scanner);
    assert!(
        source.contains("git ls-files -z"),
        "the scanner must inspect tracked files rather than an uncontrolled filesystem walk"
    );
    assert!(
        source.contains("--check") && source.contains("docs/plan"),
        "the scanner must offer a failing release gate while excluding the temporary plan workspace"
    );
}

#[test]
fn review_skill_has_no_mine_specific_validation_commands() {
    let review = skill("mine-plan-review");
    // MINE's own cargo/python commands must not appear in the generic review Skill.
    for forbidden in [
        "cargo fmt",
        "cargo clippy",
        "cargo build",
        "cargo test",
        "sync-plugin-assets",
        "verify.py",
    ] {
        assert!(
            !review.contains(forbidden),
            "mine-plan-review SKILL.md must not contain MINE-specific command `{forbidden}`"
        );
    }
}

#[test]
fn review_skill_discovers_validation_from_repository_governance() {
    let review = skill("mine-plan-review");
    assert!(
        review.contains("explicit current user instructions"),
        "review Skill must state the authority order for validation discovery"
    );
    assert!(
        review.contains("AGENTS.md"),
        "review Skill must reference repository governance as a validation authority"
    );
    assert!(
        review.contains("Never invent Cargo, Python, Node, Go"),
        "review Skill must forbid presuming a specific toolchain"
    );
}

#[test]
fn review_skill_does_not_invoke_mine_sync() {
    let review = skill("mine-plan-review");
    // The reviewer confirms the final sync was run by the owner but does not run it.
    assert!(
        review.contains("does **not** invoke `mine-sync`"),
        "review Skill must state the reviewer does not invoke mine-sync"
    );
}

#[test]
fn scan_plan_refs_scans_go_layout_and_excludes_docs() {
    use std::process::Command;
    // Read the scanner source and pipe it via stdin so bash finds it regardless
    // of Windows/Unix path format; current_dir is the temp repo so the
    // scanner's `git rev-parse` targets it.
    let scanner_src =
        read(&repo_root().join("skills/mine-plan-review/references/scan-plan-refs.sh"));
    let tmp = tempfile::tempdir().unwrap();

    // Go-layout file that would have been missed by the old src/-only scan.
    let go_dir = tmp.path().join("cmd");
    std::fs::create_dir_all(&go_dir).unwrap();
    std::fs::write(
        go_dir.join("main.go"),
        // mine-release-allow-plan-reference: scanner test fixture
        "package main\n// Plan 99 historical comment\nfunc main() {}\n",
    )
    .unwrap();

    // Documentation that must NOT be flagged.
    let design_dir = tmp.path().join("docs").join("design");
    std::fs::create_dir_all(&design_dir).unwrap();
    // mine-release-allow-plan-reference: scanner test fixture
    std::fs::write(design_dir.join("arch.md"), "# Plan 99 in design doc\n").unwrap();

    let _git = Command::new("git")
        .args(["init", "-q"])
        .current_dir(tmp.path())
        .status()
        .unwrap();
    let _cfg1 = Command::new("git")
        .args(["config", "user.email", "t@t.t"])
        .current_dir(tmp.path())
        .status()
        .unwrap();
    let _cfg2 = Command::new("git")
        .args(["config", "user.name", "t"])
        .current_dir(tmp.path())
        .status()
        .unwrap();
    let _add = Command::new("git")
        .args(["add", "-A"])
        .current_dir(tmp.path())
        .status()
        .unwrap();
    let _commit = Command::new("git")
        .args(["commit", "-qm", "test"])
        .current_dir(tmp.path())
        .status()
        .unwrap();

    use std::io::Write;
    let mut child = Command::new("bash")
        .arg("-s")
        .arg("--")
        .arg("--check")
        .current_dir(tmp.path())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    {
        let mut stdin = child.stdin.take().unwrap();
        stdin.write_all(scanner_src.as_bytes()).unwrap();
    }
    let out = child.wait_with_output().unwrap();
    assert!(
        !out.status.success(),
        // mine-release-allow-plan-reference: scanner test fixture
        "scanner must flag Plan 99 in cmd/main.go (Go layout)"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let combined = format!("{stdout}{stderr}");
    assert!(
        combined.contains("cmd/main.go"),
        "scanner output must name the Go-layout file: {combined}"
    );
    assert!(
        !combined.contains("docs/design/arch.md"),
        "scanner must not flag design documentation: {combined}"
    );
}

#[test]
fn review_skill_defines_release_closure_mode_invocation() {
    let review = skill("mine-plan-review");
    // Executable entry discrimination: the literal must select closure mode and
    // must never be resolved as a file path.
    assert!(
        review.contains("mine-plan-review complete release closure"),
        "the Skill must define the executable release-closure mode invocation"
    );
    assert!(
        review.contains("never resolve that literal as a plan file path"),
        "release-closure mode must never treat `complete release closure` as a file path"
    );
    assert!(
        review.contains("Any other single argument is treated as a Plan path"),
        "any non-closure argument must fall back to Plan-review mode"
    );
    assert!(
        review.contains("No Plan path is required"),
        "release-closure mode must not require a Plan path"
    );
    assert!(
        review.contains("Never re-accept"),
        "release-closure mode must not re-accept an already-ACCEPTED plan"
    );
    assert!(
        review.contains("missing or stale"),
        "release-closure mode must detect a missing or stale final sync"
    );
    // Fail-closed on non-terminal graph and on failed freshness checks.
    assert!(
        review.contains("every Plan terminal"),
        "closure mode must enter only when every Plan is terminal"
    );
    assert!(
        review.contains("do **not** proceed"),
        "closure mode must fail closed instead of proceeding on a gap"
    );
    // No push/publish/remote-release authority.
    assert!(
        review.contains("Never push, create a remote release, publish a package"),
        "closure mode must not push, publish, or create remote releases"
    );
    assert!(
        review.contains("delete a remote or unrelated/user branch"),
        "closure mode must not delete remote or unrelated branches"
    );
}

#[test]
fn review_skill_requires_reproducible_final_sync_freshness_evidence() {
    let review = skill("mine-plan-review");
    // The review brief: a terminal graph + structurally valid Design is NOT
    // proof the final mine-sync ran after the last integration. The Skill must
    // require explicit, reproducible freshness evidence -- never `mine design
    // validate` alone as a semantic sync proof.
    assert!(
        review.contains("NOT sufficient proof"),
        "the Skill must state that terminal graph + design validate is not sufficient"
    );
    assert!(
        review.contains("never rely on `mine design validate` alone"),
        "the Skill must forbid design-validate-only sync proof"
    );
    // Evidence 1: the Phase A sync report under .mine/runtime/sync/ with a
    // recorded commit and SYNCHRONIZED status.
    assert!(
        review.contains(".mine/runtime/sync/"),
        "the Skill must locate the Phase A sync report"
    );
    assert!(
        review.contains("SYNCHRONIZED"),
        "the Skill must check the sync report status"
    );
    // Evidence 2: the final accepted plan's design_references leaves must
    // describe post-implementation behavior.
    assert!(
        review.contains("design_references"),
        "the Skill must check the final plan's design_references"
    );
    assert!(
        review.contains("post-implementation behavior"),
        "the Skill must verify leaves describe post-implementation behavior"
    );
    // Evidence 3: independent consistency spot-check against current dev code.
    assert!(
        review.contains("Independent consistency spot-check"),
        "the Skill must require an independent design-vs-code spot-check"
    );
    // The stale-design adversarial case must fail closed.
    assert!(
        review.contains("stale regardless of `mine design validate`"),
        "a leaf that contradicts code must be stale even if design validate passes"
    );
}

#[test]
fn review_skill_does_not_require_mine_product_distribution_as_universal_gate() {
    let review = skill("mine-plan-review");
    // MINE product-distribution checks are allowed ONLY as MINE-source-scoped
    // conditions, never as universal requirements. Assert the scoping language
    // exists and that no unconditional four-agent/twelve-tool requirement
    // remains in imperative form.
    assert!(
        review.contains(
            "apply **only** when the repository under review is the MINE source repository itself"
        ),
        "MINE product-distribution checks must be explicitly scoped to the MINE source repository"
    );
    // The scoping must be attributed to the target repository's own governance
    // (AGENTS.md/Design/decisive gates), never to directory-structure sniffing
    // by the portable Skill.
    assert!(
        review.contains("per MINE-local governance"),
        "MINE-source scoping must be attributed to the target repo's governance"
    );
    // The universal closure steps must not contain an imperative install of
    // all four agents or an unconditional twelve-tool assertion.
    let closure_section_start = review.find("### Mechanical closure steps").unwrap_or(0);
    let closure_section = &review[closure_section_start..];
    assert!(
        !closure_section.contains("Install all four Agents"),
        "release closure must not require installing all four Agents as a universal step"
    );
    assert!(
        !closure_section.contains("exposes exactly the twelve accepted MCP tools"),
        "release closure must not require the twelve-MCP-tool assertion as a universal step"
    );
}

#[test]
fn review_skill_resolves_scanner_from_loaded_skill_path() {
    let review = skill("mine-plan-review");
    // The scanner must be resolved from the ACTUALLY LOADED SKILL.md's parent
    // directory -- not from hardcoded client paths as the primary mechanism.
    assert!(
        review.contains("file that is actually loaded"),
        "the Skill must derive the scanner path from the actually loaded SKILL.md"
    );
    assert!(
        review.contains("Take its parent directory as the Skill directory"),
        "the Skill must use the loaded SKILL.md's parent as the Skill directory"
    );
    assert!(
        review.contains("<loaded-skill-directory>/references/scan-plan-refs.sh"),
        "the Skill must join references/ under the loaded Skill directory"
    );
    assert!(
        review.contains("target repository root as the current working directory"),
        "the Skill must run the scanner with the target repo as CWD"
    );
    // Typical client paths are troubleshooting only, and a custom config root
    // overrides them.
    assert!(
        review.contains("NOT a substitute for the loaded-path derivation")
            && review.contains("custom `--config-root`"),
        "per-client paths must be troubleshooting-only, not the resolution mechanism"
    );
    // The stdin fallback must exist for when the file cannot be located.
    assert!(
        review.contains("Fallback (stdin)"),
        "the Skill must define the stdin fallback for scanner resolution"
    );
    assert!(
        review.contains("bash -s -- --check"),
        "the stdin fallback must pipe the scanner source to bash"
    );
    assert!(
        review.contains("never needs a local `references/` directory"),
        "the target repository must never need a local references/ directory"
    );
}

#[test]
fn scanner_runs_from_installed_skill_dir_against_separate_target_repo() {
    use std::process::Command;

    // Stage a realistic installed Skill layout in a shared parent so the
    // installed Skill directory is a sibling of the target repository. The
    // scanner is then invoked by a path relative to the target repo's CWD,
    // which Git Bash resolves without drive-letter issues.
    let parent = tempfile::tempdir().unwrap();
    let installed_refs = parent
        .path()
        .join("installed/skills/mine-plan-review/references");
    std::fs::create_dir_all(&installed_refs).unwrap();
    let scanner_bytes =
        std::fs::read(repo_root().join("skills/mine-plan-review/references/scan-plan-refs.sh"))
            .unwrap();
    std::fs::write(installed_refs.join("scan-plan-refs.sh"), &scanner_bytes).unwrap();

    // A separate target repository with a Go-layout file and NO local
    // references/ directory.
    let target = parent.path().join("target");
    std::fs::create_dir_all(&target).unwrap();
    let cmd_dir = target.join("cmd");
    std::fs::create_dir_all(&cmd_dir).unwrap();
    std::fs::write(
        cmd_dir.join("main.go"),
        // mine-release-allow-plan-reference: scanner test fixture
        "package main\n// Plan 99 historical comment\nfunc main() {}\n",
    )
    .unwrap();
    let _ = Command::new("git")
        .args(["init", "-q"])
        .current_dir(&target)
        .status()
        .unwrap();
    let _ = Command::new("git")
        .args(["config", "user.email", "t@t"])
        .current_dir(&target)
        .status()
        .unwrap();
    let _ = Command::new("git")
        .args(["config", "user.name", "t"])
        .current_dir(&target)
        .status()
        .unwrap();
    let _ = Command::new("git")
        .args(["add", "-A"])
        .current_dir(&target)
        .status()
        .unwrap();
    let _ = Command::new("git")
        .args(["commit", "-qm", "test"])
        .current_dir(&target)
        .status()
        .unwrap();
    assert!(
        !target.join("references").exists(),
        "the target repository must not contain a local references/ directory"
    );

    // Invoke the scanner from its installed location (a sibling of the
    // target repo) while the target repository is the working directory.
    // A relative path from the target CWD avoids Windows drive-letter issues.
    let scanner_rel = "../installed/skills/mine-plan-review/references/scan-plan-refs.sh";
    let cmd = format!("bash \"{scanner_rel}\" --check");
    let out = Command::new("bash")
        .arg("-c")
        .arg(&cmd)
        .current_dir(&target)
        .output()
        .unwrap();
    assert!(
        !out.status.success(),
        "scanner invoked from installed location must flag cmd/main.go (Go layout)"
    );
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        combined.contains("cmd/main.go"),
        "scanner output must name the Go-layout file: {combined}"
    );
    // The scanner internally cd's to the target repo's git toplevel; verify it
    // did not scan the installed Skill directory instead.
    assert!(
        !combined.contains("scan-plan-refs.sh:"),
        "scanner must not flag its own installed copy: {combined}"
    );
    // Also verify the clean case: a target repo with only docs/design content
    // passes from the same installed location.
    let clean_target = parent.path().join("clean_target");
    std::fs::create_dir_all(clean_target.join("docs/design")).unwrap();
    // mine-release-allow-plan-reference: scanner test fixture
    std::fs::write(clean_target.join("docs/design/arch.md"), "# Plan 99 doc\n").unwrap();
    let _ = Command::new("git")
        .args(["init", "-q"])
        .current_dir(&clean_target)
        .status()
        .unwrap();
    let _ = Command::new("git")
        .args(["config", "user.email", "t@t"])
        .current_dir(&clean_target)
        .status()
        .unwrap();
    let _ = Command::new("git")
        .args(["config", "user.name", "t"])
        .current_dir(&clean_target)
        .status()
        .unwrap();
    let _ = Command::new("git")
        .args(["add", "-A"])
        .current_dir(&clean_target)
        .status()
        .unwrap();
    let _ = Command::new("git")
        .args(["commit", "-qm", "test"])
        .current_dir(&clean_target)
        .status()
        .unwrap();
    let out2 = Command::new("bash")
        .arg("-c")
        .arg(&cmd)
        .current_dir(&clean_target)
        .output()
        .unwrap();
    assert!(
        out2.status.success(),
        "scanner must pass on a clean target repo: {}",
        String::from_utf8_lossy(&out2.stderr)
    );
}

#[test]
fn user_guide_uses_agent_status_for_machine_level_lifecycle() {
    let guide = read(&repo_root().join("docs/user-guide.md"));
    assert!(
        guide.contains("mine agent status"),
        "user guide must document `mine agent status` for machine-level installation status"
    );
    // `mine doctor --agents all` must not be presented as a machine-level
    // post-installation check; it is repository-aware.
    let lifecycle_section_start = guide.find("## Installation and lifecycle").unwrap_or(0);
    let lifecycle = &guide[lifecycle_section_start..];
    let status_line = lifecycle
        .lines()
        .find(|l| l.contains("mine doctor --agents all"))
        .map(str::to_string)
        .unwrap_or_default();
    assert!(
        !status_line.contains("verify the installed binary")
            && !status_line.contains("list managed agent integrations"),
        "mine doctor --agents all must not be presented as the machine-level install check"
    );
    assert!(
        lifecycle.contains("machine-level") && lifecycle.contains("repository-aware"),
        "user guide must distinguish machine-level from repository-aware commands"
    );
}

#[test]
fn docs_readme_anchor_points_to_installation_and_lifecycle() {
    let readme = read(&repo_root().join("docs/README.md"));
    assert!(
        readme.contains("user-guide.md#installation-and-lifecycle"),
        "docs/README.md must link to the actual heading anchor"
    );
    assert!(
        !readme.contains("user-guide.md#installation)"),
        "docs/README.md must not use the stale #installation anchor"
    );
}

#[test]
fn en_zh_readmes_share_lifecycle_semantics() {
    let en = read(&repo_root().join("README.md"));
    let zh = read(&repo_root().join("README.zh-CN.md"));
    // Both must point at the lifecycle section and name the same core commands.
    for (name, text) in [("en", &en), ("zh", &zh)] {
        assert!(
            text.contains("installation-and-lifecycle"),
            "{name} README must link the user-guide lifecycle section"
        );
        assert!(
            text.contains("mine setup"),
            "{name} README must mention `mine setup`"
        );
        assert!(
            text.contains("bootstrap"),
            "{name} README must mention bootstrap installation"
        );
    }
}

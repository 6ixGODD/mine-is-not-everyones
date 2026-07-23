// Enforce no `unsafe` in MINE-owned test crates.
#![forbid(unsafe_code)]

//! End-to-end CLI integration tests.
//!
//! These drive `mine::cli::dispatch` against temporary repositories (and the
//! real repository graph for read-only validation) and assert the stable JSON
//! envelope contract, exit codes, human output, optimistic-concurrency
//! behavior, and that no Git mutation occurs.

use std::path::PathBuf;

use mine::cli;
use mine::domain::graph::PlanWorkspace;
use mine::infrastructure::git::GitEvidence;
use mine::infrastructure::toml_store::TomlStore;

use serde_json::Value;

/// Renders an outcome as JSON and parses the envelope. On success the JSON is on
/// stdout; on error it is on stderr (`render` routes error output to stderr so
/// stdout stays clean for pipelines consuming only successful JSON).
fn envelope_json(outcome: &cli::Outcome) -> Value {
    let (stdout, stderr) = cli::render(outcome, true, false);
    let body = if outcome.exit_code == 0 {
        stdout
    } else {
        stderr
    };
    serde_json::from_str(&body).expect("envelope must be valid JSON")
}

fn run(repo: &str, rest: &[&str]) -> Vec<String> {
    let mut v = vec!["mine".to_string(), "--repo".to_string(), repo.to_string()];
    v.extend(rest.iter().map(|s| s.to_string()));
    v
}

fn run_no_repo(rest: &[&str]) -> Vec<String> {
    let mut v = vec!["mine".to_string()];
    v.extend(rest.iter().map(|s| s.to_string()));
    v
}

fn init_git_repo(root: &std::path::Path) {
    std::process::Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["init", "--quiet"])
        .status()
        .unwrap();
    std::process::Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["config", "user.email", "t@t"])
        .status()
        .unwrap();
    std::process::Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["config", "user.name", "t"])
        .status()
        .unwrap();
    std::fs::write(root.join("README.md"), "x\n").unwrap();
    std::process::Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["add", "README.md"])
        .status()
        .unwrap();
    std::process::Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["commit", "--quiet", "-m", "init"])
        .status()
        .unwrap();
}

#[test]
fn no_subcommand_is_usage_exit_2() {
    let outcome = cli::dispatch(&run_no_repo(&[]), "mine");
    assert_eq!(outcome.exit_code, 2);
    assert_eq!(outcome.command, "usage");
    let env = envelope_json(&outcome);
    assert_eq!(env["ok"], false);
    assert_eq!(env["error"]["code"], "MINE_USAGE");
    let (stdout, _stderr) = cli::render(&outcome, false, false);
    assert!(stdout.is_empty());
}

#[test]
fn unknown_command_is_usage_exit_2() {
    let outcome = cli::dispatch(&run_no_repo(&["bogus"]), "mine");
    assert_eq!(outcome.exit_code, 2);
    let env = envelope_json(&outcome);
    assert_eq!(env["ok"], false);
    assert_eq!(env["error"]["code"], "MINE_USAGE");
}

#[test]
fn init_in_absent_repo_is_idempotent_and_json_stable() {
    let root = tempfile::tempdir().unwrap();
    init_git_repo(root.path());
    let repo = root.path().to_str().unwrap();

    let outcome1 = cli::dispatch(&run(repo, &["init", "--format", "json"]), "mine");
    assert_eq!(outcome1.exit_code, 0, "init exit 0");
    assert_eq!(outcome1.command, "init");
    let env1 = envelope_json(&outcome1);
    assert_eq!(env1["ok"], true);
    assert_eq!(env1["command"], "init");
    let id = env1["data"]["repository_id"].as_str().unwrap().to_string();
    assert!(!id.is_empty());

    let outcome2 = cli::dispatch(&run(repo, &["init", "--format", "json"]), "mine");
    assert_eq!(outcome2.exit_code, 0);
    let env2 = envelope_json(&outcome2);
    assert_eq!(env2["data"]["repository_id"], env1["data"]["repository_id"]);
    // Deterministic JSON: re-rendering the first outcome is byte-identical.
    assert_eq!(
        cli::render(&outcome1, true, false).0,
        cli::render(&outcome1, true, false).0
    );
}

#[test]
fn init_refuses_legacy_unmarked_design_root() {
    let root = tempfile::tempdir().unwrap();
    init_git_repo(root.path());
    let repo = root.path().to_str().unwrap();
    std::fs::create_dir_all(root.path().join("docs").join("design")).unwrap();
    std::fs::write(
        root.path().join("docs").join("design").join("legacy.md"),
        "legacy",
    )
    .unwrap();

    let outcome = cli::dispatch(&run(repo, &["init", "--format", "json"]), "mine");
    assert_eq!(outcome.exit_code, 3, "namespace gate failure -> exit 3");
    let env = envelope_json(&outcome);
    assert_eq!(env["ok"], false);
    assert_eq!(env["error"]["code"], "MINE_DESIGN_NAMESPACE_CONFLICT");
}

#[test]
fn status_reports_graph_and_git_evidence() {
    let root = tempfile::tempdir().unwrap();
    init_git_repo(root.path());
    let repo = root.path().to_str().unwrap();
    let _ = cli::dispatch(&run(repo, &["init", "--quiet"]), "mine");
    let outcome = cli::dispatch(&run(repo, &["status", "--format", "json"]), "mine");
    assert_eq!(outcome.exit_code, 0);
    let env = envelope_json(&outcome);
    assert_eq!(env["command"], "status");
    assert!(env["data"]["git"].is_object(), "git evidence present");
}

#[test]
fn graph_validate_on_real_repository_graph_parses_and_validates() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    assert!(repo_root.join("docs/plan/execution-graph.toml").exists());
    let outcome = cli::dispatch(
        &run(
            repo_root.to_str().unwrap(),
            &["graph", "validate", "--format", "json"],
        ),
        "mine",
    );
    assert_eq!(outcome.exit_code, 0, "real graph must validate");
    let env = envelope_json(&outcome);
    assert_eq!(env["ok"], true);
    assert!(env["data"]["plans"].as_u64().unwrap() >= 9);
}

#[test]
fn graph_render_is_deterministic_and_idempotent() {
    // Operate on a TEMP COPY of the real graph so the test never mutates the
    // repository's tracked generated view.
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let tmp = tempfile::tempdir().unwrap();
    let dst = tmp.path();
    std::fs::create_dir_all(dst.join("docs/plan")).unwrap();
    std::fs::copy(
        repo_root.join("docs/plan/execution-graph.toml"),
        dst.join("docs/plan/execution-graph.toml"),
    )
    .unwrap();
    let outcome1 = cli::dispatch(
        &run(
            dst.to_str().unwrap(),
            &["graph", "render", "--format", "json"],
        ),
        "mine",
    );
    assert_eq!(outcome1.exit_code, 0);
    let store = TomlStore::new(dst);
    let first = std::fs::read_to_string(store.md_path()).unwrap();
    // Second render: byte-identical (deterministic + idempotent).
    let _ = cli::dispatch(
        &run(dst.to_str().unwrap(), &["graph", "render", "--quiet"]),
        "mine",
    );
    let second = std::fs::read_to_string(store.md_path()).unwrap();
    assert_eq!(
        first, second,
        "graph render is deterministic and idempotent"
    );
}

#[test]
fn plan_start_refuses_non_ready_plan() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let outcome = cli::dispatch(
        &run(
            repo_root.to_str().unwrap(),
            &["plan", "start", "--id", "02-1", "--format", "json"],
        ),
        "mine",
    );
    assert_eq!(
        outcome.exit_code, 4,
        "non-ready transition -> validation exit 4"
    );
    let env = envelope_json(&outcome);
    assert_eq!(env["ok"], false);
    assert_eq!(env["error"]["code"], "MINE_INVALID_TRANSITION");
}

#[test]
fn plan_accept_requires_implemented_state() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let outcome = cli::dispatch(
        &run(
            repo_root.to_str().unwrap(),
            &[
                "plan", "accept", "--id", "03", "--review", "none.md", "--format", "json",
            ],
        ),
        "mine",
    );
    assert_eq!(outcome.exit_code, 4);
    let env = envelope_json(&outcome);
    assert_eq!(env["error"]["code"], "MINE_INVALID_TRANSITION");
}

#[test]
fn workspace_open_on_real_graph_is_idempotent() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let store = TomlStore::new(&repo_root);
    let before = store.load().unwrap();
    let outcome = cli::dispatch(
        &run(
            repo_root.to_str().unwrap(),
            &["workspace", "status", "--format", "json"],
        ),
        "mine",
    );
    assert_eq!(outcome.exit_code, 0);
    let env = envelope_json(&outcome);
    assert_eq!(env["data"]["workspace_id"], before.workspace_id);
    assert_eq!(env["data"]["revision"], before.revision);
}

#[test]
fn repository_version_show_round_trips() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let cfg = mine::cli::context::load_config(&repo_root).expect("config present");
    let outcome = cli::dispatch(
        &run(
            repo_root.to_str().unwrap(),
            &["repository", "version", "show", "--format", "json"],
        ),
        "mine",
    );
    assert_eq!(outcome.exit_code, 0);
    let env = envelope_json(&outcome);
    assert_eq!(env["data"]["version"], cfg.mine_code_version);
}

#[test]
fn cli_performs_no_git_mutation() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let before_clean = GitEvidence::collect(&repo_root).clean;
    for cmd in [
        ["status", "--format", "json"].as_slice(),
        ["graph", "validate", "--format", "json"].as_slice(),
        ["graph", "ready", "--format", "json"].as_slice(),
        ["graph", "wave", "--format", "json"].as_slice(),
        ["design", "status", "--format", "json"].as_slice(),
    ] {
        let outcome = cli::dispatch(&run(repo_root.to_str().unwrap(), cmd), "mine");
        assert_eq!(outcome.exit_code, 0, "cmd {:?}", cmd);
    }
    let after_clean = GitEvidence::collect(&repo_root).clean;
    assert_eq!(before_clean, after_clean, "CLI made no Git mutation");
}

#[test]
fn revision_conflict_surfaces_exit_5() {
    let root = tempfile::tempdir().unwrap();
    init_git_repo(root.path());
    let repo = root.path().to_str().unwrap();
    let _ = cli::dispatch(&run(repo, &["init", "--quiet"]), "mine");
    let store = TomlStore::new(root.path());
    let ws = PlanWorkspace {
        schema_version: 1,
        revision: 3,
        project_id: "p".to_string(),
        workspace_id: "ws".to_string(),
        stable_branch: "master".to_string(),
        integration_branch: "dev".to_string(),
        stable_baseline_commit: "c".to_string(),
        design_root: "docs/design/index.md".to_string(),
        ephemeral_workspace: true,
        purge_before_stable_release: true,
        plans: vec![],
    };
    std::fs::create_dir_all(store.toml_path().parent().unwrap()).unwrap();
    std::fs::write(store.toml_path(), toml::to_string(&ws).unwrap()).unwrap();

    let err = store.save_with_revision(2, Ok).unwrap_err();
    let he = mine::cli::HandlerError::from_mine(&err);
    assert_eq!(he.exit_code, mine::output::exit_code::CONFLICT);
    assert_eq!(he.code, "MINE_REVISION_CONFLICT");
}

#[test]
fn design_backup_round_trip_and_gitignore() {
    let root = tempfile::tempdir().unwrap();
    init_git_repo(root.path());
    let repo = root.path().to_str().unwrap();
    let _ = cli::dispatch(&run(repo, &["init", "--quiet"]), "mine");
    std::fs::write(root.path().join("docs/design/leaf.md"), "# Leaf\n").unwrap();

    let outcome = cli::dispatch(
        &run(repo, &["design", "backup", "--format", "json"]),
        "mine",
    );
    assert_eq!(outcome.exit_code, 0);
    let env = envelope_json(&outcome);
    let backup_rel = env["data"]["backup_path"].as_str().unwrap().to_string();
    assert!(backup_rel.starts_with("docs/design-backup-"));
    let backup = root.path().join(&backup_rel);
    assert!(backup.join("index.md").exists());
    assert!(backup.join("leaf.md").exists());
    assert_eq!(
        std::fs::read_to_string(backup.join(".gitignore")).unwrap(),
        "*\n"
    );
}

#[test]
fn json_envelope_has_stable_sorted_keys() {
    let outcome = cli::dispatch(
        &run(
            env!("CARGO_MANIFEST_DIR"),
            &["graph", "ready", "--format", "json"],
        ),
        "mine",
    );
    let body = cli::render(&outcome, true, false).0;
    // The envelope serializes with sorted keys: "command" is alphabetically
    // first, so a stable serializer emits it as the first object key.
    assert!(body.starts_with(r#"{"command":"#));
}

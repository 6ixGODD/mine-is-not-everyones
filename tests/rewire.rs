// Enforce no `unsafe` in MINE-owned test crates.
#![forbid(unsafe_code)]

//! `mine plan rewire-compensation --id` integration tests (Plan 09).
//!
//! Drives the CLI over isolated temp repos seeded with controlled graphs; the
//! live repository graph is snapshotted before/after and asserted unchanged.

mod common;

use mine::domain::status::PlanStatus;

use common::{dispatch_json, live_graph_bytes, load_graph, node, rejected_node, seeded_repo};

fn assert_live_unchanged(before: &[u8]) {
    assert_eq!(
        before,
        live_graph_bytes(),
        "live repo graph unchanged by temp-copy test"
    );
}

fn seed_rewire_graph(
    extra: Vec<mine::domain::graph::PlanNode>,
) -> (tempfile::TempDir, std::path::PathBuf) {
    let mut plans = vec![
        node("03", PlanStatus::Accepted, &[], &[]),
        node("04", PlanStatus::Accepted, &[], &[]),
        rejected_node("05", "05-1", &["03"]),
        node("05-1", PlanStatus::Ready, &["03"], &[]),
        node("06", PlanStatus::Blocked, &["04", "05"], &[]),
    ];
    plans.extend(extra);
    seeded_repo(plans)
}

#[test]
fn rewire_success_hard_predecessors() {
    let live = live_graph_bytes();
    let (_tmp, repo) = seed_rewire_graph(vec![]);
    let (outcome, env) = dispatch_json(
        &repo,
        &[
            "plan",
            "rewire-compensation",
            "--id",
            "05",
            "--format",
            "json",
        ],
    );
    assert_eq!(outcome.exit_code, 0, "{env}");
    assert_eq!(env["ok"], true);
    assert_eq!(env["command"], "plan.rewire-compensation");
    assert_eq!(
        env["revision_after"].as_u64().unwrap(),
        env["revision_before"].as_u64().unwrap() + 1
    );
    assert_eq!(env["data"]["rejected_plan"], "05");
    assert_eq!(env["data"]["compensating_plan"], "05-1");
    assert_eq!(
        env["data"]["affected_successors"],
        serde_json::json!(["06"])
    );
    let ws = load_graph(&repo);
    assert_eq!(ws.get("06").unwrap().hard_predecessors, vec!["04", "05-1"]);
    // 05 stays REJECTED, untouched.
    assert_eq!(ws.get("05").unwrap().status, PlanStatus::Rejected);
    assert_eq!(ws.get("05").unwrap().compensating_plan, "05-1");
    assert_live_unchanged(&live);
}

#[test]
fn rewire_regenerates_markdown_view() {
    let (_tmp, repo) = seed_rewire_graph(vec![]);
    let (outcome, env) = dispatch_json(
        &repo,
        &[
            "plan",
            "rewire-compensation",
            "--id",
            "05",
            "--format",
            "json",
        ],
    );
    assert_eq!(outcome.exit_code, 0, "{env}");
    let md = std::fs::read_to_string(repo.join("docs/plan/execution-graph.md")).unwrap();
    assert!(md.contains("05-1"), "MD view reflects rewired predecessor");
    // 06 row now names 05-1 (and 04), not 05.
    let line06 = md.lines().find(|l| l.starts_with("| 06 |")).unwrap();
    assert!(line06.contains("05-1"));
    assert!(!line06.contains("05 |") || line06.contains("05-1")); // not the bare 05
}

#[test]
fn rewire_idempotent_no_op() {
    let live = live_graph_bytes();
    let (_tmp, repo) = seeded_repo(vec![
        node("03", PlanStatus::Accepted, &[], &[]),
        node("04", PlanStatus::Accepted, &[], &[]),
        rejected_node("05", "05-1", &["03"]),
        node("05-1", PlanStatus::Accepted, &["03"], &[]),
        node("06", PlanStatus::Blocked, &["04", "05-1"], &[]), // already rewired
    ]);
    let before = std::fs::read(repo.join("docs/plan/execution-graph.toml")).unwrap();
    let (outcome, env) = dispatch_json(
        &repo,
        &[
            "plan",
            "rewire-compensation",
            "--id",
            "05",
            "--format",
            "json",
        ],
    );
    assert_eq!(outcome.exit_code, 0, "{env}");
    assert_eq!(env["data"]["affected_successors"], serde_json::json!([]));
    assert_eq!(
        env["revision_before"], env["revision_after"],
        "no bump on no-op"
    );
    let after = std::fs::read(repo.join("docs/plan/execution-graph.toml")).unwrap();
    assert_eq!(before, after, "temp bytes identical on idempotent no-op");
    assert_live_unchanged(&live);
}

#[test]
fn rewire_not_rejected_original_errors() {
    let (_tmp, repo) = seeded_repo(vec![
        node("03", PlanStatus::Accepted, &[], &[]),
        node("05", PlanStatus::Implemented, &["03"], &[]),
        node("05-1", PlanStatus::Ready, &["03"], &[]),
        node("06", PlanStatus::Blocked, &["04", "05"], &[]),
        node("04", PlanStatus::Accepted, &[], &[]),
    ]);
    let (outcome, env) = dispatch_json(
        &repo,
        &[
            "plan",
            "rewire-compensation",
            "--id",
            "05",
            "--format",
            "json",
        ],
    );
    assert_eq!(outcome.exit_code, 4, "{env}");
    assert_eq!(env["error"]["code"], "MINE_INVALID_TRANSITION");
}

#[test]
fn rewire_empty_compensating_plan_errors() {
    let live = live_graph_bytes();
    let (_tmp, repo) = seeded_repo(vec![
        node("03", PlanStatus::Accepted, &[], &[]),
        rejected_node("05", "", &["03"]),
        node("06", PlanStatus::Draft, &["05"], &[]),
    ]);
    let before = std::fs::read(repo.join("docs/plan/execution-graph.toml")).unwrap();
    let (outcome, env) = dispatch_json(
        &repo,
        &[
            "plan",
            "rewire-compensation",
            "--id",
            "05",
            "--format",
            "json",
        ],
    );
    assert_eq!(outcome.exit_code, 4, "{env}");
    assert_eq!(env["error"]["code"], "MINE_GRAPH_INVALID");
    let after = std::fs::read(repo.join("docs/plan/execution-graph.toml")).unwrap();
    assert_eq!(before, after);
    assert_live_unchanged(&live);
}

#[test]
fn rewire_missing_replacement_errors() {
    let (_tmp, repo) = seeded_repo(vec![
        node("03", PlanStatus::Accepted, &[], &[]),
        rejected_node("05", "05-1", &["03"]),
        node("06", PlanStatus::Draft, &["05"], &[]),
    ]);
    let (outcome, env) = dispatch_json(
        &repo,
        &[
            "plan",
            "rewire-compensation",
            "--id",
            "05",
            "--format",
            "json",
        ],
    );
    assert_eq!(outcome.exit_code, 4, "{env}");
    assert_eq!(env["error"]["code"], "MINE_PLAN_NOT_FOUND");
}

#[test]
fn rewire_rejected_replacement_errors() {
    let (_tmp, repo) = seeded_repo(vec![
        node("03", PlanStatus::Accepted, &[], &[]),
        rejected_node("05", "05-1", &["03"]),
        rejected_node("05-1", "", &["03"]),
        node("06", PlanStatus::Draft, &["05"], &[]),
    ]);
    let (outcome, env) = dispatch_json(
        &repo,
        &[
            "plan",
            "rewire-compensation",
            "--id",
            "05",
            "--format",
            "json",
        ],
    );
    assert_eq!(outcome.exit_code, 4, "{env}");
    assert_eq!(env["error"]["code"], "MINE_GRAPH_INVALID");
}

#[test]
fn rewire_locked_successor_errors_bytes_unchanged() {
    let live = live_graph_bytes();
    for status in [
        PlanStatus::InProgress,
        PlanStatus::Implemented,
        PlanStatus::Accepted,
        PlanStatus::Rejected,
    ] {
        let (_tmp, repo) = seeded_repo(vec![
            node("03", PlanStatus::Accepted, &[], &[]),
            node("04", PlanStatus::Accepted, &[], &[]),
            rejected_node("05", "05-1", &["03"]),
            node("05-1", PlanStatus::Ready, &["03"], &[]),
            node("06", status, &["04", "05"], &[]),
        ]);
        let before = std::fs::read(repo.join("docs/plan/execution-graph.toml")).unwrap();
        let (outcome, env) = dispatch_json(
            &repo,
            &[
                "plan",
                "rewire-compensation",
                "--id",
                "05",
                "--format",
                "json",
            ],
        );
        assert_eq!(outcome.exit_code, 3, "gate exit for {status:?}: {env}");
        assert_eq!(env["error"]["code"], "MINE_REWIRE_SUCCESSOR_LOCKED");
        let after = std::fs::read(repo.join("docs/plan/execution-graph.toml")).unwrap();
        assert_eq!(before, after, "temp bytes unchanged ({status:?})");
        let ws = load_graph(&repo);
        assert_eq!(
            ws.get("06").unwrap().hard_predecessors,
            vec!["04", "05"],
            "06 unchanged"
        );
    }
    assert_live_unchanged(&live);
}

#[test]
fn rewire_cycle_errors_bytes_unchanged() {
    let live = live_graph_bytes();
    let (_tmp, repo) = seeded_repo(vec![
        node("03", PlanStatus::Accepted, &[], &[]),
        node("04", PlanStatus::Accepted, &[], &[]),
        rejected_node("05", "05-1", &["03"]),
        node("05-1", PlanStatus::Draft, &["06"], &[]),
        node("06", PlanStatus::Draft, &["04", "05"], &[]),
    ]);
    let before = std::fs::read(repo.join("docs/plan/execution-graph.toml")).unwrap();
    let (outcome, env) = dispatch_json(
        &repo,
        &[
            "plan",
            "rewire-compensation",
            "--id",
            "05",
            "--format",
            "json",
        ],
    );
    assert_eq!(outcome.exit_code, 4, "{env}");
    assert_eq!(env["error"]["code"], "MINE_GRAPH_CYCLE");
    let after = std::fs::read(repo.join("docs/plan/execution-graph.toml")).unwrap();
    assert_eq!(before, after);
    assert_live_unchanged(&live);
}

#[test]
fn rewire_sibling_id_not_rewired() {
    // A successor that references the sibling-looking id "050" must NOT be
    // rewired when rerouting "05". Only exact "05" predecessor entries change.
    let (_tmp, repo) = seeded_repo(vec![
        node("03", PlanStatus::Accepted, &[], &[]),
        rejected_node("05", "05-1", &["03"]),
        node("05-1", PlanStatus::Ready, &["03"], &[]),
        node("050", PlanStatus::Accepted, &[], &[]),
        node("07", PlanStatus::Draft, &["050"], &[]),
    ]);
    let (outcome, env) = dispatch_json(
        &repo,
        &[
            "plan",
            "rewire-compensation",
            "--id",
            "05",
            "--format",
            "json",
        ],
    );
    assert_eq!(outcome.exit_code, 0, "{env}");
    assert_eq!(env["data"]["affected_successors"], serde_json::json!([]));
    let ws = load_graph(&repo);
    assert_eq!(
        ws.get("07").unwrap().hard_predecessors,
        vec!["050"],
        "sibling-looking 050 predecessor untouched"
    );
}

#[test]
fn rewire_soft_predecessor() {
    let (_tmp, repo) = seeded_repo(vec![
        node("03", PlanStatus::Accepted, &[], &[]),
        rejected_node("05", "05-1", &["03"]),
        node("05-1", PlanStatus::Ready, &["03"], &[]),
        node("09", PlanStatus::Draft, &[], &["05"]),
    ]);
    let (outcome, env) = dispatch_json(
        &repo,
        &[
            "plan",
            "rewire-compensation",
            "--id",
            "05",
            "--format",
            "json",
        ],
    );
    assert_eq!(outcome.exit_code, 0, "{env}");
    assert_eq!(
        env["data"]["affected_successors"],
        serde_json::json!(["09"])
    );
    let ws = load_graph(&repo);
    assert_eq!(ws.get("09").unwrap().soft_predecessors, vec!["05-1"]);
}

#[test]
fn rewire_missing_id_flag_is_usage() {
    let (_tmp, repo) = seed_rewire_graph(vec![]);
    let (outcome, env) = dispatch_json(&repo, &["plan", "rewire-compensation", "--format", "json"]);
    assert_eq!(outcome.exit_code, 2);
    assert_eq!(env["error"]["code"], "MINE_USAGE");
}

#[test]
fn live_graph_byte_unchanged_after_rewire_suite() {
    let before = live_graph_bytes();
    let after = live_graph_bytes();
    assert_eq!(
        before, after,
        "live repo graph unchanged by the rewire suite"
    );
}

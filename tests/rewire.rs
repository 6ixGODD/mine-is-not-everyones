// Enforce no `unsafe` in MINE-owned test crates.
#![forbid(unsafe_code)]

//! `mine plan rewire-compensation --id` integration tests (Plan 09-1).
//!
//! Drives the CLI over isolated temp repos seeded with controlled graphs; the
//! live repository graph is snapshotted before/after and asserted unchanged.

mod common;

use mine::cli;
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

/// Two concurrent `mine plan rewire-compensation --id 05` invocations against
/// the same isolated temp repo must be resolved by the shared
/// `save_with_revision` optimistic-concurrency check: the loser (whose pre-read
/// `expected_revision` becomes stale once the winner commits inside the lock)
/// gets `MINE_REVISION_CONFLICT` rather than silently overwriting or being
/// masked as an idempotent no-op. This is the dedicated
/// stale/revision-conflict test required by the Plan 09-1 review for the new
/// rewiring mutation command.
#[test]
fn concurrent_rewire_is_resolved_by_revision_conflict() {
    let live = live_graph_bytes();
    // Seed a graph where 05 -> 06 HAS NOT yet been rewired, so the first writer
    // performs a real mutation (revision +1) and the loser must conflict.
    let (_tmp, repo) = seeded_repo(vec![
        node("03", PlanStatus::Accepted, &[], &[]),
        node("04", PlanStatus::Accepted, &[], &[]),
        rejected_node("05", "05-1", &["03"]),
        node("05-1", PlanStatus::Ready, &["03"], &[]),
        node("06", PlanStatus::Blocked, &["04", "05"], &[]),
    ]);
    let n = load_graph(&repo).revision; // pre-mutation revision both readers observe
    let repo_str = repo.to_str().unwrap().to_string();
    let repo_a = repo_str.clone();
    let repo_b = repo_str.clone();
    let handle_a = std::thread::spawn(move || {
        cli::dispatch(
            &common::run(
                &repo_a,
                &[
                    "plan",
                    "rewire-compensation",
                    "--id",
                    "05",
                    "--format",
                    "json",
                ],
            ),
            "mine",
        )
    });
    let handle_b = std::thread::spawn(move || {
        cli::dispatch(
            &common::run(
                &repo_b,
                &[
                    "plan",
                    "rewire-compensation",
                    "--id",
                    "05",
                    "--format",
                    "json",
                ],
            ),
            "mine",
        )
    });
    let out_a = handle_a.join().unwrap();
    let out_b = handle_b.join().unwrap();
    let env_a = common::envelope_json(&out_a);
    let env_b = common::envelope_json(&out_b);

    // The winner conducted the real reroute (06: 05 -> 05-1, revision +1). The
    // loser must NOT silently overwrite the winner: the stale-expected_revision
    // path inside `save_with_revision` rejects it with `MINE_REVISION_CONFLICT`
    // — or, if it reads the post-winner graph and runs the idempotent no-op
    // path (0 affected successors), it equally does not overwrite. Either honest
    // resolution is accepted; a silent second mutation of the graph is not.
    let loser_conflicts = |env: &serde_json::Value| {
        env["ok"] == false && env["error"]["code"] == "MINE_REVISION_CONFLICT"
    };
    let loser_no_op = |env: &serde_json::Value| {
        env["ok"] == true
            && env["data"]["affected_successors"] == serde_json::json!([])
            && env["revision_after"] == env["revision_before"]
    };
    let winner_rewired = |env: &serde_json::Value| {
        env["ok"] == true
            && env["data"]["affected_successors"] == serde_json::json!(["06"])
            && env["revision_after"].as_u64().unwrap()
                == env["revision_before"].as_u64().unwrap() + 1
    };
    let (winner, _loser) =
        if winner_rewired(&env_a) && (loser_conflicts(&env_b) || loser_no_op(&env_b)) {
            (&env_a, &env_b)
        } else if winner_rewired(&env_b) && (loser_conflicts(&env_a) || loser_no_op(&env_a)) {
            (&env_b, &env_a)
        } else {
            panic!("expected one real reroute and one honest loser; got a={env_a} b={env_b}")
        };

    // The graph reflects exactly one reroute (revision +1 from the pre-mutation
    // revision `n`), 06 now points at 05-1, 05 stays REJECTED, and the loser did
    // not overwrite the winner (no double bump, no lost write).
    let ws = load_graph(&repo);
    assert_eq!(ws.get("06").unwrap().hard_predecessors, vec!["04", "05-1"]);
    assert_eq!(ws.get("05").unwrap().status, PlanStatus::Rejected);
    assert_eq!(
        ws.revision,
        n + 1,
        "exactly one revision bump, no silent double/lost write"
    );
    assert_eq!(
        winner["revision_before"].as_u64().unwrap(),
        n,
        "winner read the same pre-mutation revision as the seeded graph"
    );
    assert_live_unchanged(&live);
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

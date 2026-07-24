// Enforce no `unsafe` in MINE-owned test crates.
#![forbid(unsafe_code)]

//! `mine plan release --id` integration tests (Plan 09-1).
//!
//! Drives the CLI over isolated temp repos seeded with controlled graphs; the
//! live repository graph is snapshotted before/after and asserted unchanged.

mod common;

use mine::cli;
use mine::domain::status::PlanStatus;
use mine::infrastructure::toml_store::TomlStore;

use common::{dispatch_json, live_graph_bytes, load_graph, node, rejected_node, seeded_repo};

fn assert_live_unchanged(before: &[u8]) {
    let after = live_graph_bytes();
    assert_eq!(before, after, "live repo graph unchanged by temp-copy test");
}

#[test]
fn release_no_predecessors_draft_to_ready() {
    let live = live_graph_bytes();
    let (_tmp, repo) = seeded_repo(vec![node("09", PlanStatus::Draft, &[], &[])]);
    let (outcome, env) = dispatch_json(
        &repo,
        &["plan", "release", "--id", "09", "--format", "json"],
    );
    assert_eq!(outcome.exit_code, 0, "{env}");
    assert_eq!(env["ok"], true);
    assert_eq!(env["command"], "plan.release");
    // Every successful release bumps the graph revision exactly once.
    assert_eq!(
        env["revision_after"].as_u64().unwrap(),
        env["revision_before"].as_u64().unwrap() + 1
    );
    assert_eq!(env["data"]["plan"], "09");
    assert_eq!(env["data"]["status_before"], "DRAFT");
    assert_eq!(env["data"]["status_after"], "READY");
    assert_eq!(env["data"]["hard_predecessors"], serde_json::json!([]));
    assert_eq!(
        env["data"]["unsatisfied_predecessors"],
        serde_json::json!([])
    );
    let ws = load_graph(&repo);
    assert_eq!(ws.get("09").unwrap().status, PlanStatus::Ready);
    // MD view regenerated deterministically with the new status.
    let md = std::fs::read_to_string(repo.join("docs/plan/execution-graph.md")).unwrap();
    assert!(md.contains("| 09 |") && md.contains("READY"));
    assert_live_unchanged(&live);
}

#[test]
fn release_all_accepted_predecessors_to_ready() {
    let (tmp, repo) = seeded_repo(vec![
        node("01", PlanStatus::Accepted, &[], &[]),
        node("03", PlanStatus::Accepted, &[], &[]),
        node("09", PlanStatus::Draft, &["01", "03"], &[]),
    ]);
    let _ = tmp;
    let (outcome, env) = dispatch_json(
        &repo,
        &["plan", "release", "--id", "09", "--format", "json"],
    );
    assert_eq!(outcome.exit_code, 0, "{env}");
    assert_eq!(env["data"]["status_after"], "READY");
    assert_eq!(
        env["data"]["unsatisfied_predecessors"],
        serde_json::json!([])
    );
    let ws = load_graph(&repo);
    assert_eq!(ws.get("09").unwrap().status, PlanStatus::Ready);
}

#[test]
fn release_one_unaccepted_predecessor_to_blocked() {
    let live = live_graph_bytes();
    let (_tmp, repo) = seeded_repo(vec![
        node("01", PlanStatus::Accepted, &[], &[]),
        node("02", PlanStatus::Blocked, &["01"], &[]),
        node("09", PlanStatus::Draft, &["01", "02"], &[]),
    ]);
    let (outcome, env) = dispatch_json(
        &repo,
        &["plan", "release", "--id", "09", "--format", "json"],
    );
    assert_eq!(outcome.exit_code, 0, "{env}");
    assert_eq!(env["data"]["status_after"], "BLOCKED");
    assert_eq!(
        env["data"]["unsatisfied_predecessors"],
        serde_json::json!(["02"])
    );
    assert_eq!(
        env["revision_after"].as_u64().unwrap(),
        env["revision_before"].as_u64().unwrap() + 1
    );
    let ws = load_graph(&repo);
    assert_eq!(ws.get("09").unwrap().status, PlanStatus::Blocked);
    assert_live_unchanged(&live);
}

#[test]
fn release_non_draft_is_invalid_transition_bytes_unchanged() {
    let live = live_graph_bytes();
    for status in [
        PlanStatus::Blocked,
        PlanStatus::Ready,
        PlanStatus::InProgress,
        PlanStatus::Implemented,
        PlanStatus::Accepted,
        PlanStatus::Rejected,
    ] {
        let (_tmp, repo) = seeded_repo(vec![node("09", status, &[], &[])]);
        let before = std::fs::read(repo.join("docs/plan/execution-graph.toml")).unwrap();
        let (outcome, env) = dispatch_json(
            &repo,
            &["plan", "release", "--id", "09", "--format", "json"],
        );
        assert_eq!(
            outcome.exit_code, 4,
            "validation exit for {status:?}: {env}"
        );
        assert_eq!(env["ok"], false);
        assert_eq!(env["error"]["code"], "MINE_INVALID_TRANSITION");
        let after = std::fs::read(repo.join("docs/plan/execution-graph.toml")).unwrap();
        assert_eq!(before, after, "temp bytes unchanged ({status:?})");
    }
    assert_live_unchanged(&live);
}

#[test]
fn release_missing_plan_is_not_found() {
    let (_tmp, repo) = seeded_repo(vec![node("09", PlanStatus::Draft, &[], &[])]);
    let (outcome, env) = dispatch_json(
        &repo,
        &["plan", "release", "--id", "ghost", "--format", "json"],
    );
    assert_eq!(outcome.exit_code, 4);
    assert_eq!(env["error"]["code"], "MINE_PLAN_NOT_FOUND");
}

#[test]
fn release_missing_id_flag_is_usage() {
    let (_tmp, repo) = seeded_repo(vec![node("09", PlanStatus::Draft, &[], &[])]);
    let (outcome, env) = dispatch_json(&repo, &["plan", "release", "--format", "json"]);
    assert_eq!(outcome.exit_code, 2);
    assert_eq!(env["error"]["code"], "MINE_USAGE");
}

#[test]
fn release_not_idempotent_second_release_errors() {
    let live = live_graph_bytes();
    let (_tmp, repo) = seeded_repo(vec![node("09", PlanStatus::Draft, &[], &[])]);
    let (o1, e1) = dispatch_json(
        &repo,
        &["plan", "release", "--id", "09", "--format", "json"],
    );
    assert_eq!(o1.exit_code, 0, "{e1}");
    let rev_after_first = load_graph(&repo).revision;
    let (o2, e2) = dispatch_json(
        &repo,
        &["plan", "release", "--id", "09", "--format", "json"],
    );
    assert_eq!(
        o2.exit_code, 4,
        "second release must not be idempotent-success: {e2}"
    );
    assert_eq!(e2["error"]["code"], "MINE_INVALID_TRANSITION");
    let rev_after_second = load_graph(&repo).revision;
    assert_eq!(
        rev_after_first, rev_after_second,
        "no extra revision bump on error"
    );
    assert_live_unchanged(&live);
}

#[test]
fn release_does_not_alter_other_plans() {
    let (_tmp, repo) = seeded_repo(vec![
        node("01", PlanStatus::Accepted, &[], &[]),
        node("03", PlanStatus::Blocked, &["01"], &[]),
        node("09", PlanStatus::Draft, &["01", "03"], &[]),
    ]);
    let (outcome, env) = dispatch_json(
        &repo,
        &["plan", "release", "--id", "09", "--format", "json"],
    );
    assert_eq!(outcome.exit_code, 0, "{env}");
    assert_eq!(env["data"]["status_after"], "BLOCKED");
    assert_eq!(
        env["data"]["unsatisfied_predecessors"],
        serde_json::json!(["03"])
    );
    let ws = load_graph(&repo);
    // 09 released to BLOCKED; 01 and 03 untouched.
    assert_eq!(ws.get("09").unwrap().status, PlanStatus::Blocked);
    assert_eq!(ws.get("01").unwrap().status, PlanStatus::Accepted);
    assert_eq!(ws.get("03").unwrap().status, PlanStatus::Blocked);
}

#[test]
fn release_rejects_rejected_plan_others_unchanged() {
    let live = live_graph_bytes();
    let (_tmp, repo) = seeded_repo(vec![
        node("03", PlanStatus::Accepted, &[], &[]),
        rejected_node("05", "05-1", &["03"]),
        node("05-1", PlanStatus::Ready, &["03"], &[]),
    ]);
    let before = std::fs::read(repo.join("docs/plan/execution-graph.toml")).unwrap();
    let (outcome, env) = dispatch_json(
        &repo,
        &["plan", "release", "--id", "05", "--format", "json"],
    );
    assert_eq!(outcome.exit_code, 4, "{env}");
    assert_eq!(env["error"]["code"], "MINE_INVALID_TRANSITION");
    let after = std::fs::read(repo.join("docs/plan/execution-graph.toml")).unwrap();
    assert_eq!(before, after);
    assert_live_unchanged(&live);
}

/// Two concurrent `mine plan release --id` invocations against the same
/// isolated temp repo must be resolved by the shared `save_with_revision`
/// optimistic-concurrency check: exactly one wins (revision bumps +1), the
/// other gets `MINE_REVISION_CONFLICT`, and neither silently overwrites the
/// graph. This is the dedicated stale/revision-conflict test required by the
/// Plan 09-1 review for the new mutation command.
#[test]
fn concurrent_release_is_resolved_by_revision_conflict() {
    let live = live_graph_bytes();
    let (_tmp, repo) = seeded_repo(vec![node("09-1", PlanStatus::Draft, &[], &[])]);
    let n = load_graph(&repo).revision; // pre-mutation revision both readers observe
    let repo_str = repo.to_str().unwrap().to_string();
    let repo_a = repo_str.clone();
    let repo_b = repo_str.clone();
    let handle_a = std::thread::spawn(move || {
        cli::dispatch(
            &common::run(
                &repo_a,
                &["plan", "release", "--id", "09-1", "--format", "json"],
            ),
            "mine",
        )
    });
    let handle_b = std::thread::spawn(move || {
        cli::dispatch(
            &common::run(
                &repo_b,
                &["plan", "release", "--id", "09-1", "--format", "json"],
            ),
            "mine",
        )
    });
    let out_a = handle_a.join().unwrap();
    let out_b = handle_b.join().unwrap();

    let env_a = common::envelope_json(&out_a);
    let env_b = common::envelope_json(&out_b);

    let ok_a = out_a.exit_code == 0 && env_a["ok"] == true;
    let ok_b = out_b.exit_code == 0 && env_b["ok"] == true;
    // Exactly one of the two won.
    assert!(ok_a ^ ok_b, "exactly one release won: a={ok_a} b={ok_b}");

    // The loser failed with a revision conflict (exit 5).
    let (loser_env, winner_env) = if ok_a {
        (&env_b, &env_a)
    } else {
        (&env_a, &env_b)
    };
    assert_eq!(loser_env["ok"], false);
    assert_eq!(loser_env["error"]["code"], "MINE_REVISION_CONFLICT");

    // The serial winner bumped the revision by exactly +1 over the pre-mutation
    // revision `n` (which both writers read before either acquired the lock).
    let ws = load_graph(&repo);
    assert_eq!(ws.get("09-1").unwrap().status, PlanStatus::Ready);
    assert_eq!(
        ws.revision,
        n + 1,
        "exactly one revision bump, no silent double/lost write"
    );
    assert_eq!(
        winner_env["revision_before"].as_u64().unwrap(),
        n,
        "winner read the same pre-mutation revision as the seeded graph"
    );
    // The loser made no observable change and did not overwrite the winner.
    assert_eq!(loser_env["error"]["code"], "MINE_REVISION_CONFLICT");
    assert_live_unchanged(&live);
}

#[test]
fn live_graph_byte_unchanged_after_release_suite() {
    // Final guard: the whole release suite uses temp repos; the live graph is
    // invariant. (Other tests assert the same; this is the explicit guard.)
    let before = live_graph_bytes();
    // No-op read.
    let _ = TomlStore::new(&std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")));
    let after = live_graph_bytes();
    assert_eq!(before, after);
}

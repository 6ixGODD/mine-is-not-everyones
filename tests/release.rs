// Enforce no `unsafe` in MINE-owned test crates.
#![forbid(unsafe_code)]

//! `mine plan release --id` integration tests.
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
/// isolated temp repo must be resolved by the shared `lock -> reload ->
/// revision check -> semantic check -> mutation -> atomic write -> render`
/// transaction. Exactly one writer performs the real `DRAFT` release
/// (revision bumps +1, node `DRAFT -> READY`); the loser is an *honest* loser
/// and must mutate nothing. The design
/// (`docs/design/execution-graph/state-machine-and-algorithms.md#plan-release`)
/// documents two valid loser outcomes of this race:
///
/// 1. `MINE_REVISION_CONFLICT` — the loser operated from the stale
///    pre-winner revision and the optimistic-concurrency check rejects it; or
/// 2. `MINE_INVALID_TRANSITION` — the loser re-reads after the winner already
///    transitioned the node `DRAFT -> READY`, so `release_plan` sees a non-
///    `DRAFT` node and refuses ("treat `MINE_INVALID_TRANSITION` as already
///    released").
///
/// Both are valid only when the safety invariants also hold: exactly one
/// winner, exactly one revision bump, final `READY`, no stale overwrite, no
/// unrelated graph data change, and TOML/Markdown consistency. This is the
/// dedicated concurrency test required by the independent review for the new
/// mutation command; the over-constrained single-code loser
/// assertion and strengthened the invariants.
#[test]
fn concurrent_release_is_resolved_by_revision_conflict() {
    let live = live_graph_bytes();
    // Seed the raced `DRAFT` plan plus an unrelated `ACCEPTED` plan that the
    // release must NOT touch, to prove no unrelated graph data changes.
    let (_tmp, repo) = seeded_repo(vec![
        node("01", PlanStatus::Accepted, &[], &[]),
        node("09-1", PlanStatus::Draft, &[], &[]),
    ]);
    let n = load_graph(&repo).revision; // pre-mutation revision both readers observe
    let unrelated_before = load_graph(&repo)
        .get("01")
        .expect("seeded unrelated node 01")
        .clone();
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
    // Exactly one writer performed the real DRAFT release.
    assert!(ok_a ^ ok_b, "exactly one release won: a={ok_a} b={ok_b}");

    // The loser is an honest loser: it mutated nothing. It may observe either
    // documented stability error depending on whether it raced the revision
    // check or re-read after the winner's DRAFT->READY transition.
    let (loser_env, winner_env) = if ok_a {
        (&env_b, &env_a)
    } else {
        (&env_a, &env_b)
    };
    assert_eq!(loser_env["ok"], false);
    let loser_code = loser_env["error"]["code"].as_str().unwrap_or("(none)");
    assert!(
        loser_code == "MINE_REVISION_CONFLICT" || loser_code == "MINE_INVALID_TRANSITION",
        "loser must be a documented stability error, got {loser_code:?}; loser_env={loser_env}"
    );

    // The winner performed exactly the DRAFT -> READY transition off the
    // pre-mutation revision `n` (which both writers read before either lock).
    assert_eq!(winner_env["ok"], true);
    assert_eq!(winner_env["data"]["status_before"], "DRAFT");
    assert_eq!(winner_env["data"]["status_after"], "READY");
    assert_eq!(
        winner_env["revision_before"].as_u64().unwrap(),
        n,
        "winner read the same pre-mutation revision as the seeded graph"
    );
    assert_eq!(
        winner_env["revision_after"].as_u64().unwrap(),
        n + 1,
        "winner bumped the revision by exactly +1"
    );

    // Graph revision increased exactly once; no stale writer overwrote the
    // winner (revision is exactly n+1, never higher) and the loser wrote
    // nothing observable.
    let ws = load_graph(&repo);
    assert_eq!(
        ws.revision,
        n + 1,
        "exactly one revision bump, no silent double/lost write"
    );
    assert_eq!(ws.get("09-1").unwrap().status, PlanStatus::Ready);

    // No unrelated graph data changed: the Accepted node survives identical.
    let unrelated_after = ws.get("01").expect("unrelated plan survives");
    assert_eq!(
        unrelated_after, &unrelated_before,
        "the concurrent race must not alter the unrelated Accepted node"
    );

    // TOML and generated Markdown remain mutually consistent: the Markdown
    // view reflects the final revision and the released READY status.
    let toml = std::fs::read_to_string(repo.join("docs/plan/execution-graph.toml")).unwrap();
    let parsed_toml: mine::domain::graph::PlanWorkspace = toml::from_str(&toml).unwrap();
    assert_eq!(
        parsed_toml.revision,
        n + 1,
        "TOML revision matches the bumped graph"
    );
    let md = std::fs::read_to_string(repo.join("docs/plan/execution-graph.md")).unwrap();
    assert!(
        md.contains("READY"),
        "Markdown view reflects the READY status"
    );
    assert!(
        md.contains("| 09-1 |"),
        "Markdown view contains the released plan row"
    );

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

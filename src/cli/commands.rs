//! Command handlers for the `mine` CLI.
//!
//! Each handler returns `Result<(Envelope, Vec<HumanLine>), HandlerError>`. The
//! dispatcher builds the final [`Outcome`](super::Outcome) and renders it.
//!
//! Contract preservation:
//! - human lines for the default mode; JSON envelope (deterministic, sorted
//!   keys) for `--format json`;
//! - stable `MINE_*` error codes and the public exit-code mapping;
//! - revision + optimistic-concurrency via [`TomlStore::save_with_revision`]
//!   (lock → reload → recheck → mutate → atomic write → render);
//! - deterministic Markdown rendering via [`crate::render`];
//! - read-only Git evidence via [`crate::infrastructure::git`];
//! - safe repository-relative path validation via the domain layer;
//! - no shell execution and no Git mutation.

use serde_json::{Value, json};

use crate::application::init_service::InitService;
use crate::application::workspace_service::WorkspaceService;
use crate::cli::context::{CommandContext, build_context, load_config_or_error};
use crate::cli::{HandlerError, envelope_for};
use crate::domain::error::{MineError, MineResult};
use crate::domain::graph::{PlanNode, PlanWorkspace};
use crate::domain::ports::Clock;
use crate::domain::status::PlanStatus;
use crate::domain::validation;
use crate::infrastructure::design_backup::DesignBackup;
use crate::infrastructure::git;
use crate::infrastructure::git::GitEvidence;
use crate::infrastructure::system::{SystemClock, SystemUuidSource};
use crate::output::envelope::Envelope;
use crate::output::human::{self, HumanLine};

/// The unified handler result.
type HandlerResult = Result<(Envelope, Vec<HumanLine>), HandlerError>;

/// Entry point invoked by the dispatcher.
pub fn handle(
    parsed: &crate::cli::ParsedArgs,
    group: &str,
    sub: &str,
    rest: &[String],
) -> HandlerResult {
    match group {
        "init" => init(parsed, rest),
        "status" => status(parsed, rest),
        "doctor" => doctor(parsed, rest),
        "workspace" => match sub {
            "open" => workspace_open(parsed, rest),
            "status" => workspace_status(parsed, rest),
            "close" => workspace_close(parsed, rest),
            _ => Err(HandlerError::usage(format!(
                "unknown workspace subcommand {sub:?}"
            ))),
        },
        "graph" => match sub {
            "validate" => graph_validate(parsed, rest),
            "render" => graph_render(parsed, rest),
            "status" => graph_status(parsed, rest),
            "ready" => graph_ready(parsed, rest),
            "wave" => graph_wave(parsed, rest),
            "show" => graph_show(parsed, rest),
            _ => Err(HandlerError::usage(format!(
                "unknown graph subcommand {sub:?}"
            ))),
        },
        "plan" => match sub {
            "add" => plan_add(parsed, rest),
            "show" => plan_show(parsed, rest),
            "start" => plan_start(parsed, rest),
            "implemented" => plan_implemented(parsed, rest),
            "accept" => plan_accept(parsed, rest),
            "reject" => plan_reject(parsed, rest),
            _ => Err(HandlerError::usage(format!(
                "unknown plan subcommand {sub:?}"
            ))),
        },
        "design" => match sub {
            "backup" => design_backup(parsed, rest),
            "validate" => design_validate(parsed, rest),
            "status" => design_status(parsed, rest),
            _ => Err(HandlerError::usage(format!(
                "unknown design subcommand {sub:?}"
            ))),
        },
        "repository" => match sub {
            "version" => repository_version(parsed, rest),
            _ => {
                // Allow `mine repository version <verb>` (verb parsed inside).
                if sub.is_empty() {
                    Err(HandlerError::usage(
                        "repository expects a subcommand: version show|suggest|set",
                    ))
                } else {
                    Err(HandlerError::usage(format!(
                        "unknown repository subcommand {sub:?}"
                    )))
                }
            }
        },
        _ => Err(HandlerError::usage(format!(
            "unknown command group {group:?}"
        ))),
    }
}

// ----------------------------------------------------------------------------
// helpers
// ----------------------------------------------------------------------------

/// Extracts `--flag value` and `--flag=value` pairs from a token list, leaving
/// positional tokens in `positional`.
fn parse_flags(rest: &[String]) -> (Vec<(String, String)>, Vec<String>) {
    let mut flags = Vec::new();
    let mut positional = Vec::new();
    let mut i = 0;
    while i < rest.len() {
        let a = &rest[i];
        if let Some(eq) = a
            .strip_prefix("--")
            .and_then(|s| s.find('=').map(|p| p + 2))
        {
            let key = a[..eq].trim_start_matches('-').to_string();
            let val = a[eq + 1..].to_string();
            flags.push((key, val));
        } else if a.starts_with("--") {
            let key = a.trim_start_matches('-').to_string();
            let val = rest.get(i + 1).cloned().unwrap_or_default();
            flags.push((key, val));
            i += 1;
        } else {
            positional.push(a.clone());
        }
        i += 1;
    }
    (flags, positional)
}

fn flag<'a>(flags: &'a [(String, String)], name: &str) -> Option<&'a str> {
    flags
        .iter()
        .find(|(k, _)| k == name)
        .map(|(_, v)| v.as_str())
}

fn flags_all<'a>(flags: &'a [(String, String)], name: &str) -> Vec<&'a str> {
    flags
        .iter()
        .filter(|(k, _)| k == name)
        .map(|(_, v)| v.as_str())
        .collect()
}

fn ctx_err(e: MineError) -> HandlerError {
    HandlerError::from_mine(&e)
}

// ----------------------------------------------------------------------------
// init
// ----------------------------------------------------------------------------

fn init(parsed: &crate::cli::ParsedArgs, _rest: &[String]) -> HandlerResult {
    let repo_root =
        crate::cli::context::resolve_repo_root(parsed.global.repo.as_deref()).map_err(ctx_err)?;
    // Detect the stable branch from Git evidence, falling back to the managed
    // default. The init service records the configured default when git is
    // unavailable; we enrich its outcome with the detected branch.
    let detected_branch = git::detect_stable_branch(&repo_root, "master");
    let svc = InitService::new(&SystemUuidSource, &SystemClock);
    let outcome = svc.initialize(&repo_root).map_err(ctx_err)?;
    let mut env = envelope_for("init", Some(&repo_root))
        .with_workspace_id(outcome.repository_id.clone())
        .with_data(json!({
            "repository_id": outcome.repository_id,
            "mine_code_version": outcome.mine_code_version,
            "design_root": match outcome.design_root {
                crate::application::init_service::DesignRootSummary::Absent => "absent-created",
                crate::application::init_service::DesignRootSummary::Managed => "managed-preserved",
            },
            "stable_branch": detected_branch,
            "actions": outcome.actions.iter().map(|a| match a {
                crate::application::init_service::InitAction::Created(p) => json!({"kind":"created","path": p.display().to_string()}),
                crate::application::init_service::InitAction::Preserved(p) => json!({"kind":"preserved","path": p.display().to_string()}),
                crate::application::init_service::InitAction::CreatedSection(p) => json!({"kind":"created-section","path": p.display().to_string()}),
            }).collect::<Vec<_>>(),
        }));
    env = env.with_revision(0, 0); // init is not a graph mutation
    let lines = human::init_report(&outcome);
    Ok((env, lines))
}

// ----------------------------------------------------------------------------
// status / doctor
// ----------------------------------------------------------------------------

fn status(parsed: &crate::cli::ParsedArgs, _rest: &[String]) -> HandlerResult {
    let ctx = build_context(&parsed.global).map_err(ctx_err)?;
    let ws = ctx.store.load().ok();
    let git = GitEvidence::collect(&ctx.repo_root);
    let mut env = envelope_for("status", Some(&ctx.repo_root)).with_data(json!({
        "git": {
            "current_branch": git.current_branch,
            "head_commit": git.head_commit,
            "clean": git.clean,
        },
    }));
    if let Some(w) = &ws {
        env = env
            .with_workspace_id(w.workspace_id.clone())
            .with_revision(w.revision, w.revision)
            .with_data(json!({
                "git": {
                    "current_branch": git.current_branch,
                    "head_commit": git.head_commit,
                    "clean": git.clean,
                },
                "workspace": graph_summary(w),
            }));
    }
    let lines = human::status_report(&ctx.repo_root, ws.as_ref());
    Ok((env, lines))
}

fn doctor(parsed: &crate::cli::ParsedArgs, _rest: &[String]) -> HandlerResult {
    let ctx = build_context(&parsed.global).map_err(ctx_err)?;
    let mut checks: Vec<(&'static str, bool, String)> = Vec::new();
    let config = load_config_or_error(&ctx.repo_root).ok();
    let config_present = config.is_some();
    checks.push((
        "config",
        config_present,
        if config_present {
            "ok".to_string()
        } else {
            "missing .mine/config.toml (run `mine init`)".to_string()
        },
    ));
    let marker_ok = ctx.repo_root.join("docs/design/.mine-design.toml").exists();
    checks.push((
        "design_marker",
        marker_ok,
        if marker_ok {
            "ok".to_string()
        } else {
            "missing docs/design/.mine-design.toml".to_string()
        },
    ));
    let index_ok = ctx.repo_root.join("docs/design/index.md").exists();
    checks.push((
        "design_index",
        index_ok,
        if index_ok {
            "ok".to_string()
        } else {
            "missing docs/design/index.md".to_string()
        },
    ));
    let graph_ok = ctx.store.load().is_ok();
    checks.push((
        "graph",
        graph_ok,
        if graph_ok {
            "ok".to_string()
        } else {
            "graph not initialized/invalid".to_string()
        },
    ));
    let git = GitEvidence::collect(&ctx.repo_root);
    let git_ok = git.current_branch.is_some();
    checks.push((
        "git",
        git_ok,
        if git_ok {
            format!("branch={}", git.current_branch.as_deref().unwrap_or("?"))
        } else {
            "no git repository detected".to_string()
        },
    ));
    let all_ok = checks.iter().all(|(_, ok, _)| *ok);

    let mut env = envelope_for("doctor", Some(&ctx.repo_root)).with_data(json!({
        "healthy": all_ok,
        "checks": checks.iter().map(|(name, ok, msg)| json!({
            "name": name, "ok": ok, "message": msg,
        })).collect::<Vec<_>>(),
    }));
    if ctx.store.load().ok().is_some() {
        if let Ok(w) = ctx.store.load() {
            env = env
                .with_workspace_id(w.workspace_id.clone())
                .with_revision(w.revision, w.revision);
        }
    }
    let mut lines = vec![HumanLine::Section(format!(
        "mine doctor: {}",
        if all_ok { "healthy" } else { "issues found" }
    ))];
    for (name, ok, msg) in &checks {
        lines.push(HumanLine::Field {
            key: format!("  {}", name),
            value: format!("{}: {}", if *ok { "ok" } else { "FAIL" }, msg),
        });
    }
    if !all_ok {
        return Err(HandlerError {
            code: "MINE_DOCTOR",
            message: "one or more MINE checks failed".to_string(),
            exit_code: crate::output::exit_code::GATE,
            details: json!({
                "checks": checks.iter().map(|(name, ok, msg)| json!({
                    "name": name, "ok": ok, "message": msg,
                })).collect::<Vec<_>>(),
            }),
        });
    }
    Ok((env, lines))
}

fn graph_summary(w: &PlanWorkspace) -> Value {
    json!({
        "workspace_id": w.workspace_id,
        "revision": w.revision,
        "stable_branch": w.stable_branch,
        "integration_branch": w.integration_branch,
        "plan_count": w.plans.len(),
        "ready": validation::ready_frontier(w),
    })
}

// ----------------------------------------------------------------------------
// workspace
// ----------------------------------------------------------------------------

fn workspace_open(parsed: &crate::cli::ParsedArgs, _rest: &[String]) -> HandlerResult {
    let ctx = build_context(&parsed.global).map_err(ctx_err)?;
    let baseline = git::head_commit(&ctx.repo_root).unwrap_or_default();
    let svc = WorkspaceService::new(&SystemUuidSource, &SystemClock);
    let outcome = svc.open(&ctx.store, &baseline).map_err(ctx_err)?;
    let mut env = envelope_for("workspace.open", Some(&ctx.repo_root))
        .with_workspace_id(outcome.workspace_id.clone())
        .with_revision(outcome.revision_before, outcome.revision_after)
        .with_data(json!({
            "workspace_id": outcome.workspace_id,
            "stable_baseline_commit": outcome.stable_baseline_commit,
            "integration_branch": outcome.integration_branch,
        }));
    let lines = vec![
        HumanLine::Section("mine workspace open".to_string()),
        HumanLine::Field {
            key: "  workspace_id".to_string(),
            value: outcome.workspace_id.clone(),
        },
        HumanLine::Field {
            key: "  revision".to_string(),
            value: format!("{} -> {}", outcome.revision_before, outcome.revision_after),
        },
    ];
    let _ = &mut env;
    Ok((env, lines))
}

fn workspace_status(parsed: &crate::cli::ParsedArgs, _rest: &[String]) -> HandlerResult {
    let ctx = build_context(&parsed.global).map_err(ctx_err)?;
    let svc = WorkspaceService::new(&SystemUuidSource, &SystemClock);
    let outcome = svc.status(&ctx.store).map_err(ctx_err)?;
    let env = envelope_for("workspace.status", Some(&ctx.repo_root))
        .with_workspace_id(outcome.workspace_id.clone())
        .with_revision(outcome.revision, outcome.revision)
        .with_data(json!({
            "workspace_id": outcome.workspace_id,
            "revision": outcome.revision,
            "integration_branch": outcome.integration_branch,
            "stable_branch": outcome.stable_branch,
            "stable_baseline_commit": outcome.stable_baseline_commit,
            "plan_count": outcome.plan_count,
            "has_unresolved": outcome.has_unresolved,
        }));
    let lines = vec![
        HumanLine::Section("mine workspace status".to_string()),
        HumanLine::Field {
            key: "  workspace_id".to_string(),
            value: outcome.workspace_id,
        },
        HumanLine::Field {
            key: "  revision".to_string(),
            value: outcome.revision.to_string(),
        },
        HumanLine::Field {
            key: "  has_unresolved".to_string(),
            value: outcome.has_unresolved.to_string(),
        },
    ];
    Ok((env, lines))
}

fn workspace_close(parsed: &crate::cli::ParsedArgs, _rest: &[String]) -> HandlerResult {
    let ctx = build_context(&parsed.global).map_err(ctx_err)?;
    let svc = WorkspaceService::new(&SystemUuidSource, &SystemClock);
    let outcome = svc.close(&ctx.store).map_err(ctx_err)?;
    let env = envelope_for("workspace.close", Some(&ctx.repo_root))
        .with_workspace_id(outcome.workspace_id.clone())
        .with_revision(outcome.revision, outcome.revision)
        .with_data(json!({
            "workspace_id": outcome.workspace_id,
            "closable": !outcome.has_unresolved,
        }));
    let lines = vec![
        HumanLine::Section("mine workspace close".to_string()),
        HumanLine::Field {
            key: "  closable".to_string(),
            value: (!outcome.has_unresolved).to_string(),
        },
    ];
    Ok((env, lines))
}

// ----------------------------------------------------------------------------
// graph
// ----------------------------------------------------------------------------

fn graph_validate(parsed: &crate::cli::ParsedArgs, _rest: &[String]) -> HandlerResult {
    let ctx = build_context(&parsed.global).map_err(ctx_err)?;
    let ws = ctx.store.load().map_err(ctx_err)?;
    // load() already validates; confirm revision-parity with the rendered
    // view if it exists.
    let mut warnings: Vec<Value> = Vec::new();
    if ctx.store.md_path().exists() {
        if let Err(e) = validate_md_parity(&ctx) {
            warnings.push(json!({"code":"MINE_GRAPH_PARITY","message": format!("{e}")}));
        }
    }
    let env = envelope_for("graph.validate", Some(&ctx.repo_root))
        .with_workspace_id(ws.workspace_id.clone())
        .with_revision(ws.revision, ws.revision)
        .with_data(json!({"plans": ws.plans.len(), "warnings_emitted": !warnings.is_empty()}))
        .with_warning_if(
            !warnings.is_empty(),
            "MINE_GRAPH_PARITY",
            "rendered view is stale",
        );
    let lines = vec![
        HumanLine::Section("mine graph validate: ok".to_string()),
        HumanLine::Field {
            key: "  plans".to_string(),
            value: ws.plans.len().to_string(),
        },
        HumanLine::Field {
            key: "  revision".to_string(),
            value: ws.revision.to_string(),
        },
    ];
    let _ = warnings;
    Ok((env, lines))
}

fn graph_render(parsed: &crate::cli::ParsedArgs, _rest: &[String]) -> HandlerResult {
    let ctx = build_context(&parsed.global).map_err(ctx_err)?;
    let rev_before = ctx.store.load().ok().map(|w| w.revision).unwrap_or(0);
    ctx.store.render().map_err(ctx_err)?;
    let rev_after = ctx.store.load().map(|w| w.revision).unwrap_or(rev_before);
    let env = envelope_for("graph.render", Some(&ctx.repo_root))
        .with_revision(rev_before, rev_after)
        .with_data(json!({"rendered_path": ctx.store.md_path().display().to_string()}));
    let lines = vec![
        HumanLine::Section("mine graph render".to_string()),
        HumanLine::Field {
            key: "  output".to_string(),
            value: ctx.store.md_path().display().to_string(),
        },
    ];
    Ok((env, lines))
}

fn graph_status(parsed: &crate::cli::ParsedArgs, _rest: &[String]) -> HandlerResult {
    let ctx = build_context(&parsed.global).map_err(ctx_err)?;
    let ws = ctx.store.load().map_err(ctx_err)?;
    let env = envelope_for("graph.status", Some(&ctx.repo_root))
        .with_workspace_id(ws.workspace_id.clone())
        .with_revision(ws.revision, ws.revision)
        .with_data(graph_summary(&ws));
    let lines = human::status_report(&ctx.repo_root, Some(&ws));
    Ok((env, lines))
}

fn graph_ready(parsed: &crate::cli::ParsedArgs, _rest: &[String]) -> HandlerResult {
    let ctx = build_context(&parsed.global).map_err(ctx_err)?;
    let ws = ctx.store.load().map_err(ctx_err)?;
    let ready = validation::ready_frontier(&ws);
    let env = envelope_for("graph.ready", Some(&ctx.repo_root))
        .with_workspace_id(ws.workspace_id.clone())
        .with_revision(ws.revision, ws.revision)
        .with_data(json!({"ready": ready}));
    let lines = vec![HumanLine::Field {
        key: "  ready".to_string(),
        value: ready.join(", "),
    }];
    Ok((env, lines))
}

fn graph_wave(parsed: &crate::cli::ParsedArgs, _rest: &[String]) -> HandlerResult {
    let ctx = build_context(&parsed.global).map_err(ctx_err)?;
    let ws = ctx.store.load().map_err(ctx_err)?;
    let wave = validation::parallel_wave(&ws);
    let env = envelope_for("graph.wave", Some(&ctx.repo_root))
        .with_workspace_id(ws.workspace_id.clone())
        .with_revision(ws.revision, ws.revision)
        .with_data(json!({"wave": wave}));
    let lines = vec![HumanLine::Field {
        key: "  wave".to_string(),
        value: wave.join(", "),
    }];
    Ok((env, lines))
}

fn graph_show(parsed: &crate::cli::ParsedArgs, rest: &[String]) -> HandlerResult {
    let ctx = build_context(&parsed.global).map_err(ctx_err)?;
    let ws = ctx.store.load().map_err(ctx_err)?;
    let (flags, _pos) = parse_flags(rest);
    let env_data = if let Some(id) = flag(&flags, "plan") {
        let node = ws.get(id).ok_or_else(|| {
            HandlerError::from_mine(&MineError::PlanNotFound {
                plan_id: id.to_string(),
            })
        })?;
        json!({
            "plan": node_json(node),
        })
    } else {
        json!({
            "plans": ws.plans.iter().map(node_json).collect::<Vec<_>>(),
        })
    };
    let env = envelope_for("graph.show", Some(&ctx.repo_root))
        .with_workspace_id(ws.workspace_id.clone())
        .with_revision(ws.revision, ws.revision)
        .with_data(env_data);
    let lines = human::graph_table(&ws);
    Ok((env, lines))
}

fn node_json(p: &PlanNode) -> Value {
    json!({
        "id": p.id,
        "path": p.path,
        "title": p.title,
        "status": p.status.as_str(),
        "hard_predecessors": p.hard_predecessors,
        "owner": p.owner,
        "run_id": p.run_id,
        "implementation_report": p.implementation_report,
        "implementation_commits": p.implementation_commits,
    })
}

fn validate_md_parity(ctx: &CommandContext) -> MineResult<()> {
    let ws = ctx.store.load()?;
    let md = std::fs::read_to_string(ctx.store.md_path())?;
    let rev = regex_lite_rev(&md);
    if rev == Some(ws.revision) {
        Ok(())
    } else {
        Err(MineError::GraphInvalid {
            detail: format!(
                "rendered view revision {:?} != toml revision {} (run `mine graph render`)",
                rev, ws.revision
            ),
        })
    }
}

/// Extracts the revision from the generated Markdown view without pulling in a
/// regex dependency; the view always contains a `Revision: \`N\`` line.
fn regex_lite_rev(md: &str) -> Option<u64> {
    for line in md.lines() {
        if let Some(rest) = line.strip_prefix("- Revision: `") {
            if let Some(num) = rest.strip_suffix('`') {
                return num.parse().ok();
            }
        }
    }
    None
}

// ----------------------------------------------------------------------------
// plan (mutations via save_with_revision)
// ----------------------------------------------------------------------------

fn plan_add(parsed: &crate::cli::ParsedArgs, rest: &[String]) -> HandlerResult {
    let ctx = build_context(&parsed.global).map_err(ctx_err)?;
    let (flags, _pos) = parse_flags(rest);
    let id = flag(&flags, "id")
        .ok_or_else(|| HandlerError::usage("plan add requires --id"))?
        .to_string();
    let path = flag(&flags, "path")
        .ok_or_else(|| HandlerError::usage("plan add requires --path"))?
        .to_string();
    let title = flag(&flags, "title")
        .ok_or_else(|| HandlerError::usage("plan add requires --title"))?
        .to_string();
    let design_refs = flags_all(&flags, "design-ref");
    let design_refs: Vec<String> = if design_refs.is_empty() {
        return Err(HandlerError::usage(
            "plan add requires at least one --design-ref",
        ));
    } else {
        design_refs.iter().map(|s| s.to_string()).collect()
    };
    if design_refs.iter().any(|s| s.is_empty()) {
        return Err(HandlerError::usage("--design-ref must not be empty"));
    }
    let writes: Vec<String> = flags_all(&flags, "write")
        .iter()
        .map(|s| s.to_string())
        .collect();
    let hard: Vec<String> = flags_all(&flags, "hard")
        .iter()
        .map(|s| s.to_string())
        .collect();

    let expected = ctx.store.load().map_err(ctx_err)?.revision;
    let id_for_env = id.clone();
    let saved = save_with_revision(&ctx, expected, move |mut w| {
        if w.get(&id).is_some() {
            return Err(MineError::GraphInvalid {
                detail: format!("plan id {id} already exists"),
            });
        }
        // Validate path safety eagerly for a stable error.
        crate::domain::path::normalize_repo_relative(&path)?;
        let node = PlanNode {
            id: id.clone(),
            path,
            title,
            status: PlanStatus::Draft,
            hard_predecessors: hard.clone(),
            soft_predecessors: vec![],
            design_references: design_refs.clone(),
            exclusive_write_paths: writes.clone(),
            read_only_paths: vec![],
            reserved_shared_paths: vec![],
            implementation_report: String::new(),
            review_report: String::new(),
            implementation_commits: vec![],
            owner: String::new(),
            run_id: String::new(),
            started_at: String::new(),
            updated_at: String::new(),
            rejection_reason: String::new(),
            compensating_plan: String::new(),
        };
        w.plans.push(node);
        w.revision = expected + 1;
        Ok(w)
    })?;

    let node = saved.get(&id_for_env).expect("just added");
    let env = envelope_for("plan.add", Some(&ctx.repo_root))
        .with_workspace_id(saved.workspace_id.clone())
        .with_revision(expected, saved.revision)
        .with_data(json!({"plan": node_json(node)}));
    let lines = vec![HumanLine::Field {
        key: "  added".to_string(),
        value: format!("{} ({})", node.id, node.title),
    }];
    Ok((env, lines))
}

fn plan_show(parsed: &crate::cli::ParsedArgs, rest: &[String]) -> HandlerResult {
    let ctx = build_context(&parsed.global).map_err(ctx_err)?;
    let (flags, _pos) = parse_flags(rest);
    let id = flag(&flags, "id").ok_or_else(|| HandlerError::usage("plan show requires --id"))?;
    let ws = ctx.store.load().map_err(ctx_err)?;
    let node = ws.get(id).ok_or_else(|| {
        HandlerError::from_mine(&MineError::PlanNotFound {
            plan_id: id.to_string(),
        })
    })?;
    let env = envelope_for("plan.show", Some(&ctx.repo_root))
        .with_workspace_id(ws.workspace_id.clone())
        .with_revision(ws.revision, ws.revision)
        .with_data(json!({"plan": node_json(node)}));
    let lines = vec![
        HumanLine::Field {
            key: "  id".to_string(),
            value: node.id.clone(),
        },
        HumanLine::Field {
            key: "  title".to_string(),
            value: node.title.clone(),
        },
        HumanLine::Field {
            key: "  status".to_string(),
            value: node.status.as_str().to_string(),
        },
        HumanLine::Field {
            key: "  hard_predecessors".to_string(),
            value: node.hard_predecessors.join(", "),
        },
    ];
    Ok((env, lines))
}

fn plan_start(parsed: &crate::cli::ParsedArgs, rest: &[String]) -> HandlerResult {
    let ctx = build_context(&parsed.global).map_err(ctx_err)?;
    let (flags, _pos) = parse_flags(rest);
    let id = flag(&flags, "id")
        .ok_or_else(|| HandlerError::usage("plan start requires --id"))?
        .to_string();
    let owner = flag(&flags, "owner").unwrap_or("default").to_string();
    let run_id = flag(&flags, "run-id").unwrap_or("default-run").to_string();
    let now = SystemClock.now_utc_rfc3339();

    let expected = ctx.store.load().map_err(ctx_err)?.revision;
    let id_for_closure = id.clone();
    let id_for_env = id.clone();
    let owner_for_closure = owner.clone();
    let run_id_for_closure = run_id.clone();
    let now_for_closure = now.clone();
    let saved = save_with_revision(&ctx, expected, move |mut w| {
        let current_status =
            w.get(&id_for_closure)
                .map(|n| n.status)
                .ok_or_else(|| MineError::PlanNotFound {
                    plan_id: id_for_closure.clone(),
                })?;
        if !matches!(current_status, PlanStatus::Ready) {
            return Err(MineError::InvalidTransition {
                plan_id: id_for_closure.clone(),
                from: current_status.as_str().to_string(),
                to: PlanStatus::InProgress.as_str().to_string(),
            });
        }
        if !validation::hard_predecessors_accepted(&w, &id_for_closure)? {
            let preds = w
                .get(&id_for_closure)
                .map(|n| n.hard_predecessors.clone())
                .unwrap_or_default();
            let unaccepted = preds
                .into_iter()
                .find(|p| w.get(p).is_some_and(|n| n.status != PlanStatus::Accepted))
                .unwrap_or_default();
            return Err(MineError::PredecessorNotAccepted {
                plan_id: id_for_closure.clone(),
                predecessor_id: unaccepted,
                predecessor_status: "not accepted".to_string(),
            });
        }
        let node = w.get_mut(&id_for_closure).expect("checked present above");
        node.status
            .validate_transition(&id_for_closure, PlanStatus::InProgress)?;
        node.status = PlanStatus::InProgress;
        node.owner = owner_for_closure;
        node.run_id = run_id_for_closure;
        node.started_at = now_for_closure.clone();
        node.updated_at = now_for_closure;
        w.revision = expected + 1;
        Ok(w)
    })?;
    let env = envelope_for("plan.start", Some(&ctx.repo_root))
        .with_workspace_id(saved.workspace_id.clone())
        .with_revision(expected, saved.revision)
        .with_data(json!({"plan": id_for_env}));
    let lines = vec![HumanLine::Field {
        key: "  started".to_string(),
        value: id_for_env,
    }];
    Ok((env, lines))
}

fn plan_implemented(parsed: &crate::cli::ParsedArgs, rest: &[String]) -> HandlerResult {
    let ctx = build_context(&parsed.global).map_err(ctx_err)?;
    let (flags, _pos) = parse_flags(rest);
    let id = flag(&flags, "id")
        .ok_or_else(|| HandlerError::usage("plan implemented requires --id"))?
        .to_string();
    let report = flag(&flags, "report")
        .ok_or_else(|| HandlerError::usage("plan implemented requires --report"))?
        .to_string();
    let commits: Vec<String> = flags_all(&flags, "commit")
        .iter()
        .map(|s| s.to_string())
        .collect();
    if commits.is_empty() {
        return Err(HandlerError::usage(
            "plan implemented requires at least one --commit",
        ));
    }
    let now = SystemClock.now_utc_rfc3339();
    let expected = ctx.store.load().map_err(ctx_err)?.revision;
    let id_for_env = id.clone();
    let commits_for_env = commits.clone();
    let saved = save_with_revision(&ctx, expected, move |mut w| {
        let node = w.get_mut(&id).ok_or_else(|| MineError::PlanNotFound {
            plan_id: id.clone(),
        })?;
        node.status
            .validate_transition(&id, PlanStatus::Implemented)?;
        node.status = PlanStatus::Implemented;
        node.implementation_report = report.clone();
        node.implementation_commits = commits.clone();
        node.updated_at = now.clone();
        w.revision = expected + 1;
        Ok(w)
    })?;
    let env = envelope_for("plan.implemented", Some(&ctx.repo_root))
        .with_workspace_id(saved.workspace_id.clone())
        .with_revision(expected, saved.revision)
        .with_data(json!({"plan": id_for_env, "commits": commits_for_env}));
    let lines = vec![HumanLine::Field {
        key: "  implemented".to_string(),
        value: id_for_env,
    }];
    Ok((env, lines))
}

fn plan_accept(parsed: &crate::cli::ParsedArgs, rest: &[String]) -> HandlerResult {
    let ctx = build_context(&parsed.global).map_err(ctx_err)?;
    let (flags, _pos) = parse_flags(rest);
    let id = flag(&flags, "id")
        .ok_or_else(|| HandlerError::usage("plan accept requires --id"))?
        .to_string();
    let review = flag(&flags, "review").unwrap_or("").to_string();
    if review.is_empty() {
        return Err(HandlerError::usage(
            "plan accept requires --review <review report path>",
        ));
    }
    let now = SystemClock.now_utc_rfc3339();
    let expected = ctx.store.load().map_err(ctx_err)?.revision;
    let id_for_env = id.clone();
    let saved = save_with_revision(&ctx, expected, move |mut w| {
        let current = w
            .get(&id)
            .map(|n| n.status)
            .ok_or_else(|| MineError::PlanNotFound {
                plan_id: id.clone(),
            })?;
        if !matches!(current, PlanStatus::Implemented) {
            return Err(MineError::InvalidTransition {
                plan_id: id.clone(),
                from: current.as_str().to_string(),
                to: PlanStatus::Accepted.as_str().to_string(),
            });
        }
        PlanStatus::Implemented.validate_transition(&id, PlanStatus::Accepted)?;
        // Mark the target as accepted first, then release newly-ready
        // successors whose hard predecessors are all now accepted.
        let accepted_ancestors: std::collections::HashSet<String> = w
            .plans
            .iter()
            .filter(|p| p.status == PlanStatus::Accepted)
            .map(|p| p.id.clone())
            .collect();
        let target_is_only_ancestor_gate = w
            .get(&id)
            .map(|n| n.hard_predecessors.clone())
            .unwrap_or_default();
        for p in w.plans.iter_mut() {
            if p.status == PlanStatus::Blocked
                && !p.hard_predecessors.is_empty()
                && p.hard_predecessors
                    .iter()
                    .all(|hp| accepted_ancestors.contains(hp) || hp == &id)
            {
                p.status = PlanStatus::Ready;
                p.updated_at = now.clone();
            }
        }
        let _ = target_is_only_ancestor_gate;
        let node = w.get_mut(&id).expect("checked present above");
        node.status = PlanStatus::Accepted;
        node.review_report = review.clone();
        node.updated_at = now.clone();
        w.revision = expected + 1;
        Ok(w)
    })?;
    let env = envelope_for("plan.accept", Some(&ctx.repo_root))
        .with_workspace_id(saved.workspace_id.clone())
        .with_revision(expected, saved.revision)
        .with_data(json!({"plan": id_for_env}));
    let lines = vec![HumanLine::Field {
        key: "  accepted".to_string(),
        value: id_for_env,
    }];
    Ok((env, lines))
}

fn plan_reject(parsed: &crate::cli::ParsedArgs, rest: &[String]) -> HandlerResult {
    let ctx = build_context(&parsed.global).map_err(ctx_err)?;
    let (flags, _pos) = parse_flags(rest);
    let id = flag(&flags, "id")
        .ok_or_else(|| HandlerError::usage("plan reject requires --id"))?
        .to_string();
    let reason = flag(&flags, "reason")
        .ok_or_else(|| HandlerError::usage("plan reject requires --reason"))?
        .to_string();
    let compensating = flag(&flags, "compensating-plan").unwrap_or("").to_string();
    if compensating.is_empty() {
        return Err(HandlerError::usage(
            "plan reject requires --compensating-plan <id>",
        ));
    }
    let now = SystemClock.now_utc_rfc3339();
    let expected = ctx.store.load().map_err(ctx_err)?.revision;
    let id_for_env = id.clone();
    let compensating_for_env = compensating.clone();
    let saved = save_with_revision(&ctx, expected, move |mut w| {
        let node = w.get_mut(&id).ok_or_else(|| MineError::PlanNotFound {
            plan_id: id.clone(),
        })?;
        node.status.validate_transition(&id, PlanStatus::Rejected)?;
        node.status = PlanStatus::Rejected;
        node.rejection_reason = reason.clone();
        node.compensating_plan = compensating.clone();
        node.updated_at = now.clone();
        // Downstream rerouting is the reviewer's responsibility; we leave
        // successor predecessor edges to a `plan add` of the compensation node
        // (kept bounded).
        w.revision = expected + 1;
        Ok(w)
    })?;
    let env = envelope_for("plan.reject", Some(&ctx.repo_root))
        .with_workspace_id(saved.workspace_id.clone())
        .with_revision(expected, saved.revision)
        .with_data(json!({"plan": id_for_env, "compensating_plan": compensating_for_env}));
    let lines = vec![HumanLine::Field {
        key: "  rejected".to_string(),
        value: id_for_env,
    }];
    Ok((env, lines))
}

// ----------------------------------------------------------------------------
// design
// ----------------------------------------------------------------------------

fn design_backup(parsed: &crate::cli::ParsedArgs, _rest: &[String]) -> HandlerResult {
    let ctx = build_context(&parsed.global).map_err(ctx_err)?;
    let config = load_config_or_error(&ctx.repo_root).map_err(ctx_err)?;
    let svc = DesignBackup::new(&SystemClock);
    let outcome = svc.backup(&ctx.repo_root, &config).map_err(ctx_err)?;
    let env = envelope_for("design.backup", Some(&ctx.repo_root)).with_data(json!({
        "backup_path": outcome.backup_path_relative,
        "timestamp": outcome.timestamp,
        "file_count": outcome.file_count,
        "total_bytes": outcome.total_bytes,
    }));
    let lines = vec![
        HumanLine::Section("mine design backup".to_string()),
        HumanLine::Field {
            key: "  backup_path".to_string(),
            value: outcome.backup_path_relative,
        },
        HumanLine::Field {
            key: "  files".to_string(),
            value: outcome.file_count.to_string(),
        },
    ];
    Ok((env, lines))
}

fn design_validate(parsed: &crate::cli::ParsedArgs, _rest: &[String]) -> HandlerResult {
    let ctx = build_context(&parsed.global).map_err(ctx_err)?;
    let config = load_config_or_error(&ctx.repo_root).map_err(ctx_err)?;
    let mut warnings: Vec<Value> = Vec::new();
    let marker_path = ctx.repo_root.join(&config.design.marker);
    let marker_ok = marker_path.exists();
    let index_ok = ctx.repo_root.join("docs/design/index.md").exists();
    let mut ok = true;
    if !marker_ok {
        ok = false;
    }
    if !index_ok {
        warnings.push(
            json!({"code":"MINE_DESIGN_INDEX_MISSING","message":"docs/design/index.md missing"}),
        );
        ok = false;
    }
    // Stable branch must not contain docs/plan/ (read-only Git evidence: no
    // tracking check here, but flag presence under git as a warning).
    if git::current_branch(&ctx.repo_root)
        .ok()
        .flatten()
        .as_deref()
        == Some(config.branches.stable.as_str())
        && ctx
            .repo_root
            .join("docs/plan")
            .join("execution-graph.toml")
            .exists()
    {
        warnings.push(
            json!({"code":"MINE_PLANS_ON_STABLE","message":"docs/plan found on the stable branch"}),
        );
    }
    let env = envelope_for("design.validate", Some(&ctx.repo_root)).with_data(json!({
        "valid": ok,
        "warnings": warnings,
    }));
    let lines = vec![HumanLine::Field {
        key: "  valid".to_string(),
        value: ok.to_string(),
    }];
    Ok((env, lines))
}

fn design_status(parsed: &crate::cli::ParsedArgs, _rest: &[String]) -> HandlerResult {
    let ctx = build_context(&parsed.global).map_err(ctx_err)?;
    let config = load_config_or_error(&ctx.repo_root).map_err(ctx_err)?;
    let marker_path = ctx.repo_root.join(&config.design.marker);
    let marker = marker_path
        .exists()
        .then(|| {
            std::fs::read_to_string(&marker_path).ok().and_then(|c| {
                crate::domain::design_marker::DesignMarker::parse(&marker_path, &c).ok()
            })
        })
        .flatten();
    let env = envelope_for("design.status", Some(&ctx.repo_root)).with_data(json!({
        "managed": marker.is_some(),
        "repository_id": marker.as_ref().map(|m| m.repository_id.clone()),
        "created_at": marker.as_ref().map(|m| m.created_at.clone()),
        "design_root": config.design.root,
    }));
    let lines = vec![
        HumanLine::Field {
            key: "  managed".to_string(),
            value: marker.is_some().to_string(),
        },
        HumanLine::Field {
            key: "  design_root".to_string(),
            value: config.design.root,
        },
    ];
    Ok((env, lines))
}

// ----------------------------------------------------------------------------
// repository version
// ----------------------------------------------------------------------------

fn repository_version(parsed: &crate::cli::ParsedArgs, rest: &[String]) -> HandlerResult {
    // `mine repository version show|suggest|set [...]`
    let verb = rest.first().map(String::as_str).unwrap_or("show");
    match verb {
        "show" => repo_version_show(parsed, &rest[1..]),
        "suggest" => repo_version_suggest(parsed, &rest[1..]),
        "set" => repo_version_set(parsed, &rest[1..]),
        _ => Err(HandlerError::usage(format!(
            "unknown repository version subcommand {verb:?} (show|suggest|set)"
        ))),
    }
}

fn repo_version_show(parsed: &crate::cli::ParsedArgs, _rest: &[String]) -> HandlerResult {
    let ctx = build_context(&parsed.global).map_err(ctx_err)?;
    let config = load_config_or_error(&ctx.repo_root).map_err(ctx_err)?;
    let env = envelope_for("repository.version", Some(&ctx.repo_root)).with_data(json!({
        "version": config.mine_code_version,
        "repository_id": config.repository_id,
    }));
    let lines = vec![HumanLine::Field {
        key: "  version".to_string(),
        value: config.mine_code_version,
    }];
    Ok((env, lines))
}

fn repo_version_suggest(parsed: &crate::cli::ParsedArgs, _rest: &[String]) -> HandlerResult {
    let ctx = build_context(&parsed.global).map_err(ctx_err)?;
    let config = load_config_or_error(&ctx.repo_root).map_err(ctx_err)?;
    // A minimal suggestion: increment the patch component of the recorded
    // version, clamping to a stable MAJOR.MINOR.PATCH form. Deterministic and
    // side-effect-free.
    let suggested = suggest_next_version(&config.mine_code_version);
    let env = envelope_for("repository.version", Some(&ctx.repo_root)).with_data(json!({
        "current": config.mine_code_version,
        "suggested": suggested,
    }));
    let lines = vec![
        HumanLine::Field {
            key: "  current".to_string(),
            value: config.mine_code_version.clone(),
        },
        HumanLine::Field {
            key: "  suggested".to_string(),
            value: suggested,
        },
    ];
    Ok((env, lines))
}

fn repo_version_set(parsed: &crate::cli::ParsedArgs, rest: &[String]) -> HandlerResult {
    let ctx = build_context(&parsed.global).map_err(ctx_err)?;
    let (flags, _pos) = parse_flags(rest);
    let new_version = flag(&flags, "version")
        .ok_or_else(|| HandlerError::usage("repository version set requires --version <semver>"))?
        .to_string();
    if !is_valid_semver(&new_version) {
        return Err(HandlerError::usage(format!(
            "invalid --version {new_version:?}; expected MAJOR.MINOR.PATCH"
        )));
    }
    let config_path = ctx.repo_root.join(".mine").join("config.toml");
    let mut config = load_config_or_error(&ctx.repo_root).map_err(ctx_err)?;
    let old = config.mine_code_version.clone();
    config.mine_code_version = new_version.clone();
    let content = config.to_toml();
    crate::infrastructure::atomic_write::write(&config_path, content.as_bytes())
        .map_err(ctx_err)?;
    let env = envelope_for("repository.version", Some(&ctx.repo_root)).with_data(json!({
        "previous": old,
        "current": new_version,
    }));
    let lines = vec![
        HumanLine::Field {
            key: "  previous".to_string(),
            value: old,
        },
        HumanLine::Field {
            key: "  current".to_string(),
            value: new_version,
        },
    ];
    Ok((env, lines))
}

fn suggest_next_version(v: &str) -> String {
    let parts: Vec<&str> = v.split('.').collect();
    if parts.len() == 3 {
        if let Ok(patch) = parts[2].parse::<u32>() {
            return format!("{}.{}.{}", parts[0], parts[1], patch + 1);
        }
    }
    format!("{v}+1")
}

fn is_valid_semver(v: &str) -> bool {
    let parts: Vec<&str> = v.split('.').collect();
    parts.len() == 3 && parts.iter().all(|p| p.parse::<u32>().is_ok())
}

// ----------------------------------------------------------------------------
// persistent mutation helper
// ----------------------------------------------------------------------------

/// Wraps [`TomlStore::save_with_revision`] with a uniform error mapping that
/// surfaces partial-success render failures as exit code 7 (PARTIAL) rather
/// than 4, per the exit-code contract ("partial success requiring repair").
fn save_with_revision(
    ctx: &CommandContext,
    expected: u64,
    mutate: impl FnOnce(PlanWorkspace) -> MineResult<PlanWorkspace>,
) -> Result<PlanWorkspace, HandlerError> {
    match ctx.store.save_with_revision(expected, mutate) {
        Ok(ws) => Ok(ws),
        Err(MineError::GraphInvalid { detail }) if detail.contains("render") => Err(HandlerError {
            code: "MINE_GRAPH_RENDER_PARTIAL",
            message: detail,
            exit_code: crate::output::exit_code::PARTIAL,
            details: Value::Null,
        }),
        Err(e) => Err(ctx_err(e)),
    }
}

// ----------------------------------------------------------------------------
// small trait extension for ergonomic envelope-warning building
// ----------------------------------------------------------------------------

trait EnvelopeWarningExt {
    fn with_warning_if(self, cond: bool, code: &str, message: &str) -> Self;
}

impl EnvelopeWarningExt for Envelope {
    fn with_warning_if(self, cond: bool, code: &str, message: &str) -> Self {
        if cond {
            self.with_warning(code, message)
        } else {
            self
        }
    }
}

// A temporary shim for `Envelope::success("").unused()` used in `inflate`;
// kept to avoid dead-code churn. `inflate` is itself a helper reserved for
// future use and not currently called.

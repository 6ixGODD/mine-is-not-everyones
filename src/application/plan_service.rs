//! Application service for plan lifecycle transitions (`add`/`show`/`start`/
//! `implemented`/`accept`/`reject`).
//!
//! Per `docs/design/system/component-architecture.md`, the CLI and MCP
//! adapters call the **same** application services. This module owns the
//! state-machine transition checks, predecessor validation, successor
//! release, and evidence recording. It uses [`GraphService::mutate`] for the
//! lock→reload→recheck-revision→atomic-write→render transaction; it does
//! **not** reimplement persistence, locking, or revision handling.
//!
//! State-machine rules come from `docs/design/execution-graph/
//! state-machine-and-algorithms.md` and the domain
//! `PlanStatus::validate_transition` / `validation::hard_predecessors_accepted`.

use std::collections::HashSet;

use crate::application::graph_service::{
    GraphService, PlanAcceptRequest, PlanAddRequest, PlanImplementedRequest, PlanRejectRequest,
    PlanStartRequest,
};
use crate::domain::error::{MineError, MineResult};
use crate::domain::graph::{PlanNode, PlanWorkspace};
use crate::domain::status::PlanStatus;
use crate::domain::validation;

/// Shared plan lifecycle service. Constructed over a [`GraphService`] and used
/// by both the CLI `plan.*` handlers and the MCP `mine_plan_*` tools.
pub struct PlanService<'a> {
    graph: &'a GraphService<'a>,
}

impl<'a> PlanService<'a> {
    #[must_use]
    pub fn new(graph: &'a GraphService<'a>) -> Self {
        Self { graph }
    }

    /// Looks up a plan by id. Read-only.
    pub fn show(&self, id: &str) -> MineResult<(u64, PlanNode)> {
        let ws = self.graph.validate()?;
        let node = ws.get(id).cloned().ok_or_else(|| MineError::PlanNotFound {
            plan_id: id.to_string(),
        })?;
        Ok((ws.revision, node))
    }

    /// Adds a new plan node (status `DRAFT`).
    pub fn add(&self, req: PlanAddRequest) -> MineResult<PlanWorkspace> {
        let expected = self.graph.validate()?.revision;
        self.graph.mutate(expected, move |mut w| {
            if w.get(&req.id).is_some() {
                return Err(MineError::GraphInvalid {
                    detail: format!("plan id {} already exists", req.id),
                });
            }
            // Validate path safety eagerly for a stable error.
            crate::domain::path::normalize_repo_relative(&req.path)?;
            if req.design_references.is_empty() {
                return Err(MineError::GraphInvalid {
                    detail: format!("plan {} has no design references", req.id),
                });
            }
            for dr in &req.design_references {
                if dr.is_empty() {
                    return Err(MineError::GraphInvalid {
                        detail: "design-reference must not be empty".to_string(),
                    });
                }
                crate::domain::path::normalize_repo_relative(dr)?;
            }
            for hp in &req.hard_predecessors {
                if !w.ids().contains(hp.as_str()) {
                    return Err(MineError::GraphInvalid {
                        detail: format!("plan {} hard_predecessor {} not found", req.id, hp),
                    });
                }
            }
            let node = PlanNode {
                id: req.id,
                path: req.path,
                title: req.title,
                status: PlanStatus::Draft,
                hard_predecessors: req.hard_predecessors,
                soft_predecessors: vec![],
                design_references: req.design_references,
                exclusive_write_paths: req.exclusive_write_paths,
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
        })
    }

    /// Starts a plan: requires `READY` + hard predecessors `ACCEPTED`.
    pub fn start(&self, req: PlanStartRequest) -> MineResult<PlanWorkspace> {
        let expected = self.graph.validate()?.revision;
        self.graph.mutate(expected, move |mut w| {
            let current =
                w.get(&req.id)
                    .map(|n| n.status)
                    .ok_or_else(|| MineError::PlanNotFound {
                        plan_id: req.id.clone(),
                    })?;
            if current != PlanStatus::Ready {
                return Err(MineError::InvalidTransition {
                    plan_id: req.id.clone(),
                    from: current.as_str().to_string(),
                    to: PlanStatus::InProgress.as_str().to_string(),
                });
            }
            if !validation::hard_predecessors_accepted(&w, &req.id)? {
                let preds = w
                    .get(&req.id)
                    .map(|n| n.hard_predecessors.clone())
                    .unwrap_or_default();
                let unaccepted = preds
                    .into_iter()
                    .find(|p| w.get(p).is_some_and(|n| n.status != PlanStatus::Accepted))
                    .unwrap_or_default();
                return Err(MineError::PredecessorNotAccepted {
                    plan_id: req.id.clone(),
                    predecessor_id: unaccepted,
                    predecessor_status: "not accepted".to_string(),
                });
            }
            let node = w.get_mut(&req.id).expect("checked present above");
            node.status
                .validate_transition(&req.id, PlanStatus::InProgress)?;
            node.status = PlanStatus::InProgress;
            node.owner = req.owner;
            node.run_id = req.run_id;
            node.started_at = req.started_at.clone();
            node.updated_at = req.started_at;
            w.revision = expected + 1;
            Ok(w)
        })
    }

    /// Marks a plan implemented: records the report + commits.
    pub fn mark_implemented(&self, req: PlanImplementedRequest) -> MineResult<PlanWorkspace> {
        let expected = self.graph.validate()?.revision;
        self.graph.mutate(expected, move |mut w| {
            let node = w.get_mut(&req.id).ok_or_else(|| MineError::PlanNotFound {
                plan_id: req.id.clone(),
            })?;
            node.status
                .validate_transition(&req.id, PlanStatus::Implemented)?;
            node.status = PlanStatus::Implemented;
            node.implementation_report = req.report;
            node.implementation_commits = req.commits;
            node.updated_at = req.updated_at;
            w.revision = expected + 1;
            Ok(w)
        })
    }

    /// Accepts a plan: requires `IMPLEMENTED`; releases eligible BLOCKED
    /// successors whose hard predecessors are all now accepted.
    pub fn accept(&self, req: PlanAcceptRequest) -> MineResult<PlanWorkspace> {
        let expected = self.graph.validate()?.revision;
        self.graph.mutate(expected, move |mut w| {
            let current =
                w.get(&req.id)
                    .map(|n| n.status)
                    .ok_or_else(|| MineError::PlanNotFound {
                        plan_id: req.id.clone(),
                    })?;
            if current != PlanStatus::Implemented {
                return Err(MineError::InvalidTransition {
                    plan_id: req.id.clone(),
                    from: current.as_str().to_string(),
                    to: PlanStatus::Accepted.as_str().to_string(),
                });
            }
            PlanStatus::Implemented.validate_transition(&req.id, PlanStatus::Accepted)?;
            // Accept the target first, then release newly-ready successors.
            let accepted_ancestors: HashSet<String> = w
                .plans
                .iter()
                .filter(|p| p.status == PlanStatus::Accepted)
                .map(|p| p.id.clone())
                .collect();
            for p in w.plans.iter_mut() {
                if p.status == PlanStatus::Blocked
                    && !p.hard_predecessors.is_empty()
                    && p.hard_predecessors
                        .iter()
                        .all(|hp| accepted_ancestors.contains(hp) || hp == &req.id)
                {
                    p.status = PlanStatus::Ready;
                    p.updated_at = req.updated_at.clone();
                }
            }
            let node = w.get_mut(&req.id).expect("checked present above");
            node.status = PlanStatus::Accepted;
            node.review_report = req.review_report;
            node.updated_at = req.updated_at;
            w.revision = expected + 1;
            Ok(w)
        })
    }

    /// Rejects a plan: records reason + compensating plan; downstream stays
    /// blocked.
    pub fn reject(&self, req: PlanRejectRequest) -> MineResult<PlanWorkspace> {
        let expected = self.graph.validate()?.revision;
        self.graph.mutate(expected, move |mut w| {
            let node = w.get_mut(&req.id).ok_or_else(|| MineError::PlanNotFound {
                plan_id: req.id.clone(),
            })?;
            node.status
                .validate_transition(&req.id, PlanStatus::Rejected)?;
            node.status = PlanStatus::Rejected;
            node.rejection_reason = req.reason;
            node.compensating_plan = req.compensating_plan;
            node.updated_at = req.updated_at;
            w.revision = expected + 1;
            Ok(w)
        })
    }
}

/// A request to release a DRAFT plan to the startable frontier (shared by CLI
/// and MCP).
#[derive(Debug, Clone)]
pub struct PlanReleaseRequest {
    pub id: String,
    pub updated_at: String,
}

/// A request to rewire downstream successors off a REJECTED plan onto its
/// registered compensating plan (shared by CLI and MCP).
#[derive(Debug, Clone)]
pub struct PlanRewireRequest {
    pub id: String,
    pub updated_at: String,
}

impl<'a> PlanService<'a> {
    /// Releases a `DRAFT` plan: `DRAFT -> READY` when every hard predecessor is
    /// `ACCEPTED` (incl. no preds), else `DRAFT -> BLOCKED`. Returns the saved
    /// workspace and the unsatisfied predecessors (in stable order) for
    /// deterministic reporting.
    pub fn release(&self, req: PlanReleaseRequest) -> MineResult<PlanWorkspace> {
        let expected = self.graph.validate()?.revision;
        self.graph.mutate(expected, move |mut w| {
            crate::domain::plan_release::release_plan(&mut w, &req.id, &req.updated_at)?;
            w.revision = expected + 1;
            Ok(w)
        })
    }

    /// Rewires a REJECTED plan's downstream successors onto its registered
    /// compensating plan. The replacement id is derived from the rejected plan's
    /// `compensating_plan` field (the single source of truth). Returns the saved
    /// workspace and the affected successor ids (in stable insertion order); on
    /// the idempotent no-op (no successor still references the rejected id), the
    /// revision is NOT bumped and the affected list is empty.
    pub fn rewire_compensation(
        &self,
        req: PlanRewireRequest,
    ) -> MineResult<(PlanWorkspace, Vec<String>)> {
        let expected = self.graph.validate()?.revision;
        let affected_cell = std::cell::RefCell::new(Vec::<String>::new());
        let saved = self.graph.mutate(expected, |mut w| {
            let affected =
                crate::domain::rewire::rewire_compensation(&mut w, &req.id, &req.updated_at)?;
            if !affected.is_empty() {
                w.revision = expected + 1;
            }
            *affected_cell.borrow_mut() = affected;
            Ok(w)
        })?;
        Ok((saved, affected_cell.into_inner()))
    }
}

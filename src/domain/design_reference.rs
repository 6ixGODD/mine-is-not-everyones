//! Design references attached to plan nodes.
//!
//! `docs/design/execution-graph/domain-model.md` "Design references" defines
//! the structured form `{ path, anchors[], reason }`. The live fact source
//! `docs/plan/execution-graph.toml` currently stores design references as a
//! flat string array (e.g. `["docs/design/principles.md", ...]`) for the
//! bootstrap workspace; this module models both the structured target form and
//! the flat legacy form so the persistence layer can round-trip the existing
//! TOML byte-for-byte while the domain works with typed references.
//!
//! Every plan must reference at least one design leaf. Referencing only
//! `docs/design/index.md` is insufficient unless the plan exclusively changes
//! top-level scope. Path and anchor existence is checked at the persistence
//! layer (which has filesystem access); the domain type holds the data and
//! validates path safety.

use serde::{Deserialize, Serialize};

use crate::domain::error::{MineError, MineResult};
use crate::domain::path::normalize_repo_relative;

/// A structured design reference: a design leaf path, optional in-document
/// anchors, and a human-readable reason.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesignReference {
    /// Repository-relative design leaf path.
    pub path: String,
    /// In-document anchors (e.g. `#json-output`). May be empty.
    #[serde(default)]
    pub anchors: Vec<String>,
    /// Why this design document governs this plan.
    #[serde(default)]
    pub reason: String,
}

impl DesignReference {
    /// Creates and validates a design reference from raw input.
    ///
    /// # Errors
    /// Returns [`MineError::GraphInvalid`] if the path is not a safe
    /// repository-relative path.
    pub fn new(path: &str, anchors: Vec<String>, reason: String) -> MineResult<Self> {
        let normalized = normalize_repo_relative(path)?;
        Ok(Self {
            path: normalized,
            anchors,
            reason,
        })
    }
}

/// Validates a flat list of design-reference path strings (the legacy TOML
/// form) and returns the normalized structured references with empty anchors.
///
/// # Errors
/// Returns [`MineError::GraphInvalid`] if the list is empty (a plan must
/// reference at least one design leaf) or any path is unsafe.
pub fn from_flat_paths(paths: &[String]) -> MineResult<Vec<DesignReference>> {
    if paths.is_empty() {
        return Err(MineError::GraphInvalid {
            detail: "a plan must reference at least one design document".to_string(),
        });
    }
    paths
        .iter()
        .map(|p| DesignReference::new(p, Vec::new(), String::new()))
        .collect()
}

/// Serializes structured references back to the flat path form for
/// byte-compatible round-tripping with the current fact source.
#[must_use]
pub fn to_flat_paths(refs: &[DesignReference]) -> Vec<String> {
    refs.iter().map(|r| r.path.clone()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::error::MineError;

    #[test]
    fn flat_paths_round_trip() -> Result<(), MineError> {
        let paths = vec![
            "docs/design/execution-graph/domain-model.md".to_string(),
            "docs/design/execution-graph/persistence-and-concurrency.md".to_string(),
        ];
        let refs = from_flat_paths(&paths)?;
        assert_eq!(refs.len(), 2);
        assert_eq!(refs[0].path, "docs/design/execution-graph/domain-model.md");
        assert_eq!(to_flat_paths(&refs), paths);
        Ok(())
    }

    #[test]
    fn empty_references_rejected() {
        let e = from_flat_paths(&[]).unwrap_err();
        assert_eq!(e.code(), "MINE_GRAPH_INVALID");
    }

    #[test]
    fn unsafe_reference_path_rejected() {
        let e = from_flat_paths(&["../escape.md".to_string()]).unwrap_err();
        assert_eq!(e.code(), "MINE_GRAPH_INVALID");
    }
}

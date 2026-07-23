//! Typed error model with stable machine-readable codes.
//!
//! Plan 01 defines the error variants needed for repository discovery,
//! design-namespace validation, marker validation, configuration validation,
//! and identity reconciliation. Later plans extend this enum with the
//! execution-graph, plan-lifecycle, revision, lock, distribution, and
//! agent-configuration variants. Each variant exposes a stable [`MineError::code`]
//! string that becomes part of the public JSON error contract in Plan 03.

use std::path::PathBuf;
use thiserror::Error;

/// A MINE domain error carrying a stable machine-readable code.
#[derive(Debug, Error)]
pub enum MineError {
    /// The repository root could not be discovered.
    #[error("repository not found: {detail}")]
    RepositoryNotFound { detail: String },

    /// `docs/design/` exists but is not MINE-managed (no marker, or a foreign
    /// marker). MINE refuses to adopt or migrate legacy content.
    #[error("design namespace conflict at {path}: existing docs/design is not MINE-managed")]
    DesignNamespaceConflict { path: PathBuf },

    /// The design marker belongs to a different repository identifier than the
    /// one already recorded for this repository.
    #[error(
        "design ownership mismatch: marker repository_id {marker_id} does not match {expected_id}"
    )]
    DesignOwnershipMismatch {
        marker_id: String,
        expected_id: String,
    },

    /// The design marker exists but is structurally invalid.
    #[error("design marker is invalid at {path}: {detail}")]
    DesignMarkerInvalid { path: PathBuf, detail: String },

    /// The configuration repository identifier does not match the design marker.
    #[error(
        "repository identity mismatch: config repository_id {config_id} does not match marker {marker_id}"
    )]
    RepositoryIdMismatch {
        config_id: String,
        marker_id: String,
    },

    /// `.mine/config.toml` exists but cannot be parsed or is missing required
    /// fields. MINE does not silently rewrite an invalid source of truth.
    #[error("configuration is invalid at {path}: {detail}")]
    ConfigInvalid { path: PathBuf, detail: String },

    /// A filesystem input/output error.
    #[error("input/output error: {0}")]
    Io(#[from] std::io::Error),
}

impl MineError {
    /// Returns the stable machine-readable error code for this variant.
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::RepositoryNotFound { .. } => "MINE_REPOSITORY_NOT_FOUND",
            Self::DesignNamespaceConflict { .. } => "MINE_DESIGN_NAMESPACE_CONFLICT",
            Self::DesignOwnershipMismatch { .. } => "MINE_DESIGN_OWNERSHIP_MISMATCH",
            Self::DesignMarkerInvalid { .. } => "MINE_DESIGN_MARKER_INVALID",
            Self::RepositoryIdMismatch { .. } => "MINE_REPOSITORY_ID_MISMATCH",
            Self::ConfigInvalid { .. } => "MINE_CONFIG_INVALID",
            Self::Io(_) => "MINE_IO",
        }
    }
}

/// A specialized [`Result`] for MINE domain operations.
pub type MineResult<T> = std::result::Result<T, MineError>;

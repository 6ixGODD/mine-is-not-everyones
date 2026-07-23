//! Design-namespace marker validation.
//!
//! MINE exclusively owns `docs/design/` and identifies managed trees with
//! `docs/design/.mine-design.toml` (ADR-0006). This module defines the marker
//! model and a pure classifier that the initialization service uses to decide
//! whether a design root is absent, MINE-managed, or conflicting. The
//! classifier performs no filesystem I/O; the caller supplies the observed
//! directory/marker existence and the parsed marker.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::domain::error::{MineError, MineResult};

/// File name of the design ownership marker inside `docs/design/`.
pub const DESIGN_MARKER_FILE: &str = ".mine-design.toml";

/// File name of the design root index inside `docs/design/`.
pub const DESIGN_ROOT_INDEX: &str = "index.md";

/// The `docs/design/.mine-design.toml` marker.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesignMarker {
    /// Marker schema version.
    pub schema_version: u32,
    /// Always the literal `MINE` for a MINE-managed tree.
    pub managed_by: String,
    /// Stable repository identifier (UUID) this design tree belongs to.
    pub repository_id: String,
    /// UTC creation timestamp in RFC 3339 format.
    pub created_at: String,
}

impl DesignMarker {
    /// The literal `managed_by` value for a MINE-managed tree.
    pub const MANAGED_BY: &'static str = "MINE";

    /// The supported marker schema version.
    pub const SCHEMA_VERSION: u32 = 1;

    /// Creates a new marker for the given repository identifier and timestamp.
    #[must_use]
    pub fn new(repository_id: String, created_at: String) -> Self {
        Self {
            schema_version: Self::SCHEMA_VERSION,
            managed_by: Self::MANAGED_BY.to_string(),
            repository_id,
            created_at,
        }
    }

    /// Serializes the marker to TOML. Serialization of this flat struct is
    /// infallible in practice.
    #[must_use]
    pub fn to_toml(&self) -> String {
        toml::to_string(self)
            .expect("DesignMarker serialization is infallible for flat scalar fields")
    }

    /// Parses a marker from TOML.
    ///
    /// # Errors
    /// Returns [`MineError::DesignMarkerInvalid`] if the content is not valid
    /// marker TOML.
    pub fn parse(path: &Path, content: &str) -> MineResult<Self> {
        toml::from_str::<Self>(content).map_err(|e| MineError::DesignMarkerInvalid {
            path: path.to_path_buf(),
            detail: format!("could not parse marker TOML: {e}"),
        })
    }
}

/// The classified state of a design root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DesignRootState {
    /// `docs/design/` is absent; initialization may create the scaffold.
    Absent,
    /// `docs/design/` is MINE-managed with a valid matching marker.
    Managed(DesignMarker),
}

/// Classifies a design root given observed filesystem state.
///
/// The caller supplies whether the design directory and marker file exist, the
/// parsed marker (if any), and the repository identifier already recorded for
/// this repository (if any). The function is pure and performs no I/O.
///
/// # Errors
/// - [`MineError::DesignNamespaceConflict`] when the directory exists without a
///   marker, or with a foreign (non-MINE) marker.
/// - [`MineError::DesignOwnershipMismatch`] when a valid MINE marker belongs to
///   a different repository identifier than the one already recorded.
/// - [`MineError::DesignMarkerInvalid`] when the marker file exists but could
///   not be parsed, or has an unsupported schema version.
///
/// # Notes
/// When `expected_repository_id` is `None` (no existing recorded identity), a
/// structurally valid MINE marker is accepted and establishes the repository
/// identifier; ownership mismatch can only be detected against an existing
/// recorded identity.
pub fn classify(
    design_dir: &Path,
    marker_path: &Path,
    design_dir_exists: bool,
    marker_file_exists: bool,
    marker: Option<&DesignMarker>,
    expected_repository_id: Option<&str>,
) -> MineResult<DesignRootState> {
    if !design_dir_exists {
        return Ok(DesignRootState::Absent);
    }

    if !marker_file_exists {
        return Err(MineError::DesignNamespaceConflict {
            path: design_dir.to_path_buf(),
        });
    }

    let marker = marker.ok_or_else(|| MineError::DesignMarkerInvalid {
        path: marker_path.to_path_buf(),
        detail: "marker file exists but could not be parsed".to_string(),
    })?;

    if marker.managed_by != DesignMarker::MANAGED_BY {
        return Err(MineError::DesignNamespaceConflict {
            path: design_dir.to_path_buf(),
        });
    }

    if marker.schema_version != DesignMarker::SCHEMA_VERSION {
        return Err(MineError::DesignMarkerInvalid {
            path: marker_path.to_path_buf(),
            detail: format!(
                "unsupported schema_version {}: expected {}",
                marker.schema_version,
                DesignMarker::SCHEMA_VERSION
            ),
        });
    }

    if let Some(expected) = expected_repository_id {
        if marker.repository_id != expected {
            return Err(MineError::DesignOwnershipMismatch {
                marker_id: marker.repository_id.clone(),
                expected_id: expected.to_string(),
            });
        }
    }

    Ok(DesignRootState::Managed(marker.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn marker(repository_id: &str) -> DesignMarker {
        DesignMarker::new(
            repository_id.to_string(),
            "2026-07-23T00:00:00Z".to_string(),
        )
    }

    #[test]
    fn absent_when_design_dir_missing() {
        let state = classify(
            Path::new("docs/design"),
            Path::new("docs/design/.mine-design.toml"),
            false,
            false,
            None,
            None,
        )
        .expect("absent design root classifies as Absent");
        assert_eq!(state, DesignRootState::Absent);
    }

    #[test]
    fn managed_when_valid_matching_marker() {
        let m = marker("11111111-1111-1111-1111-111111111111");
        let state = classify(
            Path::new("docs/design"),
            Path::new("docs/design/.mine-design.toml"),
            true,
            true,
            Some(&m),
            Some("11111111-1111-1111-1111-111111111111"),
        )
        .expect("valid matching marker classifies as Managed");
        assert_eq!(state, DesignRootState::Managed(m));
    }

    #[test]
    fn legacy_unmarked_directory_is_conflict() {
        let err = classify(
            Path::new("docs/design"),
            Path::new("docs/design/.mine-design.toml"),
            true,
            false,
            None,
            None,
        )
        .expect_err("unmarked directory is a namespace conflict");
        assert_eq!(err.code(), "MINE_DESIGN_NAMESPACE_CONFLICT");
    }

    #[test]
    fn foreign_marker_is_conflict() {
        let mut m = marker("11111111-1111-1111-1111-111111111111");
        m.managed_by = "OtherTool".to_string();
        let err = classify(
            Path::new("docs/design"),
            Path::new("docs/design/.mine-design.toml"),
            true,
            true,
            Some(&m),
            None,
        )
        .expect_err("foreign marker is rejected");
        assert_eq!(err.code(), "MINE_DESIGN_NAMESPACE_CONFLICT");
    }

    #[test]
    fn marker_with_other_repository_id_is_ownership_mismatch() {
        let m = marker("22222222-2222-2222-2222-222222222222");
        let err = classify(
            Path::new("docs/design"),
            Path::new("docs/design/.mine-design.toml"),
            true,
            true,
            Some(&m),
            Some("11111111-1111-1111-1111-111111111111"),
        )
        .expect_err("mismatched repository id is ownership mismatch");
        assert_eq!(err.code(), "MINE_DESIGN_OWNERSHIP_MISMATCH");
    }

    #[test]
    fn malformed_marker_is_invalid() {
        let err = classify(
            Path::new("docs/design"),
            Path::new("docs/design/.mine-design.toml"),
            true,
            true,
            None,
            None,
        )
        .expect_err("unparseable marker is invalid");
        assert_eq!(err.code(), "MINE_DESIGN_MARKER_INVALID");
    }

    #[test]
    fn wrong_schema_version_is_invalid() {
        let mut m = marker("11111111-1111-1111-1111-111111111111");
        m.schema_version = 99;
        let err = classify(
            Path::new("docs/design"),
            Path::new("docs/design/.mine-design.toml"),
            true,
            true,
            Some(&m),
            None,
        )
        .expect_err("unsupported schema version is invalid");
        assert_eq!(err.code(), "MINE_DESIGN_MARKER_INVALID");
    }

    #[test]
    fn marker_establishes_identity_when_no_expected() {
        let m = marker("33333333-3333-3333-3333-333333333333");
        let state = classify(
            Path::new("docs/design"),
            Path::new("docs/design/.mine-design.toml"),
            true,
            true,
            Some(&m),
            None,
        )
        .expect("valid marker with no recorded identity is accepted");
        assert_eq!(state, DesignRootState::Managed(m));
    }

    #[test]
    fn marker_round_trips_through_toml() {
        let m = marker("44444444-4444-4444-4444-444444444444");
        let rendered = m.to_toml();
        let parsed = DesignMarker::parse(Path::new("docs/design/.mine-design.toml"), &rendered)
            .expect("rendered marker parses back");
        assert_eq!(m, parsed);
    }
}

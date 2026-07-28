//! Repository identity and MINE code-repository version persistence.
//!
//! Define the project UUID and the MINE
//! code-repository version persistence. Existing managed values are preserved;
//! an unmanaged repository gets a fresh UUID and the default version `0.1.0`
//! unless reliable root version evidence (for example `Cargo.toml`
//! `[package].version`) is available. The persistence target is
//! `.mine/config.toml` (`repository_id` and `mine_code_version` fields), per
//! the operations design.

use serde::{Deserialize, Serialize};

use crate::domain::ports::UuidSource;

/// The repository identity persisted by `mine init`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepositoryIdentity {
    /// Stable repository identifier (UUID).
    pub repository_id: String,
    /// MINE code-repository version.
    pub mine_code_version: String,
}

impl RepositoryIdentity {
    /// The default version applied to an unmanaged repository when no reliable
    /// root version evidence exists.
    pub const DEFAULT_VERSION: &'static str = "0.1.0";

    /// Resolves the repository identity, preserving existing managed values.
    ///
    /// Identifier priority: an existing valid design marker, then an existing
    /// configuration, then a freshly generated UUID. Version priority: an
    /// existing configuration, then reliable root version evidence, then the
    /// default `0.1.0`.
    #[must_use]
    pub fn resolve(
        marker_repository_id: Option<&str>,
        config_identity: Option<&RepositoryIdentity>,
        uuid_source: &dyn UuidSource,
        root_version: Option<&str>,
    ) -> Self {
        let repository_id = marker_repository_id
            .map(str::to_string)
            .or_else(|| config_identity.map(|c| c.repository_id.clone()))
            .unwrap_or_else(|| uuid_source.new_repository_id());

        let mine_code_version = config_identity
            .map(|c| c.mine_code_version.clone())
            .or_else(|| root_version.map(str::to_string))
            .unwrap_or_else(|| Self::DEFAULT_VERSION.to_string());

        Self {
            repository_id,
            mine_code_version,
        }
    }
}

/// Extracts reliable root version evidence from a Cargo manifest string.
///
/// Returns the `[package].version` value when present and non-empty. This is
/// the only root-version source MINE consults; additional sources
/// may be added by later plans.
#[must_use]
pub fn root_version_from_cargo_manifest(cargo_toml: &str) -> Option<String> {
    #[derive(Deserialize)]
    struct Manifest {
        package: Option<Package>,
    }
    #[derive(Deserialize)]
    struct Package {
        version: Option<String>,
    }

    let version = toml::from_str::<Manifest>(cargo_toml)
        .ok()?
        .package?
        .version?;
    let trimmed = version.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FixedUuid;
    impl UuidSource for FixedUuid {
        fn new_repository_id(&self) -> String {
            "fixed-uuid-0000-0000-000000000000".to_string()
        }
    }

    #[test]
    fn unmanaged_repository_gets_new_uuid_and_default_version() {
        let id = RepositoryIdentity::resolve(None, None, &FixedUuid, None);
        assert_eq!(id.repository_id, "fixed-uuid-0000-0000-000000000000");
        assert_eq!(id.mine_code_version, RepositoryIdentity::DEFAULT_VERSION);
    }

    #[test]
    fn marker_repository_id_is_preserved() {
        let id = RepositoryIdentity::resolve(
            Some("marker-uuid-1111-1111-111111111111"),
            None,
            &FixedUuid,
            None,
        );
        assert_eq!(id.repository_id, "marker-uuid-1111-1111-111111111111");
    }

    #[test]
    fn config_identity_is_preserved_over_root_version() {
        let existing = RepositoryIdentity {
            repository_id: "cfg-uuid-2222-2222-222222222222".to_string(),
            mine_code_version: "1.4.2".to_string(),
        };
        let id = RepositoryIdentity::resolve(None, Some(&existing), &FixedUuid, Some("9.9.9"));
        assert_eq!(id.repository_id, "cfg-uuid-2222-2222-222222222222");
        assert_eq!(id.mine_code_version, "1.4.2");
    }

    #[test]
    fn root_version_is_used_when_no_config() {
        let id = RepositoryIdentity::resolve(
            Some("marker-uuid-3333-3333-333333333333"),
            None,
            &FixedUuid,
            Some("2.3.4"),
        );
        assert_eq!(id.repository_id, "marker-uuid-3333-3333-333333333333");
        assert_eq!(id.mine_code_version, "2.3.4");
    }

    #[test]
    fn cargo_root_version_is_extracted() {
        let manifest = r#"
[package]
name = "mine"
version = "0.1.0"
edition = "2024"

[dependencies]
serde = "1"
"#;
        assert_eq!(
            root_version_from_cargo_manifest(manifest),
            Some("0.1.0".to_string())
        );
    }

    #[test]
    fn cargo_root_version_absent_when_no_package_version() {
        let manifest = "[package]\nname = \"mine\"\n";
        assert_eq!(root_version_from_cargo_manifest(manifest), None);
    }

    #[test]
    fn cargo_root_version_absent_when_unparseable() {
        assert_eq!(root_version_from_cargo_manifest("not = valid = toml"), None);
    }
}

// Enforce `AGENTS.md`'s "Business code must not use `unsafe`" at compile time.
#![forbid(unsafe_code)]

//! Managed installation state — the ownership record that makes safe update,
//! doctor, and uninstall possible.
//!
//! The managed-state record is the **only** proof of MINE ownership. It is:
//!
//! - **written atomically** (stage + rename; validated before use);
//! - **validated before use** (foreign/malformed state is rejected, never
//!   silently trusted);
//! - **free of secrets** (only paths, hashes, versions, timestamps, agent
//!   identity — no file contents, tokens, or credentials);
//! - **never claims ownership of pre-existing user content** — every owned
//!   record is created at install time and incrementally maintained.
//!
//! The on-disk format is JSON (deterministic, sorted keys) at a fixed path
//! inside the configuration root: `<root>/.mine/agent-installs.json`. The
//! schema version field lets future migrations reject incompatible foreign
//! state safely.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::domain::error::{MineError, MineResult};

/// Managed-state schema version. Bumped on incompatible format changes; older
/// versions are rejected as foreign rather than silently migrated.
pub const MANAGED_STATE_SCHEMA_VERSION: u32 = 1;

/// The well-known managed-state file name under the configuration root.
pub const MANAGED_STATE_FILE: &str = ".mine/agent-installs.json";

/// A single MINE-owned file installed into an Agent's tree, with its content
/// hash at install time for drift detection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OwnedFile {
    /// Path relative to the configuration root (forward slashes).
    pub path: String,
    /// Content hash at install time (see [`crate::agent_setup::safety::content_hash`]).
    pub hash: String,
}

/// A single MINE-owned structured configuration entry (e.g. an MCP server
/// entry or a `mcpServers.mine` JSON object), identified by the config file
/// and the JSON pointer to the owned entry, with a hash of the entry value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OwnedConfigEntry {
    /// Path relative to the configuration root (forward slashes).
    pub config_file: String,
    /// JSON pointer (RFC 6901) to the owned entry within the config file.
    pub json_pointer: String,
    /// Hash of the serialized owned entry value at install time.
    pub hash: String,
}

/// The managed-state record for a single Agent installation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentInstallRecord {
    /// The target Agent (`claude-code` | `codex` | `pi` | `opencode`).
    pub agent: String,
    /// The version of MINE recorded at install time.
    pub mine_version: String,
    /// The payload/source identity (the build-time embedded payload marker).
    pub source_identity: String,
    /// Installation destination root (absolute).
    pub destination: String,
    /// MINE-owned files (relative paths + hashes).
    pub files: Vec<OwnedFile>,
    /// MINE-owned structured config entries (config file + JSON pointer + hash).
    pub config_entries: Vec<OwnedConfigEntry>,
    /// UTC timestamp at install time (RFC 3339).
    pub installed_at: String,
    /// The previous managed MINE version, if this was an update.
    pub previous_version: Option<String>,
}

/// The complete managed-state document — all Agent installation records,
/// persisted atomically.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManagedState {
    /// Schema version; must equal [`MANAGED_STATE_SCHEMA_VERSION`] to load.
    pub schema_version: u32,
    /// The MINE-owned marker identifying this state as MINE-managed.
    pub managed_by: String,
    /// The installation records, one per Agent that has been installed.
    pub installs: Vec<AgentInstallRecord>,
}

impl ManagedState {
    /// Builds a fresh, empty managed-state document.
    #[must_use]
    pub fn new() -> Self {
        Self {
            schema_version: MANAGED_STATE_SCHEMA_VERSION,
            managed_by: "MINE".to_string(),
            installs: Vec::new(),
        }
    }

    /// Returns the managed-state file path under the given configuration root.
    #[must_use]
    pub fn path_for(root: &Path) -> PathBuf {
        root.join(MANAGED_STATE_FILE)
    }

    /// Loads and validates the managed state from `root`. Returns an empty
    /// document when no state exists yet (a clean first install). Foreign or
    /// malformed state is rejected with [`MineError::AgentManagedStateInvalid`].
    pub fn load(root: &Path) -> MineResult<Self> {
        let path = Self::path_for(root);
        if !path.exists() {
            return Ok(Self::new());
        }
        let raw = std::fs::read_to_string(&path).map_err(MineError::Io)?;
        Self::from_json_str(&raw).map_err(|e| MineError::AgentManagedStateInvalid {
            detail: format!("failed to parse managed state at {}: {e}", path.display()),
        })
    }

    /// Parses raw JSON and validates the schema marker/version.
    pub fn from_json_str(raw: &str) -> Result<Self, String> {
        let value: Value = serde_json::from_str(raw).map_err(|e| e.to_string())?;
        let obj = value
            .as_object()
            .ok_or("managed state root is not a JSON object")?;
        let managed_by = obj
            .get("managed_by")
            .and_then(|v| v.as_str())
            .ok_or("managed state missing managed_by")?;
        if managed_by != "MINE" {
            return Err(format!(
                "managed state managed_by={managed_by:?} is foreign (not MINE)"
            ));
        }
        let schema_version = obj
            .get("schema_version")
            .and_then(|v| v.as_u64())
            .ok_or("managed state missing schema_version")?;
        if schema_version != u64::from(MANAGED_STATE_SCHEMA_VERSION) {
            return Err(format!(
                "managed state schema_version {schema_version} is unsupported (expected {}); \
                 refusing to migrate foreign or incompatible state",
                MANAGED_STATE_SCHEMA_VERSION
            ));
        }
        let state: ManagedState = serde_json::from_value(value).map_err(|e| e.to_string())?;
        // Cross-validate each record: non-empty agent identifier, valid JSON
        // pointer strings (for config entries), no file with an empty path.
        for rec in &state.installs {
            if rec.agent.trim().is_empty() {
                return Err("an install record has an empty agent identifier".to_string());
            }
            if rec.destination.trim().is_empty() {
                return Err(format!(
                    "install record for agent {} has an empty destination",
                    rec.agent
                ));
            }
            for f in &rec.files {
                if f.path.trim().is_empty() {
                    return Err(format!(
                        "install record for agent {} has a file with an empty path",
                        rec.agent
                    ));
                }
            }
            for c in &rec.config_entries {
                if c.config_file.trim().is_empty() || c.json_pointer.trim().is_empty() {
                    return Err(format!(
                        "install record for agent {} has an empty config entry",
                        rec.agent
                    ));
                }
            }
        }
        Ok(state)
    }

    /// Serializes to a deterministic JSON string (sorted keys).
    pub fn to_json_string(&self) -> String {
        // serde_json with sorted keys for deterministic output.
        let mut value = serde_json::to_value(self).expect("ManagedState is serializable");
        sort_json_in_place(&mut value);
        serde_json::to_string(&value).expect("sorted Value serializes")
    }

    /// Atomically writes the managed state under `root` (stage + rename),
    /// creating parent directories as needed. The on-disk content is the
    /// deterministic JSON with sorted keys.
    pub fn save(&self, root: &Path) -> MineResult<()> {
        let path = Self::path_for(root);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(MineError::Io)?;
        }
        let bytes = self.to_json_string().into_bytes();
        crate::infrastructure::atomic_write::write(&path, &bytes)
    }

    /// Returns a mutable reference to the record for `agent`, replacing it if
    /// present (used by update). Returns `None` when no such record exists.
    pub fn record_mut(&mut self, agent: &str) -> Option<&mut AgentInstallRecord> {
        self.installs.iter_mut().find(|r| r.agent == agent)
    }

    /// Returns the record for `agent`, if installed.
    #[must_use]
    pub fn record(&self, agent: &str) -> Option<&AgentInstallRecord> {
        self.installs.iter().find(|r| r.agent == agent)
    }

    /// Replaces the record for `agent` (inserting if absent) with `record`.
    pub fn upsert(&mut self, record: AgentInstallRecord) {
        if let Some(slot) = self.installs.iter_mut().find(|r| r.agent == record.agent) {
            *slot = record;
        } else {
            self.installs.push(record);
        }
        self.installs.sort_by(|a, b| a.agent.cmp(&b.agent));
    }

    /// Removes the record for `agent`, if present (done only after the owned
    /// cleanup succeeds).
    pub fn remove(&mut self, agent: &str) -> Option<AgentInstallRecord> {
        let idx = self.installs.iter().position(|r| r.agent == agent)?;
        Some(self.installs.remove(idx))
    }
}

impl Default for ManagedState {
    fn default() -> Self {
        Self::new()
    }
}

/// Recursively sorts JSON object keys in place for deterministic output.
fn sort_json_in_place(value: &mut Value) {
    match value {
        Value::Object(map) => {
            // Collect into a sorted BTreeMap-equivalent and rebuild.
            let mut entries: Vec<(String, Value)> =
                map.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
            entries.sort_by(|a, b| a.0.cmp(&b.0));
            let mut sorted = Map::new();
            for (k, mut v) in entries {
                sort_json_in_place(&mut v);
                sorted.insert(k, v);
            }
            *map = sorted;
        }
        Value::Array(arr) => {
            for v in arr {
                sort_json_in_place(v);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_when_absent() {
        let tmp = tempfile::tempdir().unwrap();
        let st = ManagedState::load(tmp.path()).unwrap();
        assert!(st.installs.is_empty());
        assert_eq!(st.managed_by, "MINE");
        assert_eq!(st.schema_version, MANAGED_STATE_SCHEMA_VERSION);
    }

    #[test]
    fn round_trip_preserves_records() {
        let tmp = tempfile::tempdir().unwrap();
        let mut st = ManagedState::new();
        st.upsert(AgentInstallRecord {
            agent: "claude-code".into(),
            mine_version: "0.1.0".into(),
            source_identity: "embedded-payload-v1".into(),
            destination: "/home/u/.claude".into(),
            files: vec![OwnedFile {
                path: "skills/mine-arch/SKILL.md".into(),
                hash: "deadbeef".into(),
            }],
            config_entries: vec![OwnedConfigEntry {
                config_file: ".claude.json".into(),
                json_pointer: "/mcpServers/mine".into(),
                hash: "feedface".into(),
            }],
            installed_at: "2026-07-24T00:00:00Z".into(),
            previous_version: None,
        });
        st.save(tmp.path()).unwrap();
        let loaded = ManagedState::load(tmp.path()).unwrap();
        assert_eq!(st, loaded);
    }

    #[test]
    fn foreign_managed_by_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        let raw = r#"{"schema_version":1,"managed_by":"EVIL","installs":[]}"#;
        std::fs::create_dir_all(tmp.path().join(".mine")).unwrap();
        std::fs::write(ManagedState::path_for(tmp.path()), raw).unwrap();
        assert!(ManagedState::load(tmp.path()).is_err());
    }

    #[test]
    fn wrong_schema_version_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        let raw = r#"{"schema_version":99,"managed_by":"MINE","installs":[]}"#;
        std::fs::create_dir_all(tmp.path().join(".mine")).unwrap();
        std::fs::write(ManagedState::path_for(tmp.path()), raw).unwrap();
        assert!(ManagedState::load(tmp.path()).is_err());
    }

    #[test]
    fn malformed_json_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join(".mine")).unwrap();
        std::fs::write(ManagedState::path_for(tmp.path()), "{not json").unwrap();
        assert!(ManagedState::load(tmp.path()).is_err());
    }

    #[test]
    fn upsert_replaces_existing() {
        let mut st = ManagedState::new();
        let mk = |ver: &str| AgentInstallRecord {
            agent: "codex".into(),
            mine_version: ver.into(),
            source_identity: "p".into(),
            destination: "d".into(),
            files: vec![],
            config_entries: vec![],
            installed_at: "t".into(),
            previous_version: None,
        };
        st.upsert(mk("0.1.0"));
        st.upsert(mk("0.2.0"));
        assert_eq!(st.record("codex").unwrap().mine_version, "0.2.0");
        assert_eq!(st.installs.len(), 1);
    }

    #[test]
    fn deterministic_sorted_json() {
        let mut a = ManagedState::new();
        a.upsert(AgentInstallRecord {
            agent: "pi".into(),
            mine_version: "0.1.0".into(),
            source_identity: "p".into(),
            destination: "d".into(),
            files: vec![],
            config_entries: vec![],
            installed_at: "t".into(),
            previous_version: None,
        });
        a.upsert(AgentInstallRecord {
            agent: "codex".into(),
            mine_version: "0.1.0".into(),
            source_identity: "p".into(),
            destination: "d".into(),
            files: vec![],
            config_entries: vec![],
            installed_at: "t".into(),
            previous_version: None,
        });
        let s = a.to_json_string();
        // Records are sorted by agent name: codex before pi.
        let codex = s.find("\"codex\"").unwrap();
        let pi = s.find("\"pi\"").unwrap();
        assert!(codex < pi);
    }
}

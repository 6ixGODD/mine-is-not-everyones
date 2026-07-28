// Enforce `AGENTS.md`'s "Business code must not use `unsafe`" at compile time.
#![forbid(unsafe_code)]

//! Comment/ordering-preserving structured configuration editing for Agent MCP
//! registration — comment/ordering-preserving TOML editing via `toml_edit`.
//!
//! For Codex (TOML), editing uses `toml_edit` (a format-preserving editor) so
//! comments, whitespace, and unrelated formatting survive the merge — the
//! Destructive `toml::Value` reserialize round trips that destroy comments are
//! discarded. For Claude Code and OpenCode (JSON), a structured object merge
//! preserves unrelated keys (JSON has no comments).
//!
//! Each edit produces the new bytes plus the JSON/TOML pointer of the owned
//! entry and a hash of the owned entry value (for managed-state drift
//! evidence). Collisions with a foreign value under the same key are refused
//! (`MINE_AGENT_COLLISION`) rather than silently overwritten.

use std::path::Path;

use serde_json::{Map, Value};

use crate::agent_setup::safety::content_hash;
use crate::domain::error::{MineError, MineResult};

/// The owned MCP entry and where it lives, for managed-state recording.
#[derive(Debug, Clone)]
pub struct EditedEntry {
    /// JSON pointer (`/a/b`) of the owned entry within the config file.
    pub json_pointer: String,
    /// Hash of the serialized owned entry value.
    pub entry_hash: String,
    /// `true` if the on-disk content actually changed (vs already-correct).
    pub changed: bool,
}

/// The MCP entry value for `agent` in its native serialization (for hash +
/// managed state).
pub fn mcp_entry_value(agent: crate::agent_setup::targets::Agent) -> Value {
    match agent {
        crate::agent_setup::targets::Agent::ClaudeCode => serde_json::json!({
            "command": "mine",
            "args": ["mcp", "serve"]
        }),
        crate::agent_setup::targets::Agent::OpenCode => serde_json::json!({
            "type": "local",
            "command": ["mine", "mcp", "serve"],
            "enabled": true
        }),
        // For Codex the entry is TOML; this JSON form is used only for the
        // managed-state hash fingerprint (the actual on-disk edit is TOML).
        crate::agent_setup::targets::Agent::Codex => serde_json::json!({
            "command": "mine",
            "args": ["mcp", "serve"],
            "enabled": true
        }),
        crate::agent_setup::targets::Agent::Pi => Value::Null, // no MCP
    }
}

/// Edits the Claude Code / OpenCode JSON config in place, merging the MINE MCP
/// entry while preserving unrelated keys. Returns the owned entry descriptor.
pub fn edit_json_mcp(
    cfg_abs: &Path,
    agent: crate::agent_setup::targets::Agent,
    dry_run: bool,
) -> MineResult<EditedEntry> {
    let json_pointer = match agent {
        crate::agent_setup::targets::Agent::ClaudeCode => "/mcpServers/mine".to_string(),
        crate::agent_setup::targets::Agent::OpenCode => "/mcp/mine".to_string(),
        _ => unreachable!("edit_json_mcp is JSON-only"),
    };
    let entry = mcp_entry_value(agent);
    let entry_hash = content_hash(
        serde_json::to_vec_pretty(&entry)
            .unwrap_or_default()
            .as_slice(),
    );

    if !cfg_abs.exists() {
        // Fresh config: only the MINE entry.
        let mut root = Map::new();
        set_pointer(&mut root, &json_pointer, entry.clone());
        let doc = Value::Object(root);
        if !dry_run {
            if let Some(p) = cfg_abs.parent() {
                std::fs::create_dir_all(p).map_err(MineError::Io)?;
            }
            let bytes = serde_json::to_vec_pretty(&doc)
                .map_err(|e| MineError::Io(std::io::Error::other(e)))?;
            crate::infrastructure::atomic_write::write(cfg_abs, &bytes)?;
        }
        return Ok(EditedEntry {
            json_pointer,
            entry_hash,
            changed: true,
        });
    }

    let raw = std::fs::read_to_string(cfg_abs).map_err(MineError::Io)?;
    let mut doc: Value = serde_json::from_str(&raw).map_err(|e| MineError::AgentCollision {
        target: cfg_abs.to_path_buf(),
        detail: format!("existing JSON config is not valid: {e}"),
    })?;
    if !doc.is_object() {
        return Err(MineError::AgentCollision {
            target: cfg_abs.to_path_buf(),
            detail: "existing JSON config root is not an object".to_string(),
        });
    }
    let existing = pointer_get(&doc, &json_pointer);
    let changed = match existing {
        None => {
            let obj = doc.as_object_mut().unwrap();
            set_pointer(obj, &json_pointer, entry.clone());
            true
        }
        Some(v) if v == &entry => false, // already the standard MINE entry.
        Some(v) => {
            return Err(MineError::AgentCollision {
                target: cfg_abs.to_path_buf(),
                detail: format!("an existing non-MINE entry occupies {json_pointer}: {v}"),
            });
        }
    };
    if changed && !dry_run {
        let bytes =
            serde_json::to_vec_pretty(&doc).map_err(|e| MineError::Io(std::io::Error::other(e)))?;
        crate::infrastructure::atomic_write::write(cfg_abs, &bytes)?;
    }
    Ok(EditedEntry {
        json_pointer,
        entry_hash,
        changed,
    })
}

/// Edits the Codex TOML config using `toml_edit` so comments, whitespace, and
/// unrelated formatting are preserved. Inserts/overwrites the
/// `[mcp_servers.mine]` table.
pub fn edit_toml_mcp(cfg_abs: &Path, dry_run: bool) -> MineResult<EditedEntry> {
    use toml_edit::DocumentMut;
    let json_pointer = "/mcp_servers/mine".to_string();
    // The standard MINE entry as a toml_edit table.
    let mut mine_tbl = toml_edit::table();
    mine_tbl["command"] = toml_edit::value("mine");
    mine_tbl["args"] = toml_edit::value(toml_edit::Array::from_iter(["mcp", "serve"]));
    mine_tbl["enabled"] = toml_edit::value(true);

    // Hash the entry value (JSON form, for a stable fingerprint).
    let entry_hash = content_hash(
        serde_json::to_vec_pretty(&mcp_entry_value(crate::agent_setup::targets::Agent::Codex))
            .unwrap_or_default()
            .as_slice(),
    );

    if !cfg_abs.exists() {
        let mut doc = DocumentMut::new();
        doc["mcp_servers"] = toml_edit::table();
        doc["mcp_servers"]["mine"] = mine_tbl;
        if !dry_run {
            if let Some(p) = cfg_abs.parent() {
                std::fs::create_dir_all(p).map_err(MineError::Io)?;
            }
            crate::infrastructure::atomic_write::write(cfg_abs, doc.to_string().as_bytes())?;
        }
        return Ok(EditedEntry {
            json_pointer,
            entry_hash,
            changed: true,
        });
    }

    let raw = std::fs::read_to_string(cfg_abs).map_err(MineError::Io)?;
    let before = raw.clone();
    let mut doc = raw
        .parse::<DocumentMut>()
        .map_err(|e| MineError::AgentCollision {
            target: cfg_abs.to_path_buf(),
            detail: format!("existing Codex config.toml is not valid TOML: {e}"),
        })?;
    // Ensure the mcp_servers table exists.
    if !doc.contains_key("mcp_servers") {
        doc["mcp_servers"] = toml_edit::table();
    }
    // Check presence immutably: toml_edit's `Item::get_mut` auto-inserts an
    // `Item::None` placeholder for absent keys (returns `Some(Item::None)`
    // rather than `None`), so `get_mut` is NOT a reliable presence check.
    // `contains_key` on the table is.
    let mine_present = doc["mcp_servers"]
        .as_table()
        .is_some_and(|t| t.contains_key("mine"));
    let changed = if !mine_present {
        doc["mcp_servers"]["mine"] = mine_tbl;
        true
    } else if !doc["mcp_servers"]["mine"].is_table() {
        return Err(MineError::AgentCollision {
            target: cfg_abs.to_path_buf(),
            detail: "`mcp_servers.mine` is not a table".to_string(),
        });
    } else {
        // Existing table under `mine`: refuse a foreign value, then ensure
        // the standard keys are present/correct (format-preserving).
        for k in ["command", "enabled"] {
            if let Some(toml_edit::Item::Value(val)) = doc["mcp_servers"]["mine"].get(k) {
                let mismatch = match k {
                    "command" => val.as_str() != Some("mine"),
                    "enabled" => val.as_bool() != Some(true),
                    _ => false,
                };
                if mismatch {
                    return Err(MineError::AgentCollision {
                        target: cfg_abs.to_path_buf(),
                        detail: format!(
                            "an existing non-MINE `[mcp_servers.mine]` `{k}` occupies the table in config.toml"
                        ),
                    });
                }
            }
        }
        let existing = doc["mcp_servers"]["mine"].as_table_mut().unwrap();
        existing["command"] = toml_edit::value("mine");
        if existing.get("args").is_none() {
            existing["args"] = toml_edit::value(toml_edit::Array::from_iter(["mcp", "serve"]));
        }
        existing["enabled"] = toml_edit::value(true);
        doc.to_string() != before
    };
    if changed && !dry_run {
        crate::infrastructure::atomic_write::write(cfg_abs, doc.to_string().as_bytes())?;
    }
    Ok(EditedEntry {
        json_pointer,
        entry_hash,
        changed,
    })
}

/// Sets a nested JSON pointer path into `obj`, creating intermediate objects.
fn set_pointer(obj: &mut Map<String, Value>, pointer: &str, value: Value) {
    let parts: Vec<&str> = pointer.trim_start_matches('/').split('/').collect();
    let mut cur = obj;
    for (i, part) in parts.iter().enumerate() {
        if i + 1 == parts.len() {
            cur.insert((*part).to_string(), value.clone());
        } else {
            let next = cur
                .entry((*part).to_string())
                .or_insert_with(|| Value::Object(Map::new()));
            cur = match next {
                Value::Object(m) => m,
                _ => {
                    let m = Map::new();
                    *next = Value::Object(m.clone());
                    next.as_object_mut().unwrap()
                }
            };
        }
    }
}

/// Gets the value at a JSON pointer path, or `None`.
fn pointer_get<'a>(doc: &'a Value, pointer: &str) -> Option<&'a Value> {
    let parts: Vec<&str> = pointer.trim_start_matches('/').split('/').collect();
    let mut cur = doc;
    for part in parts {
        cur = cur.get(part)?;
    }
    Some(cur)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_setup::targets::Agent;

    #[test]
    fn codex_toml_preserves_comments_and_unrelated() {
        let tmp = tempfile::tempdir().unwrap();
        let cfg = tmp.path().join(".codex/config.toml");
        std::fs::create_dir_all(cfg.parent().unwrap()).unwrap();
        let original =
            "# user comment\n[mcp_servers]\n# another\nexisting = true\n[to_keep]\nx = 1\n";
        std::fs::write(&cfg, original).unwrap();
        edit_toml_mcp(&cfg, false).unwrap();
        let after = std::fs::read_to_string(&cfg).unwrap();
        assert!(after.contains("# user comment"), "comment preserved");
        assert!(after.contains("[to_keep]"), "unrelated table preserved");
        assert!(after.contains("existing = true"), "unrelated key preserved");
        assert!(after.contains("[mcp_servers.mine]"), "MINE table added");
        assert!(after.contains("command = \"mine\""));
        assert!(after.contains("enabled = true"));
    }

    #[test]
    fn json_merge_preserves_unrelated_keys() {
        let tmp = tempfile::tempdir().unwrap();
        let cfg = tmp.path().join(".claude.json");
        std::fs::write(
            &cfg,
            r#"{"user-theme":"dark","mcpServers":{"other":{"command":"keepme"}}}"#,
        )
        .unwrap();
        edit_json_mcp(&cfg, Agent::ClaudeCode, false).unwrap();
        let after = serde_json::from_str::<Value>(&std::fs::read_to_string(&cfg).unwrap()).unwrap();
        assert_eq!(after["user-theme"], "dark");
        assert_eq!(after["mcpServers"]["other"]["command"], "keepme");
        assert_eq!(after["mcpServers"]["mine"]["command"], "mine");
    }

    #[test]
    fn toml_refuses_foreign_mine_entry() {
        let tmp = tempfile::tempdir().unwrap();
        let cfg = tmp.path().join(".codex/config.toml");
        std::fs::create_dir_all(cfg.parent().unwrap()).unwrap();
        std::fs::write(&cfg, "[mcp_servers.mine]\ncommand = \"wrong\"\n").unwrap();
        let err = edit_toml_mcp(&cfg, false).unwrap_err();
        assert_eq!(err.code(), "MINE_AGENT_COLLISION");
    }
}

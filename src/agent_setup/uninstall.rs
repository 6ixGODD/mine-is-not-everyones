// Enforce `AGENTS.md`'s "Business code must not use `unsafe`" at compile time.
#![forbid(unsafe_code)]

//! Uninstallation — removes ONLY resources proven MINE-owned through valid
//! managed state AND current on-disk verification. Ported (validated) from the
//! rejected Plan 07 and reworked for the isolated [`Env`].
//!
//! Safety: preserve unrelated user files/config; preserve drifted/uncertain
//! content (report, do not delete); never recursively delete; reject path
//! traversal/symlink/junction escape via the `SafetyGuard` chokepoint; handle
//! missing/partially-removed installations deterministically; remove the
//! managed state record only after owned cleanup reaches the approved
//! terminal result. No `--force`/arbitrary deletion.

use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::agent_setup::managed_state::{ManagedState, OwnedConfigEntry};
use crate::agent_setup::safety::{SafetyGuard, content_hash};
use crate::agent_setup::targets::{Agent, Env};
use crate::agent_setup::transaction;
use crate::domain::error::{MineError, MineResult};

#[derive(Debug, Clone, serde::Serialize)]
pub struct UninstallOutcome {
    pub agent: String,
    pub removed_files: usize,
    pub removed_config_entries: usize,
    pub drifted_files: Vec<String>,
    pub missing_files: Vec<String>,
    pub previous_version: Option<String>,
}

pub fn uninstall(agent: Agent, env: &Env, dry_run: bool) -> MineResult<UninstallOutcome> {
    let guard = SafetyGuard::new(&env.config_root);

    // If an incomplete transaction is pending, recover it first so uninstall
    // operates on a coherent state (the pending install rolled back).
    if !dry_run {
        transaction::detect_and_recover(agent.slug(), &env.config_root, &guard)?;
    }

    let mut state = ManagedState::load(&env.config_root)?;
    let record = state
        .record(agent.slug())
        .ok_or_else(|| MineError::AgentManagedStateInvalid {
            detail: format!("no managed installation record for agent {}", agent.slug()),
        })?
        .clone();

    let mut removed_files = 0usize;
    let mut removed_config_entries = 0usize;
    let mut drifted_files: Vec<String> = Vec::new();
    let mut missing_files: Vec<String> = Vec::new();

    for f in &record.files {
        let abs = guard.ensure_within_root(&env.config_root.join(&f.path))?;
        if !abs.exists() {
            missing_files.push(f.path.clone());
            removed_files += 1;
            continue;
        }
        let cur = std::fs::read(&abs).map_err(MineError::Io)?;
        if content_hash(&cur) != f.hash {
            drifted_files.push(f.path.clone());
            continue;
        }
        if !dry_run {
            std::fs::remove_file(&abs).map_err(MineError::Io)?;
        }
        removed_files += 1;
    }

    for c in &record.config_entries {
        let cfg_abs = guard.ensure_within_root(&env.config_root.join(&c.config_file))?;
        if dry_run {
            removed_config_entries += 1;
            continue;
        }
        if !cfg_abs.exists() {
            removed_config_entries += 1;
            continue;
        }
        let removed_entry = remove_config_entry(&cfg_abs, c)?;
        if removed_entry {
            removed_config_entries += 1;
        } else {
            drifted_files.push(format!("{}:{}", c.config_file, c.json_pointer));
        }
    }

    if !dry_run {
        cleanup_empty_dirs(&record, env, &guard)?;
    }

    if !dry_run {
        state.remove(agent.slug());
        state.save(&env.config_root)?;
    }

    Ok(UninstallOutcome {
        agent: agent.slug().to_string(),
        removed_files,
        removed_config_entries,
        drifted_files,
        missing_files,
        previous_version: record.previous_version,
    })
}

fn remove_config_entry(cfg_abs: &Path, c: &OwnedConfigEntry) -> MineResult<bool> {
    let raw = std::fs::read_to_string(cfg_abs).map_err(MineError::Io)?;
    if c.config_file.ends_with(".toml") {
        remove_toml_entry(cfg_abs, &raw, c)
    } else {
        remove_json_entry(cfg_abs, &raw, c)
    }
}

fn remove_json_entry(cfg_abs: &Path, raw: &str, c: &OwnedConfigEntry) -> MineResult<bool> {
    let entry_hash = content_hash(c.json_pointer.as_bytes());
    let _ = entry_hash;
    let mut doc: Value =
        serde_json::from_str(raw).map_err(|e| MineError::AgentManagedStateInvalid {
            detail: format!("config {} is not valid JSON: {e}", cfg_abs.display()),
        })?;
    let parts: Vec<&str> = c.json_pointer.trim_start_matches('/').split('/').collect();
    let (parent, leaf) = match parts.split_last() {
        Some((leaf, parent)) => (parent, *leaf),
        None => return Ok(false),
    };
    let mut cur = &mut doc;
    for p in parent {
        cur = cur
            .get_mut(*p)
            .ok_or_else(|| MineError::AgentManagedStateInvalid {
                detail: format!(
                    "config {} missing pointer parent {}",
                    cfg_abs.display(),
                    c.json_pointer
                ),
            })?;
    }
    let obj = cur
        .as_object_mut()
        .ok_or_else(|| MineError::AgentManagedStateInvalid {
            detail: format!(
                "config {} pointer parent is not an object",
                cfg_abs.display()
            ),
        })?;
    let entry = match obj.get(leaf) {
        Some(v) => v,
        None => return Ok(false),
    };
    let cur_hash = content_hash(
        serde_json::to_vec_pretty(entry)
            .unwrap_or_default()
            .as_slice(),
    );
    if cur_hash != c.hash {
        return Ok(false); // drifted; preserve.
    }
    obj.remove(leaf);
    let bytes =
        serde_json::to_vec_pretty(&doc).map_err(|e| MineError::Io(std::io::Error::other(e)))?;
    crate::infrastructure::atomic_write::write(cfg_abs, &bytes)?;
    Ok(true)
}

fn remove_toml_entry(cfg_abs: &Path, raw: &str, c: &OwnedConfigEntry) -> MineResult<bool> {
    use toml_edit::DocumentMut;
    let mut doc = raw
        .parse::<DocumentMut>()
        .map_err(|e| MineError::AgentManagedStateInvalid {
            detail: format!("Codex config {} is not valid TOML: {e}", cfg_abs.display()),
        })?;
    // The Codex MCP json_pointer is always `/mcp_servers/mine`; navigate the
    // two-level path through the Item tree.
    let parts: Vec<&str> = c.json_pointer.trim_start_matches('/').split('/').collect();
    let (parent, leaf) = match parts.split_last() {
        Some((leaf, parent)) => (parent, *leaf),
        None => return Ok(false),
    };
    // Navigate the parent chain via a single mutable Item handle.
    let root: &mut toml_edit::Item = doc.as_item_mut();
    let mut cur = root;
    for p in parent {
        cur = cur
            .get_mut(*p)
            .ok_or_else(|| MineError::AgentManagedStateInvalid {
                detail: format!(
                    "TOML config {} missing pointer parent {}",
                    cfg_abs.display(),
                    c.json_pointer
                ),
            })?;
    }
    let tbl = cur
        .as_table_mut()
        .ok_or_else(|| MineError::AgentManagedStateInvalid {
            detail: format!(
                "TOML config {} pointer parent is not a table",
                cfg_abs.display()
            ),
        })?;
    let entry_present = tbl.contains_key(leaf);
    if !entry_present {
        return Ok(false);
    }
    // Drift check: re-derive the JSON-form hash from the live `[mcp_servers.mine]`
    // table and compare to the recorded hash. The recorded hash is over the
    // canonical JSON entry; if they differ, the user edited the table → preserve.
    let cur_json = match tbl.get(leaf) {
        Some(toml_edit::Item::Table(t)) => {
            let mut m = serde_json::Map::new();
            for (k, v) in t.iter() {
                if let Some(val) = v.as_str() {
                    m.insert(k.into(), Value::String(val.to_string()));
                } else if let Some(b) = v.as_bool() {
                    m.insert(k.into(), Value::Bool(b));
                }
            }
            content_hash(
                serde_json::to_vec_pretty(&Value::Object(m))
                    .unwrap_or_default()
                    .as_slice(),
            )
        }
        _ => return Ok(false),
    };
    if cur_json != c.hash {
        return Ok(false); // drifted; preserve.
    }
    tbl.remove(leaf);
    let out = doc.to_string();
    crate::infrastructure::atomic_write::write(cfg_abs, out.as_bytes())?;
    Ok(true)
}

fn cleanup_empty_dirs(
    record: &crate::agent_setup::managed_state::AgentInstallRecord,
    env: &Env,
    guard: &SafetyGuard,
) -> MineResult<()> {
    let skills_root = PathBuf::from(&record.destination);
    for f in &record.files {
        let mut dir = PathBuf::from(&f.path);
        while let Some(parent) = dir.parent() {
            let abs = env.config_root.join(parent);
            let abs = guard.ensure_within_root(&abs)?;
            if !abs.starts_with(&skills_root) || abs == skills_root {
                break;
            }
            if abs.is_dir()
                && std::fs::read_dir(&abs)
                    .map_err(MineError::Io)?
                    .next()
                    .is_none()
            {
                let _ = std::fs::remove_dir(&abs);
            } else {
                break;
            }
            dir = parent.to_path_buf();
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_setup::install::{FailPhase, install};

    fn env(tmp: &tempfile::TempDir) -> Env {
        Env::isolated(tmp.path().to_path_buf())
    }

    #[test]
    fn clean_uninstall_removes_owned() {
        let tmp = tempfile::tempdir().unwrap();
        install(
            Agent::ClaudeCode,
            &env(&tmp),
            "0.1.0",
            false,
            FailPhase::None,
        )
        .unwrap();
        let out = uninstall(Agent::ClaudeCode, &env(&tmp), false).unwrap();
        assert!(out.removed_files >= 10);
        assert!(
            !tmp.path()
                .join(".claude/skills/mine-arch/SKILL.md")
                .exists()
        );
        let st = ManagedState::load(tmp.path()).unwrap();
        assert!(st.record("claude-code").is_none());
    }

    #[test]
    fn uninstall_preserves_unrelated_mcp() {
        let tmp = tempfile::tempdir().unwrap();
        let cfg = tmp.path().join(".claude.json");
        std::fs::write(&cfg, r#"{"mcpServers":{"other":{"command":"foo"}}}"#).unwrap();
        install(
            Agent::ClaudeCode,
            &env(&tmp),
            "0.1.0",
            false,
            FailPhase::None,
        )
        .unwrap();
        uninstall(Agent::ClaudeCode, &env(&tmp), false).unwrap();
        let after = serde_json::from_str::<Value>(&std::fs::read_to_string(&cfg).unwrap()).unwrap();
        assert_eq!(after["mcpServers"]["other"]["command"], "foo");
        assert!(
            after
                .get("mcpServers")
                .and_then(|m| m.get("mine"))
                .is_none()
        );
    }

    #[test]
    fn uninstall_preserves_drifted() {
        let tmp = tempfile::tempdir().unwrap();
        install(Agent::Codex, &env(&tmp), "0.1.0", false, FailPhase::None).unwrap();
        let f = tmp.path().join(".agents/skills/mine-arch/SKILL.md");
        std::fs::write(&f, "USER EDITED").unwrap();
        let out = uninstall(Agent::Codex, &env(&tmp), false).unwrap();
        assert!(
            out.drifted_files
                .iter()
                .any(|p| p.contains("mine-arch/SKILL.md"))
        );
        assert!(f.exists());
    }

    #[test]
    fn uninstall_refuses_without_managed_record() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join(".pi/agent/skills/mine-sync")).unwrap();
        std::fs::write(
            tmp.path().join(".pi/agent/skills/mine-sync/SKILL.md"),
            "fake",
        )
        .unwrap();
        let err = uninstall(Agent::Pi, &env(&tmp), false).unwrap_err();
        assert_eq!(err.code(), "MINE_AGENT_MANAGED_STATE_INVALID");
        assert!(
            tmp.path()
                .join(".pi/agent/skills/mine-sync/SKILL.md")
                .exists()
        );
    }

    #[test]
    fn dry_run_uninstall_removes_nothing() {
        let tmp = tempfile::tempdir().unwrap();
        install(Agent::Pi, &env(&tmp), "0.1.0", false, FailPhase::None).unwrap();
        let out = uninstall(Agent::Pi, &env(&tmp), true).unwrap();
        assert!(out.removed_files >= 10);
        assert!(
            tmp.path()
                .join(".pi/agent/skills/mine-sync/SKILL.md")
                .exists()
        );
    }
}

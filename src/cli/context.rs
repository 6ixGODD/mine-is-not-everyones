//! Shared CLI context and helpers.
//!
//! Resolves the repository root, loads `.mine/config.toml`, and constructs a
//! [`TomlStore`] for graph access. These helpers are used by every command
//! handler so the dispatch layer stays thin and the contracts (repository
//! root, config, graph store) are resolved consistently.

use std::path::{Path, PathBuf};

use crate::domain::config::MineConfig;
use crate::domain::error::{MineError, MineResult};
use crate::infrastructure::git::GitEvidence;
use crate::infrastructure::toml_store::TomlStore;

/// The output format selected by the global `--format` flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputFormat {
    /// Concise human-readable text (default).
    #[default]
    Human,
    /// Stable JSON envelope.
    Json,
}

/// Parsed global CLI options shared by all command handlers.
#[derive(Debug, Clone)]
pub struct GlobalOpts {
    pub format: OutputFormat,
    pub quiet: bool,
    /// Explicit repository root override (`--repo <path>`). When `None`, the
    /// repository root is discovered from the current directory.
    pub repo: Option<PathBuf>,
    /// `--no-color` is accepted for interface compatibility but is a no-op:
    /// the default output is already plain text.
    pub no_color: bool,
}

impl GlobalOpts {
    #[must_use]
    pub fn is_json(&self) -> bool {
        matches!(self.format, OutputFormat::Json)
    }
}

/// A resolved command context: repository root, configuration, and graph
/// store. Handlers use this to access MINE state consistently.
pub struct CommandContext {
    pub repo_root: PathBuf,
    pub config: Option<MineConfig>,
    pub store: TomlStore,
}

impl CommandContext {
    /// Loads the configuration or returns an error if absent/invalid.
    ///
    /// # Errors
    /// - [`MineError::RepositoryNotFound`] if no `.mine/config.toml` exists.
    /// - [`MineError::ConfigInvalid`] if the configuration cannot be parsed.
    pub fn require_config(&self) -> MineResult<&MineConfig> {
        self.config
            .as_ref()
            .ok_or_else(|| MineError::RepositoryNotFound {
                detail: format!(
                    "no .mine/config.toml at {} (run `mine init`)",
                    self.repo_root.join(".mine").join("config.toml").display()
                ),
            })
    }
}

/// Resolves the repository root for a command.
///
/// If `--repo` was supplied, it is used (canonicalized). Otherwise the
/// repository root is discovered by walking up from the current directory
/// looking for `.mine/config.toml`, then falling back to a `.git`-rooted
/// tree. Returns [`MineError::RepositoryNotFound`] when no repository can be
/// located.
pub fn resolve_repo_root(override_root: Option<&Path>) -> MineResult<PathBuf> {
    if let Some(explicit) = override_root {
        let canonical = explicit
            .canonicalize()
            .map_err(|e| MineError::RepositoryNotFound {
                detail: format!(
                    "--repo {} could not be canonicalized: {e}",
                    explicit.display()
                ),
            })?;
        return Ok(canonical);
    }
    if let Ok(cwd) = std::env::current_dir() {
        let mut current: &Path = &cwd;
        loop {
            if current.join(".mine").join("config.toml").exists() {
                return Ok(current.to_path_buf());
            }
            if current.join(".git").exists() {
                // A git repo without MINE config: use it as the root and let
                // the command layer decide whether initialization is required.
                return Ok(current.to_path_buf());
            }
            match current.parent() {
                Some(parent) => current = parent,
                None => break,
            }
        }
    }
    Err(MineError::RepositoryNotFound {
        detail: "no .mine/config.toml or .git found walking up from cwd".to_string(),
    })
}

/// Builds a [`CommandContext`] for a command, loading `.mine/config.toml` if
/// present (commands that require initialization will fail later via
/// [`CommandContext::require_config`]).
pub fn build_context(global: &GlobalOpts) -> MineResult<CommandContext> {
    let repo_root = resolve_repo_root(global.repo.as_deref())?;
    let config = load_config(&repo_root);
    let store = TomlStore::new(&repo_root);
    Ok(CommandContext {
        repo_root,
        config,
        store,
    })
}

/// Loads `.mine/config.toml` if present, returning `None` when absent and
/// propagating parse failures as `Some(MineError)`. Because [`CommandContext`]
/// stores `Option<MineConfig>`, an invalid config is turned into an error
/// eagerly by [`CommandContext::require_config`] callers; here an unparseable
/// config is surfaced as a `ConfigInvalid` error wrapped in `Some` only when
/// the caller asks. To stay simple, this returns `None` for absent and `Ok(cfg)`
/// for valid; invalid configs are reported by calling
/// [`load_config_or_error`].
pub fn load_config(repo_root: &Path) -> Option<MineConfig> {
    let path = repo_root.join(".mine").join("config.toml");
    let content = std::fs::read_to_string(&path).ok()?;
    MineConfig::parse(&path, &content).ok()
}

/// Loads the config or returns a typed error when absent/invalid.
///
/// # Errors
/// - [`MineError::RepositoryNotFound`] when the config is absent.
/// - [`MineError::ConfigInvalid`] when the config is unparseable.
pub fn load_config_or_error(repo_root: &Path) -> MineResult<MineConfig> {
    let path = repo_root.join(".mine").join("config.toml");
    if !path.exists() {
        return Err(MineError::RepositoryNotFound {
            detail: format!(
                "no .mine/config.toml at {} (run `mine init`)",
                path.display()
            ),
        });
    }
    let content = std::fs::read_to_string(&path)?;
    MineConfig::parse(&path, &content)
}

/// Collects light Git evidence for JSON output. Never mutates Git.
pub fn git_evidence(repo_root: &Path) -> GitEvidence {
    GitEvidence::collect(repo_root)
}

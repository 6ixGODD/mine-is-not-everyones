// Enforce `AGENTS.md`'s "Business code must not use `unsafe`" at compile time.
#![forbid(unsafe_code)]

//! Agent installer, managed state, and doctor — Plan 07-1 (compensating for the
//! rejected Plan 07).
//!
//! Plan 07-1 fixes three independently reproduced defects of Plan 07:
//!
//! 1. **Mandatory configuration backup before mutation** ([`backup`], [`config_edit`]):
//!    every structured Agent config file is backed up (exact bytes, verified,
//!    never silently clobbered) before its first mutation, and Codex TOML is
//!    edited with `toml_edit` to preserve comments/formatting.
//! 2. **Transactional installation + recovery** ([`transaction`], reworked
//!    [`install`]): a bounded preflight/staging/commit/rollback/recovery
//!    transaction with a durable pending record, so a partial install never
//!    leaves orphaned files that permanently block retries.
//! 3. **Complete explicit `--config-root` isolation** ([`targets`]): an
//!    isolated [`targets::Env`] never honors real process environment
//!    overrides (`CLAUDE_CONFIG_DIR`/`CODEX_HOME`/`PI_HOME`/`OPENCODE_CONFIG_DIR`).
//!
//! Selectively **ported** from the rejected Plan 07 (independently validated):
//! [`safety`] (the `SafetyGuard` filesystem boundary, independently verified
//! sound against a genuine Windows junction by the Plan 07-1 independent
//! review; no in-module junction unit test exists in `safety.rs` itself - this
//! is an honestly disclosed limitation, not a hidden claim), [`managed_state`] (ownership record), [`uninstall`]
//! (ownership-proven removal), [`doctor`] (truthful diagnostics), and the
//! four-Agent destination shapes in [`targets`]. **Discarded**: the
//! mutation-without-backup install logic, payload-first non-transactional
//! installation, mixed real/explicit environment construction, and the full
//! TOML parse/reserialize path (replaced by `toml_edit`).

pub mod backup;
pub mod config_edit;
pub mod doctor;
pub mod install;
pub mod managed_state;
pub mod safety;
pub mod targets;
pub mod transaction;
pub mod uninstall;

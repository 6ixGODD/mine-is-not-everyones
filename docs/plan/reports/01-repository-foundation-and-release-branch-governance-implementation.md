# Plan 01 Implementation Report

- **Plan**: `docs/plan/01-repository-foundation-and-release-branch-governance.md`
- **Title**: Repository foundation, initialization, namespace, and branch governance
- **Execution date**: 2026-07-24
- **Conclusion**: `IMPLEMENTED` — pending independent reviewer acceptance. The implementing agent did not self-grant `ACCEPTED` and did not edit execution-graph state.

## Branches and commits

| Item | Value |
|---|---|
| Original branch (before bootstrap) | none — the repository was not a git repository; `git init` was required |
| Stable branch | `master` (`1d3a132f8bbffc6ffca60d6bea5b6f36a6a3de36`) |
| Integration branch | `dev` (created from the stable baseline, not implemented on) |
| Plan branch | `plan/01-repository-foundation-and-release-branch-governance` |
| Baseline commit | `1d3a132f8bbffc6ffca60d6bea5b6f36a6a3de36` on `master` |
| Implementation commits | `f1edf811d2786249341d577f51472faf1d37ba14`, `8a55e3ae916d66ca89302131789ef1076c7fe3b2`, `7c14f399ca42229667e339a4fa7f06e7d26e52cd`, `7da99efcca151636ab88b9e1a6bdc9fb775456df` |

The repository was not under version control at the start of bootstrap. `git init -b master` was used to create the stable branch, the pre-existing working tree (minus ignored caches) was committed as the stable baseline, and `dev` and the plan branch were created from that baseline. Nothing was merged into `dev` and nothing was pushed.

### Implementation commits

1. `f1edf81` `feat(init): Rust crate foundation, init service, and design-namespace validation` — `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `src/**`, `tests/init_service.rs`.
2. `8a55e3a` `feat(governance): MINE repository config, AGENTS.md governance, and line-ending policy` — `.mine/config.toml`, `.mine/.gitignore`, `AGENTS.md`, `.gitattributes`.
3. `7c14f39` `feat(skills): mine-sync skeleton and mine-arch no-auto-execution section` — `skills/mine-sync/SKILL.md`, `skills/mine-arch/SKILL.md`.
4. `7da99ef` `ci: add foundational Rust quality-gate workflow` — `.github/workflows/ci.yml`.

## Changed files (23 files, +2177 lines vs baseline)

```
.gitattributes
.github/workflows/ci.yml
.mine/.gitignore
.mine/config.toml
AGENTS.md
Cargo.lock
Cargo.toml
rust-toolchain.toml
skills/mine-arch/SKILL.md          (modified: added No automatic execution section)
skills/mine-sync/SKILL.md          (new)
src/application/init_service.rs
src/application/mod.rs
src/domain/config.rs
src/domain/design_marker.rs
src/domain/error.rs
src/domain/mod.rs
src/domain/ports.rs
src/domain/repository_identity.rs
src/infrastructure/mod.rs
src/infrastructure/system.rs
src/lib.rs
src/main.rs
tests/init_service.rs
```

Pre-existing files preserved unchanged: `REQUIREMENTS.md`, all of `docs/design/**` (including `docs/design/.mine-design.toml` with its existing `repository_id` `9c672a03-bab3-566e-9588-39cb5e6445c3` and `created_at` `2026-07-23T00:00:00Z`), `docs/plan/**` (including `execution-graph.toml`/`.md`), the other three root Skills, `scripts/`, `plugins/`, `package.json`, `.claude-plugin/`, `.agents/`, `LICENSE`, `README.md`, `README.zh-CN.md`.

## Work-package evidence

### WP1 — Baseline and research

Read completely before mutation: `REQUIREMENTS.md`, `docs/design/index.md`, all design documents referenced by Plan 01 (`principles.md`, `system/code-organization.md`, `governance/design-knowledge-base.md`, `governance/branch-and-plan-lifecycle.md`, `decisions/0006-mine-owns-design-namespace.md`), the CLI contract, the configuration/operations design, the design-sync governance, the skills contract, `skills/mine-plan-exec/SKILL.md`, the existing `README.md`, and the existing `skills/mine-arch/SKILL.md`. `AGENTS.md` was absent and is created by this plan. The Rust toolchain was missing and was installed (see Toolchain below).

### WP2 — Repository identity

Implemented in `src/domain/repository_identity.rs`. `RepositoryIdentity::resolve` preserves existing managed values with this priority: design-marker `repository_id` > existing `.mine/config.toml` `repository_id` > freshly generated UUID v4; and for version: existing config `mine_code_version` > reliable root-version evidence (`Cargo.toml` `[package].version`) > `0.1.0`. The persistence target is `.mine/config.toml` (`repository_id`, `mine_code_version`), matching `docs/design/operations/configuration-security-observability.md`. The existing managed UUID `9c672a03-bab3-566e-9588-39cb5e6445c3` is preserved; the version is `0.1.0` (this repository's `Cargo.toml` version).

### WP3 — Initialization service

Implemented in `src/application/init_service.rs` as a setup-only use case over injected `UuidSource`/`Clock` ports. It: discovers the repository root; initializes or validates `.mine/config.toml`; creates a repository UUID when unmanaged; creates the `docs/design/` scaffold and `.mine-design.toml` when absent; creates the AGENTS.md MINE section without erasing unrelated content; initializes the version from MINE state, root evidence, or `0.1.0`; and performs no source scan, architecture generation, plan creation, agent invocation, business-code change, branch mutation, commit, merge, or release. Configuration is preserved byte-for-byte when already valid (idempotent; never rewritten).

### WP4 — Namespace conflict

Implemented in `src/domain/design_marker.rs::classify`. A design directory without a marker fails with `MINE_DESIGN_NAMESPACE_CONFLICT`; a foreign (non-`MINE`) marker is also rejected as a namespace conflict; a valid `MINE` marker belonging to another recorded repository fails with `MINE_DESIGN_OWNERSHIP_MISMATCH`; a malformed or wrong-schema marker fails with `MINE_DESIGN_MARKER_INVALID`. No legacy migration is implemented.

### WP5 — Branch governance

Written in `AGENTS.md`: source-of-truth paths, design rules, no-historical-baggage policy, plan immutability and execution discipline, parallel-execution ownership, the Rust quality-gate matrix, evidence/report rules, commit discipline, MINE graph discipline, and the bounded standing managed-branch authorization with its exclusions (stable `master`, integration `dev`, `plan/*`).

### WP6 — Skill skeletons

Created `skills/mine-sync/SKILL.md` with the correct high-level responsibilities (refuse legacy design; verified backup before rewrite; scoped or broad exploration; code-first authority order; preserve only user-protected decisions; record uncertainty; validate and report) and an explicit No automatic execution section. Added a matching No automatic execution section to `skills/mine-arch/SKILL.md`. Procedural detail and final JSON-CLI/MCP contract integration are deferred to Plan 04.

### WP7 — Tests and report

16 domain unit tests (`src/domain/design_marker.rs`, `src/domain/repository_identity.rs`) and 10 init-service integration tests (`tests/init_service.rs`) covering absent, valid, legacy, foreign, ownership-mismatch, malformed, idempotent, AGENTS.md section handling, and root-version-evidence cases. This report is the WP7 deliverable.

## Verification

### Available checks (all pass)

| Command | Exit | Result |
|---|---|---|
| `cargo fmt --all -- --check` | 0 | clean, no diff |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | no warnings |
| `cargo test --all-targets --all-features` | 0 | 16 unit + 10 integration = 26 passed, 0 failed |
| End-to-end init on this repository (temporary example, since removed) | 0 | marker preserved; `.mine/config.toml` + `.mine/.gitignore` created; AGENTS.md preserved |
| Idempotent re-run of init on this repository | 0 | all actions `Preserved`; md5sum of config/marker/AGENTS.md/.gitignore identical before and after |

Test output (final run):

```
running 16 tests ... test result: ok. 16 passed; 0 failed
running 10 tests ... test result: ok. 10 passed; 0 failed
```

### Unavailable bootstrap checks

| Command | Reason |
|---|---|
| `mine design validate` | The `mine` CLI command dispatcher (`src/cli/`) is Plan 03; the binary does not yet dispatch subcommands. Plan 01 delivers the init service as a library and verifies it via `cargo test` and a temporary example. The marker/namespace validation logic that `mine design validate` would invoke is implemented and unit-tested, but the command is not wired and was not pretended to run. |
| `mine graph validate` | The execution-graph domain is Plan 02; the `mine` CLI is Plan 03. Not implemented in Plan 01 and not pretended to run. |

Per the bootstrap exception, the `mine` CLI, MCP server, and execution-graph state commands do not exist yet. The implementation agent did not implement later-Plan commands early, did not pretend unavailable commands were executed, and did not directly change Plan status to `ACCEPTED`.

## Toolchain

The Rust toolchain was entirely missing at bootstrap (no `rustup`, `rustc`, `cargo`, `rustfmt`, or `clippy`). A pre-existing `rustup` settings file existed at `C:\Users\BC\.rustup\settings.toml`, so `rustup` had been installed previously but was not on the shell `PATH`.

Installation command (user-level, official stable MSVC toolchain):

```bash
curl -fsSL -o rustup-init.exe https://win.rustup.rs/x86_64
./rustup-init.exe -y --default-toolchain stable --profile default --default-host x86_64-pc-windows-msvc
```

Resulting versions:

```
rustup 1.29.0
rustc 1.97.1 (8bab26f4f 2026-07-14)
cargo 1.97.1 (c980f4866 2026-06-30)
rustfmt 1.9.0-stable (8bab26f4f6 2026-07-14)
clippy 0.1.97 (8bab26f4f6 2026-07-14)
toolchain: stable-x86_64-pc-windows-msvc (active, default)
components: cargo, clippy, rust-docs, rust-std, rustc, rustfmt
```

Windows build prerequisites were already present (not installed by this plan): Visual Studio Community 2022 and Build Tools 2022 with MSVC VC Tools (`x86.x64`) and Windows SDK `10.0.22621.0` (`kernel32.lib` found). The MSVC `link.exe` is not on `PATH` in a plain shell, and `D:\Git\usr\bin\link.exe` (Git's coreutils hard-link tool, not the MSVC linker) is on `PATH`; however, `rustc 1.97` auto-discovers the VS 2022 MSVC linker, so a plain `cargo build`/`clippy`/`test` links successfully without a `vcvars64` wrapper. The official MSVC toolchain was used; the toolchain was not switched to GNU to hide any platform failure.

## Deviations and local decisions

- **Git initialization.** The repository was not under version control. `git init -b master` created the stable branch; the pre-existing working tree was committed as the baseline. This was forced by the standing authorization to create `dev` and the plan branch from a stable baseline, and is documented here for the reviewer.
- **`.gitignore` and `.gitattributes`.** Added as baseline/repository hygiene so the pre-existing `.mypy_cache/` cache and Rust `/target/` build output do not enter version control, and so `cargo fmt`/clippy/CI are line-ending-deterministic across platforms. Neither file is owned by another plan's exclusive write paths; both are standard repository foundation (`mine-arch` Phase 9 lists `.gitignore`/`.gitattributes`).
- **`docs/plan/` on the stable baseline.** The baseline commit includes `docs/plan/` (the execution graph and plans). The design's "stable branch must not contain `docs/plan/`" invariant is enforced at release closure (Plan 08), not during bootstrap; the plan documents and execution graph are the ephemeral bootstrap workflow and are explicitly stated not to survive stable release integration.
- **CLI not wired.** Plan 03 owns `src/cli/`. Plan 01 delivers the init service as a library with tests; `src/main.rs` is an honest placeholder that runs no subcommand. `mine init` as a runnable subcommand is Plan 03.
- **`mine init` stable-branch detection.** Plan 01 records the MINE-managed default (`master`) and preserves any stable branch already in configuration. Real Git-based stable-branch discovery is wired with the Git infrastructure in Plan 03.
- **Layering shortcut.** The init service performs focused `std::fs` I/O directly. Plan 02 introduces `atomic_write`/`file_lock`/`toml_store` infrastructure for the execution-graph paths and refactors this service to use them.
- **Five-skills vs four-skills tension.** ADR-0004, the design index, the skills contract, and Plan 01's own exclusive write paths define five first-class Skills including `mine-sync`. `REQUIREMENTS.md` section 2.1 lists four. This plan follows the design and Plan 01 scope (which explicitly own `skills/mine-sync/`) and creates the `mine-sync` skeleton. The discrepancy is noted for the reviewer; it is a pre-existing document inconsistency, not a decision this plan introduced.
- **`mine-arch` architecture-path inconsistency.** The existing `skills/mine-arch/SKILL.md` references `docs/design/architecture-and-detailed-design.md` as the fixed architecture output, while the current design uses the progressive knowledge base rooted at `docs/design/index.md`. `AGENTS.md` follows the authoritative design (`docs/design/index.md`). Full reconciliation of the `mine-arch` skill with the progressive-disclosure design is left to Plan 04 / a `mine-arch` run; this plan only added the required No automatic execution section.

## Acceptance criteria mapping

| Criterion | Evidence |
|---|---|
| `mine init` is idempotent and setup-only | `init_is_idempotent_when_managed` test; end-to-end re-run on this repository produced identical md5sums and all-`Preserved` actions |
| legacy `docs/design/` is rejected rather than adopted | `legacy_unmarked_design_dir_is_rejected_without_mutation` test; `MINE_DESIGN_NAMESPACE_CONFLICT` |
| MINE marker and repository identity are validated | `foreign_marker_is_rejected`, `ownership_mismatch_is_rejected`, `malformed_marker_is_rejected`, `invalid_existing_config_is_rejected` tests; `classify` + `RepositoryIdentity::resolve` |
| no branch, plan, source scan, agent run, commit, or business-code mutation occurs during init | init service touches only `.mine/config.toml`, `.mine/.gitignore`, `docs/design/.mine-design.toml` (only when absent), `docs/design/index.md` (only when absent), and `AGENTS.md`; `absent_design_root_creates_scaffold_marker_and_config` asserts `docs/plan/` is not created |
| branch authorization is explicit and bounded | `AGENTS.md` Branch governance section |
| Plan reaches `IMPLEMENTED`, never self-granted `ACCEPTED` | this report concludes `IMPLEMENTED`; execution-graph state was not edited |

## Remaining risks and external actions

- The independent bootstrap reviewer must transition the Plan 01 node to `ACCEPTED` (or `REJECTED`) through the `mine` CLI/MCP once Plan 03 wires it, and record `stable_baseline_commit` and `implementation_commits` in `docs/plan/execution-graph.toml`. This implementation agent did not edit the execution graph.
- `mine design validate` and `mine graph validate` remain unavailable until Plans 02 and 03.
- The five-skills vs four-skills document inconsistency should be resolved (REQUIREMENTS vs design) before Plan 04 finalizes the skill contract.
- The `mine-arch` skill's stale `architecture-and-detailed-design.md` path should be reconciled with the `docs/design/index.md` progressive knowledge base in Plan 04.

## Working-tree state and unrelated changes

The working tree is clean on `plan/01-repository-foundation-and-release-branch-governance` after the implementation commits; only this report remains to be committed. The pre-existing `.mypy_cache/` directory is preserved on disk (ignored via `.gitignore`, never deleted). No unrelated pre-existing modifications were discarded, reset, stashed, or cleaned. Nothing was merged into `dev` and nothing was pushed.

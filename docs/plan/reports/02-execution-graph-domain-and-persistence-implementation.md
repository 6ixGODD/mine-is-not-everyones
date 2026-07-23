# Plan 02 Implementation Report

- **Plan**: `docs/plan/02-execution-graph-domain-and-persistence.md`
- **Title**: Execution graph domain and persistence
- **Execution date**: 2026-07-24
- **Conclusion**: `IMPLEMENTED` — pending independent reviewer acceptance. The implementing agent did not self-grant `ACCEPTED`; the Plan 02 node was transitioned only to `IMPLEMENTED` via the documented bootstrap exception (the `mine` lifecycle CLI/MCP does not exist until Plan 03).

## Branches and commits

| Item | Value |
|---|---|
| Stable branch | `master` (`1d3a132f8bbffc6ffca60d6bea5b6f36a6a3de36`, unchanged) |
| Integration branch | `dev` (`93f3660f6d63851a357f1541ddf4c44399ef2840` at start; this plan does not merge into it) |
| Plan branch | `plan/02-execution-graph-domain-and-persistence` |
| Start point | clean `dev` at `93f3660` (Plan 01 accepted and merged, Plan 02 `READY`) |
| Predecessor gate | Plan 01 `ACCEPTED` with 4 implementation commits and a review report on `dev` |
| Bootstrap-start commit | `a3c7494` `chore(graph): bootstrap start Plan 02 (READY -> IN_PROGRESS)` — revision `1`→`2`, Plan 02 `READY`→`IN_PROGRESS`, owner/run_id/started_at/updated_at filled, markdown synchronized |
| Implementation commits | `f4cc22937ebdf6e2cfd4e0eb6fc1c8dbc6f54806`, `b58ad4c1710019519a5e1ecc5c9637a33f3b1ed4`, `61668619bca5b52587bedcc26153e04082869b2b` |
| Completion-bookkeeping commit | (recorded below after this report is committed) |

Nothing was merged into `dev`, nothing was pushed, `master` was not touched, and no `plan/03*` branch was created.

### Implementation commits

1. `f4cc229` `feat(graph-domain): execution-graph aggregate, states, path safety, validation` — `src/domain/{status,path,design_reference,graph,validation}.rs` (new), `src/domain/error.rs` + `src/domain/mod.rs` (extended).
2. `b58ad4c` `feat(graph-persistence): atomic writes, file locking, TOML store` — `src/infrastructure/{atomic_write,file_lock,toml_store}.rs` (new), `src/infrastructure/mod.rs` (module wiring).
3. `6166861` `test(graph): execution-graph domain and persistence integration tests` — `tests/domain.rs`, `tests/persistence.rs`.

## Changed files (13 implementation files vs `dev` start point; +2607 / -15)

```
src/domain/design_reference.rs          (new, 106 lines)
src/domain/error.rs                     (modified: +64 graph/transition/scope/revision/lock/evidence error variants + codes)
src/domain/graph.rs                     (new, 289 lines)
src/domain/mod.rs                       (modified: module wiring + doc)
src/domain/path.rs                      (new, 199 lines)
src/domain/status.rs                    (new, 175 lines)
src/domain/validation.rs                (new, 453 lines)
src/infrastructure/atomic_write.rs      (new, 147 lines)
src/infrastructure/file_lock.rs         (new, 320 lines)
src/infrastructure/mod.rs               (modified: module wiring + doc)
src/infrastructure/toml_store.rs        (new, 378 lines)
tests/domain.rs                         (new, 204 lines)
tests/persistence.rs                    (new, 247 lines)
```

The bootstrap-start commit (`a3c7494`) additionally touched the two reserved shared paths (`docs/plan/execution-graph.toml`, `docs/plan/execution-graph.md`) with the `READY`→`IN_PROGRESS` transition; those are graph-bookkeeping, not implementation, and are reserved shared by this plan's own contract.

### Files explicitly not touched

Plan 01 / Plan 03 owned files preserved unchanged: `src/application/init_service.rs`, `src/application/mod.rs`, `src/domain/{config,design_marker,ports,repository_identity}.rs`, `src/infrastructure/system.rs`, `src/lib.rs`, `src/main.rs`, `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `.mine/**`, `AGENTS.md`, `.gitattributes`, `.github/**`, `skills/**`, `tests/init_service.rs`, all of `docs/design/**`, all other plan documents and reports.

## Work-package evidence

### WP1 — Baseline and evidence

Read completely before mutation: the Plan 02 document; its three governing design references (`docs/design/execution-graph/domain-model.md`, `persistence-and-concurrency.md`, `state-machine-and-algorithms.md`); the predecessor (Plan 01) implementation and review reports; `docs/plan/execution-graph.toml` / `.md`; and the existing Plan 01 domain code (`error.rs`, `config.rs`, `ports.rs`, `init_service.rs`) for style and reuse. Confirmed the repository was on clean `dev` at `93f3660` with Plan 01 `ACCEPTED` and Plan 02 `READY`.

### WP2 — Contract extraction

Mapped the three design references to an implementation checklist:
- `domain-model.md` → `PlanWorkspace` aggregate, `PlanNode` entity, `DesignReference`, path-safety classifier, TOML schema matching the live fact source.
- `persistence-and-concurrency.md` → `atomic_write`, `file_lock`, `toml_store` with the `lock → reload → recheck revision → atomic write → render markdown` sequence.
- `state-machine-and-algorithms.md` → 7-state `PlanStatus` enum with explicit transition rules (no generic `set-status`), validation (unique IDs/paths, predecessor validity, acyclicity, legal states, safe paths, evidence, revision parity), and parallel-wave computation (stable maximal `READY` set excluding ancestor relationships and write-scope overlap).

### WP3 — Implementation

**Domain layer (`src/domain/`):**
- `status.rs`: `PlanStatus` enum (`DRAFT`/`BLOCKED`/`READY`/`IN_PROGRESS`/`IMPLEMENTED`/`ACCEPTED`/`REJECTED`) with `SCREAMING_SNAKE_CASE` serde rename, string round-trip, and `validate_transition` enforcing only the allowed edges from the design's transition table. No generic `set-status`.
- `path.rs`: pure repository-relative path safety classifier. Rejects absolute (leading separator), empty, `..` traversal, backslash separators, Windows drive roots, and broad wildcard/glob characters; normalizes redundant separators and trailing slashes. Symlink/junction escape is rejected at the I/O boundary (caller-supplied existence flags), keeping the module pure.
- `design_reference.rs`: models both the structured target form `{ path, anchors[], reason }` from `domain-model.md` and the flat-string-array legacy form currently used by the live fact source, with `from_flat_paths` construction so the persistence layer round-trips the existing graph byte-for-byte.
- `graph.rs`: `PlanWorkspace` aggregate root and `PlanNode` entity. TOML schema (`Serialize`/`Deserialize`) matches `docs/plan/execution-graph.toml` byte-for-byte (flat arrays for `design_references`, `exclusive_write_paths`, `read_only_paths`, `reserved_shared_paths`; `implementation_commits` as a string array). Provides aggregate helpers (`get`, `ids`), ancestor-relationship traversal, and `write_scope_overlaps` (exclusive vs exclusive, and exclusive vs reserved-shared) used by the parallel wave.
- `validation.rs`: `validate` (structural integrity: unique IDs, unique normalized plan paths, valid hard/soft predecessors, no self-dependency, non-empty design references, safe owned paths, acyclic hard deps via Kahn topological sort, generated-view revision parity), `topological_sort`, `hard_predecessors_accepted`, `is_derived_ready`, `ready_frontier`, `parallel_wave` (greedy stable maximal `READY` set excluding ancestor pairs and write-scope overlaps), and `validate_revision_parity`.
- `error.rs`: extended `MineError` with `GraphNotInitialized`, `GraphInvalid`, `GraphCycle`, `PlanNotFound`, `InvalidTransition`, `PredecessorNotAccepted`, `WriteScopeConflict`, `RevisionConflict`, `LockTimeout`, `EvidenceMissing`, each with a stable `MINE_*` code (part of the future public JSON error contract).

**Persistence infrastructure (`src/infrastructure/`):**
- `atomic_write.rs`: writes to an unpredictably-named temp file in the same directory (counter + pid), flushes and syncs, then renames over the target (POSIX-atomic; Windows replaces on rename). Never truncates the fact source in place.
- `file_lock.rs`: cross-platform exclusive advisory lock on `.mine/locks/<name>.lock` with a bounded retry/timeout. POSIX `fcntl(F_SETLK)` and Windows `LockFileEx` (exclusive) are conditionally compiled; the lock is released on `Drop` (file handle close). Unpredictable content is written to the lock file.
- `toml_store.rs`: `TomlStore` over a repo root. `load` parses the TOML aggregate (with `GraphNotInitialized` when absent). `save_with_revision` performs the design's write sequence: acquire the exclusive lock, reload the on-disk TOML (state may have changed while waiting), recheck `expected_revision` (`RevisionConflict` on mismatch), apply the mutation callback, atomic-write the TOML, and render the Markdown view. `render_markdown` is deterministic and includes the revision for parity; `repair_markdown` re-renders a stale view; `validate_revision_parity` cross-checks the generated view against the TOML.

### WP4 — Focused tests

**`tests/domain.rs` (9 tests):** full lifecycle transitions allowed; `IMPLEMENTED→REJECTED→BLOCKED (compensation)` path and rejection of `REJECTED→READY` short-circuit; duplicate plan id and plan path rejected; self-dependency rejected; missing predecessor rejected; cycle `MINE_GRAPH_CYCLE`; diamond dependency validates and topologizes in declaration order; parallel wave picks a disjoint `READY` set and excludes ancestor pairs; parallel wave excludes write-scope overlap (via reserved-shared paths); empty design references rejected; unsafe owned path (`../escape/`) rejected.

**`tests/persistence.rs` (9 tests):** `load` returns `GraphNotInitialized` for an absent graph; `save_with_revision` round-trips and renders markdown with revision parity; `save_with_revision` with a stale `expected_revision` returns `RevisionConflict` and does not overwrite; concurrent writers do not silently overwrite (rev-conflict); atomic write recovers from missing markdown; lock is acquired and released; Markdown render is deterministic and idempotent; `repair_markdown` fixes a stale view; and the real `docs/plan/execution-graph.toml` round-trips byte-for-byte (load → serialize → compare bytes) **and validates**, proving the serialization model matches the live fact source including the legitimate sequential scope overlap (Plan 01 `src/` vs Plan 02 `src/domain/`).

### WP5 — Integration checks

Ran the repository quality gates (see Verification). The real-graph round-trip test is the cross-cutting integration check: it loads the actual `docs/plan/execution-graph.toml`, re-serializes it, and asserts byte equality, while also running `validation::validate` on the loaded workspace.

### WP6 — Skill/design consistency

No public CLI command, MCP tool, or Skill instruction was made stale. The execution-graph domain and persistence are library-only in this plan; the CLI dispatcher that would expose `mine graph validate` / `mine plan ...` is Plan 03. The `mine` binary (`src/main.rs`) remains an honest placeholder that runs no subcommand, so no unavailable command is pretended to run. The `AGENTS.md` graph-discipline section already documents the bootstrap exception under which the manual graph transition was performed.

## Verification

### Available checks (all pass)

| Command | Exit | Result |
|---|---|---|
| `cargo fmt --all -- --check` | 0 | clean, no diff |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | no warnings |
| `cargo build --all-targets --all-features` | 0 | builds clean (rustc 1.97 auto-discovers the VS 2022 MSVC linker) |
| `cargo test --all-targets --all-features` | 0 | 63 lib unit + 9 domain + 10 init_service + 9 persistence = **91 passed, 0 failed** |

Final test output:

```
unittests src/lib.rs:           63 passed; 0 failed
unittests src/main.rs:           0 passed; 0 failed
tests/domain.rs:                 9 passed; 0 failed
tests/init_service.rs:          10 passed; 0 failed   (Plan 01 suite, still green)
tests/persistence.rs:           9 passed; 0 failed
```

### Unavailable bootstrap checks

| Command | Reason |
|---|---|
| `mine design validate` | The `mine` CLI dispatcher (`src/cli/`) is Plan 03; the binary does not dispatch subcommands. The underlying marker/namespace validation was delivered and tested in Plan 01; not re-wired here and not pretended to run. |
| `mine graph validate` | The `mine` CLI dispatcher is Plan 03; the `mine graph validate` command is not wired. The underlying graph validation (`validation::validate`) and revision-parity check are implemented, unit- and integration-tested (incl. against the real graph), and are the logic the future command will invoke. The command itself was not pretended to run. |

Per the documented bootstrap exception, the `mine` CLI, MCP server, and lifecycle state commands do not exist yet. The implementing agent did not implement later-Plan commands early, did not pretend unavailable commands were executed, and transitioned Plan 02 only to `IMPLEMENTED` (never `ACCEPTED`) via manual bookkeeping.

## Deviations and local decisions

- **Bootstrap-only graph transition.** The Plan 02 node was manually transitioned `READY`→`IN_PROGRESS` (start) and `IN_PROGRESS`→`IMPLEMENTED` (completion) directly in `docs/plan/execution-graph.toml` + `.md`, because the `mine` lifecycle CLI/MCP (Plan 03) is not yet implemented. This is the documented bootstrap exception in `AGENTS.md` (MINE graph discipline) and the Plan 02 reserved-shared-path contract. The completion transition increments the revision, records the three implementation commits and the report path, and leaves Plan 03 `BLOCKED` (only `ACCEPTED` releases a successor). This manual procedure must not be repeated once Plan 03 wires `mine plan implemented`.
- **`src/infrastructure/mod.rs` wiring edit.** Plan 02's declared exclusive write paths list specific `.rs` files under `src/infrastructure/` (`toml_store.rs`, `file_lock.rs`, `atomic_write.rs`) but not the parent `mod.rs`. Declaring the three new `pub mod` entries in `src/infrastructure/mod.rs` is structurally necessary (otherwise the modules do not compile), is purely additive wiring with a doc update, and touches no logic owned by another plan. It is the same kind of necessary structural-wiring deviation that Plan 01 recorded for `.gitattributes`; flagged here for the reviewer.
- **Write-scope conflict is a `parallel_wave` constraint, not a `validate` constraint.** The design's *Validation* paragraph lists unique IDs/paths, valid predecessors, acyclicity, legal states, evidence, safe paths, revision parity, and branch/workspace consistency — it does **not** include exclusive-write overlap. The *Parallel wave* paragraph states the wave is "without ... exclusive-write overlap". Accordingly, `validate` does **not** reject overlapping exclusive-write scopes globally; `parallel_wave` excludes overlapping scopes among `READY` plans. This is required for the live graph to validate: Plan 01 (owning broad `src/`) is `ACCEPTED` and Plan 02 (owning `src/domain/`) is its sequential successor — they never write concurrently, so the overlap is not a structural defect. A prior local draft of `validate` over-aggressively flagged this and was corrected to match the design; the real-graph round-trip test now passes.
- **`DesignReference` structured form vs flat-string fact source.** `domain-model.md` specifies the structured form `{ path, anchors[], reason }`, but the live `docs/plan/execution-graph.toml` stores `design_references` as a flat string array (e.g. `["docs/design/principles.md", ...]`). To avoid drifting the immutable fact source, `DesignReference` models both forms and the `PlanNode` TOML schema serializes the flat-array form, so the existing graph round-trips byte-for-byte. The structured form is the design target; migrating the fact source to it is a design update plus a Plan 03 (graph writers) concern, not this plan's scope, and is flagged below.
- **No `src/application/` changes.** Plan 02 owns `src/application/` exclusively, but this plan did not need to add or modify any application service: the execution-graph use-case services (workspace, graph state transitions through the store) are wired in Plan 03 on top of the domain and persistence delivered here. The owned path remains available; leaving it untouched is within scope.
- **Cross-platform lock code is conditionally compiled.** `file_lock.rs` ships POSIX (`fcntl`) and Windows (`LockFileEx`) backends behind `#[cfg]`. The Windows path was built and tested on this host; the POSIX path compiles only on Unix targets (it is excluded from the Windows build). The `extern` blocks use `unsafe extern "C"`/`"system"` as required by Rust 2024.

## Acceptance criteria mapping

| Criterion | Evidence |
|---|---|
| every governing design contract is implemented or explicitly reported as blocked | domain model, persistence/concurrency, and state-machine/algorithms implemented (see WP3); `mine design validate` / `mine graph validate` commands blocked on Plan 03 (see Unavailable bootstrap checks) |
| all writes stay within declared ownership | 13 implementation files are all under `src/domain/`, `src/infrastructure/{toml_store,file_lock,atomic_write}.rs`, `tests/domain.rs`, `tests/persistence.rs`; plus the necessary `src/infrastructure/mod.rs` wiring (flagged above); the reserved-shared graph files were touched only for the documented bootstrap transition |
| tests discriminate intended semantics from plausible wrong behavior | 18 new integration tests + 63 lib unit tests assert allowed vs rejected transitions, safe vs unsafe paths, valid vs cyclic/missing/self predecessors, disjoint vs overlapping waves, byte-exact vs drifted serialization, revision-match vs conflict, deterministic vs stale render |
| stable JSON/protocol contracts are documented where applicable | `MineError::code()` returns stable `MINE_*` strings for all new variants; TOML `schema_version = 1` is preserved; the status serde form is `SCREAMING_SNAKE_CASE` matching the live fact source |
| no direct execution-graph file editing is introduced | the production persistence path is `TomlStore::save_with_revision` (lock → reload → revision check → atomic write → render); the CLI/Skills must use it. The only direct graph edits are bootstrap bookkeeping, explicitly permitted by the documented exception |
| no unrelated changes or secrets are staged | only the 13 in-scope implementation files + bootstrap graph bookkeeping were staged; no secrets present |
| implementation evidence is reproducible | exact commands and exit codes recorded above; `cargo test` re-runnable; the real-graph round-trip test fixes the fact source as the byte-compat benchmark |
| the node reaches `IMPLEMENTED`, never self-granted `ACCEPTED` | this report concludes `IMPLEMENTED`; the completion bookkeeping transition sets `IMPLEMENTED` and leaves `ACCEPTED` to the independent reviewer |

## Remaining risks and external actions

- The independent bootstrap reviewer must review and transition Plan 02 to `ACCEPTED` (or `REJECTED`), then release Plan 03 to `READY`, and merge the plan branch into `dev`. This implementing agent did not merge, push, or touch `master`.
- `mine design validate` and `mine graph validate` remain unavailable as commands until Plan 03 wires the CLI dispatcher; the underlying logic is delivered here.
- The `DesignReference` structured-form vs flat-string-fact-source tension should be resolved before Plan 03/04 finalize graph writers and design-sync: either update the design to keep the flat form, or migrate the fact source to the structured form via a Plan 03 graph-writer migration. Until then, the domain round-trips the live graph byte-for-byte.
- The POSIX `file_lock` backend is cfg-gated and was not built on this Windows host; it should be exercised on a Linux/macOS CI runner once the CI workflow executes on multiple OSes (the Plan 01 CI workflow currently targets Ubuntu; the lock test there will validate the POSIX path).

## Toolchain

Unchanged from Plan 01: `rustc 1.97.1`, `cargo 1.97.1`, `rustfmt 1.9.0`, `clippy 0.1.97`, `stable-x86_64-pc-windows-msvc`. VS 2022 MSVC tools + Windows SDK `10.0.22621.0` present; `rustc` auto-discovers the MSVC linker, so no `vcvars64` wrapper is needed. The official MSVC toolchain was used; the toolchain was not switched to GNU to hide any platform failure.

## Working-tree state and unrelated changes

The working tree is clean on `plan/02-execution-graph-domain-and-persistence` after the three implementation commits; this report is the only remaining file before the completion bookkeeping. The pre-existing `.mypy_cache/` directory remains on disk (gitignored, never deleted). No unrelated pre-existing modifications were discarded, reset, stashed, or cleaned. Nothing was merged into `dev`, nothing was pushed, `master` was not touched, and no `plan/03*` branch was created.
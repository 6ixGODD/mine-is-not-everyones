# Plan 02-1 Implementation Report

- **Plan**: `docs/plan/02-1-execution-graph-safe-file-locking.md`
- **Title**: Execution graph safe file locking (compensation for rejected Plan 02)
- **Execution date**: 2026-07-24
- **Conclusion**: `IMPLEMENTED` — pending independent reviewer acceptance. The implementing agent did not self-grant `ACCEPTED`; the Plan 02-1 node was transitioned only to `IMPLEMENTED` via the documented bootstrap exception (the `mine` lifecycle CLI/MCP does not exist until Plan 03).

## What this plan remediates

Independent review rejected Plan 02 (`docs/plan/reports/02-execution-graph-domain-and-persistence-review.md`) for exactly one hard contract failure: `src/infrastructure/file_lock.rs` used hand-written `unsafe extern "C"`/`unsafe extern "system"` FFI (`fcntl`/`LockFileEx`) and `unsafe { ... }` calls inside `mine`'s own crate, violating `AGENTS.md`'s unconditional "Business code must not use `unsafe`". The review verified the rest of Plan 02 (domain, validation, TOML/atomic-write persistence, tests — 91/91) as sound and directed a compensating plan that ports that work forward rather than reimplementing it. This plan does exactly that.

## Branches and commits

| Item | Value |
|---|---|
| Stable branch | `master` (`1d3a132f8bbffc6ffca60d6bea5b6f36a6a3de36`, unchanged) |
| Integration branch | `dev` (`e783b29d819d7e996990f33b02e63d4d383809a8` at branch creation; this plan does not merge into it) |
| Rejected Plan 02 branch (preserved, source of ported code) | `plan/02-execution-graph-domain-and-persistence` at `1e007ede3361b3b137cab02943735dfe5853cd76` (unchanged, not deleted) |
| This plan branch | `plan/02-1-execution-graph-safe-file-locking` (created from the rejected branch HEAD `1e007ed`, not from `dev`) |
| Reconcile commit | `2df28a3613d0b0c3b0251a115a823cc65069d665` — bring this branch's context (execution graph, Plan 02-1 doc, corrected design, Plan 03 rerouting, Plan 02 rejection report) forward from `dev`'s authoritative post-rejection state |
| Start-bookkeeping commit | `6b58dc41b2d8a83b7f683b7a6309d1a155f15a72` — Plan 02-1 `READY`→`IN_PROGRESS`, revision `2`→`3`, ownership/timestamps, markdown synchronized |
| Implementation commits | `61adf3f84ba921eebaf5f2a50ce0d9f7456348a7`, `2f4332227e253f0c1336c6b5eaf2286c1604674f` |

Nothing was merged into `dev`, nothing was pushed, `master` was not touched, the rejected Plan 02 branch was not modified or deleted, and no `plan/03*` branch was created.

### Why the branch starts from the rejected branch and is reconciled

The instruction required the new branch to start from the preserved rejected Plan 02 branch HEAD (`1e007ed`), carrying forward the sound domain/persistence/tests *code*, and explicitly "not from `dev`". However, the authoritative post-rejection *context* — the execution graph (Plan 02 `REJECTED` with `compensating_plan = "02-1"`; Plan 02-1 `READY`; Plan 03 `hard_predecessors` rerouted `02`→`02-1`), the Plan 02-1 document, the corrected `persistence-and-concurrency.md` "Revision and locking" section, the Plan 03 predecessor note, and the Plan 02 rejection review — lives only on `dev` (commit `e783b29`), made by the independent reviewer. The rejected branch HEAD predates that and carried a stale graph (revision 3, "Plan 02 IMPLEMENTED", no 02-1 node). So after creating the branch from `1e007ed`, the first commit (`2df28a3`) reconciled the reserved-shared graph and the read-only context docs to `dev`'s authoritative state via `git checkout e783b29 -- <files>`. This is context reconciliation (no production code touched, no new graph *transition* invented — it copies the reviewer's already-done work), recorded as a distinct commit for an clean audit trail. The Plan 02-1 start transition then followed as its own bookkeeping commit (`6b58dc4`).

### Implementation commits

1. `61adf3f` `feat(file-lock): replace unsafe FFI with fs4 safe locking crate` — `Cargo.toml`, `Cargo.lock`, `src/infrastructure/file_lock.rs` (rewrite), `tests/domain.rs`, `tests/persistence.rs`.
2. `2f43322` `feat(crate-policy): forbid unsafe_code at mine crate roots` — `src/lib.rs`, `src/main.rs` (`#![forbid(unsafe_code)]`).

## Changed files (7 implementation files vs start commit `6b58dc4`; +103 / −186)

```
Cargo.lock                         (fs4 + transitive deps added)
Cargo.toml                         (fs4 = "1.1.0", sync feature)
src/infrastructure/file_lock.rs    (rewrite: removed all unsafe FFI; fs4 backend)
src/lib.rs                         (#![forbid(unsafe_code)] + rationale comment)
src/main.rs                        (#![forbid(unsafe_code)] + rationale comment)
tests/domain.rs                    (#![forbid(unsafe_code)])
tests/persistence.rs               (#![forbid(unsafe_code)] + real-graph assertions updated for the 02-1 compensation node)
```

### Files explicitly NOT touched (ported forward unchanged from the rejected branch)

The whole execution-graph domain and persistence layer from the rejected Plan 02 is carried forward verbatim because the `file_lock` public contract was preserved: `src/domain/{status,path,design_reference,graph,validation,error,config,design_marker,ports,repository_identity,mod}.rs`, `src/application/init_service.rs`, `src/infrastructure/{atomic_write,toml_store,system,mod}.rs`, `tests/init_service.rs`. `atomic_write.rs` (`retry_io`) and `toml_store.rs` (`acquire_exclusive`) consume `file_lock` unchanged. This is the "preserve and port forward rather than reimplement" requirement satisfied.

## Work-package evidence

### WP1 — Baseline and evidence

Read before mutation: the Plan 02-1 document; its three governing design references including the corrected `persistence-and-concurrency.md` "Revision and locking" section; the Plan 02 rejection review report; the authoritative `dev` execution graph (`e783b29`); and the rejected branch's `src/infrastructure/file_lock.rs` (the defective unsafe FFI) plus its consumers (`toml_store.rs`, `atomic_write.rs`). Confirmed the predecessor gate: Plan 01 `ACCEPTED`, Plan 02-1 `READY` (hard predecessor `01`).

### WP2 — Dependency selection

Selected **`fs4` 1.1.0** (default `sync` feature). Rationale (per the corrected design, which names `fs4` as the maintained successor to `fs2`):
- **Maintained & widely used**: actively supported, ~55M downloads, pure-Rust (uses `rustix` rather than raw `libc` on POSIX).
- **Safe API**: exposes `fs4::FileExt` (sealed trait implemented for `std::fs::File`) with `try_lock() -> Result<(), fs4::TryLockError>`, `lock()`, `unlock()`, etc. All platform `unsafe` (POSIX `flock` via `rustix`; Windows `LockFileEx`) lives inside `fs4`/`rustix`, not in `mine`'s crate.
- **MSRV**: 1.75.0 (sync feature) ≤ our `rust-version = "1.85"` and toolchain (stable 1.97). Verified against the published crate source in the local cargo registry.
- **Semantics**: whole-file advisory exclusive lock; held until `unlock()` or the file handle closes (release-on-drop), matching the required contract.
- Added via `cargo add fs4@1.1.0` — the only manifest change in scope. `Cargo.toml`/`Cargo.lock` are in this plan's exclusive write paths solely for this one dependency.

The std library (Rust 1.89+) also stabilized `File::try_lock`/`unlock`, but the corrected design explicitly mandates a maintained **external** locking crate; `fs4` is used as directed. std 1.89+'s inherent `File::try_lock`/`unlock` methods shadow fs4's trait methods, so the code calls `fs4::FileExt::try_lock(&file)` / `fs4::FileExt::unlock(&self.file)` fully-qualified to disambiguate (both back onto the same OS primitive; fs4 is the vetted backend standardized on).

### WP3 — Locking reimplementation

`src/infrastructure/file_lock.rs` rewritten (239 lines changed, net −186):
- **Removed**: the `unsafe extern "C"` `fcntl` block, the `unsafe extern "system"` `LockFileEx` block, both `unsafe { ... }` call sites, the `LockHandle`/`LockHandle` enum, the `libc_flock`/`Overlapped` `#[repr(C)]` structs, all `cfg(unix)`/`cfg(windows)` FFI constants and helpers. `grep -rnE "unsafe[[:space:]]*(\{|extern|fn|impl)" src/` now returns zero code matches (the single grep hit is inside a `//!` doc comment).
- **Preserved public contract**: `pub fn acquire_exclusive(lock_path: &Path, timeout: Duration) -> MineResult<FileLock>`, `pub struct FileLock` with `pub fn path(&self) -> &Path`, `pub(crate) fn retry_io<F,T>`, bounded poll-then-timeout (`POLL_INTERVAL` 25 ms, `MineError::LockTimeout` on deadline), lock-file parent creation, and release-on-`Drop`.
- **New backend**: open the lock file (read+write+create) once, then loop `fs4::FileExt::try_lock(&file)`; on `fs4::TryLockError::WouldBlock` sleep and retry until the deadline; on `fs4::TryLockError::Error(e)` return `MineError::Io(e)`. `FileLock` holds the `std::fs::File` (which owns the OS lock); `Drop` calls `fs4::FileExt::unlock` then the file auto-closes.
- **`retry_io` and `is_sharing_violation`** kept unchanged (already safe; used by `atomic_write` for rename retry on Windows sharing violations, raw OS error 32).

### WP4 — Port forward sound Plan 02 work

No changes required to the domain layer, `atomic_write.rs`, or `toml_store.rs`: the `file_lock` public API surface (`acquire_exclusive`/`FileLock`/`retry_io`) is identical, so consumers compile unchanged. The 91-test suite (63 lib + 9 domain + 10 init + 9 persistence) is carried forward verbatim except the two intentional test changes below.

### WP5 — Focused tests

- `src/infrastructure/file_lock.rs` unit tests: `acquire_and_release_lock` (acquire, path, release, reacquire), `lock_file_parent_created` (nested parent dirs). **Strengthened** `contended_lock_times_out` into a deterministic regression test: because `fs4` uses `flock` (POSIX) / `LockFileEx` (Windows) — both per-open-file-description/handle — a second open of the same lock file in the same process/thread contends on every platform (unlike the old `fcntl` per-process semantics where same-process succeeded). The test now asserts the second `acquire_exclusive` returns `MINE_LOCK_TIMEOUT` and that a fresh acquire succeeds after the first guard is dropped — no hang, deterministic on both platforms.
- `tests/persistence.rs::concurrent_writers_do_not_silently_overwrite` (and revision-conflict, atomic-write-recovers-from-missing-markdown, render-determinism, render-repair): carried forward, all green against the new backend.
- `tests/persistence.rs::real_repository_graph_round_trips_byte_for_byte`: updated the bootstrap-state assertions for the compensation node (the graph now carries 9 plans including `02-1`; asserts `ws.plans.len() >= 9`, `ws.get("02-1").is_some()`, `revision >= 2`, Plan 01 `ACCEPTED`). The byte-for-byte round-trip and `validation::validate` on the real graph still pass — proving the serialization model remains byte-compatible with the authoritative fact source *including* the new `REJECTED`/`IN_PROGRESS`/compensation state.

### WP6 — Integration checks

Full quality-gate matrix run (see Verification), including the explicit `cargo clippy --all-targets --all-features -- -D warnings -W unsafe-code` gate (the one the original Plan 02 gate omitted) and a repository-scoped `unsafe` search (see Unsafe verification).

## Verification

### Available checks (all pass)

| Command | Exit | Result |
|---|---|---|
| `cargo fmt --all -- --check` | 0 | clean, no diff |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | no warnings |
| `cargo clippy --all-targets --all-features -- -D warnings -W unsafe-code` | 0 | no `unsafe_code` warnings (the gate the rejected Plan 02 omitted) |
| `cargo build --all-targets --all-features` | 0 | builds clean under `#![forbid(unsafe_code)]` |
| `cargo test --all-targets --all-features` | 0 | 63 lib + 9 domain + 10 init + 9 persistence = **91 passed, 0 failed** |
| focused `file_lock::` tests (3) | 0 | acquire/release, contended-timeout (`MINE_LOCK_TIMEOUT`), parent-created |
| focused `concurrent_writers_do_not_silently_overwrite` | 0 | passes against new backend |
| `real_repository_graph_round_trips_byte_for_byte` | 0 | real `docs/plan/execution-graph.toml` parses, validates, and re-serializes byte-for-byte |

Final test output:

```
unittests src/lib.rs:           63 passed; 0 failed
unittests src/main.rs:           0 passed; 0 failed
tests/domain.rs:                 9 passed; 0 failed
tests/init_service.rs:          10 passed; 0 failed   (Plan 01 suite, still green)
tests/persistence.rs:            9 passed; 0 failed
```

### Unsafe verification (repository-scoped)

The structural guarantee is compile-time: `#![forbid(unsafe_code)]` is active in `src/lib.rs`, `src/main.rs`, `tests/domain.rs`, and `tests/persistence.rs`. `cargo build --all-targets --all-features` succeeds; **any** `unsafe` block/extern/fn/impl in those crates would be a hard compile error, not merely a warning. Build success is therefore the structural proof that `mine`'s own crate contains no `unsafe` code.

Repository-scoped `grep` confirms the same:

```
$ grep -rnE "unsafe[[:space:]]*(\{|extern|fn|impl)" src/ tests/domain.rs tests/persistence.rs
src/infrastructure/file_lock.rs:8://! hand-written `unsafe extern` FFI in `mine`'s own crate.
```

The single structural-construct match is inside a `//!` doc comment (prose), not code. All other `unsafe` occurrences in `src/` are prose in doc/line comments ("any path is unsafe", "broad glob ownership is unsafe") and a test-fn name (`unsafe_reference_path_rejected` / `unsafe_owned_path_rejected`, referring to *path* safety, not Rust `unsafe`). No `unsafe { ... }` block, `unsafe extern`, `unsafe fn`, or `unsafe impl` exists in `mine`'s own source or owned tests.

Note: `fs4` and `rustix` use `unsafe` internally to call the platform lock APIs — that `unsafe` lives inside the vetted external dependencies, exactly as the corrected design requires, and is outside `mine`'s crate (so `#![forbid(unsafe_code)]` in `mine` does not touch it).

### Unavailable bootstrap checks

| Command | Reason |
|---|---|
| `mine design validate` | The `mine` CLI dispatcher (`src/cli/`) is Plan 03; not wired. The underlying marker/namespace validation was delivered/tested in Plan 01; not pretended to run. |
| `mine graph validate` | The `mine` CLI dispatcher is Plan 03; not wired. The underlying graph validation and the real-graph byte-for-byte round-trip + `validate` are exercised by `tests/persistence.rs::real_repository_graph_round_trips_byte_for_byte`. Not pretended to run. |

Per the documented bootstrap exception, the `mine` CLI, MCP server, and lifecycle state commands do not exist yet. The implementing agent transitioned Plan 02-1 only to `IMPLEMENTED` (never `ACCEPTED`) via manual bookkeeping.

## Deviations and local decisions

- **Reconciliation commit (`2df28a3`)** bringing `dev`'s authoritative post-rejection context (graph + Plan 02-1 doc + corrected design + Plan 03 rerouting + rejection report) onto a branch that started from the rejected code branch HEAD. Required because the instruction mandates branching from the rejected branch (to carry code) while the authoritative post-rejection context lives only on `dev`. No production code was touched and no *new* graph transition was invented; it copies the reviewer's already-done work. Recorded as a distinct commit for audit clarity.
- **`src/lib.rs` / `src/main.rs` `#![forbid(unsafe_code)]` edits outside declared exclusive write paths.** Plan 02-1's exclusive paths enumerate specific files but not the crate roots. The plan's acceptance criteria explicitly mandate the crate-level `forbid` guard, which can only be placed at a crate root. These are necessary, additive, one-attribute structural edits with no ownership conflict (Plan 01 is accepted/closed) — analogous to Plan 01's `.gitattributes` and Plan 02's `src/infrastructure/mod.rs` wiring. Flagged here for the reviewer. `tests/init_service.rs` was intentionally *not* modified (it is not in this plan's exclusive paths); `grep` confirms it already contains no `unsafe`.
- **`fs4` vs std 1.89+ `File::try_lock`/`unlock`.** Rust 1.89+ stabilized native `File` locking in `std`, which would also be safe (no `unsafe` in `mine`'s code). The corrected design explicitly directs use of a maintained **external** crate (`fs4`), so `fs4` is used. Fully-qualified `fs4::FileExt::...` calls disambiguate from std's shadowing inherent methods.
- **Contention test became deterministic.** With the old `fcntl` (per-process) backend, same-process contention did not occur on POSIX; the carried-over test accepted either outcome. `fs4` uses `flock` (per-OFD) / `LockFileEx` (per-handle), so same-process contention now occurs on every platform, letting the test assert a hard `MINE_LOCK_TIMEOUT` and a clean reacquire after release. This is a stronger, more portable regression test, not a weakened one.
- **No design/CLI/MCP/Skills changes.** Per scope, the execution-graph format, state machine, persistence format, CLI, MCP, Skills, and later Plans were not redesigned. The corrected `persistence-and-concurrency.md` and Plan 03 rerouting were authored by the independent reviewer and brought forward read-only.

## Acceptance criteria mapping

| Criterion | Evidence |
|---|---|
| `src/` contains no `unsafe`; exclusive lock via a maintained external crate's safe API | `file_lock.rs` rewritten on `fs4::FileExt`; `grep` structural-construct check returns only a doc-comment match; `cargo build` succeeds under `#![forbid(unsafe_code)]`; `cargo clippy -W unsafe-code` zero warnings |
| every governing design contract implemented or reported as blocked | domain model, persistence/concurrency (incl. corrected locking requirement), state machine/algorithms implemented (ported from verified-sound Plan 02); `mine design validate`/`mine graph validate` commands blocked on Plan 03 (logic delivered & tested) |
| all writes within declared ownership | 5 of 7 files under 02-1 exclusive paths; `src/lib.rs`/`src/main.rs` are the disclosed necessary crate-root deviation; reserved-shared graph touched only for documented bootstrap bookkeeping |
| tests discriminate intended semantics incl. real-graph byte-for-byte round-trip | `contended_lock_times_out` (hard timeout + reacquire), `concurrent_writers_do_not_silently_overwrite`, `real_repository_graph_round_trips_byte_for_byte` (parses + validates + byte-identical re-serialization, incl. the 02-1 compensation node) all pass |
| no direct execution-graph editing outside the documented bootstrap exception | production persistence path is `TomlStore::save_with_revision`; the only direct graph edits are the bootstrap start/completion bookkeeping transitions explicitly permitted |
| no unrelated changes or secrets staged | only the 7 implementation files staged; no secrets |
| implementation evidence reproducible | exact commands/exit codes recorded; `cargo test` re-runnable; real-graph round-trip fixes the fact source as the byte-compat benchmark |
| node reaches `IMPLEMENTED`, never self-granted `ACCEPTED` | this report concludes `IMPLEMENTED`; completion bookkeeping sets `IMPLEMENTED` and leaves `ACCEPTED` to the independent reviewer |

## Remaining risks and external actions

- The independent reviewer must review Plan 02-1, transition it to `ACCEPTED` (or `REJECTED`), release Plan 03 to `READY`, and merge the plan branch into `dev`. This agent did not merge, push, or touch `master`.
- `fs4`'s POSIX backend (`flock` via `rustix`) was built on this Windows host (the Windows `LockFileEx` path is what `cargo test` exercised here, including the `contended_lock_times_out` contention and `concurrent_writers_do_not_silently_overwrite` concurrency tests). The POSIX `flock` path should be exercised on the Ubuntu CI runner configured by Plan 01's CI workflow; the same tests are platform-neutral and will run there.
- `#![forbid(unsafe_code)]` is now in `mine`'s lib and bin crate roots and the two 02-1-owned test crates; `tests/init_service.rs` (Plan 01-owned) does not carry it yet, though `grep` confirms it is unsafe-free today. A future plan may add it there for uniformity (touching a Plan 01-owned file, so out of this plan's scope).
- `mine design validate` / `mine graph validate` remain unavailable as commands until Plan 03 wires the CLI dispatcher.

## Toolchain

Unchanged from Plan 01/02: `rustc 1.97.1`, `cargo 1.97.1`, `rustfmt 1.9.0`, `clippy 0.1.97`, `stable-x86_64-pc-windows-msvc`; VS 2022 MSVC tools auto-discovered by rustc. Added dependency: `fs4 1.1.0` (sync feature; transitive: `rustix`, `bitflags`, `linux-raw-sys` on POSIX). MSRV 1.75 (fs4 sync) ≤ 1.85 (crate rust-version). The official MSVC toolchain was used; not switched to GNU to hide any platform failure.

## Working-tree state and unrelated changes

The working tree is clean on `plan/02-1-execution-graph-safe-file-locking` after the two implementation commits; this report is the only remaining file before the completion bookkeeping. The rejected `plan/02-execution-graph-domain-and-persistence` branch is preserved unchanged at `1e007ed`. The pre-existing `.mypy_cache/` remains on disk (gitignored, never deleted). No unrelated pre-existing modifications were discarded, reset, stashed, or cleaned. Nothing was merged into `dev`, nothing was pushed, `master` was not touched, and no `plan/03*` branch was created.
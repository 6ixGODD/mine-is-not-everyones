# Plan 02-1 Independent Bootstrap Review

- **Plan reviewed**: `docs/plan/02-1-execution-graph-safe-file-locking.md` (compensation for rejected Plan 02)
- **Reviewer role**: independent bootstrap reviewer (MINE CLI/MCP not yet available; graph transitions recorded manually per the documented `AGENTS.md` bootstrap exception)
- **Predecessor**: Plan 01 `ACCEPTED` (merged into `dev`); Plan 02 `REJECTED` (`docs/plan/reports/02-execution-graph-domain-and-persistence-review.md`)
- **Branch reviewed**: `plan/02-1-execution-graph-safe-file-locking`; working tree clean at review start
- **Commits reviewed**: `2df28a3` (context reconciliation), `6b58dc4` (start bookkeeping, `READY`→`IN_PROGRESS`), `61adf3f` (`feat(file-lock)`: fs4 replacement), `2f43322` (`feat(crate-policy)`: `forbid(unsafe_code)`), `3ac1efb` (implementation report), `307ecf6` (completion bookkeeping, `IN_PROGRESS`→`IMPLEMENTED`)
- **Incremental diff basis**: `git diff 1e007ed 307ecf6` (rejected Plan 02 HEAD → this plan's HEAD) — 14 files, +567/−197, matching the report's claim
- **Scope check basis**: `git diff 6b58dc4 307ecf6 -- src tests Cargo.toml Cargo.lock` — 7 files, matching the report's "changed files" list exactly

## Verdict: **ACCEPTED**

Plan 02-1 fully remediates the sole defect that caused Plan 02's rejection (undisclosed hand-written `unsafe` FFI in `mine`'s own crate for file locking), ports forward the rest of Plan 02's already-verified-sound domain and persistence work byte-for-byte unchanged, adds a structural compile-time guard (`#![forbid(unsafe_code)]`) that the original Plan 02 gate lacked, and does so within a scope that introduces no unrelated redesign. All independently re-run commands pass; the real execution graph continues to parse, validate, and round-trip byte-for-byte including the new bootstrap states.

## Verification against the specific ordered concerns

### 1. All hand-written unsafe FFI and unsafe blocks removed from MINE-owned code

Confirmed by direct code reading and by two independent, complementary methods:

- `rg -n --glob '*.rs' '\bunsafe\b' src tests` (run myself) returns 10 matches, **all prose** — comments and doc comments referencing "unsafe path"/"unsafe glob ownership"/the historical defect being fixed. Zero `unsafe {`, `unsafe fn`, `unsafe impl`, or `unsafe extern` constructs anywhere in `src/` or `tests/` (including `tests/init_service.rs`, which has zero matches at all).
- Read `src/infrastructure/file_lock.rs` in full: the old `unsafe extern "C"`/`"system"` FFI blocks, `#[repr(C)]` structs, and both `unsafe { ... }` call sites are gone. The new implementation opens the lock file once and loops on `fs4::FileExt::try_lock`/`unlock`, entirely safe Rust.
- Structural (compile-time) proof, not just textual: `cargo build --all-targets --all-features` succeeds under `#![forbid(unsafe_code)]` in `src/lib.rs` and `src/main.rs` — any reintroduced `unsafe` construct in those crates would be a hard compile error, not a lint warning.

This matches the rejection report's finding precisely and closes it. **PASS.**

### 2. `#![forbid(unsafe_code)]` covers every applicable MINE crate and target

- `src/lib.rs` (library crate) and `src/main.rs` (binary crate): both carry `#![forbid(unsafe_code)]` with an explanatory comment citing `AGENTS.md`. Confirmed by direct read.
- `tests/domain.rs` and `tests/persistence.rs` (the two integration-test crates this plan owns): both carry the same attribute. Confirmed by direct read and by the incremental diff (`+3` lines in `tests/domain.rs`, matching attribute addition; `tests/persistence.rs` diff includes the same two-line addition at the top).
- `tests/init_service.rs` (Plan 01-owned, not in this plan's exclusive write paths) does **not** carry the attribute. This is disclosed candidly in the implementation report ("out of scope... `grep` confirms it already contains no `unsafe`"). Independently confirmed via `rg` above: zero `unsafe` occurrences there. This is a minor, low-risk, honestly-disclosed gap in structural (compile-time) coverage of one file that is textually clean today and not owned by this plan — **not blocking**, but flagged as a follow-up for whichever future plan next touches that file.
- No other `.rs` crate roots exist yet (CLI/MCP/skills modules are Plan 03+ and not present). **PASS**, with one disclosed, non-blocking, low-risk gap noted above.

### 3. The selected `fs4` locking API preserves required semantics

Independently verified against official `fs4` v1.1.0 documentation (docs.rs, crates.io/lib.rs feature list), not just the report's paraphrase:

- **Exclusive lock**: `fs4::FileExt::try_lock`/`lock` provide whole-file exclusive (read-write) locks, distinct from the shared/read variants — matches the design's "exclusive lock" requirement.
- **Timeout/retry**: `try_lock` is non-blocking and returns `TryLockError::WouldBlock` on contention (confirmed against the published API surface, which mirrors std's stabilized `TryLockError` shape). The reimplementation's `acquire_exclusive` loops on this exact variant, sleeping `POLL_INTERVAL` (25 ms) and checking a deadline computed from the caller's `timeout`, returning `MineError::LockTimeout` on expiry — read directly in `src/infrastructure/file_lock.rs`. This is the same poll-then-timeout contract the rejected Plan 02 implementation had (public contract preserved, per the plan's own requirement).
- **Drop-release**: `FileLock`'s `Drop` impl calls `fs4::FileExt::unlock(&self.file)` before the file handle auto-closes — read directly in code.
- **Cross-platform**: `fs4` wraps POSIX `flock` (via `rustix`, a pure-Rust binding, no raw `libc`) and Windows `LockFileEx` behind one safe trait; `Cargo.lock` confirms `fs4 1.1.0` pulls in `rustix` and `windows-sys` as platform backends — matching the design's "cross-platform (POSIX and Windows) advisory file lock" requirement.
- **Feature/MSRV**: `Cargo.toml` declares `fs4 = "1.1.0"` with no `features = [...]` override, meaning `fs4`'s default feature (`sync`) is used — independently confirmed the crate's default is `default = ["sync"]` with no extra dependencies, and MSRV 1.75 for that feature, well under this crate's declared `rust-version = "1.85"`.

All claims independently corroborated against the published crate documentation, not merely trusted from the implementation report. **PASS.**

### 4. Contention and concurrent-writer tests genuinely exercise the lock

- `contended_lock_times_out` (`src/infrastructure/file_lock.rs` unit test, read directly): opens a real lock file, acquires it, then attempts a **second real** `acquire_exclusive` call against the *same path* with a short timeout while the first guard is still alive. This is a genuine second OS-level lock attempt (not a mock, not a pre-computed assertion) — it must time out because `flock`/`LockFileEx` both lock per open-file-description/handle, so a second open in the same process genuinely contends. The test then drops the first guard and asserts a fresh acquire succeeds. I independently re-ran this exact test (via `cargo test`) and it passed deterministically on this host (Windows, so the `LockFileEx` backend), consistent with the report's disclosure that only the Windows path was exercised here.
- `lock_acquired_and_released` (`tests/persistence.rs`, read directly): calls the real `mine::infrastructure::file_lock::acquire_exclusive` (not a stub) directly, confirming the public API is genuinely wired end-to-end from the integration-test crate, not only from the unit-test module inside `file_lock.rs` itself.
- `concurrent_writers_do_not_silently_overwrite` (`tests/persistence.rs`, unchanged since Plan 02, previously verified sound): two `TomlStore::save_with_revision` calls against the same on-disk graph, second with a stale `expected_revision`; asserts `MINE_REVISION_CONFLICT` rather than a silent overwrite. This exercises the lock-then-reload-then-recheck-revision sequence through the real `TomlStore`, not a mock.
- None of these are trivially-passing (e.g., asserting `Ok(())` unconditionally or asserting on a value the code can't actually fail to produce); each has a genuine failure mode it would catch (a hang, a silent overwrite, a wrong error code). **PASS.**

### 5. The real execution graph still parses, validates, and round-trips exactly

- Independently re-ran `cargo test real_repository_graph_round_trips_byte_for_byte` (part of the full suite run below): loads the actual `docs/plan/execution-graph.toml` from disk, parses it into `PlanWorkspace`, calls `validation::validate`, re-serializes, and asserts byte-for-byte equality with the original file — passed.
- Read the test's updated assertions: `ws.plans.len() >= 9` and `ws.get("02-1").is_some()`, correctly reflecting the current graph shape (Plan 01 `ACCEPTED`, Plan 02 `REJECTED`, Plan 02-1 present). This is a strengthening of the assertion to match the new bootstrap state, not a weakening.
- Independently confirmed the graph node data itself: read `docs/plan/execution-graph.toml` at this branch's HEAD (`307ecf6`) — `revision = 4`; node `02` = `REJECTED` with the correct `rejection_reason` and `compensating_plan = "02-1"`; node `02-1` = `IMPLEMENTED` with both real implementation commit hashes (`61adf3f8...`, `2f433222...`) recorded, `owner="bootstrap"`, correct `started_at`/`updated_at`; node `03` remains `BLOCKED` with `hard_predecessors = ["02-1"]` (correctly *not* auto-released to `READY` on `IMPLEMENTED`, since only independent-review `ACCEPTED` releases a successor). **PASS.**

### 6. Plan 02-1 contains only the compensation, no unrelated redesign

- `git diff 1e007ed 307ecf6 -- src/domain/ src/application/ src/infrastructure/toml_store.rs src/infrastructure/atomic_write.rs tests/init_service.rs` is **empty** — the entire domain layer, application layer, `toml_store.rs`, `atomic_write.rs`, and the Plan 01 test suite are byte-for-byte identical to the rejected Plan 02 branch. This directly confirms the plan's "port forward sound Plan 02 work... without re-litigating it" requirement was honored literally, not just claimed.
- The only production-code changes are exactly the 5 files declared as touched: `Cargo.toml`, `Cargo.lock`, `src/infrastructure/file_lock.rs`, `src/lib.rs`, `src/main.rs`, plus the 2 test files (`tests/domain.rs`, `tests/persistence.rs`) — verified via `git diff --stat 6b58dc4 307ecf6 -- src tests Cargo.toml Cargo.lock`, matching the report's "Changed files" list exactly, 7 files.
- No CLI, MCP, Skills, rendering, or Git-infrastructure code exists on this branch (confirmed no `src/cli/`, `src/mcp/` etc.) — no premature Plan 03+ work.
- `docs/plan/03-cli-json-rendering-git-and-workspace-lifecycle.md` is byte-identical between `dev` (`e783b29`) and this branch's HEAD (`git diff` empty) — the reviewer's Plan 02 rerouting was carried forward untouched, not re-edited. **PASS.**

### 7. Disclosed write-scope deviations are necessary and harmless

- `src/lib.rs`/`src/main.rs` edits (crate roots, +8/+4 lines, single attribute + comment each) are outside the plan's literally-declared exclusive write paths but are structurally required — `#![forbid(unsafe_code)]` can only be placed at a crate root, and the plan's own acceptance criteria mandate it. Plan 01 (which broadly owns `src/`) is `ACCEPTED`/closed with no active conflicting branch. This is the same accepted pattern as Plan 01's `.gitattributes` and Plan 02's `src/infrastructure/mod.rs` wiring in prior reviews — disclosed, minimal, additive, no ownership conflict. **Not a defect.**
- `Cargo.toml`/`Cargo.lock` changes are explicitly claimed in this plan's own declared exclusive write paths (added specifically and only for this purpose, per the plan document itself) and the diff confirms exactly one dependency line added (`fs4 = "1.1.0"`) plus its transitive lockfile entries — no other manifest change. **Not a defect.**
- The reconciliation commit (`2df28a3`) bringing `dev`'s authoritative post-rejection graph/design/Plan-doc state onto a branch that (per instruction) had to start from the rejected code branch's HEAD: independently confirmed via `git diff e783b29 2df28a3 -- <the six affected files>` — **empty diff**, i.e., this commit reproduces `dev`'s state exactly, byte-for-byte, introducing no drift and no new decision. **Not a defect.**

### 8. Bootstrap graph state is consistent

- Revision monotonically incremented at each transition: `dev` at `e783b29` = revision 2 → `6b58dc4` (start) = revision 3 → `307ecf6` (complete) = revision 4. Each transition commit's message documents the exact revision delta and field changes; independently confirmed via `git show <commit> -- docs/plan/execution-graph.toml`.
- Plan 02-1 was transitioned only to `IMPLEMENTED`, never self-`ACCEPTED` — confirmed by direct read of the final TOML state and the report's explicit statement.
- Plan 03 correctly remains `BLOCKED` throughout (successor release is gated on independent-review `ACCEPTED`, not `IMPLEMENTED`) — confirmed in both bootstrap commits' messages and the final TOML.
- `docs/plan/execution-graph.md` (generated view) was resynchronized at each transition (revision + status rows) — confirmed via `git show --stat` on both bookkeeping commits.
- The rejected Plan 02 node (`02`) is untouched by this plan (still `REJECTED`, same `rejection_reason`, same `compensating_plan = "02-1"`) — confirmed via diff, no regression of the reviewer's finding. **PASS.**

## Independently executed commands

| Command | Exit | Result |
|---|---|---|
| `cargo fmt --all -- --check` | 0 | clean, no diff |
| `cargo clippy --all-targets --all-features -- -D warnings -W unsafe-code` | 0 | zero warnings (including the `unsafe_code` lint explicitly enabled) |
| `cargo test --all-targets --all-features` | 0 | 63 lib + 9 domain + 10 init_service + 9 persistence = **91 passed, 0 failed** |
| `rg -n --glob '*.rs' '\bunsafe\b' src tests` | 0 (10 matches) | all prose (comments/doc comments); zero `unsafe` code constructs |
| `git diff 1e007ed 307ecf6 -- src/domain/ src/application/ src/infrastructure/toml_store.rs src/infrastructure/atomic_write.rs tests/init_service.rs` | 0 (empty) | confirms byte-for-byte port-forward, no unrelated redesign |
| `git diff e783b29 2df28a3 -- <6 reconciled files>` | 0 (empty) | confirms the reconciliation commit introduces no drift from `dev`'s authoritative state |
| `git diff e783b29 307ecf6 -- docs/plan/03-...md` | 0 (empty) | Plan 03's rerouting untouched |
| `cargo build --all-targets --all-features` (implicit via clippy/test) | 0 | succeeds under `#![forbid(unsafe_code)]` in lib/bin crates |
| fs4 v1.1.0 API/feature/MSRV cross-check against docs.rs/crates.io | — | independently corroborates `try_lock`/`unlock`/`TryLockError::WouldBlock`, default `sync` feature, MSRV 1.75 |

## Findings summary

No blocking findings. One pre-existing, low-severity, disclosed, non-blocking observation carried forward as a follow-up, not a defect:

- `tests/init_service.rs` lacks `#![forbid(unsafe_code)]` (Plan 01-owned file, out of this plan's scope; confirmed textually unsafe-free today). A future plan touching that file should add the attribute for uniformity.

## Contract and scope assessment

- **Defect remediation**: PASS — the exact rejection cause is closed with independent, multi-method verification (textual `rg`, structural `forbid` + successful build, and direct code reading of the new backend).
- **Fidelity of ported-forward work**: PASS — byte-identical diff proof, not just a claim.
- **Scope discipline**: PASS — only the declared/disclosed files changed; no unrelated redesign; no premature Plan 03+ code.
- **Test genuineness**: PASS — contention and concurrency tests exercise real OS-level locking and real revision-conflict paths, independently re-run.
- **Graph/bootstrap bookkeeping consistency**: PASS — monotonic revisions, correct statuses, no self-acceptance, Plan 03 correctly withheld from release until now.
- **Repository safety**: PASS — no evidence of `reset --hard`/`clean`/force-push/blind-stash; `master` untouched; nothing merged into `dev` by the implementer; rejected Plan 02 branch untouched.

## Handoff summary

**ACCEPTED.** Plan 02-1 closes the exact defect that caused Plan 02's rejection (undisclosed `unsafe` FFI file locking), replacing it with a byte-for-byte-verified port of the sound domain/persistence work plus a `fs4`-backed lock whose semantics (exclusive, timeout/retry via `WouldBlock`, Drop-release, cross-platform) are independently corroborated against official crate documentation, and adds compile-time `#![forbid(unsafe_code)]` enforcement at both applicable crate roots and the two test crates it owns. All specified and requested commands were independently re-run and pass (91/91 tests, zero unsafe-code lint warnings, zero real `unsafe` constructs by direct grep). The real execution graph continues to parse, validate, and round-trip byte-for-byte, and the bootstrap graph bookkeeping is internally consistent and honest (`IMPLEMENTED`, never self-`ACCEPTED`; Plan 03 correctly withheld until this review).

Proceeding with the bootstrap acceptance procedure: mark Plan 02-1 `ACCEPTED`, release Plan 03 to `READY`, merge into `dev`, verify `dev`, delete only the accepted local compensation branch, and preserve the rejected Plan 02 branch as evidence.

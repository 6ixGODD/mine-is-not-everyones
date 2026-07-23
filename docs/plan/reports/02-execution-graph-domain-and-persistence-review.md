# Plan 02 Independent Bootstrap Review

- **Plan reviewed**: `docs/plan/02-execution-graph-domain-and-persistence.md`
- **Reviewer role**: independent bootstrap reviewer (MINE CLI/MCP not yet available; graph transitions recorded manually per the documented `AGENTS.md` bootstrap exception, exactly as the Plan 02 implementer itself did for its own start/completion bookkeeping)
- **Baseline (predecessor)**: Plan 01 `ACCEPTED`, merged into `dev` at `93f3660f6d63851a357f1541ddf4c44399ef2840`
- **Implementation commits reviewed**: `f4cc229` (graph-domain), `b58ad4c` (graph-persistence), `6166861` (tests) on `plan/02-execution-graph-domain-and-persistence`
- **Report commit reviewed**: `d610ba5` (`docs(plan-02): implementation report`)
- **Bootstrap bookkeeping commits inspected**: `a3c7494` (`READY`→`IN_PROGRESS`), `1e007ed` (→`IMPLEMENTED`)
- **Branch reviewed**: `plan/02-execution-graph-domain-and-persistence`; working tree was clean at review start
- **`skills/mine-plan-review/SKILL.md`**: present in the repository and read/followed in full

## Verdict: **REJECTED**

Plan 02's execution-graph domain model, validation algorithms, and TOML/atomic-write persistence are well-designed, well-tested, and independently verified as sound. However, `src/infrastructure/file_lock.rs` — part of this plan's own delivered scope — contains hand-written `unsafe extern "C"`/`unsafe extern "system"` FFI blocks and `unsafe { ... }` calls (`fcntl`/`LockFileEx`) directly inside `mine`'s own crate. This violates `AGENTS.md`'s explicit, unconditional repository-wide rule: **"Business code must not use `unsafe`."** The implementation report does not disclose this deviation anywhere, and the automated quality gate specified by the plan (`cargo clippy --all-targets --all-features -- -D warnings`) does not catch it, because clippy's `unsafe_code` lint is not enabled by `-D warnings` unless explicitly requested. This is exactly the class of undisclosed hard-rule violation that independent review exists to catch and that a green test suite does not excuse.

This is a hard, unambiguous governance-contract failure (not a design ambiguity), so per the review contract "one failed hard contract is sufficient to reject."

## Findings, ordered by severity

### 1. (BLOCKING) `unsafe` code in `mine`'s own crate, violating `AGENTS.md`'s "Business code must not use `unsafe`"

**Evidence:**

```
src/infrastructure/file_lock.rs:171:    let rc = unsafe { fcntl_setlk(fd, &mut flock) };
src/infrastructure/file_lock.rs:202:    let rc = unsafe {
src/infrastructure/file_lock.rs:238:unsafe extern "system" {
src/infrastructure/file_lock.rs:269:unsafe extern "C" {
src/infrastructure/file_lock.rs:274:unsafe fn fcntl_setlk(fd: i32, flock: &mut libc_flock) -> i32 {
```

- `grep -rn "unsafe" src/` on the Plan 01 baseline (`3a5864e`) returns **zero** matches; the same command on this plan's HEAD (`1e007ed`) returns the five matches above, all in `src/infrastructure/file_lock.rs`. This is a regression introduced entirely by this plan.
- `AGENTS.md` → "Repository quality gates" → "Business code must not use `unsafe`." This sentence is unconditional and is the last bullet of the Rust quality-gate section governing exactly this crate (`mine` core). No design document (`domain-model.md`, `persistence-and-concurrency.md`, `state-machine-and-algorithms.md`) carves out an exception for locking/FFI, and the implementation report's "Deviations and local decisions" section — which candidly discloses several other local decisions (write-scope-overlap semantics, `DesignReference` dual-form modeling, `src/infrastructure/mod.rs` wiring, no `src/application/` changes, cross-platform lock code) — says nothing about introducing `unsafe`. This omission means the reviewer, not the process, is the only safeguard that caught it.
- Independently confirmed that the specified gate does not flag it:
  ```
  $ cargo clippy --all-targets --all-features -- -D warnings
  Finished ... (exit 0, no warnings)
  $ cargo clippy --all-targets --all-features -- -W unsafe-code
  warning: usage of an `unsafe` block --> src\infrastructure\file_lock.rs:202:14
  warning: usage of an `unsafe extern` block --> src\infrastructure\file_lock.rs:238:1
  ... (additional occurrences)
  ```
  No `#![forbid(unsafe_code)]` or `#![deny(unsafe_code)]` exists anywhere in `src/lib.rs` or `src/main.rs`, so nothing in the committed configuration enforces the rule either.
- The report's own "Deviations" section states the Windows path was exercised but "the POSIX path compiles only on Unix targets ... and was not built on this Windows host," meaning the `unsafe` POSIX branch shipped with **zero execution evidence** on this review host, compounding the risk.
- This finding matches the review skill's "require a compensating plan" category directly: it changes "cross-component ownership/lifecycle, **concurrency** or process cleanup" (file locking is exactly the concurrency-control mechanism the whole persistence design depends on), and it is a hard governance-contract violation, not a local cosmetic defect. It cannot be "fixed directly during review" because (a) the correct replacement requires adding an external dependency, which touches `Cargo.toml`/`Cargo.lock` — files this plan does not exclusively own — and (b) selecting and vetting a locking crate is more than a trivial, fully-verifiable-in-session fix; it changes the persistence layer's concurrency implementation, which downstream Plan 03 (Git/CLI) and every future graph writer depend on.

### 2. (non-blocking, disclosed) Write-scope-overlap semantics interpretation

The report explicitly discloses and justifies why `validate()` does not reject the sequential Plan 01/Plan 02 exclusive-write-path overlap (`src/` vs `src/domain/`), reserving that check for `parallel_wave` among `READY` plans only. Independently re-read `docs/design/execution-graph/state-machine-and-algorithms.md`: "Validation" indeed does not list write-scope overlap, while "Parallel wave" explicitly excludes "exclusive-write overlap" only among wave candidates. The implementation matches the design text. Verified via `tests/domain.rs::diamond_dependency_validates_and_topologizes`, `tests/persistence.rs::real_repository_graph_round_trips_byte_for_byte` (which needs exactly this interpretation to validate the live graph), and `src/domain/validation.rs::validate`/`parallel_wave`. **Not a defect.**

### 3. (non-blocking, disclosed) `DesignReference` structured form vs. flat-array fact source

The design (`domain-model.md`) specifies `{ path, anchors[], reason }`; the live TOML stores a flat string array. The implementation models both and serializes the flat form so the real graph round-trips byte-for-byte (independently verified below). The report discloses this as an open tension for Plan 03/04 to resolve. Reasonable, bounded, disclosed. **Not a defect for this plan.**

### 4. (non-blocking) `src/infrastructure/mod.rs` wiring edit outside the letter of "Exclusive write paths"

Same pattern as Plan 01's `.gitattributes` note: structurally necessary `pub mod` wiring, disclosed in the report, no ownership conflict. **Not a defect.**

### 5. (non-blocking, code-quality observation) `topological_sort`'s re-scan loop

`src/domain/validation.rs::topological_sort` re-scans the entire plan list once per dequeued node to preserve declaration order (`O(n²)`/worse in the inner `order.iter().any` / `queue.iter().any` checks), rather than a straightforward Kahn's-algorithm queue. Functionally correct per the diamond-dependency and cycle tests, and workspace-scoped graphs are small (single digits of plans), so this is not a real performance risk. Noted for future simplification, not blocking.

### 6. (non-blocking, code-quality observation) `PlanStatus::validate_transition` allows same-status "transitions" (`Accepted → Accepted`, etc.)

Not in the design's explicit edge table but not prohibited either; a harmless idempotency convenience with no exercised call site yet. Worth revisiting when Plan 03 wires the actual transition service (should probably not allow a no-op re-"acceptance" of an already-terminal `ACCEPTED` node).

## Acceptance traceability matrix

| Area | Governing reference | Evidence | Independent verification | Result |
|---|---|---|---|---|
| Domain model (`PlanWorkspace`/`PlanNode`) | `domain-model.md` | `src/domain/graph.rs` | Read code; fields match the design's aggregate list; TOML round-trips the real `docs/plan/execution-graph.toml` byte-for-byte (see below) | PASS |
| Safe paths | `domain-model.md` "Safe paths" | `src/domain/path.rs::normalize_repo_relative` | Read code + 9 unit tests (absolute, drive-root, UNC, backslash, traversal, wildcard, empty); independently re-ran `cargo test` | PASS |
| Design references | `domain-model.md` "Design references" | `src/domain/design_reference.rs` | Confirmed empty-reference rejection and unsafe-path rejection via tests; confirmed dual-form modeling is disclosed, bounded | PASS |
| 7-state machine, no generic `set-status` | `state-machine-and-algorithms.md` "States"/"Allowed transitions" | `src/domain/status.rs::validate_transition` | Verified every edge in the design's table is allowed and every non-edge is rejected via `tests/domain.rs::full_lifecycle_transitions_allowed`, `reject_path_requires_review_then_compensation`, `accepted_is_terminal_no_back_transition`, plus unit tests | PASS |
| Validation (unique IDs/paths, predecessors, acyclicity, safe paths, non-empty design refs) | `state-machine-and-algorithms.md` "Validation" | `src/domain/validation.rs::validate` | Independent tests: duplicate id/path, missing predecessor, self-dependency, cycle, empty design references, unsafe path — all rejected with correct `MINE_*` codes | PASS |
| Parallel wave | `state-machine-and-algorithms.md` "Parallel wave" | `src/domain/validation.rs::parallel_wave` | Verified ancestor-pair exclusion and write-scope-overlap exclusion via `tests/domain.rs` and `tests/validation` unit tests | PASS |
| TOML persistence, machine fact source | `persistence-and-concurrency.md` | `src/infrastructure/toml_store.rs` | Real-graph byte-for-byte round-trip test independently re-run; revision-conflict, atomic-write-recovers-from-missing-markdown, deterministic render tests re-run | PASS |
| Revision + locking (lock → reload → recheck revision → atomic write → render) | `persistence-and-concurrency.md` "Revision and locking" | `TomlStore::save_with_revision` | Sequence matches design; revision-conflict and concurrent-writer tests re-run and pass | PASS (sequence) / **FAIL (implementation safety of the lock itself — see Finding 1)** |
| Exclusive lock implementation must not use `unsafe` in `mine`'s own code | `AGENTS.md` "Repository quality gates" | `src/infrastructure/file_lock.rs` | `grep` confirms 5 `unsafe` occurrences; regression vs. Plan 01 baseline (0 occurrences); confirmed undisclosed in report; confirmed clippy's default `-D warnings` gate does not catch it | **FAIL** |
| Atomic writes never truncate in place | `persistence-and-concurrency.md` | `src/infrastructure/atomic_write.rs` | 5 unit tests (create, overwrite, no temp file left behind, parent-dir creation, unrelated sibling preserved) independently re-run | PASS |
| No writes outside declared ownership | plan Scope ownership | `git diff` | 13 implementation files all under declared exclusive paths + disclosed `mod.rs` wiring; reserved-shared graph files touched only for disclosed bootstrap bookkeeping | PASS |
| No unrelated changes | AGENTS.md | full diff vs. `dev` (`93f3660`) | `git diff --stat 93f3660 HEAD`: only the 16 files claimed (13 implementation + report + 2 graph-bookkeeping files); Plan 01-owned files, `docs/design/**` (content), `REQUIREMENTS.md` untouched | PASS |
| Node reaches `IMPLEMENTED`, never self-`ACCEPTED` | plan Acceptance criteria | graph state | `docs/plan/execution-graph.toml` at review start: node `02` status `IMPLEMENTED`, not `ACCEPTED` | PASS |
| Quality gates | AGENTS.md | — | Independently re-ran all three; see Commands below | PASS (gates as specified) / gates insufficiently scoped (see Finding 1) |

## Independently executed commands

| Command | Exit | Result |
|---|---|---|
| `cargo fmt --all -- --check` | 0 | clean |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | no warnings (does **not** flag `unsafe_code`) |
| `cargo clippy --all-targets --all-features -- -W unsafe-code` | 0 (warnings only, not denied) | flags multiple `unsafe` block/extern-block usages in `src/infrastructure/file_lock.rs` |
| `cargo test --all-targets --all-features` | 0 | 63 lib unit + 9 domain + 10 init_service + 9 persistence = 91 passed, 0 failed (matches report) |
| `grep -rn "unsafe" src/` (HEAD) | 0 (5 matches) | all in `src/infrastructure/file_lock.rs` |
| `grep -rn "unsafe" src/` (Plan 01 baseline `3a5864e`) | 1 (no matches) | confirms this is a Plan 02 regression, not pre-existing |
| `git diff --stat 93f3660 HEAD` | 0 | 16 files changed (13 implementation + report + `execution-graph.toml`/`.md` bootstrap bookkeeping), matches report's claim |
| `git diff 93f3660 HEAD -- docs/design/ REQUIREMENTS.md` (excluding this review's own edits, checked before making them) | 0 | empty — untouched by the reviewed implementation |
| Read `tests/persistence.rs::real_repository_graph_round_trips_byte_for_byte` and re-ran `cargo test real_repository_graph_round_trips_byte_for_byte` | 0 | passes; independently confirms the real graph parses, validates, and re-serializes byte-identically |

## Contract and scope assessment

- **Scope compliance**: PASS with disclosed, non-conflicting deviations (Finding 4).
- **Repository safety**: PASS — no `reset --hard`/`clean`/force-push/blind-stash evidence; `master` untouched; nothing merged into `dev` by the implementer.
- **Domain correctness (states, paths, validation, algorithms)**: PASS, independently verified against the design text and adversarial tests.
- **Persistence correctness (TOML/atomic-write/revision/render)**: PASS for logic and sequencing.
- **Persistence *safety* of the concurrency primitive itself**: **FAIL** — see Finding 1. This is the one hard contract (a repository-wide, explicit, unconditional AGENTS.md rule) that fails, and it is sufficient on its own to reject regardless of how much other work passes.
- **No premature Plan 03+ implementation**: PASS — no `src/cli/`, `src/mcp/`, Git infrastructure, or application-layer transition service exists.
- **Unrelated-change preservation**: PASS.

## Security / data-handling statement

No secrets are introduced. The security-relevant issue is architectural: hand-rolled `unsafe extern` FFI to `fcntl`/`LockFileEx` inside the project's own crate is exactly the kind of code the repository's blanket "no unsafe in business code" rule exists to prevent (memory-safety risk surface owned and maintained by this project rather than by an externally vetted, widely-used crate). The POSIX branch additionally shipped with no execution evidence on this host. No other privacy/security boundary is affected.

## Reviewer fixes

None applied. Per the review skill, a direct reviewer fix is only appropriate when the defect is local, requires no product/architecture decision, and needs no change outside the plan's own exclusive write paths. This defect requires adding an external dependency (a `Cargo.toml`/`Cargo.lock` change outside Plan 02's declared ownership) and reworking a concurrency-critical component that Plan 03 and all future graph writers depend on — squarely in the "require a compensating plan" category, not a direct fix.

## Passed / failed / skipped / unavailable checks summary

- Passed: `cargo fmt`, `cargo test` (91/91), all traceability-matrix rows except the lock-safety row.
- Failed: `unsafe`-in-business-code contract (Finding 1).
- Skipped: none.
- Unavailable (correctly, per bootstrap exception): `mine design validate`, `mine graph validate` (Plan 03 CLI not wired; consistent with the report's honest disclosure).

## Remedial actions taken by this review

1. Updated `docs/design/execution-graph/persistence-and-concurrency.md` ("Revision and locking") to explicitly require that the exclusive lock be implemented through a maintained external locking crate (e.g. `fs4`) rather than hand-written `unsafe extern` FFI in `mine`'s own crate, per `AGENTS.md`.
2. Created the compensating plan `docs/plan/02-1-execution-graph-safe-file-locking.md` (`READY`, hard predecessor `01`), scoped to replace only `src/infrastructure/file_lock.rs`'s backend via a safe external crate while porting forward the rest of Plan 02's independently-verified-sound work (domain layer, `atomic_write.rs`, `toml_store.rs`, tests).
3. Updated `docs/plan/execution-graph.toml` (revision `3`→`4`, manual bootstrap transition, consistent with the same exception the implementer itself used): node `02` → `REJECTED` with `rejection_reason` and `compensating_plan = "02-1"`; added node `02-1` (`READY`); rerouted node `03`'s `hard_predecessors` from `["02"]` to `["02-1"]`. Regenerated `docs/plan/execution-graph.md` to match (revision parity maintained).
4. Updated `docs/plan/03-cli-json-rendering-git-and-workspace-lifecycle.md`'s "Hard predecessors" section from `02` to `02-1` with an explanatory note. Plan 03 has not started execution (status remains `BLOCKED`, no implementation commits, no report), so it is not yet immutable under `AGENTS.md`'s plan-immutability rule; this is a routing correction, not a rewrite of an executed plan.
5. Did **not** implement the compensating plan, per the review skill ("Do not implement the compensating plan during review unless the user separately asks").
6. Did **not** merge `plan/02-execution-graph-domain-and-persistence` into `dev`, per "For a material failure ... do not merge the implementation branch."

## Remaining risks

- The rejected branch's domain/persistence code (aside from `file_lock.rs`) is sound and should be ported forward into Plan 02-1 rather than reimplemented from scratch, to avoid wasted, re-litigated work; Plan 02-1 documents this explicitly.
- The POSIX lock backend (now to be replaced) had zero execution evidence on this Windows review host; whichever crate Plan 02-1 selects should be exercised on the Ubuntu CI runner already configured by Plan 01's workflow.
- No `#![forbid(unsafe_code)]` (or equivalent enforced lint) exists yet anywhere in the crate; Plan 02-1 or a later plan should consider adding one so this class of defect is caught by the automated gate rather than relying on manual review each time.
- `mine design validate` / `mine graph validate` remain unavailable until Plan 03; unaffected by this rejection.

## Downstream release

**Plan 03 remains `BLOCKED`.** It is rerouted (in the execution graph and in its own not-yet-immutable "Hard predecessors" section) to depend on `02-1` instead of the rejected `02`. Plan 02-1 is `READY` for implementation. The rejected `plan/02-execution-graph-domain-and-persistence` branch is **not** merged into `dev` and is preserved as-is for reference; it must not be silently deleted since it is evidence for this review's findings and a source of reusable code for Plan 02-1.

## Handoff summary

**REJECTED.** Plan 02's domain model, state machine, validation/algorithms, and TOML/atomic-write persistence are sound and independently verified (91/91 tests re-run, real-graph byte-for-byte round-trip confirmed, all design-table transitions and validation rules confirmed against the governing design text). However, `src/infrastructure/file_lock.rs` introduces undisclosed `unsafe` FFI code in `mine`'s own crate, violating `AGENTS.md`'s unconditional "Business code must not use `unsafe`" rule — a regression not present in the Plan 01 baseline, not caught by the specified `cargo clippy -D warnings` gate, and not mentioned anywhere in the implementation report's otherwise-candid deviations section. This is a hard contract failure sufficient for rejection on its own. Compensating plan `docs/plan/02-1-execution-graph-safe-file-locking.md` is created and released as `READY`; downstream Plan 03 is rerouted to it and remains `BLOCKED`. The governing design (`persistence-and-concurrency.md`) is corrected to require a maintained external locking crate. `master` was not touched; `dev` was not touched; the rejected plan branch was not merged and not deleted. Next action: implement Plan 02-1.

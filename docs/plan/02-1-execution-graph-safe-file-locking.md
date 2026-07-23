# Plan 02-1: Execution graph safe file locking (compensation for rejected Plan 02)

## Status

`READY`

## Goal

Replace the rejected Plan 02 implementation's hand-written `unsafe extern` FFI file-locking backend (`src/infrastructure/file_lock.rs`) with a locking implementation that contains no `unsafe` code in `mine`'s own crate, using a maintained external locking crate as required by the corrected `docs/design/execution-graph/persistence-and-concurrency.md` ("Revision and locking"). Port forward the remainder of Plan 02's execution-graph domain and persistence work, which independent review found sound, without re-litigating it.

This plan compensates for `docs/plan/02-execution-graph-domain-and-persistence.md`, which is `REJECTED`. See `docs/plan/reports/02-execution-graph-domain-and-persistence-review.md` for the rejection evidence. Downstream Plan 03 is rerouted to depend on this plan instead of the rejected Plan 02 node.

## Branch contract

- Stable branch: the branch detected by `mine init` (currently `master` for this repository).
- Integration branch: managed `dev`.
- Implementation branch: `plan/02-1-execution-graph-safe-file-locking`.
- Never implement directly on the stable branch or `dev`.
- The user grants standing authorization to create/switch the managed branch, commit scoped files, and let an independent accepted review merge it into `dev`.
- Do not force push, reset hard, clean, blindly stash, rewrite public history, or discard unrelated changes.
- This plan and its reports are ephemeral and must not survive stable release integration.

## Hard predecessors

01

## Governing design references

- [`docs/design/execution-graph/domain-model.md`](../design/execution-graph/domain-model.md)
- [`docs/design/execution-graph/persistence-and-concurrency.md`](../design/execution-graph/persistence-and-concurrency.md) (corrected "Revision and locking" section — read this first; it now mandates a vetted external locking crate and prohibits hand-written `unsafe extern` FFI in `mine`'s own crate)
- [`docs/design/execution-graph/state-machine-and-algorithms.md`](../design/execution-graph/state-machine-and-algorithms.md)

The executor reads the exact documents before mutation, including the rejected Plan 02 review report (`docs/plan/reports/02-execution-graph-domain-and-persistence-review.md`) for the precise defect and the disposition of the rest of Plan 02's work.

## Scope ownership

### Exclusive write paths

- `src/domain/`
- `src/application/`
- `src/infrastructure/toml_store.rs`
- `src/infrastructure/file_lock.rs`
- `src/infrastructure/atomic_write.rs`
- `Cargo.toml`
- `Cargo.lock`
- `tests/domain.rs`
- `tests/persistence.rs`
- `tests/domain/`
- `tests/persistence/`

`Cargo.toml`/`Cargo.lock` are added to this plan's exclusive write paths (they are not otherwise claimed by any active plan branch) solely to add one maintained file-locking dependency; no other manifest change is in scope.

### Reserved shared paths

- `docs/plan/execution-graph.toml`
- `docs/plan/execution-graph.md`
- files owned by other active plan branches

### Read-only context

- `REQUIREMENTS.md`
- non-target `docs/design/` documents
- the rejected Plan 02 implementation and review reports (evidence, not a template to copy the defective locking code from)

## Required work packages

1. **Baseline and evidence** — inspect `dev` at the Plan 01 baseline, the rejected Plan 02 branch's commits (`f4cc229`, `b58ad4c`, `6166861`) as reference material for what already passed review, the review report's exact finding, and the corrected design section.
2. **Dependency selection** — evaluate and add one maintained, actively-supported Rust file-locking crate (for example `fs4`) via `cargo add` against the official crate documentation and repository license/MSRV constraints; record the choice and rationale.
3. **Locking reimplementation** — reimplement `src/infrastructure/file_lock.rs`'s public contract (`acquire_exclusive`, timeout/retry behavior, `FileLock` guard, release-on-drop) using only safe wrappers from the chosen crate; remove all `unsafe extern` blocks and `unsafe { ... }` calls from `mine`'s own source. Confirm with `grep -rn "unsafe" src/` that no occurrence remains outside comments/tests referring to path safety.
4. **Port forward sound Plan 02 work** — bring in the domain layer (`status.rs`, `path.rs`, `design_reference.rs`, `graph.rs`, `validation.rs`), `atomic_write.rs`, and `toml_store.rs` from the rejected branch unchanged (or with only the minimal adaptation required by the new `file_lock` API surface), since review found no other defect in them.
5. **Focused tests** — reuse and adapt `tests/domain.rs` and `tests/persistence.rs` from the rejected branch; add a regression test asserting `file_lock` timeout/acquire/release behavior against the new backend, and a repository-wide lint/grep check (documented, not necessarily automated) that no `unsafe` token appears in `src/` outside safe, reviewed exceptions.
6. **Integration checks** — run the full quality-gate matrix; additionally run `cargo clippy --all-targets --all-features -- -D warnings -W unsafe-code` (or an equivalent explicit `unsafe_code` check) and confirm zero occurrences, since default clippy `-D warnings` does not include the `unsafe_code` lint.
7. **Implementation report** — exact commands, exit codes, commits, the chosen crate and why, deviations, risks, and preserved unrelated changes.

## Verification

```text
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo clippy --all-targets --all-features -- -D warnings -W unsafe-code
cargo test --all-targets --all-features
mine design validate
mine graph validate
```

Run narrower and platform-specific checks required by scope, including exercising the lock on at least one POSIX target if available. Missing tools, skipped checks, timeouts, and non-zero exits are not passes.

## Acceptance criteria

- `src/` (mine's own crate) contains no `unsafe` code; the exclusive file lock is implemented entirely through a maintained external crate's safe API;
- every governing design contract (domain model, persistence/concurrency including the corrected locking requirement, state machine/algorithms) is implemented or explicitly reported as blocked;
- all writes stay within declared ownership;
- tests discriminate intended semantics from plausible wrong behavior, including the real-repository-graph byte-for-byte round-trip check carried over from the rejected branch;
- no direct execution-graph file editing is introduced outside the documented bootstrap exception;
- no unrelated changes or secrets are staged;
- implementation evidence is reproducible;
- the node reaches `IMPLEMENTED`, never self-granted `ACCEPTED`.

## Report path

`docs/plan/reports/02-1-execution-graph-safe-file-locking-implementation.md`

## Downstream release

On independent acceptance, release: 03. (Plan 03's execution-graph hard-predecessor edge is rerouted from the rejected "02" to "02-1"; see the Plan 02 review report and the execution-graph state.)

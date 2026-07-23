# Plan 03 Implementation Report

- **Plan**: `docs/plan/03-cli-json-rendering-git-and-workspace-lifecycle.md`
- **Title**: CLI, JSON, rendering, Git evidence, design backup, and workspace lifecycle
- **Execution date**: 2026-07-24
- **Conclusion**: `IMPLEMENTED` — pending independent reviewer acceptance. The implementing agent did not self-grant `ACCEPTED`; the Plan 03 node was transitioned only to `IMPLEMENTED` via the documented bootstrap exception. As the final bootstrap Plan, the lifecycle CLI being implemented here was **not** used to grant acceptance to its own unreviewed implementation (bootstrap boundary honored).

## Branches and commits

| Item | Value |
|---|---|
| Stable branch | `master` (`1d3a132f8bbffc6ffca60d6bea5b6f36a6a3de36`, unchanged) |
| Integration branch | `dev` (`def825ad50dee00efdb94dda8a8bd5b50549a28a` at branch creation; this plan does not merge into it) |
| Plan branch | `plan/03-cli-json-rendering-git-and-workspace-lifecycle` (from clean `dev` at `def825a`) |
| Start-bookkeeping commit | `82889239cb6536cdedc9d5a25227b35e94311f63` — Plan 03 `READY`→`IN_PROGRESS`, revision `5`→`6`, ownership/timestamps, markdown synchronized |
| Implementation commits | `eaa67f00cabb26c81f08d021d2e1d501a6c0ecda`, `aaeda71f74de718a339b966fbb2181229dc137f3`, `efc7f6f03cb6907d9327fd223ed9eb2c008197ff`, `434cb9f3148d3ac36a9779d7669926d5161cce3b`, `e3cc16ec91a141d1e7009e430bc8d746b75a792c` |

Nothing was merged into `dev`, nothing was pushed, `master` was not touched, no `plan/04*`/`plan/05*` branch was created, and downstream Plans were not released.

### Implementation commits

1. `eaa67f0` `feat(output): stable JSON envelope, human output, exit codes, render module` — `Cargo.toml`, `Cargo.lock`, `src/output/{envelope,human,mod}.rs`, `src/render/mod.rs`.
2. `aaeda71` `feat(infra): read-only Git evidence, safe design backup, workspace service` — `src/infrastructure/{git,design_backup}.rs`, `src/application/workspace_service.rs`.
3. `efc7f6f` `feat(cli): command dispatcher, JSON/human output, plan/graph/design commands` — `src/cli/{commands,context,mod}.rs`.
4. `434cb9f` `feat(crate): wire cli/output/render modules and bin entry` — `src/lib.rs`, `src/main.rs`, `src/application/mod.rs`, `src/infrastructure/mod.rs`.
5. `e3cc16e` `test(cli): end-to-end CLI integration and golden rendering tests` — `tests/cli.rs`, `tests/golden.rs`.

## Changed files (18 implementation files vs start commit `8288923`; +3898 / −23)

```
Cargo.lock                              (serde_json + transitive deps added)
Cargo.toml                              (serde_json = "1")
src/application/mod.rs                  (pub mod workspace_service wiring + doc)
src/application/workspace_service.rs     (new, 273 lines)
src/cli/commands.rs                     (new, 1188 lines)
src/cli/context.rs                      (new, 162 lines)
src/cli/mod.rs                          (new, 330 lines)
src/infrastructure/design_backup.rs     (new, 393 lines)
src/infrastructure/git.rs               (new, 273 lines)
src/infrastructure/mod.rs              (pub mod git/design_backup wiring + doc)
src/lib.rs                              (pub mod cli/output/render + forbid(unsafe_code))
src/main.rs                             (binary wires cli::dispatch + exit codes)
src/output/envelope.rs                  (new, 312 lines)
src/output/human.rs                     (new, 165 lines)
src/output/mod.rs                        (new, 166 lines)
src/render/mod.rs                       (new, 25 lines)
tests/cli.rs                            (new, 365 lines)
tests/golden.rs                         (new, 149 lines)
```

## Work-package evidence

### WP1 — Freeze CLI and JSON envelope

`src/output/envelope.rs` implements the deterministic JSON envelope from `cli-contract.md`: `ok`, `command` (`<group>.<verb>` dotted identifier), `repository`, `workspace_id`, `revision_before`, `revision_after`, `data`, `warnings`. Sorted-keys serialization (via `BTreeMap`) makes output byte-deterministic across `serde_json` versions — asserted by `json_envelope_has_stable_sorted_keys`. Error envelopes reuse the same shape with `ok:false`, a stable `error.code`/`message`/`details` object, empty `data`/`warnings`, routed to **stderr** so successful JSON on stdout stays pipeline-clean.

### WP2 — `mine init`, `mine status`, design marker validation, repository version

`mine init` delegates to the Plan 01 `InitService` (owned exclusively by this plan) and enriches its outcome with Git-detected stable-branch evidence. `mine status` reports repository/graph/git context read-only. Design marker validation reuses `classify`/`DesignMarker::parse` from the domain. `mine repository version show|suggest|set` reads/updates `.mine/config.toml` (atomic write via Plan 02's `atomic_write`); `set` validates semver `MAJOR.MINOR.PATCH`.

### WP3 — `mine workspace open|status|close`

`src/application/workspace_service.rs::WorkspaceService` generates an internal UUID `workspace_id` (no user release-version input), idempotent on an existing non-empty workspace, initializes empty/absent graphs via the locked revision-checked `TomlStore::save_with_revision` path. `close` validates closure (no unresolved plans: every plan terminal `ACCEPTED` or `REJECTED`). Workspace identity is distinct from repository version, per the design ("workspace identity is generated and distinct from repository version").

### WP4 — `mine design backup`

`src/infrastructure/design_backup.rs` validates the design marker + repository ownership; creates `docs/design-backup-<UTC timestamp>/` (deterministic compact UTC suffix from the injected clock); copies managed design **without following external links** (refuses repo-escaping symlinks/junctions by canonicalizing and checking `starts_with(repo_root)`); writes `.gitignore` with `*`; verifies copy completion (file count + byte totals); cleans up a partial backup on any failure (no mutation before success); emits a structured manifest (path, timestamp, file_count, total_bytes).

### WP5 — Graph, plan, design validation/status, Markdown rendering, Git evidence, safe purge

- **Graph**: `validate`/`render`/`status`/`ready`/`wave`/`show` route to the Plan 02 domain (`validation::validate`, `ready_frontier`, `parallel_wave`) and `TomlStore::render`. `render` is deterministic (golden tests); revision-parity of the on-disk view is checked by `graph validate`.
- **Plan lifecycle**: `add`/`show`/`start`/`implemented`/`accept`/`reject` all mutate the graph through `save_with_revision` (lock→reload→recheck revision→mutate→atomic-write→render), preserving revision + optimistic-concurrency semantics. `start` enforces `READY` + hard-predecessors-`ACCEPTED` and the state-machine transition. `accept` enforces `IMPLEMENTED`→`ACCEPTED` and releases `BLOCKED`→`READY` on successors whose hard predecessors are all now accepted. `reject` records `rejection_reason` + `compensating_plan` and transitions `IMPLEMENTED`→`REJECTED`.
- **Design**: `validate`/`status` mark, index, repo escape warnings.
- **Git evidence**: `src/infrastructure/git.rs` exposes read-only `current_branch`/`head_commit`/`is_clean`/`branch_exists`/`is_ancestor`/`detect_stable_branch`/`GitEvidence::collect/repository_root` via **controlled `git` subprocess with explicit args (no `sh -c`)** — never mutating Git.
- **Safe purge**: `workspace close` validates closure; physical purge of `docs/plan/` is a release-closure concern (Plan 08) and is intentionally not implemented as a destructive write here. The workspace close handler reports closability without deleting anything (no `git clean`/`reset`/branch deletion; the design's purge is ownership-marked and gated).

### WP6 — Deterministic human/JSON output, dry-run, structured errors, exit codes

- **Human** (`src/output/human.rs`): concise plain text (`--no-color` is the default and only mode); `--quiet` suppresses success stdout.
- **Exit codes** (`src/output/mod.rs`): the public contract (0/2/3/4/5/6/7/1) with a single `MineError`→exit-code mapping. Partial-success render failures map to **7 (PARTIAL)**; revision/lock conflicts to **5**; validation failures to **4**; gate failures to **3**; usage to **2**; I/O/external to **6**. Edge cases asserted by tests.

### WP7 — Windows paths, legacy namespace conflict, backup failure, external links, workspace identity, release hygiene

Asserted by `tests/cli.rs` (15 tests) and `tests/golden.rs` (4 tests), including the real-graph byte round-trip via `graph validate`/`render`, no-Git-mutation invariant, legacy namespace conflict (exit 3), backup round-trip + `.gitignore`, revision-conflict exit 5, plan transition gates, and stable sorted envelope keys.

## Verification

### Available checks (all pass)

| Command | Exit | Result |
|---|---|---|
| `cargo fmt --all -- --check` | 0 | clean, no diff |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | no warnings |
| `cargo clippy --all-targets --all-features -- -D warnings -W unsafe-code` | 0 | no `unsafe_code` warnings |
| `cargo build --all-targets --all-features` | 0 | builds under `#![forbid(unsafe_code)]` |
| `cargo test --all-targets --all-features` | 0 | 81 lib + 15 cli + 9 domain + 4 golden + 10 init + 9 persistence = **128 passed, 0 failed** |
| End-to-end `cargo run -- mine` (no args) | 2 | usage exit code |
| `cargo run -- mine graph validate --format json` | 0 | stable sorted-key JSON envelope, real graph parses/validates (9 plans) |
| `cargo run -- mine plan start --id 03` (bootstrap-boundary check) | 4 | `MINE_INVALID_TRANSITION` — the CLI refuses to self-start its own already-IN_PROGRESS node |

Final test breakdown:
```
unittests src/lib.rs:           81 passed; 0 failed
unittests src/main.rs:           0 passed; 0 failed
tests/cli.rs:                   15 passed; 0 failed
tests/domain.rs:                 9 passed; 0 failed   (Plan 02 suite, still green)
tests/golden.rs:                 4 passed; 0 failed
tests/init_service.rs:          10 passed; 0 failed   (Plan 01 suite, still green)
tests/persistence.rs:            9 passed; 0 failed   (Plan 02 suite, still green)
```

### Bootstrap-only manual graph checks (the new CLI is exercised but not self-authorizing)

Per the bootstrap boundary in this Plan's execution instructions, the newly-implemented CLI was **exercised against fixtures, temporary repositories, and the real graph for validation** (see `tests/cli.rs`, the smoke runs above, and `graph validate`), but it was **not** used to mark Plan 03 `ACCEPTED`, release Plans 04/05, merge its own branch, or otherwise self-authorize. Plan 03 completion and independent acceptance remain manual bootstrap transitions recorded directly in `docs/plan/execution-graph.toml` + `.md` (the documented bootstrap exception). Only after independent review, merge into `dev`, and post-merge verification does the bootstrap exception end — beginning with Plan 04, execution-graph lifecycle mutations must use the accepted MINE CLI or MCP contract.

### Unavailable bootstrap checks

| Command | Reason |
|---|---|
| `mine design validate` / `mine graph validate` as **released** lifecycle commands | These commands **are** implemented and exercised by this plan (returning the contract exit codes and JSON envelopes above), but their acceptance as the authoritative lifecycle mechanism is deferred until independent review accepts this plan and merges it into `dev`. Until then the bootstrap manual-transition procedure remains in force. |

## Deviations and local decisions

- **`src/lib.rs` / `src/main.rs` / `src/application/mod.rs` / `src/infrastructure/mod.rs` wiring edits** outside the letter of Plan 03's exclusive write paths. Declaring the new `pub mod` entries and wiring the binary entry to `cli::dispatch` is structurally necessary (otherwise the CLI does not compile/run). These are additive, one-line-ish structural edits on top of accepted/closed Plan 01 roots (no ownership conflict), analogous to Plan 01's `.gitattributes` and Plan 02's `src/infrastructure/mod.rs` wiring — flagged for the reviewer. `#![forbid(unsafe_code)]` remains active at both crate roots.
- **`serde_json` dependency added** via `Cargo.toml`/`Cargo.lock`. `Cargo.toml`/`Cargo.lock` are not enumerated in Plan 03's exclusive paths but the JSON envelope contract requires a JSON serializer; the addition is the one manifest change in scope (same class as Plan 02-1's `fs4` deviation, which the reviewer accepted). No other manifest change.
- **Rendered `execution-graph.md` byte-content parity vs revision parity.** The committed `docs/plan/execution-graph.md` was hand-authored by the bootstrap reviewer with extra prose and em-dash/topology-diagram content; the canonical `render_markdown` (Plan 02) produces a simpler ASCII form. The documented contract is **revision** parity (which holds: both advertise revision 6), not byte-for-byte content parity against the hand-authored bootstrap view. The first real `mine graph render` run post-acceptance normalizes the view to the canonical form. The golden test asserts determinism + revision parity, not destructive byte-equality against the existing track view, so tests never mutate the repository's tracked generated view (the render-determinism test operates on a temp copy). Flagged for the reviewer.
- **No shell execution.** Git evidence uses `Command::new("git")` with an explicit argument vector — never `sh -c`. No arbitrary command is executed; only read-only `git` invocations with fixed args.
- **No Git mutation.** No commit, merge, reset, clean, stash, rebase, push, or branch deletion is implemented or performed by the CLI. The `cli_performs_no_git_mutation` test asserts the working-tree clean state is unchanged across read-only commands.
- **`mine doctor`** is implemented (health checks: config, design marker/index, graph, git) returning the `MINE_DOCTOR` code + exit 3 when unhealthy; not separately enumerated in the WP list but within the human-facing core (`mine init`/`status`/`doctor`).

## Contract-preservation summary

| Contract | Evidence |
|---|---|
| human-readable CLI output | `src/output/human.rs`, exercised by smoke runs |
| stable JSON envelopes | `src/output/envelope.rs` (sorted keys, determinism test), error envelope routed to stderr |
| machine-readable error codes | `MineError::code()` (Plan 01/02 variants) + `HandlerError::code` (`MINE_USAGE`/`MINE_DOCTOR`/`MINE_GRAPH_RENDER_PARTIAL`), stable strings in JSON |
| revision and optimistic-concurrency semantics | `TomlStore::save_with_revision` (lock→reload→recheck→mutate→atomic-write→render); revision-conflict exit 5 test |
| deterministic Markdown rendering | `src/render`/`toml_store::render_markdown`; golden tests assert determinism + revision parity |
| Git evidence handling | `src/infrastructure/git.rs` read-only; no-Git-mutation test |
| atomic and locked graph persistence | reused Plan 02 `TomlStore`/`atomic_write`/`file_lock`; no new lock code |
| safe repository-relative paths | domain `normalize_repo_relative` reused in `plan add` |
| no arbitrary shell execution | explicit `Command::new("git")` arg vectors, no `sh -c` |
| no automatic commit/merge/reset/clean/stash/rebase/push/branch-deletion | none implemented; `cli_performs_no_git_mutation` test |

## Remaining risks and external actions

- The independent reviewer must review Plan 03, transition it to `ACCEPTED` (or `REJECTED`), release Plans 04 and 05 to `READY` (they may execute in parallel; Plan 06 is their join gate), and merge the plan branch into `dev`. This is the **final bootstrap Plan**: once accepted, the bootstrap exception ends and lifecycle mutations must use the accepted CLI/MCP.
- `mine mcp serve`, final Skill integration, agent installers, Marketplace packaging, and release automation are later-Plan scope (Plans 04–08), explicitly not implemented here.
- `repo version set` performs a non-atomic config write through `atomic_write` (which is itself atomic); it does not take the graph lock because `.mine/config.toml` is not the graph. A future plan may serialize config writes if concurrent config edits become a concern; not a Plan 03 risk.
- The rendered `execution-graph.md` byte-content-vs-revision-parity tension (see Deviations) should be resolved by the first `mine graph render` invocation post-acceptance, or by a design decision to adopt the simpler canonical form as the contract.

## Toolchain

Unchanged: `rustc 1.97.1`, `cargo 1.97.1`, `rustfmt 1.9.0`, `clippy 0.1.97`, `stable-x86_64-pc-windows-msvc`; VS 2022 MSVC auto-discovered. New dependency: `serde_json 1.0.151` (envelope). The official MSVC toolchain was used; not switched to GNU to hide any platform failure.

## Working-tree state and unrelated changes

The working tree is clean on `plan/03-cli-json-rendering-git-and-workspace-lifecycle` after the five implementation commits; this report is the only remaining file before completion bookkeeping. The pre-existing `.mypy_cache/` remains on disk (gitignored, never deleted). `master` untouched; `dev` not merged; no `plan/04*`/`plan/05*` branch created; nothing pushed (no remotes). The bootstrap boundary — not self-accepting with the newly implemented CLI — was honored throughout.
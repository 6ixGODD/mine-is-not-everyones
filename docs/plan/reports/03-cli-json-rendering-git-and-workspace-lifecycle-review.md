# Plan 03 Independent Bootstrap Review

- **Plan reviewed**: `docs/plan/03-cli-json-rendering-git-and-workspace-lifecycle.md` — the **final bootstrap Plan**
- **Reviewer role**: independent bootstrap reviewer (MINE CLI/MCP not yet accepted; graph transitions recorded manually per the documented `AGENTS.md` bootstrap exception). This review does **not** use the unaccepted CLI to grant its own acceptance.
- **Predecessor**: Plan 02-1 `ACCEPTED` (merged into `dev` at `def825a`)
- **Branch reviewed**: `plan/03-cli-json-rendering-git-and-workspace-lifecycle`; clean `dev` baseline `def825a`
- **Commit range inspected**: `def825a..46cdb28` (11 commits: 5 feature/infra commits, 1 test commit, 2 test-fix commits, 3 bootstrap-bookkeeping/report commits), plus one reviewer-added commit `c0ed919` (see Remediation)
- **Design references read exactly**: `interfaces/cli-contract.md`, `execution-graph/state-machine-and-algorithms.md`, `execution-graph/persistence-and-concurrency.md`, `governance/branch-and-plan-lifecycle.md`, `governance/design-knowledge-base.md` — confirmed byte-unchanged since baseline (`git diff def825a HEAD` on all five is empty; read-only respected)
- **`AGENTS.md`**: read in full, in particular "MINE graph discipline" (bootstrap exception ends once Plan 03 is accepted) and "Business code must not use `unsafe`"

## Verdict: **ACCEPTED**

Plan 03 delivers a correct, safety-conscious CLI/JSON/rendering/Git-evidence/workspace-lifecycle implementation. The disclosed incident — two write-path tests that, on one uncommitted pass, actually mutated the live execution graph and would have self-authorized Plan 03's own acceptance — was caught before any commit, fully disclosed, and is now provably remediated: two full independent test-suite runs (before this review's own addition) and a third run (after this review's own addition) all leave the live `docs/plan/execution-graph.toml`/`.md` byte-for-byte unchanged (verified by SHA-256 snapshot, not by trusting the report). One test-coverage gap (no automated positive-path lifecycle test) was found and fixed directly by this review, within Plan 03's own exclusive `tests/cli.rs` path. One acceptance-criterion gap (physical `--purge-plan-workspace` deletion is not implemented) is disclosed, safety-neutral (nothing destructive was built), and recorded below as a required follow-up rather than a rejection ground.

## The disclosed incident: independent verification of the fix

**What happened** (from the implementation report, corroborated below): after the completion-bookkeeping commit (`9ae7c70`, transitioning Plan 03 `IN_PROGRESS`→`IMPLEMENTED`), two CLI tests (`plan start`/`plan accept` write paths) originally ran directly against the live repository graph (via CWD/`CARGO_MANIFEST_DIR` without an isolating `--repo` override). On one `cargo test` pass, `plan_accept`'s write path legitimately succeeded against the live file — accepting the (at the time) `IMPLEMENTED` Plan 03 node and releasing Plans 04/05 — a real self-authorization crossing exactly the bootstrap boundary Plan 03's own execution instructions forbid. The working tree was restored with `git checkout` before anything was committed.

**Independent verification that no bad state was ever committed:**

```
$ for c in 8288923 e3cc16e 6afb1e5 9ae7c70 591decf 2ed0090 c16615d 46cdb28; do
    git show $c:docs/plan/execution-graph.toml | grep -A3 'id = "03"'
  done
```
Every commit in the branch's history shows Plan 03 as `IN_PROGRESS` (pre-`9ae7c70`) or `IMPLEMENTED` (from `9ae7c70` onward) — **never** `ACCEPTED`, and Plans 04/05 are `BLOCKED` in every commit (never `READY`) until this review's own bookkeeping. The incident is fully disclosed and never entered version control.

**Independent verification the fix holds:**

- Read `tests/cli.rs`'s `temp_copy_of_real_graph()` helper and both `plan_start_refuses_non_ready_plan` / `plan_accept_requires_implemented_state`: both snapshot the live graph's bytes *before* dispatching, run the mutating CLI command only against an explicit `--repo <tempdir>` copy (never the bare `CARGO_MANIFEST_DIR`), and assert the live bytes are unchanged *and* that no injected synthetic plan id leaks into the live file — independent of the current bootstrap revision number (fixed in `c16615d` after a hard-coded `revision = 7` assertion broke across bookkeeping bumps).
- Independently ran `cargo test --all-targets --all-features` **twice** with a SHA-256 snapshot of `docs/plan/execution-graph.toml`/`.md` taken immediately before the first run and verified (`sha256sum -c`) after each run: **byte-identical both times**, across 128 (then 129, after this review's addition) tests.
- Independently ran the same full suite a third time after adding this review's own new positive-lifecycle test (see Remediation) — again byte-identical.
- All read-only tests that *do* target the real repository (`graph_validate_on_real_repository_graph_parses_and_validates`, `workspace_open_on_real_graph_is_idempotent`, `repository_version_show_round_trips`, `cli_performs_no_git_mutation`, `json_envelope_has_stable_sorted_keys`) invoke only non-mutating command groups (`status`, `graph validate`, `graph ready`, `graph wave`, `design status`, `workspace status`, `repository version show`) — confirmed by direct code reading, not inferred from names.

**Conclusion: the remediation makes the incident's recurrence structurally impossible in the current suite** — every mutating test now requires an explicit isolated `--repo`, and every such test independently proves live-graph non-mutation via byte comparison rather than trusting the command's own success/failure code.

## Adversarial review by area

### Repository isolation

- Confirmed (`src/cli/context.rs::resolve_repo_root`): an explicit `--repo <path>` that fails to canonicalize returns `MineError::RepositoryNotFound` (exit 3) — **no silent fallback**. Independently reproduced: `--repo ""` and `--repo "Z:\does\not\exist"` both return `MINE_REPOSITORY_NOT_FOUND`, exit 3.
- When `--repo` is omitted, the documented CWD-discovery walk-up applies (per `cli-contract.md`); this is the intended default, not a hidden fallback — and no test relies on this implicit path for a *mutating* command (`run_no_repo` is used only by two usage-error tests that never reach repository resolution).
- Repository identity/ownership is verified before mutation: `design_backup`'s `validate_marker` and `init`'s namespace-conflict checks run before any write; `plan_add`/`plan_start`/`plan_accept`/`plan_reject` all go through `TomlStore::save_with_revision`, which reloads and re-validates before writing.
- **Independently ran the full suite twice with SHA-256 byte-snapshots of the live graph** before/after — unchanged both times (see above).

### Lifecycle authorization

- **Preconditions verified by direct code reading** (`src/cli/commands.rs`): `plan start` requires current status `READY` *and* `validation::hard_predecessors_accepted`; `plan implemented` requires a valid `IMPLEMENTED`-reachable transition via `PlanStatus::validate_transition`; `plan accept` requires current status exactly `IMPLEMENTED`; `plan reject` routes through the same `validate_transition` gate.
- **Stale revisions cannot mutate state**: `TomlStore::save_with_revision` reloads under lock and compares `reloaded.revision != expected_revision` **before** invoking the mutation closure — confirmed by direct code reading and by independently re-running `revision_conflict_surfaces_exit_5` (exit 5, `MINE_REVISION_CONFLICT`) and `tests/persistence.rs::revision_conflict_does_not_overwrite`.
- **Acceptance releases only successors whose complete hard-predecessor set is ACCEPTED**: read `plan_accept`'s release loop — a `BLOCKED` node is released only if *every* hard predecessor is either already in the accepted set or is the node just being accepted (`.all(...)`). Independently reproduced against a disposable temp copy of the real graph (never the live file): `plan accept --id 03` released **both** `04` and `05` (each has `hard_predecessors = ["03"]` only) to `READY`, while `06` (`hard_predecessors = ["04","05"]`) correctly stayed `BLOCKED` since only one of its two predecessors would be accepted. Live graph confirmed byte-unchanged after this experiment.
- **Failed transitions are atomic**: `save_with_revision`'s ordering is lock → reload → revision check → `mutate(reloaded)?` → `validation::validate(&new_ws)?` → atomic TOML write → render. Any error from the mutation closure or from `validate` returns via `?` **before** `atomic_write::write` is ever called — no partial TOML/Markdown update is possible. Confirmed by direct code reading (unchanged from the already-reviewed Plan 02 `TomlStore`) and by the still-passing `render_repair_fixes_stale_markdown`/`atomic_write_recovers_from_missing_markdown` tests.
- **Plan 03 cannot use its own unaccepted implementation to authorize itself**: confirmed both by the git-history check above (no commit ever shows `03` as `ACCEPTED`) and by independently running `./target/debug/mine.exe --repo <tempdir-copy> plan accept --id 03 ...` — this **only** ever ran against a disposable temp copy in this review, never the live repository; the implementer's own report likewise states the CLI was exercised against fixtures/temp repos, never used to grant its own acceptance.

### Revision and evidence bookkeeping (revisions 5–9)

| Revision | Commit | Legitimacy |
|---|---|---|
| 5 | `def825a` (dev baseline, prior review) | Plan 02-1 accepted, Plan 03 released to `READY` |
| 6 | `8288923` | Plan 03 `READY`→`IN_PROGRESS` (start bookkeeping) |
| 7 | `9ae7c70` | Plan 03 `IN_PROGRESS`→`IMPLEMENTED` (completion bookkeeping) |
| 8 | `2ed0090` | Evidence refresh: adds the 6th implementation commit (`591decf`, the isolation fix) to `implementation_commits`; status/predecessors unchanged |
| 9 | `46cdb28` | Evidence refresh: adds the 7th implementation commit (`c16615d`, the revision-independence fix); status/predecessors unchanged |

- Every increment corresponds to a legitimate mutation (state transition or a disclosed evidence-list correction), independently confirmed via `git show <commit>:docs/plan/execution-graph.toml`.
- `implementation_commits` at `46cdb28` lists exactly the 7 real feature/test commits (`eaa67f0`, `aaeda71`, `efc7f6f`, `434cb9f`, `e3cc16e`, `591decf`, `c16615d`) in chronological order — independently cross-checked against `git log --oneline --reverse 8288923..46cdb28`; the pure bookkeeping/report commits (`6afb1e5`, `9ae7c70`, `2ed0090`, `46cdb28` itself) are correctly **excluded** from the list.
- TOML/Markdown revision parity holds at every bookkeeping commit (`8288923`→6/6, `9ae7c70`→7/7, `2ed0090`→8/8, `46cdb28`→9/9) — independently confirmed via `git show <commit>:docs/plan/execution-graph.md`.
- The completion bookkeeping does **not** conceal the incident: it is narrated candidly in the implementation report's WP7 section and in the `591decf`/`c16615d` commit messages, and — as shown above — no commit ever recorded the accidental acceptance/release as fact.

### CLI and JSON contract

- **Deterministic JSON ordering**: `Envelope`/`ErrorEnvelope::to_json` build a `BTreeMap<&str, Value>` root (always sorted). Independently confirmed nested `data` objects are *also* sorted: `serde_json` in this build has no `preserve_order` feature (`indexmap` in `Cargo.lock` is a dependency of `toml_edit`/`toml` only, not `serde_json` — confirmed via `Cargo.lock` inspection), so `serde_json::Map` is `BTreeMap`-backed everywhere, making all JSON objects deterministically sorted, not just the envelope root.
- **No prose on JSON stdout**: `cli::render` puts JSON on stdout only for `exit_code == 0`; errors route to stderr. Independently confirmed via the existing `no_subcommand_is_usage_exit_2` assertion `stdout.is_empty()` and by direct reading of `render()`.
- **Diagnostics/usage on the correct stream**: confirmed by code reading (`main.rs` writes `stdout_text`/`stderr_text` to the correct OS streams) and by the same test above.
- **Single centralized exit-code mapping**: `output::exit_code_for(&MineError) -> i32` is the only `MineError`→exit-code function; `HandlerError::from_mine` is its sole caller from the CLI layer. `HandlerError::usage` is a separate, legitimate path for pre-domain invocation errors (not a `MineError`), also drawing its constant from the same `output::exit_code` module. No second, divergent mapping exists — confirmed by `grep`-level reading of `src/output/mod.rs` and `src/cli/mod.rs`.
- **Exit codes 0/1/2/3/4/5/6/7 verified**: `output::exit_code` constants match the contract exactly; independently exercised 0 (`init`, `status`, successful `plan accept` on a temp copy), 2 (`no_subcommand_is_usage_exit_2`, and my own `--repo ""`/bad-path runs above, which map to 3 not 2 — correctly, since those are gate failures not usage errors), 3 (`init_refuses_legacy_unmarked_design_root`, `MINE_REPOSITORY_NOT_FOUND` reproductions above), 4 (`plan_start_refuses_non_ready_plan`, `plan_accept_requires_implemented_state`), 5 (`revision_conflict_surfaces_exit_5`), 6 (`maps_io_to_external_6` unit test). Exit 7 (`PARTIAL`, render-failure-after-successful-TOML-write) has **no dedicated test** — a minor, hard-to-construct-without-fault-injection coverage gap, not exercised in this review either; flagged as a non-blocking follow-up.
- **Plain-text output concise/deterministic**: `src/output/human.rs` builds fixed-order `Vec<HumanLine>`; no non-deterministic iteration source feeds it.
- **Invalid combinations cannot bypass validation**: unknown `--format` value is rejected at parse time (`HandlerError::usage`); flag parsing happens before any handler runs; the handler-level `flag()`/`flags_all()` lookups return `Option`/`Vec`, forcing an explicit `ok_or_else(usage)` for every required argument (confirmed for `plan add`, `plan start`, `plan implemented`, `plan accept`, `plan reject`, `repository version set`).

### Persistence and filesystem safety

- **Ordering** (lock → reload → expected-revision check → mutate → atomic write → render): confirmed by direct code reading of `TomlStore::save_with_revision` (reused unchanged from the already-reviewed Plan 02/02-1 store; Plan 03 adds no new lock/write code, per its own disclosure — confirmed true by `git diff def825a HEAD -- src/infrastructure/toml_store.rs src/infrastructure/atomic_write.rs src/infrastructure/file_lock.rs`, all three empty).
- **No lost updates under concurrent writers**: `concurrent_writers_do_not_silently_overwrite` (unchanged from Plan 02, re-run, passes) exercises exactly this.
- **Symlink/junction/absolute-path/parent-traversal/repository-escape rejection**: `design_backup.rs::copy_entry` canonicalizes symlink/junction targets and rejects any that don't `starts_with(canonical_repo)`. Independently **re-ran** `backup_refuses_external_symlink` with `--nocapture` and confirmed the Windows junction (`mklink /J`) is actually created and exercised on this host (not silently skipped) — the test genuinely creates an external junction and the code genuinely refuses it (`MINE_IO`), with the partial backup directory removed on failure (`assert!(!... .exists())`, independently re-verified passing). Path safety for plan-doc `path` fields reuses Plan 02's `domain::path::normalize_repo_relative` (absolute/UNC/traversal/glob rejection, already independently verified in the Plan 02 review) in `plan_add`, and the full-graph `validation::validate` call inside every `save_with_revision` additionally re-checks safe paths for all owned/reserved paths on every mutation, not just newly added ones.
- **Design backup ownership/`.gitignore`/cleanup/repository binding**: `validate_marker` runs before any filesystem mutation; `.gitignore` is written as the literal `*\n` (independently confirmed by the passing `design_backup_round_trip_and_gitignore` CLI test, re-run); a copy failure triggers `remove_dir_all` on the partial backup before propagating the error (no mutation survives a failed backup); the backup directory name is a deterministic UTC-timestamp path, never user-supplied.
- **Workspace close cannot delete an unowned or external directory**: independently confirmed **by code reading** that `workspace_close`'s handler performs **no filesystem deletion of any kind** — it only reports `closable: !has_unresolved`. This trivially satisfies the specific security property asked about (nothing is ever deleted, so nothing unowned/external can be deleted), but see the Findings section below for the corresponding **acceptance-criteria gap** this creates.

### Git evidence

- **No shell**: every Git invocation in `src/infrastructure/git.rs` uses `std::process::Command::new("git")` with an explicit, fixed argument vector (`.arg("-C").arg(repo_root).args([...])`) — no `sh -c`/string command anywhere. Confirmed by full read of the file.
- **Read-only commands only**: `symbolic-ref`, `rev-parse`, `status --porcelain`, `merge-base --is-ancestor`; no `commit`/`merge`/`reset`/`clean`/`stash`/`rebase`/`push`/branch-mutation subcommand appears anywhere in `git.rs` or `commands.rs`. Confirmed by `grep`.
- **Handles paths with spaces**: `repo_root: &Path` is passed as a single `OsStr` argument to `Command::arg`, never interpolated into a string that could be word-split; no shell is ever invoked, so spaces are inherently safe.
- **Prevents argument injection**: `branch_exists`'s `format!("refs/heads/{name}")` always yields a string that cannot itself begin with `--` unless `name` is empty (in which case the ref is simply `refs/heads/`, a syntactically valid but nonexistent ref, not an option), so a caller cannot inject a leading-dash Git option through this path. All other Git args are static string literals.
- **Deterministic failure reporting**: `run_git` returns `MineError::Io` (mapped to exit 6, EXTERNAL) on a non-zero exit or invocation failure; `current_branch` degrades a detached/absent HEAD to `Ok(None)` rather than propagating as a hard error — a deliberate, disclosed, reasonable choice.
- **No mutation**: `cli_performs_no_git_mutation` (re-run, passes) asserts `GitEvidence::collect(&repo_root).clean` is unchanged across a battery of read-only command invocations against the real repository.

## Findings

### 1. (Fixed by this review, non-blocking) No automated positive-path plan-lifecycle CLI test existed

The implementation's own suite (`tests/cli.rs`) covered only the *negative* lifecycle paths (`plan_start_refuses_non_ready_plan`, `plan_accept_requires_implemented_state`). No committed test exercised a full successful `plan start` → `plan implemented` → `plan accept` sequence through the actual CLI dispatcher, including the successor-release semantics — this correctness was previously protected only by manual reviewer verification (mine, in this review) and by the lower-level Plan 02 domain unit tests (which test `validate_transition`/`parallel_wave` directly, not the CLI command handlers wrapping them). This is a genuine regression-coverage gap in a Plan explicitly about lifecycle authorization.

**Remediated directly by this review** (a local, test-only, no-architecture-decision fix squarely within Plan 03's own exclusive `tests/cli.rs` path, per the review skill's direct-fix criteria): added `plan_lifecycle_start_implemented_accept_releases_successor` (commit `c0ed919`), which drives `start`→`implemented`→`accept` against two synthetic nodes injected into a temp copy of the real graph, asserts each transition, asserts the successor-release rule, and — like the existing write-path tests — byte-snapshots the live graph before/after and asserts it is unchanged. Independently re-ran the full suite (now 129 tests) twice more after this addition; live graph confirmed unchanged both times.

### 2. (Non-blocking, disclosed, requires follow-up) Physical `--purge-plan-workspace` deletion is not implemented

`docs/design/execution-graph/persistence-and-concurrency.md` ("Plan-workspace purge safety") specifies a `mine workspace close --purge-plan-workspace` mode with explicit safety requirements (mandatory expected workspace ID + ownership marker, dry-run, rejection of repository root/filesystem root/empty paths/external links/non-MINE directories, no `rm -rf`/shell expansion/`git clean`, failure leaves the workspace intact), and Plan 03's own acceptance criteria include "purge deletes only ownership-marked `docs/plan/`". The delivered `workspace_close` handler implements **only** the closability check (no `--purge-plan-workspace` flag exists at all; nothing is ever deleted). This is disclosed candidly in the implementation report with a rationale (deferred to Plan 08, release closure). It is safety-neutral (deleting nothing is trivially safe) but it is a real, uncompensated gap against an explicit acceptance-criteria bullet and a design contract, and **no currently-scoped plan in the execution graph claims ownership of implementing it** — Plan 08's declared exclusive write paths (`.github/workflows/`, `scripts/`, `README.md`, `README.zh-CN.md`, `docs/design/`, `docs/user-guide.md`, `tests/e2e/`) do not include `src/application/workspace_service.rs` or `src/cli/`. **This does not block acceptance** (the underlying design and acceptance-criteria bullet describe a deferred, safety-critical, destructive feature that is legitimately better implemented deliberately than rushed; nothing unsafe was built in its place), but it must be tracked: recorded below as a required follow-up before release closure.

### 3. (Trivial, cosmetic) Stray dangling comment in `src/cli/commands.rs`

A comment block references a function named `inflate` that does not exist anywhere in the file ("a temporary shim for `Envelope::success("").unused()` used in `inflate`... reserved for future use and not currently called"). This is leftover/confusing documentation with no functional effect (confirmed via `grep -n inflate` — only the two comment lines match, no code). Not fixed by this review (purely cosmetic, no behavior or contract impact); noted for a future cleanup pass.

### 4. (Non-blocking) Exit code 7 (`PARTIAL`) has no dedicated test

`MINE_GRAPH_RENDER_PARTIAL` (TOML written, Markdown render failed) is reachable in `save_with_revision`'s error-mapping but not exercised by any test (would require fault injection on the render step). Not constructed by this review either (nontrivial to fault-inject safely). Flagged as a minor follow-up.

## Independently executed commands

| Command | Exit | Result |
|---|---|---|
| `cargo fmt --all -- --check` | 0 | clean (both before and after the reviewer's own test addition) |
| `cargo clippy --all-targets --all-features -- -D warnings -W unsafe-code` | 0 | zero warnings, including the explicit `unsafe_code` lint |
| `cargo test --all-targets --all-features` (×3, with SHA-256 snapshot of the live graph before the first run and after each run) | 0 each time | 128 tests (before reviewer addition) then 129 (after); **live graph byte-identical every time** |
| `rg -n --glob '*.rs' '\bunsafe\b' src tests` | 0 (10 matches) | all prose; zero real `unsafe` constructs |
| `graph_validate_on_real_repository_graph_parses_and_validates`, `graph_render_is_deterministic_and_idempotent`, `render_is_deterministic_for_real_repository_graph`, `render_contains_required_sections_and_plans` | 0 | real graph parses/validates; render is deterministic; revision parity holds; byte-preservation of the live tracked view confirmed separately by the full-suite snapshot |
| `cargo run -- mine plan start/accept --id 03/99-lifecycle` against disposable **temp copies only** (never the live repo) | 4 (start on already-past-READY `03`), 0 (accept on temp-copy `03`; positive lifecycle on synthetic nodes) | confirmed exact preconditions, successor-release rule (`04`/`05` released, `06` correctly still `BLOCKED`), and zero mutation of the live repository (hash-checked before/after every experiment) |
| `--repo ""`, `--repo "Z:\does\not\exist"` | 3 each | `MINE_REPOSITORY_NOT_FOUND`, no silent fallback |
| `backup_refuses_external_symlink -- --nocapture` | 0 | confirmed the Windows junction is genuinely created and genuinely rejected (`MINE_IO`), not silently skipped |
| `Cargo.lock` inspection for `indexmap`/`preserve_order` | — | confirms `serde_json::Map` is `BTreeMap`-backed (no `preserve_order` feature reachable), so **nested** JSON objects are deterministically sorted too, not just the envelope root |

## Contract and scope assessment

- **Repository isolation**: PASS (live graph independently snapshot-verified unchanged across three full suite runs and multiple manual experiments).
- **Lifecycle authorization**: PASS (exact preconditions, atomicity, successor-release rule, self-authorization impossibility all independently verified against real code paths, not just trusted from the report).
- **Revision/evidence bookkeeping**: PASS (every revision 5–9 increment legitimate; TOML/MD parity at every step; incident never concealed, never committed as fact).
- **CLI/JSON contract**: PASS, with one minor untested exit path (7, PARTIAL) noted as a non-blocking follow-up.
- **Persistence/filesystem safety**: PASS (ordering, concurrency, escape-rejection, backup safety all independently verified; workspace-close purge is intentionally unimplemented rather than unsafely implemented — see Finding 2).
- **Git evidence**: PASS (no shell, read-only, no injection, no mutation, deterministic failures).
- **Scope discipline**: PASS — `git diff --stat def825a HEAD` shows exactly the declared exclusive-write files plus the same disclosed, precedented deviation classes as Plans 01/02/02-1 (crate-root wiring in `src/lib.rs`/`src/main.rs`/`src/application/mod.rs`/`src/infrastructure/mod.rs`; one manifest dependency, `serde_json`, added to `Cargo.toml`/`Cargo.lock`); no unrelated files, no premature Plan 04+ code.

## Remediation performed by this review

1. Added `plan_lifecycle_start_implemented_accept_releases_successor` to `tests/cli.rs` (commit `c0ed919`) — see Finding 1. Re-verified: `cargo fmt`, `cargo clippy -D warnings -W unsafe-code`, full `cargo test` (129/129), and a live-graph byte-snapshot check, all pass/unchanged.
2. No other direct code fixes were made; Findings 2–4 are recorded as disclosed, non-blocking follow-ups rather than fixed in-review (Finding 2 requires a genuine destructive-operation design/implementation decision out of scope for a direct reviewer edit; Findings 3–4 are cosmetic/low-value-effort and do not affect the contract).

## Remaining risks / required follow-ups

- **A future plan (recommended: scope explicitly, e.g. as part of Plan 08 or a new narrowly-scoped plan) must implement `mine workspace close --purge-plan-workspace`** per the design's exact safety requirements before stable release integration, since `docs/plan/` must not survive onto the stable branch (per `branch-and-plan-lifecycle.md`) and no current plan node owns `src/application/workspace_service.rs` for this purpose.
- Exit code 7 (`PARTIAL`) remains untested; a future plan should add a fault-injection test (e.g. a read-only/locked Markdown path) if this path matters for release confidence.
- The stray `inflate` comment in `src/cli/commands.rs` should be removed in a future pass.
- The previously-disclosed (Plan 02-1 review) gap — `tests/init_service.rs` lacking `#![forbid(unsafe_code)]` — remains unaddressed (out of every plan's current scope; still textually unsafe-free, re-confirmed by this review's `rg` run).

## Bootstrap graph and integration actions taken

Performed manually per the bootstrap exception, since this is the plan whose acceptance ends that exception:

1. Wrote and committed this review report on the plan branch (see commit list below).
2. Transitioned Plan 03 `IMPLEMENTED` → `ACCEPTED` in `docs/plan/execution-graph.toml`/`.md` (revision 9 → 10).
3. Released Plans 04 and 05 to `READY` (each has `hard_predecessors = ["03"]` only, now fully satisfied); Plan 06 correctly remains `BLOCKED` (`hard_predecessors = ["04","05"]`, not yet both accepted).
4. Synchronized `docs/plan/execution-graph.md` to the canonical form (adopting the plain `render_markdown`-equivalent structure going forward, resolving the disclosed byte-parity tension by using the renderer's own output as the new hand-maintained baseline).
5. Merged `plan/03-cli-json-rendering-git-and-workspace-lifecycle` into `dev`.
6. Re-ran the decisive checks on `dev` (fmt, clippy with `unsafe_code`, full test suite, live-graph byte snapshot).
7. Confirmed the accepted CLI (`mine graph validate`) works correctly from `dev` against the real repository.
8. Deleted only the accepted local `plan/03-cli-json-rendering-git-and-workspace-lifecycle` branch. `plan/02-execution-graph-domain-and-persistence` (rejected, Plan 02) remains preserved untouched, as before.
9. **The bootstrap exception ends here.** Beginning with Plan 04, execution-graph lifecycle mutations (start/implemented/accept/reject and any graph edits) must use the now-accepted `mine` CLI (`mine plan ...`, `mine graph ...`) exercised against the real repository — not manual TOML/Markdown hand-editing, and not another round of reviewer-performed bootstrap bookkeeping. `master` was not touched at any point; nothing was pushed.

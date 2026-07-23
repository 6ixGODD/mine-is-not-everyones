# Execution Graph Persistence and Concurrency

## Machine fact source

During an active plan workspace:

```text
docs/plan/execution-graph.toml
```

Generated view:

```text
docs/plan/execution-graph.md
```

Markdown is never a mutation target.

## Why TOML

TOML is readable, diffable, branch-aware, and simple to parse. SQLite and binary formats are rejected because the graph is small, workspace-scoped, and Git-reviewable.

## Local MINE directory

```text
.mine/
├── config.toml
├── runtime/
│   ├── events.jsonl
│   ├── mine.log
│   └── sync/
└── locks/
    └── execution-graph.lock
```

The graph never moves into `.mine/`.

## Revision and locking

Every successful mutation increments revision exactly once. Mutations accept `expected_revision`; mismatch returns `MINE_REVISION_CONFLICT` without writing.

Writers acquire an exclusive lock, reload state, recheck revision, write atomically, then render Markdown from committed TOML.

The exclusive lock is a cross-platform (POSIX and Windows) advisory file lock. Per `AGENTS.md` ("Business code must not use `unsafe`"), `mine` core Rust code (`src/`) must implement this lock through a maintained, vetted external locking crate (for example `fs4`, the maintained successor to `fs2`, which wraps the platform lock APIs — POSIX `flock`/`fcntl` and Windows `LockFileEx` — behind a safe `std::fs::File` extension trait) rather than hand-written `unsafe extern` FFI blocks in this repository's own source. Any `unsafe` needed to call the platform lock API must live inside the vetted dependency, not inside `mine`'s own crate. Adding this dependency is a routine `Cargo.toml`/`Cargo.lock` change and does not require a design exception.

## Plan-workspace purge safety

`mine workspace close --purge-plan-workspace` may delete only canonical repository-relative `docs/plan/` after all gates pass.

- expected workspace ID and ownership marker are mandatory;
- dry-run is available and used by Skills before mutation;
- repository root, filesystem root, empty paths, external links, and non-MINE directories are rejected;
- no `rm -rf`, shell expansion, `git clean`, or broad deletion is used;
- failure leaves the workspace intact.

## Design backup is separate

Timestamped `docs/design-backup-*` directories are local recovery artifacts created by `mine design backup`. They are not execution-graph state, are never included in stable release, and are not automatically purged until design validation succeeds and the managing agent confirms they are no longer needed.

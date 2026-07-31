# Design Knowledge-Base Structure

## Purpose

`docs/design/` is a navigable, MINE-owned knowledge base, not a single architecture manuscript and not a shared namespace for arbitrary legacy documentation.

## Ownership marker

A managed design root contains:

```text
docs/design/.mine-design.toml
```

Minimum content:

```toml
schema_version = 1
managed_by = "MINE"
repository_id = "<stable repository UUID>"
created_at = "<UTC timestamp>"
```

`mine init` behavior:

- absent `docs/design/`: create scaffold and marker;
- valid matching marker: preserve and validate;
- present directory without marker: fail with `MINE_DESIGN_NAMESPACE_CONFLICT`;
- marker with another repository ID: fail with `MINE_DESIGN_OWNERSHIP_MISMATCH`.

`mine init` does not adopt or migrate arbitrary existing content. When it encounters an unmarked `docs/design/`, it moves the legacy directory aside to a timestamped `docs/design-backup-<UTC timestamp>/` backup and creates a fresh managed root; it does not abort. `mine-sync` refuses an unmarked or foreign-owned `docs/design/` only after `mine init` has established the managed namespace. MINE never guesses whether old documents are authoritative, compatible, or safe to overwrite.

## Hierarchical indexing

Every design area owns an `index.md`.

```text
docs/design/
├── .mine-design.toml
├── index.md
└── database/
    ├── index.md
    ├── primary/
    │   ├── index.md
    │   ├── schema.md
    │   ├── tables/
    │   │   ├── users.md
    │   │   └── orders.md
    │   └── migrations.md
    └── analytics/
        ├── index.md
        └── datasets.md
```

The root links to domain indexes; domain indexes link to component indexes; leaves contain exact contracts.

## Local backup convention

Before rewriting existing managed design, `mine-sync` creates:

```text
docs/design-backup-YYYYMMDDTHHMMSSZ/
```

The backup contains a byte-for-byte logical copy of the managed design tree plus:

```text
docs/design-backup-.../.gitignore
```

with:

```gitignore
*
```

The backup is local recovery material:

- it is never part of durable design;
- it must not be committed;
- release validation rejects tracked `docs/design-backup-*` paths;
- it may be removed only after the user or release agent confirms the new design is valid;
- copying must not dereference links that leave the repository.

## Leaf document contract

Each leaf should include:

- purpose and scope;
- governing decisions and invariants;
- interfaces or data contracts;
- lifecycle and ownership;
- failure behavior;
- security and operational considerations;
- tests or verification expectations;
- links to related documents;
- status and last verified version when useful.

## Avoiding duplication

An index may summarize a child in one or two sentences but does not duplicate its full contract. A contract has one authoritative leaf.

## Change rules

- `mine-sync` updates descriptive design from current code using the configured authority order.
- `mine-arch` updates target design before plan creation.
- Every plan points to exact design paths and anchors.
- Accepted implementation details are incorporated before release.
- Obsolete design is deleted or rewritten directly unless the user explicitly protects it.
- A stale design document is not preserved merely because it existed.

## Context-budget rules

Skills do not recursively read the entire tree by default. They begin at indexes, identify affected areas, and load only relevant leaves and direct dependencies.

An unscoped `mine-sync` is the deliberate exception: it may progressively explore the entire repository and design tree. The sync report must state what was fully inspected, sampled, inferred, or left uncertain.

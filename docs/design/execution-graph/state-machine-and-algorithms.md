# Execution Graph State Machine and Algorithms

## States

```text
DRAFT
BLOCKED
READY
IN_PROGRESS
IMPLEMENTED
ACCEPTED
REJECTED
```

## Allowed transitions

| From | To | Gate |
|---|---|---|
| DRAFT | BLOCKED | Plan registered but prerequisites or design gates unresolved |
| DRAFT | READY | Plan complete and all hard predecessors accepted |
| BLOCKED | READY | All blocking conditions resolved |
| READY | IN_PROGRESS | Successful start, owner assigned, revision matches |
| IN_PROGRESS | IMPLEMENTED | Report and commit evidence registered |
| IMPLEMENTED | ACCEPTED | Independent review passes every hard contract |
| IMPLEMENTED | REJECTED | Independent review finds material failure |

`REJECTED` is **terminal**. A rejected plan is never revived, restarted, or
re-statused; repository evidence confirms this (the bootstrap-era rejected
Plan `02` stayed `REJECTED` after its compensating Plan `02-1` was accepted).
The earlier `REJECTED -> BLOCKED` row is removed as misleading historical
baggage: it implied a status transition that no operation performs.

Closing a rejection has two independent parts, both performed through the
accepted CLI; neither changes the rejected plan's status:

1. **Register the compensating plan** with `mine plan add` (hard predecessor
   set to the rejected plan's accepted predecessor, not the rejected plan).
2. **Rewire downstream successors** off the rejected plan onto the
   compensating plan with `mine plan rewire-compensation` (see
   [Compensation rewiring](#compensation-rewiring) below).

No generic `set-status` exists.

## Validation

Validation includes unique IDs/paths, valid predecessors, acyclic hard dependencies, legal states, required files/evidence, valid design anchors, safe paths, generated-view revision parity, and branch/workspace consistency.

## Derived readiness

A node is ready only when every hard predecessor is accepted, design references are valid, no material decision remains, path ownership does not conflict, and stored state allows readiness.

## Compensation rewiring

`mine plan rewire-compensation --id <rejected-id>` is a deterministic,
first-class graph operation that replaces an explicitly rejected predecessor
with its registered compensating plan in every still-mutable downstream
successor. It is the only supported way to reroute downstream dependencies
after the bootstrap exception ended; manual editing of the graph files is
forbidden (`AGENTS.md`).

### Anchors and inputs

- The single input is the rejected plan's id. The replacement plan id is
  **derived** from the rejected plan's `compensating_plan` field — the single
  source of truth registered at reject time. The caller never supplies a
  replacement id, so dependency substitution can never be driven by a
  similar-looking id (no fuzzy/substring matching).

### Preconditions (all checked under the graph lock, atomically)

1. The original plan exists and its status is exactly `REJECTED`.
2. Its `compensating_plan` is non-empty and names an existing plan (the
   replacement). The replacement must not itself be `REJECTED`.
3. Every successor that lists the rejected id in either hard or soft
   predecessors must be in a still-mutable status: `DRAFT`, `BLOCKED`, or
   `READY`. Any successor in `IN_PROGRESS`, `IMPLEMENTED`, `ACCEPTED`, or
   `REJECTED` fails the operation with a stable error and rewires nothing.
4. The proposed rewired graph must remain acyclic (reuses `topological_sort`).
5. The graph's current revision must match the caller-supplied expectation
   (optimistic-concurrency), per the shared `save_with_revision` transaction.

### Mutation

- For each still-mutable successor, every exact occurrence of the rejected
  id in `hard_predecessors` and `soft_predecessors` is replaced by the
  compensating id in place. Predecessor order is preserved; no edges are
  added, removed, or reordered; unrelated predecessors and successors are
  untouched.
- The successor's `updated_at` is refreshed; the rejected plan's fields are
  not modified.
- Persistence and rendering are atomic via the shared
  `TomlStore::save_with_revision` path: lock -> reload -> revision check ->
  semantic checks above -> in-place replacement -> atomic TOML write ->
  deterministic Markdown render -> release lock. From the caller's
  perspective the graph mutation and its generated view change atomically.

### Idempotency

Re-running a completed rewiring is **safe idempotent success**: if no
successor still references the rejected id, the operation performs a
read-only load, writes nothing, bumps no revision, and returns
`revision_before == revision_after` with `affected_successors: []`. A
re-run that finds newly-added successors still referencing the rejected id
rewires them normally (a real mutation, revision bumps). A precise stable
error is reserved for genuine invalid state (not-rejected original,
missing/compensating-plan mismatch, locked successor, cycle), never for the
no-work case.

### Result

The JSON envelope reports deterministically:

- `command`: `"plan.rewire-compensation"`;
- `revision_before` / `revision_after` (equal on the idempotent no-op);
- `data.rejected_plan`, `data.compensating_plan`, and
  `data.affected_successors` (ids of successors whose predecessor lists
  changed, in stable plan-insertion order).

### Stability errors

- `MINE_PLAN_NOT_FOUND`: original or compensating plan missing.
- `MINE_INVALID_TRANSITION`: original is not `REJECTED`.
- `MINE_GRAPH_INVALID`: `compensating_plan` empty, replacement is `REJECTED`,
  or successors already rewired is not raised (see idempotency).
  `MINE_REWIRE_SUCCESSOR_LOCKED`: a referencing successor is in
  `IN_PROGRESS`/`IMPLEMENTED`/`ACCEPTED`/`REJECTED` (details name it).
- `MINE_GRAPH_CYCLE`: rewiring would create a hard-dependency cycle.
- `MINE_REVISION_CONFLICT` / `MINE_LOCK_TIMEOUT`: shared transactional checks.

No arbitrary graph-editing API is introduced: rewiring only substitutes a
rejected id with its declared compensating id in mutable successors' exact
predecessor entries, behind all validation above.

## Parallel wave

A wave is a stable maximal set of ready plans without ancestor relationships, exclusive-write overlap, exclusive/reserved overlap, or active conflicts.

## Workspace closure gate

Closure requires:

- every terminal implementation node accepted;
- no unresolved draft, blocked, ready, in-progress, implemented, or rejected node;
- final full `mine-sync` attestation not blocked;
- release verification passes;
- current branch is managed `dev`;
- working tree has no unclassified changes;
- expected workspace ID matches the ownership marker.

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
| REJECTED | BLOCKED | Compensation registered and downstream rerouted |

No generic `set-status` exists.

## Derived readiness

A node is ready only when every hard predecessor is accepted, design references are valid, no material decision remains, path ownership does not conflict, and stored state allows readiness.

## Validation

Validation includes unique IDs/paths, valid predecessors, acyclic hard dependencies, legal states, required files/evidence, valid design anchors, safe paths, generated-view revision parity, and branch/workspace consistency.

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

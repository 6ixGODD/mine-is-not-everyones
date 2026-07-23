# Ephemeral Plan Workspace

> This directory exists only on managed `dev` and `plan/*` branches. It must not enter the stable release tree or imported stable history.

The workspace contains immutable implementation plans, the machine execution graph, generated graph view, parallel coordination, and temporary reports.

## Lifecycle

1. created automatically by `mine-plan-create` on `dev`;
2. assigned an internal generated workspace ID, not a user-supplied version;
3. modified and reviewed during development;
4. used by final `mine-sync` and release validation;
5. safely purged before stable integration;
6. omitted from stable history through squash or curated commits.

## Immutability

A plan becomes immutable when execution starts. Rejected implementation is repaired through a compensating plan.

## Design references

Every plan cites exact `docs/design/` leaves and anchors. Missing or stale references block readiness.

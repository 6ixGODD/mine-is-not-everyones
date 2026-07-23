# MINE Parallel Execution Protocol

## Scope

This protocol applies to plans in the same graph wave or explicitly parallel lanes inside one plan.

## Branches and workspaces

- Each plan normally uses `plan/<id>-<slug>` from the accepted current `dev` baseline.
- A MINE Skill may create and switch its managed branch under standing repository authorization.
- A scheduler may prepare independent worktrees; v1 does not require the Rust CLI to create them.
- Agents never implement directly on the stable branch or `dev`.
- An implementation agent does not merge itself.
- An independent review agent may merge an accepted plan branch into `dev`, verify the integration, and delete the accepted local plan branch.

## Ownership

Each plan declares exclusive write paths, read-only paths, reserved shared paths, and integration requests.

Shared root configuration, lockfiles, generated registries, schemas, and release manifests have one serialized owner.

## State

Every executor starts through MINE with an expected revision. A timed-out session does not authorize a second executor to start the same plan; inspect graph state and the existing managed branch first.

## Git safety

Standing authorization covers managed `dev` and `plan/*` branch creation, switching, scoped commits, accepted merge into `dev`, and deletion of accepted local plan branches.

It does not cover force push, `reset --hard`, `git clean`, blind stash, unrelated branch deletion, public-history rewriting, or discarding unrelated changes.

Use explicit path staging. Never use broad staging in a dirty shared workspace.

## Join gate

Parallel plans join only when:

- each required plan is independently accepted;
- integration requests are resolved by the declared owner;
- shared artifacts are regenerated once;
- combined verification passes;
- durable design remains consistent.

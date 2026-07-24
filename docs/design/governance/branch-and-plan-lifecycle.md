# Branch and Plan Lifecycle

## Branch roles

### Stable branch (`main` or `master`)

- detected and recorded by `mine init`;
- contains stable product state, configuration, root README files, and `docs/design/`;
- must not contain `docs/plan/` or tracked `docs/design-backup-*` paths;
- direct plan implementation is forbidden.

### `dev`

- temporary integration branch for the active body of work;
- created from the latest accepted stable baseline by an authorized MINE Skill;
- owns the active `docs/plan/` workspace;
- receives independently accepted plan branches;
- is deleted after stable release integration.

### `plan/<id>-<slug>`

- short-lived implementation/review branch based on accepted `dev`;
- owns one plan unless an explicit parallel-lane design says otherwise;
- is merged into `dev` only after independent acceptance;
- is deleted after successful integration.

## Standing Git authorization

By invoking a MINE Skill, the repository owner grants the active agent authority to:

- inspect Git state and remote/default-branch metadata;
- create and switch the MINE-managed `dev` and `plan/*` branches;
- commit explicit files owned by the current operation;
- merge an accepted plan branch into `dev`;
- delete an accepted and merged local `plan/*` branch;
- perform final squash or curated integration into the stable branch after all release gates pass;
- create a release tag when configured;
- delete the local managed `dev` branch after release.

The authorization excludes:

- unrelated or unknown branches;
- force push;
- `reset --hard`;
- `git clean`;
- blind stash;
- rewriting public/shared history;
- discarding unrelated work;
- deleting remote branches unless the user explicitly requests it.

Dirty or ambiguous worktree state blocks branch mutation until safely classified.

## Repository initialization

`mine init` does not create `dev`, `plan/*`, or `docs/plan/`. It establishes configuration, the design namespace, governance, templates, and integrations only.

The first model-driven action is invoked explicitly by the user:

- `mine-arch` for requirement-first design;
- `mine-sync` for code-first onboarding or reconciliation.

## Compensation and downstream rewiring

When a plan is rejected (`mine plan reject --compensating-plan <id>`), closing
the rejection has two independent, CLI-managed steps. **Neither edits the
execution graph by hand**; the bootstrap exception that allowed manual
rerouting (e.g. Plan `02` -> `02-1` during bootstrap) has ended.

1. **Register the compensating plan** with `mine plan add`, whose hard
   predecessor is the rejected plan's accepted upstream (not the rejected
   plan itself).
2. **Rewire downstream successors** off the rejected plan onto the
   compensating plan with
   `mine plan rewire-compensation --id <rejected-plan-id>`
   (see
   `docs/design/execution-graph/state-machine-and-algorithms.md#compensation-rewiring`).
   The replacement is derived from the rejected plan's `compensating_plan`
   field; the caller never supplies a replacement, so no similar-id
   substitution can occur.

A rejected plan is terminal; it is not revived or re-statused. Accepted and
active successors (`IN_PROGRESS`/`IMPLEMENTED`/`ACCEPTED`/`REJECTED`) are
never rewritten by compensation rewiring, preserving their immutability.
Downstream work that depended on the rejected plan may only resume after the
rewiring is accepted into `dev` and the compensating plan is itself accepted.

## Plan workspace creation

`mine-plan-create` ensures `dev` exists, switches to it, and opens an internal plan workspace when absent.

The workspace has a generated `workspace_id`; the user does not supply a workspace version. The workspace marker records repository ID, stable baseline commit, integration branch, creation time, and MINE ownership.

## During development

- Design changes precede plans.
- Plan files are immutable after execution starts.
- Reports and graph state are temporary but committed on managed development branches for reviewability.
- `docs/design/` on `dev` describes approved target behavior.
- Every plan references exact design paths and anchors.
- Execution agents commit scoped implementation but never self-accept.
- Review agents may merge accepted work into `dev` and remove the local accepted plan branch.

## Release closure

1. every plan is accepted and integrated into `dev`;
2. run a full `mine-sync` against the complete `dev` tree;
3. resolve all blocking uncertainty and update durable design;
4. run product, release, and actual client-discovery verification;
5. determine the next MINE code-repository version from accepted changes and current managed version;
6. safely purge the MINE-owned `docs/plan/` workspace;
7. verify no tracked or untracked release-bound plan workspace or design backup enters the stable tree;
8. integrate the final tree through squash or curated commits so temporary plan history is not imported;
9. tag/publish when configured;
10. delete local managed `plan/*` and `dev` branches.

## Why squash or curated integration

A normal merge makes temporary plan commits reachable from stable history even after file deletion. Squash or curated integration keeps stable history focused on accepted product and design state.

## Hotfix exception

A hotfix may use `hotfix/<slug>` from the stable branch when explicitly requested, but it still updates design, uses temporary planning for non-trivial changes, passes independent review, and leaves no plan workspace on the stable tree.

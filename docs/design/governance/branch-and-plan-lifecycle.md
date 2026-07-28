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

## Registration and release

`mine plan add` **registers** a plan as `DRAFT`: it records identity, design
references, write paths, and dependencies, but makes no claim about whether
the plan may execute. `mine plan release --id <plan-id>` is the explicit,
deterministic gate that moves a `DRAFT` plan into the startable frontier
(`READY` when all hard predecessors are `ACCEPTED`, otherwise `BLOCKED`). The
distinction between registration and release is deliberate and must be
preserved: a freshly added plan never becomes silently executable.

Automatic successor release inside `mine plan accept` is unchanged: accepting
a plan may release `BLOCKED` successors whose hard predecessors are all now
accepted, in the same accept transaction. `mine plan release` covers the
standalone case (a newly registered `DRAFT` plan with no pending accepted
upstream), which the accept pass cannot reach.

## During development

- Design changes precede plans.
- Plan files are immutable after execution starts.
- Reports and graph state are temporary but committed on managed development branches for reviewability.
- `docs/design/` on `dev` describes approved target behavior.
- Every plan references exact design paths and anchors.
- Execution agents commit scoped implementation but never self-accept.
- Review agents may merge accepted work into `dev` and remove the local accepted plan branch.

## Reviewer authority to correct, not only to verdict

A reviewer's independence is about independently inspecting and validating a submitted implementation, never about a prohibition on correcting
a defect once found. A reviewer is responsible for bringing submitted work to an acceptable, mergeable state:

- Reviewers may directly apply localized corrections — a local code fix, a strengthened or added test, a corrected CI/CD workflow step, a
  corrected manifest or generated-copy synchronization, a documentation/Skill-wording correction, or a narrow release-closure blocker — when
  the correct behavior is already unambiguous from the accepted Design and the plan, the fix carries no new product/design decision, and it is
  fully verifiable in the same review session.
- Reviewer-authored corrections are committed as their own explicit, separately labeled commit(s) and are described in the review report
  (what changed, why, and its revalidation evidence); they are never concealed, folded silently into the implementer's commits, or accepted
  without rerunning every gate the change could affect.
- A localized reviewer correction does not require a second reviewer, and does not by itself require a new compensating plan solely to
  preserve reviewer/implementer role separation.
- `REJECTED` plus a compensating plan is reserved for material Design changes, replacement of the core approach, a substantial independent
  work package, major scope expansion, or a finding that cannot be safely and fully completed within the current review session.
- During final release closure, a narrow release blocker (for example an ineffective validation gate, a missing CI matrix leg, or a
  diagnostic command that drops data it should preserve) is normally fixed and documented directly in the same closure session rather than
  deferred to another plan.

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

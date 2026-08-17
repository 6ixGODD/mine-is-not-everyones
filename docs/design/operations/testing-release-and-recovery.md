# Testing, Release, and Recovery

## Quality gates

Expected repository gates include:

```text
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
cargo doc --no-deps
cargo deny check            # when adopted
```

Platform CI covers Windows, Linux, and macOS.

## Test layers

### Unit and property tests

- identifiers, markers, and path normalization;
- state transitions and readiness;
- conflict detection and revision rules;
- design-index and anchor validation;
- backup destination validation;
- arbitrary DAG properties;
- serialization round-trips;
- repository escape prevention.

### Integration tests

- `mine init` for absent, valid, foreign, and legacy design roots;
- design backup creation, ignore behavior, link safety, and failure atomicity;
- workspace open/close without user version input;
- CLI lifecycle and JSON snapshots;
- concurrent writers and lock timeout;
- MCP lifecycle calls;
- Skill contracts against real CLI/MCP schemas;
- installer idempotency and backup;
- managed branch and release behavior.

### Real-client smoke tests

Verify actual discovery on supported current versions of Claude Code, Codex, Pi, and OpenCode. Copied files are not equivalent to discoverable Skills.

## Legacy repository fixtures

Tests include:

1. large repository with no design;
2. MINE scaffold with no meaningful leaves;
3. valid MINE design that is stale relative to code;
4. legacy unmarked `docs/design/`, which must be rejected;
5. foreign repository marker, which must be rejected;
6. repository-internal and external symlinks/junctions;
7. unscoped sync with explicit incomplete-coverage reporting.

## Release model

### Generic repository release gates

Every MINE-managed repository must pass these gates before local stable integration:

1. valid MINE repository configuration (`.mine/config.toml`);
2. valid Design (`mine design validate`);
3. terminal Plan graph on development state (every Plan `ACCEPTED` or `REJECTED` with accepted compensation);
4. accepted compensation chains for every rejected Plan;
5. clean working tree;
6. no pending MINE-owned transactions;
7. stable candidate cleanliness: no `docs/plan/` or tracked `docs/design-backup-*` on the stable branch;
8. configured stable/integration branch correctness (read from `config.branches`, not hardcoded); a configured stable branch that does not exist in git is a decisive, actionable failure (run `mine init` to repair a stale recorded branch), never a silent empty resolution;
9. repository-defined decisive validation evidence (discovered from `AGENTS.md`, Design, and the Plan - never presuming a specific toolchain).

The generic `mine release` preflight enforces these. It must **not** require `skills/`, `plugins/mine/skills/`, four-client installation, or MCP tool-count verification.

### MINE source-repository additional gates

The MINE source repository adds project-local gates enforced by `AGENTS.md` quality tables, CI, and MINE-local Design - not by the generic preflight:

- root/generated Skill synchronization (`python scripts/sync-plugin-assets.py --check`);
- embedded payload verification;
- four-client installation smoke tests;
- MCP discovery (exactly twelve tools);
- bootstrap tests;
- release artifact packaging and cross-platform CI.

### Lifecycle

1. develop on managed `dev` and `plan/*` branches;
2. independently accept and integrate all plans;
3. run full `mine-sync` against accepted code (Phase A);
4. `mine-plan-review complete release closure` performs mechanical closure (Phase B);
5. determine next managed repository version;
6. close and purge `docs/plan/`;
7. verify no plan workspace or design backup enters stable release;
8. integrate through squash or curated commits;
9. tag and publish checksummed binaries (MINE source repository only);
10. delete local managed temporary branches.

## Recovery

- bad sync: restore selected or full design from the timestamped local backup, then rerun with narrower scope or corrected instructions;
- backup failure: do not mutate design;
- namespace conflict: `mine init` auto-backs-up the legacy `docs/design/` to a timestamped backup and creates a fresh root; rerun `mine-sync` afterward;
- revision conflict: reload and re-evaluate;
- lock timeout: report evidence, never break blindly;
- render failure: retain TOML and rerun render;
- failed workspace purge: leave files intact;
- incomplete repository scan: keep explicit warnings and block full-release attestation.

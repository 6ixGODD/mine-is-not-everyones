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

1. develop on managed `dev` and `plan/*` branches;
2. independently accept and integrate all plans;
3. run full `mine-sync` against accepted code;
4. validate design, product, and supported clients;
5. determine next managed repository version;
6. close and purge `docs/plan/`;
7. verify no plan workspace or design backup enters stable release;
8. integrate through squash or curated commits;
9. tag and publish checksummed binaries;
10. delete local managed temporary branches.

## Recovery

- bad sync: restore selected or full design from the timestamped local backup, then rerun with narrower scope or corrected instructions;
- backup failure: do not mutate design;
- namespace conflict: rename/remove legacy directory, then rerun init;
- revision conflict: reload and re-evaluate;
- lock timeout: report evidence, never break blindly;
- render failure: retain TOML and rerun render;
- failed workspace purge: leave files intact;
- incomplete repository scan: keep explicit warnings and block full-release attestation.

# Component Architecture

## Dependency direction

```text
CLI Adapter ─┐
             ├──> Application Services ───> Domain
MCP Adapter ─┘             │                  ▲
                           ├──> Repository Port│
                           ├──> Git Port       │
                           ├──> Clock/ID Ports │
                           └──> Agent/Dist Ports
                                      │
                               Infrastructure Adapters
```

The domain does not depend on clap, rmcp, filesystem APIs, Git subprocesses, agent configuration formats, or Markdown rendering.

## Components

### Domain

Owns plan-workspace identity, plan states, legal transitions, dependency validation and graph-cycle detection, readiness, parallel waves, write-path conflicts, revisions, safe paths, marker ownership, and workspace closure invariants.

### Application services

- `InitService`;
- `WorkspaceService`;
- `GraphService`;
- `PlanService`;
- `DesignService`;
- `DesignBackupService`;
- `RepositoryVersionService`;
- `DistributionService`;
- `AgentInstallationService`;
- `DoctorService`.

### Infrastructure

Implements TOML persistence, atomic writes, locks, repository discovery, Git evidence and managed-branch actions, design indexes and markers, safe design backups, deterministic rendering, embedded Skills, agent configuration adapters, and event logging.

### CLI and MCP adapters

Adapters map typed requests to the same application services and contain no duplicate state-machine, path, backup, or branch policy.

## Transaction pattern

Every graph write:

1. locates repository and validates marker/branch/workspace gate;
2. acquires the fixed lock;
3. reloads configuration and graph;
4. validates expected revision;
5. executes one domain transition;
6. writes TOML atomically;
7. renders generated views;
8. records a best-effort event;
9. releases the lock;
10. returns structured output.

Design backup is a separate deterministic operation that completes and verifies before any model-driven design rewrite begins.

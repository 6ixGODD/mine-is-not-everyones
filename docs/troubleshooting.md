# MINE Troubleshooting

This page is for failures that leave you unsure how to recover. Routine installation, workflow, and release steps belong in the [User Guide](user-guide.md).

## An Agent is installed, but MINE is missing from it

Start with the machine-level view:

```sh
mine agent status
```

If the Agent is not listed as healthy, rerun setup for that client:

```sh
mine setup --agents <slug>
```

Then restart the Agent so it reloads Skills and MCP configuration.

If setup still cannot detect the client, check whether that client's executable and configuration root are actually available to the environment from which `mine setup` is running. Do not use `mine doctor` as the first check here: `doctor` also evaluates repository state, while `agent status` isolates machine-level installation.

## `mine init` refuses a Design ownership mismatch

A MINE-managed Design tree carries repository ownership in `docs/design/.mine-design.toml`. If that marker belongs to another repository, MINE refuses to adopt it silently.

Inspect the repository-level diagnostic:

```sh
mine doctor --format json
```

Do not delete or rewrite the marker just to make the error disappear. First determine why the Design tree came from another repository. Preserve any content you need, then deliberately establish the correct Design namespace for this repository.

An unmarked legacy `docs/design/` is different: `mine init` backs that directory up automatically before creating the managed namespace.

## A Plan cannot start because it is `BLOCKED`

Inspect the Plan and the current frontier:

```sh
mine plan show --id <id> --format json
mine graph ready --format json
```

`BLOCKED` normally means a hard predecessor has not reached `ACCEPTED`. Complete or compensate the predecessor first. Do not hand-edit the graph to force the Plan into `READY`.

If a graph mutation fails with a revision conflict, refresh the graph state and retry the intended transition against the new revision; the conflict means another valid transition won the race.

## Release closure says the final sync is missing or stale

Release closure requires synchronization evidence that corresponds to the current final `dev` state. If `dev` changed after Phase A, the previous evidence is no longer fresh.

Run Phase A again:

```text
mine-sync prepare this repository for stable release
```

Then retry:

```text
mine-plan-review complete release closure
```

Do not bypass the freshness check with `mine design validate`; structural Design validity is not proof that final Design matches the latest integrated implementation.

## `mine release --format json` says `can_release: false`

Treat the returned `errors` as the starting point. Release preflight is a collection of independent gates; there is no single generic repair command.

```sh
mine release --format json
```

Resolve the specific failing gate, then rerun preflight. If the error concerns graph state, inspect the graph; if it concerns Design, inspect Design validation; if it concerns Git or temporary release artifacts, fix that repository state. Do not weaken or skip the gate to obtain a green result.

## A command fails, but this page has no matching entry

Use the command's own help and machine-readable diagnostic first:

```sh
mine <command> --help
mine doctor --format json
```

If the failure is reproducible and the CLI does not explain how to act on it, that is a useful bug report: include the exact command, exit code, JSON/human error, platform, and `mine --version`.
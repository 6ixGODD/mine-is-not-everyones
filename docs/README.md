# MINE Documentation

MINE has two kinds of documentation:

- **User documentation** explains how to use MINE.
- **Design documentation** records MINE's own durable engineering contracts and implementation decisions.

If you are trying MINE on a project, start with the User Guide. You usually do not need to read `docs/design/`.

## User documentation

English:

- [User Guide](user-guide.md) — installation, repository setup, daily workflow, review, and release
- [Concepts](concepts.md) — the mental model behind Design, Plans, branches, independent review, and release closure
- [Troubleshooting](troubleshooting.md) — recovery from concrete error states

简体中文：

- [用户指南](user-guide.zh-CN.md)
- [核心概念](concepts.zh-CN.md)
- [故障排查](troubleshooting.zh-CN.md)

## Internal Design

[Design index](design/index.md) is the entry point for MINE's internal durable Design. It is written for people changing MINE itself, not as a prerequisite for using the tool.

`docs/design/` stays on stable branches because it describes accepted engineering behavior. `docs/plan/` is different: it is temporary execution state created during a development cycle and removed during release closure.

## When something fails

Start with the diagnostic surface closest to the failure:

```sh
mine --help
mine <command> --help
mine agent status
mine doctor --format json
```

Then use [Troubleshooting](troubleshooting.md) only for recovery cases where the next action is not obvious from the command output.
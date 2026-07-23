# Plan 04: Skills bootstrap integration with JSON CLI

## Status

`BLOCKED`

## Goal

在 MCP 尚未完成时，先把四个根 Skill 改成使用 Plan 03 的真实 JSON CLI 契约，并建立 Skill contract tests，消除手工执行图写入和占位命令。

## User-visible outcome

Claude/Codex/Pi/OpenCode 即使没有 MCP，也能通过 shell 工具可靠调用 `mine --format json` 完成初始化、Plan 注册、领取、IMPLEMENTED、ACCEPT/REJECT。

## Governing architecture references

- 架构 §12 四个 Skill 的最终集成
- 架构 §10 CLI 公开契约
- 架构 §13 四平台发行策略

## Requirements traceability

| Requirement | Architecture section | Work package | Acceptance evidence |
|---|---|---|---|
| four Skills only | §12 | WP2–WP5 | inventory exactly four |
| JSON CLI fallback | §12.1 | WP1–WP5 | fixture lifecycle |
| no direct graph edit | §2.3, §12 | WP6 | forbidden-text test |
| expected revision | §7.4, §12 | WP3–WP5 | conflict scenario |
| harness-neutral source | §13 | all | manual/platform review |

## Current evidence and baseline

- Four root Skills already exist and contain intended workflow prose.
- They currently reference future MCP tool names and provisional CLI syntax.
- Plugin copies are generated drafts and are not modified in this parallel Plan.
- Plan 03 is the authoritative source for actual CLI syntax and JSON.

## Decisions

- This Plan deliberately creates a complete CLI-only fallback before MCP exists.
- MCP references remain conditional descriptions only; final exact MCP-first behavior waits Plan 06.
- Skill text may invoke harness shell tools generically but must not invent a tool name common to all harnesses.

## Research source register

| Source | URL | Accessed | Claim | Plan implication |
|---|---|---|---|---|
| Pi Skills | https://github.com/badlogic/pi-mono/blob/main/packages/coding-agent/docs/skills.md | 2026-07-23 | Skills can invoke tools/scripts and use `/skill:name` | keep harness-neutral text |
| OpenCode Skills | https://opencode.ai/docs/skills | 2026-07-23 | SKILL.md discovery/on-demand load | no fake slash command |
| Claude Skills/Plugins | https://code.claude.com/docs/en/plugin-marketplaces | 2026-07-23 | plugin Skills namespaced | user examples must distinguish modes |
| Codex skill creator context | https://github.com/openai/codex | 2026-07-23 | Agent Skills supported | validate frontmatter and explicit invocation |

## Scope

### In scope

- root four `SKILL.md`；
- references where CLI examples live；
- Skill contract scanner/test；
- handoff behavior for CLI errors/revision conflicts；
- current README/docs invocation snippets if needed。

### Non-goals

- MCP-first final wording；
- plugin generated copy；
- marketplace manifests；
- agent installer。

## Dependency and parallelism

This Plan can run in parallel with Plan 05. It must not edit:

```text
src/mcp/
Cargo.toml
plugins/
```

Work packages for four Skills may run in parallel, but one integrator owns shared contract test and README.

## Work packages

### WP1 — Extract actual CLI contract

From compiled `mine --help` and JSON integration fixtures, generate/record:

- exact command syntax；
- output fields；
- error codes；
- revision behavior；
- exit codes。

Do not copy architecture examples blindly.

### WP2 — mine-arch CLI integration

Update:

- `mine init --format json` real ordering；
- validate/status；
- missing binary behavior；
- no Plan registration；
- AGENTS TOML/Markdown rule。

### WP3 — mine-plan-create CLI integration

Update:

- write Plan before registration；
- query revision；
- exact plan add args；
- parse single JSON envelope；
- revision conflict decision tree；
- validate and handoff。

### WP4 — mine-plan-exec CLI integration

Update:

- query/show；
- start before mutation；
- structured failure handling；
- implemented after report/commit；
- never self-accept。

### WP5 — mine-plan-review CLI integration

Update:

- validate/get；
- accept/reject exact args；
- compensation order；
- immutable original Plan。

### WP6 — Contract validation

Implement Rust test or temporary script that rejects:

- direct graph edit instructions；
- undefined `mine` commands/options；
- parsing human output；
- missing expected revision on writes；
- obsolete MCP placeholder treated as guaranteed；
- wrong Skill name/frontmatter。

## Verification matrix

- standard Rust gates；
- `mine dist verify` is not yet available, so run new Skill contract test and existing Python verify；
- execute each Skill’s CLI sequence against a fixture repository using scripted commands；
- manual read for harness-neutral clarity。

## Acceptance checklist

- [ ] Four Skills use exact Plan 03 CLI.
- [ ] No Skill edits graph files.
- [ ] Every write handles expected revision.
- [ ] Missing mine causes safe stop.
- [ ] JSON only is parsed.
- [ ] Plan 05 paths were untouched.
- [ ] Plugin copies are intentionally not synced until Plan 06.

## Report path

`docs/plan/reports/04-skills-json-cli-bootstrap-integration-implementation.md`

## Suggested commit

`feat(skills): integrate MINE workflow with structured CLI`

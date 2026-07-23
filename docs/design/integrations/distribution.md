# Distribution and Installation

## Source-of-truth rule

Repository-root `skills/` is the only hand-edited Skill source. Claude/Codex plugin directories and embedded binary payloads are generated or synchronized artifacts.

## Claude Code

Provide both:

- standalone installation for short commands such as `/mine-arch`;
- a versioned Claude marketplace plugin whose Skills are namespaced, for example `/mine:mine-arch`.

Marketplace layout follows current Claude Code requirements:

```text
.claude-plugin/marketplace.json
plugins/mine/.claude-plugin/plugin.json
plugins/mine/skills/<skill>/SKILL.md
```

The plugin directory is self-contained because Claude Code copies installed plugins into a cache.

## Codex

Provide:

- shared Agent Skills installation into a currently supported real directory;
- Codex plugin/marketplace metadata when the current stable Codex release validates and exposes it;
- a standalone Skill fallback because plugin support may evolve independently from Skill discovery.

Codex plugin installation is not declared complete until the actual client can discover the Skills and MCP server. Configuration presence alone is insufficient.

## Pi

The repository remains a Pi package with root `skills/` exposed through `package.json` or conventional discovery. Installation supports Git package sources. Pi invokes Skills with `/skill:<name>`.

Pi does not have MCP in its minimal core. MINE supports either:

- a user-installed MCP-capable Pi extension/adapter; or
- JSON CLI fallback from the Skills.

No duplicate TypeScript implementation of graph rules is allowed.

## OpenCode

Install Skills into one supported location only, preferring a shared Agent Skills or Claude-compatible directory. Configure the local stdio MCP server using OpenCode's current MCP configuration or CLI.

Do not publish an npm OpenCode plugin unless MINE later needs OpenCode-specific hooks or tools beyond Skills and MCP.

## `mine agent install`

The installer:

- supports only the four named harnesses;
- offers `--dry-run`;
- backs up structured configuration before mutation;
- records managed files and hashes;
- preserves unrelated configuration;
- refuses same-name conflicting entries unless forced;
- validates actual Skill discovery where possible;
- does not claim success merely because files were copied.

## Embedded payload

Release binaries embed the five Skill directories so standalone installation does not require a Git checkout. Build-time verification ensures embedded content matches root `skills/`.

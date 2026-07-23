# External Source Register

Verified for this design revision on 2026-07-23. Implementation plans must re-check current official documentation before relying on unstable platform details.

| Area | Primary source | Design implication |
|---|---|---|
| Claude Code marketplaces | https://code.claude.com/docs/en/plugin-marketplaces | Marketplace root, self-contained plugin directory, namespaced Skills, validation commands |
| Claude Code plugins | https://code.claude.com/docs/en/plugins | Standalone versus plugin behavior and Skill namespace |
| Codex plugin creator | https://github.com/openai/codex/blob/main/codex-rs/skills/src/assets/samples/plugin-creator/SKILL.md | Current plugin manifest and personal marketplace scaffold; implementation must validate against installed stable Codex |
| Codex Skills | https://github.com/openai/skills | Agent Skills model and distribution expectations |
| Pi Skills | https://github.com/badlogic/pi-mono/blob/main/packages/coding-agent/docs/skills.md | Skill locations, `/skill:name`, package discovery |
| Pi packages | https://github.com/badlogic/pi-mono/blob/main/packages/coding-agent/docs/packages.md | Git/npm/local package installation and manifest |
| OpenCode Skills | https://opencode.ai/docs/skills | Native, Claude-compatible, and Agent-compatible discovery roots |
| OpenCode MCP | https://opencode.ai/docs/mcp-servers/ | Local stdio MCP configuration and context-cost warning |
| MCP transport | https://modelcontextprotocol.io/specification/2025-11-25/basic/transports | stdio process and protocol separation |
| MCP Rust SDK | https://github.com/modelcontextprotocol/rust-sdk | Official Rust implementation basis |

# MINE 故障排查

常见问题的实用修复。以下所有命令与当前 MINE CLI 一致。每条目使用统一结构：**症状 / 原因 / 检查 / 修复**。

---

## `mine: command not found`

**症状：** shell 找不到 `mine` 二进制。

**原因：** 二进制未安装，或安装目录不在 `PATH` 中。

**检查：** Unix 运行 `which mine`，PowerShell 运行 `Get-Command mine`。确认安装目录存在：Windows 为 `%LOCALAPPDATA%\Programs\mine`，Linux/macOS 为 `~/.local/bin`。

**修复：** 重新运行对应平台的 bootstrap 安装器（见 README），然后重开终端。用 `mine --version` 验证。

---

## `mine setup` 未检测到 Agent

**症状：** `mine setup` 找不到或未为你的编码 Agent 安装。

**原因：** Agent 配置目录在非默认位置，或 Agent 不是四个受支持客户端之一（Claude Code、Codex、Pi、OpenCode）。

**检查：** `mine agent status` 列出受管安装。确认 Agent 使用受支持的配置位置（如 `CLAUDE_CONFIG_DIR`、`CODEX_HOME`、`PI_HOME`、`OPENCODE_CONFIG_DIR`）。

**修复：** 设置相关环境变量并重新运行 `mine setup --agents <slug>`（如 `--agents claude-code,codex`）。隔离安装使用 `--config-root <path>`。

---

## setup 后 Skill 不可见

**症状：** `mine setup` 后 MINE Skills 未出现在你的 Agent 中。

**原因：** Agent 需要重启，或它缓存了已发现的 Skills。

**检查：** `mine agent status` 报告 Agent 健康。确认 Skill 目录存在于 Agent 配置根下（如 `~/.claude/skills/mine-arch/SKILL.md`）。

**修复：** 重启 Agent 会话。若 Skills 仍缺失，为该 Agent 重新运行 `mine setup --agents <slug>`。

---

## MCP 未注册 / 不可见

**症状：** Agent 中无法使用 MCP 工具（如 `mine_plan_*`）。

**原因：** MCP 配置未合并，或 Agent 未加载它。

**检查：** `mine agent config <slug>` 预览 MINE 将合并的 MCP 条目。`mine mcp serve` 应能独立成功运行。

**修复：** 为该 Agent 重新运行 `mine setup`，然后重启 Agent 以重新加载 MCP 配置。

---

## `mine init` 遇到已有的非 MINE `docs/design/`

**症状：** 仓库已有一个非 MINE 管理的 `docs/design/` 目录。

**原因：** 遗留文档占用了 MINE 所有的命名空间。

**检查：** 查找 `docs/design/.mine-design.toml`。若不存在，则树未被标记。

**修复：** 这是预期行为。`mine init` 会把遗留目录移到 `docs/design-backup-<UTC 时间戳>/` 并创建全新的受管理根。遗留内容保留在备份中。

---

## 所有权不匹配 / 外来 Design 标记

**症状：** `mine doctor` 报告设计所有权不匹配。

**原因：** `docs/design/.mine-design.toml` 记录的仓库 ID 与 `.mine/config.toml` 中的不一致（例如设计树从其他仓库复制而来）。

**检查：** `mine doctor --format json` 显示哪个检查失败及原因。

**修复：** 这是刻意的拒绝——MINE 不静默采用外来标记。有意识地重新初始化设计命名空间（先用 `mine design backup` 保留内容，再重跑 `mine init`），或恢复正确的标记。

---

## `mine doctor` 报告 graph 未初始化

**症状：** `graph` 检查失败，报 "graph not initialized/invalid"。

**原因：** 仓库是 stable 树（没有 `docs/plan/` 工作区——这在 stable 分支上是正确的），或开发树的图尚未打开。

**检查：** `mine graph status --format json`。如果当前分支是 stable 分支，缺少 `docs/plan/` 是预期的，报告为 "not applicable"。

**修复：** 在开发分支上打开工作区：`mine workspace open --format json`。在 stable 树上无需操作。

---

## 为什么 `mine agent status` 与 `mine doctor` 不同

**症状：** 两个命令报告不同内容。

**原因：** 它们在不同层级操作。

**检查：** `mine agent status` 是**机器级**（已安装的 Agent 集成，与仓库无关）。`mine doctor` 是**仓库感知**（在已初始化仓库内检查 `.mine/config.toml`、设计、图、Git 分支）。

**修复：** 机器级安装后用 `mine agent status`；在仓库内用 `mine doctor`。

---

## Plan 保持 `BLOCKED`

**症状：** Plan 保持 `BLOCKED` 且无法开始。

**原因：** 并非所有硬前置都已 `ACCEPTED`，或 Plan 尚未 release。

**检查：** `mine plan show --id <id> --format json` 显示状态和 `hard_predecessors`。

**修复：** 接受（或带补偿拒绝）未完成的前置；然后 Plan 变为 `READY`（或用 `mine plan release --id <id>` 显式 release）。

---

## 发布预检失败

**症状：** `mine release --format json` 报告 `can_release: false`。

**原因：** 一个或多个发布关卡失败（非终态 Plan、无补偿的被拒 Plan、设计/图无效、工作树脏、挂起的 Agent 事务、或 stable 分支上有 Plan 工件/设计备份）。

**检查：** 查看预检 envelope 中的 `errors` 数组。

**修复：** 解决每个报告的关卡，然后重新运行预检。

---

## 最终同步证据缺失或过期

**症状：** 发布收口（阶段 B）拒绝继续。

**原因：** 最终 `mine-sync prepare this repository for stable release`（阶段 A）未运行，或证据过期（同步报告后 dev HEAD 移动了）。

**检查：** 查看 `.mine/runtime/sync/` 下的阶段 A 报告；将记录的提交与当前 `dev` HEAD 比较。

**修复：** 重新运行最终同步，然后重新调用 `mine-plan-review complete release closure`。

---

## 如何安全重跑 setup

**症状：** 安装不完整或想添加另一个 Agent。

**修复：** `mine setup` 是幂等且事务性的：它在变更前备份配置，并从不完备事务中恢复。重新运行 `mine setup --agents <列表>` 或不带参数进入交互式流程。在已初始化仓库内用 `mine doctor --agents all` 查看每个 Agent 的健康状态。

---

## 如何更新

**修复：** `mine update` 将二进制更新到最新发布（用 `--yes` 跳过提示）。用 `mine --version` 验证。

---

## 如何卸载

**修复：** `mine uninstall` 从所有 Agent 和本机移除 MINE（用 `--yes` 跳过提示）。它只移除 MINE 管理的文件；无关的 Agent 配置被保留。

---

## Windows 扫描器 / shell 预期

**症状：** stale-plan-reference 扫描在 Windows 失败，或错误信息提到 WSL 或 `bash`。

**原因（历史）：** 扫描器曾是 Bash helper，Windows 可能把 `bash` 解析为 WSL shim。

**检查：** `mine scan plan-refs --check --format json` 是原生跨平台扫描器。它没有 Bash/WSL/Git Bash 依赖。

**修复：** 发布扫描使用 `mine scan plan-refs`。遗留 Bash helper（安装的 Skill 目录中的 `references/scan-plan-refs.sh`）只是手动 Unix 兼容 helper，不是权威实现。

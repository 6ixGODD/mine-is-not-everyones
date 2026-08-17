# MINE 用户指南

本指南介绍 MINE 的实际使用方式。

关于 Design、Plan、Execution Graph、Review、worktree、补偿 Plan 和 stable 的设计意图，参见[核心概念](concepts.zh-CN.md)。

---

## 1. 安装

### Windows

在 PowerShell 中执行：

```powershell
irm https://raw.githubusercontent.com/6ixGODD/mine-is-not-everyones/master/scripts/bootstrap.ps1 | iex
```

### macOS / Linux

在终端中执行：

```sh
curl -fsSL https://raw.githubusercontent.com/6ixGODD/mine-is-not-everyones/master/scripts/bootstrap.sh | sh
```

安装完成后，重新打开终端，验证安装是否成功：

```sh
mine --version
```

然后安装 Agent 集成：

```sh
mine setup
```

查看当前集成状态：

```sh
mine agent status
```

MINE 当前支持 Claude Code、Codex、Pi 和 OpenCode。

对于支持 MCP 的客户端，mine setup 会注册本地 mine mcp serve；当 MCP 不可用时，Skill 会自动回退到 JSON CLI 模式。Pi
的最小核心本身不包含 MCP，因此可以直接使用 Skills + CLI 完成全部操作。

### 更新

`mine update` 会替换二进制**并**用新版本的 embedded payload 刷新所有已安装 Agent 的 Skill，因此升级后无需重新运行
`mine setup`：

```sh
mine update
```

`mine setup` 用于**首次安装**以及添加或移除 Agent（例如 `mine setup --agents claude-code,codex`）。更新后用
`mine --version` 和 `mine agent status` 验证。

### Pi 共享 Skill

Pi 会在共享 Agent Skills 目录（`~/.agents/skills`，即 Codex 的安装位置）和自身目录（`~/.pi/agent/skills`）同时发现
Skill。为避免 Pi 加载两份（以及由此产生的冲突告警），当共享目录已有完整 MINE Skill 集时，MINE 会将 Pi 的 Skill 安装到
共享目录，并移除 `~/.pi/agent/skills` 下的遗留 MINE Skill。

### 安装指定版本

bootstrap 默认安装最新 Release。如需安装指定版本，请使用`MINE_REF`。

例如，安装 `v0.1.4`：

#### Windows

```powershell
$env:MINE_REF = "v0.1.4"
irm https://raw.githubusercontent.com/6ixGODD/mine-is-not-everyones/master/scripts/bootstrap.ps1 | iex
```

#### macOS / Linux

```sh
MINE_REF=v0.1.4 \
sh -c "$(curl -fsSL https://raw.githubusercontent.com/6ixGODD/mine-is-not-everyones/master/scripts/bootstrap.sh)"
```

---

## 2. 初始化仓库

在 Git 仓库根目录运行：

```sh
mine init
```

一个仓库通常只需要初始化一次。

`mine init` 会建立 MINE 所需的仓库状态和 `docs/design/`，但不会替你设计系统或修改业务代码。

---

## 3. 开始一次工程变更

日常工作主要通过以下五个 Agent Skill 完成：

```text
mine-arch
mine-sync
mine-plan-create
mine-plan-exec
mine-plan-review
```

需要说明的是，这些名称指的是 **Agent Skill**，而非 `mine` CLI 的子命令。不同 Agent 的调用语法存在差异，请以所使用的 Agent
的实际调用方式为准。

以下列举几种常见 Agent 的调用形式：

**Codex：**

```text
$mine-arch ...
```

**Claude Code（通过 Marketplace 安装的插件）：**

```text
/mine:mine-arch ...
```

**Pi：**

```text
/skill:mine-arch ...
```

**其他 Agent：**

```text
/mine-arch ...
```

上述示例均以 `mine-arch` 为代表，其余四个 Skill 的调用方式遵循相同规则，只需将 Skill 名称替换为对应的 `mine-sync`、
`mine-plan-create`、`mine-plan-exec` 或 `mine-plan-review` 即可。

## 4. 创建 Plan

Design 明确后：

```text
mine-plan-create <需要规划的范围>
```

例如：

```text
mine-plan-create 将刚才确定的 Passkey 登录改动整理为执行计划。
```

明确的 scope 会作为本次规划边界。`mine-plan-create` 会围绕这一范围检查相关 Design、代码、测试、依赖和成熟实践，并创建一个或多个可执行
Plan。

如果直接调用：

```text
mine-plan-create
```

而不指定 scope，Skill 会自行从 Design、Execution Graph 和仓库状态中寻找下一批需要规划的工作，因此通常会进行范围更广的探索。

如果规划过程中发现当前 Design 不足以支撑实现，`mine-plan-create` 可以主动调用 `mine-arch` 更新 Design，然后继续规划。用户通常不需要手工切换
Skill。

查看当前可执行 Plan：

```sh
mine graph ready
```

查看某个 Plan 的详情：

```sh
mine plan show --id <id>
```

---

## 5. 执行和 Review

对处于 READY 状态的 Plan：

```text
mine-plan-exec <Plan 路径>
```

执行完成后，Plan 进入：

```text
IMPLEMENTED
```

然后在 **独立的 Agent session** 中运行：

```text
mine-plan-review <Plan 路径>
```

**不要**在刚刚执行 `mine-plan-exec` 的同一个上下文中继续 Review。

可以使用：

* 同一种 Agent 的两个独立 session；
* 两种不同 Agent。

例如：

```text
Codex session A
    → mine-plan-exec

Codex session B
    → mine-plan-review
```

或者：

```text
Claude Code
    → mine-plan-exec

Codex
    → mine-plan-review
```

Review 可能产生三种结果：

* **ACCEPTED**：实现被接受并集成；
* **局部修正后 ACCEPTED**：Reviewer 可以直接修复明确、局部且不改变工程契约的问题；
* **REJECTED**：当前 Plan 无法接受，通常会创建补偿 Plan，并更新受影响的后续依赖。

如果 REJECTED 的根因需要修改 Design，Reviewer 可以主动调用 `mine-arch`。用户通常无需手工修改 Execution Graph。

---

## 6. 并行执行

如果存在多个彼此独立的 READY Plan，可以并行执行。

MINE 会使用独立 branch / worktree 隔离不同 Plan，例如：

```text
dev
├── plan/01-api       → worktree A
├── plan/02-storage   → worktree B
└── plan/03-ui        → worktree C
```

通常不需要手工：

* 创建 Plan branch；
* 创建 worktree；
* 修改 Execution Graph；
* 管理 graph revision；
* 管理 report 路径；
* 将 accepted Plan 手工集成到 `dev`。

这些状态由 MINE 和对应 Skill 自动管理。

---

## 7. 发布

所有计划工作完成后，先执行 Final Sync：

```text
mine-sync prepare this repository for stable release
```

它会根据最终代码重新核对 `docs/design/`。

然后在独立 Review 上下文中执行：

```text
mine-plan-review complete release closure
```

Release Closure 会完成本地发布收口，包括：

* 验证 Execution Graph 和 release gates；
* 确认最终 Design 已同步；
* 构造并验证 stable candidate；
* 检查产品代码中是否残留当前开发周期的临时 Plan 引用；
* 移除 `docs/plan/` 和执行报告；
* 将最终状态集成到 stable；
* 清理 MINE 管理的本地临时 branch / worktree。

MINE **不会**主动执行：

* push；
* Git tag；
* GitHub Release；
* package publish；
* 其他远程发布操作。

这些操作由用户决定。

---

## 8. 查看状态和诊断

### Agent 集成

```sh
mine agent status
mine setup
```

### 当前仓库

```sh
mine status
mine doctor
```

### Design

```sh
mine design status
mine design validate
```

### Execution Graph

```sh
mine graph status
mine graph ready
mine graph wave
mine graph validate
```

### Plan

```sh
mine plan show --id <id>
```

### 发布检查

```sh
mine release --format json
```

当命令执行失败时，优先查看 CLI 返回的 human 或 JSON 格式的诊断信息。

关于各机制的设计意图，参见[核心概念](concepts.zh-CN.md)。

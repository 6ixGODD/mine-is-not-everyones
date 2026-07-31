<h1 align="center">MINE</h1>

<p align="center"><strong>MINE Is Not Everyone's.</strong></p>

<p align="center">
  面向 Claude Code、Codex、Pi 与 OpenCode 的强约束、文档驱动工程工作流。
</p>

<p align="center">
  <a href="https://github.com/6ixGODD/mine-is-not-everyones/blob/master/LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="License: MIT"></a>
  <img src="https://img.shields.io/badge/rust-1.85%2B-orange.svg" alt="Rust 1.85+">
  <img src="https://img.shields.io/badge/agents-Claude%20Code%20%7C%20Codex%20%7C%20Pi%20%7C%20OpenCode-6a5acd.svg" alt="Supported agents">
</p>

<p align="center"><a href="README.md">English</a> · <a href="README.zh-CN.md">简体中文</a></p>

---

## 为什么需要 MINE

* 架构文档会逐渐偏离实现；
* 规划随对话上下文消散；
* 实施、审查与发布之间缺少明确边界；
* 临时分支和过程文档容易进入稳定版本；
* 新会话需要重新建立工程语境。

MINE 把架构、规划、实施、审查与发布收口留在仓库中，由确定性工具约束，可版本化、可追溯。

## 工作方式

五个 Agent Skill 负责工作流与工程判断，一个 Rust 二进制负责确定性部分。

```mermaid
flowchart LR
    A["mine-arch\n需求 → 设计"] --> B["mine-plan-create\n设计 → Plan"]
    B --> C["mine-plan-exec\nPlan → 实现"]
    C --> D["mine-plan-review\n验证 · 修正 · 裁决 · 发布收口"]
    S["mine-sync\n设计 ↔ 代码 对齐"] -.->|已有仓库| A
```

| Skill | 职责 |
|---|---|
| `mine-arch` | 依据需求创建或演进目标架构 |
| `mine-sync` | 将 Design 与真实仓库对齐 |
| `mine-plan-create` | 把已确认的 Design 变更拆解为可执行 Plan |
| `mine-plan-exec` | 在仓库治理约束下实施单个 Plan |
| `mine-plan-review` | 独立审查、直接修正、裁定接受或拒绝，并完成发布收口 |

`mine` 二进制负责：

* 仓库初始化；
* 执行图状态管理；
* Plan 生命周期流转；
* 文件锁与原子写入；
* Design 与执行图校验；
* Agent 安装与诊断；
* 分发资产同步；
* 发布前检查。

需求与边界只需说明一次。

## 安装

### Windows（PowerShell）

```powershell
irm https://raw.githubusercontent.com/6ixGODD/mine-is-not-everyones/master/scripts/bootstrap.ps1 | iex
```

### macOS 与 Linux

```sh
curl -fsSL https://raw.githubusercontent.com/6ixGODD/mine-is-not-everyones/master/scripts/bootstrap.sh | sh
```

装好后需要重启终端。Windows 安装到 `%LOCALAPPDATA%\Programs\mine`，Linux/macOS 安装到 `~/.local/bin`。安装、更新与卸载的生命周期管理见[用户指南](docs/user-guide.md#installation-and-lifecycle)。

### 指定版本

默认装最新发布的 Release。锁定版本时设置 `MINE_REF`：

```sh
MINE_REF=v0.1.0 sh -c "$(curl -fsSL https://raw.githubusercontent.com/6ixGODD/mine-is-not-everyones/master/scripts/bootstrap.sh)"
```

```powershell
$env:MINE_REF = 'v0.1.0'
irm https://raw.githubusercontent.com/6ixGODD/mine-is-not-everyones/master/scripts/bootstrap.ps1 | iex
```

### Claude Code 插件

```text
/plugin marketplace add 6ixGODD/mine-is-not-everyones
/plugin install mine@mine-is-not-everyones
```

<details>
<summary>从源码构建</summary>

需 Rust 1.85：

```sh
cargo install --path . --locked
mine setup
```

</details>

## 快速开始

示例直接使用 Skill 名称，具体调用方式因 Agent 客户端而异。

### 新仓库

初始化 Git，再初始化 MINE：

```bash
git init
mine init
```

`mine init` 只做确定性初始化；架构、Plan、实现由 Skill 负责。

打开 Agent，给出需求：

```text
mine-arch 创建一个支持导入、导出和撤销操作的本地任务管理 CLI。
```

随后：

```text
mine-plan-create
mine-plan-exec <Plan 路径>
mine-plan-review <Plan 路径>
```

### 已有仓库

若仓库已用 `docs/design/` 存放无关文档，`mine init` 会将其备份到 `docs/design-backup-<时间戳>/` 并创建新的受管理根目录。

```bash
mine init
```

建立与当前代码一致的 Design 基线：

```text
mine-sync 认证与授权子系统
```

再照常演进仓库：

```text
mine-arch 增加 Passkey 登录。
mine-plan-create
mine-plan-exec <Plan 路径>
mine-plan-review <Plan 路径>
```

> 大型仓库应为 `mine-sync` 明确范围。

## 仓库模型

MINE 独占管理 `docs/design/`，标记文件为 `docs/design/.mine-design.toml`。

同步时，现有代码、Schema、配置、测试与可观测运行行为优先于过时的 Design，除非用户明确要求保留某项设计决策。

重写 Design 前，MINE 先创建本地、被 Git 忽略的备份。

`dev`、`plan/*`、`docs/plan/`、执行图、实施报告、审查报告只存在于开发过程，不进入稳定版本。

## 审查行为

Reviewer 独立验证实现，也可直接修复范围明确的局部缺陷、补强测试、修正工作流问题、解决发布收口的小型阻塞，并单独提交、写入审查报告、重新验证。

只有大量独立工作、实质性 Design 变更、重大范围扩张、公共契约变更，或无法在本轮审查中完成时，才创建补偿 Plan。

## 权限边界

MINE 面向接受强约束，并允许 Coding Agent 在治理规则内操作仓库的单一仓库所有者。

| MINE 可以 | MINE 绝不会 |
|---|---|
| 重写不准确的 MINE 受管理 Design | 执行任意 `git reset --hard` |
| 创建并使用受管理的 `dev`、`plan/*` 分支 | 执行 `git clean` |
| 在当前 Plan 范围内提交修改 | 盲目 stash |
| 合并已通过审查的工作 | 强制推送 |
| 清理 MINE 自有的临时发布产物 | 重写公共历史 |
| | 删除无关分支或文件 |
| | 执行无边界的 Shell 删除 |
| | 向仓库外写入文件 |

"破坏性"仅针对过时的 MINE 受管理 Design 与临时过程状态。

## 支持的客户端

* Claude Code
* Codex
* Pi
* OpenCode

## 当前状态

MINE 仍处于早期阶段，而且有意保持强烈倾向性。

建议在可恢复的仓库中试用，阅读生成的 Design，检查 Git 历史，验证最终稳定输出。

## 文档

* [文档索引](docs/README.md)
* [用户手册](docs/user-guide.md)
* [Design 索引](docs/design/index.md)

## 许可证

MIT
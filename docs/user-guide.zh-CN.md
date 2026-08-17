# MINE 用户指南

## 快速开始

MINE 是一套意见明确的、以文档为驱动的工程工作流，运行在你的编码 Agent 中。正常操作遵循以下心智模型：

```text
安装（bootstrap + mine setup）
  ↓
mine init                      每个仓库一次
  ↓
mine-arch                      新需求 / 目标设计
或
mine-sync                      已有仓库 / 代码基线
  ↓
mine-plan-create               把 Design 变更变成 Plan
  ↓
mine-plan-exec                 实现一个 Plan
  ↓
mine-plan-review               独立审查
  ↓
按需重复
  ↓
mine-sync prepare this repository for stable release
  ↓
mine-plan-review complete release closure
```

你**不需要**手动管理执行图修订号、报告路径、临时 `plan/*` 分支、`dev` 集成机制或候选构建。这些是 MINE 的内部工作流机制，由 Skills 和 CLI 处理。

**轻量维护：** 并非每次编辑都需要 Plan。错别字修复、文字润色、翻译、坏链接修复、README 改进，以及其他明确的、保持行为不变的编辑性维护，可以直接进行（仍需遵守 `AGENTS.md`）。改变行为的工程工作使用 MINE。

## 用户需要记住什么

正常使用时，仓库所有者只需要一个 CLI 命令和五个 Agent Skills：

```text
mine init

mine-arch
mine-sync
mine-plan-create
mine-plan-exec
mine-plan-review
```

其余 CLI 主要服务于 Skills、诊断和高级检查。

## `mine init` 是初始化，不是执行

在仓库根目录运行一次：

```bash
mine init
```

它只执行确定性的初始化：

- 发现 Git 根目录和 stable 分支；
- 创建 `.mine/config.toml` 和被忽略的运行时目录；
- 安装或验证 MINE 设计命名空间标记；
- 在缺失时创建空的模块化 `docs/design/` 脚手架；
- 在 `AGENTS.md` 中创建或更新 MINE 治理条款；
- 在授权时配置受支持的 Agent 集成；
- 将 MINE 代码仓库版本初始化为现有受管值、可靠的显式仓库版本，或默认 `0.1.0`；
- 验证初始化结果。

它**不**扫描源代码、不写架构、不创建 `docs/plan/`、不创建开发分支、不调用 Agent、不写业务代码、不提交、不合并、不发布。

## 设计命名空间处理

MINE 拥有以下路径：

```text
docs/design/
```

受管理树包含：

```text
docs/design/.mine-design.toml
```

如果仓库已在 `docs/design/` 存放无关文档，`mine init` 会把现有目录移到 `docs/design-backup-<UTC 时间戳>/` 并创建全新的 MINE 受管理设计根。旧内容保留在备份中；MINE 不猜测如何迁移任意旧布局，但也不会中止初始化。

## 新仓库工作流

```bash
mine init
```

然后打开 Claude Code、Codex、Pi 或 OpenCode，调用：

```text
mine-arch <需求>
```

`mine-arch` 扫描相关仓库状态、研究外部契约、创建或更新模块化目标设计。在 MINE 治理定义的常设 Git 授权下，它可能创建受管理的 `dev` 分支。

设计就绪后：

```text
mine-plan-create
mine-plan-exec <plan 路径>
mine-plan-review <plan 路径>
```

重复执行和审查，直到每个 Plan 都被接受。

## 已有仓库工作流

### 1. 处理命名空间

如果 `docs/design/` 包含遗留的非 MINE 文档，`mine init` 会将其备份到 `docs/design-backup-<时间戳>/` 并创建新的受管理根。

### 2. 初始化 MINE

```bash
mine init
```

### 3. 建立与代码一致的 Design 基线

调用：

```text
mine-sync
```

大型仓库可提供范围：

```text
mine-sync synchronize the payment domain, order persistence, and webhook delivery paths
```

Agent 从用户指定的路径、服务、包、符号或子系统开始，然后追踪它们的直接依赖和外部可见契约。

不提供范围时，Agent 被授权广泛探索，直到能准确表示仓库。这可能成本较高。MINE 将无范围请求视为用户接受该成本。

### 4. 演进架构

基线反映当前代码后：

```text
mine-arch <新需求>
```

`mine-arch` 以需求为先。它可以有意让目标设计与当前实现不同。`mine-plan-create` 随后规划这一转变。

## `mine-sync` 做什么

当受管理设计树存在时，`mine-sync`：

1. 创建 `docs/design-backup-<时间戳>/`；
2. 复制当前设计树，不跟随仓库外部符号链接或 junction；
3. 在备份目录写入 `*` 到 `.gitignore`，使备份保持本地化；
4. 盘点请求的代码范围，或未提供范围时自由探索；
5. 将代码、schema、配置、运行时行为、测试和公共契约与设计比较；
6. 应用以下权威顺序：
   - 明确的当前用户指令和受保护的设计决策；
   - 当前可观察的仓库行为；
   - 仅在代码无法决定答案时才参考现有设计；
7. 更新模块化设计树和索引；
8. 报告可疑代码、不确定性和不完整覆盖，不假装它们不存在；
9. 验证链接、标记、文档大小和设计所有权。

当没有有意义的设计时，`mine-sync` 从当前代码库创建描述性基线。

除非用户单独请求实现工作，`mine-sync` 不修改业务代码。

## 临时分支和 Plan

MINE 使用：

```text
stable 分支              仅已发布代码和 docs/design/
dev                      临时集成分支
plan/<id>-<slug>         临时实现分支
docs/plan/               临时 Plan 工作区
```

仓库所有者授予 MINE Skills 常设授权：

- 创建和切换受管理的 `dev` 和 `plan/*` 分支；
- 提交属于当前 Plan 的文件；
- 将独立接受的 Plan 分支合并到 `dev`；
- 删除已接受、已合并的本地 `plan/*` 分支；
- 在全部关卡通过时执行最终的 squash 或 curated 发布集成；
- 发布后删除临时的本地 `dev` 分支。

该授权不包括任意分支、force push、`reset --hard`、`git clean`、盲目 stash、重写公共历史或丢弃无关更改。

用户不手动提供开发周期版本。`mine-plan-create` 打开一个带生成标识符的内部工作区。仓库版本在发布时根据已接受的更改和现有 MINE 版本状态决定。

## 发布收口

发布收口由两个不同角色负责的两个阶段组成。

### 阶段 A - 最终设计调和（仓库所有者）

所有 Plan 被接受并集成到 `dev` 后，调用最终的全仓库同步：

```text
mine-sync prepare this repository for stable release
```

这将已接受的实现调和进 `docs/design/`，解决或报告每个不完整区域，并验证整个仓库。这是一个独立的、有意的会话。

### 阶段 B - 机械发布收口（mine-plan-review）

最终同步后，审查者执行机械收口。以发布收口模式调用 Skill：

```text
mine-plan-review complete release closure
```

此模式不要求 Plan 路径，也不会重新接受已接受的 Plan。仅当所有 Plan 均为终态且最终同步已完成时才会进入；缺失或过期的同步会明确停止收口。收口步骤：

1. 确认最终 `mine-sync` 已完成（审查者不运行它）；
2. 运行 `mine release --format json` 预检加上仓库自身的决定性验证；
3. 确定下一个 MINE 代码仓库版本；
4. 安全清除 MINE 所有的 `docs/plan/` 工作区；
5. 验证 stable 发布树不含 Plan 文件或本地备份；
6. 将已接受状态集成到 stable 分支，不导入临时 Plan 历史；
7. 删除临时受管分支。

审查者从不 push、不创建远程发布、不发布包。远程发布明确在 MINE 权限之外。

stable 树只保留代码和 `docs/design/`，不保留产生它们的过程。

## 安装与生命周期

bootstrap 安装后（见 README），以下 CLI 命令管理 MINE 生命周期：

```bash
mine --version          # 验证已安装的二进制
mine agent status       # 机器级：列出受管的 Agent 集成和健康状态
mine setup              # 机器级：（重新）安装 MINE 到编码 Agent（交互式）
mine setup --agents claude-code,codex --yes  # 非交互式，指定 Agent
mine update             # 机器级：更新二进制到最新发布
mine uninstall          # 机器级：从所有 Agent 和本机移除 MINE
```

### 机器级 vs 仓库级

MINE 有三个不同的操作层级，不要混淆：

- **`mine setup`** - **机器级** Agent 集成。将 Skills 和 MCP 配置安装到编码 Agent 客户端目录。每台机器运行一次；重跑以添加或移除集成。
- **`mine init`** - **仓库级**初始化。在仓库根目录创建 `.mine/config.toml`、设计命名空间和治理。每个仓库运行一次。
- **`mine agent status`** - **机器级**状态。列出受管 Agent 安装及其健康状态，与仓库无关。
- **`mine doctor`** - **仓库感知**诊断。对已初始化的 MINE 仓库做完整健康检查：`.mine/config.toml`、设计标记/索引、执行图、Git 分支，以及（带 `--agents`）每个 Agent 的安装状态。在仓库内运行；它不是机器级命令。

`mine doctor --agents all` 是对已初始化仓库的仓库感知诊断，不是全局安装后检查。全新机器级安装后，用 `mine agent status` 验证集成。

非交互式标志：`--agents <列表>`（逗号分隔的 slug）、`--yes`（跳过提示）、`--config-root <路径>`（CI/测试的隔离安装）。

## 手动检查命令

这些对正常流程有用但非必需。它们都接受 `--format json` 以输出稳定的机器可消费 envelope（以及 `--repo <path>` 指定其他仓库根）；本指南展示人类可读形式。

```bash
mine status
mine doctor
mine design status
mine design validate
mine graph status
mine graph ready
mine graph wave
mine plan show --id <id>
```

面向 Agent 的变更通过已接受的 `mine` CLI 子命令（`mine plan add|start|implemented|accept|reject`、`mine graph render`、`mine workspace open|status|close`、`mine design backup|validate|status`、`mine repository version show|suggest|set`），全部使用 `--format json`。当接受类型化 MCP 桥接时优先使用它，并回退到 `--format json` CLI。绝不直接编辑 `docs/plan/execution-graph.toml` 或 `docs/plan/execution-graph.md`。

## 受支持客户端

仅支持 Claude Code、Codex、Pi 和 OpenCode。其他环境被有意排除在 MINE 的兼容性负担之外。

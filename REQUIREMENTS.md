# MINE 产品与实现需求

> **MINE Is Not Everyone’s.**
>
> 本仓库只适配 Claude Code、Codex、Pi 和 OpenCode。
> 不为 Cursor、Windsurf、Cline 或其他 Coding Agent 提供兼容层。其他人如有需要，可以 fork 后自行维护。

---

## 0. 文档定位

本文是 `mine-is-not-everyones` 仓库的产品需求与实现约束，适用于仓库内全部 Rust 程序、Skills、插件发行物、安装脚本和文档。

实现 Agent 必须先完整阅读：

- `REQUIREMENTS.md`；
- `README.md`；
- `skills/` 下的四个核心 Skill；
- `scripts/`；
- 已有插件 manifest；
- `docs/design/architecture-and-detailed-design.md`（存在时）；
- 根 `AGENTS.md`（存在时）。

不得将 Rust CLI 另建到其他仓库。Rust 程序、Skills 和四个平台发行层必须共同存在于本仓库，并使用同一版本号发布。

需求中出现的第三方命令、配置路径和 manifest 结构，实施前必须联网核对对应平台的最新官方文档。若官方接口发生变化，应在架构文档记录差异和采用方案，但不得擅自改变本文定义的产品目标。

---

## 1. 产品名称与哲学

项目名称：`MINE`

全称：`MINE Is Not Everyone’s`

GitHub 仓库：

```text
6ixGODD/mine-is-not-everyones
```

MINE 是一套为仓库工程治理设计的个人 Coding Agent 工作流。它不追求适合所有人，也不追求成为通用项目管理软件。

核心原则：

1. 架构是事实源，不是装饰文档；
2. Plan 是可执行契约，不是模糊任务列表；
3. 外部技术结论必须检索并精读官方资料；
4. 执行图属于确定性状态，不允许模型自由手写；
5. Agent 负责判断，程序负责状态机、图算法和并发安全；
6. 新项目默认无历史兼容包袱；
7. 证据优先于自信，独立审查优先于“测试看起来通过”；
8. 只适配作者实际使用的四个平台。

---

## 2. 最终产品组成

MINE 由三部分组成。

### 2.1 四个核心 Skill

v1 必须且只保留以下四个核心 Skill：

| Skill | 职责 |
|---|---|
| `mine-arch` | 创建或维护架构与详细设计，并初始化仓库工程规范 |
| `mine-plan-create` | 基于需求、代码、架构和官方资料创建可执行 Plan |
| `mine-plan-exec` | 领取并执行一个 Plan，完成验证和实施报告 |
| `mine-plan-review` | 独立审查实施结果，接受或拒绝 Plan |

暂不新增 `mine-doctor`、`mine-install`、`mine-graph`、`mine-repo-audit` 或其他 Skill。

原因：安装、诊断、图计算、状态迁移、文件同步和发行构建都是确定性操作，应由 Rust CLI 负责，不应浪费模型上下文，也不应产生职责重叠。

未来只有在出现稳定、反复、且明显需要模型判断的新工作流时，才考虑增加 Skill。

### 2.2 Rust 单文件 CLI / MCP Server

仓库生成单一可执行文件：

```text
mine
```

同一二进制同时提供：

- 人类可用 CLI；
- Agent 可调用的稳定 JSON CLI；
- stdio MCP Server；
- 执行图存储和状态机；
- 图校验、拓扑排序、并行 wave 和写路径冲突检测；
- 四个平台的安装、配置、诊断和卸载辅助；
- Skills 与插件发行物同步和验证；
- GitHub Release 单文件安装支持。

### 2.3 四个平台发行层

仅支持：

1. Claude Code；
2. Codex；
3. Pi；
4. OpenCode。

四个平台应共享根 `skills/` 的内容，不允许人工维护四套相互漂移的 Skill 文本。

---

## 3. 仓库最终结构

目标结构如下。允许架构阶段细化 Rust 源码模块，但产品目录与事实源边界不得随意改变。

```text
mine-is-not-everyones/
├── Cargo.toml
├── Cargo.lock
├── rust-toolchain.toml
├── REQUIREMENTS.md
├── README.md
├── LICENSE
├── AGENTS.md
├── package.json                         # Pi Git package manifest
│
├── src/                                 # Rust mine CLI / MCP
├── tests/
│
├── skills/                              # 四个 Skill 的唯一人工维护源
│   ├── mine-arch/
│   ├── mine-plan-create/
│   ├── mine-plan-exec/
│   └── mine-plan-review/
│
├── plugins/
│   └── mine/                            # 发行构建生成，禁止手工漂移
│       ├── .claude-plugin/plugin.json
│       ├── .codex-plugin/plugin.json
│       ├── .mcp.json                    # 平台支持时使用
│       └── skills/                      # 从根 skills/ 生成
│
├── .claude-plugin/
│   └── marketplace.json                 # Claude Marketplace catalog
│
├── .agents/
│   └── plugins/
│       └── marketplace.json             # Codex repo/team marketplace catalog
│
├── scripts/
│   ├── bootstrap.ps1
│   ├── bootstrap.sh
│   └── legacy helpers                   # 可在 Rust install 稳定后精简
│
├── docs/
│   ├── design/
│   │   └── architecture-and-detailed-design.md
│   ├── user-guide.md
│   └── plan/
│       ├── execution-graph.toml
│       ├── execution-graph.md
│       └── reports/
│
└── .mine/
    ├── config.toml
    ├── runtime/                         # 不提交 Git
    └── locks/                           # 不提交 Git
```

### 3.1 唯一源与生成物

以下内容是人工维护源：

```text
skills/
```

以下内容是发行生成物：

```text
plugins/mine/skills/
```

必须提供确定性的同步命令，例如：

```bash
mine dist sync
mine dist verify
```

`mine dist verify` 必须在 CI 中检查：

- 根 Skills 与插件 Skills 字节一致或语义一致；
- manifest 中声明的 Skill 均存在；
- 不存在额外、陈旧、漏复制的 Skill；
- `name` 与目录名一致；
- 本地 Markdown 引用存在；
- Skill 中的 CLI 命令和 MCP 工具名属于当前公开契约。

---

## 4. 非目标

v1 明确不做：

- 通用项目管理平台；
- Web UI；
- 云端多用户协作服务；
- 自动编写需求、架构或 Plan；
- 自动修改业务代码；
- 任意 shell 执行 MCP 工具；
- 自动 push、merge、rebase、reset、clean 或 stash；
- 自动启动多个 Agent 进程；
- 自动创建、合并或清理 Git worktree；
- 自动集成并行分支；
- SQLite、远程数据库或自定义二进制图格式；
- HTTP MCP Server；
- Cursor 等其他 Agent 适配；
- 为未发布内部格式提供无条件兼容层。

后续 worktree 调度、Agent 进程监管和自动集成可以作为独立版本设计，不得污染 v1 执行图内核。

---

## 5. 技术栈与工程原则

### 5.1 Rust

- Rust stable；
- Rust 2024 edition；
- 通过 `rust-toolchain.toml` 固定工具链；
- Windows 为一等支持平台；
- Linux 和 macOS 必须通过 CI；
- 本项目业务代码禁止 `unsafe`；
- 不过早拆多 crate，v1 默认单 package；
- CLI 与 MCP 必须共用 application/domain 层，禁止两套状态机。

推荐依赖需在实施时核对最新官方资料：

- `clap`；
- `serde`、`serde_json`、`toml`；
- `thiserror`；
- `tracing`、`tracing-subscriber`；
- 跨平台文件锁库；
- `tempfile` 或等价原子写方案；
- `petgraph` 或内部简单图实现；
- `tokio`；
- 官方 MCP Rust SDK（当前通常为 `rmcp`）；
- `uuid`；
- `time` 或 `chrono`；
- `assert_cmd`、`predicates`、`proptest`。

不得自行手写完整 MCP/JSON-RPC 协议栈。

### 5.2 分层

```text
CLI Adapter ─┐
             ├──> Application Service ──> Domain ──> Repository Port
MCP Adapter ─┘                                      │
                                                   └──> TOML Store / Git / Lock
```

领域层不得依赖：

- clap；
- MCP SDK；
- 文件系统具体实现；
- Git 子进程；
- Claude/Codex/Pi/OpenCode 配置格式。

---

## 6. 执行图持久化

### 6.1 机器事实源

固定路径：

```text
docs/plan/execution-graph.toml
```

它必须提交 Git，是唯一执行图事实源。

### 6.2 人类与 Agent 阅读视图

固定路径：

```text
docs/plan/execution-graph.md
```

该文件由 `mine graph render` 自动生成，也提交 Git，用于 PR diff、人工阅读和 Agent 上下文。

顶部必须明确写明：

```text
GENERATED FILE. DO NOT EDIT.
Source: docs/plan/execution-graph.toml
```

任何 Skill 和 Agent 都不得直接编辑 `execution-graph.md`。

### 6.3 为什么不使用 SQLite

执行图需要：

- 与 Plan 一起进入 Git；
- 可读 diff；
- 分支切换时自然变化；
- 人类和 Agent 可直接审计；
- 冲突可人工解决；
- 规模通常只有几十至数百节点。

因此 v1 禁止 SQLite 和自定义二进制格式。

### 6.4 `.mine/` 目录

项目必须创建：

```text
.mine/config.toml
.mine/runtime/events.jsonl
.mine/runtime/mine.log
.mine/locks/execution-graph.lock
```

Git 规则：

必须提交：

```text
.mine/config.toml
docs/plan/execution-graph.toml
docs/plan/execution-graph.md
```

不得提交：

```text
.mine/runtime/
.mine/locks/
```

优先在 `.mine/.gitignore` 中忽略 `runtime/` 和 `locks/`，避免无意义修改根 `.gitignore`。

运行日志和事件日志不是事实源，删除或损坏不得阻止图操作。

---

## 7. 执行图数据模型

顶层至少包含：

```toml
schema_version = 1
revision = 7
project_id = "mine-is-not-everyones"
updated_at = "2026-07-23T12:00:00Z"
```

每个 Plan 节点至少包含：

```toml
[[plans]]
id = "02"
path = "docs/plan/02-domain-model.md"
title = "Domain model"
status = "READY"
hard_predecessors = ["01"]
soft_predecessors = []
exclusive_write_paths = ["src/domain/"]
read_only_paths = ["docs/design/"]
reserved_shared_paths = ["Cargo.toml", "Cargo.lock"]
implementation_report = ""
review_report = ""
implementation_commits = []
owner = ""
run_id = ""
started_at = ""
updated_at = "2026-07-23T12:00:00Z"
rejection_reason = ""
compensating_plan = ""
```

### 7.1 状态

合法状态：

- `DRAFT`；
- `BLOCKED`；
- `READY`；
- `IN_PROGRESS`；
- `IMPLEMENTED`；
- `ACCEPTED`；
- `REJECTED`。

禁止提供任意 `set-status`。

状态只能通过语义命令迁移。

### 7.2 状态规则

- `DRAFT -> READY/BLOCKED`：Plan 完成并注册后由依赖和门禁计算；
- `BLOCKED -> READY`：全部硬前驱被接受且外部门禁满足；
- `READY -> IN_PROGRESS`：执行者成功领取；
- `IN_PROGRESS -> IMPLEMENTED`：实施报告和 commit 证据登记；
- `IMPLEMENTED -> ACCEPTED`：独立审查通过；
- `IMPLEMENTED -> REJECTED`：独立审查发现硬失败；
- `REJECTED` 不直接恢复为 `READY`，应创建补偿 Plan；
- `ACCEPTED` 为历史终态，除非未来有明确迁移设计。

### 7.3 revision

每次成功写操作：

- 校验调用者给出的 `expected_revision`；
- 成功后 `revision + 1`；
- revision 不一致返回结构化冲突错误；
- 不允许静默覆盖。

---

## 8. 图算法和并行性

必须实现：

- ID 和路径唯一性校验；
- 缺失前驱检测；
- 环检测；
- 拓扑排序；
- READY frontier；
- 可并行 wave；
- 祖先/后代关系；
- 状态与前驱一致性；
- 路径越界校验；
- 写路径冲突检测。

两个 Plan 不得处于同一并行 wave，当以下任一成立：

1. `exclusive_write_paths` 重叠；
2. 一个 Plan 的独占写路径与另一个的 `reserved_shared_paths` 重叠；
3. 与当前 `IN_PROGRESS` 节点写范围冲突；
4. 两者存在硬依赖祖先关系。

路径必须是仓库相对路径，禁止：

```text
../other-repo
C:\absolute\path
/**/*.rs
```

v1 可采用保守的目录前缀冲突算法，不能为追求最大并行度而错误放行。

---

## 9. 并发、锁和原子写入

所有写命令必须：

1. 获取 `.mine/locks/execution-graph.lock` 独占锁；
2. 默认等待不超过配置时间；
3. 获取锁后重新读取 TOML；
4. 校验 `expected_revision`；
5. 完成领域校验；
6. 写临时文件；
7. flush，并在可行时同步；
8. 原子替换 TOML；
9. 生成 Markdown；
10. 释放锁。

不得 truncate 后原地写事实源。

如果 TOML 成功而 Markdown 生成失败：

- TOML 仍为事实源；
- 返回部分成功错误；
- 提示运行 `mine graph render` 修复；
- 不得回滚为未知状态。

所有写命令支持 `--dry-run`。

---

## 10. 核心 CLI 契约

### 10.1 全局参数

至少支持：

```text
--repo <path>
--format human|json
--no-color
--dry-run
--expected-revision <n>
```

仓库发现顺序：

1. `--repo`；
2. 向上寻找 `.mine/config.toml`；
3. 向上寻找 `docs/plan/execution-graph.toml`；
4. 向上寻找 `.git`；
5. 否则返回 `MINE_REPOSITORY_NOT_FOUND`。

### 10.2 必须实现的命令

```bash
mine init
mine doctor

mine graph validate
mine graph render
mine graph status
mine graph show
mine graph ready
mine graph wave

mine plan add
mine plan show <id-or-path>
mine plan start <id>
mine plan implemented <id>
mine plan accept <id>
mine plan reject <id>

mine mcp serve

mine agent config <claude|codex|pi|opencode>
mine agent install --agent <name|all>
mine agent uninstall --agent <name|all>

mine dist sync
mine dist verify
```

### 10.3 `mine init`

必须：

- 幂等；
- 创建 `.mine/`；
- 创建空 TOML，`revision = 0`；
- 创建生成 Markdown；
- 创建配置；
- 不覆盖已有非空执行图；
- 创建必要的 ignore 规则；
- 检查架构文档和 `AGENTS.md`，但不代替 `mine-arch` 写架构。

### 10.4 `mine plan add`

输入至少包括：

- Plan 路径；
- title；
- hard/soft predecessors；
- exclusive/read-only/reserved paths；
- 初始状态建议。

命令应根据依赖自动确定 `BLOCKED` 或 `READY`，不得信任 Agent 直接指定不合法 READY。

### 10.5 `mine plan start`

至少要求：

- 当前状态为 `READY`，或同一 owner/run 恢复既有 `IN_PROGRESS`；
- owner 非空；
- 不与活跃计划写范围冲突；
- 生成 `run_id`；
- 写入 `started_at`。

### 10.6 `mine plan implemented`

至少要求：

- 当前状态为 `IN_PROGRESS`；
- implementation report 存在；
- 至少一个 implementation commit；
- 可选严格 Git 校验；
- 状态变为 `IMPLEMENTED`。

### 10.7 `mine plan accept/reject`

accept：

- 仅允许 `IMPLEMENTED`；
- review report 必须存在；
- 状态变为 `ACCEPTED`；
- 自动重新计算下游 READY/BLOCKED。

reject：

- 仅允许 `IMPLEMENTED`；
- review report 和 rejection reason 必须存在；
- 状态变为 `REJECTED`；
- 下游继续阻塞；
- 支持登记 compensating plan，但不自动创建 Plan 文档。

---

## 11. JSON 输出契约

所有命令在 `--format json` 下输出单个 JSON envelope，stdout 不得混入日志。

成功示例：

```json
{
  "ok": true,
  "command": "plan.start",
  "repository": "D:/WorkSpace/project",
  "revision_before": 7,
  "revision_after": 8,
  "data": {
    "plan_id": "02",
    "status_before": "READY",
    "status_after": "IN_PROGRESS",
    "owner": "codex",
    "run_id": "..."
  },
  "warnings": []
}
```

失败示例：

```json
{
  "ok": false,
  "command": "plan.start",
  "error": {
    "code": "MINE_REVISION_CONFLICT",
    "message": "expected revision 7, actual revision 8",
    "details": {}
  }
}
```

必须为稳定错误分类定义机器码，包括但不限于：

- `MINE_REPOSITORY_NOT_FOUND`；
- `MINE_GRAPH_NOT_INITIALIZED`；
- `MINE_GRAPH_INVALID`；
- `MINE_GRAPH_CYCLE`；
- `MINE_PLAN_NOT_FOUND`；
- `MINE_INVALID_TRANSITION`；
- `MINE_PREDECESSOR_NOT_ACCEPTED`；
- `MINE_WRITE_SCOPE_CONFLICT`；
- `MINE_REVISION_CONFLICT`；
- `MINE_LOCK_TIMEOUT`；
- `MINE_EVIDENCE_MISSING`；
- `MINE_AGENT_CONFIG_CONFLICT`；
- `MINE_DISTRIBUTION_DRIFT`。

---

## 12. MCP Server

### 12.1 传输

v1 仅实现 stdio：

```bash
mine mcp serve --repo <project>
```

MCP 日志只能写 stderr 或文件，绝不能污染 stdout。

### 12.2 工具

总数控制在 12 个以内，至少包含：

- `mine_graph_validate`；
- `mine_graph_status`；
- `mine_graph_ready`；
- `mine_graph_wave`；
- `mine_plan_get`；
- `mine_plan_add`；
- `mine_plan_start`；
- `mine_plan_mark_implemented`；
- `mine_plan_accept`；
- `mine_plan_reject`；
- `mine_graph_render`；
- `mine_doctor`（若数量允许）。

MCP Tool 必须直接调用与 CLI 相同的 application service。

所有写工具必须支持 `expected_revision`。

禁止 MCP 暴露：

- 任意 shell；
- 任意文件写入；
- Git destructive operation；
- 自定义无约束状态设置。

---

## 13. 四个 Skill 与 `mine` 的强制集成

这是 v1 必须交付，不是可选优化。

### 13.1 总规则

Rust CLI/MCP 的最终命令、参数、错误码和工具名稳定后，实现 Agent 必须回头修改根 `skills/` 下四个 `SKILL.md`。

修改后的 Skill 必须：

1. 使用最终真实命令和 MCP Tool 名，不保留占位符；
2. 优先使用 MCP；
3. MCP 不可用时使用 `mine --format json` CLI；
4. 不解析 human 输出；
5. 不直接编辑 `execution-graph.toml`；
6. 不直接编辑 `execution-graph.md`；
7. 读取 revision，并在写操作携带 `expected_revision`；
8. 对结构化错误做明确分支；
9. `mine` 不存在时停止图状态变更，并给出安装/诊断命令；
10. 修改后执行 `mine dist sync` 和 `mine dist verify`。

CI 必须扫描并拒绝以下陈旧文本：

- “直接更新 execution-graph.md”；
- “手工将节点改为 ACCEPTED”；
- 未定义的 `mine` 子命令；
- 未定义的 MCP Tool 名；
- 将 Markdown 视为事实源。

### 13.2 `mine-arch`

职责仍然是架构与仓库工程初始化。

新增要求：

- 检测 `mine` 是否可用；
- 在新仓库或未初始化仓库中调用 `mine init`；
- 在 `AGENTS.md` 写明 TOML/Markdown 边界；
- 写明所有状态变更必须通过 `mine`；
- 不自行创建或修改 Plan 节点；
- 不让 `mine init` 替代架构设计。

### 13.3 `mine-plan-create`

完成 Plan 文档后必须：

1. 提取 Plan ID、路径、title、前驱和文件所有权；
2. 调用 `mine_plan_add`，或 CLI `mine plan add --format json`；
3. 调用 graph validate；
4. 将返回的节点状态和 revision 写入最终 handoff；
5. 不直接维护 TOML 或 Markdown；
6. 架构变化仍必须先修改架构文档。

### 13.4 `mine-plan-exec`

执行前必须：

1. validate；
2. 获取计划详情和当前 revision；
3. 调用 `mine_plan_start`；
4. 只有成功进入 `IN_PROGRESS` 才能修改代码。

完成后：

1. 创建实施报告；
2. 完成被授权的 commit；
3. 调用 `mine_plan_mark_implemented`；
4. revision 冲突时停止并重新检查，不得覆盖他人状态。

### 13.5 `mine-plan-review`

审查前：

- validate；
- 获取节点、计划、报告、commit 和 revision。

通过时调用 `mine_plan_accept`。

拒绝时：

- 写独立 review report；
- 必要时更新架构并创建补偿 Plan 文档；
- 先注册补偿 Plan，再调用 `mine_plan_reject` 登记关系，具体顺序由架构设计确定但必须原子一致或可恢复；
- 不直接手写图状态。

---

## 14. Skills 数量决策

v1 最终为四个 Skill。

不新增 `mine-plan-integrate`：v1 不自动管理 worktree 和并行分支集成，集成工作由计划中明确的 owner 作为普通 `mine-plan-exec` 节点执行。

不新增 `mine-repo-audit`：架构审计由 `mine-arch` 覆盖，确定性检查由 `mine doctor` 和 `mine graph validate` 覆盖。

若未来重复出现独立、稳定的“并行集成”或“仓库审计”模型工作流，再单独立项，不能预先堆 Skill。

---

## 15. Claude Code 发行与安装

### 15.1 Marketplace

仓库必须提供：

```text
.claude-plugin/marketplace.json
plugins/mine/.claude-plugin/plugin.json
plugins/mine/skills/
```

Marketplace 名称：

```text
mine-is-not-everyones
```

Plugin 名称：

```text
mine
```

插件必须包含四个 Skill，并在 Claude 官方支持的方式下注册本地 stdio MCP Server，前提是 `mine` 二进制已在 PATH。

实现阶段必须依照最新 Claude 官方文档验证 manifest，不得照抄过期字段。

### 15.2 目标安装体验

Native Marketplace 路线：

```text
/plugin marketplace add 6ixGODD/mine-is-not-everyones
/plugin install mine@mine-is-not-everyones
/reload-plugins
```

插件模式的命令预期带 namespace：

```text
/mine:mine-arch
/mine:mine-plan-create
/mine:mine-plan-exec
/mine:mine-plan-review
```

个人短命令路线由统一安装器提供，将 Skill 安装为 standalone：

```text
/mine-arch
/mine-plan-create
/mine-plan-exec
/mine-plan-review
```

README 必须明确两种模式不能无意重复安装，以免出现重名 Skill。

---

## 16. Codex 发行与安装

### 16.1 Plugin

仓库必须提供：

```text
.agents/plugins/marketplace.json
plugins/mine/.codex-plugin/plugin.json
plugins/mine/skills/
plugins/mine/.mcp.json（当前 Codex manifest 支持时）
```

插件必须声明四个 Skill 和 `mine mcp serve`。

实施时必须：

- 使用 Codex 官方 plugin creator/spec 校验 manifest；
- 使用当前 Codex 版本真实安装测试；
- 记录 plugin marketplace 当前已知限制；
- 不因“插件显示已启用”就认为 Skills 一定进入会话；
- `mine doctor --agents codex` 必须检查 Skill 的实际可发现性。

### 16.2 目标安装体验

优先目标是 Codex 原生 Marketplace/Plugin 安装。具体命令必须在实现时以最新官方 CLI 为准并写入 README。

统一安装器必须提供稳定兜底：

```bash
mine agent install --agent codex
```

兜底可将根 Skills 链接或复制到 Codex 支持的用户 Skill 目录，并写入 MCP 配置。

Codex 中明确调用 Skill 的目标形式：

```text
$mine-arch
$mine-plan-create
$mine-plan-exec
$mine-plan-review
```

若当前 Codex 版本调用语法不同，README 必须使用实测语法。

---

## 17. Pi 发行与安装

Pi 使用仓库根 `package.json` 作为 Git package manifest。

必须声明：

```json
{
  "pi": {
    "skills": ["./skills"]
  }
}
```

目标安装命令：

```bash
pi install git:github.com/6ixGODD/mine-is-not-everyones
```

或固定 release tag：

```bash
pi install git:github.com/6ixGODD/mine-is-not-everyones@v1.0.0
```

Pi 的明确调用形式：

```text
/skill:mine-arch
/skill:mine-plan-create
/skill:mine-plan-exec
/skill:mine-plan-review
```

Pi 当前没有被本项目假定为原生 MCP 客户端。v1 支持：

1. 检测已安装的 MCP adapter 并配置 `mine mcp serve`；或
2. Skill 通过 Pi 的 shell/bash 工具调用 JSON CLI。

项目保持 Rust 核心，不为 Pi 写一套独立 TypeScript 业务状态机。

---

## 18. OpenCode 发行与安装

OpenCode 可发现 Claude-compatible Skills。

统一安装器应优先选择一种且只选择一种路径：

```text
~/.config/opencode/skills/
```

或复用：

```text
~/.claude/skills/
```

不得同时安装两份同名 Skill。

同时写入 OpenCode 的本地 MCP 配置，使其启动：

```text
mine mcp serve --repo <current-project>
```

目标安装：

```bash
mine agent install --agent opencode
```

OpenCode 中的推荐使用方式是自然语言明确要求：

```text
Use the mine-arch skill to initialize this repository.
```

或使用 OpenCode 当前官方支持的 skill tool 进行加载。README 必须写实测方式，不虚构 Slash Command。

OpenCode npm plugin 不是 v1 必需品，因为核心能力已经由 Skills + MCP 提供。

---

## 19. 统一安装器与最终安装体验

### 19.1 Bootstrap

GitHub Releases 必须发布：

- Windows x86_64 二进制；
- Linux x86_64；
- Linux aarch64（可行时）；
- macOS x86_64 和 aarch64；
- SHA256 checksums。

提供：

```powershell
irm https://raw.githubusercontent.com/6ixGODD/mine-is-not-everyones/main/scripts/bootstrap.ps1 | iex
```

```bash
curl -fsSL https://raw.githubusercontent.com/6ixGODD/mine-is-not-everyones/main/scripts/bootstrap.sh | sh
```

Bootstrap 只负责：

1. 检测平台；
2. 下载并校验 `mine`；
3. 放入用户 PATH；
4. 调用 Rust 命令完成剩余安装。

### 19.2 Rust 安装命令

目标体验：

```bash
mine agent install --agent all
mine doctor --agents all
```

安装器必须：

- 幂等；
- 支持 dry-run；
- 修改配置前备份；
- 保留所有无关配置；
- 同名不同配置默认拒绝，除非 `--force`；
- 路径含空格时正确处理；
- 卸载只移除 MINE 管理项；
- 输出每个平台的实际安装模式和调用方式。

推荐作者本人日常使用统一安装器，以获得短 Skill 名。Marketplace 主要用于原生分发和验证。

---

## 20. 最终用户操作手册

实现完成后 `docs/user-guide.md` 和 README 必须给出以下完整闭环。

### 20.1 第一次安装

```powershell
irm https://raw.githubusercontent.com/6ixGODD/mine-is-not-everyones/main/scripts/bootstrap.ps1 | iex
mine agent install --agent all
mine doctor --agents all
```

### 20.2 在业务仓库初始化

进入目标业务仓库后，在任意 Agent 中调用 `mine-arch`。

Skill 应完成：

- 仓库审计；
- 架构文档；
- `AGENTS.md`；
- 质量配置；
- `mine init`；
- 验证执行图初始化。

也可人工执行：

```bash
mine init
mine graph validate
```

### 20.3 制定计划

Claude standalone：

```text
/mine-plan-create <需求>
```

Claude plugin：

```text
/mine:mine-plan-create <需求>
```

Codex：

```text
$mine-plan-create <需求>
```

Pi：

```text
/skill:mine-plan-create <需求>
```

OpenCode：明确要求加载 `mine-plan-create`。

完成后检查：

```bash
mine graph status
mine graph ready
mine graph wave
```

### 20.4 执行计划

选择 READY Plan，然后调用 `mine-plan-exec` 并传 Plan 路径。

Skill 内部必须先成功执行 `mine plan start`，才能改代码。

人工查询：

```bash
mine plan show 02
mine graph wave
```

### 20.5 独立审查

换一个 Agent 或独立会话调用 `mine-plan-review`。

通过后节点变为 `ACCEPTED`，下游自动释放；拒绝后生成补偿 Plan。

### 20.6 日常诊断和更新

```bash
mine doctor --agents all
mine graph validate
mine dist verify
```

更新二进制和插件的最终命令由实现确定，但必须在 README 中提供一条明确路径，不能让用户手动猜缓存位置。

---

## 21. `mine doctor`

至少检查：

- `mine` 版本和 PATH；
- 当前仓库；
- `.mine/config.toml`；
- TOML 和生成 Markdown revision；
- 图结构和状态；
- Git 可用性；
- 四个根 Skill；
- plugin 发行物漂移；
- Claude Marketplace/plugin；
- Codex marketplace/plugin 和 Skill 实际发现；
- Pi package/Skill 和 MCP adapter 或 CLI fallback；
- OpenCode Skill 和 MCP；
- 重复安装和命名冲突；
- 二进制路径引用失效。

每项输出：

```text
PASS
WARN
FAIL
SKIP
```

JSON 模式必须可由测试消费。

---

## 22. Git 集成

v1 只允许使用有限只读 Git 命令验证证据，例如：

```text
git rev-parse --show-toplevel
git rev-parse --verify <commit>^{commit}
git merge-base --is-ancestor <commit> HEAD
git ls-files --error-unmatch <path>
git status --porcelain -- <path>
```

必须通过参数数组启动，不得拼接 shell 字符串。

v1 的 `mine` 不得执行：

```text
git add
git commit
git push
git pull
git merge
git rebase
git reset
git clean
git checkout
git switch
git stash
```

Skills 是否 commit 由其自身治理规则和用户授权决定，CLI 只记录/验证 commit。

---

## 23. 配置

`.mine/config.toml` 示例：

```toml
schema_version = 1

[graph]
source = "docs/plan/execution-graph.toml"
rendered = "docs/plan/execution-graph.md"
lock_timeout_ms = 5000
render_after_write = true

[git]
required = true
strict_commit_ancestry = true
require_tracked_plan_files = true

[output]
default_format = "human"
color = "auto"

[integration]
prefer_mcp = true
allow_cli_fallback = true
```

优先级：

1. CLI 参数；
2. `.mine/config.toml`；
3. 内置默认值。

环境变量只允许少量进程级覆盖，例如：

```text
MINE_REPO
MINE_LOG
MINE_NO_COLOR
```

不得在配置和日志中写秘密。

---

## 24. 日志与审计

本地事件日志：

```text
.mine/runtime/events.jsonl
```

至少记录：

- 命令或 MCP Tool；
- 节点；
- revision 前后；
- 状态变化；
- owner/run_id；
- 结果和错误码；
- 时间。

事件日志不得记录：

- Prompt 正文；
- 业务代码；
- 秘密；
- 用户私人数据；
- 完整第三方 payload。

事件日志不是事实源。

---

## 25. 安全要求

- 所有仓库路径做 canonicalization 和越界检查；
- 不跟随路径逃逸到仓库外；
- Agent 配置修改前备份；
- 不覆盖未知同名 MCP server；
- MCP 不提供任意 shell；
- stdout/stderr 严格隔离；
- 临时文件使用不可预测名称；
- 配置写入使用原子替换；
- Release checksum 必须验证；
- 第三方依赖执行 `cargo audit` 或等价检查；
- Skills 和插件拥有高权限，README 必须提示用户审查来源。

---

## 26. 测试要求

### 26.1 单元测试

- 状态机每条合法/非法路径；
- 图环、缺依赖和拓扑；
- path conflict；
- revision；
- READY/BLOCKED 计算；
- deterministic render；
- error mapping。

### 26.2 性质测试

至少覆盖：

- 任意 DAG 的拓扑序合法；
- 同一输入 render 稳定；
- revision 单调递增；
- 非法状态迁移永不成功；
- wave 内无已知写冲突。

### 26.3 集成测试

- `mine init` 幂等；
- CLI 完整生命周期；
- JSON envelope；
- 两个并发写不会静默覆盖；
- lock timeout；
- 原子写恢复；
- MCP tools/list；
- MCP start -> implemented -> accept；
- stdout 无日志污染；
- Skills 契约扫描；
- dist sync/verify；
- 四个平台 config 生成和安装的 fixture 测试。

### 26.4 平台验收

至少在真实或 CI 环境测试：

- Windows PowerShell；
- Ubuntu；
- macOS；
- Claude Code 当前稳定版；
- Codex 当前稳定版；
- Pi 当前稳定版；
- OpenCode 当前稳定版。

配置“写成功”不能代替“模型能发现 Skill/Tool”的真实测试。

---

## 27. CI 和发布

PR 必须运行：

```text
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
mine dist verify（构建出 mine 后）
manifest/schema validation
Skill contract validation
```

Release 必须：

- 使用 tag；
- 构建多平台二进制；
- 生成 checksums；
- 验证 bootstrap；
- 验证 Claude/Codex marketplace manifest；
- 验证 Pi package；
- 更新 changelog；
- 记录四个平台兼容版本。

MINE 自身是新项目，不承诺 v1 前内部格式兼容。首个正式 release 后，schema 变更必须提供显式版本和迁移命令，不能静默重写。

---

## 28. 实施阶段

### Milestone 0：研究和架构

交付：

- 精读本文和现有 Skills；
- 联网核对 Rust/MCP/四个平台官方资料；
- 创建 `docs/design/architecture-and-detailed-design.md`；
- 创建/更新 `AGENTS.md`；
- 明确模块边界、错误模型和公开契约；
- 将以下 Milestone 变成 MINE Plan。

禁止直接无计划堆 CLI handler。

### Milestone 1：执行图内核与 JSON CLI

交付：

- TOML 模型；
- 状态机；
- 图算法；
- path conflict；
- revision；
- 锁和原子写；
- Markdown render；
- 核心 graph/plan CLI；
- JSON envelope；
- Git 证据验证；
- 完整测试。

同时第一次修改四个 Skill，使其使用最终 CLI JSON，不再手写图。

M1 完成后，MINE 必须能够使用 MINE 管理自己的后续 Plan。

### Milestone 2：MCP

交付：

- stdio MCP；
- 最多 12 个工具；
- CLI/MCP 共用 application service；
- MCP 集成测试；
- 再次修改四个 Skill：MCP 优先、CLI fallback；
- Skill 契约自动验证。

### Milestone 3：发行物和四端安装

交付：

- `mine dist sync/verify`；
- Claude Marketplace；
- Codex Plugin/Marketplace 和稳定 fallback；
- Pi Git package；
- OpenCode Skills + MCP；
- `mine agent config/install/uninstall`；
- `mine doctor`；
- README 与用户手册；
- 平台真实发现测试。

### Milestone 4：Release

交付：

- CI；
- GitHub Release；
- 多平台二进制；
- checksums；
- bootstrap 脚本；
- 从零 10 分钟安装验收；
- 四平台兼容矩阵。

不得在 M1 内提前实现复杂平台安装器。

---

## 29. 最终验收标准

v1 只有全部满足才完成：

1. 四个 Skill 且只有四个核心 Skill；
2. 根 `skills/` 是唯一人工维护源；
3. Rust 二进制可在 Windows/Linux/macOS 构建；
4. `mine init` 幂等；
5. 执行图 TOML 是事实源，Markdown 是稳定生成物；
6. 图可处理至少 1000 个节点；
7. 环、缺依赖、非法状态、越界路径和冲突准确报错；
8. 完整生命周期可通过 CLI；
9. 并发写不会静默覆盖；
10. revision 冲突可稳定复现；
11. MCP 完成完整生命周期；
12. stdout 不被日志污染；
13. 四个 Skill 不再直接修改执行图；
14. Skills 使用真实最终命令和工具名；
15. Claude Marketplace 可安装并发现四个 Skill；
16. Codex plugin 或稳定 fallback 可发现四个 Skill；
17. Pi Git package 可安装四个 Skill；
18. OpenCode 可发现四个 Skill 并调用 MCP/CLI；
19. `mine doctor --agents all` 能识别断链和重复安装；
20. `mine dist verify` 通过；
21. 所有配置修改可 dry-run、备份、幂等卸载；
22. README 和用户手册包含实测命令；
23. 不实现未授权 destructive Git 操作；
24. 无业务层 `unsafe`；
25. 所有测试、fmt、clippy 和发布验证通过。

---

## 30. 对实现 Agent 的强制要求

1. 在当前仓库实施，不创建另一个产品仓库；
2. 编码前先更新架构文档和实施 Plan；
3. 必须联网精读官方资料，不只看搜索摘要；
4. 不虚构 Claude/Codex/Pi/OpenCode 命令；
5. 平台命令变化时，以实测官方稳定版为准并更新用户手册；
6. 不把 CLI handler 当业务层；
7. 不为 CLI 和 MCP 写两套逻辑；
8. 不用 SQLite；
9. 不把 Markdown 当执行图事实源；
10. 不保留已经被新设计淘汰的直接图编辑逻辑；
11. 每个 Milestone 完成后必须回看并修改相关 Skills；
12. 最终必须对四个 Agent 做真实可发现性验收；
13. 重要未决策影响公开契约时询问用户；
14. 普通局部实现决策自行做出并写入报告；
15. 不能把 timeout、未运行、缺工具或非零退出说成通过。

---

## 31. 官方资料最低调研清单

实施时必须重新访问最新页面，至少包括：

- Rust Book / Cargo / Rust release notes；
- MCP Specification；
- MCP Rust SDK；
- MCP Inspector；
- Claude Code Plugins、Plugin Marketplace 和 MCP 文档；
- Codex 官方仓库、plugin creator/spec、配置 schema；
- Pi Packages、Skills、Extensions 文档；
- OpenCode Skills、MCP 和 Plugins 文档；
- GitHub Actions 和 Release artifact 官方文档。

本文不固定第三方库版本。架构文档必须记录最终选择、版本和理由。

---

## 32. 最终目标体验

作者在新机器上：

```powershell
irm https://raw.githubusercontent.com/6ixGODD/mine-is-not-everyones/main/scripts/bootstrap.ps1 | iex
mine agent install --agent all
mine doctor --agents all
```

作者进入任意项目，在喜欢的 Agent 中运行对应的 `mine-arch`，随后：

```text
mine-arch
    ↓
mine-plan-create
    ↓
mine-plan-exec
    ↓
mine-plan-review
```

执行图所有状态由 `mine` 确定性维护。

Agent 可以通过 MCP 结构化调用；没有 MCP 的 Pi 环境可以安全调用 JSON CLI。

四个平台共享同一套 Skill 语义，同一仓库发布，同一版本演进。

**MINE Is Not Everyone’s. That is the point.**

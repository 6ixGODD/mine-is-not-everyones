# MINE

> **MINE Is Not Everyone’s.**

MINE 是一套面向 Claude Code、Codex、Pi 和 OpenCode 的个人化、强约束、具有有限破坏性的工程工作流。

它服务于愿意让 Coding Agent 在严格仓库治理下作出明确决策的单一仓库所有者。它不是兼容性框架，不是迁移助手，也不会温柔地保留它遇到的每一种历史文档和旧约定。

**你可以使用它。这并不代表你的仓库配得上它。**

## MINE 的设计哲学

MINE 使用文档驱动开发，但不会把已经过时的文档奉为圣旨。

- `docs/design/` 是 MINE 独占管理的架构状态。
- `mine-sync` 负责把真实仓库同步到这套架构状态。
- 除非用户明确要求保留某项设计，执行同步时以当前代码、Schema、配置和可观测运行行为为准。
- 重写 Design 前，MINE 会在 `docs/design-backup-<timestamp>/` 创建本地忽略的备份。
- Plan、执行图、实施报告和审查报告只存在于 `dev` 与 `plan/*` 临时分支，不进入稳定版本。
- MINE 不保留不受支持的旧 Design 目录结构、废弃内部契约和偶然形成的兼容债务。

MINE 可以重写不准确的设计文档，并在发版时销毁临时开发过程产物。它不代表可以随意删除文件、执行破坏性 Git 恢复，或者向模型暴露任意 Shell。

## MINE 独占 `docs/design/`

MINE 管理的 Design 树必须包含：

```text
docs/design/.mine-design.toml
```

老项目如果已经把其他架构文档放在 `docs/design/`，请在执行 `mine init` 前自行重命名或删除。MINE 故意不兼容任意历史目录结构。

发现没有 MINE 标记的 `docs/design/` 时，初始化必须直接报命名空间冲突，而不是猜测如何迁移。

## 五个 Skill

```text
mine-arch
mine-sync
mine-plan-create
mine-plan-exec
mine-plan-review
```

- `mine-arch`：以需求为中心创建或演进目标架构。
- `mine-sync`：以代码为中心将仓库现实同步到 Design。
- `mine-plan-create`：创建不可变、用完即丢的 Plan。
- `mine-plan-exec`：受治理地实施一份 Plan。
- `mine-plan-review`：独立验收或拒绝实现。

初始化、执行图状态、校验、加锁、安装、诊断和发行等确定性工作全部属于 Rust `mine` 可执行程序。

## 只支持四个平台

- Claude Code；
- Codex；
- Pi；
- OpenCode。

Cursor、Windsurf、Cline 等不在范围内。需要的人自己维护 Fork。

## 快速开始

### 新仓库

```bash
mine init
```

然后打开支持的 Coding Agent，调用：

```text
mine-arch <你的需求>
```

`mine init` 只负责创建 MINE 所需的配置、标记、模板和 Agent 集成。它不扫描仓库、不写架构、不创建 Plan、不启动 Agent，也不实现代码。

### 老仓库

如果原仓库已经存在非 MINE 管理的 `docs/design/`，先重命名或删除。

```bash
mine init
```

然后建立与当前代码一致的 Design 基线：

```text
mine-sync
```

大仓库最好给范围：

```text
mine-sync 只同步认证、授权和身份持久化子系统
```

用户不给范围时，Agent 被允许自由探索整个仓库。由此产生的 Token 和时间成本由提交无范围需求的用户承担。

同步完成后，按以下流程演进系统：

```text
mine-arch <新需求>
mine-plan-create
mine-plan-exec
mine-plan-review
```

## 长期状态与临时状态

稳定分支保留：

```text
代码
测试
配置
README.md
README.zh-CN.md
docs/design/
.mine/config.toml
AGENTS.md
```

稳定分支不得保留：

```text
docs/plan/
执行图
实施报告
审查报告
临时 Design 备份
dev
plan/*
```

详细说明见 [文档索引](docs/README.md) 和 [用户手册](docs/user-guide.md)。

## 语言策略

- `README.md`：英文源文档；
- `README.zh-CN.md`：中文翻译；
- `docs/**`：只写英文。

## 警告

MINE 默认仓库所有者接受强约束，并授权 Agent 按规则重构 Design 文档、创建受管理分支、提交明确范围的修改、将通过审查的 Plan 分支合入 `dev`，以及在发布收口时删除 MINE 临时产物。

MINE 仍然拒绝任意 `reset --hard`、`git clean`、盲目 stash、强制推送、无限制 Shell 删除和仓库外写入。

MINE 之所以固执，是因为模棱两可太贵了。

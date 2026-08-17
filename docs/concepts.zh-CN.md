# MINE 概念

本页以用户可读的方式解释 MINE 的核心思想。它刻意保持简洁；想了解实现或设计内部细节的读者请参考[设计索引](design/index.md)。

## MINE 是什么

MINE 是一套意见明确的、以文档为驱动的编码 Agent 工程工作流。它把架构、规划、实现、审查和发布收口保留在仓库中，由确定性工具约束，状态可版本化、可追溯。一个 Rust 二进制（`mine`）处理确定性部分；五个 Agent Skills（`mine-arch`、`mine-sync`、`mine-plan-create`、`mine-plan-exec`、`mine-plan-review`）处理工作流和工程判断。

## Design 作为持久工程知识

`docs/design/` 是持久的、MINE 所有的设计知识库。它跨发布存在，并存在于 stable 分支上。它描述所在分支的已接受架构。设计变更先于代码变更：先更新目标设计，再规划实现。

## Plan 作为临时执行工作包

Plan 是临时的、可执行的工作包：一份精确的契约，描述要构建什么、为什么以及如何验证。Plan 在开发期间位于 `docs/plan/` 下，并在执行图中以状态跟踪（DRAFT、READY、IN_PROGRESS、IMPLEMENTED、ACCEPTED、REJECTED）。

## 为什么 Plan 在执行后不可变

一旦 Plan 交给实现 Agent 或执行开始，它就不可变。在执行中途更改契约会使实现、审查以及依赖它的下游工作失效。如果 Plan 被证明有误，正确的做法是创建新的补偿 Plan，而不是编辑旧 Plan。

## stable / dev / plan 分支角色

- **stable 分支**（`main` 或 `master`）：仅已发布代码和 `docs/design/`。没有 `docs/plan/`，没有临时过程状态。
- **`dev`**：临时集成分支，拥有活动的 `docs/plan/` 工作区，接收独立接受的 Plan 分支。
- **`plan/<id>-<slug>`**：一个 Plan 的短期实现分支，仅在独立接受后合并到 `dev`。

## 为什么实现与审查分离

实现 Agent 提交有范围的工作并报告证据，但从不自我授予接受。独立审查者先尝试推翻实现的声明，然后才接受。这种分离使接受有意义。

## 为什么 `docs/plan/` 从 stable 发布中消失

规划工作区是过程，不是产品。stable 发布包含已接受的产品状态和持久设计——而不是产生它们的临时 Plan、报告和图。发布收口清除 `docs/plan/`，并通过 squash 或 curated 提交集成 stable 树，以免导入临时历史。

## 为什么 MINE 在发布前同步

发布前，`mine-sync` 将已接受的实现调和进 `docs/design/`，使持久设计与实际构建的代码一致。这个最终同步（阶段 A）是独立的、有意的会话；机械发布收口（阶段 B）紧随其后。

## 常规工程变更 vs 轻量维护

MINE 治理工程变更，而非每次仓库编辑。

- **轻量维护**（错别字、文字润色、翻译、坏链接、README 改进、仅格式变更、描述已接受行为的示例/注释）在结果明确且无持久工程契约变化时，可以直接进行，无需 Plan。
- **常规工程变更**（行为、架构、公共 API、CLI 语义、MCP 契约、Skill 工作流、执行图/发布/分支行为、持久化/schema、安全/隐私边界、部署契约、持久设计决策）使用完整的 Design → Plan → Execute → Review 生命周期。

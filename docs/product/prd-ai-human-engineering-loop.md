# PRD: AI Human 下一阶段工程助手闭环

**Status**: Draft
**Date**: 2026-07-09
**Owner**: AI Human Product
**Primary Users**: Go + Rust 服务端开发者、Reviewer

## 1. 背景与定位

AI Human 是一个 Rust + rig 驱动的服务端研发 AI 工程助手。当前 CLI 已具备 `init`、`doctor`、`ask`、`plan`、`impact`、`review`、`learn`、`fix dry-run`、`fix --apply` 等能力。

下一阶段的核心不是把 AI Human 做成更自由的通用 coding agent，而是做成更可靠的团队工程助手。

定位：

> AI Human 面向 Go + Rust 服务端团队，通过优雅 CLI、项目知识、安全边界、报告沉淀和受控执行，把代码理解、影响分析、Review、学习和受控修复串成可复用的工程闭环。

### 1.1 与 Codex / 通用 Coding Agent 的差异

| 维度 | Codex / 通用 Coding Agent | AI Human |
|---|---|---|
| 核心定位 | 单人编程效率工具 | 团队工程流程助手 |
| 主要价值 | 写代码、改代码、解释代码 | 规范化分析、评审、影响面判断、修复建议、报告沉淀 |
| 使用边界 | 更开放、更通用 | 更受控、更贴合团队约束 |
| 知识来源 | 当前上下文、文件、用户指令 | 项目结构、团队规则、历史报告、工程约定 |
| 执行方式 | 偏 agent 自主完成任务 | CLI 命令封装，先解释、再计划、再 dry-run、再受控 apply |
| 结果形态 | 代码变更或回答 | 可审计的工程结论、报告、计划、修复记录 |

核心 trade-off：

- 不追求通用 agent 的最大自由度，换取团队可控、可审计、低门槛。
- 不优先做 Web、IM、RAG 大平台，先把 CLI 工程闭环做稳。
- 不承诺自动解决所有问题，而是把复杂工程判断拆成清晰、可执行、可复核的步骤。

### 1.2 产品设计原则

AI Human 的设计必须优先服务真实使用者，而不是展示模型能力。

- 人优先：用户必须始终知道当前命令做了什么、没做什么、下一步是什么。
- 默认安全：读操作和 dry-run 优先，写操作必须显式触发。
- 可解释：重要结论必须给出依据、来源或不确定性说明。
- 可恢复：错误提示必须包含可执行的修复建议。
- 可审计：关键动作必须留下报告、输入、输出、验证结果和风险记录。
- 可扩展：命令、工作流、模型调用、报告、记忆、安全策略应保持高内聚低耦合。
- 少即是多：v1 不追求能力数量，优先把高频工程闭环做得稳定、可信、顺手。

## 2. 问题陈述

### 2.1 一线开发者问题

- 不确定需求或 bug 修复会影响哪些模块。
- 不了解跨模块调用链、状态一致性、持久化风险。
- 希望 AI 帮忙修复，但不希望 AI 不透明地修改代码。
- 希望输出结果能直接指导下一步，而不是泛泛建议。

### 2.2 Reviewer 问题

- Review 前缺少结构化影响面分析。
- Review 中重复发现同类问题，但团队规则没有沉淀。
- 很难快速判断修改是否覆盖了业务、协议、状态、持久化、测试风险。

### 2.3 技术负责人问题

- 团队工程规则分散在经验、聊天记录和口头约定中。
- AI 工具如果没有边界，难以进入真实工程流程。
- 需要把 AI 产出变成可追踪、可审计、可复盘的工程资产。

### 2.4 当前不用 AI Human 的成本

- 新人和跨模块开发理解成本高。
- Review 往返次数高。
- 影响面遗漏导致回归风险。
- AI 对话结果一次性消费，无法成为团队知识。

## 3. 目标与成功指标

### 3.1 产品目标

- 降低 Go + Rust 服务端团队理解项目、评估影响、Review、受控修复的成本。
- 建立安全、可审计、可复用的 AI 工程闭环。
- 让输出对人友好：清楚、可操作、可恢复、可验证。

### 3.2 激活指标

| 指标 | 目标 |
|---|---|
| 首次 `init` 成功率 | >= 90% |
| `doctor` 后用户能完成下一步操作比例 | >= 80% |
| 新用户首次完成有效命令时间 | <= 10 分钟 |
| CLI 命令失败后可恢复比例 | >= 80% |

### 3.3 使用指标

| 指标 | 目标 |
|---|---|
| 周活跃开发者占目标团队比例 | >= 50% |
| `impact / review / fix dry-run` 周使用次数 | 持续增长 |
| dry-run 后进入 apply 的比例 | 20% 到 50% |
| 单次任务平均交互轮次 | 下降，但不牺牲安全确认 |

### 3.3.1 本地指标事件 v1

指标先采用本地 JSONL，不做远端上报，不采集源码内容和用户 prompt 原文。

存储位置：

- `.ai-human/memory/metrics.jsonl`

每条命令事件包含：

- `recorded_at`: 记录时间。
- `command`: 命令名，例如 `plan`、`impact`、`review`、`fix`。
- `status`: `success` 或 `failed`。
- `duration_ms`: 命令耗时。
- `task_id`: 有任务 ID 时记录。
- `report_path`: 命令生成报告时记录。
- `error`: 失败时记录短错误摘要。

### 3.4 工程结果指标

| 指标 | 目标 |
|---|---|
| Review 前发现的问题数 | 上升 |
| Review 往返次数 | 下降 20%+ |
| 变更影响面遗漏率 | 下降 |
| 用户认为输出可直接行动的比例 | >= 70% |
| 团队规则被正确引用比例 | >= 80% |
| AI 直接造成的 P0/P1 错误变更 | 0 |

## 4. 用户角色与优先级

### 4.1 P0: 后端开发

最高频用户。直接使用 `doctor`、`ask`、`plan`、`impact`、`fix`。后端开发的判断标准是工具是否能节省实际工程时间，而不是能力是否完整。

### 4.2 P0: Reviewer

核心价值用户。重点使用 `impact`、`review`、`fix dry-run`。Reviewer 决定 AI Human 能否形成团队质量标准。

### 4.3 P1: 技术负责人

规则制定者和推广者。关注团队规则、安全边界、默认验证命令、知识沉淀方式。

### 4.4 P2: 团队管理者

关注效率、质量、风险和产出报告。当前阶段不应优先做管理看板，避免牺牲一线开发体验。

## 5. 核心场景与用户故事

### 5.1 项目诊断

用户故事：

> 作为后端开发者，我希望运行一个不调用模型的诊断命令，确认项目是否初始化、模型配置是否可用、下一步该做什么。

验收标准：

- `doctor` 不调用模型。
- 输出项目初始化状态。
- 输出模型配置状态。
- 当配置缺失时给出下一步命令。

### 5.2 项目问答

用户故事：

> 作为开发者，我希望用自然语言询问项目规则、模块归属、常见工程约定，从而减少翻代码和问人的成本。

验收标准：

- `ask` 使用本地项目知识和上下文。
- 如果上下文不足，明确说明缺失信息。

### 5.3 计划生成

用户故事：

> 作为技术负责人或开发者，我希望在动手前生成计划，明确目标、非目标、影响范围、风险和验证方式。

验收标准：

- `plan` 输出目标、非目标、影响区域、风险、验证计划、开放问题。
- 报告写入 `.ai-human/reports/`。

### 5.4 影响面分析

用户故事：

> 作为开发者或 Reviewer，我希望在修改前知道可能影响哪些文件、调用链、状态、协议、持久化和测试入口。

验收标准：

- `impact` 输出文件、调用链、风险类型和测试入口。
- 输出结果可直接用于 Review 和测试计划。

### 5.5 代码 Review

用户故事：

> 作为 Reviewer，我希望 AI Human 按服务端工程风险维度审查 diff 或文件，提前发现业务正确性、状态一致性、持久化、协议兼容和测试缺口。

验收标准：

- `review` 支持 diff 文件和单文件输入。
- finding 必须包含严重级别、位置、问题和建议。
- review 发现追加到 memory。

### 5.6 dry-run 修复

用户故事：

> 作为开发者，我希望 AI 先给出修复计划，而不是直接改代码。

验收标准：

- `fix` 默认 dry-run。
- 输出明确说明源码未修改。
- 报告包含目标文件、意图、风险、验证命令和 proposed replacement。

### 5.7 apply 执行

用户故事：

> 作为开发者，我希望在确认 dry-run 后显式执行受控修复，并看到改了什么、验证结果如何、还剩什么风险。

验收标准：

- `fix --apply` 只写 `--path` 指定文件。
- 只有模型 replacement path 与 `--path` 完全匹配才允许写入。
- 验证和格式化命令只来自 CLI/config，不来自模型自由输出。
- 输出 apply report，列出 written files 和 verification results。

### 5.8 报告沉淀

用户故事：

> 作为团队成员，我希望每次关键分析、Review、修复都有可追踪报告，后续可以复盘、学习和引用。

验收标准：

- 报告默认写入 `.ai-human/reports/`。
- 输出中明确报告路径。
- 学习结果写入 Markdown 知识和 JSONL memory。

## 6. 下一阶段产品主线

### 6.1 人性化交互

目标：用户不需要理解 agent 内部机制，也能知道下一步做什么。

要求：

- 命令语义清晰。
- 输出结构稳定。
- 默认行为保守。
- 错误信息可恢复。
- 每次执行都说明：发生了什么、是否写文件、产物在哪里、下一步是什么。

### 6.2 工程闭环

推荐闭环：

```text
doctor -> ask / learn -> plan -> impact -> review -> fix dry-run -> fix --apply -> report / learn
```

命令不是孤立能力，而应组成连续工程流程。

### 6.3 团队知识

当前阶段采用轻量知识机制：

- `.ai-human/knowledge/` Markdown 文档。
- `.ai-human/memory/*.jsonl` 结构化记录。
- `.ai-human/config.toml` 项目配置。
- `.ai-human/reports/` 历史报告。

不在当前阶段建设完整 RAG。

### 6.4 安全执行底座

要求：

- dry-run 默认优先。
- apply 显式触发。
- 高风险操作需要拦截或二次确认。
- 命令执行来源可控。
- 报告必须保留审计信息。

### 6.5 领先特性：Evidence-Based Engineering Loop

AI Human 要形成区别于通用 Codex 的核心特性：基于证据的工程闭环。

定义：

> 每一次工程任务都有可追踪的 task id，每一个关键结论都有证据来源，每一次风险判断都有明确等级，每一次写入都有验证和审计记录。

核心能力：

- Task ID：把 `plan`、`impact`、`review`、`fix`、`learn` 串成同一次工程任务。
- Evidence：报告中明确引用文件、规则、历史经验、命令输出或模型不确定性。
- Rule Hit：标识命中了哪些团队规则、工程约定或历史经验。
- Risk Gate：对高风险修改阻断 apply，要求人工升级确认或拆分任务。
- Engineering Memory：把一次任务的结论沉淀为团队可复用的知识，而不是一次性对话。

领先性不来自“能不能自动改代码”，而来自“能不能把 AI 工程判断变成团队可复核、可治理、可沉淀的资产”。

## 7. 功能范围

### 7.1 v1 必做

- CLI 输出结构统一。
- Task ID 贯穿主要命令和报告。
- 报告包含 Evidence、Risk、Next Action。
- `doctor` 诊断标准化。
- `plan` 输出可执行计划。
- `impact` 输出影响范围、风险点、建议测试范围。
- `review` 输出问题分级、依据、建议修复方向。
- `fix dry-run` 输出变更计划和预期 replacement。
- `fix --apply` 受控执行并保留审计信息。
- `learn` 沉淀规则、FAQ、案例、经验。
- 项目配置机制支持团队规则和安全边界。

### 7.2 v1 明确不做

- 不做 Web 控制台作为主入口。
- 不做 IM Bot 作为主入口。
- 不做完整 RAG 平台。
- 不做多租户 SaaS。
- 不做通用聊天机器人。
- 不做无限制自动改代码。
- 不做跨仓库自动重构。
- 不做自动提交、自动推送、自动合并。
- 不做绕过团队 Review 的自动发布链路。
- 不做覆盖所有语言和框架。
- 不承诺替代人工 Review。
- 不优先做管理驾驶舱。

### 7.3 Later

- Web 查看报告。
- IM 通知或轻量交互。
- 团队知识库 / RAG。
- CI 集成。
- PR Review Bot。
- 团队级指标看板。

## 8. CLI 交互原则

### 8.1 命令设计原则

- 高频命令必须短、清晰、可复制。
- 默认值必须安全。
- 写操作必须显式。
- 同一类参数命名保持一致，如 `--project-root`、`--input`、`--path`。

### 8.2 输出结构原则

所有命令输出应逐步统一为：

```markdown
# <Command Result>

## Summary
...

## Task
...

## What Happened
...

## Evidence
...

## Files
...

## Risks
...

## Next
...
```

要求：

- Summary 必须用短句说明结论。
- Task 必须包含 task id；没有传入时自动生成。
- Evidence 必须说明依据来自文件、规则、历史报告、命令输出还是模型推断。
- Risks 必须显式标注 Low / Medium / High。
- Next 必须给出 1 到 3 个可执行动作，不能只写泛泛建议。

### 8.3 错误提示原则

错误信息必须包含：

- 发生了什么。
- 为什么失败。
- 用户下一步可以执行什么。

示例：

```text
path must stay inside project root: ../secret.go
Next: pass a path under --project-root, for example --path service/pay/audit.go
```

### 8.4 风险提示原则

- dry-run 必须明确源码未修改。
- apply 必须明确写入文件。
- verification failed 不得宣称成功。
- 高风险文件和命令必须拦截或升级确认。

## 9. 安全与权限边界

### 9.1 dry-run 策略

- `fix` 默认 dry-run。
- dry-run 不写源码。
- dry-run 只写报告。

### 9.2 apply 策略

- `fix --apply` 必须显式传入。
- 只允许写 `--path` 指定文件。
- replacement path 必须与 `--path` 匹配。
- 命令执行只来自 CLI/config。

### 9.3 风险等级

建议引入风险等级：

- Low: 小范围单文件、无协议/状态/持久化影响。
- Medium: 涉及状态、持久化、跨模块调用。
- High: 涉及协议、生产配置、数据迁移、并发一致性。

v1 默认对 High 风险阻断 apply。

### 9.4 审计与报告

每次 apply report 必须包含：

- 输入目标。
- 写入文件。
- 验证命令。
- 验证结果。
- residual risks。
- 报告路径。

## 10. 报告与知识沉淀

### 10.1 报告类型

- Plan report
- Impact report
- Review report
- Learn report
- Fix dry-run report
- Fix apply report

### 10.2 保存位置

- 报告：`.ai-human/reports/`
- 知识：`.ai-human/knowledge/`
- 结构化记忆：`.ai-human/memory/*.jsonl`

### 10.3 复用方式

- `ask / plan / impact / review / fix` 应逐步复用知识和历史报告。
- `learn` 负责把一次性结论转成长期知识。

### 10.4 与 later RAG 的关系

当前阶段先保证知识内容结构清晰、来源明确、可维护。RAG 只作为 later，不作为下一阶段前置依赖。

## 11. 风险与 Trade-off

### 11.1 通用能力 vs 团队约束

选择团队约束。牺牲部分自由度，换取安全、审计和可推广性。

### 11.2 自动执行 vs 安全可控

选择安全可控。默认 dry-run，apply 显式，命令来源受控。

### 11.3 快速上线 vs 知识体系完整度

选择快速形成可用闭环。知识体系先轻量，后续根据使用反馈扩展。

### 11.4 CLI 优先 vs 多端入口

选择 CLI 优先。服务端工程师当前最直接的工作入口是本地 CLI。Web、IM、PR Bot 都是 later。

## 12. 三个月失败预演

如果 AI Human 三个月后失败，最可能不是因为模型能力不够，而是产品闭环没有站在使用者真实工作流里。

### 12.1 失败原因

| 风险 | 具体表现 | 产品应对 |
|---|---|---|
| 与 Codex 差异不明显 | 用户认为直接用 Codex 更快 | 强化证据链、任务闭环、团队规则、审计报告 |
| 使用门槛高 | 用户不知道先跑什么、参数怎么填、失败后怎么恢复 | 强化 `doctor`、错误提示、默认值、Next Action |
| 输出不可验证 | 报告看起来合理，但没有来源和依据 | 所有关键报告引入 Evidence |
| 知识库污染 | `learn` 沉淀大量低质量、重复、过期内容 | 引入规则类型、来源、更新时间和可废弃机制 |
| 没有高频场景 | 功能很多，但不在日常开发中反复使用 | 优先围绕 `impact -> review -> fix` 闭环打磨 |
| 安全边界不足 | 一次错误 apply 造成信任坍塌 | 默认 dry-run，高风险阻断，写入范围硬约束 |
| 没有度量 | 说不清是否真的提效 | 记录命令使用、报告产出、修复闭环和失败恢复指标 |

### 12.2 失败信号

- 新用户第一次成功使用超过 10 分钟。
- 用户频繁跳过 `impact / review`，只把工具当普通问答。
- 报告中缺少文件、规则、证据和下一步动作。
- dry-run 后用户仍然不敢 apply。
- 团队规则无法被稳定引用。
- Review 质量没有明显改善。
- 负责人无法从报告中判断工具是否创造价值。

### 12.3 反制策略

- 先优化高频路径，不扩张低频能力。
- 每个命令都必须能单独解释价值，也必须能进入工程闭环。
- 每个报告都必须回答：结论是什么、依据是什么、风险是什么、下一步做什么。
- 先用轻量本地知识建立可信边界，再考虑 RAG 和多端入口。
- 把“好用”作为验收项，而不是上线后的附加优化。

## 13. 开放问题与决策记录

| 决策项 | 默认决策 | 状态 |
|---|---|---|
| v1 主用户 | 后端开发 + Reviewer | 已采用 |
| 技术负责人角色 | 规则制定者和推广者 | 已采用 |
| 核心闭环 | `impact / review / fix dry-run / fix --apply` 优先，同时保留 `ask / learn` | 已采用 |
| `fix --apply` 安全策略 | v1 保守，后续按风险等级二次确认 | 待细化 |
| 团队知识形态 | 配置文件 + Markdown + JSONL | 已采用 |
| 报告默认对象 | 开发者 + Reviewer | 已采用 |
| CI / PR 集成 | 不进入当前主线，只保证报告可复用 | 已采用 |
| 项目知识可信边界 | 仓库文件 + 团队规则文件 | 已采用 |
| 领先特性 | Evidence-Based Engineering Loop | 已采用 |
| 三个月失败反制 | 优先证据链、可操作输出、任务闭环和风险门禁 | 已采用 |

## 14. 第二阶段优先级

### 14.1 P0 优先级

1. 统一所有命令输出结构。
2. 引入 task id，贯穿报告和主要工作流命令。
3. 在报告中加入 Evidence、Risk、Next Action。
4. 为 `fix --apply` 增加风险等级和 High 风险阻断。
5. 增强 `.ai-human/config.toml`，支持默认 verify/format 命令和安全策略。

### 14.2 P1 优先级

1. 强化 `impact` 与 `review` 的可行动输出。
2. 为 `learn` 增加知识类型、来源和更新时间。
3. 增加用户友好的命令示例和错误恢复建议。
4. 建立最小可用的使用指标记录。

### 14.3 P2 / Later

1. Web 报告查看。
2. CI / PR Review Bot。
3. 团队知识库 / RAG。
4. 管理指标看板。

## 15. 第二阶段验收标准

第二阶段完成时，应满足以下条件：

- 新用户只通过 `init`、`doctor` 和输出中的 Next Action，可以完成第一次有效 `impact` 或 `review`。
- `plan`、`impact`、`review`、`fix` 报告都包含 task id。
- 主要报告都有 Summary、Evidence、Risk、Next Action。
- `fix` 默认 dry-run，`fix --apply` 遇到 High 风险默认阻断。
- 用户能从报告中看出结论依据来自代码、规则、历史报告还是模型推断。
- 每一次写操作都能在报告中追溯输入、写入文件、验证命令和验证结果。
- 所有命令失败时，错误信息都包含可执行的下一步建议。

### 15.1 当前实现状态（2026-07-11）

已完成：

- `doctor` 输出项目状态、模型配置检查、建议工作流，并在命令末尾输出结果摘要。
- `plan`、`impact`、`review`、`learn`、`fix` 报告已包含任务上下文、依据、风险、下一阶段动作。
- 成功命令统一输出 `## Result Summary`，报告型命令输出报告路径和 `Next stage:`。
- `fix` 默认 dry-run，`fix --apply` 遇到 High 风险默认阻断。
- 报告型命令已写入 `.ai-human/memory/tasks.jsonl`，`task --id` 可查看同一任务的报告链路，并输出可复制的 `Suggested command:`。
- 命令已写入 `.ai-human/memory/metrics.jsonl`，记录本地成功率、失败摘要、耗时和报告产出。
- `impact --from-task`、`review --from-task`、`fix --from-task`、`learn --from-task` 已支持从任务上下文继承 task id、input、path 或 source report。
- 模型/provider 配置错误已覆盖 `OPENAI_API_KEY` 缺失、不支持的 `AI_HUMAN_MODEL_PROVIDER`、非法 `AI_HUMAN_OPENAI_WIRE_API` 和 endpoint 状态错误的 `Next:` 恢复提示。
- 主要报告已输出结构化 Evidence 类型标签，覆盖 Project Context、File、History Report、Command、Model Inference、User Input，并新增 `## Rule Hits` 展示命中的团队规则来源。
- Evidence/Rule Hit 已写入 `.ai-human/memory/tasks.jsonl`，`task --id` 已展示跨报告 `Evidence Trail` 和 `Rule Hits`。
- `evidence --task-id` 已支持直接查询 Evidence/Rule Hit，并支持 `--kind`、`--source` 过滤。

部分完成：

- 高频可恢复错误已输出 `Next:` 建议，仍需继续细分 provider 网络、认证、限流等运行时错误。
- 写操作已记录报告路径和验证结果，任务索引已有跨报告证据链展示和基础查询能力，仍需扩展 Evidence/Rule Hit 的摘要、聚合和管理视图。

待推进：

- 细化 provider 网络、认证、限流等运行时错误分类和恢复建议。
- 为 Evidence/Rule Hit 增加摘要、聚合和管理视图。

## 16. 下一步建议

1. 统一所有命令输出结构。
2. 建立任务 ID，把 `plan`、`impact`、`review`、`fix`、`learn` 串成同一次工程任务。
3. 在主要报告中增加 Evidence、Risk、Next Action。
4. 为 `fix --apply` 增加风险等级和 High 风险阻断。
5. 增强 `.ai-human/config.toml`，支持默认 verify/format 命令和安全策略。

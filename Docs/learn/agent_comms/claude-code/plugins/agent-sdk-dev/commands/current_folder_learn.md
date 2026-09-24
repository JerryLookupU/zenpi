# `claude-code/plugins/agent-sdk-dev/commands` 目录汇总

## 覆盖范围与目录职责

本目录的已完成 1:1 笔记对应一个命令文件：`new-sdk-app.md`。该目录不是 SDK 的运行时实现，而是 Claude Code 中“创建 Agent SDK 项目”的交互式编排入口：收集项目约束，读取官方资料，生成最小可运行工程，安装并核对最新版 SDK，执行本地检查，再交给语言专用 verifier。它的核心职责是把“需求采集 → 计划确认 → 有副作用的初始化 → 验证 → 使用说明”串成有明确门槛的顺序流程。

## 模块清单

### `new-sdk-app.md`

文件前置区的关键公开入口是 `description` 与 `argument-hint: [project-name]`；命令入口为 `/new-sdk-app`，没有 Rust 函数或类型导出。它逐次询问 `TypeScript`/`Python`、项目名、`coding`/`business`/`custom` agent 类型、`minimal`/`basic`/具体示例起点和 `npm`/`yarn`/`pnpm`/`pip`/`poetry` 工具偏好；随后创建配置和入口文件、安装 `@anthropic-ai/claude-agent-sdk` 或 `claude-agent-sdk`、生成 `.env.example`/`.gitignore`，最后调用 `agent-sdk-verifier-ts` 或 `agent-sdk-verifier-py`。TypeScript 的关键验证命令是 `npx tsc --noEmit`，Python 至少须通过 import 与语法检查。

## 运行时数据流与控制流

1. 输入是可选的 `$ARGUMENTS` 项目名和用户逐轮回答。即使带有项目名，也不能跳过语言问题；只有已有充分用例描述时才可跳过 agent 类型问题。问题必须一次一个并等待回答。
2. 需求收集完后生成 `Setup Plan`，覆盖目录初始化、语言配置、版本查询、依赖安装、起始文件、环境变量和可选 `.claude/` 内容；未得到用户确认前不得安装、写文件或创建 subagent/command。
3. 计划确认后，重新检查最新版本，检查目标目录是否已存在，按用户选择的包管理器执行初始化和安装，生成入口代码，并把所用版本写进结果。TypeScript 需要 `package.json` 的 `type: "module"` 和 `typecheck` script；Python 使用 `requirements.txt` 或 `pyproject.toml`。
4. 本地验证通过后，调用对应 verifier。verifier 的结论包含 `PASS`、`PASS WITH WARNINGS` 或 `FAIL`，并输出 critical issues、warnings、通过项和文档化建议；该报告是独立于编译/语法检查的第二层行为判据。
5. 全部成功后输出运行命令（如 `npm start` 或 `python main.py`）、API key 设置方式和 system prompt、permissions、tools、MCP servers、subagents 等后续指引。数据流中的主要持久化产物是工程文件、依赖清单、`.env.example` 与 `.gitignore`，不是命令自身定义的消息队列。

## 错误、取消与恢复语义

源文件明确要求版本查询失败、目录冲突、依赖安装失败、TypeScript 类型错误、Python import/语法错误或 verifier 发现问题时停在对应阶段修复；`npx tsc --noEmit` 必须清零类型错误，不能以“示例已生成”代替成功。它没有定义取消 token、超时、重试次数、回滚、并发锁或恢复游标。用户未确认计划时相当于安全停留在无副作用状态；中断后只能依据已存在的 `package.json`、`tsconfig.json`、`requirements.txt`、源码和安装记录重新审计。源命令还要求不提交真实 `.env` 或 API key，并在创建前检查文件/目录存在性。因而 Rust 映射不应虚构“安装成功即关闭”的语义，而应保留可重放的阶段事件和失败状态。

## 与 zenpi Rust 的映射建议

### 可复用通信原语

- 持久消息/邮箱优先复用 `src/session.rs` 的 `MailboxMessage`、`MailboxStatus`、`MailboxAction` 和 `SessionMailbox::{enqueue, find_message, list, update}`。现有 digest、`sequence`、TTL、`claim_token`、`Complete`/`Fail` 状态适合承载节点请求、进度、绿色回执、失败回执和派生意图；相同 `request_id`/digest 重放必须幂等。
- 保活和接收者资格复用 `LiveSessionRegistry::{register, heartbeat, claim_next, claim_message, finish_claim}`。它提供 owner epoch、workspace 边界和 liveness admission，但本身不是 DAG 调度器，不能仅凭 session 文件存在推断 worker 仍活着。
- 进程内实时通知复用 `src/core.rs` 的 `AgentEvent`，运行时生命周期复用 `src/runtime.rs` 的 `RuntimeEvent::{Accepted, Started, Queued, CancelRequested, Completed, Closed}`；跨进程输出沿用 `src/headless.rs` 的版本化 JSONL、request-id 相关性、有限 replay 和 terminal replay。持久 mailbox/session journal 是恢复真相，事件流只做实时唤醒和展示。
- 协议层应在现有 `MailboxRequest`/`StdioRequest` 上增加受限 DAG envelope，而不是另造无界通道。至少携带 `dag_id`、`node_id`、`parent_id`、`grandparent_id`、`edge_kind`、`generation`、`lease_id`、`request_id` 和状态摘要，并继续使用现有 ID、payload、TTL、事件/重放大小限制。

### 会话父子关系与四向通信

建议在 `src/session.rs` 的 durable record 中引入受校验的 `DagNodeState`/`DagNodeContext`：`dag_id`、`node_id`、`session_id`、`parent_id`、`grandparent_id`、直接 `sibling_ids`、直接 `child_ids`、`generation`、`owner_epoch`/`lease_id`、状态和 closure proof。它与 `src/core.rs` 的 `Turn.parent_id` 分工不同：后者描述会话内 turn 链，前者描述跨 worker 的 DAG 拓扑；所有边都应写入 journal，恢复时拒绝缺失父节点、重复边和跨 workspace 节点。

一个 worker 负责节点时，必须通过同一套 addressed mailbox 与四类邻居通信：向 `parent_id` 发送工作结果/阻塞原因；向 `grandparent_id` 发送聚合进度或升级信息；向每个直接 `sibling_ids` 发送协作事件/依赖通知；向每个直接 `child_ids` 发送任务、取消或验证请求。收件人必须由持久 DAG 索引解析为明确 `session_id`，不能用模糊广播或仅凭路径猜测。发送采用 `enqueue`，接收采用 `claim_message`，完成采用 `finish_claim`/`Complete`，回复带原消息 digest 和 generation，防止旧 worker 的晚到结果覆盖新代状态。

### 保活、派生和 close 门槛的最小机制

最小路径是：父节点将任务写入子节点 mailbox；子节点注册 `LiveSessionOwner`，周期性 `heartbeat`，claim 后运行；运行中发布 `AgentEvent`/mailbox `Progress`；结束时写 `Green`、`Blocked` 或 `Failed` 回执；父节点在 journal 锁内更新聚合状态。`src/runtime.rs` 的 `BackgroundRunner::try_submit` 负责有界派生和 FIFO 排队，`JobId` 绑定 `DagNodeState`，`CancellationToken` 负责协作式取消；`SubmitError::QueueFull`/`Closed` 必须保留为可观测失败，不能伪装成节点已关闭。

close 必须是原子、可重放的聚合判定，而不是某一个 provider turn 或某一个 worker 的成功返回：只有“节点自身为 Green，并且该节点的全部直接 child 以及递归的全部 grandchild/后代均为 Green，所有 closure proof 的 `generation` 一致，且无未完成 mailbox、活动 lease、未决 approval/tool batch”同时成立，才追加 `NodeClosed` 并停止该节点保活。任一后代为 `Blocked`、`Failed`、`Expired`、未知、仍有 claim，或出现新 generation/新工作，节点必须保持 `Alive`，继续 heartbeat，并由父调度器通过 `BackgroundRunner::try_submit` 派生新的 worker 处理新工作。派生意图要先写 journal 再提交 runtime；新 worker 使用新的 `generation`/`lease_id`，旧 epoch 的 late completion 只能被拒绝或记录，不能改变当前状态。

### 既有文件的落点

- `src/headless.rs`：增加 DAG 的 `send`、`claim`、`complete`、`heartbeat`、`derive`、`status` 命令和 `StdioEvent` 输出；复用请求指纹、terminal replay、reconnect journal。EOF 只代表输入端结束，已 admitted 的 DAG 工作仍须 drain；显式 shutdown 才请求取消。
- `src/core.rs`：在 `Agent` 上下文中装配节点身份、邻接关系、worker lease、heartbeat 和 `AgentEvent`；将 provider/tool 结果转换为 DAG 状态，并集中调用纯的 `evaluate_close(snapshot)`。provider 单次成功不得直接调用 `close`。
- `src/session.rs`：持久化 DAG 边、generation、heartbeat、派生意图、child terminal receipt 和 `NodeClosed`；复用 append-only journal、`SessionMailbox` 的锁/digest/claim/TTL 约束以及 `LiveSessionRegistry` 的 owner epoch，重启后从事件重建未闭合节点。
- `src/runtime.rs`：只负责有界 command/event channel、worker 生命周期、`CancellationToken`、队列满和 bounded shutdown；DAG 策略由 core/session 驱动。取消是 cooperative，不能回滚已经发生的 provider/tool side effect；`mark_completed` 仍用于处理完成与晚到取消竞态。

## 未决问题

源命令没有规定 SDK 版本下限、网络/官方文档不可用时的 fallback、verifier 的机器可读 schema、安装失败回滚和恢复协议；这些不能从 `new-sdk-app.md` 推导。zenpi 还需明确 DAG 最大深度/节点数、heartbeat TTL 与派生预算、grandchild 是否指全部递归后代、sibling 是否允许双向发送、closure proof 的摘要算法、`Green` 是否必须包含 provider、tool 和 approval 全部成功，以及新工作在 close 竞态中如何原子地提升 `generation`。实现前还应决定 DAG 拓扑是扩展现有 session JSONL 还是独立索引，但无论选择哪种，必须保持 mailbox 幂等、旧 epoch 隔离、后代未全绿时保活不 close 的不变量。

# `claude-code/plugins/agent-sdk-dev` 目录级学习汇总

> 学习模式：`understand`（目录汇总）。
>
> 源目录：`/Users/wangweiyang/GitHub/claude-code/plugins/agent-sdk-dev`。
> 已完成的逐文件笔记位于
> `Docs/learn/agent_comms/files/claude-code/plugins/agent-sdk-dev/`；本汇总
> 覆盖 README、两个 verifier agent、`new-sdk-app.md`，并直接核对了
> `.claude-plugin/plugin.json`。

## 目录职责

这是 Claude Code 的 Agent SDK 开发插件，职责是把 Python/TypeScript Agent
SDK 应用从需求收集、官方资料与最新版本核对、项目脚手架、依赖安装、本地
编译/语法检查，一直推进到语言专用 verifier 和启动说明。它是“流程编排与
审查规则”目录，不是 SDK provider、模型协议实现或持久化运行时。核心约束
是交互问题一次一个且等待回答，用户确认 `Setup Plan` 以前不应创建文件或
安装依赖；确认后仍需通过本地检查和 verifier 才能报告完成。Python 与
TypeScript verifier 都只审查项目并输出报告，不负责自动修改被审查项目。

## 模块清单

- `.claude-plugin/plugin.json`：插件注册元数据；关键字段/导出是
  `name: agent-sdk-dev`、`description`、`version: 1.0.0` 和 `author`，不含
  可执行函数。
- `README.md`：插件使用说明与生命周期总览；公开入口是 `/new-sdk-app`、
  `agent-sdk-verifier-py`、`agent-sdk-verifier-ts`，规定 latest SDK、API key
  不入库、部署前验证及三态报告 `PASS`/`PASS WITH WARNINGS`/`FAIL`。
- `commands/new-sdk-app.md`：创建新 Agent SDK 项目的交互式命令；关键前置
  配置为 `description` 与 `argument-hint: [project-name]`，行为导出是按序
  收集语言、项目名、agent 类型、起点和包管理器，随后生成项目、安装依赖、
  本地验证并调用对应 verifier。
- `agents/agent-sdk-verifier-py.md`：Python 审查 worker；关键 front matter
  为 `name: agent-sdk-verifier-py`、`model: sonnet`，检查依赖/版本、
  `claude_agent_sdk` 用法、错误处理、`.env` 安全、MCP/subagents/session、
  文档，并输出 `Overall Status`、`Critical Issues`、`Warnings`、
  `Passed Checks`、`Recommendations`。
- `agents/agent-sdk-verifier-ts.md`：TypeScript 审查 worker；关键 front matter
  为 `name: agent-sdk-verifier-ts`、`model: sonnet`，额外强制检查
  `package.json` 的 ES module 配置、`tsconfig.json`、脚本以及
  `npx tsc --noEmit`，输出与 Python verifier 对齐的五部分报告。

## 运行时数据流与控制流

1. 输入是可选的 `$ARGUMENTS` 项目名和用户逐轮回答。即使命令带项目名，仍
   必须先询问 TypeScript/Python；项目名可跳过，已有充分用例描述时可跳过
   agent 类型，但不能跳过起点和工具链确认。
2. 命令先用 `WebFetch`/`WebSearch` 读取官方 overview、语言参考及所需的
   Streaming/Single、Permissions、Custom Tools、MCP、Subagents、Sessions
   指南，并核对 npm/PyPI 的 latest 版本。随后生成 `Setup Plan`，内容包括
   目录初始化、`package.json`/`tsconfig.json` 或 `requirements.txt`/
   `pyproject.toml`、起始文件、`.env.example`、`.gitignore` 和可选 `.claude/`
   配置。
3. 用户确认计划后，按语言执行初始化、安装、版本核验和示例生成。TypeScript
   需要 `type: "module"`、`typecheck` script、正确 SDK import，并循环修复
   直到 `npx tsc --noEmit` 通过；Python 至少通过 import 与语法检查。真实
   `ANTHROPIC_API_KEY` 不应写入源码或提交历史。
4. 本地检查通过后调用 `agent-sdk-verifier-ts` 或 `agent-sdk-verifier-py`。
   verifier 依次读取工程配置、对照官方文档、执行编译/导入检查、核对 SDK
   API 与示例模式，再形成三态报告。最后输出 `npm start`、`python main.py`
   等启动命令以及 system prompt、permissions、tools、MCP servers、subagents
   的后续使用说明。

因此该目录的控制流是严格串行的“询问 → 计划 → 确认 → 有副作用的 setup →
本地验证 → 独立 verifier → 使用指南”；主要数据产物是项目文件、依赖清单、
版本信息和验证报告，源文件没有定义并发 worker、共享锁或报告持久化协议。

## 错误、取消与恢复语义

源文件没有取消 token、deadline、重试次数、恢复游标、回滚或机器可读错误
码。用户未回答或未确认计划时，流程必须停留在无副作用阶段；目录冲突、latest
查询失败、依赖安装失败、TypeScript 编译失败、Python import/语法失败或
verifier 的 critical issue 都应停在对应阶段处理，不能用“文件已生成”代替
成功。`PASS WITH WARNINGS` 可以继续使用，`FAIL` 表示存在阻断运行、安全或
SDK 使用问题。网络文档不可访问、检查证据不足和中途中断的处理方式未规定，
宿主需要保留诊断；中断后只能从已存在的 `package.json`、`tsconfig.json`、
依赖文件、源码、日志和报告重新审计。verifier 本身只读工程及官方文档，不
应替被测项目执行 provider/tool 副作用。

映射到 zenpi 时，不能把 `Cancelled`、`QueueFull`、owner 过期、失联或报告
缺证据当作 `Green`。取消必须是协作式的，已经发生的安装、provider、tool 或
文件副作用不能回滚；已 claim 的消息也不能被超时静默重试，须以 journal 和
幂等键恢复或显式标记失败/放弃。

## 与 zenpi Rust 的映射建议

### 可复用的消息、邮箱、事件和协议

zenpi 已有两层可复用通信原语。第一层是 `src/dag.rs` 的 `DagStore`、
`DagMessage`、`DagRelation::{Parent, Grandparent, Sibling, Child, All}`、
`recipients`、`send`、`claim_inbox`、`ack`、`wait_for_message` 和 `touch_worker`：
它通过文件锁保护 bounded JSON store，已能按 parent、grandparent、直接
sibling、直接 child 寻址，并以 claim/ack 避免重启丢消息。第二层是
`src/session.rs` 的 `MailboxMessage`、`MailboxStatus`、`MailboxAction`、
`SessionMailbox` 与 `LiveSessionRegistry`：它提供 `digest`、`sequence`、
TTL、`claim_token`、`finish_claim`/`finish_claim_with_reply`、同 workspace
访问校验和 `owner_epoch` 隔离，适合承载可靠的工作请求、进度、Green/Blocked/
Failed 回执、取消和派生意图。新的 DAG 协议应优先桥接这两层，而不是再造
无限队列；`SessionMailbox` 更适合作为恢复真相，`DagStore` 可继续作为当前
邻接图和工具操作的轻量投影。

实时通知复用 `src/core.rs` 的 `AgentEvent`（如 `TurnAccepted`、`ToolProgress`、
`Provider`、`Warning`、`Error`、`Handoff`）以及 `src/runtime.rs` 的
`RuntimeEvent::{Accepted, Started, Queued, CancelRequested, Completed, Closed}`；
外部 JSONL 用 `src/headless.rs` 的有界帧、request-id admission、terminal
replay 和 `StdioEvent`。事件只用于唤醒、展示和关联，节点状态、消息 claim、
close 判定和派生 intent 必须持久化。`src/protocol.rs::MailboxRequest`
已有 `Send`、`Receive`、`Acknowledge`、`Claim`、`Complete`，可增加受限 DAG
envelope，至少携带 `dag_id`、`node_id`、`sender_node_id`、
`recipient_node_id`、`relation`、`generation`、`lease_id`、`request_id`、
幂等键和有界 payload；sender identity 不能由客户端伪造。

### 会话父子关系和四向通信

`src/core.rs::Turn.parent_id` 只表达同一会话的 turn 因果链，不能代替 DAG
边。建议在 `src/session.rs` 的 durable record/journal 中保存不可变的
`DagNodeRecord`：`dag_id`、`node_id`、对应 `session_id`、`parent_id`、
`generation`、`owner_epoch`/`lease_id`、状态、直接 child 集合和 closure proof。
grandparent、grandchild、sibling 尽量由持久化 parent/child 索引计算，避免复制
关系漂移；恢复时拒绝缺失父节点、重复 edge、跨 workspace 目标和过期 owner。

一个 worker 负责一个节点时，应由图索引解析目标 session，再通过同一套
addressed mailbox 通信：向 `parent` 发送结果、阻塞原因和状态；向
`grandparent` 发送聚合进度或升级；向每个直接 `sibling` 发送协作事件/依赖
通知；向每个直接 `child` 发送任务、验证、取消或补充工作。发送走
`SessionMailbox::enqueue` 或 `DagStore::send`，接收走 `claim_message`/
`claim_inbox`，完成走 `finish_claim`/`finish_claim_with_reply` 或 `ack`；回复
必须带原消息 digest、`generation` 和幂等键。不能用模糊广播、文件路径猜测或
provider 文本代替拓扑寻址。

### 保活、派生与 close 的最小机制

最小闭环是：父节点先写入子节点工作请求，子 worker 以 `LiveSessionRegistry::register`
登记 `owner_epoch`，运行期间周期性 `heartbeat`/`DagStore::touch_worker`，claim
工作后发布进度，完成时写 `Green`、`Blocked` 或 `Failed` 回执；父节点在同一
个持久化状态转移中聚合自身和后代状态。现有 `DagStore::can_close` 已递归
遍历全部 descendant：节点自身必须是 `green`，每个直接 child 以及继续向下的
grandchild/后代也必须是 `green`。建议把这个判定提升为带 generation、消息积压、
活动 lease、approval/tool batch 和 closure proof 校验的原子纯函数，并把
`dag_close_decision`/`NodeClosed` 写入 journal，保证重放只关闭一次。

只要任一后代不是 Green、缺失、过期、Blocked、Failed、仍有 claim，或出现新
工作/新 generation，节点就必须保持 `Alive`，继续 heartbeat，不得 close。新
工作先持久化 `spawn_requested`/generation，再由 `runtime::BackgroundRunner::try_submit`
或现有 `dag::spawn_worker` 派生新的 worker；成功取得 `JobId`/child identity
后再追加 admitted/started 事件。`SubmitError::QueueFull`、`Closed`、进程启动
失败或 owner 过期都保留非 Green，并继续保活或显式报告失败。新 worker 使用新
`lease_id`/`generation`；旧 worker 的 late completion 只能幂等拒绝或记录，不能
覆盖当前代状态。

### 四个既有 Rust 文件的具体落点

- `src/headless.rs`：`run_headless` 注册 live owner、逐帧解析有界 JSONL、做
  request-id admission、调用命令处理并在 EOF/显式停止时清理 owner；
  `run_async_streams` 连接异步有界 runtime。这里接入 `dag_send`、claim/recv、
  complete、heartbeat、derive、status/close 命令和 `StdioEvent` 输出；EOF 只
  结束输入端，已 admitted 的 DAG 工作仍须 drain/保活，不能直接变 Green。
- `src/core.rs`：`Agent` 保存 session、turn parent、`AgentEvent`、live owner
  和工具/provider 结果。这里放节点身份、邻接授权、lease、心跳与纯
  `evaluate_close(snapshot)`/`close_if_green`；provider 单次成功只能成为节点
  Green 的证据，不能单独触发 DAG close；跨节点派生/工具动作仍需 policy/approval。
- `src/session.rs`：`SessionStore` 的 append-only journal、replay/recovery、
  `SessionMailbox` 和 `LiveSessionRegistry` 是 durable DAG 边、状态、heartbeat、
  generation、claim、child receipt、spawn intent 和 `NodeClosed` 的落点。锁内
  更新要保证 close 与新消息线性化，并以 `owner_epoch` 拒绝旧进程完成新 claim。
- `src/runtime.rs`：`BackgroundRunner` 提供 bounded command/event queue、FIFO
  follow-up、`JobId`、`CancellationToken`、`RuntimeEvent` 和 bounded shutdown；
  它只负责 worker 生命周期，不理解 DAG 拓扑。派生必须由 core/session 驱动，
  取消在 provider chunk、mailbox claim、重试和 tool 边界检查；detach 不能回滚
  外部副作用，也不能视为 Green。

## 未决问题

源插件未规定 SDK 的准确最低版本、Node/Python 兼容矩阵、WebFetch/安装失败的
统一状态码、verifier 是否产生机器可读 JSON、报告持久化、并发审查、重试和取消
协议，也未定义项目文件的回滚方式。zenpi 还需固定 DAG 的最大深度/节点数、
heartbeat TTL、派生预算、grandchild 是否表示递归全部后代、sibling 是否必须
双向 ACK、消息 TTL 到期是 `Blocked`/`Expired`/`Waiting` 哪一种、closure proof
的摘要算法，以及 `Green` 是否必须同时意味着没有未完成 mailbox、provider、
tool、approval 和 pending spawn。还需决定 `DagStore` 与 `SessionMailbox` 是
单一真相还是“图投影 + durable mailbox”，并用 parent、grandparent、两个
sibling、多个 child 和 grandchild 的恢复/竞态测试固定 close 与新工作同时到达
时的 generation 线性化顺序。

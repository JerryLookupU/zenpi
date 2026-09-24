# `core/src/agent/builtins` 目录级学习汇总

## 范围与目录职责

本目录是 built-in agent 的配置入口，而不是 Rust 执行器。它把角色的等待策略、模型推理档位和开发者提示交给宿主加载；真正的任务状态、工具调用、会话关系、取消和重试都在宿主运行时实现。当前可见的 1:1 笔记对应两个文件；用户给出的 `/Users/wangweiya/.../vendor/...` 源码绝对路径在本工作区不可见，因此下述文件事实以已完成笔记为准，zenpi 映射再以本仓库的 `src/headless.rs`、`src/core.rs`、`src/session.rs`、`src/runtime.rs` 为落点核对。

## 模块清单

- `awaiter.toml`：非 Rust 的 awaiter 角色配置，关键键为 `background_terminal_max_timeout = 3600000`、`model_reasoning_effort = "low"` 和多行 `developer_instructions`；没有 Rust 函数/类型导出，但导出了“等待指定 command/task 直到 terminal state、只在终态报告”的行为契约。
- `explorer.toml`：零字节文件，没有 TOML 键、注释、类型、导出符号或默认 prompt；任何 explorer 默认值、工具白名单和错误都来自调用它的宿主，不能从此文件推断。

## 运行时数据流与控制流

配置加载后，awaiter 接收一个 command 或 task identifier，调用适当工具查询同一任务，任务处于运行态就重复 poll，等待间隔应逐步增加；收到 status 查询时只报告当前已知状态，然后继续等待。只有成功、失败或明确 stop 才退出，不能把 timeout、断连或未知结果猜成成功，也不能在轮询时重新提交 command。`background_terminal_max_timeout` 只提供等待上限候选值，源文件未规定单位、超时状态或错误编码；按数值量级推测为毫秒一小时只是宿主映射假设。`explorer.toml` 不参与这条数据流。

映射到 zenpi 后，建议采用：`headless.rs` 接收 JSONL 请求/流式事件；`src/protocol.rs` 校验 `MailboxRequest::{Send, Receive, Acknowledge, Claim, Complete}`、`MailboxOutcome::{Succeeded, Failed, Abandoned}`、ID、TTL、正文和关联 ID；`src/core.rs` 验证 DAG 关系、worker lease、policy 并驱动状态 reducer；`src/session.rs` 持久化邮箱与 DAG 事件；`src/runtime.rs::BackgroundRunner` 承载有界 worker、poll 和派生。执行结果从 `runtime.rs`/provider/tool 回到 `core.rs`，落成 heartbeat、Green、失败、派生或 close 事件，再由 `headless.rs` 按 `sequence`、`request_id`、`node_id`、`generation` 输出。高频 progress 可走 bounded event channel，可靠 work/ack/terminal 必须走 durable mailbox，不能只依赖 stdout。

## 错误、取消与恢复语义

源契约禁止修改任务、优化任务和无关动作；poll 是观察，不是重试。zenpi 应至少区分 `Running`、`Succeeded`、`Failed`、`Stopped`、`TimedOut`/`Unknown`，超时、取消、工具异常、`QueueFull`、`Closed`、`Panicked`、lease 过期和 TTL `Expired` 均是可观测的非绿色结果。`CancellationToken` 是合作式取消：worker 在 provider 输出块、tool batch、mailbox poll 和派生前检查；`mark_completed` 后的迟到 cancel 不得改写成功结果。shutdown grace 后 detach 只表示 runtime 不再等待，不表示外部副作用已回滚；可能产生副作用的 operation 必须以 operation journal/receipt 决定恢复，不能隐式重跑。

恢复应依赖 `SessionStore` 的 append-only journal、`MailboxMessage` 的 `digest`/`sequence`/`claim_token`、`LiveSessionRegistry` 的 `owner_epoch`、DAG `generation` 和幂等 `message_id`。重启时重建 parent/child 索引；旧 epoch 或旧 generation 的迟到 completion 拒绝；重复 terminal receipt 只能得到一致结果。`explorer.toml` 为空，所以不存在可兼容的取消、恢复或重试实现。

## 与 zenpi DAG 编排的 Rust 映射

### 通信原语与会话拓扑

可复用的最小通信原语是 `src/protocol.rs` 的 mailbox 请求/结果类型，配合 `src/session.rs` 的 `SessionMailbox`、`MailboxMessage`、`MailboxStatus`、`MailboxAction` 和 `LiveSessionRegistry`。`Claim` 提供单 worker 消费，`Complete`/`Fail` 提供结果回执，TTL、digest、sequence、claim token 和 owner epoch 提供防重复、防旧 owner 覆盖的边界；当前 mailbox 还限制 payload 为 64 KiB、消息数为 256、总 mailbox 为 8 MiB。实时状态可复用 `runtime.rs` 的 `RuntimeEvent`，持久状态变化建议追加 `dag_heartbeat`、`dag_green`、`dag_failed`、`dag_derive_intent`、`dag_derived`、`dag_close_deferred`、`dag_closed` 事件。建议新增 `DagEnvelope { message_id, request_id, dag_id, node_id, sender, recipient, relation, generation, payload, deadline_ms }`，其中 `relation` 仅允许 `Parent`、`Grandparent`、`DirectSibling`、`DirectChild`。

会话父子关系不能只从 `Turn::parent_id` 猜完整 DAG。可复用 `src/core.rs` 的 `Turn::parent_id` 作为对话父链，同时在 `src/session.rs` 持久化 `DagNodeRecord { node_id, session_id, parent_id, children, state, generation, owner_epoch }` 和边记录。`parent` 是一条显式边，`grandparent` 是两次 parent 索引，直接 sibling 是共同 parent 的 children 减去自身，直接 child 是当前节点的 children。路由器必须依据 server-side DAG 索引校验目标和 workspace；不能允许 worker 任意填写 session ID，也不能把 sibling 广播扩大为整棵树。

### 保活、派生与 close 门闩

worker 启动先以 `(session_id, node_id, owner_epoch, workspace)` 注册并获得 `generation`/lease，周期性 heartbeat；每次 poll、状态变化和 terminal 都记录事件。`DagCloseGate::can_close(node)` 必须在 session 单写者/事务边界内原子检查：节点自身为 `Green`，并且全部直接 child、grandchild 及递归全部后代均为 `Green`，同时没有 pending claim、approval 或 unsettled operation。只有检查通过才能追加唯一 `dag_closed`；`Completed`、`Shutdown`、`is_final` 或“当前不再运行”都不能单独等价于 DAG `Closed`。

只要自身或任一后代不是 `Green`、收到新工作、失败/超时、lease 仍需续租，worker 就保持 `Keepalive`，向 parent 报告原因，并保留可恢复状态。需要新 worker 时，先以 `(parent, work_id, generation)` 写幂等 `derive_intent`，再调用 `BackgroundRunner::try_submit`；成功 admission 后才写 `derived` 并建立 child 边。派生 worker 首先 claim 未完成 mailbox，再处理新工作；`QueueFull`/`Closed` 返回明确错误、继续保活且不丢任务，不得宣称 close。这个机制满足“节点自身及其全部 child/grandchild 全绿才允许 close，否则保活并派生新 worker 处理新工作”。

### 四个既有模块的具体落点

- `src/headless.rs`：唯一 wire I/O 所有者，把 `DagEnvelope`/mailbox action 转成 `StdioRequest`、`StdioEvent` 和 JSONL replay；处理 request id、sequence、断线重放和有界事件缓冲，但不在此判定 subtree Green。
- `src/core.rs`：新增或集中承载 `dispatch_dag_message`、`record_dag_state`、`evaluate_close_gate`、`spawn_recovery_worker`；已有 `Agent::register_live_owner`、`heartbeat_live_owner`、`claim_live_mailbox`、`finish_live_mailbox` 可作为 owner、claim、reply 的外壳。`AgentEvent`/`AgentSnapshot` 可扩展为 DAG 状态投影，但普通 agent `is_final` 不能直接关闭 DAG。
- `src/session.rs`：保存 `DagNodeRecord`/边与 heartbeat、Green、派生、close 事件，重建 `parent_of`、`children_of`、`ancestors`、`descendants` 投影；继续使用 mailbox 的文件锁、TTL、digest、claim token 和 owner epoch，保证旧 worker 不能完成新代任务。
- `src/runtime.rs`：用 `BackgroundRunner::spawn`、`try_submit`、`recv_timeout`、`try_cancel` 和 `CancellationToken` 承载节点 worker、poll、heartbeat 与派生；`JobOutcome::{Succeeded, Failed, Cancelled, Panicked}` 仅表示单 job 结果，必须经过 `DagCloseGate` 才能转成 DAG `Closed`。`RuntimeConfig` 的 bounded command/event/pending 队列用于限制派生风暴。

## 未决问题

1. `background_terminal_max_timeout` 的单位、是否为硬 deadline、超时错误码、指数退避的初始值/倍率/上限，以及调用方能否覆盖，均未由 TOML 定义。
2. zenpi 的 `Green` 是否要求审批落盘、所有 tool 成功、外部副作用可验证、无未决 operation，仍需产品协议明确；`Cancelled`、`Unknown` 和失败 child 是否允许由新 generation 替代也未定。
3. 动态派生与 close 的并发原子性需要确定：close 快照建立后新 child 是否阻止 close，还是必须通过 compare-and-append 重试；推荐新边一旦 durable 即纳入门闩。
4. DAG 节点与 session 是一对一还是一对多、grandparent/sibling 是否直连还是必须经 parent 转发、跨 workspace 是否绝对禁止，需要权限模型确认；默认应采用显式关系白名单和同 workspace 限制。
5. 原始外部目录当前不可读，无法重新核验后续源码变更；若恢复，应以源 hash 和 schema 复核配置事实，但不应放宽“自身及全部后代全绿才 close”的安全门闩。

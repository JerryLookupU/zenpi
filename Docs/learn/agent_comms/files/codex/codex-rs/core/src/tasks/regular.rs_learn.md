# AC-030 — codex/codex-rs/core/src/tasks/regular.rs

- source_id/item_id: `AC-030`
- source_path: `codex/codex-rs/core/src/tasks/regular.rs`
- source_hash: `36eaf4297c8700e5c610a12c64a8e40652e4f01b29bbc3a7f05bed495aa1be35`
- source_bytes: `2263`
- source_lines: `76`
- coverage: 已按顺序读取完整文件，字节范围 `0-2262`（共 `2263` 字节），行范围 `L1-L76`。

## 完整行为复盘

源文件是一个把“普通会话轮次”接入统一任务接口的薄适配器。`L1` 导入 `Arc`，说明任务实例和会话上下文以共享所有权传递；`L3-L4` 导入 `async_trait` 与 Tokio `CancellationToken`，任务实现是异步 trait 方法，并以协作取消为边界；`L6-L17` 导入 `TurnContext`、`run_turn`、`EventMsg`、`TurnStartedEvent`、`SessionStartupPrewarmResolution`、`TaskKind`、`UserInput`、`Instrument`、`trace_span` 以及父模块的 `SessionTask`/`SessionTaskContext`。这些导入也定义了本文件唯一的外部副作用：发事件、更新会话状态、执行实际 turn。

- `RegularTask`（`L19-L20`）是 `pub(crate)`、零字段结构体，并派生 `Default`。它不保存请求状态，因此同一实例可被 `Arc` 共享；并发安全来自外部会话/上下文对象，而不是内部锁。
- `RegularTask::new`（`L22-L26`）返回 `Self`，无输入校验、无失败分支、无资源分配；默认值与 `Default::default()` 等价。
- `SessionTask for RegularTask::kind`（`L28-L32`）返回 `TaskKind::Regular`，是静态任务分类，不产生 I/O。
- `span_name`（`L34-L36`）恒定返回 `"session_task.turn"`，供 tracing 关联同类任务；不会因会话或输入变化。
- `run`（`L38-L75`）接收 `self: Arc<Self>`、`Arc<SessionTaskContext>`、`Arc<TurnContext>`、`Vec<UserInput>` 和 `CancellationToken`，返回 `Option<String>`。首先在 `L45` 调用 `session.clone_session()` 得到本次运行使用的会话句柄，避免后续异步操作借用短生命周期引用。`L46` 创建 `run_turn` tracing span。
- `L47-L54` 明确先发 `EventMsg::TurnStarted`：事件包含 `ctx.sub_id` 作为 `turn_id`、`ctx.model_context_window()` 作为模型上下文窗口、`ctx.collaboration_mode.mode` 作为协作模式。注释说明这样做是为了让普通 turn 的首个生命周期事件不等待 startup prewarm；`sess.send_event(...).await` 是异步发送点，发送顺序先于预热消费和 `run_turn`。
- `L55` 调用 `sess.set_server_reasoning_included(false).await`，将本轮服务器 reasoning 标记重置为未包含。该状态更新在预热和模型运行之前完成；源文件没有错误返回值，失败语义由会话实现内部处理。
- `L56-L65` 消费普通 turn 的 startup prewarm。`consume_startup_prewarm_for_regular_turn(&cancellation_token)` 接受取消 token 并异步等待：`Cancelled`（`L60`）立即返回 `None`，不进入 `run_turn`；`Unavailable { .. }`（`L61`）也映射为 `None`，表示没有可复用预热客户端但仍会继续普通运行；`Ready(prewarmed_client_session)`（`L62-L64`）把装箱的客户端会话解引用后放入 `Some`。因此“预热不可用”不是任务失败，“取消”是唯一在本文件中显式短路执行的路径。
- `L66-L74` 调用 `run_turn(sess, ctx, input, prewarmed_client_session, cancellation_token)`，把会话、上下文、输入、可选预热客户端和同一个取消 token 原样交给核心执行器；`.instrument(run_turn_span).await` 保证异步执行在 tracing span 中完成，并把 `run_turn` 的 `Option<String>` 结果直接透传。源文件不重试、不捕获错误、不派生线程，也不改变输入顺序。

并发语义上，`Arc<Self>`/`Arc` 上下文允许任务句柄跨线程或异步任务共享；`send_event`、reasoning 标记、prewarm 消费、`run_turn` 是严格串行的本轮阶段。取消是合作式的：本文件在 prewarm 阶段显式检查结果，后续是否及时停止由 `run_turn` 和 provider 实现检查同一 `CancellationToken` 决定。源码没有锁、超时常量或 join；任务完成与会话关闭由上层调度器负责。

## 状态、取消、恢复与副作用

状态副作用有三类：发送 `TurnStarted` 事件（`L49-L54`）、清除/设置服务器 reasoning 包含标记（`L55`）、消费一次 startup prewarm 并可能把客户端会话交给 `run_turn`（`L56-L64`）。随后 `run_turn` 可能产生 provider、工具、持久化 transcript 等副作用，但本文件不定义其细节。取消只通过 `CancellationToken` 传播；prewarm 返回 `Cancelled` 时不调用 `run_turn`，返回 `Unavailable` 时仍执行。没有显式超时、自动重试、恢复或回滚逻辑；恢复能力取决于 `SessionTaskContext`、`run_turn` 和上层会话日志。`TurnStarted` 在取消被观察前已经可能发出，因此消费者必须允许“已开始但未完成”的生命周期。文件没有 detach、后台派生或外部进程控制；网络、工具和文件副作用均由下游 `run_turn`/provider 决定。

## 源内测试与行为判据

源内未包含测试（`L1-L76` 没有 `#[cfg(test)]` 或测试模块）。可独立验证的判据：构造 `RegularTask::new()` 后 `kind() == TaskKind::Regular`、`span_name() == "session_task.turn"`；给定可观测会话，调用 `run` 时第一条生命周期事件必须是带 `ctx.sub_id` 的 `TurnStarted`，且在预热解析之前；reasoning 标记必须先写入 `false`；prewarm 为 `Cancelled` 时 `run_turn` 不得被调用并返回 `None`，为 `Unavailable` 时仍应调用 `run_turn` 且预热参数为 `None`，为 `Ready(x)` 时参数应为 `Some(*x)`；所有正常返回值应与 `run_turn` 完全一致。

## zenpi Rust 映射

zenpi 已有的实现可以复用这段适配器的分层思想，但需要补齐 DAG 节点通信和保活规则。

- `src/core.rs`：`Agent` 是会话状态机；`Turn` 的 `parent_id`（`Turn::with_parent`，现有约 `L143-L184`）可作为节点的会话父子关系。建议增加 `DagNodeId`、`DagNodeState`（`Running/Green/Failed/Cancelled/WaitingChildren/Closed`）及 `DagNodeRecord { node_id, parent_id, children, status, generation }`，让一个 worker 的 turn 与 DAG 节点一一关联。现有 `submit_with_cancel`/`run_active_turn_cancelable`（约 `L3240-L3725`）适合作为“节点开始/执行/终态”入口；`set_worker_execution_binding`、`admit_blueprint_worker`、`settle_blueprint_worker`（约 `L1543-L1700`、`L2700` 附近）可承载 worker lease、预算和终态结算。`Agent::try_close`/`close`（约 `L5711-L5750`）目前是本地 agent 生命周期关闭点，必须在 DAG 规则下改为仅在节点及全部 child/grandchild 绿色时允许关闭。
- `src/session.rs`：`SessionStore` 的追加事件日志是持久化真相源；`Turn.parent_id` 和 session tree ancestry（`tree_ancestry_turns`、`fork_at_tree_leaf`，约 `L1038-L1128`）可表达父、祖父和分支。已有 `SessionMailbox`/`MailboxMessage`/`MailboxStatus`、`claim_next`、`claim_message`、`finish_claim`、`finish_claim_with_reply`（约 `L3171-L3555`）正好提供耐久邮箱、claim token、owner epoch、幂等完成和回复。建议新增事件类型 `dag_node_created/green/keepalive/derived/closed`，并在单次事务式 append 中记录 parent、直接 sibling、直接 child 的关系快照；恢复时从事件重建未闭合节点和待处理派生任务。
- `src/protocol.rs`：已有 `MailboxRequest::{Send,Receive,Acknowledge,Claim,Complete}`、`MailboxOutcome`（约 `L81-L113`）和 `MAX_MAILBOX_TEXT_BYTES`/TTL 限制，是可复用的消息/邮箱协议。建议扩展带 `relation`（`Parent/Grandparent/Sibling/Child`）、`node_id`、`correlation_id`、`generation` 的 DAG 消息体，保持现有 envelope 校验、大小上限和显式 claim/complete。事件协议可沿用 `StdioEvent`，增加 `DagEvent`/`NodeLifecycle`，使 headless 客户端能观察 keepalive 和 derived worker。
- `src/headless.rs`：现有 `Command::Mailbox` 分派和 `handle_mailbox_request`（约 `L6392-L6478`、`L8943-L9078`）已经把 JSONL 请求转换到 session mailbox；`MAX_PENDING_STEERS`、异步 provider/agent 有界邮箱和 reconnect replay（文件前部约 `L33-L78`）提供背压、重连和事件重放。建议在同一 dispatch 层加入 `dag_send/dag_receive/dag_ack`，按当前 session owner 做授权，收到 `KeepAlive` 时刷新节点 lease，收到 `Derived` 时仅入队而不在持锁期间启动 worker。worker 完成后先写终态事件，再发送 mailbox reply，避免客户端看到成功而日志未落盘。
- `src/runtime.rs`：`BackgroundRunner` 使用有界 `sync_channel`，`CancellationToken`、`RuntimeEvent::{Accepted,Started,CancelRequested,Completed,Closed}` 和 `shutdown_and_join`（约 `L49-L100`、`L171-L195`、`L279-L420`）是最小 worker 生命周期原语。建议每个 DAG node 使用一个 `JobId`，把 `Started/Completed` 映射为节点状态；用父级 token 派生 child token（或 `CancellationToken` 包装的层级 token）传播取消；keepalive 由 host 定时器/事件循环续租，超时只标记 `WaitingChildren`/`Cancelled`，不把未回收 child 误报为绿色。`Closed` 只能在聚合器确认节点和全部后代绿色后发出。
- `src/tool_runtime.rs`：`execute_tool_batch` 的有界并发、取消前置、线程 join 和“未知副作用不自动重试”语义（约 `L157-L353`）适合节点内部工具批次。直接 child 派生应通过 runtime 的提交队列而非工具线程自行 spawn；派生请求先做预算/审批，再写 `dag_node_derived` 事件，失败则保留父节点保活。
- `src/approval.rs`：`ApprovalCoordinator` 的 pending/accepted、持久化后释放、`cancel_all`/`emergency_cancel`（约 `L350-L468`）可作为 side-effect 消息的审批门。DAG 节点向 sibling/child 发送外部副作用请求时，必须绑定 `turn_id`、`node_id`、lease/policy digest；审批未持久化不得进入 handler，取消时应生成可恢复的 deny/abandoned 结果。
- `src/providers/**`：`anthropic.rs`、`codex.rs`、`deepseek.rs`、`google.rs`、`openai.rs` 及 `connection.rs`/`registry.rs` 是 provider 适配层。应让 provider 请求接收节点的 `CancellationToken`、deadline 和 `RequestScope`，把流式 delta 作为有界 `ProviderEvent` 发回 `core`；transport/部分响应错误继续按未知副作用处理，不能因为 worker 派生或 keepalive 失败自动重放同一外部请求。

面向 zenpi DAG 的最小可执行机制是：`DagNodeRecord` 保存 `parent_id`、直接 child 集合和状态；一个有界 mailbox/事件协议发送到 parent、grandparent、直接 sibling、直接 child；host 维护 lease/keepalive 和 `CancellationToken`；派生流程为“验证父节点仍活跃 -> 预算/审批 -> durable `dag_node_derived` -> `BackgroundRunner::submit`”；聚合器只有在自身为 `Green` 且递归所有 child/grandchild 为 `Green` 时才调用 `Agent::try_close`/发 `Closed`，否则保持 `WaitingChildren` 并允许派生新 worker。每个转移都应可由 session 事件重放验证，重复 message/complete 必须按 `message_id` 和 generation 幂等。

差异清单：源 `RegularTask` 只有单会话、单轮、单方向事件，没有 DAG 邻接表、邮箱寻址、keepalive、派生、递归绿色判定；zenpi 目前已有 durable mailbox、parent turn、bounded runtime 和取消/审批，但尚未把这些原语统一成节点聚合器，也未见“全后代绿色才 close”的强制门。可验证实现顺序是：先在 `session.rs` 增加节点事件投影和幂等状态机，再在 `protocol.rs`/`headless.rs` 暴露关系消息，接着在 `runtime.rs` 接入父子 token 与派生队列，最后在 `core.rs` 的 close/settle 路径加入递归绿色判定，并用 provider cancellation 测试端到端终态。

## 未决问题

无法从 `regular.rs` 单独确认：`SessionTaskContext::clone_session` 是否深拷贝或仅复制句柄；`send_event` 的失败/背压策略；`SessionStartupPrewarmResolution::Unavailable` 的具体原因；`run_turn` 的重试、持久化和 provider 取消检查频率；`TurnContext::sub_id` 是否始终唯一；上层是否保证同一 session 不并发运行多个 `RegularTask::run`。这些点需要继续读取对应模块或运行时测试才能确认。

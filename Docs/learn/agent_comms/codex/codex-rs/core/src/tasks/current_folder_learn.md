# `codex-rs/core/src/tasks` 目录级学习汇总

> learn_mode: `understand`；本文是目录汇总。实际核对的源码目录为 `/Users/wangweiyang/GitHub/codex/codex-rs/core/src/tasks`，逐文件研究笔记位于该 Codex 工作树的 `docs/researches/codex-rs/core/src/tasks`。用户给出的 `/Users/wangweiyang/GitHub/Docs/learn/...` 在当前环境不存在。

## 一、目录职责

本目录是 Codex Core 的会话任务层：把一次会话回合包装成统一的 `SessionTask`，负责任务创建、单活跃任务调度、取消、完成通知、事件流和回合指标；具体任务只实现业务工作，公共框架负责把它们接入 `Session`、`TurnContext`、历史记录和协议事件。它既包含前台模型回合，也包含审查子代理、上下文压缩、Git 幽灵快照、撤销和用户 Shell 等非同质工作，因此本质上是“会话状态机之上的可取消异步任务适配层”。

共同生命周期是：入口调用 `Session::spawn_task`，先以 `TurnAbortReason::Replaced` 终止旧任务，建立 `CancellationToken`、`Notify` 和 `AbortOnDropHandle`，注册 `RunningTask`；Tokio worker 调用任务的 `run`，随后刷新 rollout，并在任务未被取消时统一调用 `Session::on_task_finished` 发出 `TurnComplete`。取消则由 `abort_all_tasks` 取出 `ActiveTurn`，取消令牌、短暂等待优雅退出，必要时 abort handle，再调用任务自己的 `abort` 清理，最后发出 `TurnAborted`。这个“公共壳统一收尾、任务自带局部清理”的边界，是迁移到 zenpi DAG worker 时最有价值的设计。

## 二、模块清单（每个文件）

| 文件 | 一句话职责 | 关键导出/验证点 |
|---|---|---|
| `mod.rs` | 定义任务统一契约、任务上下文和调度/取消/完成闭环。 | `SessionTask`、`SessionTaskContext`、`Session::spawn_task`、`Session::abort_all_tasks`、`Session::on_task_finished`；重导出 `CompactTask`、`GhostSnapshotTask`、`RegularTask`、`ReviewTask`、`UndoTask`、`UserShellCommandMode`、`UserShellCommandTask`、`execute_user_shell_command`。 |
| `regular.rs` | 执行标准用户—模型回合，并消费会话启动预热连接。 | `RegularTask::new`、`SessionTask for RegularTask`；发出 `TurnStarted`，根据 `SessionStartupPrewarmResolution` 选择预热会话，调用 `run_turn`。 |
| `review.rs` | 创建隔离的代码审查子代理，过滤其事件并把结果写回主会话。 | `ReviewTask::new`、`exit_review_mode`；内部 `start_review_conversation`、`process_review_events`、`parse_review_output_event`；用 `async_channel::Receiver<Event>` 接收子代理事件。 |
| `compact.rs` | 按 provider 在本地压缩与远程压缩之间选择，替换会话历史以释放上下文。 | `CompactTask`、`SessionTask for CompactTask`；调用 `run_compact_task` 或 `run_remote_compact_task`，保留 `GhostSnapshot` 语义。 |
| `undo.rs` | 查找最近的 `ResponseItem::GhostSnapshot`，在阻塞线程中恢复 Git 状态并更新历史。 | `UndoTask::new`、`SessionTask for UndoTask`；发出 `UndoStarted`/`UndoCompleted`，实现明确的无快照、Git 错误和取消结果。 |
| `user_shell.rs` | 执行用户 Shell 命令，支持独立回合和活跃回合辅助两种生命周期。 | `UserShellCommandMode`、`UserShellCommandTask::new`、`execute_user_shell_command`；发出 `ExecCommandBegin`/`ExecCommandEnd`，将输出写入历史或注入 pending input。 |
| `ghost_snapshot.rs` | 后台创建 Git 幽灵提交，为 undo 提供恢复点，并通过 readiness gate 阻挡过早的工具调用。 | `GhostSnapshotTask`；内部 `format_snapshot_warnings`、`format_large_untracked_warning`、`format_ignored_untracked_files_warning`、`format_bytes`；使用 `Readiness`/`Token`、`oneshot` 和 `spawn_blocking`。 |
| `ghost_snapshot_tests.rs` | 测试幽灵快照大目录警告是否带阈值，以及阈值为 `None` 时不告警。 | `large_untracked_warning_includes_threshold`、`large_untracked_warning_disabled_when_threshold_disabled`；通过 `use super::*` 覆盖私有格式化函数。 |
| `mod_tests.rs` | 测试回合网络代理指标及 `active`、`tmp_mem_enabled` 标签。 | `emit_turn_network_proxy_metric_records_active_turn`、`emit_turn_network_proxy_metric_records_inactive_turn`；使用 `InMemoryMetricExporter` 验证 OpenTelemetry 数据。 |

## 三、运行时数据流与控制流

### 1. 公共任务路径

`Session::submit_user_input`、`compact`、`undo`、review 入口或 Shell 入口构造任务后进入 `spawn_task`。调度器先清掉旧 `ActiveTurn`，同步 MCP 请求头和 connector 状态，再生成 `RunningTask { done, kind, task, cancellation_token, handle, turn_context, _timer }`。任务体在 Tokio 中运行，结束后执行 `flush_rollout`；没有取消时进入 `on_task_finished`，处理 pending input hook、工具调用数和 token/network proxy 指标，最后发送 `EventMsg::TurnComplete`。

### 2. 各任务的数据流

* `RegularTask`：发送 `TurnStarted` → 重置 server reasoning 标志 → 消费 prewarm → `run_turn` 流式驱动 provider、工具和模型循环 → 返回最后一条 agent message，由公共层发 `TurnComplete`。
* `ReviewTask`：复制并收紧配置，关闭 Web search、`SpawnCsv`、`Collab`，设置 `REVIEW_PROMPT` 与 `AskForApproval::Never` → `run_codex_thread_one_shot` 建立一次性子代理 → 从 `async_channel` 接收事件；抑制 delta 与重复 assistant item，把最终 `TurnComplete.last_agent_message` 解析为 `ReviewOutputEvent` → 发 `ExitedReviewMode`，记录用户/助手消息并物化 rollout。它是“父会话启动子会话并消费子会话事件”的直接范例。
* `CompactTask`：根据 provider 分支到本地或远程压缩，生成摘要、替换历史并重新计算 token；上下文超限时裁旧项重试，保留快照项以便 undo。
* `GhostSnapshotTask`：启动可选慢操作 warning 的 `oneshot` → `spawn_blocking` 调用 `create_ghost_commit_with_report` → 记录 `GhostSnapshot` 或 warning → 无论成功、非 Git 仓库、错误或取消都要释放 `tool_call_gate` 的 `Token`，避免会话永久卡在未 ready。
* `UndoTask`：从历史尾部找最近快照 → `spawn_blocking` 调用 `restore_ghost_commit_with_options` → 成功则移除该历史项并保留 reference context，失败则保留历史 → 用 `UndoCompleted` 报告结果。
* `UserShellCommandTask`：准备登录 shell、环境和 `SandboxPolicy::DangerFullAccess` → 发 `ExecCommandBegin` → `execute_exec_request(...).or_cancel(&cancellation_token)`，通过 `StdoutStream` 实时转发 → 发 `ExecCommandEnd`；`StandaloneTurn` 完整记录回合，`ActiveTurnAuxiliary` 优先向当前 pending input 注入，注入失败再落历史。

### 3. 事件、指标与持久化

任务内部事件通过 `Session::send_event` 发给宿主，历史通过 conversation/rollout API 记录；`Notify` 只表达任务完成，不承载业务消息；`CancellationToken` 表达协作式取消；`oneshot` 表达快照完成或 warning timer 的一次性信号；审查使用 `async_channel` 作为子代理事件流。公共层的 `TURN_E2E_DURATION_METRIC`、token/tool/network proxy 指标在回合结束统一补齐，测试只直接验证 `turn.network_proxy` 的值和属性。

## 四、错误与取消语义

错误通常不靠 `SessionTask::run` 返回 `Result` 传播，而是由具体任务记录日志、写入历史或发送协议事件；`run` 的 `Option<String>` 只代表可选的最终 agent message。`ReviewTask` 的子代理启动失败、通道关闭和无效 JSON 最终多表现为 `None` 或纯文本回退，说明 DAG 版不能只依赖“通道关闭即完成”，必须为失败、取消、超时和未知结果设置显式终态。Git 操作放在 `spawn_blocking`，会区分无 Git 仓库、Git 错误和 join/panic；Shell 将取消映射为失败状态和 `exit_code = -1`；压缩取消时不改写历史；Ghost Snapshot 在取消时不记录快照但必须释放 readiness。

取消是分层的：上层取消 `CancellationToken`，任务在 provider、Shell、Git 或子代理边界观察它；框架最多等待 `GRACEFULL_INTERRUPTION_TIMEOUT_MS = 100` 毫秒，随后 abort Tokio handle，但 abort 不能回滚已经发生的外部副作用。被用户明确中断时，框架还写入 `turn_aborted` 指示和可恢复的 rollout marker，再发 `TurnAborted`；`ReviewTask::abort` 额外调用 `exit_review_mode(..., None, ...)`。该语义适合映射为 zenpi 的“取消请求已发出”和“最终 outcome=cancelled/unknown”两阶段，不应把取消误报为成功或自动重试。

## 五、与 zenpi Rust 的映射建议

### 5.1 直接复用的通信原语

1. **可靠点对点消息：`SessionMailbox`/`mailbox_slash_view`。** `src/session.rs` 已有面向 session 的持久邮箱，`src/headless.rs` 已把 Send、Receive、Acknowledge、Claim、Complete 映射到协议请求；它有消息 ID、序列、TTL、大小和数量上限，适合 worker → parent、parent → child 以及跨进程 sibling 的命令/结果。消息 payload 应携带 `sender_node_id`、`recipient_node_id`、`parent_node_id`、`message_id`、`correlation_id`、`kind`、`generation` 和 `delivery_attempt`，而不是只传裸文本。
2. **短生命周期调度：`BackgroundRunner` 的 bounded `sync_channel`。** `src/runtime.rs` 已有 command/event 两条有界通道，`RuntimeEvent::{Accepted, Queued, Started, CancelRequested, Completed, Closed}` 能承载 admission、排队、开始、取消和终态。可为每个 DAG worker 建立一个 runner，或由一个 supervisor 为多个 worker 复用；不要用无界 channel 把新工作、进度和历史混在一起。
3. **流式观察：`AgentEvent`/provider event + `AsyncEventBuffer`。** `src/headless.rs` 已按 provider/agent 分开缓存，限制数量和字节，优先保留 admission、tool call、tool result 等正确性边界。DAG 的 ChildState、KeepAlive、DerivedWorker、WorkerClosed 也应作为不可丢弃的优先事件，普通 token/progress 可丢弃并带 dropped 统计。
4. **协作取消与一次性完成：`runtime::CancellationToken`、`InputBoundaryGate`、done channel。** 父 worker 取消时用 child token 传播给后代；同一 worker 的新一轮工作应在 `InputBoundaryGate` 的安全边界进入，不能在 tool batch 或 preparation 中间消费；每个 job 保留独立 done/terminal marker，避免晚到结果重新进入已替换 worker。

### 5.2 会话父子关系与 DAG 拓扑

现有 `core::Turn` 的 `parent_id` 只能表达一条直接父链，适合 steering 的 turn 关系，但不足以表达 worker 的全量拓扑。建议在 `src/core.rs` 增加受校验的 `WorkerNode`/`WorkerId` 记录：`node_id`、`session_id`、`parent_id`、`root_id`、`depth`、`generation`、`state`、`children`、预算/lease 和创建原因。直接 sibling 通过同一 `parent_id` 的 child 集合得到，grandparent 通过两次 parent 查找得到，direct child 通过 `children` 得到；所有关系变更都拒绝自环、重复 child、跨 session 错配和超过 fan-out/深度上限。

`src/session.rs` 已有 `SessionTree`、`tree_ancestry_turns`、`fork_at_tree_leaf`、`fork_to` 和 `Turn.parent_id`，可作为 transcript/context 的父子基础；持久拓扑应写成类型化的 `worker_spawned`、`worker_state_changed`、`worker_edge_removed` 等 session journal event，并以 session ID 做隔离。`fork_to` 明确不会复制 pending work、mailbox、request/lifecycle/reconnect 记录，所以派生新 worker 时应复制必要的只读上下文，再显式建立新的 mailbox owner、lease 和 parent edge，不能把 fork 当成“继承活跃执行权”。

建议把会话层关系分成两层：`SessionStore`/`SessionMailbox` 是持久身份与消息边界；`WorkerNode` 是同一 DAG 内的执行节点。一个 child 可以拥有自己的 `Agent`/`BackgroundRunner`，但必须保存 `parent_session_id` 与 `parent_node_id`；父、grandparent、sibling、child 的收发都通过授权后的 node selector 解析，不能让 worker 任意扫描所有 session。

### 5.3 保活与派生的最小机制

在 `src/core.rs` 增加显式状态聚合，至少需要：

```text
WorkerState = Running | Waiting | KeepAlive | Green | Failed | Cancelled | Closed
is_green_subtree(node) = node.self_green && every descendant.is_green_subtree()
```

“绿”必须是稳定的成功语义：本节点工作已得到成功终态、无未确认 tool/provider outcome、无 pending child、所有直接/间接 child 均为 `Green`；`Failed`、`Cancelled`、`UnknownOutcome`、超时、仍有 mailbox work 或尚未结算的 child 都不是绿。`try_close` 只有在 `is_green_subtree` 为真时才写 `worker_closed` 并释放 runner/mailbox owner；否则返回/记录 `KeepAlive`，保持父节点与未完成后代的通信能力。

派生新 worker 的最小闭环是：`derive_worker(parent_id, reason, input)` 校验 parent 仍可接收工作、创建新的 `node_id/generation/lease`、写入 `worker_spawned`、登记 child edge、建立 child `CancellationToken` 与 bounded command/event 通道、发送 `WorkerStarted`，再把新工作投递给新节点。旧 worker 不因派生而关闭，它进入 `KeepAlive` 并继续收集 child state；新节点完成后以 `ChildStateChanged` 通知父节点，父节点重新计算全子树绿色条件。派生次数、深度、并发数和 wall-time 要接入现有 `WorkerExecutionBinding`、`worker_budget`、`provider_request_deadline` 与 `renew/settle/cancel_blueprint_worker`，避免“保活”绕过治理预算。

### 5.4 四个 zenpi 文件的具体落点

* **`src/headless.rs`：宿主边界与路由。** 保留 `run_async_streams`、`AsyncEventBuffer`、`PendingSteer` 和现有 bounded backpressure；扩展请求/事件的 node 定位字段，把 mailbox 的 Send/Receive/Claim/Complete 作为 DAG 控制面入口。将 admission/terminal/child-state/keep-alive/derived-worker 标成 priority event；provider 流和普通进度仍受字节上限约束。`PendingSteer.target_job` 可同时关联 `target_node_id`，当 worker 尚在 admission→`Agent::submit` 窗口时继续排队，而不是错误返回 `no_active_turn`。
* **`src/core.rs`：状态机、拓扑和关闭判定。** 复用 `Agent` 的 `phase`、`active_turn_id`、`submit_with_cancel`、`run_active_turn_cancelable_with_events` 和已有 `Turn::parent_id`，新增 worker registry、父子边、状态聚合、`spawn_worker`/`derive_worker`/`send_worker_message`/`close_worker`。`AgentEvent` 增加 child state 与 close/keep-alive/derivation 的不可丢失生命周期事件；`try_close` 成为 `is_green_subtree` 的唯一关闭闸门。现有 `admit_blueprint_worker` 和 binding 校验负责授权、lease 与预算，不应被新的 mailbox 入口绕过。
* **`src/session.rs`：持久拓扑、邮箱和恢复。** 复用 `SessionStore` 的追加锁、序列、WAL/恢复、`SessionMailbox` 的 TTL/ACK/claim/complete 及 `SessionTree` 的 ancestry/fork。为 worker spawn/state/terminal 写 durable event，先写 `worker_spawned`/message reservation，再允许调度；结果和 terminal marker 必须先于 stdout/host event。恢复时把无 terminal 的 worker 置为 `UnknownOutcome`/需显式重试，不能假设进程退出等于失败或成功；GC/retire 必须同时检查 session mailbox，避免孤儿队列。
* **`src/runtime.rs`：执行监督与取消。** 复用 `BackgroundRunner` 的单 active + bounded pending 队列、`RuntimeEvent` 顺序和 shutdown grace；每个 worker job 使用独立 `CancellationToken`，parent cancel 时显式 cancel 后代，child cancel 不默认取消 parent/sibling。扩展事件以表达 `WorkerSpawned`、`ChildState`、`KeepAlive`、`Derived`、`UnknownOutcome`；保留 `closed` 原子位与 join 行为，因为 event channel 可能被 host 提前放弃但 job 仍需有权威终态。若 DAG 需要并行 child，采用每节点 runner + supervisor，而不是破坏现有 runner 的单 active 不变量。

## 六、未决问题

1. **DAG 还是树：** 现有 `SessionTree` 和 `Turn.parent_id` 是单父模型；需求中的 parent/grandparent/sibling/child看起来是树，但名称称为 DAG。必须明确是否允许一个 worker 多父、共享 child；若允许，`is_green_subtree` 需要引用计数和重复遍历去重。
2. **绿色定义与结果优先级：** provider 成功但 child 失败、child 被取消、tool outcome 未知、父节点主动关闭时分别如何判定，需固定状态表；否则 close 会出现竞态和错误早关。
3. **消息保证：** mailbox 的持久投递与 runtime 的内存事件如何对齐，ACK/claim 后崩溃怎样恢复，sibling 消息是否允许广播，需规定顺序、幂等键、重试上限和 TTL。
4. **取消传播：** parent cancel 是否递归取消所有 descendants，grandparent cancel 与派生 worker 的 lease 到期如何交互，child 失败是否只通知父节点还是触发全树保活，尚需策略。
5. **派生触发与治理：** 哪些新工作必须 derive、哪些可复用当前 worker，派生是否受 `worker_budget`、模型额度、fan-out、深度和 wall-time 共同限制，需和 blueprint 生命周期统一。
6. **会话隔离与恢复：** child 是同一 `SessionStore` 中的节点还是独立 fork session；独立 session 如何共享只读上下文而不继承 mailbox/lifecycle ownership；进程重启后如何恢复 live worker，需由 session schema 与协议共同确定。
7. **测试缺口：** 现有测试只覆盖网络代理指标和快照 warning，尚未覆盖 spawn/abort/finish、事件背压、DAG sibling 路由、全子树绿色关闭、保活派生、崩溃恢复和幂等投递；这些应成为实现前的验收矩阵。

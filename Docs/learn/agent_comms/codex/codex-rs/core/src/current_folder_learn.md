# `codex-rs/core/src` 目录级学习汇总

## 范围与目录职责

本汇总覆盖该目录对应的 18 份逐文件中文笔记：根级 `external_agent_config.rs`，`agent/` 下的配置、角色、资源守卫、控制面和状态 reducer，以及 `tasks/` 下的会话任务适配器和测试/配置文件。整体职责是把一个 session 配置成可运行的 agent，按角色创建或恢复 child thread，向 thread 投递输入，调度可取消的 turn/task，记录 rollout 与事件，并在任务结束后通知直接 parent。这里的中心模型仍是“一个 `Thread`/`Session` 的生命周期”，不是完整 DAG：Codex 现成实现主要有直接 parent 通知、thread depth 限制和单 agent 的 `is_final`，没有 grandparent、直接 sibling、多级 descendant 的通信，也没有“自身及全部 child/grandchild 全绿才 close”的聚合门。因此它适合作为 zenpi 的控制面、邮箱、任务取消和资源配额参考，不能直接当作 DAG 实现。

## 模块清单

- `agent/agent_names.txt`：101 个按行排列的科学家/思想家昵称（如 `Euclid`、`Newton`、`Jason`），无 Rust 导出；由 `control.rs` 读取并用于可读身份，不能代替 `JobId`、`session_id` 或 `node_id`。
- `agent/builtins/awaiter.toml`：声明 `background_terminal_max_timeout`、`model_reasoning_effort` 和只等待 terminal state 的 `developer_instructions`，无 Rust 导出；它规定轮询、保守报告、明确 stop 才退出，不规定 DAG 状态。
- `agent/builtins/explorer.toml`：空 TOML 文件，无键、类型或导出，不能从源文件推导 explorer 行为。
- `agent/control.rs`：多 agent 控制面；关键导出为 `AgentControl`、`SpawnAgentOptions`、`spawn_agent`、`spawn_agent_with_options`、`resume_agent_from_rollout`、`send_input`、`interrupt_agent`、`shutdown_agent`、状态查询和完成 watcher。
- `agent/control_tests.rs`：控制面的异步集成测试，无生产导出；覆盖输入投递、状态 watch、fork 历史、parent 完成通知、最大 thread 数和 SQLite 身份恢复。
- `agent/guards.rs`：同一用户 session 的派生资源守卫；关键导出为 `Guards`、`SpawnReservation`、`next_thread_spawn_depth`、`exceeds_thread_spawn_depth_limit`，提供原子槽位、昵称池和 RAII 回滚。
- `agent/guards_tests.rs`：守卫测试，无生产导出；验证 depth、reservation `commit`/`Drop`/release 幂等、失败回滚、昵称池耗尽后的序号化重置。
- `agent/mod.rs`：agent 模块装配入口；重导出 `AgentControl`、`AgentStatus`、`agent_status_from_event`、`is_final`、`exceeds_thread_spawn_depth_limit` 和 `next_thread_spawn_depth`。
- `agent/role.rs`：角色配置层叠、内建角色和 spawn 工具描述；关键导出为 `DEFAULT_ROLE_NAME`、`apply_role_to_config`、`resolve_role_config`、`spawn_tool_spec` 及 `built_in` 查询函数。
- `agent/role_tests.rs`：角色、profile/provider 优先级、skills 禁用和 `spawn_tool_spec` 的测试，无生产导出。
- `agent/status.rs`：纯事件 reducer；关键导出为 `agent_status_from_event` 与 `is_final`，把 `TurnStarted`、`TurnComplete`、`TurnAborted`、`Error`、`ShutdownComplete` 映射为 `AgentStatus`。
- `external_agent_config.rs`：检测并导入 Claude 配置、skills 和 `CLAUDE.md` 到 Codex 配置；关键导出为 `ExternalAgentConfigService`、`ExternalAgentConfigDetectOptions`、`ExternalAgentConfigMigrationItem`、`detect` 和 `import`。
- `tasks/compact.rs`：`CompactTask` 的 local/remote 压缩路由、telemetry 和 delegated result 适配；关键导出为 `CompactTask`，多数 delegated 结果归并为 `None`。
- `tasks/ghost_snapshot.rs`：`GhostSnapshotTask` 在 `spawn_blocking` 中创建 Git ghost commit，发送 warning，并调用 `tool_call_gate.mark_ready`；同时提供警告格式化函数。
- `tasks/mod.rs`：统一 `SessionTask` 生命周期入口；重导出 `CompactTask`、`GhostSnapshotTask`、`RegularTask`、`ReviewTask`、`UndoTask`、`UserShellCommandTask`，并提供 `SessionTaskContext`、`SessionTask`、`Session::spawn_task`、`abort_all_tasks` 和约 100ms 的中断宽限。
- `tasks/regular.rs`：`RegularTask` 先发 `TurnStarted`，消费 startup prewarm，再调用 `run_turn`；关键导出为 `RegularTask`。
- `tasks/review.rs`：`ReviewTask` 启动受限 review 子代理，串行转发非增量事件，解析 `ReviewOutputEvent`，写入 review 收尾记录；关键导出为 `ReviewTask`、`exit_review_mode`。
- `tasks/undo.rs`：`UndoTask` 查找最近 `GhostSnapshot`，在阻塞池恢复 Git 状态并发送 `UndoStarted`/`UndoCompleted`；关键导出为 `UndoTask`。
- `tasks/user_shell.rs`：`UserShellCommandTask` 与 `execute_user_shell_command` 执行最长一小时的用户 shell，流式发送 `ExecCommandBegin/End` 并持久化结果；关键导出为 `UserShellCommandMode`、`UserShellCommandTask`、`execute_user_shell_command`。

## 运行时数据流与控制流

1. **配置和创建**：`apply_role_to_config` 按 session flags 层叠角色配置，未指定的 profile/provider 继续保留。`AgentControl::spawn_agent_with_options` 先升级 `Weak<ThreadManagerState>`，再由 `Guards::reserve_spawn_slot` 预留资源，计算 `ThreadSpawn` 的 parent、depth、nickname、role；需要 fork 时先 `ensure_rollout_materialized`/`flush_rollout`，复制可读历史，创建 child，`commit` reservation，最后发送 `Op::UserInput`。`resume_agent_from_rollout` 则从 rollout/SQLite 恢复 thread id、nickname、role、parent 和 depth。
2. **输入和状态**：`send_input` 向指定 thread 投递 `Op::UserInput`；child 状态由事件 reducer 与 `watch::Receiver<AgentStatus>` 投影。完成 watcher 只向直接 parent 注入一跳的合成通知；parent 被移除时通知可丢弃。`agent_names.txt` 只是展示名池，恢复时应重放 node/thread 到名称的绑定，不能依赖随机选择顺序。
3. **任务调度**：`Session::spawn_task` 先取消旧 task，建立父子 `CancellationToken`、`RunningTask` 和 `Notify`，后台运行 `SessionTask::run`；结束后 flush rollout，`on_task_finished` 结算 token/tool 指标并发送 `TurnComplete`。`RegularTask` 进入 `run_turn`；`ReviewTask` 经 `async_channel` 接收子代理事件并抑制增量；`UserShellCommandTask` 由 `StdoutStream` 将 stdout 变成事件；`GhostSnapshotTask` 和 `UndoTask` 把 Git 操作放到 blocking pool；`CompactTask` 选择 local/remote delegated 路径。
4. **结果事实源**：不少 task 返回 `None`，真实结果在 `Session::send_event`、rollout materialization、会话历史或 `ExecCommandEnd` 中。`ReviewTask` 将合法 JSON 解析成结构化 review，解析失败仍保留文本解释；user shell 先发 begin/stream/end，再把完整 `ExecToolCallOutput` 写入会话。
5. **资源约束**：创建 child 受最大 thread/depth 和 `SpawnReservation` 约束；角色配置控制可用工具、model reasoning 和 provider。外部配置 import 是检测—写入流程，前面文件已写成功时后续失败不会整体回滚。

## 错误、取消与关闭语义

控制面使用 `Weak` manager：manager 已释放时返回 `unsupported operation: thread manager dropped`，缺失 thread 返回 `NotFound`/`ThreadNotFound`，资源超限返回 `CodexErr::AgentLimitReached`。未 `commit` 的 `SpawnReservation` 在 `Drop` 时释放原子槽位；已提交线程必须显式 release，昵称的 used 历史不会因 release 回滚。`AgentStatus::is_final` 的 `Completed`、`Errored`、`Shutdown`、`NotFound` 只表示当前 agent 不再等待，不表示业务成功或 DAG 的 green。

取消全部是协作式的：`CancellationToken` 需要 provider、mailbox wait、task executor 或 worker 主动检查；`tasks/mod.rs` 先 cancel，等待约 100ms，再 abort Tokio handle，并写 `TurnAborted`/中断 marker。`runtime::BackgroundRunner` 暴露 `JobOutcome::{Succeeded,Failed,Cancelled,Panicked}` 与 `RuntimeEvent::{Accepted,Started,CancelRequested,Completed,Rejected,Closed}`；`Closed` 是 runner 终态，不是节点 green。`spawn_blocking` 的 Git 操作、shell 子进程、provider 请求和工具调用可能在取消后仍留下副作用，取消不回滚；user shell 取消仍持久化失败输出，`ReviewTask` 的 EOF/解析失败可能收束为 `None`，上层不能只凭 `Option` 判定成功。`awaiter.toml` 的重复 tool call 只能是同一任务的轮询，不能隐式重新提交副作用任务；超时应单列为 `TimedOut`/`Unknown`，不能假定成功。

## 与 zenpi Rust 的映射建议

### 通信原语和关系寻址

优先复用现有三层原语，不让 provider 或 stdout 成为 DAG 真相源。

- **DAG 快照和短消息**：`src/dag.rs` 的 `DagStore` 使用文件锁 JSON 快照；`DagNode` 已有 `parent`、`children`、`status`、`worker`、`worker_heartbeat_ms`，`DagRelation::{Parent,Grandparent,Sibling,Child,All}`、`recipients`、`send`、`inbox`、`claim_inbox`、`ack`、`wait_for_message` 可直接覆盖 worker 到 parent、grandparent、直接 sibling、直接 child 的寻址。`MAX_DAG_NODES`、`MAX_DAG_MESSAGES`、`MAX_DAG_BODY_BYTES` 提供有界保护。
- **耐久消息/邮箱/协议**：`src/protocol.rs` 的 `MailboxRequest::{Send,Receive,Acknowledge,Claim,Complete}`、`MailboxOutcome`、`StdioEvent` 和 `CheckpointCursor` 适合作为 JSONL wire protocol；`src/session.rs` 的 `SessionMailbox`、`MailboxMessage`、`MailboxStatus`、`MailboxAction`、`LiveSessionRegistry` 提供 digest、sequence、TTL、claim token、owner epoch 和 finish reply，适合跨重启可靠交付。
- **进程内事件和 worker**：`src/core.rs` 的 `AgentEvent` 记录 turn/provider/tool/error/warning；`src/runtime.rs` 的 bounded `mpsc`、`BackgroundRunner`、`JobId`、`RuntimeEvent` 和 `CancellationToken` 承载实时控制、背压、取消和 join。`src/headless.rs` 负责 stdin/stdout JSONL、事件缓冲、request id 和 replay，不应在 wire 层自行计算 subtree green。

建议在消息 envelope 中固定 `dag_id`、`node_id`、`session_id`、`parent_id`、`sender`、`recipient`、`relation`、`message_id/request_id`、`reply_to`、`generation`、`lease_id`、`ttl_ms` 与 `kind`。路由目标必须由 graph owner 根据真实邻接计算并授权，不能信任客户端任意填写 sender/target；发送到 sibling 只能是共同 parent 的直接 child 去掉自身，grandparent 必须沿两条 parent 边上溯，direct child 只能是当前 `children` 集合。

### 会话父子关系

Codex 的 `SessionSource::SubAgent(ThreadSpawn{parent_thread_id, depth, ...})` 和直接 parent watcher 是最接近的先例，但 zenpi 需要把三种关系分开：`core::Turn.parent_id` 是对话/turn 关联，`session_tree` 是 transcript entry graph，`DagNode.parent/children` 才是 worker 拓扑。建议在 `src/session.rs` 的 durable journal 或 DAG 元数据中保存不可变的 `DagNodeRecord { dag_id, node_id, session_id, parent_id, generation, children, state, lease_id }`；grandparent/sibling/child 通过索引计算，不把关系藏在自由格式文本里，也不要让 child 持有 parent 的 `Arc` 造成引用环。`LiveSessionRegistry` 继续负责 workspace、owner epoch 和心跳，旧 owner 不能完成新 generation 的工作。

### 保活、派生和最小 close gate

zenpi 当前 `DagStore::can_close` 已表达“自身为 `green` 且递归全部 descendant 为 `green`”的读取检查，`touch_worker` 写心跳，`spawn_worker` 启动 headless worker 并保留 stdin，`send_to_worker` 可以向已有 child 发送 follow-up。把它固化为以下最小闭环：

1. worker 启动时先 durable 写入 `dag_node_created`/parent edge，注册 `LiveSessionOwner`，绑定 `node_id + session_id + generation + lease_id`，再用 `BackgroundRunner` 获得 `JobId`。
2. 执行期间周期调用 `DagStore::touch_worker` 和 `Agent::heartbeat_live_owner`，从 `SessionMailbox` claim 新 work；heartbeat 只表示活着，不表示 green。
3. 工具、provider、approval、子任务及本节点的 side effect 都有终态且审计写入后，才可把本节点标为 `GreenCandidate`。`Mailbox Complete` 只表示一项工作完成，不能单独把节点标绿。
4. 在同一可串行化 session/DAG 边界内再次检查：自身为 `Green`、递归所有 child/grandchild/后代为 `Green`、没有未完成 claim、未决 approval、未知工具结果或有效 lease 缺失，才追加一次 `dag_node_closed` 并调用 `core::Agent::try_close`/`settle_blueprint_worker`。
5. gate 为 false 时写 `dag_close_blocked`，节点保持 `open`/`WaitingChildren`，继续续租、消费邮箱和发状态；新工作先投递给仍活跃的 direct child，只有没有合适 child 时才写幂等 `spawn_request_id`，做预算/审批，再通过 `BackgroundRunner::try_submit` 或 `spawn_worker` 创建新 `node_id/generation`，回写 parent edge。派生失败、`QueueFull`、runner `Closed`、TTL 过期或 child `Cancelled/Panicked/Unknown` 都保持非绿，不能伪造 close。

当前 `can_close` 是只读检查，检查与 close 之间可并发新增 child 或重复 close；建议补 `try_close(id, expected_generation)`，在锁内原子复核、写 close 事件并拒绝旧 generation。当前 `spawn_worker` 的 `SPAWNED` registry 是进程内的，需用 durable `spawn_request_id`/lease 防止重启后重复派生。`DagMessage` 还应从单纯截断 `body` 升级为带 `kind`、`request_id`、`reply_to`、`ttl`、`attempt` 的受限 envelope。

### 对照 zenpi 四个既有落点

- `src/headless.rs`：在现有 `Command::Mailbox`、`run_headless`、`AsyncEventBuffer`、`write_events` 和 runtime 事件路由处增加 `dag_send`、`dag_receive/status`、`dag_heartbeat`、`dag_derive`、`dag_close` 薄命令；复用 bounded admission、request id、replay 和 reconnect。它只负责传输、背压、授权入口和可观测事件，不能越过 core/session 的 close gate；关键 `NodeGreen`、`NodeDerived`、`NodeClosed` 不得因慢 stdout 被普通进度丢弃。
- `src/core.rs`：`Agent`、`AgentPhase::{Idle,Running,Closed}`、`Turn::parent_id`、`AgentEvent`、`WorkerExecutionBinding`、`run_active_turn_cancelable`、`admit/renew/settle/cancel_blueprint_worker` 是单 worker 执行和政策边界。建议在这里或专门 coordinator 增加 `DagWorkerContext`、`DagNodeState`、`record_dag_event`、`children_green`、`evaluate_close_gate`、`derive_worker`；`Agent::try_close` 只在 gate 通过后改变为 `Closed`。
- `src/session.rs`：`SessionStore::append_event`/`append_turn` 是 durable journal，`tree_ancestry_turns`/`fork_at_tree_leaf` 可作会话历史；`SessionMailbox` 和 `LiveSessionRegistry::{register,heartbeat,claim_next,claim_message,finish_claim,finish_claim_with_reply}` 负责可恢复投递和 owner epoch。增加 `dag_node_created/edge/heartbeat/message/close_blocked/derived/closed` 事件，恢复时重建 parent/child projection、lease 和未完成派生意图。
- `src/runtime.rs`：每个 DAG node 绑定一个 `JobId`；复用 `BackgroundRunner` 的 bounded command/event/pending 队列、FIFO、`CancellationToken`、`RuntimeEvent` 和 shutdown grace。父级取消应传播到 child，但取消只改变协作式状态；未 join 的外部进程或 blocking Git 操作不能被描述为已回滚。heartbeat tick、邮箱 poll、派生 admission 和 close 复核必须受队列上限约束。

配套边界：`src/protocol.rs` 承担版本化 schema、ID/正文/TTL/epoch 校验；`src/tool_runtime.rs` 继续使用有界 `execute_tool_batch`，取消/未知副作用不可自动重试；`src/approval.rs` 继续要求 side effect 经过 `ApprovalCoordinator` 和持久化后的 approval；`src/providers/**` 只产生 provider/stream/transport 事件，不创建 DAG 边、不授予 sibling/child 权限、不决定 close。

## 未决问题

1. Codex 笔记只证明直接 parent、thread depth 和单 agent final；zenpi 仍需冻结 `Green` 的严格定义，以及 `Failed`、`Expired`、`Abandoned`、`Interrupted`、`UnknownOutcome` 是否能被替代 worker 重算为 green。
2. `DagStore`、`SessionMailbox`、`SessionStore` journal 和 `LiveSessionRegistry` 的权威边界尚未统一：DAG 边、邮箱 claim、owner lease、事件 projection 是否需要同一次事务，跨进程恢复如何避免半完成状态，仍需协议和集成测试确定。
3. parent 在 close 检查后新增 child 时，旧 close 结果如何失效；一个 session 是否允许多个 DAG worker；以及 direct sibling 是否允许直接互发工作而不经 parent，尚未固定。
4. heartbeat 周期、lease TTL、邮箱容量、派生预算、最大 depth/child 数、背压优先级和 stale worker 接管策略需要在 `protocol.rs` schema 中确定；过期只能标为 stale/unknown，不能推断为 green。
5. `BackgroundRunner`、`spawn_blocking`、shell/provider/tool 的取消均可能留下外部副作用；需要定义 `UnknownOutcome` 的人工确认、operation id、重试与审计规则，不能把 cancellation 当 rollback。
6. role/profile/provider 的 immutable snapshot、approval policy 是否沿 parent 继承、外部配置 import 的非事务性，以及名称池是否允许复用/必须全局唯一，需要由 zenpi 的 session/DAG 元数据和恢复测试给出可重放答案。

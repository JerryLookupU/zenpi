# AC-029 — codex/codex-rs/core/src/tasks/mod.rs

- source_id/item_id：`AC-029`
- source_path：`codex/codex-rs/core/src/tasks/mod.rs`
- source_hash：`e85e5bd59fa9d50fbb365a362d59a29223baeb9b5ffa119817d2cfa38a08a432`
- source_bytes：`17790`
- source_lines：`465`
- coverage：已按原文件顺序读取完整字节范围 `0-17789`、行范围 `L1-L465`，包含注释、导入、模块声明、类型、trait、实现、测试模块声明及所有导出符号。

## 完整行为复盘

文件先声明六个私有任务子模块 `compact`、`ghost_snapshot`、`regular`、`review`、`undo`、`user_shell`（L1-L6），导入 `Arc`、`Duration`、`Instant`、Tokio `select!`/`Notify`、`CancellationToken`、`AbortOnDropHandle`、tracing，以及会话、协议、指标和用户输入类型（L8-L48）。这些导入表明任务由 Tokio 后台执行、通过取消令牌协作终止，并在会话层统一收尾。

`pub(crate) use` 在 L50-L58 重新导出六类任务/用户 shell 执行入口：`CompactTask`、`GhostSnapshotTask`、`RegularTask`、`ReviewTask`、`UndoTask`、`UserShellCommandMode`、`UserShellCommandTask`、`execute_user_shell_command`。它们是本文件之外的具体工作流，统一通过 `SessionTask` 进入生命周期管理。常量 `GRACEFULL_INTERRUPTION_TIMEOUT_MS` 固定为 `100` 毫秒；`TURN_ABORTED_INTERRUPTED_GUIDANCE` 是中断后写入模型历史的固定说明（L60-L61）。

`emit_turn_network_proxy_metric`（L63-L78）输入 `&SessionTelemetry`、布尔值 `network_proxy_active` 和一个临时内存标签元组 `(&str,&str)`，把布尔值转换成字符串 `"true"/"false"`，向 `TURN_NETWORK_PROXY_METRIC` 计数器增加 1，并带上 `active` 与临时标签。函数无返回值；指标写入失败不会由该函数报告，调用方承担 telemetry 实现的行为。

`SessionTaskContext`（L80-L84）是 `Arc<Session>` 的窄包装，派生 `Clone`，避免任务直接依赖完整会话接口。`new`（L86-L89）接收 `Arc<Session>` 并保存；`clone_session`（L91-L93）返回新的 `Arc` 强引用；`auth_manager`（L95-L97）和 `models_manager`（L99-L101）分别从 `session.services` 克隆 `Arc<AuthManager>` 与 `Arc<ModelsManager>`。这些方法没有错误分支，生命周期由引用计数保证；包装层只暴露任务需要的服务，降低任务与会话内部状态的耦合。

`SessionTask` trait（L104-L145）要求 `Send + Sync + 'static`，因此任务可安全放进后台 Tokio 任务并跨线程共享。`kind`（L114-L116）返回 `TaskKind`，用于 telemetry/UI；`span_name`（L118-L119）返回静态 tracing 名称；`run`（L121-L135）消费 `Arc<Self>`，接收 `Arc<SessionTaskContext>`、`Arc<TurnContext>`、`Vec<UserInput>` 和 `CancellationToken`，异步运行至完成或取消，返回 `Option<String>`，其中 `Some` 会成为最终 agent 消息；实现应持续观察令牌并尽快停止。`abort`（L137-L145）接收同样的会话/turn 上下文，默认空操作，供实现释放资源或发送清理通知。trait 本身没有强制超时、重试或持久化，具体实现必须配合调用方语义。

`Session::spawn_task`（L147-L227）是统一提交入口。输入为 `Arc<Self>`、`Arc<TurnContext>`、`Vec<UserInput>`、具体 `T: SessionTask`，返回 `()`。它先顺序执行 `abort_all_tasks(Replaced)`、`clear_connector_selection`、`sync_mcp_request_headers_for_turn`（L154-L157），因此新任务会替换旧活动任务并刷新本轮连接器/MCP 头。随后把任务装入 `Arc<dyn SessionTask>`，读取 `kind`/`span_name`，记录开始时间和起始 token 用量（L159-L168），创建父 `CancellationToken` 与 `Notify` 完成通知（L169-L170），并启动端到端计时器（L172-L175）。

后台闭包（L177-L213）创建受限 `SessionTaskContext`、克隆 `TurnContext` 与任务，使用子取消令牌；建立包含会话、turn、model 字段的 `info_span!`。Tokio 任务调用 `run`（L194-L202），完成后先 `flush_rollout`（L203-L204）；仅当任务未被取消时调用 `on_task_finished` 发出统一完成事件（L205-L209），随后 `notify_waiters`（L210-L211）。这意味着被取消任务不会走正常 `TurnComplete` 路径。句柄被 `AbortOnDropHandle` 包装，和任务种类、令牌、上下文、计时器一起写入 `RunningTask`（L216-L224），最后由 `register_new_active_task` 登记（L225-L227）。并发边界是 `active_turn` 的异步互斥状态；任务内部并发由具体实现决定。

`Session::abort_all_tasks`（L229-L239）先 `take_active_turn` 原子地取走当前活动 turn；逐个 `drain_tasks` 并调用 `handle_task_abort`，之后才 `clear_pending`。注释明确说明顺序：先让任务观察取消，再丢弃待处理 approval，避免 approval 等待先产生模型可见拒绝而遮蔽 `TurnAborted`。无活动 turn 时仍清理 MCP 请求头。

`Session::on_task_finished`（L241-L369）负责统一终结。先取消 git enrichment（L246-L249），锁定 `active_turn`，按 `turn_context.sub_id` 删除任务并提取 pending input、工具调用数、起始 token（L250-L263）；若该任务是活动 turn 的最后任务则清空活动状态并清理 MCP 头（L264-L270）。每个 pending input 经 `inspect_pending_input` 分为 `Accepted`（记录输入）或 `Blocked`（记录额外上下文）（L271-L284）。若拿到起始 token，则读取内存 feature `MemoryTool` 标签，并异步查询 managed network proxy；查询错误只记录 warning 并按 inactive 处理（L285-L314）。随后记录工具调用 histogram，并计算总 token 与本轮增量；每个差值用 `.max(0)` 防止计数回退，记录 total/input/cached/output/reasoning 五类 histogram（L315-L363）。最后构造 `EventMsg::TurnComplete { turn_id, last_agent_message }` 并 `send_event`（L364-L369）。它只在任务仍被视为活动任务时提取状态，重复完成不会重复清空或重复结算。

`register_new_active_task`（L371-L383）锁住 `active_turn`，创建默认 `ActiveTurn`，把本轮起始 token 写入共享 `turn_state`，添加一个 `RunningTask` 并替换活动值；它假定调用点已经完成前置取消。`take_active_turn`（L384-L387）在锁内 `take`，把所有权移出以便取消流程不长期持锁。

`close_unified_exec_processes`（L389-L394）调用 `unified_exec_manager.terminate_all_processes`，是统一的外部进程副作用清理。`cleanup_after_interrupt`（L396-L402）仅在 JS REPL manager 已初始化时按 turn id 调用 `interrupt_turn_exec`；错误只 warning，不向调用者返回。

`handle_task_abort`（L404-L460）是取消、宽限和中断事件的核心。它复制 `sub_id`，若令牌已取消则幂等返回（L405-L408）；否则记录 trace、调用 `cancel`，取消 git enrichment，取出任务对象（L410-L416）。通过 `select!` 等待 `done.notified()` 或 100ms 睡眠（L417-L423）；宽限期过后 warning，然后无条件 `task.handle.abort()`（L425）。接着创建新的 `SessionTaskContext`，调用任务自定义 `abort`（L427-L430），所以清理钩子发生在强制 abort 之后。若原因是 `Interrupted`，还会中断 JS REPL，构造带 `TURN_ABORTED_OPEN_TAG` 和固定指导文本的用户 `ResponseItem::Message`，同时写入内存历史和 rollout（L432-L449），再 `flush_rollout` 确保持久化先于 `TurnAborted` 事件（L450-L453）。最后发送带 turn id 和 reason 的 `EventMsg::TurnAborted`（L455-L459）。其他 abort reason 不写中断 marker，但仍发终止事件。

文件末尾 `#[cfg(test)] #[path = "mod_tests.rs"] mod tests`（L463-L465）把测试放在同目录。该主文件没有其他公开函数；所有 `pub(crate)` 任务重导出和 `Session` 方法共同构成任务调度 API。

## 状态、取消、恢复与副作用

状态由 `Session.active_turn`、`ActiveTurn`/`RunningTask`、`CancellationToken` 和 `Notify` 共同表示：提交前先替换旧任务，运行中令牌可被任意持有者观察，完成通知只用于取消方等待。取消是协作优先、强制兜底：先 `cancel`，最多等 100ms，再 abort Tokio handle；`SessionTask::abort` 提供补偿清理。没有本地 retry/backoff，也没有任务级超时配置；100ms 只是中断宽限，不是 `run` 的总时限。

恢复语义只覆盖 rollout/历史与外部资源：正常完成先 `flush_rollout`，中断写入 marker 后再次 flush；pending input、token usage、工具调用数在完成时结算。MCP 头、git enrichment、connector selection、统一 exec 进程、JS REPL 都是外部或会话副作用，分别在替换、完成或中断路径清理。代码没有把 `RunningTask` 本身持久化，也没有自动从崩溃进程恢复；重新提交必须由上层重新建立 task/context。网络 proxy 状态读取失败按 false 计量，不阻断完成。

并发语义包括：`Arc` 共享任务/会话；`active_turn`、turn state 由异步锁保护；Tokio 后台任务持有子取消令牌；`Notify` 可能唤醒多个等待者但取消方只等待自己的 task；完成闭包通过 `remove_task(sub_id)` 防止旧任务误终结新 turn。`abort_all_tasks` 先移出活动状态再逐项处理，减少锁竞争。

## 源内测试与行为判据

主文件仅在 L463-L465 声明同目录 `mod_tests.rs`。该测试文件实际测试 `emit_turn_network_proxy_metric`：`emit_turn_network_proxy_metric_records_active_turn` 与 `...inactive_turn` 均要求计数值为 `1`，且属性分别准确包含 `active=true/false` 和 `tmp_mem_enabled=true/false`（`mod_tests.rs` L74-L114）。因此可独立验证的判据是：对两种布尔输入分别 snapshot metrics，找到 `TURN_NETWORK_PROXY_METRIC`，断言单个 counter point、值为 1、属性无缺失且字符串值正确。

任务生命周期可独立验证的判据：提交新任务前旧任务收到 `Replaced` 取消；`run` 返回后 rollout 已 flush 且未取消时恰好发送一个 `TurnComplete`；取消时先观察令牌、最多等待 100ms、最终发送 `TurnAborted`；`Interrupted` 必须在 abort 事件前能从 rollout 读取 `<turn_aborted>` marker。重复调用 abort 不应重复发送清理/终止事件。指标验证还应检查 token 差值不会为负、network proxy 查询错误不会使完成失败。

## zenpi Rust 映射

zenpi 已有的通信原语足以承载 DAG，但需把本文件的“单活动 turn”语义提升为“每个 DAG 节点一个可取消 worker”。`src/protocol.rs` 的 `MailboxRequest` 已提供 `Send/Receive/Acknowledge/Claim/Complete`（L87-L113），并有 TTL/文本上限（L20-L37）；`Command::Mailbox`、`Command::Tree`（L251-L255）是协议入口。建议新增或扩展 `DagMessage`/`DagEvent`，字段至少为 `message_id`、`sender_session_id`、`recipient_session_id`、`node_id`、`parent_id`、`grandparent_id`、`kind`、`payload`、`ttl_ms`、`causal_sequence`。消息路由规则为：parent、grandparent、直接 sibling、直接 child 都只能通过节点登记的 session id 定向发送；禁止任意“广播所有祖先/后代”，避免 DAG 变成无界图。事件适合进程内实时通知，邮箱适合跨进程/重启后的可靠投递，协议负责 JSONL 校验与相关 id，状态协议负责 close/keepalive/spawn。

- `src/session.rs` 是会话父子关系与持久化落点。现有 `SessionStore`/`SessionSummary` 和 session tree API 可保存 `node_id -> parent_id`；应新增 `DagNodeRecord { node_id, session_id, parent_id, grandparent_id, status, generation, required_children, green_children, lease_expires_at_ms }`，并以 append-only journal 记录 `node_started`、`heartbeat`、`node_green`、`node_close_requested`、`node_kept_alive`、`worker_spawned`。已有 `MailboxMessage` 的 sender/recipient、digest、sequence、TTL、状态机（`src/session.rs` L3171-L3255）可直接复用；`LiveSessionRegistry` 的 owner epoch、heartbeat、TTL、claim/finish 机制（L3274-L3488）正好提供最小保活与幂等投递。现有 `claim_next` 明确“不启动 worker”，因此应由 DAG supervisor 在 claim 成功后显式派生 worker。

- `src/runtime.rs` 是 worker 执行落点。`BackgroundRunner`、`JobId`、`CancellationToken`、有界 command/event channel 及 `RuntimeEvent::{Accepted,Queued,Started,CancelRequested,Completed,Closed}`（约 L40-L195、L236-L369）可包装为 `DagWorkerRuntime`。最小机制是 `spawn_node(node_id, work)`、`cancel_node(node_id)`、`emit_node_event`、`join_node`：每个节点保留自己的 cancellation token 和 bounded event mailbox；parent 只收到聚合状态，不直接持有 child 线程。保活由周期 heartbeat job 更新 `LiveSessionRegistry`；lease 过期转为 `Blocked/Offline` 并禁止 close。派生新 worker 必须在 journal 先写 `worker_spawned`/lease，再调用 `BackgroundRunner::spawn`，以便崩溃后通过幂等 operation id 重放。

- `src/core.rs` 负责业务状态机与 close gate。现有 `Turn` 的 `parent_id`（约 L140-L175）可作为对话因果关系，但不能单独表达 DAG 节点；应新增 `DagNodeState` 和 `DagSupervisor`，把 `WorkerExecutionBinding` 的 `goal_id/item_id/lease_id/policy_digest/expires_at_ms`（L37-L81）绑定到节点任务。`DagSupervisor::can_close(node_id)` 必须同时检查自身 `green`、全部直接 child 与递归 descendant 的 green、lease 未过期、无 pending/claimed mailbox、无未知工具结果；任何条件不满足就写 `node_kept_alive` 并保留 worker。`derive_worker_for_new_work` 生成新 `node_id`/generation，继承 parent/grandparent 元数据并通过 mailbox 发送工作，不得复用已完成节点的 operation id。

- `src/headless.rs` 是外部 wire/事件编排落点。其 per-request `AsyncEventBuffer`、bounded provider/agent event mailbox、replay 与 `event_dropped` 机制（约 L810-L1140、L4042-L4174）可以承载 `DagEvent`，并保证慢 stdout 不阻塞 worker。把 `AsyncTurn::Tree`（L3912-L3923）接到 `DagSupervisor` 的 inspect/close/keepalive/spawn 动作；把现有 `mailbox_slash_view`（L2065-L2119）扩展为 parent/grandparent/sibling/child 的定向快捷命令。所有终端响应要带 request id、session id、node id，事件序列进入 replay journal；丢弃事件只允许丢进度，不能丢 `NodeGreen`、`CloseDenied`、`WorkerSpawned` 等 admission/终态标记。

- `src/tool_runtime.rs` 已明确工具批处理的取消、并发上限、顺序执行和“无可信结果不得自动重试”语义（L1-L7、L157-L249）。DAG worker 的工具调用应把 node lease 和 `WorkerExecutionBinding` 注入 `ToolContext`，在 `prepare` 阶段先持久化 dispatch intent；取消或 `Io/Internal/CommandTimeout` 后把节点标为 `Blocked/UnknownOutcome`，不得直接判绿。只有 core 的显式 recovery 决策才能重试，重试需新 operation id。

- `src/approval.rs` 的 `ApprovalCoordinator` 是 worker 与 host 的 bounded rendezvous：请求校验、可见队列、取消轮询、accepted 审计（约 L126-L245）。DAG worker 的 approval request 应带 node/session/parent 信息及 lease；`WorkerAllowAfterPreflight` 只能在 `WorkerExecutionBinding` 与 policy digest 匹配且 lease 活跃时自动放行，否则沿 headless approval event 返回 host。approval 等待必须响应 node cancellation，防止 close/abort 时遗留挂起请求。

- `src/runtime.rs` 与 `src/headless.rs` 的事件通道是实时事件/邮箱，`src/session.rs` 的 mailbox/journal 是可靠协议；`src/protocol.rs` 负责版本、长度、TTL、id 校验；`src/providers/**`（`providers/mod.rs` 的 `Protocol`、`Dialect`、`ProviderDefinition`、`RouteRule`，L1-L161）继续只负责 provider 路由，不能直接决定 DAG close 或派生。provider 事件应标注 node/turn correlation 后进入 headless buffer，provider 断流只使节点进入可恢复/未知状态。

最小可执行方案是：`DagNodeState`（core）+ `DagSupervisor`（core）+ `SessionStore` journal/`LiveSessionRegistry` heartbeat（session）+ `MailboxRequest` 扩展（protocol）+ 每节点 `BackgroundRunner` handle（runtime）+ headless tree/mailbox dispatch（headless）。验证顺序为：发送到四类邻接节点均能在目标 mailbox 生成一条有 TTL 的消息；claim/complete 只接受当前 owner epoch；子节点未 green 时 `can_close` 返回 false 且父节点保持 active；全部递归后代 green 才发 `NodeClosed`；新工作在 close gate false 时生成新 worker，并在重启后通过 journal/replay 恢复其 lease 与幂等状态。

差异清单：Codex 源文件只有一个 `ActiveTurn`，zenpi 需要多节点 DAG registry；Codex 的完成事件是单 turn，zenpi 需要递归 green 聚合；Codex 的取消依赖 Tokio `CancellationToken` 与 100ms grace，zenpi 还需 lease/heartbeat 失效；Codex 没有 parent/grandparent/sibling/child 地址协议，zenpi 必须把关系写进消息并校验邻接；Codex 不持久化 `RunningTask`，zenpi 必须持久化 worker spawn/close gate/keepalive 记录；provider 路由只能产生工作流事件，不能绕过 approval、工具未知结果和 DAG close gate。

## 未决问题

无法从本源文件确认具体六类任务实现如何处理中断令牌、是否自行派生子任务，也无法确认 `TurnContext`、`ActiveTurn`、`RunningTask` 内部字段的完整约束；这些点需分别阅读对应子模块和 `state` 定义后验证。源文件也未定义跨 session 的 parent/grandparent/sibling/child 通信协议、递归 DAG close 判定或 worker 派生策略；上述 zenpi 映射是基于现有协议、邮箱、session tree 与 runtime 接口提出的可执行落点。

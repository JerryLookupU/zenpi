# AC-018 — codex/codex-rs/core/src/agent/control.rs

| 元信息 | 值 |
|---|---|
| source_id/item_id | `AC-018` |
| source_path | `codex/codex-rs/core/src/agent/control.rs` |
| source_hash | `1bbaad2c63db6b05cc7048947501b88ab00f552531d045efa9a3e463782264be` |
| source_bytes | `21127` |
| source_lines | `538` |
| coverage | 已按源文件顺序读取完整字节范围 `0-21126`、行范围 `L1-L538`（含注释、导入、常量、私有函数、类型、`impl`、测试模块声明）。 |

## 完整行为复盘

### 文件级常量、选项和命名

- `AGENT_NAMES`（`L30`）通过 `include_str!("agent_names.txt")`提供默认昵称池；`FORKED_SPAWN_AGENT_OUTPUT_MESSAGE`（`L31`）是 fork 子线程收到的合成成功输出，告诉新 agent 历史已 fork、下一条用户消息是新任务。
- `SpawnAgentOptions`（`L33-L36`）派生 `Clone/Debug/Default`，唯一字段 `fork_parent_spawn_call_id: Option<String>`；为 `Some` 时要求 fork 一个 `ThreadSpawn` 父线程上下文。
- `default_agent_nickname_list`（`L38-L44`）逐行 trim、丢弃空行，返回静态字符串切片；`agent_nickname_candidates`（`L46-L61`）把缺省 role 归一为 `DEFAULT_ROLE_NAME`，优先使用 `resolve_role_config(...).nickname_candidates`，否则复制默认池。角色存在但候选字段为 `None` 时也回退默认池。

### `AgentControl` 与构造

- `AgentControl`（`L63-L76`）是多 agent 控制面句柄，由每个 session 持有；同一 user session 的子 agent 共享它，因此 `Arc<Guards>` 共享并发配额和昵称占用。`manager: Weak<ThreadManagerState>` 有意避免 `ThreadManagerState -> CodexThread -> Session -> SessionServices -> ThreadManagerState` 强引用环及影子持久化。该类型 `Clone + Default`，默认句柄的 `Weak` 无法升级。
- `new`（`L78-L85`）只接收 `Weak<ThreadManagerState>`，构造时新建默认 `Guards`；正常运行应由上层把同一 `Arc<Guards>` 通过 clone 传播。
- `upgrade`（`L490-L494`）是所有需要 manager 的入口的共同边界：`Weak::upgrade` 失败返回 `CodexErr::UnsupportedOperation("thread manager dropped")`，不会尝试重建状态。

### 创建、fork 与恢复

- `spawn_agent`（`L87-L96`）是薄封装，使用 `SpawnAgentOptions::default()` 调用下层；输入为 `Config`、`Vec<UserInput>`、可选 `SessionSource`，成功返回新 `ThreadId`。
- `spawn_agent_with_options`（`L98-L225`）按以下顺序执行：
  1. `upgrade` manager；调用 `self.state.reserve_spawn_slot(config.agent_max_threads)`（`L105-L107`）原子预留线程槽，达到上限返回 `AgentLimitReached`。随后从 `ThreadSpawn` 父线程异步继承 shell snapshot 和可复用的 exec-policy（`L107-L112`）。
  2. 对 `SessionSource::SubAgent(SubAgentSource::ThreadSpawn{...})` 依据 role 取候选昵称，向 reservation 预留唯一 nickname，再重建 source，保留 `parent_thread_id/depth/agent_role`（`L113-L131`）。非线程派生 source 原样传递；保存 clone 供完成 watcher（`L133`）。候选耗尽会返回 `UnsupportedOperation`，尚未 commit 的 reservation 由 Drop 释放槽位。
  3. 有 `session_source` 且带 `fork_parent_spawn_call_id` 时，强制 source 必须是 `ThreadSpawn`，否则立即 `Fatal`（`L136-L146`）。找到 parent thread 后先 `ensure_rollout_materialized`、`flush_rollout`（`L148-L158`），再从内存 rollout path 或 `find_thread_path_by_id_str` 找 JSONL；两者都无则 `Fatal("parent thread rollout unavailable for fork")`（`L159-L171`）。读取完整 `RolloutRecorder` 历史，追加带原 `call_id` 的 `ResponseItem::FunctionCallOutput`，`success = Some(true)`，再以 `InitialHistory::Forked` 调 `state.fork_thread_with_source`，并把继承的 shell/policy、同一个 `AgentControl` 传入（`L172-L197`）。这使 fork 子线程看见父历史且原 spawn 调用闭合。
  4. 有 `session_source` 但无 fork id 时调用 `spawn_new_thread_with_source`；没有 source 时调用 `spawn_new_thread`（`L198-L213`）。两条路径都关闭 extended history 持久化；source 路径显式传入继承上下文。
  5. 新线程成功后 `reservation.commit(new_thread.thread_id)` 注册活动线程和 nickname（`L214`），`notify_thread_created` 让客户端可订阅/排空（`L216-L220`），然后向子线程发送 `Op::UserInput{items, final_output_json_schema: None}`（`L221`），最后为 ThreadSpawn 子线程启动完成 watcher（`L222`），返回 id（`L224`）。注意 commit 发生在 `send_input` 之前；发送阶段若 agent 已死会主动 remove/release，但其他发送错误不会回滚已经 commit 的线程。
- `resume_agent_from_rollout`（`L227-L304`）恢复已有 rollout：先升级 manager、预留槽位（`L234-L235`）。若 source 是 `ThreadSpawn`，通过可用 SQLite state DB 查询 thread metadata，恢复原 `agent_nickname/agent_role`；数据库缺失、无记录或查询错误均降级为 `None`（`L236-L252`），如有昵称则使用 `reserve_agent_nickname_with_preference` 保持稳定身份（`L253-L271`）。随后继承父 shell/policy，查找 rollout path；不存在返回 `CodexErr::ThreadNotFound`（`L276-L285`）。`resume_thread_from_rollout_with_source` 成功后 commit、重新 `notify_thread_created`（恢复线程需要重新注册监听）、启动 watcher，返回原 id（`L287-L303`）。任何早期错误让 reservation Drop 释放计数。

### 输入、生命周期与查询

- `send_input`（`L306-L327`）把 rich `UserInput` 包装为 `Op::UserInput`，schema 固定 `None`，异步交给 `ThreadManagerState::send_op`，返回 submission id。若错误精确匹配 `InternalAgentDied`，删除线程并 `release_spawned_thread`；其它错误原样传播。
- `interrupt_agent`（`L329-L333`）只发送 `Op::Interrupt`，不移除线程、不释放槽位，故是可继续工作的软取消。
- `shutdown_agent`（`L335-L342`）发送 `Op::Shutdown {}` 后无论发送结果如何都尝试 remove，并释放 spawned-thread guard，最后返回发送结果；关闭路径是显式回收槽位。
- `get_status`（`L344-L354`）manager 或 thread 查找失败均返回 `AgentStatus::NotFound`，成功则异步读取最后状态。`get_agent_nickname_and_role`（`L356-L371`）和 `get_agent_config_snapshot`（`L373-L384`）同样把 manager/thread 不可用映射为 `None`，否则分别从 `config_snapshot().session_source` 提取 nickname/role 或返回完整 snapshot。
- `subscribe_status`（`L386-L394`）manager/thread 不可用返回 `CodexResult` 错误；成功返回 `watch::Receiver<AgentStatus>`，订阅者先看到当前值，随后通过 `changed()` 得到变化。`get_total_token_usage`（`L396-L404`）不可用返回 `None`，否则读取累计 `TokenUsage`。
- `format_environment_context_subagents`（`L406-L438`）枚举 manager 当前所有 thread，仅保留 `SessionSource::SubAgent::ThreadSpawn` 且 `agent_parent_thread_id == parent_thread_id` 的直接子节点，格式化为 `format_subagent_context_line(thread_id,nickname)`，排序后用换行连接；manager 不可升级返回空串。它不递归 grandchild，也不读取完成状态。

### 完成通知、继承与并发

- `maybe_start_completion_watcher`（`L440-L488`）只接受带 `parent_thread_id` 的 `ThreadSpawn`；其它 source 立即返回。它 clone control 后 `tokio::spawn` 一个 detached task：订阅 child `watch` 状态，初始值不是 final 才循环 `changed()`；receiver 关闭时回退 `get_status`（`L455-L470`）。订阅失败也回退查询。非 `is_final` 状态直接结束而不通知；final 状态再升级 manager、查 parent，parent 不存在静默返回，存在则向 parent `inject_user_message_without_turn(format_subagent_notification_message(child_id,status))`（`L471-L487`）。因此通信是一跳 parent 注入的用户消息，watcher 自身不阻塞 spawn 调用。
- `inherited_shell_snapshot_for_source`（`L496-L510`）只有 `ThreadSpawn` 才取 parent thread 的 `user_shell().shell_snapshot()`；找不到 parent或其它 source 返回 `None`。`inherited_exec_policy_for_source`（`L512-L534`）先取 parent config，只有 `child_uses_parent_exec_policy(parent_config, child_config)` 为真时才 clone parent `SessionServices.exec_policy`，否则隔离为 `None`。
- 文件末尾 `#[cfg(test)] mod tests`（`L536-L538`）把行为测试放在同目录 `control_tests.rs`。

## 状态、取消、恢复与副作用

- **状态机与取消**：本文件不定义 worker 状态枚举，而通过 `AgentStatus`、`watch::Receiver` 和 `is_final` 消费状态。`interrupt_agent` 是向指定线程投递 `Op::Interrupt` 的协作式中断；`shutdown_agent` 是终止并清理注册的强生命周期操作。没有 deadline、超时参数或内部 retry；watcher 的 `changed()` 只在 channel 关闭时回退一次 `get_status`，不会循环重试父线程查找或通知。
- **并发语义**：`AgentControl` 可 clone，`Guards` 以 `Arc` 跨 session service 共享；槽位预留/commit/release 在并发 spawn 下提供上限保护。manager 用 `Weak`，manager drop 后所有控制入口快速失败或返回 NotFound/None/空字符串。完成 watcher 是 detached Tokio task，调用者没有 join/abort 句柄；child 状态和 parent 注入在不同任务中发生，parent 被移除时通知丢弃且不报错。
- **恢复与持久化**：fork 前显式 `ensure_rollout_materialized` 与 `flush_rollout`，保证异步 rollout writer 的实时内容已落到 JSONL，再读取历史；resume 从 `codex_home` 的 rollout 路径恢复，并可从 SQLite 重新水合 nickname/role。源文件自身不写 SQLite，只通过 `state_db::get_state_db` 读元数据；新线程的注册、监听和输入发送由 manager/session 完成。完成通知通过 `inject_user_message_without_turn` 进入 parent 会话历史，属于可审计会话副作用。
- **资源与错误副作用**：spawn/fork/resume 会创建或恢复线程、占用全局 agent slot、注册 nickname、触发 `notify_thread_created`；`send_input` 会提交 provider turn。`InternalAgentDied` 会 remove/release，`shutdown_agent` 也总是 remove/release；reservation 在 manager、rollout、昵称或线程创建错误时通过 Drop 回滚槽位。shell snapshot 与 exec policy 的继承可能扩大子 worker 可见的执行上下文，但 exec policy 只有显式兼容条件才共享。
- **未实现的语义**：这里没有“所有后代完成才 close”、DAG sibling/child 多播、保活心跳或自动派生策略；只通知直接 parent，且 `format_environment_context_subagents` 只列直接 child。zenpi 若需要 DAG 关闭门槛，必须在更高层增加聚合状态和派生决策。

## 源内测试与行为判据

同目录测试由 `#[path = "control_tests.rs"]` 引入（`control.rs:L536-L538`）。关键可核对判据如下：

- manager 被 drop 时，`send_input`、`spawn_agent`、`resume_agent_from_rollout` 都返回 `unsupported operation: thread manager dropped`，而 `get_status` 返回 `NotFound`；对应 `control_tests.rs:L146-L170`、`L220-L246`。
- 缺少 thread 时，发送和订阅返回 `CodexErr::ThreadNotFound`，状态查询为 `NotFound`；新 thread 初始状态为 `PendingInit`，shutdown 后 `watch` 变为 `Shutdown`（`L248-L311`）。成功 `send_input` 必须捕获带原输入、`final_output_json_schema: None` 的 `Op::UserInput`（`L313-L346`）；spawn 必须注册新 thread 并发送 prompt（`L348-L377`）。
- fork 测试验证 parent 历史被复制、child 收到新任务（`L379-L463`）；验证合成 `FunctionCallOutput` 的原 `call_id`、文本等于 `FORKED_SPAWN_AGENT_OUTPUT_MESSAGE` 且 `success == Some(true)`（`L465-L539`）；未预先 flush 时仍必须先物化 rollout，且 function call 出现在 synthetic output 之前（`L541-L616`）。
- max thread 限制、shutdown 释放槽位、clone 共享 guard、resume 受限及失败后释放槽位分别由 `L618-L806` 验证，判据是 `CodexErr::AgentLimitReached`、成功复用 slot、失败 resume 不泄漏计数。
- child 完成后 parent 历史必须出现 subagent 通知；child id 不存在时 watcher 仍通知 `status:not_found`（`L808-L877`）。ThreadSpawn 会生成 nickname、按 role 使用候选（默认/`Atlas`）见 `L879-L962`；SQLite 开启时 resume 必须恢复原 nickname、role、parent、depth（`L964-L1095`）。
- 可独立验证方法：构造 `AgentControlHarness`，用 `ThreadManager::with_models_provider_and_home_for_tests` 启动 parent；断言 `captured_ops`、`clone_history`、`subscribe_status`、slot limit 和 parent history；再用无 manager 的 `AgentControl::default()` 覆盖弱引用失败分支。测试中已有 2 秒通知等待和 5 秒恢复等待，可作为异步验收上限。

## zenpi Rust 映射

### 可直接复用的通信原语

1. **消息/邮箱协议**：`src/protocol.rs` 已有 `MailboxRequest::{Send,Receive,Acknowledge,Claim,Complete}` 与 `MailboxOutcome::{Succeeded,Failed,Abandoned}`（`src/protocol.rs:L81-L113`），并已在 `StdioRequest.mailbox` 暴露（`L152-L209`）。复用它作为跨 session 的 durable DAG 消息：`recipient_session_id` 映射 node/session id，`message_id` 做幂等键，`ttl_ms` 做保活消息过期；发送方身份仍由服务端 session owner 推断，不能信任客户端字段。`validate_mailbox` 已限制非空、NUL、`MAX_MAILBOX_TEXT_BYTES`、TTL 和分页边界（`L544-L599`）。建议新增 `DAGMessage` 的 `serde_json` payload（`kind`, `run_id`, `node_id`, `parent_id`, `ancestor_id`, `relation`, `generation`, `state`, `required_descendant_epoch`, `work`），而不改动 mailbox 外层。
2. **事件流**：`src/core.rs` 的 `AgentEvent::{TurnAccepted,AssistantMessage,ToolCall,ToolResult,Error,Warning,...}`（`src/core.rs:L281-L325`）可作为 worker/node 局部事件；在其上新增 `DagEvent::{NodeStarted,NodeGreen,NodeFailed,NodeKeepAlive,NodeDerived,NodeClosed}`，通过 headless 的 bounded event mailbox 转发。事件必须带 `run_id/node_id/parent_id`，使 parent、grandparent 和 sibling 可以过滤，而不是依赖字符串通知。
3. **会话输入/队列**：`Agent::input_port` 与 `input_queue_request`（`src/core.rs:L674-L692`）提供“忙时排队、journal append 后才返回 receipt”的入口；`src/protocol.rs` 的 `InputQueueAction` 及严格 `InputQueueRequest::validate`（`L1035-L1158`）可承载 node 的新工作、steer 和 cancel。对 child 的派生请求应在 queue receipt 后才算 admitted。
4. **审批/工具事件**：`src/approval.rs` 的 `ApprovalCoordinator` 是 worker-host rendezvous，等待 host 决策但支持 cancelled（`src/approval.rs:L126-L180`）；`src/tool_runtime.rs::execute_tool_batch` 已有 bounded batch、cooperative cancellation、结果按输入顺序归并（`src/tool_runtime.rs:L157-L176`）。DAG 状态变绿前只能把 `ToolResult.success == true` 且审批/持久化完成视为该 node 的工作完成。

### 会话父子关系与四类邻接

- `control.rs` 的 `SessionSource::SubAgent(SubAgentSource::ThreadSpawn{parent_thread_id, depth,...})` 是最接近的关系模型：`AgentControl` 负责创建、继承、resume、直接 parent 完成通知（`control.rs:L113-L131`, `L440-L488`）。zenpi 可在 `src/core.rs::Turn` 的 `parent_id`（`src/core.rs:L140-L175`）之外新增持久化 `NodeLink { run_id, node_id, parent_id, depth, siblings: BTreeSet<String> }`；`parent_id` 表示 session/turn 的直接父，`depth` 用于祖先路由。
- **parent/grandparent**：每个节点保存 `parent_id` 和 `ancestor_chain`（至少 `grandparent_id`，或从 `SessionTree` 反查），向上发送 `MailboxRequest::Send`，消息 `relation=parent|grandparent`。不要让 child 直接持有 `Arc` parent；使用 session id + mailbox，避免类似 `ThreadManagerState` 引用环。
- **直接 sibling**：由 parent 的 child 集合派生 sibling 列表；`format_environment_context_subagents` 只枚举直接 child 并排序（`control.rs:L406-L438`），可复用其过滤原则，但增加 `node_id != self` 后向 sibling mailbox 发送。grandchild 不应误算为 sibling。
- **直接 child**：child 在 spawn 时记录 `parent_id/depth`，并通过 `NodeStarted/NodeGreen/NodeFailed` 事件向 parent 发送；父节点维护 `BTreeMap<child_id, ChildState>`。child 完成 watcher 的“一跳 parent 注入”（`control.rs:L449-L487`）可作为最小实现，但 DAG 需要把注入从非结构化用户文本升级为带 schema 的 `DagEvent`。

### 保活、派生和关闭门槛（最小可执行机制）

建议在 `src/core.rs` 或新建 `src/dag.rs` 放置以下最小类型和函数：

```rust
struct DagNodeState {
    run_id: String,
    node_id: String,
    parent_id: Option<String>,
    child_ids: BTreeSet<String>,
    child_states: BTreeMap<String, ChildState>,
    own_state: NodeState, // Running | Green | Failed | KeepAlive | Closed
    lease_epoch: u64,
    last_heartbeat_ms: u64,
}

fn record_node_event(&mut self, event: DagEvent) -> Result<CloseDecision, DagError>;
fn can_close(&self) -> bool; // own_state == Green && all descendants_green()
fn keep_alive_or_derive(&mut self, work: WorkItem) -> Result<NodeId, DagError>;
```

`record_node_event` 要幂等检查 `(run_id,node_id,epoch,event_id)`，更新直接 child 状态并向上汇总；`descendants_green()` 递归查询本地 child projection 或 `SessionTree`，只有“自身 Green 且全部 child/grandchild 已 Green”才返回 `CloseDecision::Close`。任何 child/grandchild 为 Running、Failed、Unknown、过期 lease，或新 work 已入队，都返回 `KeepAlive`。`keep_alive_or_derive` 的最小流程是：原 node 原子递增 `lease_epoch`、写入 heartbeat/keep-alive 事件；若新 work 不属于当前 node，则通过 `Agent::input_port`/`InputQueueAction::Enqueue` 产生可追踪 receipt，再调用一个类似 `AgentControl::spawn_agent_with_options` 的 `derive_worker(config,parent_source,work)`，把当前 node 作为 parent、`depth + 1`，并将 child id 写入 parent projection；派生成功才发 `NodeDerived`，失败保持 parent alive 并暴露错误。

保活消息应使用 `MailboxRequest::Send`，`ttl_ms` 短于 lease（例如 lease 的 2/3），由后台 tick 重发 heartbeat；接收端 `Claim`/`Complete` 保证同一新 work 只有一个 worker 执行。`Complete` 只代表 mailbox work 的处理结果，不直接代表 node green；必须等工具结果、审批记录和所有 descendants 聚合后再发 `NodeGreen`。可用 `runtime::CancellationToken` 的 cancelled/completed 语义（`src/runtime.rs:L49-L99`）保护派生 worker：关闭门槛未满足时不要 cancel；确认 `can_close` 后再 cancel watcher/runner。

### 对照现有模块的落点

- `src/headless.rs`：这里已有有界 `AsyncEventBuffer`、优先级 admission 事件和 pending steer（`src/headless.rs:L786-L875`, `L933-L1105`）。在 `run_async_streams` 的事件路由增加 `DagEvent` 类型；保留 bounded/backpressure，`QueueFull` 时返回可重试的 typed response，不丢 `NodeGreen/NodeFailed/NodeDerived` 等 admission/terminal 事件。headless 只做传输和订阅，不决定 DAG 是否可 close。
- `src/core.rs`：`Agent` 是 owner 状态机，已有 `AgentPhase::{Idle,Running,Closed}`（`src/core.rs:L275-L279`）、`active_turn_id`、`worker_budget` 和 `live_owners`（`L402-L442`）。建议加 `DagNodeState`、`DagEvent`、`children_green()`、`submit_dag_work()`；在 turn/tool 完成和 `AgentPhase::Closed` 前调用 `can_close`，未满足则把 phase 保持 Running/Idle-with-lease，并通过 `derive_worker` 接新任务。
- `src/session.rs`：`SessionStore` 是 append-only JSONL 及内存 projection（`src/session.rs:L144-L161`）；`append_turn` 先落盘再改变 projection（`L861-L891`），`tree_snapshot`/`enable_tree`/`select_tree_leaf` 可取消并校验 writer 新鲜度（`L894-L1000`）。把 `dag_node`, `dag_edge`, `dag_heartbeat`, `dag_event`, `dag_close_decision` 作为事件写入；恢复时重建 parent/child/descendant 状态，未知或缺 heartbeat 的 child 默认 `Unknown`，阻止 close。
- `src/runtime.rs`：`BackgroundRunner` 已有 bounded command/event channel、`JobOutcome`、`try_shutdown_with_grace` 和 `shutdown_and_join_with_grace`（`src/runtime.rs:L236-L390`），可承载 heartbeat tick、mailbox poll、派生任务。`CancellationToken` 可做 lease 取消，但注意文档明确取消不是 rollback；只有 job `mark_completed` 后才安全接受 late cancel（`L49-L99`）。`InputBoundaryGate` 的 `context_parent`、工具批次和安全 boundary（`L794-L912`）可作为 node 与 child 交接点；不得在 active tool batch 中 close。
- `src/protocol.rs`：沿用 `MailboxRequest`、`InputQueueRequest`、严格 identifier/text/TTL 校验；新增 `DagEvent`/`DagQuery` 时使用 `deny_unknown_fields`、`MAX_ID_BYTES` 和 `MAX_MAILBOX_TEXT_BYTES`，并在 parse 阶段拒绝过期 epoch、空 node id、超过 descendants page limit。`MailboxOutcome::Abandoned` 对应 worker 崩溃/lease 过期，接收端据此保持 parent alive。
- `src/approval.rs`：DAG worker 的 tool side effect 必须经过现有 `ApprovalCoordinator`；只有 approval response 已持久化（`AcceptedApproval` 语义，`src/approval.rs:L144-L151`）才允许把 node 标为 Green。取消时沿用 `ApprovalError::Cancelled`（`L580-L595`），并将未决 approval 转为 Failed/KeepAlive，而非误报 Green。
- `src/tool_runtime.rs`：`execute_tool_batch` 的 1..32 并发上限、整批 cancellation、顺序结果和 panic 转内部错误（`src/tool_runtime.rs:L157-L176`, `L280-L347`, `L349-L455`）可直接作为一个 DAG node 的 work executor。节点只有所有 calls 有 terminal result 且副作用 journal 已完成才发 `NodeGreen`；任何 cancelled/internal result 只能 `KeepAlive` 或 `Failed`。
- `src/providers/**`：`openai.rs`、`anthropic.rs`、`google.rs`、`deepseek.rs`、`codex.rs` 和 `connection.rs/registry.rs` 应继续只实现 provider 请求、流事件和重试边界；在 provider stream 转成 `AgentEvent::Provider`/`ToolResult` 时由 `core.rs` 记录 node event。不要把 parent/sibling routing 放入 provider；provider 失败只产生 node failure/keep-alive 事件，由 DAG coordinator 决定是否派生替代 worker。

### 与 control.rs 的差异清单及验收

1. `control.rs` 只有一跳 parent 通知、无 sibling/grandparent 路由；zenpi 需结构化 `DagEvent` + mailbox，并用 parent 的 child map 递归汇总 descendants。
2. `control.rs` 的 `is_final` 即可触发 parent 通知；zenpi 必须把“自身 Green + 全部 child/grandchild Green + 无过期 lease/未决 work”作为唯一 `can_close` 条件。
3. `control.rs` 有 max-thread slot reservation；zenpi 应复用该思想，在 `derive_worker` 前做 bounded worker/lease reservation，失败时保持原 node alive，不丢新 work。
4. `control.rs` 的 watcher 无心跳和显式超时；zenpi 要增加 `lease_epoch/last_heartbeat_ms`、tick、过期转 `Unknown`、重派生策略，并使 mailbox `ttl_ms` 可验证。
5. `control.rs` fork 会复制 rollout 并注入 synthetic function output；zenpi 新 worker 可复用 session snapshot/parent source，但必须记录 `derived_from_work_id` 和幂等 event，避免把 fork 历史误当作 DAG 完成。
6. 可执行验收：建立 parent→child→grandchild，另建 sibling；验证 child 能向 parent、grandparent、sibling、direct child 发送并由 `MailboxRequest::Receive/Claim` 唯一消费；让 grandchild 未绿时 parent close 请求返回 KeepAlive；入队新 work 后原 node 仍有 heartbeat 且派生新 worker；所有 descendants 发 Green 后才写 `dag_close_decision=close`，重放 `SessionStore` 后结果一致。

## 未决问题

1. `control.rs` 本身没有定义 `ThreadManagerState::send_op`、`AgentStatus::is_final` 的具体状态转换，也没有定义 rollout JSONL/SQLite schema；本笔记只能依据调用点说明其契约。
2. 源文件没有规定 parent 被删除后 child watcher 的重试、通知补偿或 mailbox 的持久化实现；zenpi 需要在 DAG coordinator 中明确 dead-letter/Abandoned 策略。
3. zenpi 当前 `SessionStore` 的 session tree 与 `MailboxRequest` 的实际存储/路由实现边界，需要在接入时确认，尤其是跨进程接收、TTL 清扫和 lease tick 的 owner。
4. “全部 child/grandchild 全绿”中的 Green 是否允许 `Interrupted` 后恢复、是否允许 Failed 节点替代派生成功即视为绿，源文件无法确认；建议把策略作为显式 `ClosePolicy` 配置并写入 session journal。
5. provider 重试是否会产生重复 `ToolResult`、以及新 worker 是否共享 parent 的 `ApprovalPolicy`，在 `control.rs` 未定义；需由 `src/core.rs` 的 operation id/idempotency 和 `src/approval.rs` 的持久化边界补充验证。


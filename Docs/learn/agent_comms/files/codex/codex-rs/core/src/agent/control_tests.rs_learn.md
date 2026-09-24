# AC-019 — codex/codex-rs/core/src/agent/control_tests.rs

- source_id/item_id：`codex/codex-rs/core/src/agent/control_tests.rs` / `AC-019`
- source_path：`codex/codex-rs/core/src/agent/control_tests.rs`
- source_hash：`2f5d91da9952f766230292b12652053c2d8354f5c0f855e7cc1339e03d816dfc`
- source_bytes：`34808`
- source_lines：`1095`
- coverage：已按源文件顺序读取完整字节范围 `1..=34808`、行范围 `L1-L1095`（含导入、注释、辅助类型、全部测试函数）。

## 完整行为复盘

文件是 `agent` 控制面的异步集成测试，不定义生产导出符号；`use super::*` 引入 `AgentControl`、`AgentStatus`、`Op`、`SpawnAgentOptions`、`ThreadId`、`UserInput` 等被测类型，另显式引入 `ThreadManager`、`CodexThread`、`SessionSource/SubAgentSource`、事件模型和 `tokio::time`（`L1-L29`）。测试关注控制句柄与线程管理器之间的消息提交、状态广播、父子会话历史和资源槽位。

- `test_config_with_cli_overrides(cli_overrides)`（`L31-L48`）：创建 `TempDir` 作为隔离 `codex_home`，将调用方 TOML 覆盖和空的受管 macOS 配置交给 `ConfigBuilder`，异步 `build`；配置错误直接 `expect` panic，输出 `(TempDir, Config)`，由返回的临时目录保证测试生命周期内持久化路径有效。
- `test_config()`（`L50-L52`）：以空覆盖调用前者，默认配置是唯一默认值来源。
- `text_input(text)`（`L54-L59`）：把 `&str` 复制成一个 `UserInput::Text`，`text_elements` 固定为空，返回单元素 `Vec`；没有截断、校验或错误。
- `AgentControlHarness` 与 `new()`/`start_thread()`（`L61-L93`）：harness 持有 `_home`、`Config`、`ThreadManager`、`AgentControl`。`new` 用 dummy API key、测试模型 provider/home 构造 manager，再取得共享控制句柄；`start_thread` 异步注册线程并返回 `(ThreadId, Arc<CodexThread>)`，失败 panic。该结构模拟一个可派生多个线程的 parent 控制面。
- `has_subagent_notification(history_items)`（`L95-L110`）：只匹配 `ResponseItem::Message` 且 `role == "user"`，在文本内容（输入或输出）查找 `SUBAGENT_NOTIFICATION_OPEN_TAG`；图片和非消息项为 false。
- `history_contains_text(history_items, needle)`（`L112-L125`）：对所有消息的文本片段作 `str::contains`，不限制角色；图片和非消息项跳过。
- `wait_for_subagent_notification(parent_thread)`（`L127-L144`）：每 25ms 克隆 parent session history，直到通知出现；外层 2 秒 `timeout`，超时返回 `false`，没有主动取消被测 watcher。

控制错误、状态和订阅判据：

- `send_input_errors_when_manager_dropped`（`L146-L163`）：`AgentControl::default()` 没有关联 manager 时提交文本，必须返回错误字符串 `unsupported operation: thread manager dropped`；验证断链不会静默丢消息。
- `get_status_returns_not_found_without_manager`（`L165-L170`）：无 manager 查询任意新 `ThreadId`，返回 `AgentStatus::NotFound`。
- `on_event_updates_status_from_task_started`（`L172-L180`）：`TurnStarted`（`turn_id`、无 context window、默认 mode）映射为 `Some(Running)`。
- `on_event_updates_status_from_task_complete`（`L182-L190`）：`TurnComplete` 携带 `last_agent_message=Some("done")` 映射为 `Completed(Some("done"))`，保留最后回答。
- `on_event_updates_status_from_error`（`L192-L201`）：`Error{message:"boom", codex_error_info:None}` 映射为 `Errored("boom")`。
- `on_event_updates_status_from_turn_aborted`（`L203-L212`）：带 turn id、`Interrupted` 原因的 `TurnAborted` 映射为 `Interrupted`。
- `on_event_updates_status_from_shutdown_complete`（`L214-L218`）：`ShutdownComplete` 映射为 `Shutdown`。
- `spawn_agent_errors_when_manager_dropped`（`L220-L232`）与 `resume_agent_errors_when_manager_dropped`（`L234-L246`）：无 manager 的派生/恢复均返回相同 unsupported-operation 错误；恢复参数含 `SessionSource::Exec`，说明 source 不会绕过 manager 检查。
- `send_input_errors_when_thread_missing`（`L248-L264`）：有 manager 但目标不存在时，`send_input` 返回 `CodexErr::ThreadNotFound(id)`，精确保留请求 id。
- `get_status_returns_not_found_for_missing_thread`（`L266-L271`）：缺失线程状态仍为 `NotFound`，与 manager 缺失的可观察结果一致。
- `get_status_returns_pending_init_for_new_thread`（`L273-L279`）：刚 `start_thread` 后状态必须是 `PendingInit`，初始化不是同步完成的假设。
- `subscribe_status_errors_for_missing_thread`（`L281-L291`）：缺失线程订阅返回 `ThreadNotFound`，不产生空 receiver。
- `subscribe_status_updates_on_shutdown`（`L293-L311`）：新线程订阅初始 `PendingInit`；向线程提交 `Op::Shutdown {}` 后等待 `watch::Receiver::changed()`，最终必须广播 `Shutdown`，表明状态流是异步、可等待且终态可观测。
- `send_input_submits_user_message`（`L313-L346`）：有效线程提交一个文本，返回非空 `submission_id`，manager 的捕获操作必须含同一 thread id 和 `Op::UserInput{items, final_output_json_schema:None}`；提交是控制面到线程邮箱/操作队列的显式消息。

派生、历史分叉与完成回注：

- `spawn_agent_creates_thread_and_sends_prompt`（`L348-L377`）：`spawn_agent` 创建并注册新 thread，且捕获到 child 上的 `Op::UserInput`；返回 id 可由 manager 查询。
- `spawn_agent_can_fork_parent_thread_history`（`L379-L463`）：parent 先注入 seed，再记录 `spawn_agent` `FunctionCall` 并 materialize/flush rollout；`spawn_agent_with_options` 传 `SessionSource::SubAgent(ThreadSpawn{parent_thread_id, depth:1,...})` 和 `fork_parent_spawn_call_id` 后创建不同 id 的 child。child history 必须包含 parent seed，且 child 收到 `child task`；最后显式 shutdown child 与 parent，验证 fork 后生命周期独立。
- `spawn_agent_fork_injects_output_for_parent_spawn_call`（`L465-L540`）：同样准备 parent call，child history 必须含相同 `call_id` 的 `FunctionCallOutput`，其文本为 `FORKED_SPAWN_AGENT_OUTPUT_MESSAGE` 且 `success == Some(true)`；派生将“父调用已完成”的合成结果写入 child 上下文，避免模型看到悬空工具调用。
- `spawn_agent_fork_flushes_parent_rollout_before_loading_history`（`L542-L616`）：parent call 不预先 flush；fork 仍必须在 child 中同时出现该 call 和合成 output，且 call index 小于 output index，证明派生前强制 flush parent rollout，再按顺序加载历史。

并发额度、关闭和恢复：

- `spawn_agent_respects_max_threads_limit`（`L618-L659`）：配置 `agents.max_threads=1`，已有一个普通 thread 后允许第一个 agent，再次 spawn 返回 `CodexErr::AgentLimitReached{max_threads:1}`；shutdown 首个 agent 释放测试资源。
- `spawn_agent_releases_slot_after_shutdown`（`L661-L693`）：额度为 1 时，shutdown 第一个 agent 后第二个 spawn 成功，说明槽位与活动线程生命周期绑定而非永久计数。
- `spawn_agent_limit_shared_across_clones`（`L695-L729`）：clone `AgentControl` 后从 clone 派生一个 agent，原句柄再次派生仍触发同一额度错误；guard 位于共享状态而非句柄副本。
- `resume_agent_respects_max_threads_limit`（`L731-L776`）：先 spawn/shutdown 得到可恢复 id，再占满活动槽；`resume_agent_from_rollout(..., SessionSource::Exec)` 返回 `AgentLimitReached`，恢复与新建共享同一并发预算。
- `resume_agent_releases_slot_after_resume_failure`（`L778-L806`）：对不存在 rollout 的随机 id 恢复应失败，但失败后立即 spawn 成功，证明恢复路径采用临时 guard/回滚，错误不泄漏槽位。

父子完成通知与身份元数据：

- `spawn_child_completion_notifies_parent_history`（`L808-L839`）：用 `ThreadSpawn` source 派生带 role `explorer` 的 child，提交 child shutdown；`wait_for_subagent_notification` 必须在 2 秒内看到 parent history 的用户通知，完成事件跨线程回写 parent 会话。
- `completion_watcher_notifies_parent_when_child_is_missing`（`L841-L877`）：直接调用 `maybe_start_completion_watcher` 监听不存在的 child；parent 仍收到通知，且 history 文本含精确 `agent_id` 与 `"status":"not_found"`，缺失被转换为可消费的终态而非永久等待。
- `spawn_thread_subagent_gets_random_nickname_in_session_source`（`L879-L920`）：传 `agent_nickname:None` 的 ThreadSpawn 仍在 child config snapshot 中得到非空 nickname，同时保留 parent id、depth=1、role `explorer`。
- `spawn_thread_subagent_uses_role_specific_nickname_candidates`（`L921-L962`）：向 config 的 `agent_roles` 注册 `researcher` 候选 `Atlas`，派生 role=researcher 后 snapshot nickname 必须是 `Some("Atlas")`，候选优先于通用随机名。
- `resume_thread_subagent_restores_stored_nickname_and_role`（`L964-L1095`）：启用 `Feature::Sqlite`，派生 explorer child；等待 status 越过 `PendingInit`（最多 5s），再轮询 state DB（10ms，最多 5s）直到 metadata 持久化 nickname/role。shutdown 后以空 nickname/role 调 resume，必须复用同一 thread id、parent、depth，并从 SQLite 恢复原 nickname 与 `explorer` role；最终再次 shutdown。该测试同时验证异步初始化、持久化可见性和 resume 的身份稳定性。

并发语义贯穿所有 `#[tokio::test]`：控制 API 是 async，状态通过 watch receiver，history 通过异步 clone；测试使用显式 timeout 防止 watcher、初始化或持久化无限等待。`manager.captured_ops()` 是测试观测点，说明操作提交与实际执行可分离。

## 状态、取消、恢复与副作用

本文件没有直接调用 cancel/timeout/retry API，但测试边界揭示了控制面语义。状态是 `PendingInit -> Running/Completed/Errored/Interrupted/Shutdown` 的事件投影（`L172-L218`、`L293-L311`）；缺失对象投影为 `NotFound`（`L165-L170`、`L266-L271`）。`Op::Shutdown` 是显式取消/关闭消息，child 和 parent 都需提交后才结束（`L454-L463`、`L607-L616`、`L680-L692`）；没有强杀线程的证据，完成 watcher 负责把 shutdown 或缺失变成 parent history 通知（`L808-L877`）。

时间控制只用于测试等待：通知轮询 25ms、总计 2s（`L127-L144`）；child 初始化和 SQLite 元数据等待各 5s，持久化轮询 10ms（`L1010-L1045`）。超时只让断言失败/返回 false，不会回滚已经提交的 `Op`、历史记录或线程；因此超时语义应理解为观测超时，而非业务取消。源码没有重试循环来重放 prompt；唯一“恢复”是 `resume_agent_from_rollout`，额度拒绝与缺失 rollout 都必须快速失败并释放 guard（`L731-L806`）。

持久化副作用包括 parent/child rollout materialize+flush、合成 `FunctionCallOutput`、parent 通知、SQLite thread metadata（`L395-L405`、`L465-L616`、`L1030-L1045`）。`TempDir` 清理测试文件；测试末尾 shutdown 用于回收活动线程。没有网络副作用（dummy auth/provider）；没有验证跨进程锁或崩溃恢复。父子 source 保存 `parent_thread_id`、`depth`、nickname、role；直接 sibling/grandparent 通信在源内没有 API 或测试。

## 源内测试与行为判据

源文件本身全部是 `#[tokio::test]`（`L146-L1095`），没有生产实现；不存在同目录额外测试引用可从该文件确认的判据，因此行为判据就是上述断言集合：

1. 控制句柄断链/缺 thread 必须得到稳定错误或 `NotFound`，不能伪造成功（`L146-L170`、`L220-L291`）。
2. 事件到状态的映射必须一一对应，订阅者必须能观察 shutdown 终态（`L172-L218`、`L293-L311`）。
3. spawn 必须注册 child、投递 prompt、可选择性 fork parent history，并按 call 顺序注入成功的 synthetic output（`L348-L616`）。
4. 线程并发上限在 spawn/resume 和 `AgentControl` clone 间共享，shutdown 或失败后释放槽位（`L618-L806`）。
5. child 完成或不存在都必须通知 parent；通知内容含 child id/status；nickname/role 生成和 SQLite resume 必须稳定（`L808-L1095`）。

可独立验证方法：为 zenpi 构造 parent/child `SessionStore`，向 parent journal 写入 spawn call，fork 后检查 child 的 call/output 顺序；用 bounded runtime 提交 shutdown 并监听终端事件；将同一 `Arc` 控制句柄 clone 两次并把 `max_threads` 设为 1；模拟缺失 child 写入 `not_found` 通知；启用 SQLite 后检查 metadata 的 `agent_nickname/agent_role`，再 resume 并比对同一 node id。

## zenpi Rust 映射

zenpi 已有的控制、运行时和持久化原语足以承载该测试文件的核心语义，但需要把“线程”提升为有身份的 DAG worker node，并补上后代闭合判定。

- `src/core.rs`：`Agent`/`Turn` 是单会话状态机；`Turn.parent_id`（定义与校验见 `src/core.rs`）可承载一次消息或 turn 的直接父关系，`AgentEvent`、`ProcessResult`、`submit`/`run_active_turn_cancelable` 是事件和任务边界。建议新增 `WorkerNodeId`、`WorkerNodeState { PendingInit, Running, Green, Errored, Interrupted, Closing, Closed, NotFound }`、`WorkerRelation { parent, depth, role, nickname }` 和 `DagNodeRecord { node_id, parent_id, children: BTreeSet<WorkerNodeId>, state, lease_id, generation }`，由 `Agent` 所属编排器维护。`spawn` 对应测试中的 `spawn_agent_creates_thread_and_sends_prompt`，应先写 node record，再通过 `Agent::input_port`/`TurnInputRequest` 投递；`resume` 应复用 node id 和 persisted relation。`Agent::close` 只能标记自身关闭候选，不应直接宣布 DAG 完成。
- `src/session.rs`：`SessionStore` 是追加式 JSONL，已有 `Handoff/HandoffRecord/RuntimeIntent`、`append_event`，以及 `LiveSessionRegistry`、`SessionMailbox`。复用 mailbox 的 `MailboxMessage`（sender/recipient/request_id/digest/sequence/status/claim_token/result）、`claim_next`、`claim_message`、`finish_claim`、`finish_claim_with_reply`（实现位置约 `L3280-L3560`）作为跨 worker 的 durable message/邮箱/请求-回复协议；每次 claim 绑定 owner epoch，完成结果幂等校验。新增 `DagNodeRecord`/`DagEdge` 事件类型时追加到同一 journal，或在独立 sidecar 中以 node id 索引，不能让内存 `BTreeSet` 成为唯一事实源。`SessionTree` 的 `TreeEntry.parent_id`、`ancestry`、`fork_at_tree_leaf` 适合 transcript 分支和祖先回放（`src/session_tree.rs`），但它是树而不是完整 worker DAG；worker 边应独立记录，以支持 sibling 多父引用/共享依赖。
- `src/protocol.rs`：`MailboxRequest::{Send,Receive,Acknowledge,Claim,Complete}`、`MailboxOutcome::{Succeeded,Failed,Abandoned}` 以及 `MAX_MAILBOX_TEXT_BYTES`、TTL/ID 校验是可复用协议面（定义约 `L81-L110`、验证约 `L544-L585`）。建议扩展 `Command::Worker`/`WorkerRequest`：`Spawn { parent, goal, role }`、`Send { recipient }`、`Status`、`Close`、`Heartbeat`、`Derive`，复用现有 request `id` 做幂等键。协议中明确 `recipient_session_id` 可指 parent、grandparent、直接 sibling 或直接 child；路由必须检查 DAG registry 的真实边，不能由客户端任意伪造 sender/authorization。事件中增加 `worker_started/green/blocked/derived/close_deferred`，保持 JSONL 一行一事件。
- `src/headless.rs`：该模块已是唯一 wire I/O，拥有有界 event mailbox、`BackgroundRunner` 作业映射、`handle_runtime_event` 对 `Accepted/Started/Queued/CancelRequested/Completed/Closed` 的顺序化输出，并有 `mailbox_slash_view` 将 slash mailbox 映射到 `execute_mailbox`（约 `L2040-L2130`、`L4952-L5220`）。把 worker DAG 命令接在现有 `Command::Mailbox`/runtime dispatch 旁：接收时登记 `request_to_job`，完成时先刷 worker/agent 事件，再写 terminal response；为 parent notification 使用 `MailboxRequest::Send`，而非直接从后台线程写 stdout。需要给 terminal response 加 `node_id`、`parent_id`、`generation`，让 reconnect/replay 能按节点去重。
- `src/runtime.rs`：`BackgroundRunner` 的 bounded command/event channels、FIFO queued follow-up、`CancellationToken`、`RuntimeEvent::Completed/Cancelled/Panicked` 和 bounded shutdown grace（`L51-L203`、`L279-L413`）正对应测试的异步提交、状态流和关闭语义。为每个 DAG node 保存一个 `JobId` 与 `WorkerNodeId` 映射；`try_submit` 失败返回 `QueueFull/Closed`，不得把 node 标成 green；`CancellationToken` 只提供 cooperative cancel，不能假设 detached job 的副作用回滚。建议增加 `heartbeat` 事件或由 scheduler 周期性向 node mailbox 发 lease renew；超 lease 进入 `Interrupted`，等待显式 resume/retry。
- `src/tool_runtime.rs`：`ToolBatchDecision`、bounded `max_concurrency`、owner unwind 前先 cancel workers（约 `L275-L341`）可复用作一个 node 内工具并发；但它不是 DAG worker registry。将 side effect 绑定 `WorkerExecutionBinding { blueprint_id, goal_id, item_id, lease_id, policy_digest, expires_at_ms }`，每个派生 worker 继承只读 provenance，不共享可变 approval。子 worker 完成前必须 drain tool events；取消后 operation journal 标为 unknown/cancelled，避免把 detached side effect 当成功。
- `src/approval.rs`：`ApprovalCoordinator` 是 worker 与 host 的同步 rendezvous，`request/request_response`、`cancel_all`、`cancellation_epoch` 和 accepted-record 持久化适合工具审批和取消传播。parent 派生 child 时应把 node/lease/call id 放进 `ApprovalRequest`，仍由 host policy 决定 Allow/Deny；不能把 parent 的 remembered approval 自动升级成 child 的全局授权。child 被关闭时先 `cancel_all`，再写 node terminal event。
- `src/providers/**`：provider 只负责模型请求/流事件；把 node id、turn id、operation id 作为 `RequestScope` correlation metadata，provider 不负责 parent/sibling 路由或 close 判定。失败映射 `Errored`，取消映射 `Interrupted/Cancelled`，成功回答才可候选为 `Green`。

面向 zenpi DAG 的最小可执行机制：

1. 建立 `DagRegistry`（建议 `src/dag.rs`，由 `core::Agent` 或 headless project owner 持有），存 `node_id -> parent_id, children, state, lease, generation, session_id`，并在 `SessionStore.append_event` 中记录 `node_spawned/node_state/node_edge`。注册时校验 parent 存在、depth 单调、无环、节点 id 幂等。
2. 为每个 node 建一个 `SessionStore`/session id 和 mailbox owner；`Send`/`Receive` 使用 `SessionMailbox`。收件目标允许：`parent_id`、沿 parent 链的任一 `grandparent`、`children` 中任一直接 child、同一 parent 的直接 sibling；授权由 registry 计算，拒绝越级或非共享 workspace 的地址。消息 envelope 至少含 `message_id`, `from_node`, `to_node`, `kind`, `payload`, `generation`, `reply_to`, `expires_at_ms`。
3. scheduler 从 mailbox `claim_next` 取任务并调用 `BackgroundRunner::try_submit`；事件回写 node state 和 parent notification。派生使用 `Derive { parent, goal }`：先持久化边和 lease，再提交 child job；失败删除/标记 edge admission，不吞掉错误。grandparent/sibling 通信只走邮箱，不直接持有对方 `Agent` 锁。
4. 定义 `is_green(node)`：自身最近一次终态为 `Green`，且 `children` 集合非空/空均满足“所有直接 child 递归 `is_green`”；递归检查全部 descendants（grandchild），发现 `Running/PendingInit/Errored/Interrupted/NotFound/unknown` 均不可 close。为避免竞态，在 registry 锁内读取 generation/children 快照，再用 compare-and-append 写 `close_requested`；child 新增会使 parent close token 失效。
5. close 流程：worker 自身回答完成后只发 `NodeCandidateGreen`；scheduler 原子检查自身及全部 descendants 全绿且无 in-flight mailbox/approval/tool operation，才写 `NodeClosed` 并释放 lease/owner。条件不满足时写 `CloseDeferred { blocking_nodes }`，保持 parent worker alive，向其 mailbox 投递新工作；若有新工作则 `generation += 1`、派生最小数量新 worker，旧 worker 继续保活直到 handoff/ack 完成。
6. 保活采用 `LiveSessionRegistry::heartbeat` + lease TTL；heartbeat 失败或 TTL 到期进入 `Interrupted`，scheduler 可在明确 retry policy 下 resume 同一 node id。shutdown 是取消请求，先发 `CancelRequested`，在 runtime grace 后才 detached；日志明确“取消不回滚外部副作用”。

差异清单与验证落点：

- 源测试验证 ThreadManager 的 max slot、fork history、completion watcher；zenpi 目前已有 bounded runtime/mailbox，但没有统一 `DagRegistry`、祖先/兄弟授权和递归 green 判定。
- 源测试只覆盖 parent-child；zenpi 需新增 parent→grandparent、parent↔sibling、parent→child 的四向 mailbox 测试，断言非直接关系被拒绝、TTL/claim token/幂等 reply 可恢复。
- 源的 nickname/role 可映射 `DagNodeRecord.role/nickname`；zenpi 需在 `session.rs` 事件和 resume projection 中保存，不能只存内存。
- 源的 synthetic `FunctionCallOutput` 对应 zenpi 的 `HandoffRecord`/mailbox result；验证 child history 中 call 先于 output，再验证 parent 收到 `Succeeded` receipt。
- 关闭验证应新增 Rust 测试：child green 但 grandchild running 时 parent `CloseDeferred` 且 worker 仍可接收新任务；全部 descendants green 后才出现唯一 `NodeClosed`；派生新 worker 后旧 generation 的重复 close 被幂等拒绝。运行 `cargo test` 目标模块并检查 JSONL journal 顺序、replay 与 `BackgroundRunner` terminal event。

## 未决问题

- `AgentControl`、`ThreadManager` 内部如何实现 slot guard、completion watcher 及 parent 通知，`control_tests.rs` 只通过黑盒行为确认，无法确定锁类型、任务取消时序或 watcher 是否单独持有 manager（`L618-L877`）。
- 源未定义 grandparent、sibling 或多父 DAG 边的通信授权，也没有“自身及全部 child/grandchild 全绿才 close”的判定；上述 `DagRegistry` 方案是 zenpi 映射设计，需要新增测试确认。
- `FORKED_SPAWN_AGENT_OUTPUT_MESSAGE` 的完整协议格式、通知 JSON schema、nickname 随机源和 SQLite schema 未在本文件定义；只能确认文本存在、role/nickname 最终值和持久化可见性（`L465-L540`、`L879-L1095`）。
- `resume_agent_from_rollout` 对缺失 rollout 的具体错误枚举、恢复期间 parent notification 是否重复、以及跨进程 resume 的并发锁行为，源测试没有断言。
- 测试没有验证 provider/tool 外部副作用、崩溃后的 journal replay、消息 TTL 到期、事件邮箱溢出或 detached worker，因此这些边界需在 zenpi 的 `session.rs`、`headless.rs`、`runtime.rs` 集成测试中补齐。

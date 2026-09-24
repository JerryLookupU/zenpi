# AC-025 — codex/codex-rs/core/src/agent/status.rs

- source_id/item_id: `codex/codex-rs/core/src/agent/status.rs` / `AC-025`
- source_path: `/Users/wangweiyang/GitHub/codex/codex-rs/core/src/agent/status.rs`
- source_hash: `fbbbec8b65035d17ae977ca3d73a24286a6e11fcb176daa1909a3cbc777e3910`
- source_bytes: `1094`
- source_lines: `27`
- coverage: 已按文件顺序读完字节 `0-1093`（共 1094 字节），行 `L1-L27`（含注释、空行、导入、全部类型引用、函数与匹配分支）。

## 完整行为复盘

源文件只有两个 `pub(crate)` 函数，职责是把协议事件折叠成生命周期状态，并判断状态是否终态；没有结构体、宏、静态变量或外部 I/O。

- `agent_status_from_event(msg: &EventMsg) -> Option<AgentStatus>`（`L4-L20`）：输入是对一个 `EventMsg` 的共享借用，输出为新的 `Option<AgentStatus>`。`L4-L5` 的注释明确“不影响状态跟踪”的事件返回 `None`。匹配行为如下。
  - `EventMsg::TurnStarted(_)`（`L7-L8`）无条件返回 `Some(AgentStatus::Running)`；事件载荷被忽略，因此不会把 `turn_id`、模型窗口等信息带入状态。
  - `EventMsg::TurnComplete(ev)`（`L9`）返回 `Some(AgentStatus::Completed(ev.last_agent_message.clone()))`。`last_agent_message` 是 `Option<String>`，通过 `clone` 解除借用，故调用者可继续使用事件；完成状态保留最后一条 assistant 文本（也允许为 `None`）。
  - `EventMsg::TurnAborted(ev)`（`L10-L15`）再按 `ev.reason` 分支。`Interrupted`（`L11-L13`）映射为 `Interrupted`；其他所有原因（当前协议中的 `Replaced`、`ReviewEnded`，以及未来新增但未单独匹配的原因）都映射为 `Errored(format!("{:?}", ev.reason))`（`L14`）。这里对 reason 采用调试格式字符串，边界是枚举 Debug 表示的稳定性而非机器可解析错误码；匹配按值读取 `ev.reason`，该字段必须实现 Copy/可复制语义。
  - `EventMsg::Error(ev)`（`L16`）复制 `ev.message`，返回 `Some(AgentStatus::Errored(...))`。空消息不会在本文件拒绝。
  - `EventMsg::ShutdownComplete`（`L17`）返回 `Some(AgentStatus::Shutdown)`，代表 agent 关闭完成。
  - 通配分支 `_`（`L18`）返回 `None`；因此 token、工具进度、初始化等事件不会改变此 reducer 的当前状态。
  - 函数没有 `Result` 错误路径、没有默认状态写入，也不检查事件顺序、turn ID 或重复事件。它是纯函数：只读输入，最多克隆一个字符串，不持锁、不发消息、不产生线程或持久化副作用；可被多个线程并行调用，线程安全性取决于 `EventMsg`/`AgentStatus` 的 Send/Sync，而函数本身没有共享可变状态。

- `is_final(status: &AgentStatus) -> bool`（`L22-L27`）：输入是状态共享借用，输出布尔值。`L23-L26` 使用 `matches!` 的否定：只有 `PendingInit`、`Running`、`Interrupted` 被判定为非终态，其他状态均为终态，即 `Completed(_)`、`Errored(_)`、`Shutdown`、`NotFound` 都返回 `true`。它不区分成功与失败，也不读取 `Completed` 内的消息；没有错误路径、默认参数或副作用，同样可并发调用。重要边界是“final”只表示不再等待本 agent 的普通生命周期事件，并不证明子 worker 已完成，也不证明外部副作用已提交。

导入 `AgentStatus` 与 `EventMsg`（`L1-L2`）来自 `codex_protocol::protocol`；本文件未重新导出它们。由于函数是 `pub(crate)`，可见性限制在 core crate 内。

## 状态、取消、恢复与副作用

本文件没有取消 token、超时、重试、恢复、持久化或外部副作用实现。`Interrupted` 只是由 `TurnAborted` 的 `Interrupted` reason 派生的观察状态（`L10-L13`），不会自动重启；`Replaced`/`ReviewEnded` 会被标成 `Errored`（`L14`），也不会自动重试。`ShutdownComplete` 仅映射为 `Shutdown`（`L17`），没有关闭通道或回收线程动作。状态不会在本地缓存，调用者必须自行保存返回值或在事件流上重放；事件乱序时最后一次调用结果可能覆盖更早的语义，源内没有去重或版本检查。字符串 clone 与 `format!` 是唯一内存分配，未写日志、文件、网络或 provider。

这意味着 `is_final` 的终态判据不能直接当作 DAG 节点可关闭判据：`Errored`、`Shutdown`、`NotFound` 虽然 final，却不是“全绿”；`Interrupted` 反而是非 final，适合保活和后续输入。zenpi 需要在此 reducer 之上另加成功色标、父子汇总和 lease 生命周期。

## 源内测试与行为判据

源内未包含测试。可独立验证的判据是：构造每个 `EventMsg` 变体并断言 `TurnStarted -> Some(Running)`、`TurnComplete(last_agent_message) -> Some(Completed(克隆文本))`、`TurnAborted(Interrupted) -> Some(Interrupted)`、其他 abort reason -> `Some(Errored(Debug reason))`、`Error -> Some(Errored(克隆 message))`、`ShutdownComplete -> Some(Shutdown)`，任意未列事件 -> `None`；再断言 `is_final(PendingInit|Running|Interrupted)==false`，以及 `Completed(None/Some(_))`、`Errored(_)`、`Shutdown`、`NotFound` 均为 `true`。还应验证返回的 `String` 与输入脱钩（修改/释放原事件后结果仍有效），以及多线程只读调用无数据竞争。

## zenpi Rust 映射

**可复用原语。** 将本文件的 reducer 语义落在 `src/core.rs` 的 `AgentEvent`（`src/core.rs:L281-L325`）与 `ProcessResult`（`src/core.rs:L5368-L5408`）之上，新增 `agent_comms::status_from_event(&AgentEvent) -> Option<NodeStatus>`，保留“未影响状态则 `None`”的稀疏事件约定。消息/邮箱优先复用 `src/protocol.rs:L81-L113` 的 `MailboxRequest::{Send,Receive,Acknowledge,Claim,Complete}`；`MailboxRequest` 已在 `Command::Mailbox` 路由（`src/protocol.rs:L479-L484`）并有 TTL、文本、ID 校验（`src/protocol.rs:L544-L590`）。持久对象使用 `src/session.rs:L3180-L3255` 的 `MailboxMessage`/`MailboxStatus`，其 digest、TTL、claim token 和终态校验可避免 sibling 重放。进程内快速事件使用 `src/runtime.rs:L171-L193` 的 `RuntimeEvent` 与有界 `sync_channel`；有界邮箱/事件流能复用 `src/headless.rs:L940-L1020` 的优先级和丢弃计数策略，但 DAG admission、completion、error 事件不得被普通进度挤掉。

**会话父子关系与寻址。** `src/core.rs:L140-L205` 的 `Turn.parent_id`/`Turn::with_parent` 是最小父链；`src/session_tree.rs` 已维护 entry-parent、children 计数及祖先校验，可作为 DAG 视图而不是把 mailbox 顺序误当树。建议新增 `WorkerNodeId`、`WorkerRelation { parent: Option<WorkerNodeId>, children: BTreeSet<WorkerNodeId> }`，持久化到 session event；直接 parent 由 `parent_id` 查找，grandparent 沿 parent 链上溯一次，direct sibling 取同一 parent 的 children 后排除自身，direct child 取当前节点 children。每条 mailbox 消息附 `sender_node_id`、`recipient_node_id`、`relation`、`work_id` 与 `reply_to`，这样同一 session 中也不会把不同 DAG 节点混淆。跨进程写入仍走 `SessionMailbox`；同进程 worker 之间可用 `BackgroundRunner` 事件通道，再以 mailbox 做 durable fallback。

**最小保活与派生机制。** 复用 `src/session.rs:L3274-L3355` 的 `LiveSessionRegistry::{register,heartbeat,unregister,active}` 和 `src/core.rs:L821-L844` 的 `register_live_owner`/`heartbeat_live_owner`/`unregister_live_owner`。建议在 `agent_comms::NodeLedger` 中维护 `self_status`、每个 child 的最新 `NodeStatus`、`last_heartbeat_ms`、`owner_epoch`、`generation`；`heartbeat` 只延长当前 epoch，过期节点不能继续 claim。把本文件的 `is_final` 拆成两层：`is_terminal`（Completed/Errored/Shutdown/NotFound）和 `is_green`（仅业务成功的 Completed，或明确的 Succeeded mailbox 结果）。`can_close(node)` 必须同时满足自身 `is_green`、所有直接 child `is_green`，并递归确认全部 grandchild/后代；任何 `PendingInit/Running/Interrupted` 或错误/失败后代都阻止 close。close 前调用 `src/core.rs:L2823-L2843` 的 `settle_blueprint_worker`，并确保 owned children 已 reaped；续租可复用 `renew_blueprint_worker`（`src/core.rs:L2846-L2868`）。若 `can_close` 为假且收到新工作：先保持 owner heartbeat，再用 `src/runtime.rs:L279-L315` 的 `BackgroundRunner::spawn/try_submit` 派生一个新 worker；新 worker 的 `parent` 指向当前节点，复制受限的 lease/budget correlation，完成后通过 mailbox `Complete` 回传。派生失败、队列满或 lease 过期时保活并返回可重试错误，不能伪造绿色或提前 close。`src/core.rs:L2870-L2885` 的取消接口只发出 host-owned cancellation，不能声称 children 已回收，这正好适合作为 close 前的反事实保护。

**模块落点对照。**

- `src/headless.rs:L553-L724` 在 JSONL 循环中注册/注销 live owner、处理 EOF/Shutdown；应在 `handle_command` 的 mailbox/tree 分支发出节点状态和 heartbeat，不能把 EOF 直接解释为 DAG 全绿。`run_stdio_owned`/`run_async_streams`（`L792-L812`）可承载事件推送与重连。
- `src/core.rs:L403-L442` 的 `Agent` 已有 `session`、`events`、`worker_budget`、`live_owners`；建议新增 `node_ledger` 字段和 `close_if_descendants_green`、`derive_child_worker`、`status_from_event`，在 `process_with_cancel_and_events`（`L5373-L5408`）每次 provider/tool 终态后更新 ledger。
- `src/session.rs` 提供 append-only journal、tree projection、`MailboxMessage` 与 `LiveSessionRegistry`（`L3170-L3510`）；应新增类型化的 `worker_node_started/status/heartbeat/derived/closed` 事件并让恢复重建 ledger。不要只依赖内存 registry：重启后以 owner epoch + durable mailbox 恢复，过期 claim 只能重新入队/显式失败。
- `src/tool_runtime.rs:L1-L7` 明确 executor 不写第二份 journal、不重试 interrupted call；因此 worker 派生与重试决策放在 `core`/`session`，tool runtime 只报告有序 `ToolResult`。其有界并行与取消（`L274-L347`）可作为 child work 批处理，但 stop 后未执行 call 必须保持非绿。
- `src/runtime.rs:L49-L97` 的 `CancellationToken` 是 lock-free、幂等、协作式；`L156-L193` 的 `JobOutcome`/`RuntimeEvent` 已区分 `Succeeded/Failed/Cancelled/Panicked` 和 `Completed/Closed`。DAG 状态 reducer 应把 `Cancelled/Panicked` 映射为非绿，并在 `mark_completed` 之后才接受 late cancel 的成功结果；`try_shutdown_with_grace` 的 detach 语义不能当作 child reaped。
- `src/protocol.rs` 负责 wire schema、边界校验和命令名；复用 mailbox action，新增 `DAGStatus`/`DAGClose`/`DAGDerive` 需使用严格 ID、TTL、payload 上限，回复用 `StdioResponse`/`StdioEvent`。直接 sibling/ancestor 寻址必须由 host 注入授权身份，不能由客户端任意填写 sender 或 workspace（已有注释说明身份归 mailbox owner）。
- `src/approval.rs:L126-L216,L359-L452` 的 `ApprovalCoordinator` 可作为需要人类批准的 child work rendezvous；取消应 `cancel_all`/`emergency_cancel`，持久化用 `persist_accepted` 后才释放等待 worker，避免 approval side effect 在 DAG 状态尚未 durable 时被误标绿。
- `src/providers/**` 继续提供 backend/model/connection 与 `ProviderEvent`；provider stream 只产生进度和结果，节点状态转换统一在 core 的事件适配器完成。provider transport error、`BackendError::Cancelled`、空响应应映射非绿并触发保活/重试策略；只有 journal 已记录的成功业务结果才能进入 `is_green`。

**差异清单（可执行、可验证）。** (1) 当前 status.rs 只认识单 agent 的 `EventMsg`，zenpi 需新增 `NodeStatus`/`is_green` 与 descendants 聚合；(2) 当前没有祖父/兄弟寻址 API，需在 `session_tree` 上补 `parent_of`、`grandparent_of`、`siblings_of`、`children_of` 并用 fixture 验证四种关系；(3) 当前 mailbox 已支持 claim/complete，但 payload 没有 DAG relation，需扩展 schema 并保持旧请求兼容；(4) 当前 live owner 是 admission boundary、不会启动 scheduler（`src/session.rs:L3274-L3276`），需由 core 显式调用 `BackgroundRunner` 派生且测试 owner epoch 防旧 worker 完成新任务；(5) 当前没有“全后代绿色才 close”原子检查，需在 session journal 锁/单写者边界内实现 compare-and-append，测试并发 child completion、重复 completion、过期 lease、队列满和 shutdown detach；(6) close/reap、heartbeat、derive、mailbox result 必须可从 journal 恢复，不能只依赖 `Vec<AgentEvent>`。

## 未决问题

无法从该源文件确认：事件是否保证按 turn 顺序到达；`TurnAborted` 的 reason 是否会继续增加；`Errored` 的 Debug 字符串是否有稳定协议契约；`Completed(None)` 是否代表成功但无文本；以及真实业务中哪些结果应定义为“绿色”。这些点需要 zenpi 的产品协议、provider 约定和 DAG 任务策略另行确定。

# AC-032 — codex/codex-rs/core/src/tasks/undo.rs

- source_id/item_id: `codex/codex-rs/core/src/tasks/undo.rs` / `AC-032`
- source_path: `codex/codex-rs/core/src/tasks/undo.rs`
- source_hash: `0bb393e546e415cf8bb33d088a09bdc668035f0683aedc4f9925859197a3ba44`
- source_bytes: `4227`
- source_lines: `131`
- coverage: 已按文件顺序读取完整字节范围 `1-4227` 与行范围 `L1-L131`，包含注释、导入、类型、实现和函数体。

## 完整行为复盘

1. 导入与依赖（L1-L18）：使用 `Arc` 共享任务；`TurnContext` 提供工作目录和 ghost snapshot；`EventMsg::{UndoStarted,UndoCompleted}` 是会话事件协议；`TaskKind`、`SessionTask`、`SessionTaskContext` 是任务调度边界；`restore_ghost_commit_with_options` 是实际 Git 恢复副作用；`ResponseItem` 用于识别历史中的 `GhostSnapshot`；`CancellationToken` 提供协作式取消；`tracing` 记录成功、可恢复失败和 join 失败。

2. `pub(crate) struct UndoTask`（L20）：无字段的任务标记类型，不携带输入状态；状态全部来自会话、`TurnContext` 和历史快照。

3. `UndoTask::new() -> Self`（L22-L26）：构造零大小的 `UndoTask`，无错误、无默认配置、无分配；返回 `Self`。

4. `SessionTask::kind(&self) -> TaskKind`（L29-L32）：固定返回 `TaskKind::Regular`，因此撤销走普通会话任务生命周期，而不是特殊后台类别。

5. `SessionTask::span_name(&self) -> &'static str`（L34-L36）：固定返回 `"session_task.undo"`，供 tracing span 关联。

6. `SessionTask::run`（L38-L129）：
   - 输入是共享的 `SessionTaskContext`、`TurnContext`、被忽略的 `Vec<UserInput>` 和 `CancellationToken`；输出类型为 `Option<String>`，所有路径最终均返回 `None`（L38-L44、L68、L93、L129），所以结果通过事件而非返回值传递。
   - 先递增 `codex.task.undo` telemetry counter（L45-L49），克隆会话句柄并发送 `UndoStarted{message: Some("Undo in progress...")}`（L50-L57）。事件发送是异步等待；发送失败在本文件中不分支处理。
   - 仅在真正读取历史前检查一次取消（L59-L69）。已取消时发送 `UndoCompleted{success:false,message:"Undo cancelled."}` 并立即返回；没有超时、重试或回滚逻辑。
   - 读取会话历史的原始项并复制成可修改的 `Vec`；初始化失败结果（L71-L76）。从尾部向前查找最近的 `ResponseItem::GhostSnapshot`，复制其 `ghost_commit` 和索引（L78-L88）。边界是没有快照：设置 `"No ghost snapshot available to undo."`，发送完成事件并返回（L89-L94）；历史中更早的快照不会被选中。
   - 取得 commit ID、当前工作目录和 ghost snapshot（L96-L98），在 `tokio::task::spawn_blocking` 中构造 `RestoreGhostCommitOptions` 并调用 `restore_ghost_commit_with_options`（L99-L103），避免阻塞异步执行器。闭包捕获快照和路径，恢复动作是外部 Git/工作区副作用。
   - 匹配阻塞任务结果（L105-L125）。`Ok(Ok(()))` 时删除历史中对应快照项，读取 reference context item，再以 `replace_history` 持久化替换；commit ID 截取前 7 个字符作为用户消息，记录 info，并置 `success=true`（L106-L114）。`Ok(Err(err))` 表示 Git 恢复返回业务错误，记录 warn，保持失败并把错误文本放入消息（L115-L119）。`Err(err)` 表示 blocking task join/panic 错误，记录 error，同样保持失败（L120-L124）。
   - 最后无论恢复成功或失败都发送一次 `UndoCompleted`（L127-L129）。本实现没有并发锁语义说明；历史替换依赖 `SessionTaskContext` 内部串行/锁定保证。取消只在恢复前检查，恢复开始后取消不会中断 Git 调用，也不会撤销已完成的工作区改变。

## 状态、取消、恢复与副作用

状态序列是“telemetry → UndoStarted →（取消/无快照/恢复结果）→ UndoCompleted”。成功路径同时改变工作区和会话历史；失败路径只写日志与完成事件。历史项先复制，只有 `restore_ghost_commit_with_options` 成功后才移除快照并调用 `replace_history`，因此 Git 失败不会丢失可重试的快照记录。没有显式超时、指数退避、自动重试或持久化事务；`spawn_blocking` 的 join 错误被转成失败消息。取消 token 是协作式且只做一次前置检查；任务运行中的取消、进程崩溃或 Git 部分副作用都没有在本文件中恢复。事件发送本身是异步副作用，发送失败不会改变后续分支。

## 源内测试与行为判据

源文件未包含测试模块或 `#[cfg(test)]`。可独立验证的判据：给定含多个快照的历史，必须选择最后一个 `GhostSnapshot`；无快照必须只发失败完成事件；预先取消必须发 started 后发 `Undo cancelled.` 且不调用 Git；Git 成功必须删除选中历史项并发 `success=true`、7 字符短 ID；Git 错误和 join 错误都必须发 `success=false`；所有路径的 `run` 返回 `None`，且完成事件最多一次。

## zenpi Rust 映射

zenpi 已有可复用的通信原语是：`src/protocol.rs` 的 `MailboxRequest`（Send/Receive/Acknowledge/Claim/Complete）、`MailboxOutcome`、`CheckpointRequest`、`StdioEvent`；`src/session.rs` 的 `SessionMailbox` 将消息按 recipient session、digest、TTL、sequence 持久化，并用 claim/complete 防止隐式重试；`SessionStore::append_event`/`append_turn`/`tree_snapshot` 可作为事件和 DAG 状态日志。对应 Undo 的 started/completed 事件，应采用带 `operation_id`、`node_id`、`parent_id`、`outcome` 的结构化事件，而不是仅依赖字符串消息。

DAG 节点通信建议新增 `src/dag.rs`（或 `session_tree.rs` 的扩展）：定义 `NodeId`、`NodeState::{Queued,Running,Green,Failed,Cancelled,Blocked,Closed}`、`NodeLinks{parent,grandparent,children}`、`DagMessage` 和 `DagEvent`。worker 负责节点时，通信路由必须允许向 parent、grandparent、直接 sibling、直接 child 发送；路由先由 `SessionMailbox` 做同 workspace 与 session 身份校验，再由 `protocol.rs` 校验 ID、TTL、正文上限。直接 sibling 可由共同 parent 的 child 集合解析，不能让客户端任意伪造 sender 身份。

会话父子关系可复用 `core.rs::Turn::parent_id`（L146-L180 附近的既有模型）保存对话父节点，但 DAG 拓扑不能只靠线性 turns 推导；应在 `SessionStore` 记录 `dag_node_created`、`dag_edge_added`、`dag_node_green`、`dag_close_requested`、`dag_worker_derived` 事件，并通过 `tree_snapshot/tree_ancestry_turns` 或新查询函数得到祖先、直接子节点和兄弟集合。grandparent 是 parent 的 parent，缺失时通信应返回明确的 not-found，而不是降级广播。

保活与派生的最小机制：在 `core.rs` 增加 `DagWorkerLease{node_id, lease_id, expires_at_ms, policy_digest}`、`renew_node_lease`、`cancel_node_worker`、`settle_node_worker`；借鉴现有 `settle_blueprint_worker`、`renew_blueprint_worker`、`cancel_blueprint_worker` 的 durable ledger 语义。节点只有在自身为 Green 且其全部直接/间接 child/grandchild 均为 Green（无 Running、Queued、Failed、Blocked、UnknownOutcome）时才允许 `close_node`；否则保持 lease/Running（或 WaitingChildren），并由 `derive_worker(node_id, work_item)` 产生带 parent 链接的新 worker。派生必须写入 durable intent 后再提交运行时，避免“事件已发但 worker 未创建”造成假绿。

落点对应关系如下：`src/runtime.rs` 的 `BackgroundRunner`、有界 mpsc、`CancellationToken`、`RuntimeEvent::{Accepted,Started,Completed,Closed}` 适合承载一个 DAG worker 的启动、取消、保活 tick 和终态；必须扩展为 worker/node correlation，且 bounded queue 满时保留 queued 状态。 `src/headless.rs` 已有 bounded event mailbox、replay/WAL、`mailbox_slash_view`，可作为跨会话消息与重连事件出口；新增 DAG 命令应走同一 `StdioRequest -> Command` admission，不能绕过 replay ledger。 `src/session.rs` 的 `SessionMailbox` 是消息/邮箱原语，`SessionStore` 是状态持久化与恢复点。 `src/core.rs` 的 `Agent`、worker budget、operation recovery 是 close/settle 的权威 owner；节点关闭检查应放在 Agent 或专门 DAG coordinator，不能交给 provider。 `src/tool_runtime.rs` 已规定批量工具的有界执行、先 prepare 后 dispatch、不中途隐式重试，可把每个 DAG work item 映射为一个受治理的 tool batch。 `src/protocol.rs` 应新增严格的 `DagRequest/DagAction`（send、status、renew、derive、close、cancel），复用 `MailboxOutcome` 与 checkpoint cursor。 `src/approval.rs` 的 `ApprovalCoordinator`、cancellation epoch、持久化后再放行可保护派生 worker 的副作用；取消必须使 lease 进入 cancelling，不能声称 child 已回收。 `src/providers/**` 及 `backend.rs` 的 `ProviderEvent`/stream cancellation 只负责模型流和工具调用，需把 provider cancel/断流映射为节点事件，不能直接将节点标绿。

差异清单（可执行、可验证）：(1) 增加 DAG 节点与边的持久化 schema 及恢复校验；(2) 增加按 parent/grandparent/sibling/child 的授权路由和 mailbox TTL/claim 测试；(3) 增加 close gate 的全后代递归判定，覆盖空 child、失败 child、UnknownOutcome、过期 lease；(4) 增加 derive intent→runtime admission→worker started 的幂等链；(5) 将 runtime 的取消、超时、shutdown grace 与 session 事件对齐；(6) 为 headless replay、provider 断流、approval 拒绝分别验证“保活而非误关闭”；(7) 用 session journal 重启恢复测试证明所有非 Green 后代都会阻止 close。

## 未决问题

无法从源文件确认：`SessionTaskContext::replace_history` 的事务/锁粒度；Git ghost commit 恢复是否原子；`send_event` 失败是否可观测；取消发生在 `spawn_blocking` 内时的具体行为；同一会话是否可能并发运行多个 `UndoTask`。这些点需要读取其定义或运行时测试才能定论。


# AC-027 — codex/codex-rs/core/src/tasks/compact.rs

- source_id/item_id：`codex/codex-rs/core/src/tasks/compact.rs` / `AC-027`
- source_path：`codex/codex-rs/core/src/tasks/compact.rs`
- source_hash：`696d43f6f62440a1e99375b650632db44db7750f65a76bdf3ceb7ac82b95d8f`
- source_bytes：`1442`
- source_lines：`49`
- coverage：已读取字节 `1-1442`、行 `L1-L49`，包括全部导入、注释、类型、派生属性、`SessionTask` 实现与结尾。

## 完整行为复盘

1. 文件级导入（`L1-L9`）：`Arc` 用于共享任务和会话；`SessionTask`、`SessionTaskContext` 来自同级模块；`TurnContext` 携带 provider；`TaskKind` 标识任务；`async_trait` 允许异步 trait 实现；`UserInput` 是压缩输入；`CancellationToken` 是任务取消令牌。导入本身不产生运行时副作用。
2. `CompactTask`（`L11-L12`）：`pub(crate)` 零大小任务标记类型，`#[derive(Clone, Copy, Default)]` 表示可复制、可拷贝且可默认构造。它不保存状态，因此并发调用之间没有实例级共享可变数据。
3. `impl SessionTask for CompactTask`（`L14-L49`）：该实现满足同级 `SessionTask` 的异步工作契约。
   - `kind(&self) -> TaskKind`（`L16-L18`）恒定返回 `TaskKind::Compact`，用于任务分类、UI/遥测和生命周期路由；无输入校验、无错误返回、无状态改变。
   - `span_name(&self) -> &'static str`（`L20-L22`）恒定返回字面量 `"session_task.compact"`，作为 tracing span 名称；默认值就是该字符串。
   - `run(self: Arc<Self>, session: Arc<SessionTaskContext>, ctx: Arc<TurnContext>, input: Vec<UserInput>, _cancellation_token: CancellationToken) -> Option<String>`（`L24-L30`）接收共享任务、会话上下文、turn 上下文、用户输入和取消令牌。参数名带下划线明确表示本函数不读取取消令牌。
     1. `let session = session.clone_session()`（`L31`）把轻量上下文转换成共享 `Arc<Session>`；后续两个分支都使用这个克隆，避免借用跨越 `.await`。
     2. `should_use_remote_compact_task(&ctx.provider)`（`L32`）只负责选择路径；该谓词定义在本文件之外，因此本文件不能证明哪些 provider 属于 remote。
     3. remote 分支先调用 `session.services.session_telemetry.counter("codex.task.compact", 1, [("type", "remote")])`（`L33-L37`），返回值被 `let _ =` 丢弃；随后以 `session.clone()` 和 `ctx` 调用并等待 `crate::compact_remote::run_remote_compact_task`（`L38`）。
     4. local 分支以同样方式记录 `("type", "local")`（`L40-L44`），然后把 `input` 传给 `crate::compact::run_compact_task(session.clone(), ctx, input).await`（`L45`）。输入只进入 local 调用；remote 分支没有转交 `input`。
     5. 整个条件表达式结果被 `let _ =` 丢弃（`L32-L46`），所以被调用函数的 `Result` 成功或失败都不会向上层传播；遥测计数失败也不会阻断压缩。函数最终无条件返回 `None`（`L47`），不提供最终 agent 消息。
   - 边界与并发：provider 判定为单次二选一；每次 `run` 只等待一个压缩 future，不在此处并行 remote/local。`Arc` 与 trait 的异步实现适合后台 task，但是否可取消、是否重试、是否持久化完全交给被调用函数和外层 session runner。

## 状态、取消、恢复与副作用

- 本文件没有字段状态、缓存或恢复游标；`CompactTask` 是无状态标记。唯一显式状态操作是克隆 session `Arc`（`L31`）。
- 取消：传入的 `CancellationToken` 被命名为 `_cancellation_token` 且从未传给 local/remote 调用（`L24-L30`、`L38`、`L45`）。因此本层不会主动响应取消、超时或中断；外层若取消 task，是否能停止取决于 delegated future 和 runner。`SessionTask` 的取消语义不能由此文件保证。
- 超时与重试：本文件没有 timeout、backoff 或重试循环。被调用函数内部可能有这些策略，但不在覆盖范围；其失败 `Result` 在此被吞掉。
- 持久化与外部副作用：本文件直接可见的外部副作用只有两次按分支打点的 `session_telemetry.counter`（`L33-L37`、`L40-L44`）。真正的上下文压缩、provider 请求、会话历史改写及事件发送由 `crate::compact`/`crate::compact_remote` 完成；本文件不检查提交结果，也不写持久化记录。失败被吞掉意味着外层可能看到 `None` 并继续统一收尾，存在“压缩失败但任务返回正常形状”的语义风险。
- 恢复/重入：无幂等键、操作 ID 或恢复逻辑；重复调用会再次选择 provider、再次计数并再次调用压缩函数。

## 源内测试与行为判据

源文件或同目录检索未包含针对 `CompactTask`、`session_task.compact` 或该实现的测试（源内未包含测试）。可独立验证的判据：构造可观测的 fake `TurnContext.provider` 与 `SessionTaskContext`，分别覆盖 remote/local 两条路径；断言对应 telemetry 标签各增加一次、只调用一个 delegated function、local 收到原始 `Vec<UserInput>`、remote 不收到该输入；让 delegated future 返回 `Err` 时断言 `run` 仍返回 `None`；预先取消 `CancellationToken` 后仍应观察到本函数未读取它。并以 `sha256sum` 与 `wc -l -c` 验证源文件为 49 行/1442 字节及给定哈希。

## zenpi Rust 映射

目标是把“压缩任务的路由/遥测/结果收束”扩展成 DAG worker 编排：worker 负责节点时可与 parent、grandparent、直接 sibling、直接 child 通信；只有节点自身及其全部 child/grandchild 均为 green 才能 close，否则保活并派生新 worker。

- 可复用通信原语：`src/protocol.rs` 的 `MailboxRequest::{Send,Receive,Acknowledge,Claim,Complete}`（`L91-L111`）已经是消息/邮箱/协议边界；`src/session.rs` 的 `SessionMailbox` 使用锁保护的 append-only journal，消息有 `sequence`、`digest`、TTL、claim/result 状态（`L3180-L3255`、`L3570-L3697`），适合把 DAG 边通信编码为 payload。`src/headless.rs` 将 `Command::Mailbox` 交给 session owner，忙时返回 `agent_busy`（`L6392-L6408`），可作为节点间发送/领取的 host 路由。实时进度可复用每请求的 `AsyncEventBuffer`（provider/agent 两个流，`L3925-L4050`），用于事件而非 durable mailbox。
- 通信关系应显式受图 ACL 约束：节点可向自己的 `parent_id`、沿父链解析出的 grandparent、同一 `parent_id` 的直接 sibling、以及 `parent_id == node_id` 的直接 child 发消息；接收端用 `Claim` 防止重复执行、`Complete` 回传结果，必要时沿 `sender_session_id` 建立 reply。Mailbox 的 session/workspace 校验（`src/session.rs:L3598-L3614`）应继续作为边界，不能仅凭客户端提供的 session ID 放开跨图通信。
- 会话父子关系：`src/core.rs::Turn` 的 `parent_id` 与 `Turn::with_parent`（`L140-L175`）是直接父关系；从父链递归即可得到 grandparent。直接 sibling 是相同 `parent_id` 的 turn 集合，直接 child 是 `parent_id == node_id` 的集合；`src/core.rs::selected_history`/`src/session.rs` 的树快照可作为查询入口。建议新增 `DagNodeId` 与 `DagEdge::{Parent,Child}`，不要把 sibling 关系复制存储，以免与 parent 链分叉。
- 保活与派生的最小机制：在 `src/session.rs` 增加 durable `DagNodeRecord`（node/parent/status/generation/lease/required_children/green）及 append-only transitions；在 `src/core.rs` 增加 `evaluate_dag_close(node)`：先要求 node 自身 terminal-green，再遍历所有 descendants（至少 child、grandchild，实际应递归全子树），全部 green 才返回 `Close`，否则返回 `KeepAlive{renew_lease}` 与 `SpawnWorker{new_worker_id}`。已有 `renew_blueprint_worker`、`cancel_blueprint_worker`、`settle_blueprint_worker` 可承载 lease、取消、终态结算（`L2823-L2885`）；settle 前应确认终态并回收 owned children，避免误关节点。
- `src/runtime.rs` 落点：`BackgroundRunner::spawn`、有界提交/事件队列与 `CancellationToken`（`L49-L99`、`L272-L315`）适合运行一个 DAG worker；用 child token 绑定节点生命周期，另设周期 heartbeat job。当前 runtime 只提供合作式取消，不能强杀非合作代码（`L51-L54`），因此 close 判定必须先持久化并确认子 worker 已回收。
- `src/headless.rs` 落点：扩展 `AsyncTurn`/命令分派，把 DAG `Send/Receive/Claim/Complete/Heartbeat/Close` 映射到 `Agent` 方法；沿用每请求 event mailbox，保持 provider/agent 事件相关性（`L4467-L4503`）。close/派生结果应产生可重放的 terminal response，避免 stdout 重连时丢失。
- `src/protocol.rs` 落点：可新增 `DagAction`/`DagRequest`，或在 `TreeAction`（`List/Select/Fork/Annotate`，`L1161-L1235`）旁增加节点状态与邻接查询；必须校验 node/session/lease/generation、目标关系和 TTL。`MailboxRequest` 的 `message_id`、`ttl_ms` 与 outcome 可直接复用。
- `src/tool_runtime.rs` 与 `src/approval.rs`：worker 派生若执行工具，继续走 `execute_tool_batch` 的取消谓词和并发上限，并由 `ApprovalCoordinator` 记录 host 决策；派生 worker 不应绕过 approval 或把 provider 响应当作授权。节点 green 只能由受控终态和子树检查产生。
- `src/providers/**`：现有 provider registry/connection 暴露 capabilities 与 route；建议添加 `CompactionStrategy::{Remote,Local}` 或 provider capability，而不是在 core 中硬编码 provider 名。由 `src/providers/connection.rs`/各 provider definition 选择策略，统一向 `core` 返回可观测的 compact result/event；当前目录没有与 Codex `run_remote_compact_task` 等价的压缩 API，需要新增 provider-neutral `compact_history` 接口。
- 可执行差异清单：①补齐 durable DAG 节点/lease/transition 记录；②实现 parent/grandparent/sibling/child 查询及全 descendants green 判定；③为 heartbeat、派生、close 定义幂等 operation ID；④把 mailbox claim/complete 与 worker budget settle 绑定，过期消息不得隐式重试（现有 mailbox 已拒绝该行为，`L3788-L3795`）；⑤在 headless 协议增加可重放状态事件；⑥为 remote/local compact adapter 传播 cancellation、错误和 telemetry，而不是像源文件一样无条件丢弃 `Result`；⑦补测试：并发 sibling 完成、grandchild 延迟、lease 过期、取消竞态、重复 message/operation 及重启恢复。

## 未决问题

1. `should_use_remote_compact_task` 的 provider 判定实现不在源文件内，无法仅凭本文件确认 remote 的准确条件。
2. `run_compact_task` 与 `run_remote_compact_task` 的压缩算法、provider 请求、内部重试、历史持久化和错误事件不在覆盖文件内。
3. 外层 `SessionTask` runner 如何解释被吞掉的 `Result`、何时允许 `None` 收尾，需结合同级 `tasks/mod.rs` 与 session 生命周期实现确认。
4. zenpi 当前 DAG 节点的“green”枚举、全 descendants 的最大深度/数量、派生预算与 heartbeat 周期尚未由本文件规定，需由产品协议补充。

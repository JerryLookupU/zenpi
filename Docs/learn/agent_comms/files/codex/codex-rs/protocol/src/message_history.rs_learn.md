# AC-037 — codex/codex-rs/protocol/src/message_history.rs

- source_id/item_id：`codex/codex-rs/protocol/src/message_history.rs` / `AC-037`
- source_path：`/Users/wangweiyang/GitHub/codex/codex-rs/protocol/src/message_history.rs`
- source_hash：`997e939050137d59e704bb0d5cb0868141584d46df8a6454635fbe9337c8cfc1`
- source_bytes：`252`
- source_lines：`11`
- coverage：已从字节 `1-252`、行 `L1-L11` 按顺序读完，包含全部注释、导入、类型和导出符号。

## 完整行为复盘

源文件没有自由函数、常量、模块级状态或测试；唯一公开导出符号是 `HistoryEntry`（`L6-L11`）。`L1-L4` 导入 `schemars::JsonSchema`、`serde::{Deserialize, Serialize}` 与 `ts_rs::TS`，它们只被派生宏使用，没有运行时初始化、I/O 或线程行为。`L6` 的 `#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, TS)]` 为该结构生成 JSON 序列化/反序列化、调试打印、按值克隆、JSON Schema 和 TypeScript 类型描述。派生宏没有在本文件声明字段默认值、重命名、未知字段策略或校验规则，因此序列化字段名就是 Rust 字段名，反序列化时三个字段均为必需；缺失字段、类型不匹配或无效 JSON 由派生的 `Deserialize` 返回错误，错误具体类型由 serde 生成实现决定。

`pub struct HistoryEntry`（`L7`）是一个可复制的历史消息值对象。`conversation_id: String`（`L8`）表示会话标识；输入输出均为拥有所有权的 UTF-8 字符串，源内没有空串、长度、控制字符或格式约束。`ts: u64`（`L9`）表示无符号时间数值；源内不规定单位、时区、零值含义或单调性，`0` 和最大 `u64` 都能进入结构，只要反序列化类型合法。`text: String`（`L10`）保存消息正文，同样没有大小、空白或内容限制。结构体结束于 `L11`。

构造方面没有 `new`、校验器或转换函数，调用者须直接用字段字面量或反序列化创建。输出方面，`Serialize` 可生成包含 `conversation_id`、`ts`、`text` 的对象；`JsonSchema`/`TS` 可供协议或前端生成契约；`Debug` 和 `Clone` 不改变状态。字段公开且没有内部可变状态，所以单个值的并发读取可由调用者安全安排；本类型自身不提供共享、锁、邮箱、顺序保证或并发写入语义。克隆只复制字符串和值，不连接会话，也不产生外部副作用。

边界与错误路径完全由派生实现及上层决定：无显式默认值、无 `Option`、无取消/超时分支、无重试策略；未知 JSON 字段默认是否接受取决于 serde 的默认容器行为（本结构没有 `deny_unknown_fields`）。时间戳和会话关系不会被自动推断，`HistoryEntry` 只是单条记录载体。

## 状态、取消、恢复与副作用

本文件没有全局或静态状态，`HistoryEntry` 实例只有三个字段；创建、克隆、序列化和反序列化都没有文件、网络、进程、provider 或工具副作用。没有取消令牌、超时、重试、幂等键或持久化钩子。若上层把它写入日志，持久化、崩溃恢复、去重和顺序必须由上层实现；`ts` 不是恢复游标，`conversation_id` 也不是父子关系证明。该类型不能单独表达“已完成/失败/取消/未知结果”，也不能表达消息发送者、接收者、确认或事件序列。

## 源内测试与行为判据

源内未包含测试。可独立验证的判据是：读取 `L1-L11` 后确认文件仅有一个三字段 `pub struct HistoryEntry`；对合法对象（例如 `{"conversation_id":"c1","ts":1,"text":"hi"}`）执行 serde 反序列化后三个字段保持原值，再序列化得到同名字段；缺失任一字段或把 `ts` 写成非整数应得到反序列化错误；`clone()` 与原值字段相等；生成的 JSON Schema/TS 声明应包含 `conversation_id: string`、`ts: number`（Rust `u64` 的目标语言表示由 `ts_rs` 配置决定）和 `text: string`。

## zenpi Rust 映射

**可复用通信原语。** `HistoryEntry` 可作为轻量消息载荷，放入 `src/protocol.rs` 的版本化 JSONL 协议；现有 `MailboxRequest::{Send,Receive,Acknowledge,Claim,Complete}`（`src/protocol.rs`）已经覆盖发送、分页接收、确认、租约式领取和完成结果，`src/session.rs::SessionMailbox` 负责加锁、追加日志、TTL、claim token 与幂等检查。事件流可复用 `src/runtime.rs::RuntimeEvent`（`Accepted/Started/Queued/CancelRequested/Completed/Closed`）及 `src/headless.rs` 的有界事件/replay mailbox；需要把 `HistoryEntry` 放进 `MailboxRequest::Send.text` 或新增结构化 payload 时，应保留 `conversation_id`、`ts`、`text` 三字段并在协议层增加大小和标识校验。协议入口是 `src/protocol.rs::StdioRequest::into_command`，因此通信不会绕过现有边界。

**会话父子关系与 DAG。** `src/core.rs::Turn` 已有 `parent_id: Option<String>`，`Turn::with_parent` 可保存直接父节点；`src/runtime.rs::InputBoundaryGate::with_context_parent` 可携带模型回合的上下文父标识；`src/session_tree.rs::SessionTree`/`src/session.rs` 已有可恢复树、active leaf、分支和 `TreeAction::{List,Select,Fork,Annotate}`。建议新增一个可持久化的 `DagNode`（建议落在 `src/session.rs` 或独立 `src/dag.rs`），至少含 `node_id`、`parent_id`、`grandparent_id`（可由 ancestry 推导）、`status`、`child_ids`、`worker_session_id`、`lease_id` 与 `green`/`close_requested` 状态；边关系以 `SessionTree` 的追加事件形式保存，避免只存在内存。直接 sibling 用同一 `parent_id` 查询，直接 child 用反向索引查询；grandparent 沿父链上溯一跳即可。`HistoryEntry.conversation_id` 应映射到 `worker_session_id` 或独立 `conversation_id`，不能假设它天然等于 DAG 节点 ID。

**最小保活与派生机制。** 复用 `src/session.rs::LiveSessionRegistry::{register,heartbeat,active,unregister}` 作为 worker liveness；复用 `SessionMailbox` 的 TTL 和 claim token 作为定向消息租约。建议在 `src/core.rs::Agent` 增加（或由新的 DAG owner 持有）`heartbeat_node(node_id, lease_id, now_ms)`、`send_node_message(target, HistoryEntry)`、`evaluate_close(node_id)`、`spawn_child(node_id, work)` 四个薄接口：`heartbeat_node` 更新 owner/lease；`send_node_message` 只允许 parent、grandparent、直接 sibling、直接 child 四类经关系校验的目标；`spawn_child` 先向 `src/session.rs` 追加 `node_spawned`/lease 事件，再调用 `src/runtime.rs::BackgroundRunner::try_submit`，成功后记录 worker session 与 operation ID。派生不得隐式重试未知副作用，沿用 `src/session.rs` 的 operation marker/unknown outcome 规则。

`evaluate_close` 的最小判据应是：当前节点自身状态为 `green`，且递归可达的全部 child/grandchild 均为 `green`、无活动 lease、无 queued/claimed mailbox 工作、无未结算 operation；只有此时才调用 `src/core.rs::Agent::try_close` 并写入 `node_closed`。任一后代非绿、过期但未收敛、取消未完成或结果未知时，节点保持保活，发送 heartbeat/状态事件，并通过 `spawn_child` 派生新 worker 处理新工作；派生结果必须重新进入同一 DAG，而不是覆盖旧节点。`src/runtime.rs::CancellationToken` 用于合作式取消，`BackgroundRunner` 的 bounded command/event channel 负责背压；超时只能触发取消和重新派生候选，不能把未知结果标成 green。

**既有文件落点对照。** `src/headless.rs` 适合实现 JSONL 命令分派、异步事件投影、关闭时 drain/replay，并调用 mailbox/tree owner；不要在 headless 层自行维护 DAG 真相。`src/core.rs` 适合保存节点状态机、父子关系校验、green 聚合和 close gate；`src/session.rs` 负责 append-only `node_*` 事件、恢复、`SessionMailbox`、`LiveSessionRegistry` 与 operation recovery；`src/tool_runtime.rs` 只承载 worker 调用工具时的批处理、并行上限和副作用策略，派生前仍需经过 approval/lease。`src/runtime.rs` 提供 `BackgroundRunner::spawn/try_submit/try_cancel/shutdown_and_join` 与 `CancellationToken`，是 worker 生命周期执行层。`src/protocol.rs` 增加 DAG/消息命令时应沿用 `MAX_ID_BYTES`、`MAX_MAILBOX_TEXT_BYTES`、TTL 和 `TreeRequest` 校验；`src/approval.rs` 的 `ApprovalRequest`/`WorkerExecutionBinding` 可绑定 `policy_digest`、`lease_id`、`expires_at_ms`，确保派生 worker 不扩大工具权限。`src/providers/**`（`openai.rs`、`anthropic.rs`、`google.rs`、`deepseek.rs`、`codex.rs`、`registry.rs`、`connection.rs`）只负责 provider 连接、能力和模型选择；它们不应直接决定 DAG close 或 worker 邻接通信，provider 事件应通过 runtime/headless event mailbox 回传。

**差异清单与可验证动作。** 源类型缺少消息方向、发送者、目标、序号、状态、父子边、租约和错误字段；zenpi 现有 mailbox 有这些执行语义但 payload 仍是文本/`Value`，需要新增结构化 DAG envelope 并写协议 round-trip 测试。源类型没有大小/ID/时间单位约束，zenpi 应在 `protocol.rs` 统一验证并拒绝空 ID、超长文本、零 TTL。源类型没有 close 聚合，需在 `core.rs` 增加后代状态折叠并用 session 事件恢复后重算。验证应覆盖：四类邻居可收发、非邻居被拒；heartbeat 过期后不能 claim；child 未绿时 parent close 被拒并可派生；全部后代绿且无活动 lease 时 close 只发生一次；取消、断线重启和未知 operation 不会错误标绿；JSONL 与 mailbox 事件可重放且序号单调。

## 未决问题

无法从源确认 `ts` 的单位/时区、`conversation_id` 的全局唯一性、未知字段兼容策略是否由上层统一设置，以及 TypeScript `u64` 的具体生成表示；这些必须由 zenpi 协议版本和调用方约定补充。

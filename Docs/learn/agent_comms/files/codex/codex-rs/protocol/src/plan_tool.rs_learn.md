# AC-038 — codex/codex-rs/protocol/src/plan_tool.rs

## 元信息

- source_id/item_id：`codex/codex-rs/protocol/src/plan_tool.rs` / `AC-038`
- source_path：`codex/codex-rs/protocol/src/plan_tool.rs`
- source_hash：`0a5482699b191fbeeb8f58790b62f6b331b64e29a38b3286a669aff4c699fdd9`
- source_bytes：`801`
- source_lines：`29`
- coverage：已从字节 `1-801`、行 `L1-L29` 顺序读完，包含全部注释、导入、derive、属性、类型和字段。

## 完整行为复盘

该文件只有协议数据类型，没有函数、构造器、线程、I/O 或工具执行逻辑。`L1-L4` 导入 `schemars::JsonSchema`、`serde::{Deserialize, Serialize}` 和 `ts_rs::TS`；这些 trait derive 使同一数据模型可用于 Rust 序列化/反序列化、JSON Schema 生成和 TypeScript 类型导出。

- `StepStatus`（`L7-L13`）是公开枚举，变体按源码顺序为 `Pending`、`InProgress`、`Completed`。`L8` 的 `#[serde(rename_all = "snake_case")]` 规定 JSON/TS 侧的 serde 名称为 `pending`、`in_progress`、`completed`；反序列化只接受对应枚举值，未知值、拼写错误或缺失值都会产生 serde 错误。`Debug`、`Clone`、`Serialize`、`Deserialize`、`JsonSchema`、`TS` 均为派生能力，未声明 `Copy`、`Eq` 或默认变体，因此不会自动复制，也没有默认状态。该类型本身没有并发语义，多个线程安全与否取决于外部是否以锁或消息传递共享它。
- `PlanItemArg`（`L15-L20`）是公开结构体，`L16` 的 `#[serde(deny_unknown_fields)]` 使 JSON 中出现 `step`、`status` 之外的键立即失败。`step: String` 与 `status: StepStatus` 都是必填字段，没有 `#[serde(default)]`，缺任何一个都失败；空字符串、超长文本和状态与 DAG 的关系均未在本文件验证。序列化输出字段名仍为 `step`、`status`，结构体可 clone、调试、生成 JSON Schema 与 TS 类型。输入是一个计划项，输出是同值的协议对象；没有转换、副作用或隐式排序。
- `UpdatePlanArgs`（`L22-L29`）是公开结构体，`L23` 同样拒绝未知键。`explanation: Option<String>`（`L25-L27`）带 `#[serde(default)]`：缺失时反序列化为 `None`，显式 `null` 也可得到 `None`；未加 `skip_serializing_if`，因此序列化 `None` 通常会写成 `"explanation": null`。有字符串时原样保留，没有长度、空白或内容校验。`plan: Vec<PlanItemArg>`（`L28-L29`）是必填数组；缺失时报错，空数组在本文件层面允许，数组项逐个受 `PlanItemArg` 的必填字段和未知字段规则约束。数组顺序被保留，不去重、不合并、不检查状态转移或父子关系。
- `L6` 注释说明这些类型用于 TODO 工具参数，并与 `codex-vscode/todo-mcp/src/main.rs` 对齐；这只是兼容意图，文件没有对应工具调用函数、返回值、错误枚举或执行器。所谓“输入/输出”仅指 serde 协议边界：成功得到类型实例或 JSON/Schema/TS 表示，失败返回库层解析错误。
- 并发方面，所有类型都是纯值对象；没有 `Arc`、锁、原子量、channel、异步 future 或取消 token。并发更新同一计划时的竞态、最后写入者、版本冲突和幂等性都由调用方决定。

## 状态、取消、恢复与副作用

`StepStatus` 只有静态标签，不实现状态机：没有从 `Pending` 到 `InProgress` 或 `Completed` 的合法转移表，也没有禁止回退的规则。文件没有取消、超时、重试、恢复、持久化或外部副作用代码；序列化只在调用方显式执行时发生。`explanation` 的默认值仅是缺省字段处理，不是重试原因或恢复标记。若同一 `UpdatePlanArgs` 被重复提交，本文件既不提供 request ID、revision、幂等键，也不判断是否重复。崩溃恢复、部分写入、并发写入和“完成”是否可关闭均必须由上层会话/运行时实现。

对 zenpi 的 DAG 需求，不能把 `Completed` 单独当成可关闭信号：节点自身及其全部 child/grandchild 都为 `Completed` 才能 close；任一后代仍为 `Pending`/`InProgress`、失败或未知结果时必须保活。保活应有可续租的 lease/heartbeat；发现新工作时应派生新 worker，并把派生关系写入持久会话树。该规则不是源文件行为，而是使用这些协议标签时必须补上的外层不变量。

## 源内测试与行为判据

源内未包含测试，也没有同目录测试模块或函数调用。可独立验证的判据如下：

1. 用 `serde_json` 反序列化 `{"step":"a","status":"in_progress"}` 为 `PlanItemArg` 应成功；加入 `"extra":1` 应失败（对应 `L15-L20`）。
2. 反序列化 `{"plan":[]}` 为 `UpdatePlanArgs` 应成功且 `explanation == None`；缺少 `plan` 应失败；缺少 `explanation` 不应失败（对应 `L22-L29`）。
3. `pending`、`in_progress`、`completed` 应分别映射到三个 `StepStatus` 变体；`"inProgress"` 或未知字符串应失败（对应 `L7-L13`）。
4. 序列化 `UpdatePlanArgs { explanation: None, plan: vec![] }` 应保留 `plan`，并按 serde 默认行为输出 `explanation: null`；这可检验 `L26-L27` 没有省略属性。
5. 生成 `JsonSchema` 与 `TS` 声明时，应看到三个状态值及 `PlanItemArg`/`UpdatePlanArgs` 的必填性和拒绝未知字段约束；具体文件名/导出配置不由本源确认。

## zenpi Rust 映射

建议把本文件的轻量协议语义嵌入已有模块，而不是复制一个孤立 TODO 状态：

- `src/protocol.rs`：现有 `MailboxOutcome`（`L79-L85`）、`MailboxRequest`（`L89-L111`）和 `Command::Mailbox`（`L211-L252`）已经提供消息/邮箱原语；`validate_mailbox`（`L544-L583`）及文本上限（`L585-L593`）提供边界。可新增 `PlanStatus`（snake_case）、`PlanItem`、`UpdatePlanRequest`，复用 `deny_unknown_fields`、ID/文本上限，并增加 `node_id`、`parent_id`、`lease_id`、`revision`。`Send/Receive/Claim/Complete` 可承载 worker 到 parent、grandparent、直接 sibling、直接 child 的定向消息；授权应由 mailbox owner 校验，不能让客户端自行伪造 sender。
- `src/session.rs`：`SessionStore` 是可恢复的追加式投影（`L145-L160`），已有 `SessionTree`；`selected_tree_turns` 会按当前叶子取祖先路径（`L1293-L1302`），并明确 sibling/descendant checkpoint 不应混入当前上下文（`L1326-L1335`）。建议添加持久 `DagNodeRecord { node_id, parent_id, status, children, lease_id, generation }` 与 `PlanEvent`，每次状态、派生、心跳、邮箱 claim/complete 都 append event。由 `parent_id` 计算 parent/grandparent；同一 `parent_id` 的节点是直接 sibling；children 表给出直接 child，递归遍历给出全部 child/grandchild。恢复时重放事件并拒绝旧 revision，避免崩溃后误判完成。
- `src/runtime.rs`：`BackgroundRunner` 的 `try_submit`、有界 `RuntimeConfig`（`L101-L115`、`L308-L315`）适合派生 worker 和限制待处理新工作；`CancellationToken` 是幂等取消原语（`L51-L63`），`JobOutcome` 明确 `Succeeded/Failed/Cancelled/Panicked`（`L156-L168`）。建议每个 DAG node worker 持有 `NodeLease` 和 `CancellationToken`，用 `try_submit(DeriveWorker { parent, work })` 派生，`QueueFull/Closed` 变成可观测事件而不是静默丢失；取消只停止计算，不回滚已发送消息或外部副作用。现有 `InputBoundaryGate::with_context_parent`（`L794-L830`）可复用上下文父关系，但需扩展为 DAG node identity。
- `src/headless.rs`：该模块拥有 JSONL wire I/O，并为请求提供相关响应、事件和重放；适合把 `PlanEvent`、邮箱回执、heartbeat/lease 状态作为带 request ID 的 `AgentEvent`。输入线解析、限流和 replay ledger 应保持现有边界；客户端断线恢复只重放已持久化事件，不能凭最后一条 stdout 推断节点已 close。
- `src/core.rs`：`Agent` 的 admission 边界区分 `Started`、`Steered`、`NotSubmitted`，拒绝输入不触达 backend；`Turn` 的 `parent_id` 是现成会话父子关联（约 `L176-L204`），`WorkerExecutionBinding` 提供 blueprint/goal/item/lease/policy 相关性。建议在 core 增加 `DagCoordinator`，将 `UpdatePlanArgs` 变成受验证的 plan intent，再由 `can_close(node)` 检查“自身 `Completed` 且递归全部后代 `Completed`”。不能 close 时发 heartbeat 并保留 lease；检测到新 work 时创建带新 `lease_id` 的 child worker。
- `src/tool_runtime.rs`：`ToolBatchOptions` 有并发上限（`L112-L120`），`execute_batch` 在取消时停止派发并使用有界 scoped worker（`L252-L298`）。DAG worker 的工具调用应把 `StepStatus` 更新、邮箱发送和派生意图放在可审计的 batch decision 之后；任何 child 的 `Cancelled/Failed/UnknownOutcome` 都使祖先保持活跃或进入人工恢复，而不是自动标为 `Completed`。
- `src/approval.rs`：`ApprovalCoordinator` 是 host-worker 的有界条件变量 rendezvous（`L126-L167`）；`persist_accepted` 要先持久化决定再释放等待 worker（`L370-L378`），失败则 retract、fail-closed（`L449-L458`）。涉及写文件、命令或派生 worker 的 DAG 操作应沿用 `ApprovalRequest` 的 `turn_id`/`lease_id`/policy digest，把 approval 记录写入 `SessionStore` 后才执行副作用。
- `src/providers/**`：`openai.rs`、`anthropic.rs`、`deepseek.rs`、`google.rs`、`codex.rs` 和 `connection.rs`/`registry.rs` 是 provider 适配边界，只负责流式响应、错误和连接；不要让 provider 自己决定 DAG close 或 sibling 路由。provider event 应转成 core 的进度事件，worker 在每个 chunk/重试点检查 `CancellationToken`，最终由 `DagCoordinator` 写 `Completed`。

最小可执行机制是四部分：`DagNodeRecord`（节点/父/孩子/状态/lease/revision）、定向 `MailboxRequest`（消息、claim、complete）、`PlanEvent` 追加日志（可恢复、可去重）、`LeaseHeartbeat` 加 `DeriveWorker` 命令。关闭判定伪代码应等价于：`self.status == Completed && descendants(node).all(|d| d.status == Completed)`；否则 `heartbeat(node.lease_id)`，若有新工作则 `derive(parent=node, work)`。直接 sibling 通过同 parent 查询，grandparent 通过两次 parent 查询，child 通过 children 查询，所有通信都带 `node_id`、`message_id`、`revision` 和 TTL，使用现有 mailbox 上限与 claim/ack 防止重复消费。

## 未决问题

- 源文件没有说明 TODO 工具的实际消费者、成功响应格式、状态更新是否全量替换 `plan`，也没有说明 TypeScript 导出配置和生成文件位置。
- 源文件没有定义 DAG 节点 ID、父子拓扑、失败/取消是否终态、lease 时长、heartbeat 周期或派生上限；这些必须由 zenpi 的新 `DagCoordinator` 与持久化策略定稿。
- `Option<String>` 序列化为 `null` 的具体 JSON 行为依赖 serde 默认实现；若 zenpi 需要省略字段，应显式加入 `skip_serializing_if`，不能从本文件推断为省略。

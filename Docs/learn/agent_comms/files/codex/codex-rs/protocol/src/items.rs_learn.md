# AC-035 — codex/codex-rs/protocol/src/items.rs

- source_id/item_id: `codex/codex-rs/protocol/src/items.rs` / `AC-035`
- source_path: `codex/codex-rs/protocol/src/items.rs`
- source_hash: `6a05493e2c334d722447ea5ac56fdb50a22a0a90733a12067c2e14b49a2e79f0`
- source_bytes: `9228`
- source_lines: `295`
- coverage: 已按顺序读取完整文件，字节范围 `0-9227`、行范围 `L1-L295`，包含全部注释、derive、类型、impl、导出方法。

## 完整行为复盘

文件先导入 `MemoryCitation`、`MessagePhase`、`WebSearchAction`、多个 `EventMsg` 事件类型、`UserInput`/`TextElement`/`ByteRange`，以及 `serde`、`schemars`、`ts_rs` 的序列化和 schema 工具（L1-L18）。所有数据类型都是 `Debug, Clone, Deserialize, Serialize, TS, JsonSchema`，因此可在 Rust、JSON Schema、TypeScript 三个边界复用；没有锁、原子量或异步 runtime，方法本身是同步的纯转换。

- `TurnItem` 是带 `type` 标签的总和枚举，变体为 `UserMessage`、`AgentMessage`、`Plan`、`Reasoning`、`WebSearch`、`ImageGeneration`、`ContextCompaction`（L20-L31）。输入是对应 item，输出由匹配分支决定；反序列化遇到未知或缺字段由 serde 返回错误。枚举只保存数据，不隐含取消、重试或并发顺序。
- `UserMessageItem { id, content }`（L33-L37）保存稳定 item ID 和有序 `Vec<UserInput>`；`AgentMessageContent` 当前只有 `Text { text }`（L39-L44）。空向量合法，是否为空由上层决定。
- `AgentMessageItem` 保存 `id`、有序文本内容（L46-L54），`phase: Option<MessagePhase>` 和 `memory_citation: Option<MemoryCitation>` 均默认 `None`、序列化时省略（L55-L65）。注释说明 `phase` 用于 TUI 区分中途 commentary 与最终答案、避免状态指示抖动；旧 provider 没有 phase 时必须保留旧完成语义。
- `PlanItem` 只有 `id` 与 `text`（L67-L71）；它能被持久化/传输，但后续 `TurnItem::as_legacy_events` 明确丢弃它，不产生旧事件（L284-L293）。
- `ReasoningItem` 保存 `summary_text: Vec<String>` 与默认空的 `raw_content`（L73-L79）；摘要总是可转换，原始推理是否外发由显式布尔参数控制。
- `WebSearchItem` 保存 `id/query/action`，并实现 `PartialEq`（L81-L86）；`ImageGenerationItem` 保存 `id/status/result`，可选 `revised_prompt/saved_path` 默认 `None` 且省略序列化，也实现 `PartialEq`（L88-L99）。这里不校验 URL、状态枚举或路径存在性。
- `ContextCompactionItem { id }`（L101-L104）。`new()` 使用 `uuid::Uuid::new_v4().to_string()` 生成非确定 ID（L106-L111）；`as_legacy_event()` 固定产生空载荷 `EventMsg::ContextCompacted(ContextCompactedEvent {})`（L113-L116）；`Default` 直接调用 `new()`（L118-L122），所以 default 每次都新 ID。
- `UserMessageItem::new(content)` 复制输入 slice，生成 UUID（L124-L130）；无内容限制或错误返回。`as_legacy_event()` 将 `message()`、`image_urls()`、`local_image_paths()`、`text_elements()` 填入 `UserMessageEvent`（L132-L141），图片字段即使为空也传 `Some(Vec::new())`。`message()` 只拼接 `UserInput::Text` 的 text，非文本变成空串，再无分隔符 join（L143-L152）；因此文本块边界不可从结果恢复，图片/本地图片不进入 message。`text_elements()` 只处理文本输入，按每块 `text.len()`（字节数）累加 offset，把块内 `ByteRange` 平移到拼接消息坐标，并通过 `placeholder(text).map(str::to_string)` 重建可选 placeholder（L154-L179）。它不会检查范围越界、UTF-8 字符边界或重叠；非文本不增加 offset，空内容返回空向量。`image_urls()` 按原顺序筛出 `UserInput::Image` 并 clone URL（L181-L189）；`local_image_paths()` 同理筛出 `LocalImage` 的 `PathBuf`（L191-L199），不访问文件系统。
- `AgentMessageItem::new(content)` 复制内容、生成 UUID，并把 `phase/memory_citation` 置为 `None`（L202-L210）。`as_legacy_events()` 对每个 `AgentMessageContent::Text` 产生一个 `EventMsg::AgentMessage`，复制文本，并把整个 item 的 phase 与 citation clone 到每个事件（L212-L224）；空 content 产生空 vec，当前单变体 match 没有其它错误路径。
- `ReasoningItem::as_legacy_events(show_raw_agent_reasoning)` 先按 `summary_text` 顺序产生 `AgentReasoning`，参数为 true 时再按 `raw_content` 顺序产生 `AgentReasoningRawContent`（L226-L247）。false 时 raw 完全不泄漏；空列表对应空 vec，没有截断、去重或并发语义。
- `WebSearchItem::as_legacy_event()` 将自身 id 作为 `call_id`，复制 query/action，产生 `WebSearchEnd`（L249-L257）；`ImageGenerationItem::as_legacy_event()` 同样把 id 映射为 call_id，复制 status、可选 revised_prompt、result、saved_path，产生 `ImageGenerationEnd`（L259-L269）。二者都不执行搜索、生成或保存动作。
- `TurnItem::id()` 对七个变体 clone 并返回其 id（L271-L282），所以调用者拿到的是独立 String。`TurnItem::as_legacy_events(show_raw_agent_reasoning)` 分派到各 item；User/Context/搜索/图像各包一条，Agent/Reasoning 可多条，Plan 返回空 vec（L284-L295）。转换是确定的（除构造时 UUID 已提前生成），不会修改 item；同一 item 可重复转换。

## 状态、取消、恢复与副作用

源码没有 cancellation token、deadline、超时、重试计数、持久化写入或线程同步；所有 `as_legacy_*`、聚合和筛选均为同步内存操作。唯一状态生成是 `new()` 的 UUID，不能据此实现幂等重放，重试应复用已持久化的 item/id。`ImageGenerationItem.saved_path` 只是字符串元数据，不代表文件已存在；`WebSearch`、图像生成、上下文压缩均不产生外部副作用。`phase=None`、`memory_citation=None`、`raw_content` 缺省为空、可选图像字段缺省为 None 是兼容旧生产者的默认值。真正的取消、恢复和审计必须在调用方（如 session/runtime/approval）完成，不能从本文件推断“成功”。

## 源内测试与行为判据

源内未包含测试（文件 L1-L295 没有 `#[cfg(test)]` 或测试模块）。可独立验证的判据：1）对七种 `TurnItem` 做 serde round-trip，`type` 标签和可选字段默认值保持一致；2）构造含文本、图片、本地图片的 `UserMessageItem`，检查 `message()` 只连接文本、`text_elements()` 的 byte range 等于各块 offset 后结果、两个图片筛选器保持相对顺序；3）`ReasoningItem` 在布尔参数 false/true 时分别不含/包含 raw 事件；4）Plan 的 legacy 事件为空；5）同一 item 的 legacy 转换不改变输入，重复调用结果相等；6）`ContextCompactionItem::default()` 的 id 非空且两次构造通常不同。边界判据应覆盖空 vectors、非 ASCII 文本、空 placeholder、可选字段为 None；越界 ByteRange 是否被拒绝属于上游 `UserInput/TextElement` 合约，本文件不保证。

## zenpi Rust 映射

可复用的抽象是“带类型标签的数据 item + 旧事件适配器”，而不是把 provider 副作用塞进 item。建议在 `src/protocol.rs` 增加 `DagItem`/`DagMessage`（serde `#[serde(tag="type")]`），字段至少含 `id: String`、`session_id`、`parent_id`、`grandparent_id`、`relation`（`Parent|Grandparent|Sibling|Child`）、`payload`、`created_at_ms`、`ttl_ms` 与 `status`；参考现有 `MailboxRequest::Send/Receive/Acknowledge/Claim/Complete`（`src/protocol.rs:L87-L113`）及 `Command::Mailbox`（L214-L263）。`MailboxRequest` 已有 recipient、message_id、TTL、游标、claim/complete/outcome，可直接作为 DAG 通信的线协议：发送方身份和 workspace 权限必须由 owner 注入，不能由客户端字段自报。

- **父子会话关系**：`src/core.rs` 的 `Turn { id, parent_id, ... }` 与 `with_parent/validate`（L140-L204）可复用为会话边；扩展 `SessionRelation { parent, grandparent, direct_sibling, direct_child }` 或在 `Turn.metadata` 写入受验证的 DAG 边。`src/session.rs` 的 `SessionStore::append_turn` 先校验、再写 journal、成功后才更新内存（L861-L891），适合记录 `DagNodeCreated`、`DagEdge`、`DagStatus` 和 worker lease。`tree_snapshot` 的取消检查与可恢复树读取（L894-L912）可作为祖先/后代闭包查询入口；建议新增 `descendant_status(node_id)`，返回直接 child 与 grandchild 的聚合状态。
- **最小通信原语**：同进程优先复用 `src/runtime.rs` 的 `BackgroundRunner`：有界 `sync_channel` 命令/事件邮箱、`try_submit`、`try_cancel`、`next_event`/`recv_timeout`、`Closed` 终态（L236-L365），每个 DAG worker 持有 `JobId` 和 `CancellationToken`。节点到 parent/grandparent/直接 sibling/直接 child 的消息统一走 `DagMailbox`（可由现有 `MailboxRequest` 落盘并由 runtime 投递）；事件层复用 `src/protocol.rs` 的 `StdioEvent { sequence, request_id, turn_id, event }`（L882-L918），使进度与终端响应可重放而不混淆。`src/headless.rs` 已有 `AsyncEventBuffer` 有界缓存、provider/agent 双队列和 `PendingSteer`（约 L806-L1081），可作为外部 host 的背压、steer 和丢失计数实现。
- **保活与派生的最小机制**：在建议的 `src/dag.rs` 定义 `DagNodeState::{Running, Green, Blocked, Failed, Closed}`、`DagNode { id, parent, children, state, worker: Option<JobId>, generation }` 和 `DagCoordinator::{send, claim, complete, evaluate_close, keep_alive, spawn_child}`。`evaluate_close(node)` 必须原子地检查“节点自身为 Green 且所有 child 与 grandchild 均 Green”；条件不满足时保持 `Running/Blocked`，写入 `KeepAlive` 事件并通过 `BackgroundRunner::try_submit` 派生新 worker（新 generation、同 parent、幂等 operation id），绝不能提前 `Closed`。派生前先把新工作写入 `SessionStore` journal，提交成功后再发布 `WorkerSpawned`；提交队列满返回 `QueueFull` 时保活状态仍有效，稍后重试。完成消息使用 `MailboxOutcome::{Succeeded, Failed, Abandoned}`（L79-L85），`Complete` 必须幂等且禁止旧 generation 覆盖新 generation。
- **核心落点**：`src/core.rs` 的 `AgentEvent` 已有 `TurnAccepted/AssistantMessage/Handoff/ToolCall/ToolResult/Provider/Error/Warning`（L281-L325），可增加 `DagMessage`、`DagStatus`、`WorkerSpawned`、`KeepAlive` 事件；`Agent` 现有 `active_turn_id`、`worker_budget`、`live_owners`（L403-L442）可承载节点租约、预算和 owner 检查。`Turn::parent_id` 可作为直接父边，但 grandparent/sibling/child 应从持久化图计算，避免仅凭客户端传入。
- **取消、工具与审批**：`src/tool_runtime.rs` 的批处理以 `AtomicBool` 停止、在并发 worker 间传播取消，并明确副作用未知时不能把取消当作回滚（约 L230-L346）；DAG worker 应沿同一规则把 `Cancelled` 记成 `UnknownOutcome/Abandoned`，保活而非误关。`src/approval.rs` 的 `ApprovalCoordinator` 是可复用的进程内 rendezvous：有界 Condvar 等待、`drain_pending/reveal/respond`，以及先持久化后释放 worker 的 `persist_accepted`（L126-L190、L248-L390）；DAG 派生 worker 的 side effect 必须绑定 node/generation/approval digest，`emergency_cancel/cancel_all`（L409-L467）要能让祖先取消传递到后代。
- **provider 对照**：`src/providers/**` 通过 `Backend`/`ProviderEvent`（`src/backend.rs:L238-L278,L495-L523`）向 `core` 流式产出文本和工具事件；不要让 provider 直接访问 DAG mailbox。应由 `core` 将 provider event 包装为带 `turn_id/node_id` 的 `DagMessage`，由 `runtime` 投递、由 `session` 持久化，保持 provider 可替换。
- **可执行验证**：新增单元/集成测试验证四种关系各能收发、TTL 过期转 `Abandoned`、重复 `message_id`/`Complete` 幂等、祖先取消能停止后代、队列满仍保活、节点/child/grandchild 全 Green 才 Closed、任一非 Green 都触发 `KeepAlive+spawn_child`；检查 journal 重放后图状态和 worker generation 不倒退。对照现有 `src/headless.rs` 的 bounded event mailbox、`src/session.rs` 的 append-before-project、`src/runtime.rs` 的 bounded command/event channels，确保没有无界 channel 或未审计副作用。

## 未决问题

无法从源文件确认 `UserInput` 的完整变体集合、`TextElement::placeholder` 对非法 byte range 的精确行为，以及 `MessagePhase`/`MemoryCitation` 的 JSON 兼容策略；这些需读取其定义后再定 zenpi 字段。源文件也没有 DAG、worker lease、parent/grandparent/sibling/child 图语义，因此上述 DAG 状态机和派生策略是基于 zenpi 需求的映射建议，不是源行为。

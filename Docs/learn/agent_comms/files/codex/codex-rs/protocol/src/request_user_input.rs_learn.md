# AC-040 — codex/codex-rs/protocol/src/request_user_input.rs

- source_id/item_id：`codex/codex-rs/protocol/src/request_user_input.rs` / `AC-040`
- source_path：`codex/codex-rs/protocol/src/request_user_input.rs`
- source_hash：`f617bec633ca8ccc9df9f5b9c567cddb93a366ed51cc16f9948496d23eb492a6`
- source_bytes：`1784`
- source_lines：`55`
- coverage：已按顺序读取完整字节范围 `0..=1783` 与行范围 `L1-L55`，包含全部注释、导入、derive、字段和导出符号。

## 完整行为复盘

该文件是协议数据模型文件，没有 `fn`、`impl`、构造器或运行时调度；全部行为由 Rust 类型、derive 和 serde/schema/TypeScript 属性提供。`L1` 导入 `HashMap`，`L3-L6` 导入 `JsonSchema`、serde 的 `Deserialize`/`Serialize` 以及 `TS`。五个公开结构体均派生 `Debug, Clone, Deserialize, Serialize, PartialEq, Eq, JsonSchema, TS`（`L8-L8`、`L14-L14`、`L31-L31`、`L36-L36`、`L46-L46`），因此可调试、复制、JSON 双向编解码、相等比较、生成 JSON Schema 和 TypeScript 类型；文件本身不提供线程安全或锁。

- `RequestUserInputQuestionOption`（`L8-L12`）：导出 `label: String` 与 `description: String`（`L9-L12`），反序列化时两个字段都必须能转成字符串，缺失/类型错误由 serde 返回错误；没有空串、长度、选项数量或唯一性检查。序列化总是输出这两个字段。`Clone` 只复制内存值，没有共享状态。
- `RequestUserInputQuestion`（`L14-L29`）：`id`、`header`、`question` 为必需 `String`（`L15-L18`）；`is_other: bool` 使用 `#[serde(rename = "isOther", default)]`（`L19-L22`），`is_secret: bool` 使用 `#[serde(rename = "isSecret", default)]`（`L23-L26`），所以输入缺失时默认 `false`，线上的键名是 camelCase；`schemars` 与 `ts_rs` 同样把两个 Rust 字段映射为 `isOther`/`isSecret`。`options: Option<Vec<RequestUserInputQuestionOption>>`（`L27-L29`）可缺失并成为 `None`，且 `None` 序列化时省略键；`Some(vec![])` 会保留为空数组。没有校验 `id` 是否唯一、`options` 是否必须、`isOther` 与选项的语义是否一致，也没有敏感字段遮蔽逻辑，`is_secret` 只是协议标记。
- `RequestUserInputArgs`（`L31-L34`）：只有 `questions: Vec<RequestUserInputQuestion>`（`L32-L34`）。缺失字段会导致 serde 反序列化失败；空数组合法，因为没有自定义校验。顺序由 `Vec` 保留，调用方应按数组顺序展示问题。
- `RequestUserInputAnswer`（`L36-L39`）：只有 `answers: Vec<String>`（`L37-L39`）。允许零个答案、多个答案及重复字符串；文件不把答案与问题数量或选项集合绑定，任何一致性检查必须在上层完成。
- `RequestUserInputResponse`（`L41-L44`）：`answers: HashMap<String, RequestUserInputAnswer>`（`L42-L44`），以问题 `id` 作为映射键的约定可由字段形状推断，但源文件没有强制键必须对应请求问题，也没有确定迭代顺序；JSON 对象键冲突由 serde/JSON 输入规则处理，业务未知键仍可进入内存。
- `RequestUserInputEvent`（`L46-L55`）：`call_id: String`（`L47-L50`）承载“可用时关联工具调用的 Responses API call id”（注释 `L48-L49`）；`turn_id: String`（`L50-L53`）承载所属 turn，`#[serde(default)]` 明确为向后兼容，旧消息缺失时得到空字符串；`questions: Vec<RequestUserInputQuestion>`（`L54-L55`）承载待回答问题。三个字段都参与序列化，`turn_id` 即使为空也会输出，因为没有 `skip_serializing_if`。

边界与错误路径仅来自派生编解码/schema/TS 生成：字段缺失、JSON 类型不匹配或非法 JSON 可失败；默认字段只有 `isOther`、`isSecret` 和 `turn_id`。没有显式错误枚举、超时、重试、取消、并发合并、持久化或外部副作用。derive 的 `Send/Sync` 没有在此声明，是否可跨线程取决于字段类型和调用方约束；这些结构体本身是不可变值对象，多个线程可各自持有 clone，但不会自动广播或同步回答。

## 状态、取消、恢复与副作用

源文件没有状态机；问题、答案、响应、事件都是一次性快照。没有 `CancellationToken`、deadline、超时字段或取消传播；没有重试计数、幂等键或持久化钩子；没有文件、网络、provider、shell 或工具副作用。`turn_id` 的 `serde(default)` 只解决旧载荷恢复，不代表 session 恢复语义。若上层等待用户输入期间取消，必须由宿主取消等待任务并决定是否记录未完成请求；本文件不会自动清理。

## 源内测试与行为判据

源文件及其同目录文件中未看到针对该文件的专门测试，源内未包含测试。可独立验证的判据：使用 serde 反序列化一题缺少 `isOther`、`isSecret`、`turn_id` 的 JSON，断言前三个默认 `false`、后者为空串；序列化 `options: None` 时断言不出现 `options`，而 `Some(vec![])` 出现空数组；Schema/TS 输出应出现 `isOther`、`isSecret`；`RequestUserInputResponse.answers` 应允许任意字符串键；`RequestUserInputArgs.questions` 和 `RequestUserInputAnswer.answers` 的数组顺序应保持不变；错误 JSON 或缺失必需字段应返回 serde 错误。

## zenpi Rust 映射

对照现有实现：`src/protocol.rs` 已有版本化 JSONL 边界、`MAX_ID_BYTES`/文本上限和 `MailboxRequest::{Send,Receive,Acknowledge,Claim,Complete}`（`src/protocol.rs:L20-L35,L79-L113`），并把它们纳入 `StdioRequest`/`Command`（`L152-L263`、`L265-L518`）；邮箱字段校验在 `validate_mailbox`（`L544-L598`）。这比源文件的纯 DTO 更严格，可复用为 DAG 节点通信协议。`src/session.rs` 的 `MailboxMessage` 保留 sender/recipient、request id、digest、TTL、sequence、status、claim token、result（`L3180-L3255`），`LiveSessionRegistry` 提供带 `owner_epoch` 的 register/heartbeat/active/unregister 与过期清理（`L3274-L3358`），`claim_next`/`claim_message`/`finish_claim`/`finish_claim_with_reply` 实现显式领取、完成和回信（`L3361-L3560`）；这是可靠 worker 邮箱的最小持久化原语。`src/core.rs` 的 `AgentEvent` 提供 Turn、Handoff、Tool、Provider、Error、Warning 事件（`L281-L325`），并暴露 live owner 与 mailbox 包装方法（`L821-L914`）。`src/headless.rs` 仅拥有 wire I/O、异步事件缓冲、请求相关响应与 runtime runner 接线（模块注释 `L1-L5`，事件队列相关 `L863-L1051`），适合把用户问题事件投影到 JSONL；不要把 DAG 判定塞进 I/O 层。`src/runtime.rs` 的 `BackgroundRunner` 使用有界 command/event channel，`RuntimeConfig` 默认容量为 32/128/32，`CancellationToken` 合作式取消（`L49-L193`、`L236-L421`），可直接作为派生 worker 调度器；`InputBoundaryGate` 具有 `context_parent`、epoch 失效和安全输入边界（`L765-L912`），可承载节点 turn 的父上下文。`src/tool_runtime.rs` 明确工具批次由 owner 负责策略、日志、结果持久化且不会隐式重试（`L1-L7`、`L157-L176`），所以 DAG 完成判定应在 owner/core 层，而不是工具线程。`src/approval.rs` 的 `ApprovalCoordinator` 是 bounded 条件变量 rendezvous，取消会清除 pending/decision（`L126-L240`），可复用作需要人工回答的节点门；`src/providers/**`（`providers/mod.rs:L1-L50`、`registry.rs:L146-L159`）只负责 provider protocol、route、model 定义，不应保存 DAG 拓扑或 worker 存活状态。

建议的可执行落点与最小机制如下：

1. 在 `src/protocol.rs` 增加与本文件同形的 `RequestUserInputQuestionOption`、`RequestUserInputQuestion`、`RequestUserInputArgs`、`RequestUserInputAnswer`、`RequestUserInputResponse`、`RequestUserInputEvent`，保留 `isOther`/`isSecret`/`turn_id` 的 serde 兼容规则；新增 `DagMessage`/`DagEvent` 的 `node_id`、`parent_id`、`recipient_session_id`、`message_id`、`ttl_ms`、`kind`、payload，并在 `Command` 中加入 `DagSend/DagReceive/DagComplete/DagSpawn/DagClose`。复用现有 `MailboxRequest` 的边界校验，禁止从客户端字段推断 sender 身份。
2. 在 `src/session.rs` 以 session journal + `.mailbox` 作为持久化通信。节点会话用 `SessionTree::TreeEntry.parent_id`/`Turn.parent_id` 建立 parent 关系；为 DAG 增加一份受限的 `DagNodeRecord { node_id, parent_node_id, child_ids, direct_sibling_ids, state, generation, worker_session_id }`。父、grandparent、直接 sibling、直接 child 的寻址应先由拓扑索引解析为 session id，再通过 `SessionMailbox::enqueue`/`MailboxAction` 发送；不要通过扫描文件名判断在线。
3. 在 `src/core.rs` 增加 `DagNodeState`（`Pending/Running/Waiting/Green/Failed/Cancelled/Closed`）、`DagNodeSnapshot` 和 `DagCoordinator`。`DagCoordinator::can_close(node_id)` 必须同时检查节点自身为 `Green` 且所有直接/间接 child、grandchild 的聚合状态为 `Green`；任一未完成、失败、取消或未知状态都拒绝 close，并发出 `AgentEvent::Warning`/结构化 `DagEvent::CloseDeferred`。`close` 应写入 session 事件后再改变内存状态，保证重启可重放。
4. 保活最小机制：注册 worker 时调用 `Agent::register_live_owner(owner_epoch, workspace, now_ms)`，每个 heartbeat 周期调用 `heartbeat_live_owner`；`active(session_id, workspace, now, ttl)` 过期即视为不可接收。worker 退出调用 `unregister_live_owner`。在 `DagNodeRecord` 中保存 `last_heartbeat_ms`、`lease_id`/generation，拒绝旧 epoch 的 mailbox claim；这复用 `LiveSessionRegistry` 的所有权防重放语义。
5. 派生最小机制：`DagCoordinator::spawn_child` 先把 parent-child 边和派生意图 append 到 journal，再用 `BackgroundRunner::try_submit` 提交 `{node_id, context_parent, work}`；向 parent、grandparent、direct sibling、direct child 的结果均走有界 mailbox，发送失败保留可重试的 durable intent。`RuntimeEvent::{Accepted,Started,Queued,Completed,Rejected,Closed}` 作为 worker 生命周期事件，`CancellationToken` 只作合作式取消；取消/关闭不能声称已回滚外部副作用。
6. `src/headless.rs` 负责把 `DagEvent`、`RequestUserInputEvent` 和 mailbox 结果编码为相关 JSONL，并利用现有有界异步事件缓冲；`src/runtime.rs` 负责线程与队列；`src/approval.rs` 负责需要用户回答的 `is_secret`/工具批准；`src/tool_runtime.rs` 负责节点工具批次；`src/providers/**` 仅生成模型请求/流事件。DAG close 条件、拓扑寻址、保活与派生不能下沉到 provider。

差异清单：源文件只有问题/答案 DTO，没有 sender/recipient、TTL、claim、拓扑、状态聚合、取消、重试或错误类型；zenpi 已有 durable mailbox、owner heartbeat、session tree、bounded runtime 和 approval，但缺少“祖先/同胞/后代寻址索引”和“全后代绿才能关闭”的原子聚合规则。还需为输入问题事件补充 request id/幂等键、答案校验策略和过期后的明确 `Abandoned` 结果；这些应在 protocol/session/core 三层分别验证、持久化、决策。

## 未决问题

无法从源确认：`call_id` 缺失时上层应如何关联工具调用；`id` 是否必须唯一及其命名规则；`isOther`、`isSecret` 的 UI/安全语义；答案数量、选项合法性和未知答案键是否应拒绝；`turn_id` 为空时恢复器应丢弃、补写还是归入匿名 turn；请求用户输入的超时、取消结果和持久化格式。上述均需由 zenpi 的上层协议与产品策略明确。

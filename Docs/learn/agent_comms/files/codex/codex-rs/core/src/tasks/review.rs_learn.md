# AC-031 — codex/codex-rs/core/src/tasks/review.rs

- source_id/item_id: `codex/codex-rs/core/src/tasks/review.rs` / `AC-031`
- source_path: `codex/codex-rs/core/src/tasks/review.rs`
- source_hash: `72bbbd98e7a6e95e25c9f411b28225f76e8f921e511bed3d0b41b43a39cf81ba`
- source_bytes: `9687`
- source_lines: `273`
- coverage: 已按文件顺序读取完整字节范围 `0-9687`、行范围 `L1-L273`（含注释、导入、类型、导出符号与函数体）。

## 完整行为复盘

源文件导入 `Arc`、`async_trait`、`CancellationToken`，并使用 `Event/EventMsg`、`TurnItem`、`ReviewOutputEvent`、`SubAgentSource`、`Session`、`TurnContext`、`run_codex_thread_one_shot` 等类型作为事件桥和子代理启动接口（`L1-L30`）。

- `ReviewTask` 是 `Clone + Copy` 的无状态任务标记（`L32-L34`）。`ReviewTask::new()` 只返回 `Self`，没有配置或资源副作用（`L35-L39`）。
- `SessionTask for ReviewTask::kind()` 固定返回 `TaskKind::Review`；`span_name()` 固定为 `"session_task.review"`，用于任务分类和 tracing（`L41-L49`）。
- `ReviewTask::run(self: Arc<Self>, session, ctx, input, cancellation_token) -> Option<String>` 先递增 `codex.task.review` telemetry counter；计数失败被忽略（`L51-L63`）。随后调用 `start_review_conversation`；成功得到事件接收器就交给 `process_review_events`，启动失败则得到 `None`（`L64-L75`）。若 token 未取消，调用 `exit_review_mode` 发出退出审查模式结果；无论结果为何，当前实现最终都返回 `None`，所以返回值不是审查文本通道（`L76-L80`）。取消是合作式的：函数不会强杀子任务，且取消时跳过正常退出事件。并发上 `Arc<SessionTaskContext>`/`Arc<TurnContext>` 可共享，事件消费在单个异步循环中串行进行。
- `ReviewTask::abort(session, ctx)` 无条件调用 `exit_review_mode(..., None, ...)`，将中止表现为“审查被打断”的终态（`L82-L85`）。
- `start_review_conversation(session, ctx, input, cancellation_token) -> Option<async_channel::Receiver<Event>>` 克隆当前配置后建立隔离的子代理配置（`L87-L94`）。它强制 `web_search_mode = Disabled`；按类型约束该设置失败是构造错误，因此直接 `panic!`（`L95-L102`）。同时禁用 `Feature::SpawnCsv` 与 `Feature::Collab`，覆盖审查专用安全边界（`L103-L105`）；设置 `REVIEW_PROMPT`，并把 `approval_policy` 设为 `AskForApproval::Never`（`L106-L109`）。模型优先取 `config.review_model`，缺省回退到 `ctx.model_info.slug`（`L110-L114`）。调用 `run_codex_thread_one_shot` 时透传认证、模型管理器、输入、会话、上下文、取消 token，标记来源 `SubAgentSource::Review`，无 JSON schema、无初始历史（`L115-L126`）；调用错误被 `.ok()` 丢弃，成功才映射为 `rx_event`（`L127-L130`）。因此错误路径与取消启动失败都归并为 `None`，没有自动重试。
- `process_review_events(session, ctx, receiver) -> Option<ReviewOutputEvent>` 逐个异步接收事件，维护一个 `prev_agent_message` 缓冲（`L132-L138`）。遇到新的 `EventMsg::AgentMessage` 时，先把上一条完整 agent 消息转发给父会话，再缓存当前消息（`L139-L148`），从而避免把正在形成的最后一条消息以旧式事件重复暴露。`ItemCompleted(AgentMessage)`、`AgentMessageDelta`、`AgentMessageContentDelta` 被抑制，避免 `as_legacy_events()` 产生隐藏的 legacy `AgentMessage`（`L149-L157`）。`TurnComplete` 从 `last_agent_message`（若有）调用 `parse_review_output_event` 并立即返回；这是成功终止条件（`L158-L165`）。`TurnAborted` 直接返回 `None`，交由调用者以中断结果收尾（`L166-L169`）。其它事件按原样异步 `send_event` 到父会话（`L170-L175`）。接收通道提前关闭且没有 `TurnComplete` 同样返回 `None`（`L177-L180`）。这意味着事件转发与解析由一个消费者串行化，发送背压由 `send_event` 的异步等待承担。
- `parse_review_output_event(text: &str) -> ReviewOutputEvent` 先尝试把整个文本反序列化为目标 JSON（`L182-L190`）；失败后取首个 `{` 到末个 `}` 的闭区间，且要求 `start < end`、切片合法，再尝试反序列化（`L191-L197`）。两次都失败时返回 `Default` 结构，仅将原文放入 `overall_explanation`，因此纯文本不会丢失且 findings 为空（`L198-L202`）。边界包括没有花括号、空对象、嵌套/前后 Markdown；使用首尾截取可能把多个 JSON 拼接成不可解析片段。
- `exit_review_mode(session: Arc<Session>, review_output: Option<ReviewOutputEvent>, ctx)` 是公开给 crate 的终态落点（`L204-L210`）。成功结果分支把 `overall_explanation.trim()` 和 `format_review_findings_block(findings, None)` 拼成结果，再套 `REVIEW_EXIT_SUCCESS_TMPL`；assistant 展示文本来自 `render_review_output_text`（`L211-L226`）。`None` 分支使用中断模板，并生成要求重新运行 `/review` 的固定 assistant 文本（`L227-L233`）。随后以固定 ID `review_rollout_user`/`review_rollout_assistant` 追加 user `ResponseItem::Message`（`L235-L245`）和 assistant response item；中间先发送 `EventMsg::ExitedReviewMode { review_output }`（`L248-L267`）。最后调用 `ensure_rollout_materialized()`，确保即使没有普通 user turn 也持久化 rollout；刻意放在面向客户端的输出之后，避免文件创建/git 元数据拖慢可见事件（`L269-L273`）。这些写入是外部副作用，重复调用可能重复固定 ID 记录，幂等性由下层会话实现决定。

## 状态、取消、恢复与副作用

任务状态由父级 `SessionTask` 调度器和子代理事件共同决定；本文件没有本地状态机、超时计时器或重试计数。`CancellationToken` 只在启动、事件流的 `TurnAborted`、`run` 的退出判定处被观察（`L51-L80`、`L166-L180`）；取消不会回滚已转发事件、模型调用或已写入会话。通道关闭、one-shot 错误、审查 JSON 解析失败都降级为 `None`，其中解析失败仍保留纯文本解释（`L182-L202`）。`abort` 明确写入中断消息；正常完成写入结构化审查结果。持久化包括 user/assistant 两条 `ResponseItem`、`ExitedReviewMode` 事件以及 rollout materialization（`L235-L273`）；没有本地 checkpoint 或恢复游标，重试语义是由用户重新执行 `/review`。工具副作用通过子代理配置被限制：禁用 web search、`SpawnCsv`、`Collab`，且审批模式为 `Never`（`L95-L109`）。

## 源内测试与行为判据

源文件没有 `#[test]` 或测试模块，源内未包含测试。可独立验证的判据：输入完整合法 `ReviewOutputEvent` JSON 时应得到等价结构；输入带前后 Markdown 的单个 JSON 对象时应解析花括号子串；普通文本应生成 `overall_explanation` 且 `findings` 使用默认值（`L187-L202`）。事件流应抑制三类 delta/assistant-completed 事件、转发其它事件、在 `TurnComplete` 返回结果、在 `TurnAborted` 或 EOF 返回 `None`（`L132-L180`）。正常结果必须观察到 `ExitedReviewMode`、固定 user/assistant 记录和最终 materialization；取消/abort 必须观察到中断模板（`L204-L273`）。

## zenpi Rust 映射

zenpi 已有的最小复用面与建议落点如下。

- **DAG 拓扑与会话父子关系**：`src/dag.rs:L1-L10` 已明确“一个 worker 持有一个 DAG 节点、可与 parent/grandparent/direct sibling/direct child 通信、仅全 descendant green 才能 close”。`DagNode` 的 `parent/children/status/worker` 字段在 `L57-L73`，`DagStore::recipients` 在 `L279-L347` 精确覆盖 `parent`、`grandparent`、`siblings`、`children`、`all`；因此 review 子代理的会话关系应把 DAG node id 作为 worker identity，把 `DagNode.parent` 作为会话父级，把 children 列表作为直接子级，grandparent/sibling 通过只读拓扑解析而非复制树。
- **消息/邮箱/事件原语**：`DagStore::send`/`inbox`/`wait_for_message`（`dag.rs:L349-L384`、`L547-L565`）适合节点间短消息和轮询取消；它们有 `MAX_DAG_MESSAGES`、`MAX_DAG_BODY_BYTES` 上限。跨进程、需恢复的请求则复用 `src/protocol.rs:L81-L117` 的 `MailboxRequest::{Send,Receive,Acknowledge,Claim,Complete}` 与 `L544-L585` 的 ID、TTL、正文校验，避免另造 review 协议。实时父会话事件沿 `src/core.rs:L281-L325` 的 `AgentEvent`（`AssistantMessage`、`Provider`、`Error` 等）并由 `src/headless.rs:L9145-L9200` 的事件投影/回放通道输出。
- **耐久邮箱与 claim/完成**：`src/session.rs:L3171-L3272` 的 `MailboxMessage`、状态和 `MailboxAction` 是可复用的请求-结果协议；`SessionMailbox::enqueue/list/update` 位于 `L3640-L3743`，`LiveSessionRegistry` 位于 `L3274-L3359`。`src/headless.rs:L8954-L9093` 已实现 recipient 定位、TTL enqueue、receive、live-owner claim、带 reply 的 complete。DAG worker 向 parent、grandparent、sibling、child 发消息时，应将 `DagStore::recipients` 解析出的每个目标映射为目标 `session_id`，短期通知走 `DagStore`，需跨重启交付走 `SessionMailbox`；结果必须带原 `request_id`/digest。
- **最小保活与派生机制**：用 `DagStore::can_close`（`dag.rs:L395-L425`）作为唯一 close gate：节点自身状态必须为 `green`，深度遍历的每个 child/grandchild 也必须 `green`；返回 unfinished 列表时，worker 保持 `open`/活跃，读取 inbox 并为新工作调用 `spawn_worker`（`dag.rs:L446-L496`），后续工作通过保持打开的 stdin 的 `send_to_worker`（`L518-L533`）发送。`spawn_worker` 已设置 `ZENPI_DAG_NODE`/`ZENPI_DAG_STORE` 并使用 headless session；这就是“保活并派生新 worker”的最小可执行机制。close 成功后才调用 `Agent::close`/终端事件，不得仅因本节点 green 提前关闭。
- **`src/headless.rs` 落点**：异步宿主在 `L4467-L4550` 使用 `runtime::BackgroundRunner` 提交 `Agent::submit_with_cancel`；邮箱命令分发在 `L6392-L6400`，同步 mailbox 执行在 `L8954-L9093`。建议增加 `Command::DagSend/DagStatus/DagSpawn/DagClose` 的薄分发，复用 `dag.rs`，并让 `write_events` 的回放序列承载 `DagMessage`/worker lifecycle 事件。审查式事件过滤可仿照 `process_review_events`，只把最终结构化 review 结果和非增量生命周期事件暴露给客户端。
- **`src/core.rs` 落点**：`Turn` 的 `parent_id` 与 `Turn::with_parent` 在 `L133-L204` 可承载会话父子关联；`Agent` 状态机和 `AgentEvent` 在 `L402-L442`、提交/执行 API 在 `L3242-L3368`、可取消运行在 `L3662-L3695`。建议新增 `DagWorkerContext { node_id, parent_session_id, child_ids, cancellation, lease }`，在 `Agent` 的 active turn 元数据中保存 node id；`settle_blueprint_worker`/`renew_blueprint_worker`/`cancel_blueprint_worker`（`L2827-L2899`）可作为派生 worker 的预算、保活 lease 和终止接口。`WorkerExecutionBinding`（`L41-L82`）继续约束 worker 的工具权限，不能把 DAG 拓扑身份当成审批授权。
- **`src/session.rs` 落点**：已有 append-only journal、`append_event`（`L1207-L1223`）、operation recovery（`L1414-L1590`）和 mailbox sidecar；建议将 `dag_node_id`、`parent_node_id`、`worker_epoch`、`close_check` 作为有界事件写入当前 session journal，恢复时重建 worker lease 与未完成消息。保留 sidecar 的 digest、TTL、claim token 和 workspace 校验，避免把 stdin/文件名当作身份。
- **`src/runtime.rs` 落点**：`CancellationToken` 在 `L49-L99` 是幂等合作式取消；`BackgroundRunner` 的 bounded command/event channel、FIFO pending、`RuntimeEvent::{Accepted,Started,CancelRequested,Completed,Closed}` 在 `L101-L188`、`L279-L365`、`L443-L684` 可直接承载 review worker 与 DAG 子 worker。建议每个 DAG node 一个 `JobId`，`Completed` 只表示该 node 本次工作结束；是否 DAG close 仍由 `DagStore::can_close` 决定。shutdown grace 不能回滚 provider/tool 副作用。
- **`src/tool_runtime.rs` 落点**：`execute_tool_batch`（`L157-L180`）已规定有界批量、顺序持久化、取消谓词、不中途重试；把 review 子代理视为受限 worker 时，复用 `ToolBatchOptions` 的并发上限并在每次派生/重试前检查 cancellation。工具执行结果不是 worker terminal outcome，必须等 node 及 descendants 都 green 后由 DAG 层结算。
- **`src/approval.rs` 落点**：`ApprovalMode::Never` 与 `WorkerAllowAfterPreflight` 在 `L20-L38`，`ApprovalRequest` 的 `policy_digest/lease_id` 校验在 `L40-L104`，`ApprovalCoordinator` 是带取消谓词的进程内邮箱/条件变量（`L126-L190`）。映射 review 的“禁止重新启用工具”时，应为子 worker 绑定不可变 policy digest 与 lease；DAG parent/sibling 消息不能直接授予工具权限，任何 side effect 仍要经过 host-owned approval/preflight。
- **`src/providers/**` 落点**：`providers::Protocol` 在 `providers/mod.rs:L13-L60` 统一 wire protocol；`connection.rs:L18-L60` 的 `ValidatedRoute` 固化 provider/model/能力/route digest；`registry.rs:L74-L110` 的 `ModelDescriptor` 校验能力与 reasoning。OpenAI、Anthropic、Codex、DeepSeek、Google 的静态 route 分别见各文件 `L4-L27` 或 `L7-L68`。`start_review_conversation` 的 `review_model` 回退逻辑应在 zenpi 配置层选择一个已由 `ModelRegistry` 验证的 model descriptor，再由 provider connection 生成 route；不能让 DAG worker 从消息正文自行改 endpoint、凭据或能力。

差异清单与可验证动作：zenpi 已有 DAG/邮箱/取消基础设施，但 review 专用的“事件抑制 + 最后一条消息结构化解析 + success/interrupted 双模板”仍需在 headless/core 的 worker adapter 中实现；补齐后可用单元测试验证 `recipients` 的五种目标、`can_close` 的 descendant gate、`MailboxMessage` 的 claim/TTL、取消后不再派生，以及仅在全绿时发出 close 事件。短期消息容量、邮箱 TTL、worker lease 和 provider route digest 都必须被纳入测试断言。

## 未决问题

无法从源文件确认：`SessionTaskContext::send_event` 是否保证顺序持久化、`run_codex_thread_one_shot` 在取消时是否仍可能产生尾部事件、`ReviewOutputEvent` 的完整字段集合及 `Default` 的具体值、`ensure_rollout_materialized` 的幂等策略，以及重复固定消息 ID 时底层 journal 的去重规则。源文件也没有定义 review worker 的超时或自动重试策略；这些行为需查调用方和协议实现后才能确定。

# AC-007 — claude-code/plugins/hookify/agents/conversation-analyzer.md

- source_id/item_id: `AC-007`
- source_path: `claude-code/plugins/hookify/agents/conversation-analyzer.md`
- source_hash: `535ec8acb55b51bfac83a54f5f5c70746a52adb7a0b785e9be0e376c7e2eb12c`
- source_bytes: `5478`
- source_lines: `176`
- coverage: 已从字节 `0-5477`、行 `L1-L176` 按文件顺序完整读取，包含 frontmatter、注释性说明、示例、规则模板与边界条件。

## 完整行为复盘

这是一个给 `/hookify` 使用的提示型 agent 定义，而非可执行程序；源内没有函数、Rust 类型或语言级导出符号。frontmatter 的可见配置符号是 `name: conversation-analyzer`（L1-L3）、`model: inherit`（L4）、`color: yellow`（L5）和只读工具集合 `tools: ["Read", "Grep"]`（L6）。`description` 将触发条件限定为：无参数运行 `/hookify`，或用户要求回看对话并为错误创建 hooks（L3）。因此默认模型继承调用方，默认颜色为黄色，默认能力只有读取和 grep；没有写文件、执行命令或联网能力。

角色总职责是从 Claude Code 会话中识别可由 hooks 预防的问题行为（L9-L16）：读取用户消息找挫败信号，定位工具使用模式，抽出可正则匹配的模式，按严重度/类型分类，并输出可生成规则的结构化结果。输入是已有会话转录及其中的用户、工具行为；输出是结构化文本，不是直接写入 hook 配置。源没有规定转录缺失、不可读或工具失败时的异常类型，因此这些都只能由宿主处理。

分析顺序明确为用户消息逆时间顺序，最新消息优先（L18-L23）。候选问题包括四类：明确禁止/纠正（如 `Don't use X`、`Never...`，L24-L30）；挫败反应（质问、澄清“不是我的意思”、判定错误，L31-L36）；撤销、修复和逐步纠正（L37-L40）；以及重复发生、需要多次提醒的同类错误（L42-L45）。边界是消息中的信号必须与实际行为关联；单纯陈述偏好或假设不应被误判。

对每个候选问题，分析器必须确定四个字段：具体工具（`Bash`、`Edit`、`Write`、`MultiEdit`）、动作/命令或代码模式、发生时的任务阶段、以及用户明示或隐含的危害理由（L47-L53）。随后要保留可核查的实际例子：Bash 留真实命令，Edit/Write 留加入的代码，`Stop` 留停止前缺失的内容（L55-L58）。这构成输入证据到规则模式的最小链路。

正则抽取按对象分为三组：危险 shell 命令如 `rm\\s+-rf`、`sudo\\s+`、`chmod\\s+777`（L60-L67）；代码风险如 `console\\.log\\(`、`eval\\(|new Function\\(`、`innerHTML\\s*=`（L69-L72）；路径风险如 `\\.env$`、`/node_modules/`、`dist/|build/`（L74-L77）。模式应能匹配动作本身而非宽泛主题；没有匹配证据时不得臆造规则。

严重度是默认决策框架：High 应在未来阻断，覆盖 `rm -rf`、`chmod 777`、硬编码 secret、`eval` 与数据丢失风险（L79-L85）；Medium 只警告，覆盖生产代码 `console.log`、编辑生成文件和缺少最佳实践（L86-L89）；Low 可选，覆盖编码风格等非关键偏好（L91-L93）。源未规定同一行为跨级时的冲突算法；可按最高风险并保留证据计数处理。

输出必须遵循 `## Hookify Analysis Results`，逐问题给出标题、`Severity`、`Tool`、`Pattern`、`Occurrences`、`Context`、`User Reaction`（L95-L109）。每个问题还要给 `Suggested Rule` 的 `Name`、`Event`、`Pattern`、`Message`（L110-L115）；示例把 Bash 绑定到 `bash` 事件、文件编辑绑定到 `file` 事件（L118-L130）。问题之间用分隔线继续，最后输出 `## Summary`、总数及 High/Medium/Low 计数，并建议为 High 与 Medium 创建规则（L132-L144）。`{N}` 是待替换计数占位语义，不能原样作为最终结果。

质量判据是具体而不过宽、带真实对话例子、解释危害、给可直接使用的 regex，并避免把“不要做 X”的讨论误报为已发生行为（L146-L151）。边界处理有四项：假设问题（例如询问 `rm -rf` 后果）不算行为（L153-L157）；教学说明“不应这样做”不算问题（L159-L161）；一次性且已修复的事故可以提及但降为 Low（L163-L165）；主观偏好标 Low，由用户决定（L167-L169）。最终结果交给 `/hookify`：展示发现、询问选哪些规则、生成 `.local.md`，并保存到 `.claude`（L171-L176）。这些是外部命令的后续副作用，分析器自身没有写入动作。

并发语义：源只给出逆序扫描和只读 `Read`/`Grep`，未声明并发读取、锁、竞态或多分析任务合并规则；因此实现应默认为单次快照、确定性顺序，宿主若并发调用需自行隔离会话与结果。

## 状态、取消、恢复与副作用

源文件没有显式状态机、取消 token、超时、重试、断点恢复或持久化字段；也没有错误返回协议。分析状态可抽象为“读取转录 → 候选问题 → 工具/动作证据 → regex → 严重度 → 结构化结果”，但这是行为推导，不是源内类型（L18-L23、L47-L62、L79-L97）。工具失败、空转录、无问题和重复问题的默认表现未明示；可独立验证的安全默认是输出 `Found 0 behaviors`，而不是生成规则。

分析本身应无外部副作用，因为能力仅为 `Read`、`Grep`（L4-L6）。真正的副作用发生在后续 `/hookify` 流程：用户选择后才生成 `.local.md` 并保存 `.claude`（L171-L176）。源未说明选择前是否写缓存，也未说明重新分析是否幂等；实现时应以会话快照哈希和确定性排序保证可重放。没有取消/超时约定，宿主只能在读取或 grep 边界自行中断。

## 源内测试与行为判据

源内未包含测试。可独立验证的行为判据如下：

1. 给出包含 3 个最新到最旧用户消息的转录，最新消息明确要求“不要用 X”，结果必须先处理它，并能关联对应工具证据（L20-L23、L47-L58）。
2. 出现 `rm -rf`、`console.log(`、`.env` 时，分别能得到可匹配模式、工具/事件和严重度；计数与上下文必须来自实际出现次数（L64-L77、L97-L130）。
3. 只出现“如果我用 `rm -rf` 会怎样？”或教学句“不要这样做”时，不得生成 High 规则（L153-L161）。
4. 一次已修复事故标 Low，主观“偏好 X”标 Low；重复提醒提高证据权重（L42-L45、L163-L169）。
5. 最终文本包含规定标题、每个字段、Summary 计数和高/中严重度建议，且不产生文件写入（L95-L115、L136-L144、L171-L176）。

## zenpi Rust 映射

源 agent 没有 DAG 语义；以下是把“行为发现/规则建议”与 zenpi 的会话编排能力结合起来，并满足 DAG worker 通信要求的可执行落点。

- **通信原语复用**：优先复用 `protocol::MailboxRequest` 的 `Send/Receive/Acknowledge/Claim/Complete`（`src/protocol.rs:L81-L113`）和 JSONL `Command::Mailbox`（L211-L263）。底层用 `session::SessionMailbox` 的 `MailboxMessage`、digest、TTL、claim token 和结果状态（`src/session.rs:L3180-L3242`、L3563-L3679）；它是本地有界、可寻址、可持久化邮箱，不应另造无认证 channel。短生命周期进度继续用 `core::AgentEvent` 的 `Provider/ToolProgress/Warning/Error`（`src/core.rs:L281-L325`），通过 `headless.rs` 的有界 agent/provider event mailbox（约 L4014-L4065）输出。`Handoff` 可作为跨节点摘要，session 的 `append_handoff`/`append_event` 负责审计（`src/session.rs:L1142-L1207`）。
- **会话父子关系**：`core::Turn.parent_id` 与 `Turn::with_parent` 已表达同一会话内的直接父子输入（`src/core.rs:L140-L175`）；`steer_existing` 会以 active turn 为 parent 写入新 user turn（L3429-L3497）。DAG 节点应增加持久化 `DagNode { node_id, session_id, parent_node_id, edge_kind, status, generation }`，其中 `parent_node_id` 指向直接 parent；祖父通过重复 parent 查询，直接 sibling 是同 parent 的其它 child，直接 child 是反向索引。记录应写 `SessionStore::append_event` 或新 `session_tree` 事件，不能只放内存。
- **parent/grandparent/sibling/child 通信**：为每个节点把目标 `session_id` 和 `node_id` 放入 mailbox payload，并在 `MailboxMessage` 的 sender/recipient/request digest 外增加协议版本、`correlation_id`、`edge_kind`、`reply_to`。路由层先查 parent，再沿两级 parent 找 grandparent；同 parent 的 node 列表得到直接 sibling；child 列表得到直接 child。所有跨节点请求都走 `LiveSessionRegistry::claim_next/claim_message`，完成走 `finish_claim` 或 `finish_claim_with_reply`（`src/session.rs:L3274-L3453`），避免绕过 workspace、owner epoch 和 claim token 校验。
- **保活的最小机制**：注册节点时调用 `LiveSessionRegistry::register`，周期调用 `heartbeat`，以 `active(..., ttl_ms)` 判定在线并清理过期 owner（`src/session.rs:L3285-L3355`）。每个 worker 持有 `lease_id/owner_epoch`，将 `last_seen_ms` 和当前 DAG 状态写入事件；邮箱发送使用 `MAX_MAILBOX_TTL_MS` 上限（`src/protocol.rs:L34-L35`），过期只产生 `Expired` 视图，不伪造执行回执（`src/session.rs:L3170-L3178`、L3245-L3255）。
- **派生的最小机制**：`LiveSessionRegistry` 明确“只做 admission，不启动 scheduler”（`src/session.rs:L3274-L3276`），所以新增 `DagCoordinator`（可放 `src/runtime.rs` 或新 `src/dag.rs`）接收 `spawn_child` mailbox 事件：验证父节点 lease/approval，创建 child `SessionStore`/`DagNode`，调用 `BackgroundRunner::try_submit`。`runtime::BackgroundRunner` 的 `CancellationToken`、有界 command/event 队列和 FIFO follow-up 可直接承载 worker（`src/runtime.rs:L49-L121`、L156-L199、L236-L343）。派生必须产生新 `JobId`、generation 和 parent correlation；不可在 mailbox claim 内隐式 fork，因为源 mailbox 设计要求 claimed message 不自动重试（`src/session.rs:L3570-L3573`）。
- **节点关闭判据**：新增 `DagCoordinator::close_if_green(node_id)`，原子检查：自身状态为 `Green`、无未完成 tool/approval operation、直接 child 集合非空时每个 child 为 `Green`，并递归确认所有 descendant（grandchild 等）为 `Green`，同时无未确认 mailbox/claim 和有效 child lease；满足才写 `node_closed` 事件并释放 owner。任一 descendant 非 Green、超时或新工作到达时，保持 `Running/KeepAlive`，发送 parent/sibling 通知并 enqueue 一个新的 child worker。现有 `SessionTree`/`tree_snapshot` 可提供 ancestry/branch 投影（`src/session.rs:L895-L1069`），但其本身没有“全 descendant green”聚合，必须增加可验证的 descendant index 和事务性状态事件。
- **取消、重试与运行落点**：`runtime::CancellationToken::cancel/is_cancelled/mark_completed` 保证协作取消及 late-cancel 不改写已完成结果（`src/runtime.rs:L49-L99`）；`headless.rs` 已有 pending steer 上限、shutdown grace 和 cancel/reissue 路径（约 L39-L57、L1064-L1099、L4560-L4626），可复用为 DAG worker 的保活重派生边界。重试只允许对未 claim 或明确 `Abandoned` 的消息；已 claim 的消息须读 operation journal 决定恢复，不能盲重放。`tool_runtime.rs` 的 `execute_tool_batch`、side-effect policy 和 cancellation 参数（`src/tool_runtime.rs:L133-L175`、L275-L442）承接节点工具执行，避免分析建议绕过审批。
- **协议、审批与 provider 落点**：`protocol.rs` 的 `StdioRequest` 已有 `mailbox`、`tree`、`checkpoint`、`session_id` 字段及版本/大小校验（L152-L209、L265-L312、L527-L592），可加 `DagAction::{Status,Spawn,KeepAlive,Close}`，并保留 bounded IDs/text/TTL。`approval.rs::ApprovalCoordinator` 的 `request_response`、`drain_pending`、取消和 `mark_persisted`（`src/approval.rs:L126-L245`、L248-L260、L393-L445）作为 spawn/close 的 side-effect gate；`WorkerExecutionBinding` 的 lease/policy digest 校验位于 `core.rs:L37-L81`，可绑定每个派生 worker。provider 层保持现有同步 `Backend` 边界和 `ProviderEvent` 流（`src/backend.rs:L236-L276`、L492-L531）；`src/providers/**` 只负责具体路由/协议，不应自行决定 DAG close 或派生。

差异清单（可执行、可验证）：

1. 已有：有界 JSONL、邮箱、TTL、claim/complete、heartbeat、父 turn、事件流、协作取消、审批和 session journal。验证：调用 `MailboxRequest::Send` 后能以 digest claim/complete，并在 owner TTL 过期时拒绝 claim。
2. 缺少：统一 `DagNode`/edge schema、grandparent/sibling/descendant 查询、`Green` 聚合 close gate、KeepAlive 状态和 `spawn_child` 命令。实现：新增模块与 session 事件，写属性测试确保任何非 Green descendant 都不能 close。
3. 缺少：派生调度器。实现：`DagCoordinator` 消费 mailbox 后再向 `BackgroundRunner` 提交，验证每次派生有唯一 `JobId`、父 correlation 和可恢复 journal。
4. 缺少：跨节点权限模型。实现：把 workspace、owner epoch、lease、`WorkerExecutionBinding`、approval policy digest 一并校验；验证 sibling 不能冒充 parent，旧 epoch 不能完成新 claim。
5. 缺少：分析器结果与 DAG 工作项的协议。实现：定义 `HookifyFinding`（pattern、severity、tool、occurrences、evidence）作为 mailbox payload，`core::AgentEvent::Warning` 只传进度，最终 finding 写 session event；验证假设/教学文本不会派生 worker。

## 未决问题

1. 源未定义空转录、读取失败、正则编译失败、重复问题合并和并发分析时的错误/排序协议。
2. 源未定义 hook 事件枚举与 `.local.md` 的精确 schema，也未说明 `/hookify` 是否在用户确认前持久化中间结果。
3. 源未定义 DAG、parent/grandparent/sibling/child、保活、派生或“全 descendant green 才能 close”；上述 Rust 类型与 close gate 均为满足 zenpi 需求的明确设计建议，不是源文件事实。
4. `LiveSessionRegistry` 当前只提供在线 owner admission，不负责 scheduler；派生进程边界、崩溃恢复和跨机器传输仍需产品级决策。

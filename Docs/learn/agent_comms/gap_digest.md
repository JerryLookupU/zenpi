# agent_comms learn — gap digest (54/54)

## AC-001 claude-code/examples/hooks/bash_command_validator_example.py
这段 Python 的可迁移核心是“在副作用边界前做纯校验，使用结构化诊断和明确阻断结果”。在 zenpi 中建议把规则实现为 `ToolRegistry`/`SideEffectPolicy` 的纯 preflight，而不是放进 `src/providers/**`；provider（`src/providers/anthropic.rs`、`openai.rs`、`codex.rs`、`google.rs`、`deepseek.rs` 及 `registry.rs`）只负责请求/流事件，不能绕过统一门。

- `src/protocol.rs` 已提供版本化 JSONL、`StdioRequest`/`Command`、`StdioResponse`/`StdioEvent`（协议骨架在 `L1-L37`、请求字段在 `L152-L209`，`parse_line`/编码器在 `L795-L1000`）。可增加 `Command::DagMessage` 或复用现有 `MailboxRequest::{Send,Receive,Acknowledge,Claim,Complete}`（`L81-L113`）传递校验结果、心跳和派生请求。建议 Rust 类型为 `DagRecipient::{Parent,Grandparent,Sibling(NodeId),Child(NodeId)}`、`DagEnvelope { message_id, sender, recipient, ttl_ms, payload }`；解析时复用 `MAX_ID_BYTES`、文本/TTL 边界和 `ProtocolError`，验证失败返回带稳定 code 的 `StdioResponse::error_with_code`，对应 Python 的退出 `1/2`。
- DAG 通信原语的最小实现是：每个节点一个有界 mailbox（`VecDeque` 或 `mpsc::sync_channel`），一个事件流（`StdioEvent`/`RuntimeEvent`），以及一个带 `message_id`、父会话/节点 ID、序号、TTL 的协议 envelope。recipient resolver 维护 `parent` 与 `children` 索引：parent 直接寻址；grandparent 先取 `parent.parent`；direct sibling 通过同一 parent 的 children 集合过滤自身；direct child 直接遍历 children。邮箱操作按 `Send → Claim → Complete/Acknowledge` 建立幂等边界，过期消息产出 `MailboxOutcome::Abandoned`，避免无限重试。
- `src/core.rs` 已有 `WorkerExecutionBinding`（`L41-L80`）将 `goal_id`、`item_id`、`lease_id`、策略摘要和过期时间绑定到 worker；`Turn` 的 `parent_id`（`L140-L175`）可承载会话父子关系。建议新增 `DagNodeState { node_id, session_id, parent_session_id, status, children, lease_deadline }` 与 `DagStatus::{Running,Green,Failed,Cancelled,Waiting}`，并让每个派生 worker 的 `Turn::with_parent` 指向父 turn/session。grandparent 和 sibling 关系必须由持久化图索引解析，不能仅靠 provider prompt 文本。
- `src/session.rs` 是追加式 JSONL 恢复边界：`SessionStore`、`OperationKind/OperationOutcome` 和 `RuntimeIntent` 记录（`L49-L95`、`L117-L160`）适合记录 `dag_node_created`、`dag_message_sent/claimed/comple

## AC-002 claude-code/examples/settings/README.md
- `src/protocol.rs` 是通信协议落点。可复用 `MailboxRequest::{Send, Receive, Acknowledge, Claim, Complete}`、`MailboxOutcome`、`StdioRequest.mailbox`、`Command::Mailbox`，沿用 `MAX_MAILBOX_TEXT_BYTES`、`MAX_MAILBOX_TTL_MS`、`MAX_ID_BYTES` 及严格 `validate_*`。把 DAG 通信编码成 `MessageEnvelope { message_id, sender_node, recipient_node, relation, payload, ttl_ms, parent_id }`；`relation` 至少取 `Parent`、`Grandparent`、`DirectSibling`、`DirectChild`、`Control`，让 worker 能精确寻址 parent、grandparent、直接 sibling 和直接 child。`TreeAction::{List, Select, Fork, Annotate}` 可承载树观察/派生请求，但不能替代 mailbox 的投递确认。
- `src/session.rs` 已有持久通信原语：`SessionMailbox` 的 `enqueue/list/update`、`MailboxMessage` 的 digest/sequence/status、`LiveSessionRegistry::{register, heartbeat, active, claim_next}`，以及 `MailboxStatus::{Queued, Acknowledged, Claimed, Succeeded, Failed, Expired}`。建议在同一 session journal 增加 `DagNodeRecord`（`node_id,parent_id,status,generation,lease_id,expected_children`）和 `DagTransition`（spawn、heartbeat、green、close、reopen）；用现有 append-only 记录保证崩溃恢复，用 digest + `request_id` 保证重试幂等。会话父子关系必须持久化：`parent_id` 指向直接 parent，向上两次得到 grandparent；同一 `parent_id` 的 active records 是直接 siblings；以该节点为 `parent_id` 的 records 是直接 children，递归查询得到 grandchildren。当前 `Turn.parent_id` 只表达对话 turn 关系，不能直接充当 DAG 节点关系，应避免混用。
- `src/runtime.rs` 提供最小执行机制：`BackgroundRunner::spawn`、有界 `try_submit`、`try_cancel`、`CancellationToken`、`RuntimeEvent::{Accepted,Started,Queued,CancelRequested,Completed,Closed}`、`JobOutcome` 和 bounded shutdown grace。每个派生 worker 应是一个带 `JobId` 的 runtime job；派生动作先落 `spawn` 记录再 `try_submit`，队列满返回 `SubmitError::QueueFull` 并保持父节点 lease；关闭前必须检查 DAG 聚合状态。`CancellationToken` 只取消执行，不把未完成后代伪装成 green。
- `src/headless.rs` 是宿主/编排落点。现有 `run_async_streams`、`AsyncEventBuffer`、`EventMailboxStream`、`handle_runtime_event` 和 bounded event/terminal replay 可复用为节点事件出口

## AC-003 claude-code/plugins/agent-sdk-dev/README.md
**可复用原语。** README 的“命令询问→创建→验证→报告”可落到 `src/protocol.rs` 的 `StdioRequest`/`Command`/`StdioResponse`/`StdioEvent`：输入用有界 JSONL 请求，过程事件用 `StdioEvent` 的 `sequence`、`request_id`、`turn_id` 关联，终态用 `StdioResponse` 的 `success`/`error_code`（`StdioEvent::new` 与响应构造在 `src/protocol.rs:L882-L999`）。直接通信优先复用 `src/session.rs` 的 `SessionMailbox`、`MailboxMessage`、`MailboxAction` 和 `MailboxStatus`（`L3170-L3265`、`L3570-L3775`）：digest 寻址、TTL、claim token、幂等 request ID、`Queued → Claimed → Succeeded/Failed` 正好适合 agent 节点消息/邮箱；实时进度用 `src/headless.rs` 的有界 provider/agent event mailbox 与 replay（`MAX_ASYNC_*` 在 `L33-L52`，异步事件排放在约 `L4251-L4395`）。协议层事件是观察通道，邮箱是可重放的命令/结果通道，二者不可混为一个终端结果。

**DAG 会话父子关系和地址。** 在 `src/session.rs` 的 `SessionStore` 记录上新增/复用一个 DAG metadata 结构（建议字段 `node_id`、`parent_session_id`、`grandparent_session_id`、`relation`、`child_ids`、`status`、`generation`），节点身份继续使用 session ID；`src/core.rs` 的 `Turn.parent_id`/`Turn::with_parent`（`L140-L175`）可保存一次 turn 的直接因果父项，但不能单独表达全部 DAG。发送方把 `recipient_session_id` 指向 parent、grandparent、直接 sibling 或直接 child，关系和 `goal_id/item_id` 放进受验证的 JSON payload；`SessionMailbox::check_access` 已要求双方是同一 workspace 且 addressed session（`src/session.rs:L3598-L3614`），可作为跨节点授权边界。`src/protocol.rs::MailboxRequest`（`L89-L113`）提供 Send/Receive/Acknowledge/Claim/Complete；可扩展一个受 `deny_unknown_fields` 保护的 `relation`，而不允许客户端伪造 sender identity（该身份刻意不在 client fields，L87-L89）。

**保活、派生和 close 的最小可执行机制。** `src/session.rs::LiveSessionRegistry` 已有 `register`、`heartbeat`、`active`、`unregister`、owner epoch 和 TTL（`L3274-L3359`），可作为每个 DAG worker 的 lease/保活；`claim_next`/`claim_message`/`finish_claim`/`finish_claim_with_reply`（`L3361-L3559`）可作为邮箱消费和结果回送。建议新增一个纯函数 `dag_closeable(node, graph) -> bool`：仅当自身 `status == Green` 且递归遍历的全部 child/grandchild 都为 `Green` 才返回 true；存在 pending/running/failed/unk

## AC-004 claude-code/plugins/agent-sdk-dev/agents/agent-sdk-verifier-py.md
现有代码已经提供可复用的通信原语。跨进程/跨会话优先复用 `src/protocol.rs` 的 `MailboxRequest::{Send,Receive,Acknowledge,Claim,Complete}`、`MailboxOutcome`、带 `session_id` 的请求字段以及版本化 JSONL；该协议已有消息大小、TTL、ID 校验（对应 `src/protocol.rs` L1-L110、L520-L560）。持久邮箱复用 `src/session.rs` 的 `MailboxMessage`、`MailboxStatus`、`SessionMailbox::{enqueue,list,update}` 和 `LiveSessionRegistry::{register,heartbeat,claim_next,claim_message,finish_claim,finish_claim_with_reply}`（约 L3171-L3570、L3574-L3798）；它提供 digest、TTL、claim token、幂等完成/失败和同 workspace 约束。进程内事件/响应复用 `src/headless.rs` 的有界 replay/event mailbox 与请求 ID 相关性；不要另造无限队列。工具/审批事件可复用 `src/approval.rs` 的 `ApprovalCoordinator`、`ApprovalRequest` 与取消广播（`cancel_all`），只把它当策略闸门，不当 DAG 拓扑存储。provider 流式增量继续走 `src/providers/**` 的 `ProviderEvent`/streaming 能力，并由宿主转成节点事件。

建议的会话关系模型：在 `src/session.rs` 的 durable record 或现有 tree/handoff 记录中增加受校验的 `DagNodeContext`（`node_id`、`parent_session_id`、`grandparent_session_id`、直接 sibling 会话 ID 列表、直接 child 会话 ID 列表、`generation`、`required_descendant_count`）；在 `src/core.rs` 的 `Agent` 建立/校验上下文，并让 `Turn.parent_id`（约 L146-L193）保留会话内父子链。跨 session 关系必须同时写进 handoff/tree 事件，避免仅凭内存指针推断祖先。

DAG 节点通信的最小可执行机制如下：节点启动时由 `src/core.rs` 注册 `LiveSessionOwner`（已有 `register_live_owner`、`heartbeat_live_owner`、`claim_live_mailbox`、`finish_live_mailbox`，约 L820-L875），为 parent、grandparent、每个直接 sibling、每个直接 child 各发送一个带 `message_id`/TTL/拓扑 epoch 的 mailbox envelope；接收端按 `claim_message` 原子认领，处理后 `finish_claim_with_reply` 回原发送者。消息类型至少包括 `NodeStarted`、`Progress`、`Green`、`Blocked`、`CloseRequest`、`SpawnRequest`、`Cancel`，payload 必须携带 `node_id`、`generation`、依赖摘要和幂等键。这样 parent/grandparent/sibling/child 全都使用同一 mailbox 原语，事件只用于本进程实时唤醒，持久 mailbox 才是恢复依据。

保活与派生的最小机制：`src/runtime.rs` 的 `BackgroundRunner::try_submit/try_cancel/try_shutdown_with_grace`、`RuntimeEvent::{Accepted,Started,Queu

## AC-005 claude-code/plugins/agent-sdk-dev/agents/agent-sdk-verifier-ts.md
该规格可作为 zenpi 的“worker verifier/审查策略”，而不是直接搬运 TypeScript API。现有落点与差异如下。

- **输入/输出与协议：** `src/protocol.rs` 已有 `MailboxRequest` 的 `Send/Receive/Acknowledge/Claim/Complete`，带 `recipient_session_id`、`message_id`、`text`、`ttl_ms` 和 `MailboxOutcome`（L81-L113）；`StdioRequest` 与 `Command` 把 JSONL 输入解析为有界命令（L152-L240），`parse_line`/响应编码在 L795-L803、L857-L995。可复用原语是“消息 + 有界协议 + 事件响应”，建议新增 `DagMessage`/`DagEvent` 只作为已有 `MailboxRequest`/`StdioEvent` 的受限 payload，避免另造无界通道。
- **邮箱实现与四类邻接通信：** `src/session.rs` 的 `MailboxMessage`、`SessionMailbox::enqueue/list/update` 和 `LiveSessionRegistry::claim_next/claim_message/finish_claim`（L3171-L3185、L3286-L3574、L3574-L3743）已经提供持久化消息、TTL、claim、完成/失败。DAG 节点应把每个节点绑定一个 `session_id`，通过 `recipient_session_id` 向 parent、grandparent、直接 sibling、直接 child 发送；grandparent/sibling 不应依赖路径猜测，而应由持久化 DAG 索引解析为 session ID。发送端使用 `Send`，接收端 `Receive`，处理前 `Claim`，处理结束用 `Complete`，并保留 `message_id` 作为幂等相关键。headless 的 `execute_mailbox` 已限制收件人必须是同一 session directory 中唯一的干净 journal，并执行 enqueue/list/claim/complete（L8954-L9093），可直接作为 transport adapter。
- **会话父子关系：** `src/core.rs` 的 `Turn` 有 `parent_id`，`with_parent` 和 `validate` 保证父引用可追踪且 ID/内容有界（L140-L205）；`src/session.rs` 另有 tree snapshot/ancestry/fork 能力（`tree_snapshot`、`tree_ancestry_turns`、`fork_at_tree_leaf`，L895-L1070）。建议新增 `DagNodeRecord { node_id, session_id, parent_id, status, generation, child_ids }`，把 DAG 拓扑与对话 turn 的 parent_id 分开保存；启动或恢复时从 session journal 重建 parent、grandparent、sibling、child 索引，拒绝缺失父节点、重复 edge 和跨 workspace 目标。
- **worker 生命周期、保活与派生的最小机制：** `src/runtime.rs` 的 `BackgroundRunner::spawn`、`try_submit`、`try_cancel` 与有界 `RuntimeConfig`（L101-L133、L272-L325）提供派生 worker、排队、非阻塞提交和取消；`CancellationToken` 是协作式、幂等取消，任务应在边界检查并可用 `mark_completed` 防止晚到取消改写成功结果（L49-L99）。建议在 `runtime.rs` 增加最小 `DagWorkerRequest { node_id

## AC-006 claude-code/plugins/agent-sdk-dev/commands/new-sdk-app.md
**总体边界。** 源命令的“逐步收集、计划确认、执行、验证”可映射为 zenpi 的有界会话/运行时状态；它本身不是 SDK provider。现有 `src/headless.rs` 的 `run_headless`/`run_async_streams`（约 L556-L806）负责 JSONL 帧、request-id、重放、owner 注册与关闭；`src/core.rs` 的 `Agent`（约 L403 起）持有同步 backend、session、工具与事件；`src/runtime.rs` 的 `BackgroundRunner`/`CancellationToken`（L237-L440）适合把一个 DAG 节点变成可取消 job；`src/providers/**` 仅负责 provider 协议/路由，不应承载 DAG 拓扑或父子邮箱。

**可复用通信原语。**
- 消息/邮箱：复用 `src/session.rs` 的 `MailboxMessage`、`SessionMailbox::enqueue/list/update/find_message`（约 L3181-L3260、L3574-L3953）和 `MailboxStatus`/`MailboxAction`。每条消息有 sender、recipient、request_id、digest、TTL、sequence、claim token、结果，适合节点工作请求、完成回执、错误回执。
- 事件：复用 `src/core.rs` 的 `AgentEvent` 与 `src/headless.rs` 的有界 async event buffer；用 `StdioEvent`/`RuntimeEvent::{Accepted,Started,Queued,CancelRequested,Completed,Closed}`（`src/runtime.rs` L174-L212）发布节点生命周期和验证进度，不把 provider 流当作拓扑真相。
- 协议：扩展 `src/protocol.rs` 的 `MailboxRequest`（L81-L119）以携带 `dag_id,node_id,parent_id,edge_kind,generation,required_children`，保留 `Send/Receive/Acknowledge/Claim/Complete`；复用 `TreeRequest/TreeAction`（约 L1164-L1239）展示/选择/fork 拓扑。所有新增字段应维持 `deny_unknown_fields`、ID/文本/TTL 上限。
- 地址关系：在 session 元数据或新 `DagNodeRef` 中保存 `parent_id`、`grandparent_id`、直接 sibling 集合和直接 child 集合；发送时只允许同一 workspace、已注册且 epoch 有效的 session，正好沿用 `SessionMailbox::check_access` 的访问校验（约 L3610-L3640）。grandparent/sibling/child 都是显式 recipient，不通过模糊广播。

**会话父子关系与最小 DAG 机制。** 建议在 `src/session.rs` 增加 `DagNodeState { dag_id, node_id, parent: Option<NodeRef>, children: BTreeSet<NodeRef>, status, generation, live_epoch }`，并让每个 worker session 的 `SessionStore` 记录不可变 parent/edge 元数据；`LiveSessionRegistry`（约 L3278-L3410）的 `register/heartbeat/active/unregister` 作为保活租约。最小可执行路径是：父节点 enqueue 子节点任务；子节点 `Claim` 后运行；子节点向 parent、grandparent、直接 sibling、直接 child 各自发送有界事件

## AC-007 claude-code/plugins/hookify/agents/conversation-analyzer.md
源 agent 没有 DAG 语义；以下是把“行为发现/规则建议”与 zenpi 的会话编排能力结合起来，并满足 DAG worker 通信要求的可执行落点。

- **通信原语复用**：优先复用 `protocol::MailboxRequest` 的 `Send/Receive/Acknowledge/Claim/Complete`（`src/protocol.rs:L81-L113`）和 JSONL `Command::Mailbox`（L211-L263）。底层用 `session::SessionMailbox` 的 `MailboxMessage`、digest、TTL、claim token 和结果状态（`src/session.rs:L3180-L3242`、L3563-L3679）；它是本地有界、可寻址、可持久化邮箱，不应另造无认证 channel。短生命周期进度继续用 `core::AgentEvent` 的 `Provider/ToolProgress/Warning/Error`（`src/core.rs:L281-L325`），通过 `headless.rs` 的有界 agent/provider event mailbox（约 L4014-L4065）输出。`Handoff` 可作为跨节点摘要，session 的 `append_handoff`/`append_event` 负责审计（`src/session.rs:L1142-L1207`）。
- **会话父子关系**：`core::Turn.parent_id` 与 `Turn::with_parent` 已表达同一会话内的直接父子输入（`src/core.rs:L140-L175`）；`steer_existing` 会以 active turn 为 parent 写入新 user turn（L3429-L3497）。DAG 节点应增加持久化 `DagNode { node_id, session_id, parent_node_id, edge_kind, status, generation }`，其中 `parent_node_id` 指向直接 parent；祖父通过重复 parent 查询，直接 sibling 是同 parent 的其它 child，直接 child 是反向索引。记录应写 `SessionStore::append_event` 或新 `session_tree` 事件，不能只放内存。
- **parent/grandparent/sibling/child 通信**：为每个节点把目标 `session_id` 和 `node_id` 放入 mailbox payload，并在 `MailboxMessage` 的 sender/recipient/request digest 外增加协议版本、`correlation_id`、`edge_kind`、`reply_to`。路由层先查 parent，再沿两级 parent 找 grandparent；同 parent 的 node 列表得到直接 sibling；child 列表得到直接 child。所有跨节点请求都走 `LiveSessionRegistry::claim_next/claim_message`，完成走 `finish_claim` 或 `finish_claim_with_reply`（`src/session.rs:L3274-L3453`），避免绕过 workspace、owner epoch 和 claim token 校验。
- **保活的最小机制**：注册节点时调用 `LiveSessionRegistry::register`，周期调用 `heartbeat`，以 `active(..., ttl_ms)` 判定在线并清理过期 owner（`src/session.rs:L3285-L3355`）。每个 worker 持有 `lease_id/owner_epoch`，将 `last_seen_ms` 和当前 DAG 状态写入事件；邮箱发送使用 `MAX_MAILBOX_TTL_MS` 上限（`src/protocol.rs:L34-L35`），

## AC-008 claude-code/plugins/hookify/core/config_loader.py
源逻辑可抽象为“声明式输入→受限状态对象→按事件筛选→逐项容错”。zenpi 已有更强的 mailbox、session、runtime 和 approval 基础，DAG 映射应复用它们。

- **消息/邮箱/事件/协议**：复用 src/protocol.rs:L79-L113 的 MailboxOutcome 与 MailboxRequest::{Send,Receive,Acknowledge,Claim,Complete}；入口和字段校验在 L479-L484、L544-L590。持久通道用 src/session.rs:L3169-L3194 的 MailboxStatus/MailboxMessage 与 L3570-L3710 的 SessionMailbox；更新、过期拒绝和 claim token 在 L3743-L3795。短进度用 src/runtime.rs 的有界 command/event channel 和 RuntimeEvent；src/headless.rs:L833-L905、L940-L1041 已按请求限制事件数量/字节并记录丢弃。命中、子节点结果和 close 应为事件，跨进程可靠投递走 mailbox。
- **父子会话及邻居**：src/session_tree.rs:L48-L84 的 TreeEntry.parent_id/depth/branch_id 表达父链和分支；src/core.rs 中 Turn.parent_id 的校验在 L141-L196。新增 DagNodeId、DagRelation（Parent、Grandparent、DirectSibling、DirectChild、Grandchild）和 DagNodeState，把关系与目标 node/session id 放入 mailbox payload。发送端不能自报关系：core 服务层沿 parent_id 向上查祖先、按同一 parent 查 direct children，再查 children 的 children；限制最大深度并校验同一 workspace。现有 mailbox 只按 session_id 寻址，不能自动保证 direct sibling，因此关系授权必须在 core。
- **保活和最小派生**：用 src/session.rs:L3274-L3355 的 LiveSessionRegistry::{register,heartbeat,unregister,active} 做 owner_epoch+TTL 租约；过期即不可投递。派生只提交一个带 parent node、generation、idempotency key 的 DagWork 到 src/runtime.rs:L272-L300 BackgroundRunner::spawn 或 src/tool_runtime.rs:L274-L330 的有界批处理。SessionMailbox::claim_next 明确只 claim、never starts a worker（L3361-L3409），所以由 DagSupervisor 显式 spawn。完成用 finish_claim/finish_claim_with_reply（L3440-L3559），并回送 sender mailbox。
- **close 门槛**：在 src/core.rs 增加 DagNodeStatus::{Open,Waiting,Closed,Failed} 与 close_if_green(node_id)。节点自身成功、全部 direct child 和 grandchild 成功、无未完成 descendant mailbox、无活动 approval/tool 操作时，才追加 durable dag_node_closed。任一 child 失败、超时、未开始或有新消息就保持 Waiting，heartbeat 保持 owner 活跃并派生新 worker；派生不能重复执行旧 generation。祖父/兄弟只可报告或发送输入，不能绕过 close 门槛；恢复时 journal 没有 terminal close 就

## AC-009 claude-code/plugins/hookify/core/rule_engine.py
现有可复用原语是结构化消息、邮箱、事件和会话日志：`src/protocol.rs:L79-L113` 的 `MailboxRequest::{Send,Receive,Acknowledge,Claim,Complete}` 已提供带 `message_id`、TTL、结果状态的邮箱协议，`L544-L582` 做边界校验；`StdioEvent` 是带单调 `sequence`、`request_id`、`turn_id` 的进度事件（`src/protocol.rs:L882-L918`）。`src/headless.rs:L806-L1061` 的 `AsyncEventBuffer`、有界 provider/agent mailbox 与 `PendingSteer` 可作为背压和跨 worker 事件转发模板。`src/runtime.rs:L38-L99` 的 `JobId`/`CancellationToken`、`L101-L193` 的容量配置和终态事件，以及 `L712-L759` 的 `start_job`，适合作为派生 worker 的最小调度机制。`src/approval.rs:L126-L245` 的 `ApprovalCoordinator` 展示了 `Arc<Mutex<_>>+Condvar`、可见请求、取消和持久化前保留响应的会话式父子通信。

建议的可执行落点与 DAG 约束如下：

1. `src/protocol.rs`：新增 `DagNodeId`、`DagRelation { Parent, Grandparent, Sibling, Child }`、`DagMessage { node_id, sender, recipient, relation, kind, payload, correlation_id, ttl_ms }` 与 `DagControl::{Close, KeepAlive, Spawn}`，复用 `validate_identifier`、`validate_mailbox_text` 和现有 `MailboxRequest` 的有界文本/TTL校验。接收端只能根据会话内已知父链授权：parent/grandparent 是祖先，direct sibling 必须共享直接 parent，direct child 必须是自身登记的 child；拒绝任意越级 recipient。
2. `src/session.rs`：`SessionStore` 已以 append-only JSONL 保存 `Turn`、handoff、runtime intent 和不透明 event（`L1142-L1223`），可增加 `dag_node_registered`、`dag_message_sent/claimed/completed`、`dag_status_changed`、`dag_keepalive`、`dag_spawn_requested` 事件辅助函数。用 `core::Turn.parent_id`（`src/core.rs:L140-L204`）保存会话父子关系，并在恢复时重建 `HashMap<NodeId, {parent, children, status}>`；grandparent、sibling 通过父链计算而非重复持久化。
3. `src/core.rs`：在 `Agent`（字段与事件见 `L403-L442`、`AgentEvent` 见 `L281-L325`）增加当前 DAG 节点上下文和 `DagStatus`；turn 完成时执行 `close_gate(node)`：只有“节点自身为 Green 且所有递归 child/grandchild 均 Green”才发 `Close`。否则保持 `AgentPhase::Running`/保活，写入 `dag_keepalive`，并把新任务封装为待派生请求；可把 parent/child 通信映射为新的 `AgentEvent::DagMessage`，不把 provider 文本误当作状态确认。
4. `src/runtime.rs`：复用有界 `BackgroundRunne

## AC-010 claude-code/plugins/hookify/hooks/hooks.json
**可复用的通信原语。** 这份配置把“生命周期事件→命令进程”作为最小 Hook 协议。zenpi 可复用 `src/protocol.rs` 的 JSONL 帧与 `Command::Mailbox`（`MailboxRequest` 的 `Send/Receive/Acknowledge/Claim/Complete` 在 `src/protocol.rs:L87-L113`、校验在 `L544-L598`，命令分派在 `L479-L484`、`L211-L263`），把 Hook 事件建模为带 `request_id/session_id/node_id` 的消息；用 `StdioEvent::new`（`src/protocol.rs:L882-L918`）发布异步事件。持久化消息应落到 `src/session.rs` 的 `SessionMailbox`/`MailboxMessage`（`L3171-L3265`），其 `Queued→Acknowledged→Claimed→Succeeded|Failed` 状态、digest、TTL 和 claim token 正好提供邮箱、去重和结果回传。`LiveSessionRegistry` 的注册、heartbeat、TTL 过期和 `claim_next`（`src/session.rs:L3274-L3409`）可作为活跃 worker 的最小保活门槛。

**会话父子关系与 DAG 邻接。** 现有 `Turn.parent_id`（`src/core.rs:L140-L203`）和 `SessionTree` 的 `TreeEntry.parent_id/depth/branch_id`（`src/session_tree.rs:L48-L84`、规划在 `L316-L362`）能表达父链及分支，但公开 API 主要提供 ancestry/page，尚未直接返回“直接 child ID 列表”或“直接 sibling 列表”。建议新增受限的 `DagNodeId`、`DagRelation { parent, grandparent, direct_siblings, direct_children }` 与 `DagIndex`（可放在 `src/session_tree.rs` 扩展或新 `src/dag.rs`）：以 `TreeEntry.parent_id` 建父子索引，同父节点集合求直接 sibling，沿父链一步得到 grandparent；每次查询执行数量/深度/字节上限。每个 DAG worker 以独立 `session_id` 或节点 ID 作为邮箱收件人，发送到 parent、grandparent、每个直接 sibling、每个直接 child 都走 `MailboxRequest::Send`，结果以 `Complete` 或 `finish_claim_with_reply`（`src/session.rs:L3490-L3560`）关联原 digest，禁止把 provider 输出直接当控制消息。

**保活与派生的最小机制。** (1) 节点启动时在 `LiveSessionRegistry` 注册 `(session_id, owner_epoch, workspace, last_seen_ms)`，周期性 `heartbeat`；邮箱 `ttl_ms` 和 owner epoch 防止旧 worker 继续领取。 (2) 用 `BackgroundRunner`（`src/runtime.rs:L236-L315`）承载节点工作：`try_submit` 受有界队列约束，`RuntimeEvent::Accepted/Started/Completed/Closed`（`L171-L193`）提供可核对生命周期；`CancellationToken`（`L49-L99`）用于合作式取消，不能撤销已经发生的外部副作用。 (3) 在 `src/core.rs` 的 worker 治理入口使用 `set_worker_budget_limits`、`renew_blueprint_worker`、`settle_blueprin

## AC-011 claude-code/plugins/hookify/hooks/posttooluse.py
该 hook 的核心可抽象为“工具完成事件 → 按工具类型选择规则 → 同步评估 → 输出协议消息”。zenpi 已有更强的可复用原语，但需要把 DAG 拓扑与绿色关闭条件显式化：

- **通信原语/协议**：`src/protocol.rs` 已有版本化 JSONL `StdioRequest`/`StdioResponse`/`StdioEvent`（约 L793-L900），以及 `Command::Mailbox`、`MailboxRequest` 的 `Send/Receive/Acknowledge/Claim/Complete`（约 L30-L110、L211-L259）。将 post-tool 结果建模为 `StdioEvent { kind: "tool_completed", turn_id, event }`，规则命中建模为结构化 `data`，不要把诊断混入 stdout。DAG 节点间消息优先复用 `MailboxRequest`，事件流复用 `StdioEvent`；规则选择可做成 `ToolRuleSet::for_event("bash"|"file")`。
- **持久邮箱与幂等**：`src/session.rs` 的 `MailboxMessage`/`MailboxStatus`、摘要校验、TTL、claim token、`SessionMailbox::enqueue/find_message`（L3169-L3265、L3570-L3665）可直接承载 parent、grandparent、直接 sibling、直接 child 的定向消息。发送方把 `recipient_session_id` 与 `request_id` 固定；接收方 `claim` 后只能一次 `Complete/Fail`，避免重复派生。`LiveSessionRegistry::register/heartbeat/active`（L3274-L3355）提供收件人在线租约。
- **会话父子关系**：`src/core.rs` 的 `Turn.parent_id` 与 `Turn::with_parent`（约 L100-L150）保存直接父关系；`src/session_tree.rs` 的 `TreeEntry.parent_id`、`ancestry`、`children` 索引（L48-L84、L245-L268）可推导 grandparent、直接 sibling（同一 `parent_id` 的节点）和直接 child。建议在 `session_tree.rs` 增加只读 `neighbors(node_id) -> {parent, grandparent, siblings, children}`，并在 `src/session.rs` 通过 `SessionStore` 的树快照提供给 mailbox 路由；不得靠文件名猜测关系。
- **worker/规则落点**：工具执行完成后的判定点在 `src/tool_runtime.rs::execute_tool_batch`（L157-L176、L219-L249、L252-L347）及 `src/core.rs` 的工具结果持久化路径；这里应生成 `ToolCompleted` 事件并按 `ToolExecutionOutcome` 选择 DAG 规则。`src/providers/**`（Anthropic/Codex/DeepSeek/Google/OpenAI 适配器）只负责 `Backend`/`ProviderEvent` 流和取消检查，不应直接决定 DAG close；provider 事件可作为节点进度输入。
- **保活与取消**：`src/runtime.rs::CancellationToken` 是协作取消、幂等 `cancel`、`mark_completed` 的最小基础（L49-L98）；`BackgroundRunner` 的有界命令/事件通道、`try_submit`、`try_cancel`、`RuntimeEvent::{Accepted,Started,Queued,CancelRequested,Com

## AC-012 claude-code/plugins/hookify/hooks/pretooluse.py
- 通信原语：本文件的“输入 JSON → 分类事件 → 规则评估 → JSON 输出”可映射为 `src/protocol.rs` 的严格 JSONL 请求/事件边界。已有 `MailboxRequest::{Send,Receive,Acknowledge,Claim,Complete}`（`src/protocol.rs` L87-L113）可复用作 worker 间消息/邮箱协议；`MailboxOutcome` 的 `Succeeded/Failed/Abandoned` 对应规则执行结果。`StdioEvent`/`StdioResponse`（`src/protocol.rs` L882-L966）承载逐条诊断与终态。对于 DAG 节点，建议增加或扩展 `WorkerMessage`（sender、recipient、message_id、parent_session_id、kind、payload、ttl）并复用 `MailboxRequest` 的 TTL、分页、claim/complete 语义，而不是在 worker 间共享内存。
- 会话父子关系：`src/core.rs` 的 `Turn { id, parent_id, ... }`（约 L134-L180）已能表达活动转向关系；`src/session_tree.rs` 的 `TreeEntry.parent_id`、`children` 与 `SessionTree::ancestors/page/plan_turn/commit_turn`（L48-L73、L250-L372）适合表达 DAG 节点的会话树索引。建议为 worker 会话增加不可变 `WorkerNodeId`、`parent_session_id` 与有序 `child_session_ids`，并在 `SessionTree` 旁记录直接 sibling/child 查询；grandparent 是沿 `parent_id` 向上两次，直接 sibling 是父节点 children 中除自身者，直接 child 是 children 索引的一层。必须禁止任意跨树 parent，沿用 `plan_turn` 的 ancestry 校验。
- 邮箱落点与身份校验：`src/session.rs` 已有 `MailboxMessage`（L3180-L3194）、状态/TTL/digest 校验（L3208-L3242）、`LiveSessionRegistry` 心跳和 owner epoch（L3274-L3355）、显式 `claim_next`（L3361-L3410）。这正好提供 parent、grandparent、直接 sibling、直接 child 的持久消息通道：发送前校验 workspace 与 DAG 可达关系，接收方用 owner epoch claim，处理完以 Complete 写结果。`src/headless.rs` 顶部的有界 replay/请求去重状态（L1-L18、L334-L380）可作为传输层重放保护；不要把 stdout replay 当成 durable mailbox。
- 保活与派生的最小机制：`src/runtime.rs` 的 `BackgroundRunner`、`CancellationToken`、`RuntimeConfig`（L49-L132、L236-L250）提供有界命令/事件队列、poll interval、取消和 FIFO follow-up；`JobOutcome` 与 `RuntimeEvent`（L156-L193）可承载 `Running/Completed/Cancelled/Panicked`。建议新增最小 `DagNodeState { node_id, parent_id, generation, status, heartbeat_at, required_children, green_children, pending_work }` 与 `DagSupervisor`：节点每个心跳/终态写事件；只有自身 `Succeeded` 且全部直接 child/grandchild 的递归状态为 `

## AC-013 claude-code/plugins/hookify/hooks/stop.py
语义对应的最小单元不是一个新的 Stop hook，而是“节点状态检查 + 可寻址通信 + 可取消后台工作”的组合。项目已有 `src/dag.rs` 与要求的四类模块足以承载它：`DagNode` 的 `parent`、`children`、`status`、`worker`（`src/dag.rs:L57-L73`）直接表达会话父子关系；`DagStore::recipients` 已解析 `parent`、`grandparent`、直接 `sibling`、直接 `child` 及 `all`（`src/dag.rs:L279-L347`），`send`/`inbox` 是消息邮箱原语（`L349-L393`），`can_close` 已实现“自身及全部 descendant 为 green 才可关闭，否则返回 unfinished”（`L395-L425`），`spawn_worker` 保持子进程 stdin 打开以便后续派生工作（`L446-L496`），`send_to_worker` 是同一子 worker 的 follow-up 通道（`L518-L532`）。因此 AC-013 的 DAG 要求应复用这些原语，不另造只发给父节点的管道。

- **协议/邮箱/事件落点**：`src/protocol.rs:L81-L116` 的 `MailboxRequest`（Send/Receive/Acknowledge/Claim/Complete）可作为节点间稳定协议；`TreeAction`/`TreeRequest`（`L1161-L1234`）适合传输树视图和导航，但不能单独代替 DAG 状态。`src/session.rs:L3180-L3194` 的 `MailboxMessage` 提供 sender/recipient、digest、TTL、sequence、status、claim token、result；`LiveSessionRegistry` 的 register/heartbeat/claim/finish（`L3274-L3488`）提供最小会话父子存活租约。直接 sibling/child 可由 `DagStore::recipients` 解析成 recipient IDs，再通过 `SessionMailbox` 或 `MailboxRequest` 投递；事件流可用 `src/runtime.rs:L171-L193` 的 `RuntimeEvent::{Accepted,Started,Queued,CancelRequested,Completed,Closed}`，将 `dag_status`、`dag_message`、`dag_spawned`、`dag_close_deferred` 映射为可回放的 headless 事件。
- **headless 编排落点**：`src/headless.rs:L4466-L4625` 已将 `BackgroundRunner` 接到异步请求并周期性 heartbeat；`L4951-L5169` 把 runtime 生命周期事件按 request/turn 关联写出；`L6392-L6400`、`L8943-L9093` 已有 mailbox 路由。建议在同一 dispatch 层加入 `DagAction`（status/send/wait/close/spawn），所有回复经过现有 replay/event writer，避免把 DAG 消息塞进 provider delta。收到 `close` 时先调用 `DagStore::can_close`：可关闭才调用 `Agent::try_close`/runtime shutdown；不可关闭则保持 worker job 活跃，发送 `dag_close_deferred`（含 unfinished IDs），按需对新节点调用 `spawn_worker`。
- **core/session 落点**：`src/core.rs:L819-L914` 的 live owner 与 mailbox facade 可作为节点会话 owner API；`L922-L1001` 的 `t

## AC-014 claude-code/plugins/hookify/hooks/userpromptsubmit.py
- 通信原语与协议：最接近的可复用原语在 src/protocol.rs。现有 MailboxRequest::{Send,Receive,Acknowledge,Claim,Complete}、MailboxOutcome（约 L81-L112）已覆盖消息投递、领取、确认和完成；StdioRequest/Command 与 JSONL 编解码适合作为 worker 控制面。将 DAG 消息定义为带 message_id、sender_session_id、recipient_session_id、relation（parent/grandparent/sibling/child）、ttl_ms、correlation_id 和载荷的严格 serde 类型，复用现有 ID/文本/TTL 上限与 validate_mailbox，拒绝未知字段。事件流可复用 StdioEvent/headless 的有界 event mailbox；协议层只解析，不授予发送者权限，身份和工作区授权应由 owner 校验。
- 会话父子关系：src/core.rs 的 Turn { parent_id: Option<String> }、Turn::with_parent（约 L141-L177）可作为消息上下文和派生 worker 的最小 lineage；节点级实现建议新增 DagNode { node_id, session_id, parent_node_id, status, lease_id, children: BTreeSet<NodeId> } 与 DagRelation。直接 sibling 由同一 parent_node_id 的 children 求得，grandparent 由 parent 的 parent 求得，直接 child 是 children，grandchild 是 child.children；不要把关系仅编码在 prompt 文本中。会话持久化落在 src/session.rs 的 append-only JSONL 事件，新增 dag_node_created、dag_status_changed、dag_message、worker_derived、dag_close_decision 记录即可，崩溃后按序重放重建索引。
- 节点关闭判据：在 src/core.rs 增加纯函数 can_close_node(node, snapshot)：只有节点自身状态为 Green，且其全部直接 child 递归为 Green（从而全部 grandchild/后代也为 Green）才返回 true；空 child 集合在自身 Green 时成立。任一后代为 Running、Failed、Cancelled、Unknown 或缺失记录都禁止 close。把判据结果写入 session 事件并让 headless 返回可核对的 reason，避免只看当前节点。
- 保活与派生的最小机制：src/runtime.rs 的 BackgroundRunner、有界队列、RuntimeEvent::{Accepted,Started,Queued,CancelRequested,Completed,Closed} 和 CancellationToken（约 L51-L99、L157-L193）可承载每个 DAG worker 的执行、心跳检查和合作取消。新增最小 DagSupervisor（可放 core.rs 或新模块并由 headless 调用）：(1) 为每个非 Green 节点保存带过期时间的 lease；(2) 在每个 poll_interval 或收到 child 状态/消息时发 keepalive 事件，刷新 lease 并重新评估后代；(3) 若当前节点仍有新工作且不能关闭，创建唯一 child_node_id/lease_id，以 Turn::with_parent 建立父链，通过 BackgroundRunner::submit 派生 worker；(4) 对同一 correlation_id 做幂等去重，避免重连重复派生；(5) lease 超时只标记 Unknown/需要恢复，不假装 Green。保活是有界、可

## AC-015 codex/codex-rs/core/src/agent/agent_names.txt
**可复用通信原语。** `src/protocol.rs` 已有 `MailboxRequest::{Send,Receive,Acknowledge,Claim,Complete}`（定义见 `L91-L113`）和 `Command::Mailbox`、`Command::Handoff`（`L214-L263`），适合 DAG 节点间可靠消息；`StdioEvent` 用 `sequence/request_id/turn_id` 封装异步事件（`src/protocol.rs:L882-L918`）。`src/session.rs` 的 `SessionMailbox` 是同工作区、加锁、追加式持久邮箱，`MailboxMessage` 带 sender/recipient、digest、TTL、状态和 claim token（`src/session.rs:L3180-L3242`、`L3570-L3615`）；可直接承载 parent、grandparent、直接 sibling、直接 child 的定址通信。进程内低延迟路径复用 `src/runtime.rs` 的有界 `mpsc::sync_channel` 与 `RuntimeEvent`（`L171-L193`），节点事件可转成 `AgentEvent::{Handoff,Provider,ToolResult,Error,Warning}`（`src/core.rs:L281-L325`）。

**会话父子关系。** 现有 `SessionHeader` 只有 `session_id/version/created_at_ms/cwd`（`src/session.rs:L35-L41`），没有 parent 字段。建议在会话记录或独立 DAG 元数据中增加 `NodeRelation { node_id, session_id, parent_id: Option<NodeId>, children: BTreeSet<NodeId>, name: String }`；`grandparent` 通过 `parent.parent_id` 解析，sibling 是 `parent.children - {self}`，直接 child 是 `children`。发送权限沿用 `SessionMailbox::check_access` 的“同 workspace 且 addressed session”约束（`src/session.rs:L3598-L3614`），不要允许任意路径冒充关系。

**保活与派生的最小机制。** 在 `src/runtime.rs` 的 job 输入中携带 `NodeId`、代际关系和 `CancellationToken`；`BackgroundRunner::spawn/try_submit/try_cancel`（`L272-L325`）负责启动、非阻塞提交和取消，`RuntimeEvent::Completed/Closed`（`L184-L193`）驱动状态更新。为满足“节点自身及全部 child/grandchild 全绿才 close”，在 `src/core.rs` 增加 `DagNodeState { self_green: bool, descendants: BTreeMap<NodeId, Health>, close_requested: bool, generation: u64 }` 与 `can_close()`：仅当自身和递归后代均为 Green 且没有未确认 mailbox 才允许 `Agent::close`；否则保持 Alive，发送 heartbeat/status 给 parent，并通过 `try_submit` 派生新 worker。派生必须生成新 `NodeId`/`JobId`、继承 parent 链和取消 token 的父级观察关系，使用幂等 `request_id` 防止重试重复建点；队列满返回 `SubmitError::QueueFull` 时保活并退避，runtime closed 则记录失败而不是假装完成。

**模块落点。**

- `src/headless.r

## AC-016 codex/codex-rs/core/src/agent/builtins/awaiter.toml
- 通信原语可直接复用 `src/protocol.rs` 的 `MailboxRequest::{Send,Receive,Acknowledge,Claim,Complete}`（`src/protocol.rs:L81-L110` 附近）和 `MailboxOutcome`，以及 `src/session.rs` 的 `MailboxMessage`、`MailboxStatus`、`MailboxAction`、`LiveSessionRegistry`、`SessionMailbox`（`src/session.rs:L3171-L3270`、`L3361-L3558`）。这些提供有界文本/TTL、digest、sequence、claim token 和完成/失败回执，适合把 awaiter 状态通知 parent、grandparent、直接 sibling、直接 child。建议新增 `DagRoute { Parent, Grandparent, DirectSibling, DirectChild }` 与 `DagEnvelope { message_id, dag_id, node_id, sender, recipient, route, kind, payload, deadline_ms }`，路由只接受会话树中已验证的关系，避免任意 session ID 互发。
- 会话父子关系可复用 `src/core.rs` 的 `Turn::parent_id`（`src/core.rs:L141-L196`）作为对话父链，但 DAG 关系应单独持久化，不能把“grandparent/ sibling/ child”从一条 turn 链猜出来。建议在 `src/session.rs` 增加 `DagNodeRecord { node_id, parent_id, children, state, generation }` 和 `DagState::{Running,Green,Failed,Keepalive,Closed}`，用事件记录边和状态变更；直接 sibling 通过共同 `parent_id` 解析，grandparent 通过两次 parent 边解析。
- `src/runtime.rs` 已有最小等待/派生执行机制：`BackgroundRunner::spawn`、`try_submit`、`recv_timeout`、`try_cancel`、`try_shutdown_with_grace`，以及 `CancellationToken` 的 `is_cancelled`/`mark_completed`（`src/runtime.rs:L40-L100`、`L230-L369`）。可定义 `AwaiterJob { node_id, task_id, deadline, poll_state }`，把每次 poll 作为同一 `JobId` 的观察，不在失败/超时后隐式再 submit。`RuntimeConfig` 的有界 command/event/pending 队列和 `poll_interval`（`L103-L131`）对应源的长 timeout 与保守轮询；`JobOutcome::{Succeeded,Failed,Cancelled,Panicked}`（`L158-L170`）可映射终态。
- 保活与派生的最小机制：worker 启动时在 `LiveSessionRegistry::register` 注册 `(session_id, owner_epoch, workspace)`，周期性 `heartbeat`；任务每次 poll/terminal 都写 `dag_heartbeat`/`dag_terminal` 事件。实现 `DagCloseGate::can_close(node)`：只有节点自身为 `Green`，且递归遍历的全部 child/grandchild 都为 `Green`，才允许写 `Closed`。否则写 `Keepalive`，保留 owner lease，并通过 `BackgroundRunner::try_submit` 派生一个带新

## AC-017 codex/codex-rs/core/src/agent/builtins/explorer.toml
映射前提是“空源不提供可复用实现”，因此 DAG worker 协议应落在 zenpi 现有运行时，而不是伪造一个 `explorer.toml` API。`src/runtime.rs` 已有 `CancellationToken`（约 `L56-L100`）、有界 `BackgroundRunner` 与 `try_submit/try_cancel/try_shutdown`（约 `L236-L340`），可复用为每个 DAG 节点的最小邮箱/事件循环：命令通道承载 `WorkerMessage`，事件通道承载 `WorkerEvent`，有界队列满返回显式错误；节点收到取消后停止接收新工作并发出终态。

会话父子关系应复用 `src/core.rs` 的 `Turn { parent_id }` 及 `with_parent/validate`（约 `L141-L197`），并在 `src/session.rs` 的追加式 JSONL 事件投影（`SessionStore` 约 `L144-L160`、`L327-L421`）中持久化 `session_id`、`parent_id`、`grandparent_id`、`node_id`、`edge_kind` 与状态版本。建议新增（或在现有模块中落点）`DagNodeId`、`WorkerRelation::{Parent,Grandparent,DirectSibling,DirectChild}`、`WorkerMessage::{Work,Status,Cancel,CloseRequest,SpawnRequest}`、`WorkerEvent::{Accepted,Progress,Green,Failed,Closed,Spawned}`；消息必须带 `correlation_id`、发送者节点和目标节点，邮箱按目标节点路由。父/祖父通信走显式会话索引，直接 sibling 只允许通过共同 parent 的转发器通信，direct child 由 parent 持有 child mailbox；禁止任意跨树广播。

`src/protocol.rs` 已有 `MailboxRequest`/`MailboxOutcome`（约 `L81-L119`）以及 `Command` 转换和 `validate_mailbox`（约 `L214-L305`、`L544-L585`），应作为外部 JSONL mailbox 协议的入口：扩展字段时保留接收者会话校验、文本上限和关联 ID 校验；`StdioEvent`（约 `L882-L913`）用于把异步 worker 事件投影给 headless 客户端。`src/headless.rs` 的有界请求/事件与重放状态（约 `L39-L47`、`L133-L145`、`L288-L316`）可承载断线后的事件游标，但 DAG 真正状态仍须写入 session journal，不能只依赖 stdout 缓存。

`src/tool_runtime.rs` 的 `ToolBatchOptions` 默认并发与 `execute_tool_batch` 的 worker/channel 及 owner unwind 取消（约 `L113-L133`、`L168-L325`）可复用为节点执行工具时的并发边界；节点关闭前必须等待自己及全部 child/grandchild 的 green 事件。`src/approval.rs` 的 `ApprovalCoordinator`（约 `L126-L170`、`L409-L456`）可作为需要人工批准的消息/副作用闸门，取消应调用 `cancel_all` 或 `emergency_cancel`，并将结果持久化后再释放等待 worker。

最小保活与派生机制：每个节点维护 `child_ids`、`pending_children`、`self_green`、`generation` 和 `CancellationToken`；只有 `self_green == true` 且所有 child（递归汇总 grandchild）都收到持久化 `Green`，才发 `CloseReques

## AC-018 codex/codex-rs/core/src/agent/control.rs
### 可直接复用的通信原语

1. **消息/邮箱协议**：`src/protocol.rs` 已有 `MailboxRequest::{Send,Receive,Acknowledge,Claim,Complete}` 与 `MailboxOutcome::{Succeeded,Failed,Abandoned}`（`src/protocol.rs:L81-L113`），并已在 `StdioRequest.mailbox` 暴露（`L152-L209`）。复用它作为跨 session 的 durable DAG 消息：`recipient_session_id` 映射 node/session id，`message_id` 做幂等键，`ttl_ms` 做保活消息过期；发送方身份仍由服务端 session owner 推断，不能信任客户端字段。`validate_mailbox` 已限制非空、NUL、`MAX_MAILBOX_TEXT_BYTES`、TTL 和分页边界（`L544-L599`）。建议新增 `DAGMessage` 的 `serde_json` payload（`kind`, `run_id`, `node_id`, `parent_id`, `ancestor_id`, `relation`, `generation`, `state`, `required_descendant_epoch`, `work`），而不改动 mailbox 外层。
2. **事件流**：`src/core.rs` 的 `AgentEvent::{TurnAccepted,AssistantMessage,ToolCall,ToolResult,Error,Warning,...}`（`src/core.rs:L281-L325`）可作为 worker/node 局部事件；在其上新增 `DagEvent::{NodeStarted,NodeGreen,NodeFailed,NodeKeepAlive,NodeDerived,NodeClosed}`，通过 headless 的 bounded event mailbox 转发。事件必须带 `run_id/node_id/parent_id`，使 parent、grandparent 和 sibling 可以过滤，而不是依赖字符串通知。
3. **会话输入/队列**：`Agent::input_port` 与 `input_queue_request`（`src/core.rs:L674-L692`）提供“忙时排队、journal append 后才返回 receipt”的入口；`src/protocol.rs` 的 `InputQueueAction` 及严格 `InputQueueRequest::validate`（`L1035-L1158`）可承载 node 的新工作、steer 和 cancel。对 child 的派生请求应在 queue receipt 后才算 admitted。
4. **审批/工具事件**：`src/approval.rs` 的 `ApprovalCoordinator` 是 worker-host rendezvous，等待 host 决策但支持 cancelled（`src/approval.rs:L126-L180`）；`src/tool_runtime.rs::execute_tool_batch` 已有 bounded batch、cooperative cancellation、结果按输入顺序归并（`src/tool_runtime.rs:L157-L176`）。DAG 状态变绿前只能把 `ToolResult.success == true` 且审批/持久化完成视为该 node 的工作完成。

### 会话父子关系与四类邻接

- `control.rs` 的 `SessionSource::SubAgent(SubAgentSource::ThreadSpawn{parent_thread_id, depth,...})` 是最接近的关系模型：`AgentControl` 负责创建、继承、resume、直接 parent 完成通知（`cont

## AC-019 codex/codex-rs/core/src/agent/control_tests.rs
zenpi 已有的控制、运行时和持久化原语足以承载该测试文件的核心语义，但需要把“线程”提升为有身份的 DAG worker node，并补上后代闭合判定。

- `src/core.rs`：`Agent`/`Turn` 是单会话状态机；`Turn.parent_id`（定义与校验见 `src/core.rs`）可承载一次消息或 turn 的直接父关系，`AgentEvent`、`ProcessResult`、`submit`/`run_active_turn_cancelable` 是事件和任务边界。建议新增 `WorkerNodeId`、`WorkerNodeState { PendingInit, Running, Green, Errored, Interrupted, Closing, Closed, NotFound }`、`WorkerRelation { parent, depth, role, nickname }` 和 `DagNodeRecord { node_id, parent_id, children: BTreeSet<WorkerNodeId>, state, lease_id, generation }`，由 `Agent` 所属编排器维护。`spawn` 对应测试中的 `spawn_agent_creates_thread_and_sends_prompt`，应先写 node record，再通过 `Agent::input_port`/`TurnInputRequest` 投递；`resume` 应复用 node id 和 persisted relation。`Agent::close` 只能标记自身关闭候选，不应直接宣布 DAG 完成。
- `src/session.rs`：`SessionStore` 是追加式 JSONL，已有 `Handoff/HandoffRecord/RuntimeIntent`、`append_event`，以及 `LiveSessionRegistry`、`SessionMailbox`。复用 mailbox 的 `MailboxMessage`（sender/recipient/request_id/digest/sequence/status/claim_token/result）、`claim_next`、`claim_message`、`finish_claim`、`finish_claim_with_reply`（实现位置约 `L3280-L3560`）作为跨 worker 的 durable message/邮箱/请求-回复协议；每次 claim 绑定 owner epoch，完成结果幂等校验。新增 `DagNodeRecord`/`DagEdge` 事件类型时追加到同一 journal，或在独立 sidecar 中以 node id 索引，不能让内存 `BTreeSet` 成为唯一事实源。`SessionTree` 的 `TreeEntry.parent_id`、`ancestry`、`fork_at_tree_leaf` 适合 transcript 分支和祖先回放（`src/session_tree.rs`），但它是树而不是完整 worker DAG；worker 边应独立记录，以支持 sibling 多父引用/共享依赖。
- `src/protocol.rs`：`MailboxRequest::{Send,Receive,Acknowledge,Claim,Complete}`、`MailboxOutcome::{Succeeded,Failed,Abandoned}` 以及 `MAX_MAILBOX_TEXT_BYTES`、TTL/ID 校验是可复用协议面（定义约 `L81-L110`、验证约 `L544-L585`）。建议扩展 `Command::Worker`/`WorkerRequest`：`Spawn { parent, goal, role }`、`Send { recipient }`、`Status`、`Close`、`Heartbeat`、`Derive`，复用现有 request `id` 做幂等键。协议中明确 `recipi

## AC-020 codex/codex-rs/core/src/agent/guards.rs
zenpi 已有的 `protocol.rs` 提供可复用的有界消息协议：`MailboxRequest::{Send, Receive, Acknowledge, Claim, Complete}` 与 `MailboxOutcome`（`src/protocol.rs:L81-L117`），并限制 mailbox 文本、TTL、ID；它适合作为 worker 节点给 parent、grandparent、直接 sibling、直接 child 发消息的持久化邮箱命令。建议增加 `DagNodeId`、`DagRelation` 和带 `sender/recipient/session_id/message_id/causal_parent/ttl` 的 `DagMailboxMessage`，在 `validate_mailbox` 的现有边界（`src/protocol.rs:L544-L585`）之外拒绝越权关系；`Receive` 返回事件序号，`Claim/Complete` 保证一次消费和显式成功/失败/放弃。

通信层可组合三种现有原语：一是 `protocol.rs` 的 JSONL mailbox（跨进程、可重连）；二是 `runtime.rs` 的 `BackgroundRunner` 有界 `sync_channel`、`RuntimeEvent` 和 `JobId`（`src/runtime.rs:L49-L118`、`L171-L195`、`L236-L285`），作为进程内 worker command/event mailbox；三是 `core.rs` 的 `AgentEvent`（`TurnAccepted/ToolProgress/ToolResult/Handoff/Error`，`src/core.rs:L275-L326`）作为生命周期事件流。`headless.rs` 已经把 stdin/stdout JSONL、相关 ID、事件缓冲和关闭/重放串起来（`src/headless.rs:L1-L66`、`L561-L580`），应把 DAG 状态事件编码成 `StdioEvent`，而不是让 worker 直接写 stdout。

会话父子关系目前只有可复用的 `Turn.parent_id`（`src/core.rs:L141-L204`）以及 `SessionStore` 的 append-only records、tree 与 handoff 投影（`src/session.rs:L144-L160`、`L321-L329`、`L1142-L1168`）。建议在 `session.rs` 的 session tree 记录中补充 `dag_node_id`, `parent_node_id`, `root_node_id`, `depth`, `state`, `direct_children`，并让一次 worker spawn 产生不可变的 parent link；grandparent 通过重复 parent link 得到，direct sibling 通过共同 parent 索引得到，direct child 通过 child 索引得到。不要只把这些关系塞进自由格式 `Turn.metadata`，因为 close 判定和恢复需要可验证索引。`append_runtime_intent` 已明确“只持久化请求，不启动 scheduler/worker/process/network”（`src/session.rs:L1170-L1185`），可作为派生意图的 durable receipt；真正派生仍由 runtime owner 执行。

建议在 `src/core.rs` 增加 `DagNodeState::{Pending,Running,Waiting,Green,Blocked,Failed,Closing,Closed}`、`DagNodeSnapshot`、`DagWorkerContext` 与 `Agent::derive_worker/receive_dag_event/try_close_node`。`try_close_node` 的可验证规则是：

## AC-021 codex/codex-rs/core/src/agent/guards_tests.rs
对照现有代码，最接近的通信原语已经存在：

- src/protocol.rs 的 MailboxRequest::{Send,Receive,Acknowledge,Claim,Complete}（L79-L113）和 Command::Mailbox（L214-L263）提供带 recipient_session_id、message_id、ttl_ms、分页 cursor 的消息/邮箱协议；validate_mailbox（L544-L582）限制 ID、正文、TTL 和分页边界。它适合作为 parent、grandparent、直接 sibling、直接 child 之间的持久消息入口。
- src/session.rs 的 MailboxMessage/MailboxStatus（L3170-L3255）把消息建模为 Queued→Acknowledged/Claimed→Succeeded/Failed，并以 digest、claim token、过期时间保证幂等和 owner epoch 约束；LiveSessionRegistry 的 register/heartbeat/active（L3274-L3355）可作为 worker 保活租约。claim_next 明确只 claim、不启动 scheduler（L3361-L3410），因此“派生新 worker”应由上层编排器显式执行。
- src/headless.rs 的 AsyncEventBuffer（L861-L1021）是有容量和字节上限的事件邮箱，优先保留 admission/tool 生命周期事件；PendingSteer/enqueue_pending_steer（L1064-L1101）提供 bounded steer 队列和 backpressure。它适合承载 DAG 节点状态、心跳、派生、关闭事件，但必须为终态事件保留优先级。
- src/runtime.rs 的 BackgroundRunner、JobId、CancellationToken 和 RuntimeEvent::{Accepted,Started,Queued,CancelRequested,Completed,Closed}（约 L40-L203、L236-L365）可复用为每个 worker 的执行壳。取消是合作式的；Completed 必须先于 runner Closed，且超时后不能假定任意已发生的外部副作用被回滚。
- src/session_tree.rs 已有 TreeEntry { id, parent_id, branch_id, depth }（L48-L60）、ancestry（L245-L268）、children 计数和深度/字节预算（L303-L362）。它能持久化会话 turn 的父链和分支，但当前模型是 transcript entry tree，不等价于 worker DAG：没有 sibling 消息路由、节点完成聚合或全子孙绿色判定。
- src/core.rs 的 Agent 持有 SessionStore、events、worker_budget 与 live_owners（L402-L442），并提供 live mailbox 的 heartbeat、claim、finish、reply（L836-L910）；with_live_tool_events 通过恢复 guard 清理 request-scoped sink（L505-L520）。这里适合放 DAG admission 与节点状态投影。AgentEvent 的 Tool/Provider/Warning/Error 变体（L281-L325）可作为节点进度和失败通知。
- src/tool_runtime.rs 的 execute_tool_batch（L157-L180、L252-L330）使用共享取消位、5ms 轮询、scoped worker 并在 owner unwind 前先 stop；它说明 DAG 节点内的 tool 工作必须可取消且 join，不应 detach。src/approval.rs 的 ApprovalCoordinator 是 worker-h

## AC-022 codex/codex-rs/core/src/agent/mod.rs
zenpi 已有足够的通信积木，但需要补一层 DAG 拓扑与绿色收敛协议。建议按以下可执行落点实现：

1. 通信原语复用。src/session.rs:L3570-L3655 的 SessionMailbox 是私有、按 recipient 定址、带 inode lock、恢复 journal、TTL、claim/complete 的持久邮箱；它适合作为跨进程或跨 runtime 的 parent、grandparent、直接 sibling、直接 child 控制/结果通道。src/protocol.rs:L479-L484 已把 mailbox 命令解析为 Command::Mailbox，L544-L581 对 recipient/message、TTL、分页、claim/complete 做边界校验；src/headless.rs:L2065-L2119 的 mailbox_slash_view 是 TUI/headless 共用落点。内存高频事件继续使用 src/runtime.rs:L171-L190 的有序 RuntimeEvent 与 src/headless.rs:L862-L1035 的有界事件缓冲，避免把持久邮箱当广播总线。
2. 会话父子关系。src/core.rs:L140-L175 的 Turn.parent_id 只能表达一条对话父链，不能独立表达 DAG 的多 child。新增 DagNodeId、DagParent { parent: Option<DagNodeId>, grandparent: Option<DagNodeId> } 和 DagEdge { parent, child, kind }（kind 为 child/sibling 等派生关系），把 node/session ID 写入 SessionStore 的 runtime intent 或事件记录；不要只依赖内存 HashMap。同 workspace 与 addressed-session 校验可直接复用 SessionMailbox::check_access 的约束（src/session.rs:L3598-L3614）。
3. worker 与四类邻居通信。worker 处理节点时，先由拓扑索引得到 parent；grandparent 通过 parent 的 parent 链得到；直接 sibling 通过共同 parent 的 child 集合得到；直接 child 通过本节点的 edge 集合得到。发送采用 SessionMailbox::enqueue（请求/ack/绿色状态），接收采用 claim/finish，并用 message_id/digest 做幂等关联。需要即时 UI 更新时，将同一逻辑事件映射为 AgentEvent（src/core.rs:L281-L325）并交给 BackgroundRunner 的 event channel；不要让 sibling 直接共享可变 Agent。
4. 最小保活机制。在 src/core.rs 或新 src/dag.rs 定义 DagLease { node_id, generation, last_heartbeat_ms, ttl_ms, state }，状态至少包含 Running/Green/Failed/Cancelled/Closing。worker 每个 runtime tick 更新 lease；父节点通过 mailbox 发送 heartbeat/lease-renew，child 回 LeaseAck。可复用 Agent::heartbeat_live_owner 的 owner 心跳接口（src/core.rs:L836-L844）和 SessionMailbox 的 TTL；过期只标记 stale 并触发重派，不直接删除 journal。generation 防止旧 worker 的迟到完成覆盖新 worker。
5. 绿色 close 判定。在 src/core.rs 增加 DagNodeState::can_close()：自身必须为 Green，且递归/索引中的全部 child、grandchild 都为 Green，并且没有未

## AC-023 codex/codex-rs/core/src/agent/role.rs
语义对应：`role.rs` 的“高优先级、可复用配置层”可转成 zenpi 的“worker 角色/能力声明层”，但 DAG 通信与生命周期必须落在现有 session/runtime，而不是塞进 role 文本解析。zenpi 已有可复用原语：

1. **消息/邮箱**：`protocol::MailboxRequest` 提供 `Send/Receive/Acknowledge/Claim/Complete`，并限制行、文本、TTL、ID（`src/protocol.rs` L20-L35、L81-L113）；`session::MailboxMessage` 带 sender、recipient、request_id、digest、sequence、状态、claim token、结果（`src/session.rs` L3180-L3255）。`SessionMailbox` 是同 workspace、按 inode 锁和追加 journal 的本地寻址通道，明确“不启动 daemon、不隐式重试”（`src/session.rs` L3570-L3614）。因此 parent、grandparent、直接 sibling、直接 child 都可通过 session ID 定向发送；回复用原 digest 的 `Complete/Fail`，避免 request ID 冲突。
2. **事件**：核心 `AgentEvent` 已有 `TurnAccepted/AssistantMessage/Handoff/ToolCall/Provider/Error/Warning`（`src/core.rs` L281-L325）；headless 对每个异步请求维护独立 `AsyncEventBuffer`，并分别限制 provider/agent 数量与字节数，防止 queued replacement 串线（`src/headless.rs` L814-L869）。`protocol::StdioEvent` 提供 sequence、request_id、turn_id、event 封套，可把 `DagNodeStateChanged`、`WorkerHeartbeat`、`ChildGreen` 作为新事件（`src/protocol.rs` L882-L918）。
3. **会话父子关系**：`core::Turn` 已有 `parent_id`、`with_parent` 和 ID 校验（`src/core.rs` L140-L190）；`SessionStore` 持久化 turns/handoffs/events，并可选 `SessionTree`（`src/session.rs` L144-L160）。建议新增 `DagNodeRecord { node_id, session_id, parent_session_id, root_session_id, depth, sibling_index, status, generation }`，持久化为受 schema 约束的 session event；不要仅依赖 `Turn.parent_id`，因为它表达对话 turn，不足以表达跨 session 的 grandparent/sibling/child。
4. **保活与 liveness**：`LiveSessionRegistry` 已有 owner_epoch、workspace、last_seen_ms、注册上限、heartbeat、TTL 清理（`src/session.rs` L3274-L3355），`Agent` 暴露注册、heartbeat、注销和 claim/finish live mailbox（`src/core.rs` L836-L914）。建议将 DAG node lease 复用 `owner_epoch + expires_at_ms`，增加 `DagNodeHeartbeat { node_id, generation, green_digest, child_summary }`；heartbeat 只更新租约，不能直接宣告 green。
5. **取消、派生和背压**：`

## AC-024 codex/codex-rs/core/src/agent/role_tests.rs
### 可复用原语与 DAG 通信拓扑

`role_tests.rs` 的核心可迁移点是“角色是有边界的配置层，派生 worker 得到显式覆盖、继承未指定值，并在 spec 中看到不可变约束”。zenpi 已有更直接的通信基元：`src/protocol.rs:L91-L113` 的 `MailboxRequest::{Send,Receive,Acknowledge,Claim,Complete}` 是消息协议；`StdioEvent`（`src/protocol.rs:L882-L919`）和 `src/runtime.rs:L171-L193` 的 `RuntimeEvent` 是事件流；`src/session.rs:L3180-L3268` 的 `MailboxMessage`/`MailboxAction` 是带 digest、sequence、TTL、claim token、结果状态的持久邮箱；`LiveSessionRegistry`（`src/session.rs:L3278-L3559`）提供 owner 注册、heartbeat、按序 claim、完成及回执。建议把 DAG 节点通信统一为 `DagMessage` 包在 `MailboxMessage.payload` 中，字段至少为 `{dag_id, node_id, sender_node_id, relation, kind, parent_id, generation, lease_id, payload}`，其中 `relation` 枚举 `Parent|Grandparent|Sibling|Child`；不要另造无审计的内存全局 channel。

节点 worker 负责一个 `node_id` 时，最小可验证路由是：

- parent / grandparent：向对应 session mailbox `Send`，携带 `relation` 和 `in_reply_to`；parent 可代转 grandparent，但必须保留原 sender 与 digest。
- 直接 sibling：由 parent/调度器提供 sibling session id 白名单，worker 直接 `Send`；禁止通过“任意目标 session id”绕过拓扑授权。
- 直接 child：派生时写入 child mailbox 或通过 `LiveSessionRegistry::claim_next` 交付；child 完成后用 `finish_claim_with_reply` 回写原 sender，保留幂等 reply id。
- 事件/状态：worker 本地进度用 `RuntimeEvent`/`StdioEvent`，持久完成、失败、心跳和 close 判定写 `SessionStore::append_event`，避免仅依赖 stdout。

### 会话父子关系与关闭判定

`src/core.rs` 已有 `Turn { parent_id }`（约 `L100-L170`）和 `Turn::with_parent`，可承载“会话父子/上下文父级”；`src/runtime.rs:L765-L831` 的 `InputBoundary`/`InputBoundaryGate::with_context_parent` 可把当前模型 turn 绑定到 parent。会话树控制在 `src/protocol.rs:L1161-L1234` 的 `TreeAction`/`TreeRequest`，`src/session.rs` 的 `tree_snapshot`、`tree_ancestry_turns`、`fork_at_tree_leaf`（约 `L915-L1140`）可作为 DAG 可视化与分支持久化的基础。

建议新增 `src/dag.rs`（或 `src/domain_execution.rs` 的并列模块）及以下可执行类型：

```rust
pub struct DagNodeState {
    pub dag_id: String,
    pub node_id: String,
    pub par

## AC-025 codex/codex-rs/core/src/agent/status.rs
**可复用原语。** 将本文件的 reducer 语义落在 `src/core.rs` 的 `AgentEvent`（`src/core.rs:L281-L325`）与 `ProcessResult`（`src/core.rs:L5368-L5408`）之上，新增 `agent_comms::status_from_event(&AgentEvent) -> Option<NodeStatus>`，保留“未影响状态则 `None`”的稀疏事件约定。消息/邮箱优先复用 `src/protocol.rs:L81-L113` 的 `MailboxRequest::{Send,Receive,Acknowledge,Claim,Complete}`；`MailboxRequest` 已在 `Command::Mailbox` 路由（`src/protocol.rs:L479-L484`）并有 TTL、文本、ID 校验（`src/protocol.rs:L544-L590`）。持久对象使用 `src/session.rs:L3180-L3255` 的 `MailboxMessage`/`MailboxStatus`，其 digest、TTL、claim token 和终态校验可避免 sibling 重放。进程内快速事件使用 `src/runtime.rs:L171-L193` 的 `RuntimeEvent` 与有界 `sync_channel`；有界邮箱/事件流能复用 `src/headless.rs:L940-L1020` 的优先级和丢弃计数策略，但 DAG admission、completion、error 事件不得被普通进度挤掉。

**会话父子关系与寻址。** `src/core.rs:L140-L205` 的 `Turn.parent_id`/`Turn::with_parent` 是最小父链；`src/session_tree.rs` 已维护 entry-parent、children 计数及祖先校验，可作为 DAG 视图而不是把 mailbox 顺序误当树。建议新增 `WorkerNodeId`、`WorkerRelation { parent: Option<WorkerNodeId>, children: BTreeSet<WorkerNodeId> }`，持久化到 session event；直接 parent 由 `parent_id` 查找，grandparent 沿 parent 链上溯一次，direct sibling 取同一 parent 的 children 后排除自身，direct child 取当前节点 children。每条 mailbox 消息附 `sender_node_id`、`recipient_node_id`、`relation`、`work_id` 与 `reply_to`，这样同一 session 中也不会把不同 DAG 节点混淆。跨进程写入仍走 `SessionMailbox`；同进程 worker 之间可用 `BackgroundRunner` 事件通道，再以 mailbox 做 durable fallback。

**最小保活与派生机制。** 复用 `src/session.rs:L3274-L3355` 的 `LiveSessionRegistry::{register,heartbeat,unregister,active}` 和 `src/core.rs:L821-L844` 的 `register_live_owner`/`heartbeat_live_owner`/`unregister_live_owner`。建议在 `agent_comms::NodeLedger` 中维护 `self_status`、每个 child 的最新 `NodeStatus`、`last_heartbeat_ms`、`owner_epoch`、`generation`；`heartbeat` 只延长当前 epoch，过期节点不能继续 claim。把本文件的 `is_final` 拆成两层：`is_terminal`（Completed/Errored/Shutdown/NotFound）和

## AC-026 codex/codex-rs/core/src/external_agent_config.rs
这段源代码可复用的是“强类型迁移项 + 检测/执行分离 + 缺失字段合并 + 可选指标”的组织方式，不能直接承担 DAG 通信。zenpi 当前已有可落点如下：

- 通信原语优先复用 `src/protocol.rs` 的 `MailboxRequest::{Send,Receive,Acknowledge,Claim,Complete}`（`L90-L116`）：`recipient_session_id` 可承载 parent、grandparent、直接 sibling、直接 child 的 session/worker ID，`message_id` 做幂等键，`ttl_ms` 做保活/过期界，`MailboxOutcome::{Succeeded,Failed,Abandoned}` 表示处理终态。事件广播复用 `StdioEvent`/`AgentEvent`；高频内部消息用 `src/runtime.rs` 的有界 command/event channel，避免无界邮箱。建议新增 `src/worker_graph.rs` 的 `WorkerNodeId`、`WorkerRelation`、`WorkerMessage`、`WorkerMailbox` 和 `WorkerGraph`，将关系校验与路由集中在此处，而不是让 provider 直接互发消息。
- 会话父子关系可落在 `src/core.rs` 的 `Turn.parent_id`（`L143-L207`）和已有 `src/session_tree.rs` 的 `TreeEntry.parent_id`/`children`：新增 worker 节点时同时记录 `session_id`、`parent_worker_id`、`depth`、`branch_id`；grandparent 通过父节点再上溯，直接 sibling 由同一 `parent_worker_id` 的子集合求得，直接 child 由边表查询。必须定义边只能指向已知节点、禁止自环/环、父节点最多一个，DAG 的多子关系存为持久化事件而非只放内存。
- 最小保活/派生机制：在 `worker_graph.rs` 维护 `WorkerStatus::{Running,Green,Failed,Cancelled,Closed}`、`required_children` 与 `last_heartbeat`；实现 `can_close(node)`，只有节点自身为 `Green` 且所有递归 child/grandchild 均为 `Green` 才转 `Closed`。若存在未完成/失败后新工作，`ensure_live_or_spawn(node, work)` 保持父节点 lease，向 `src/runtime.rs::BackgroundRunner` 提交新 job 并生成新 `WorkerNodeId`，写入 `worker_spawned`/`worker_status` 事件；失败或 TTL 到期只标记不可关闭并派生替代 worker，不能静默 close。可验证断言：任一非绿后代时 `can_close=false`；全部后代绿后恰好一次 `Closed` 事件；派生 worker 有正确 parent。
- `src/headless.rs` 已有有界请求 mailbox、取消与 reconnect journal（例如 `L39-L66`、`L267-L316`、`L556-L721`），应作为外部 JSONL 宿主：增加 DAG mailbox/tree 命令的解析、事件序号和重放；断线时保留消息 TTL 和终态，不能把 stdout 缓存当作 DAG 真相。
- `src/core.rs` 的 `Agent`/`AgentEvent`/`AgentPhase`（`L275-L338`、`L402-L448`）是 worker 执行与状态投影落点。建议 `Agent` 持有 `Arc<WorkerGraph>` 或由 session coordinator 注入 graph handle；在 turn 完成、工具失败、子节点汇报时发 `WorkerMessage` 

## AC-027 codex/codex-rs/core/src/tasks/compact.rs
目标是把“压缩任务的路由/遥测/结果收束”扩展成 DAG worker 编排：worker 负责节点时可与 parent、grandparent、直接 sibling、直接 child 通信；只有节点自身及其全部 child/grandchild 均为 green 才能 close，否则保活并派生新 worker。

- 可复用通信原语：`src/protocol.rs` 的 `MailboxRequest::{Send,Receive,Acknowledge,Claim,Complete}`（`L91-L111`）已经是消息/邮箱/协议边界；`src/session.rs` 的 `SessionMailbox` 使用锁保护的 append-only journal，消息有 `sequence`、`digest`、TTL、claim/result 状态（`L3180-L3255`、`L3570-L3697`），适合把 DAG 边通信编码为 payload。`src/headless.rs` 将 `Command::Mailbox` 交给 session owner，忙时返回 `agent_busy`（`L6392-L6408`），可作为节点间发送/领取的 host 路由。实时进度可复用每请求的 `AsyncEventBuffer`（provider/agent 两个流，`L3925-L4050`），用于事件而非 durable mailbox。
- 通信关系应显式受图 ACL 约束：节点可向自己的 `parent_id`、沿父链解析出的 grandparent、同一 `parent_id` 的直接 sibling、以及 `parent_id == node_id` 的直接 child 发消息；接收端用 `Claim` 防止重复执行、`Complete` 回传结果，必要时沿 `sender_session_id` 建立 reply。Mailbox 的 session/workspace 校验（`src/session.rs:L3598-L3614`）应继续作为边界，不能仅凭客户端提供的 session ID 放开跨图通信。
- 会话父子关系：`src/core.rs::Turn` 的 `parent_id` 与 `Turn::with_parent`（`L140-L175`）是直接父关系；从父链递归即可得到 grandparent。直接 sibling 是相同 `parent_id` 的 turn 集合，直接 child 是 `parent_id == node_id` 的集合；`src/core.rs::selected_history`/`src/session.rs` 的树快照可作为查询入口。建议新增 `DagNodeId` 与 `DagEdge::{Parent,Child}`，不要把 sibling 关系复制存储，以免与 parent 链分叉。
- 保活与派生的最小机制：在 `src/session.rs` 增加 durable `DagNodeRecord`（node/parent/status/generation/lease/required_children/green）及 append-only transitions；在 `src/core.rs` 增加 `evaluate_dag_close(node)`：先要求 node 自身 terminal-green，再遍历所有 descendants（至少 child、grandchild，实际应递归全子树），全部 green 才返回 `Close`，否则返回 `KeepAlive{renew_lease}` 与 `SpawnWorker{new_worker_id}`。已有 `renew_blueprint_worker`、`cancel_blueprint_worker`、`settle_blueprint_worker` 可承载 lease、取消、终态结算（`L2823-L2885`）；settle 前应确认终态并回收 owned children，避免误关节点。
- `src/runtime.rs` 落点：`BackgroundRunner::spawn`、有界

## AC-028 codex/codex-rs/core/src/tasks/ghost_snapshot.rs
zenpi 可直接复用的通信原语有三层。持久、可寻址的节点间消息使用 `src/session.rs` 的 `SessionMailbox` 与 `MailboxMessage`/`MailboxStatus`（`Queued`、`Acknowledged`、`Claimed`、`Succeeded`、`Failed`、`Expired`）：`enqueue` 具备 request id 幂等、64 KiB payload、最多 7 天 TTL，`list/update` 在 inode 锁下追加同步记录；`src/protocol.rs` 的 `MailboxRequest::{Send,Receive,Acknowledge,Claim,Complete}` 是 JSONL 协议面，`src/headless.rs` 的 `mailbox_slash_view`/`execute_mailbox` 是统一入口。瞬时进程内通知复用 `core::AgentEvent`、`runtime::RuntimeEvent` 与 `headless` 的有界 event mailbox；`InputBoundaryGate`/`CancellationToken` 提供边界取消。provider 流事件仍走 `src/providers/**` 对应的 backend route，不应承担 DAG 拓扑或持久队列。

会话父子关系目前只有 `core::Turn { id, parent_id }`（`Turn::with_parent` 可建直接父链），以及 `WorkerExecutionBinding { blueprint_id, goal_id, item_id, lease_id, expires_at_ms }` 的工作关联；它们不足以表达完整 DAG。建议新增 `src/dag.rs`（或 `src/domain_execution` 的持久类型）`DagNode { node_id, session_id, parent_id, child_ids, status, generation, lease_id }`、`DagStatus::{Pending,Running,Green,Blocked,Closed}` 和 `DagEnvelope { request_id, sender_node, recipient_node, relation, payload, ttl_ms }`，将 `parent_id` 递归索引为 grandparent，将同一 `parent_id` 的其他节点解析为直接 sibling，将 `child_ids` 解析为直接 child；grandchild 是 child 的 child。持久记录应追加到 `SessionStore`/执行日志，不能只依赖内存 `LiveSessionRegistry`。

最小可执行的保活/派生机制：worker 获得带 `lease_id`/`expires_at_ms` 的 `WorkerExecutionBinding` 后，以固定间隔向 `Agent::heartbeat_live_owner` 或一条 mailbox heartbeat 发送保活；`LiveSessionRegistry::{register,heartbeat,active}`（`src/session.rs`）可作为进程内租约门槛，`Agent::{claim_live_mailbox,finish_live_mailbox_with_reply}` 负责领取与回执。worker 在每个节点完成时写 `DagNode.status=Green`，聚合 `self`、全部 child、grandchild 的状态；只有全绿才写 `Closed` 并释放 lease，否则保持 `Running/Blocked`、继续 heartbeat。发现新工作时，通过 `runtime::BackgroundRunner::try_submit` 派生最小新 job，并监听 `RuntimeEvent::{Accepted,Queued,Started,Completed,Rejected,

## AC-029 codex/codex-rs/core/src/tasks/mod.rs
zenpi 已有的通信原语足以承载 DAG，但需把本文件的“单活动 turn”语义提升为“每个 DAG 节点一个可取消 worker”。`src/protocol.rs` 的 `MailboxRequest` 已提供 `Send/Receive/Acknowledge/Claim/Complete`（L87-L113），并有 TTL/文本上限（L20-L37）；`Command::Mailbox`、`Command::Tree`（L251-L255）是协议入口。建议新增或扩展 `DagMessage`/`DagEvent`，字段至少为 `message_id`、`sender_session_id`、`recipient_session_id`、`node_id`、`parent_id`、`grandparent_id`、`kind`、`payload`、`ttl_ms`、`causal_sequence`。消息路由规则为：parent、grandparent、直接 sibling、直接 child 都只能通过节点登记的 session id 定向发送；禁止任意“广播所有祖先/后代”，避免 DAG 变成无界图。事件适合进程内实时通知，邮箱适合跨进程/重启后的可靠投递，协议负责 JSONL 校验与相关 id，状态协议负责 close/keepalive/spawn。

- `src/session.rs` 是会话父子关系与持久化落点。现有 `SessionStore`/`SessionSummary` 和 session tree API 可保存 `node_id -> parent_id`；应新增 `DagNodeRecord { node_id, session_id, parent_id, grandparent_id, status, generation, required_children, green_children, lease_expires_at_ms }`，并以 append-only journal 记录 `node_started`、`heartbeat`、`node_green`、`node_close_requested`、`node_kept_alive`、`worker_spawned`。已有 `MailboxMessage` 的 sender/recipient、digest、sequence、TTL、状态机（`src/session.rs` L3171-L3255）可直接复用；`LiveSessionRegistry` 的 owner epoch、heartbeat、TTL、claim/finish 机制（L3274-L3488）正好提供最小保活与幂等投递。现有 `claim_next` 明确“不启动 worker”，因此应由 DAG supervisor 在 claim 成功后显式派生 worker。

- `src/runtime.rs` 是 worker 执行落点。`BackgroundRunner`、`JobId`、`CancellationToken`、有界 command/event channel 及 `RuntimeEvent::{Accepted,Queued,Started,CancelRequested,Completed,Closed}`（约 L40-L195、L236-L369）可包装为 `DagWorkerRuntime`。最小机制是 `spawn_node(node_id, work)`、`cancel_node(node_id)`、`emit_node_event`、`join_node`：每个节点保留自己的 cancellation token 和 bounded event mailbox；parent 只收到聚合状态，不直接持有 child 线程。保活由周期 heartbeat job 更新 `LiveSessionRegistry`；lease 过期转为 `Blocked/Offline` 并禁止 close。派生新 worker 必须在 journal 先写 `worker_spawned`/lease，再调用 `BackgroundRunner::spa

## AC-030 codex/codex-rs/core/src/tasks/regular.rs
zenpi 已有的实现可以复用这段适配器的分层思想，但需要补齐 DAG 节点通信和保活规则。

- `src/core.rs`：`Agent` 是会话状态机；`Turn` 的 `parent_id`（`Turn::with_parent`，现有约 `L143-L184`）可作为节点的会话父子关系。建议增加 `DagNodeId`、`DagNodeState`（`Running/Green/Failed/Cancelled/WaitingChildren/Closed`）及 `DagNodeRecord { node_id, parent_id, children, status, generation }`，让一个 worker 的 turn 与 DAG 节点一一关联。现有 `submit_with_cancel`/`run_active_turn_cancelable`（约 `L3240-L3725`）适合作为“节点开始/执行/终态”入口；`set_worker_execution_binding`、`admit_blueprint_worker`、`settle_blueprint_worker`（约 `L1543-L1700`、`L2700` 附近）可承载 worker lease、预算和终态结算。`Agent::try_close`/`close`（约 `L5711-L5750`）目前是本地 agent 生命周期关闭点，必须在 DAG 规则下改为仅在节点及全部 child/grandchild 绿色时允许关闭。
- `src/session.rs`：`SessionStore` 的追加事件日志是持久化真相源；`Turn.parent_id` 和 session tree ancestry（`tree_ancestry_turns`、`fork_at_tree_leaf`，约 `L1038-L1128`）可表达父、祖父和分支。已有 `SessionMailbox`/`MailboxMessage`/`MailboxStatus`、`claim_next`、`claim_message`、`finish_claim`、`finish_claim_with_reply`（约 `L3171-L3555`）正好提供耐久邮箱、claim token、owner epoch、幂等完成和回复。建议新增事件类型 `dag_node_created/green/keepalive/derived/closed`，并在单次事务式 append 中记录 parent、直接 sibling、直接 child 的关系快照；恢复时从事件重建未闭合节点和待处理派生任务。
- `src/protocol.rs`：已有 `MailboxRequest::{Send,Receive,Acknowledge,Claim,Complete}`、`MailboxOutcome`（约 `L81-L113`）和 `MAX_MAILBOX_TEXT_BYTES`/TTL 限制，是可复用的消息/邮箱协议。建议扩展带 `relation`（`Parent/Grandparent/Sibling/Child`）、`node_id`、`correlation_id`、`generation` 的 DAG 消息体，保持现有 envelope 校验、大小上限和显式 claim/complete。事件协议可沿用 `StdioEvent`，增加 `DagEvent`/`NodeLifecycle`，使 headless 客户端能观察 keepalive 和 derived worker。
- `src/headless.rs`：现有 `Command::Mailbox` 分派和 `handle_mailbox_request`（约 `L6392-L6478`、`L8943-L9078`）已经把 JSONL 请求转换到 session mailbox；`MAX_PENDING_STEERS`、异步 provider/agent 有界邮箱和 reconnect replay（文件前部约 `L33-L78`）提供背压、重连和事件重放。建议在同一 dispatch 层加入 `dag_send/dag

## AC-031 codex/codex-rs/core/src/tasks/review.rs
zenpi 已有的最小复用面与建议落点如下。

- **DAG 拓扑与会话父子关系**：`src/dag.rs:L1-L10` 已明确“一个 worker 持有一个 DAG 节点、可与 parent/grandparent/direct sibling/direct child 通信、仅全 descendant green 才能 close”。`DagNode` 的 `parent/children/status/worker` 字段在 `L57-L73`，`DagStore::recipients` 在 `L279-L347` 精确覆盖 `parent`、`grandparent`、`siblings`、`children`、`all`；因此 review 子代理的会话关系应把 DAG node id 作为 worker identity，把 `DagNode.parent` 作为会话父级，把 children 列表作为直接子级，grandparent/sibling 通过只读拓扑解析而非复制树。
- **消息/邮箱/事件原语**：`DagStore::send`/`inbox`/`wait_for_message`（`dag.rs:L349-L384`、`L547-L565`）适合节点间短消息和轮询取消；它们有 `MAX_DAG_MESSAGES`、`MAX_DAG_BODY_BYTES` 上限。跨进程、需恢复的请求则复用 `src/protocol.rs:L81-L117` 的 `MailboxRequest::{Send,Receive,Acknowledge,Claim,Complete}` 与 `L544-L585` 的 ID、TTL、正文校验，避免另造 review 协议。实时父会话事件沿 `src/core.rs:L281-L325` 的 `AgentEvent`（`AssistantMessage`、`Provider`、`Error` 等）并由 `src/headless.rs:L9145-L9200` 的事件投影/回放通道输出。
- **耐久邮箱与 claim/完成**：`src/session.rs:L3171-L3272` 的 `MailboxMessage`、状态和 `MailboxAction` 是可复用的请求-结果协议；`SessionMailbox::enqueue/list/update` 位于 `L3640-L3743`，`LiveSessionRegistry` 位于 `L3274-L3359`。`src/headless.rs:L8954-L9093` 已实现 recipient 定位、TTL enqueue、receive、live-owner claim、带 reply 的 complete。DAG worker 向 parent、grandparent、sibling、child 发消息时，应将 `DagStore::recipients` 解析出的每个目标映射为目标 `session_id`，短期通知走 `DagStore`，需跨重启交付走 `SessionMailbox`；结果必须带原 `request_id`/digest。
- **最小保活与派生机制**：用 `DagStore::can_close`（`dag.rs:L395-L425`）作为唯一 close gate：节点自身状态必须为 `green`，深度遍历的每个 child/grandchild 也必须 `green`；返回 unfinished 列表时，worker 保持 `open`/活跃，读取 inbox 并为新工作调用 `spawn_worker`（`dag.rs:L446-L496`），后续工作通过保持打开的 stdin 的 `send_to_worker`（`L518-L533`）发送。`spawn_worker` 已设置 `ZENPI_DAG_NODE`/`ZENPI_DAG_STORE` 并使用 headless session；这就是“保活并派生新 worker”的最小可执行机制。close 成功后才调用 `Agent::close`/终端事件，不得仅因本节点 green 提前关闭。
- **`src/

## AC-032 codex/codex-rs/core/src/tasks/undo.rs
zenpi 已有可复用的通信原语是：`src/protocol.rs` 的 `MailboxRequest`（Send/Receive/Acknowledge/Claim/Complete）、`MailboxOutcome`、`CheckpointRequest`、`StdioEvent`；`src/session.rs` 的 `SessionMailbox` 将消息按 recipient session、digest、TTL、sequence 持久化，并用 claim/complete 防止隐式重试；`SessionStore::append_event`/`append_turn`/`tree_snapshot` 可作为事件和 DAG 状态日志。对应 Undo 的 started/completed 事件，应采用带 `operation_id`、`node_id`、`parent_id`、`outcome` 的结构化事件，而不是仅依赖字符串消息。

DAG 节点通信建议新增 `src/dag.rs`（或 `session_tree.rs` 的扩展）：定义 `NodeId`、`NodeState::{Queued,Running,Green,Failed,Cancelled,Blocked,Closed}`、`NodeLinks{parent,grandparent,children}`、`DagMessage` 和 `DagEvent`。worker 负责节点时，通信路由必须允许向 parent、grandparent、直接 sibling、直接 child 发送；路由先由 `SessionMailbox` 做同 workspace 与 session 身份校验，再由 `protocol.rs` 校验 ID、TTL、正文上限。直接 sibling 可由共同 parent 的 child 集合解析，不能让客户端任意伪造 sender 身份。

会话父子关系可复用 `core.rs::Turn::parent_id`（L146-L180 附近的既有模型）保存对话父节点，但 DAG 拓扑不能只靠线性 turns 推导；应在 `SessionStore` 记录 `dag_node_created`、`dag_edge_added`、`dag_node_green`、`dag_close_requested`、`dag_worker_derived` 事件，并通过 `tree_snapshot/tree_ancestry_turns` 或新查询函数得到祖先、直接子节点和兄弟集合。grandparent 是 parent 的 parent，缺失时通信应返回明确的 not-found，而不是降级广播。

保活与派生的最小机制：在 `core.rs` 增加 `DagWorkerLease{node_id, lease_id, expires_at_ms, policy_digest}`、`renew_node_lease`、`cancel_node_worker`、`settle_node_worker`；借鉴现有 `settle_blueprint_worker`、`renew_blueprint_worker`、`cancel_blueprint_worker` 的 durable ledger 语义。节点只有在自身为 Green 且其全部直接/间接 child/grandchild 均为 Green（无 Running、Queued、Failed、Blocked、UnknownOutcome）时才允许 `close_node`；否则保持 lease/Running（或 WaitingChildren），并由 `derive_worker(node_id, work_item)` 产生带 parent 链接的新 worker。派生必须写入 durable intent 后再提交运行时，避免“事件已发但 worker 未创建”造成假绿。

落点对应关系如下：`src/runtime.rs` 的 `BackgroundRunner`、有界 mpsc、`CancellationToken`、`RuntimeEvent::{Accepted,Star

## AC-033 codex/codex-rs/core/src/tasks/user_shell.rs
zenpi 已有的 `src/dag.rs` 是最直接的目标落点。其模块注释 `L1-L10` 已明确“一 worker 一个 DAG 节点”，可与 parent、grandparent、direct sibling、direct child 通信；只有节点自身及全部 descendant 为 green 才能 close，否则保活并派生新 worker。`DagNode`（`src/dag.rs:L57-L77`）有 `parent`、`children`、`status(open|green|red)`、`worker` 和 `worker_heartbeat_ms`；`DagRelation`（`L79-L112`）将邻居关系类型化。`DagStore::recipients`（`L332-L400`）的 `all` 正好解析为 parent + grandparent + direct siblings + direct children，`send/inbox`（`L402-L437`）是消息/邮箱原语，文件锁和有界消息体（`MAX_DAG_BODY_BYTES`）保证并发快照不会互相覆盖。

可复用的通信原语应分三层：一是 `DagStore::send`/`inbox`/`wait_for_message`（`src/dag.rs:L600-L618`）作为跨进程 durable mailbox/event；二是 `src/session.rs` 的 `SessionMailbox`，它在 `L3570-L3592` 定义私有、按 recipient session 定址的 JSONL 邮箱，在 `L3640-L3697` enqueue、`L3700-L3740` 分页 list、`L3743-L3795` 做 acknowledge/claim/complete/fail 状态迁移；三是 `src/runtime.rs` 的 `BackgroundRunner` bounded `mpsc` 与 `CancellationToken`（`L236-L254`、`L712-L759`），作为进程内 worker 事件流。`src/core.rs:L281-L325` 的 `AgentEvent`、`LiveToolEventSink`（`L474-L503`）可承载进度/结果事件，但不能替代 durable mailbox。

会话父子关系要明确分层：`src/core.rs:L140-L204` 的 `Turn.parent_id` 只表示对话/turn 相关性；`src/session_tree.rs:L50-L73` 的 `TreeEntry.parent_id` 是 transcript entry graph；DAG worker 拓扑应继续由 `DagNode.parent/children` 表达，不能把 `Turn.parent_id` 当作 worker parent。建议在 `DagNode` 增加 `session_id`、`lease_id`、`generation`，建立 node→session 的不可变绑定；`src/session.rs:L3274-L3355` 的 `LiveSessionRegistry` 可保存 session owner、`owner_epoch`、workspace 和 heartbeat，并拒绝过期 owner。

最小保活/派生机制已有大部分零件：`DagStore::touch_worker`（`src/dag.rs:L320-L329`）写心跳，`wait_for_message` 最多轮询 30 秒且接受取消，`spawn_worker`（`L499-L550`）启动 `zenpi-dev --auto --mode headless`，保留 stdin 以便 `send_to_worker`（`L571-L585`）发送 follow-up；`status_view` 会展示 heartbeat/未完成 descendant。建议在 worker 主循环固定每 `T` 秒调用 `touch_worker`，收到取消或父节点关闭后退出；新工作先 `se

## AC-034 codex/codex-rs/protocol/src/approvals.rs
现有 `src/approval.rs` 已有 `ApprovalRequest/ApprovalResponse/ApprovalCoordinator`（显式 host 决定、条件变量等待、`request_id` 去重和持久化前 accepted 队列），可直接承接 `ExecApprovalRequestEvent` 的 `call_id/approval_id/turn_id`，并把 `effective_approval_id` 作为关联 ID 规则。`src/protocol.rs` 已有版本化 JSONL、`ApprovalDecision` 字段、`MailboxRequest::{Send,Receive,Acknowledge,Claim,Complete}`、会话 ID 校验；因此可新增与本文件同构的 serde DTO（建议 `protocol::ApprovalEvent`, `NetworkApprovalContext`, `ElicitationRequest`），在 `Command`/事件编码处保留 snake_case/lowercase 和旧字段默认，避免把 wire 兼容逻辑塞进 coordinator。

DAG 通信映射必须显式覆盖四类邻接：worker 节点拥有 `parent_session_id`、`grandparent_session_id`、直接 sibling IDs、直接 child IDs；通过现有 `MailboxRequest::Send` 按 `recipient_session_id` 投递消息，`Receive` 拉取、`Claim/Complete` 形成消息状态机。建议在 `src/session.rs` 的 `SessionHeader`/事件投影增加可持久化的 `parent_session_id`, `node_id`, `depth`, `role` 与 `WorkerLease`（lease ID、过期时间、generation、状态），并以 append-only 事件记录 `worker_spawned`, `mailbox_sent`, `mailbox_completed`, `node_green`, `node_close_deferred`。`src/core.rs` 的 `Turn::parent_id`（`with_parent`/校验）可复用为会话内父子因果链；DAG 关系仍应放在 session 元数据/索引而非仅靠 turn 文本。由 session registry 建立父、祖父、兄弟、子节点的受信路由，客户端不可自行伪造 sender 权限。

最小保活与派生机制：在 `src/core.rs` 增加 `NodeStatus::{Running,Green,Closing,Alive,Failed}`、`WorkerLease` 和 `close_gate(node)`；`close_gate` 只有在节点自身为 `Green` 且所有直接/间接 child/grandchild 的聚合状态均为 `Green` 时才允许 `Closing -> Closed`，任一未绿节点都写 `node_close_deferred`、续租并保持 `Alive`。发现新工作时，先原子递增 generation、写入派生意图，再用 `runtime::Runtime::submit` 派生新 job；子节点完成后向 parent 回传 `node_green`，祖父通过 mailbox 聚合。新 worker 的 lease 过期或取消只能阻止结果接纳，不能回滚已经发生的工具副作用。

`src/runtime.rs` 提供 bounded command/event channel、FIFO follow-up、`CancellationToken`、`RuntimeEvent::{Accepted,Started,Queued,CancelRequested,Completed,Closed}`，适合作为每个 worker 的执行与保活心跳层；应增加一个定期 heartbeat/lease-renew 命令和按 parent/ch

## AC-035 codex/codex-rs/protocol/src/items.rs
可复用的抽象是“带类型标签的数据 item + 旧事件适配器”，而不是把 provider 副作用塞进 item。建议在 `src/protocol.rs` 增加 `DagItem`/`DagMessage`（serde `#[serde(tag="type")]`），字段至少含 `id: String`、`session_id`、`parent_id`、`grandparent_id`、`relation`（`Parent|Grandparent|Sibling|Child`）、`payload`、`created_at_ms`、`ttl_ms` 与 `status`；参考现有 `MailboxRequest::Send/Receive/Acknowledge/Claim/Complete`（`src/protocol.rs:L87-L113`）及 `Command::Mailbox`（L214-L263）。`MailboxRequest` 已有 recipient、message_id、TTL、游标、claim/complete/outcome，可直接作为 DAG 通信的线协议：发送方身份和 workspace 权限必须由 owner 注入，不能由客户端字段自报。

- **父子会话关系**：`src/core.rs` 的 `Turn { id, parent_id, ... }` 与 `with_parent/validate`（L140-L204）可复用为会话边；扩展 `SessionRelation { parent, grandparent, direct_sibling, direct_child }` 或在 `Turn.metadata` 写入受验证的 DAG 边。`src/session.rs` 的 `SessionStore::append_turn` 先校验、再写 journal、成功后才更新内存（L861-L891），适合记录 `DagNodeCreated`、`DagEdge`、`DagStatus` 和 worker lease。`tree_snapshot` 的取消检查与可恢复树读取（L894-L912）可作为祖先/后代闭包查询入口；建议新增 `descendant_status(node_id)`，返回直接 child 与 grandchild 的聚合状态。
- **最小通信原语**：同进程优先复用 `src/runtime.rs` 的 `BackgroundRunner`：有界 `sync_channel` 命令/事件邮箱、`try_submit`、`try_cancel`、`next_event`/`recv_timeout`、`Closed` 终态（L236-L365），每个 DAG worker 持有 `JobId` 和 `CancellationToken`。节点到 parent/grandparent/直接 sibling/直接 child 的消息统一走 `DagMailbox`（可由现有 `MailboxRequest` 落盘并由 runtime 投递）；事件层复用 `src/protocol.rs` 的 `StdioEvent { sequence, request_id, turn_id, event }`（L882-L918），使进度与终端响应可重放而不混淆。`src/headless.rs` 已有 `AsyncEventBuffer` 有界缓存、provider/agent 双队列和 `PendingSteer`（约 L806-L1081），可作为外部 host 的背压、steer 和丢失计数实现。
- **保活与派生的最小机制**：在建议的 `src/dag.rs` 定义 `DagNodeState::{Running, Green, Blocked, Failed, Closed}`、`DagNode { id, parent, children, state, worker: Option<JobId>, generation }` 和 `DagCoordinator::{send, claim, complete, evaluate_close, keep_alive, s

## AC-036 codex/codex-rs/protocol/src/lib.rs
`lib.rs` 的可复用思想是“根模块只装配稳定命名空间，行为下沉到专门模块”。zenpi 已有更丰富的协议实现，应把 DAG 编排能力组合到现有边界，而不是塞入 provider 代码。

- `src/protocol.rs` 是最直接的对照层：已有版本化 JSONL、`MailboxRequest`（`Send/Receive/Acknowledge/Claim/Complete`）、`CheckpointRequest`、`TreeAction`、`Command` 和严格字段校验。复用 `MailboxRequest` 作为 worker 通信原语：发送到 parent、grandparent、直接 sibling、直接 child 都只需解析为目标 `session_id` 的消息；消息必须带稳定 `message_id`、发送者 `session_id`、`item_id`/`goal_id`、类型和 TTL。用 `StdioEvent` 表示 `worker_started/heartbeat/message_received/child_green/close_blocked/worker_closed`，使事件可被 JSONL 客户端观察和重放。
- `src/session.rs` 是会话父子关系和持久化落点。建议增加可序列化的 `DagNodeRecord { session_id, parent_session_id: Option<String>, goal_id, item_id, state, generation }` 与 `DagNodeState::{Pending,Running,Green,Failed,KeepAlive,Closed}`，通过 append-only event 记录 `dag_node_created`, `dag_edge_bound`, `dag_state_changed`, `dag_keepalive`, `dag_worker_spawned`。grandparent 由 parent 的 `parent_session_id` 得到；直接 sibling 定义为共享同一 `parent_session_id` 的节点；直接 child 定义为 `parent_session_id == self.session_id`。不要只依赖内存指针，重启后从事件恢复邻接索引。
- `src/core.rs` 适合放 `DagCoordinator`/`DagNodeHandle`。它可以复用已有 `Agent::register_live_owner`、`heartbeat_live_owner`、`claim_live_mailbox`、`finish_live_mailbox` 的所有权概念。最小 API 建议为 `send_relation_message`, `claim_relation_message`, `complete_relation_message`, `heartbeat_node`, `evaluate_close`, `derive_worker`。`evaluate_close(node)` 必须递归检查“节点自身为 `Green` 且全部 child/grandchild（即全部后代）为 `Green`”；任一后代未绿、失败、超时或仍有未完成 mailbox 时返回 `KeepAlive`，禁止 `Closed`。
- `src/runtime.rs` 提供线程与取消原语：现有 `BackgroundRunner::spawn`、`try_submit`、`try_cancel`、`try_shutdown_with_grace`、`CancellationToken` 和 `RuntimeEvent` 可承载 worker 生命周期。建议增加 `WorkerJob::RunDagNode`/`WorkerJob::DeriveChild` 或等价命令；派生只在 `evaluate_close` 返回 `KeepAlive` 且存在新工作时调用 `try_submit`。将 `Accepted/Started/Queued/Cance

## AC-037 codex/codex-rs/protocol/src/message_history.rs
**可复用通信原语。** `HistoryEntry` 可作为轻量消息载荷，放入 `src/protocol.rs` 的版本化 JSONL 协议；现有 `MailboxRequest::{Send,Receive,Acknowledge,Claim,Complete}`（`src/protocol.rs`）已经覆盖发送、分页接收、确认、租约式领取和完成结果，`src/session.rs::SessionMailbox` 负责加锁、追加日志、TTL、claim token 与幂等检查。事件流可复用 `src/runtime.rs::RuntimeEvent`（`Accepted/Started/Queued/CancelRequested/Completed/Closed`）及 `src/headless.rs` 的有界事件/replay mailbox；需要把 `HistoryEntry` 放进 `MailboxRequest::Send.text` 或新增结构化 payload 时，应保留 `conversation_id`、`ts`、`text` 三字段并在协议层增加大小和标识校验。协议入口是 `src/protocol.rs::StdioRequest::into_command`，因此通信不会绕过现有边界。

**会话父子关系与 DAG。** `src/core.rs::Turn` 已有 `parent_id: Option<String>`，`Turn::with_parent` 可保存直接父节点；`src/runtime.rs::InputBoundaryGate::with_context_parent` 可携带模型回合的上下文父标识；`src/session_tree.rs::SessionTree`/`src/session.rs` 已有可恢复树、active leaf、分支和 `TreeAction::{List,Select,Fork,Annotate}`。建议新增一个可持久化的 `DagNode`（建议落在 `src/session.rs` 或独立 `src/dag.rs`），至少含 `node_id`、`parent_id`、`grandparent_id`（可由 ancestry 推导）、`status`、`child_ids`、`worker_session_id`、`lease_id` 与 `green`/`close_requested` 状态；边关系以 `SessionTree` 的追加事件形式保存，避免只存在内存。直接 sibling 用同一 `parent_id` 查询，直接 child 用反向索引查询；grandparent 沿父链上溯一跳即可。`HistoryEntry.conversation_id` 应映射到 `worker_session_id` 或独立 `conversation_id`，不能假设它天然等于 DAG 节点 ID。

**最小保活与派生机制。** 复用 `src/session.rs::LiveSessionRegistry::{register,heartbeat,active,unregister}` 作为 worker liveness；复用 `SessionMailbox` 的 TTL 和 claim token 作为定向消息租约。建议在 `src/core.rs::Agent` 增加（或由新的 DAG owner 持有）`heartbeat_node(node_id, lease_id, now_ms)`、`send_node_message(target, HistoryEntry)`、`evaluate_close(node_id)`、`spawn_child(node_id, work)` 四个薄接口：`heartbeat_node` 更新 owner/lease；`send_node_message` 只允许 parent、grandparent、直接 sibling、直接 child 四类经关系校验的目标；`spawn_child` 先向 `src/session.rs` 追加 `node_spawned`/lease 事件，再调用 `src/ru

## AC-038 codex/codex-rs/protocol/src/plan_tool.rs
建议把本文件的轻量协议语义嵌入已有模块，而不是复制一个孤立 TODO 状态：

- `src/protocol.rs`：现有 `MailboxOutcome`（`L79-L85`）、`MailboxRequest`（`L89-L111`）和 `Command::Mailbox`（`L211-L252`）已经提供消息/邮箱原语；`validate_mailbox`（`L544-L583`）及文本上限（`L585-L593`）提供边界。可新增 `PlanStatus`（snake_case）、`PlanItem`、`UpdatePlanRequest`，复用 `deny_unknown_fields`、ID/文本上限，并增加 `node_id`、`parent_id`、`lease_id`、`revision`。`Send/Receive/Claim/Complete` 可承载 worker 到 parent、grandparent、直接 sibling、直接 child 的定向消息；授权应由 mailbox owner 校验，不能让客户端自行伪造 sender。
- `src/session.rs`：`SessionStore` 是可恢复的追加式投影（`L145-L160`），已有 `SessionTree`；`selected_tree_turns` 会按当前叶子取祖先路径（`L1293-L1302`），并明确 sibling/descendant checkpoint 不应混入当前上下文（`L1326-L1335`）。建议添加持久 `DagNodeRecord { node_id, parent_id, status, children, lease_id, generation }` 与 `PlanEvent`，每次状态、派生、心跳、邮箱 claim/complete 都 append event。由 `parent_id` 计算 parent/grandparent；同一 `parent_id` 的节点是直接 sibling；children 表给出直接 child，递归遍历给出全部 child/grandchild。恢复时重放事件并拒绝旧 revision，避免崩溃后误判完成。
- `src/runtime.rs`：`BackgroundRunner` 的 `try_submit`、有界 `RuntimeConfig`（`L101-L115`、`L308-L315`）适合派生 worker 和限制待处理新工作；`CancellationToken` 是幂等取消原语（`L51-L63`），`JobOutcome` 明确 `Succeeded/Failed/Cancelled/Panicked`（`L156-L168`）。建议每个 DAG node worker 持有 `NodeLease` 和 `CancellationToken`，用 `try_submit(DeriveWorker { parent, work })` 派生，`QueueFull/Closed` 变成可观测事件而不是静默丢失；取消只停止计算，不回滚已发送消息或外部副作用。现有 `InputBoundaryGate::with_context_parent`（`L794-L830`）可复用上下文父关系，但需扩展为 DAG node identity。
- `src/headless.rs`：该模块拥有 JSONL wire I/O，并为请求提供相关响应、事件和重放；适合把 `PlanEvent`、邮箱回执、heartbeat/lease 状态作为带 request ID 的 `AgentEvent`。输入线解析、限流和 replay ledger 应保持现有边界；客户端断线恢复只重放已持久化事件，不能凭最后一条 stdout 推断节点已 close。
- `src/core.rs`：`Agent` 的 admission 边界区分 `Started`、`Steered`、`NotSubmitted`，拒绝输入不触达 backend；`Turn` 的 `parent_id` 是现成会话父子关联（约 `L176-L204`），`WorkerExecutionBin

## AC-039 codex/codex-rs/protocol/src/protocol.rs
zenpi 已有模块的可复用落点（按实际接口核对）：

- `src/protocol.rs` 已有 JSONL `StdioRequest/Command`、`MailboxRequest`、`CheckpointRequest`、`InputQueueRequest`、`TreeRequest`、`OutputRequest`、`StdioResponse/StdioEvent`、`parse_line/encode_line/command_name` 和严格 `ProtocolError`（当前约 `L1-L1309`）。它对应源的 `Submission/Op`、`Event/EventMsg`、稳定错误码和 tagged payload。应新增 `DagNodeId`、`DagRelation`、`DagMessageKind`、`DagMailboxRequest/Reply`、`DagEvent`，沿现有 `serde(deny_unknown_fields)` 与版本校验；复用 `StdioEvent.sequence`、request/turn id 关联，避免另造无序 stdout 通道。
- `src/session.rs` 已有 append-only journal、`SessionStore`、`LiveSessionRegistry` 与持久 `SessionMailbox`：邮箱消息有 sender/recipient session id、request id、digest、TTL、Queued/Acknowledged/Claimed/Succeeded/Failed/Expired 状态、claim token、幂等冲突检测和锁（约 L3171-L3560）。这是源 `Event` correlation、`SessionMeta/InitialHistory`、`Collab*` 的最佳基础。建议在 mailbox payload 中增加 `dag_node_id`、`parent_id`、`relation`、`generation`、`work_epoch`；每次 claim/finish 都追加 journal 事件，利用现有 digest/TTL/transition 规则实现崩溃恢复。
- `src/core.rs` 的 `Turn { id, parent_id, role, content, metadata }`、`Turn::with_parent/validate`（约 L141-L205）已经表达会话父子关系；`AgentEvent` 有 `TurnAccepted/AssistantMessage/ToolCall/ToolResult/Provider/Error/Warning`，`AgentPhase::{Idle,Running,Closed}`，且 `Agent::register_live_owner/heartbeat_live_owner/claim_live_mailbox/finish_live_mailbox(_with_reply)`（约 L819-L910）能把工作和 mailbox 绑定。建议新增 `DagNodeState { node_id, parent, children, status, green_epoch }` 与 `DagCoordinator`，在 `AgentEvent::Handoff/ToolResult` 处发出 DAG 事件。
- `src/runtime.rs` 的 `CancellationToken` 是最小协作取消原语；`BackgroundRunner` 使用有界 command/event `sync_channel`，提供 `try_cancel/next_event/recv_timeout/try_next_event/join/close_with_grace`，事件序列为 Accepted/CancelRequested/Completed/Closed（约 L40-L430）。可直接把每个 DAG worker 作为一个 `BackgroundRunner` job：mailbox rec

## AC-040 codex/codex-rs/protocol/src/request_user_input.rs
对照现有实现：`src/protocol.rs` 已有版本化 JSONL 边界、`MAX_ID_BYTES`/文本上限和 `MailboxRequest::{Send,Receive,Acknowledge,Claim,Complete}`（`src/protocol.rs:L20-L35,L79-L113`），并把它们纳入 `StdioRequest`/`Command`（`L152-L263`、`L265-L518`）；邮箱字段校验在 `validate_mailbox`（`L544-L598`）。这比源文件的纯 DTO 更严格，可复用为 DAG 节点通信协议。`src/session.rs` 的 `MailboxMessage` 保留 sender/recipient、request id、digest、TTL、sequence、status、claim token、result（`L3180-L3255`），`LiveSessionRegistry` 提供带 `owner_epoch` 的 register/heartbeat/active/unregister 与过期清理（`L3274-L3358`），`claim_next`/`claim_message`/`finish_claim`/`finish_claim_with_reply` 实现显式领取、完成和回信（`L3361-L3560`）；这是可靠 worker 邮箱的最小持久化原语。`src/core.rs` 的 `AgentEvent` 提供 Turn、Handoff、Tool、Provider、Error、Warning 事件（`L281-L325`），并暴露 live owner 与 mailbox 包装方法（`L821-L914`）。`src/headless.rs` 仅拥有 wire I/O、异步事件缓冲、请求相关响应与 runtime runner 接线（模块注释 `L1-L5`，事件队列相关 `L863-L1051`），适合把用户问题事件投影到 JSONL；不要把 DAG 判定塞进 I/O 层。`src/runtime.rs` 的 `BackgroundRunner` 使用有界 command/event channel，`RuntimeConfig` 默认容量为 32/128/32，`CancellationToken` 合作式取消（`L49-L193`、`L236-L421`），可直接作为派生 worker 调度器；`InputBoundaryGate` 具有 `context_parent`、epoch 失效和安全输入边界（`L765-L912`），可承载节点 turn 的父上下文。`src/tool_runtime.rs` 明确工具批次由 owner 负责策略、日志、结果持久化且不会隐式重试（`L1-L7`、`L157-L176`），所以 DAG 完成判定应在 owner/core 层，而不是工具线程。`src/approval.rs` 的 `ApprovalCoordinator` 是 bounded 条件变量 rendezvous，取消会清除 pending/decision（`L126-L240`），可复用作需要人工回答的节点门；`src/providers/**`（`providers/mod.rs:L1-L50`、`registry.rs:L146-L159`）只负责 provider protocol、route、model 定义，不应保存 DAG 拓扑或 worker 存活状态。

建议的可执行落点与最小机制如下：

1. 在 `src/protocol.rs` 增加与本文件同形的 `RequestUserInputQuestionOption`、`RequestUserInputQuestion`、`RequestUserInputArgs`、`RequestUserInputAnswer`、`RequestUserInputResponse`、`RequestUserInputEvent`，保留 `isOther`/`isSecret`/`turn_id` 的 serde 兼容规则；新增 `DagMessage`/`DagEve

## AC-041 opencode/packages/opencode/src/acp/agent.ts
### 可复用原语与协议落点

1. **消息/邮箱**：优先复用 `src/protocol.rs` 的版本化 JSONL 与 `MailboxRequest`（`Send`、`Receive`、`Acknowledge`、`Claim`、`Complete`，`L79-L113`），而不是另造 ACP 专用队列。`Command::Mailbox`（`L242-L255`、解析在 `L479-L484`）是 host 入口；`validate_mailbox` 的 ID、文本、TTL 和分页上限（`L544-L590`）可作为 DAG 消息的硬边界。`src/session.rs` 的 `SessionMailbox` 是追加式、同 inode 锁、每次 mutation reload+append 的持久邮箱（`L3570-L3591`）；消息自带 sender/recipient/session、request ID、digest、TTL 和状态，`enqueue/list/update` 位于 `L3617-L3795`。因此 worker 节点可用 `session_id` 作为地址，`message_id`/digest 作为幂等键。
2. **事件/流**：`src/protocol.rs` 的 `StdioEvent`（`L885-L918`）适合承载节点 `accepted/started/progress/green/blocked/closed` 事件；`src/core.rs` 的 `AgentEvent`（`L283-L325`）已有 `TurnAccepted`、`ToolProgress`、`Handoff`、`Error` 等语义。`src/headless.rs` 负责把 agent/provider 事件转成关联的 JSONL 响应，适合作为 ACP `AgentSideConnection.sessionUpdate` 的 Rust 等价宿主。
3. **运行协议/取消**：`src/runtime.rs` 的 `BackgroundRunner` 以有界 command/event channel 启动 worker（`L272-L306`），`RuntimeEvent` 提供 `Accepted`、`Started`、`Queued`、`CancelRequested`、`Completed`、`Rejected`、`Closed`（`L171-L193`）；`CancellationToken` 是协作式、幂等取消（`L49-L99`）。这对应 TS 的 `run`/`cancel` 边界，但必须由 worker 主动轮询 token，不能假设线程被强杀。
4. **会话关系**：`src/core.rs::Turn` 的 `parent_id`（`L146-L191`）和 `src/session.rs` 的 tree ancestry/fork（`tree_ancestry_turns`、`fork_at_tree_leaf`，约 `L1038-L1136`）可提供历史上的父链。`TreeAction::Fork`（`src/protocol.rs:L1161-L1207`）以及 `Agent::tree_request`（`src/core.rs:L922-L1001`）已有显式 fork/选择接口；它们应成为 DAG 节点建立或派生 worker 的审计入口，而不是在 `ACP Agent` 包装层隐式 fork。
5. **保活**：`src/session.rs::LiveSessionRegistry`（`L3274-L3355`）已提供带 `owner_epoch` 的 `register`、`heartbeat`、TTL `active` 和 `unregister`；`claim_next`/`claim_message`（`L3361-L3438`）只做一次认领、不自动启动调度器。worker lease 还可用 `src/core.rs::renew_blueprint_worker`（`L2846-L2868`），取消用 `cancel_blueprint_wor

## AC-042 opencode/packages/opencode/src/acp/config-option.ts
这份源文件本身只提供“选择器/格式化器”，不提供 DAG 通信；zenpi 应复用它的确定性回退思想，同时把通信和生命周期放到已有会话/运行时层。

**配置与 provider 对照。** `src/core.rs` 的 `Agent::set_model`（约 `L1177-L1200`）和 `set_reasoning_effort`（`L1202-L1225`）是当前模型与 effort 的状态入口；`src/providers/**` 的 registry/connection 暴露模型能力（包括 reasoning effort）。建议新增 `src/config_option.rs`：定义 `ConfigOptionModel/Provider/Mode/ModelSelection` 的 Rust 等价物和 `select_variant`、`format_current_model_id`、`build_config_options`，用 `Option` 表达源函数的 `undefined`，用 `Vec` 保持顺序，并为 `localeCompare` 选择明确的 Unicode/字节排序规则。`Agent` 只在验证后调用这些纯函数，再把结果提交到现有 model/effort setter；不要把 DAG 状态塞进 provider client。

**可复用通信原语。**

1. 持久点对点消息使用 `src/protocol.rs` 的 `MailboxRequest::{Send,Receive,Claim,Complete}`（`L81-L115`）和 `src/session.rs` 的 `SessionMailbox::enqueue/list/update`（约 `L3574-L3750`）；`src/headless.rs::mailbox_slash_view`（`L2066-L2112`、执行细节约 `L8940-L9100`）已提供入口。给每个 DAG worker 一个 session id，发送目标只允许同一 session 目录中已登记的 id。parent、grandparent、直接 sibling、直接 child 都通过同一 mailbox 原语寻址；sibling 地址由 DAG 索引解析，不能由 wire path 或未验证文本伪造。
2. 进程内低延迟事件使用 `src/runtime.rs` 的 `BackgroundRunner`、有界 command/event channel 和 `RuntimeEvent::{Accepted,Started,Queued,CancelRequested,Completed,Closed}`（`L56-L76`、`L174-L195`、`L237-L300`）。provider/tool 增量和节点状态广播放入 job 事件；持久 mailbox 只承载需要跨进程/重连的命令与结果，避免把 UI 输出当作可靠队列。
3. 事实审计用 `SessionStore::append_event`、`append_turn`/`append_handoff`；父子关系可利用 `core.rs::Turn::parent_id`（约 `L141-L200`）及现有 session tree 投影（`SessionStore::tree_snapshot/tree_ancestry_turns`，约 `L895-L1068`）。建议新增严格事件类型 `dag_node_started/heartbeat/status/child_spawned/close_decided`，每条含 `session_id,node_id,parent_id,operation_id,epoch`，并用 `operation_started/finished` 的幂等模式保障重放。

**会话父子关系与 DAG 视图。** 新增 `DagNodeId`、`DagNodeRecord { node_id, session_id, parent: Option<DagNodeId>, children: Vec<DagNodeId>, state }` 和

## AC-043 opencode/packages/opencode/src/acp/content.ts
这里的核心可复用思想不是 ACP 字段本身，而是“有类型的消息/资源转换 + 明确的关系与终态”。建议如下：

- **消息/邮箱/事件/协议原语**：将 `PromptPart`/`ReplayPart` 对应为 `protocol.rs` 的带 `serde(tag)` 消息枚举；现有 `MailboxRequest::{Send,Receive,Acknowledge,Claim,Complete}`（`src/protocol.rs:L79-L113`）已覆盖 DAG 节点间消息投递，`MAX_MAILBOX_TEXT_BYTES`/TTL（L32-L37）提供边界。`SessionMailbox` 的追加式、带 digest/sequence/status 的消息（`src/session.rs:L3180-L3255`）可承载 text/file 元数据；`RuntimeEvent::{Accepted,Started,Queued,CancelRequested,Completed,Closed}`（`src/runtime.rs:L171-L193`）是事件流。可新增 `DagMessage { message_id, sender, recipient, relation, payload: ContentPayload, ttl_ms }` 与 `DagEvent { node_id, kind, sequence }`，在 `protocol.rs` 增加严格验证，沿用 JSONL `StdioEvent`（`src/protocol.rs:L882-L917`）。
- **会话父子关系**：`SessionHeader.session_id` 与 `SessionStore` 的 append-only journal（`src/session.rs:L35-L41、L144-L160`）是稳定身份；`core::Turn.parent_id` 已保存父 turn（`src/core.rs:L140-L175`），`InputBoundaryGate.context_parent` 可携带当前上下文父节点（`src/runtime.rs:L765-L831`）。建议在 `SessionHeader` 或独立 `DagNodeRecord` 增加 `parent_session_id`, `grandparent_session_id`（可由父链计算）、`relation_to_parent`，并用 session_id 建索引：parent/grandparent 是向上路由，直接 sibling 是共享 parent 的子集合，direct child 是 `parent_session_id == self` 的集合；禁止从客户端任意 sender 字段推断权限，复用 mailbox 的 workspace/access 校验（`src/session.rs:L3598-L3614`）。
- **worker 负责节点的最小通信机制**：发送给 parent、grandparent、直接 sibling、direct child 都统一调用 `SessionMailbox::enqueue`/`MailboxRequest::Send`，消息含 `relation` 与 `in_reply_to`；接收端以 `Receive → Claim → Complete` 形成显式状态机。现有 headless 分发已把 mailbox 命令接到 `mailbox_response`（`src/headless.rs:L8630-L8637`），`execute_mailbox` 对 recipient 做唯一会话解析并返回 `execution_started:false`（`src/headless.rs:L8954-L8985`），可在此加 DAG 关系授权与路由，不启动隐式 worker。
- **保活（keepalive）**：直接复用 `LiveSessionRegistry::{register,heartbeat,unregister,active}`（`src/session.rs:L3274-L

## AC-044 opencode/packages/opencode/src/acp/directory.ts
目标是把 ACP 的“目录快照 + 按作用域缓存”思想转成 DAG worker 的通信/生命周期服务。zenpi 已有可复用原语：src/protocol.rs 定义版本化 JSONL 与 MailboxRequest::{Send,Receive,Acknowledge,Claim,Complete}（约 L1-L110），src/headless.rs 有异步事件邮箱、Mailbox 分发和请求关联（如 L4014-L4170、L6392-L6405、L8943-L9000），src/session.rs 提供追加式 JSONL、事件恢复和 session tree（L145-L160、L733-L799、L894-L1089），src/runtime.rs 的 BackgroundRunner、CancellationToken、有界命令/事件通道和 RuntimeEvent::{Accepted,Started,Queued,CancelRequested,Completed,Closed}（L40-L237）可承载 worker 运行；src/core.rs 的 Turn.parent_id、worker binding/lease、register_live_owner/claim_live_mailbox/finish_live_mailbox（约 L130-L170、L821-L918）适合作为父子会话与租约入口；src/approval.rs 的 ApprovalCoordinator 可复用于跨 worker 的需要人工决策事件；src/tool_runtime.rs 的批处理与取消结果可用于 close 前副作用收敛；src/providers/** 的 ModelRegistry/ProviderDefinition/ProviderConnection 只负责 provider/model 能力，不应承担 DAG 拓扑。

建议落点与最小机制：

- 在 src/core.rs 增加 DagNodeId、DagRelation（Parent、Grandparent、DirectSibling、DirectChild）及 DagNodeState（Running、CloseRequested、Closed、KeepAlive）。每个 worker session 记录 session_id、parent_session_id、goal_id/item_id、租约与当前节点状态；复用现有 Turn.parent_id 表达对话父项，但 DAG 边应单独持久化，避免把“会话父子”误当作全部 DAG 关系。
- 在 src/protocol.rs 扩展现有 mailbox 协议而非新造传输：消息增加 sender_session_id、relation、correlation_id、epoch/lease_id、kind（work/status/cancel/close/heartbeat）和 bounded payload。发送方只能指定 receiver session；host 根据 session tree 计算 parent、grandparent、直接 sibling、直接 child，禁止任意祖先或非直接后代越权。沿用 ttl_ms、Claim、Complete 与 MailboxOutcome，实现幂等 message ID。
- 在 src/session.rs 增加可恢复的 DagNodeRecord、DagEdgeRecord、WorkerLeaseRecord、WorkerStatusEvent；用 append-only event 记录 spawned、heartbeat、green、close_requested、closed、keep_alive、derived。恢复时重建内存 adjacency 和 mailbox 未完成集合。节点关系至少持久化 parent、直接 children、兄弟计算所需 parent children 集合以及 grandparent 引用。
- 在 src/runtime.rs 复用 BackgroundRunner 为每个可运行 worker 提供有

## AC-045 opencode/packages/opencode/src/acp/error.ts
现有基础可直接复用：src/error.rs:14-L31 的 ZenpiError 是顶层错误枚举；src/core.rs:354-L399 的 AgentError 已有稳定 code() 分类；src/protocol.rs:79-L113 已有 MailboxOutcome/MailboxRequest（Send/Receive/Acknowledge/Claim/Complete），src/protocol.rs:152-L209 的 StdioRequest 负责相关协议字段；src/core.rs:281-L325 的 AgentEvent 可承载错误/警告事件；src/runtime.rs:49-L100 的 CancellationToken 和 src/tool_runtime.rs:157-L176、L252-L347 的协作取消与 join 语义可作为 worker 生命周期基础；src/approval.rs:126-L245 的 ApprovalCoordinator 提供有界等待、取消轮询和宿主响应 rendezvous；src/session.rs 的 append-only journal（错误类型位于 L98-L114）可承载恢复记录；src/headless.rs:540-L715 将解析/调度错误变成带 code 的 StdioResponse。

建议落点与可执行差异：
1. 在 src/protocol.rs 增加 AcpError/AcpErrorKind（或在 src/error.rs 增加同名枚举）九个变体：字段对应上述字符串，ServiceFailure { safe_message, service: Option<String>, error_name: Option<String> }。用 thiserror 保留人类消息，用 serde/显式 to_request_error 保证机器字段；验证空 ID、控制字符和长度时复用现有 validate_identifier（L617-L630）规则。
2. 在 src/protocol.rs 增加 AcpRequestError { code: i32, message: String, data: serde_json::Value } 与 impl From<AcpError>；映射码固定为 -32602/-32000/-32601/-32603，数据只放非空可选字段，等价于 TS L63-L93。未知枚举值返回 AcpError::Internal，禁止 Rust 中出现“静默返回 None”。
3. 在 src/headless.rs 的命令错误响应处调用该转换；在 src/core.rs 的 AgentError 增加 Acp(#[from] AcpError) 或清晰的 InvalidTurn/Session/Backend 到 ACP 错误的适配函数。src/runtime.rs 的 JobOutcome::Failed/RuntimeEvent::Failed（约 L158-L205）承载结构化错误，不要把 safe_message 与原始 defect 混合。
4. 通信原语按 DAG 需求复用并补最小协议：消息用 MailboxRequest::Send/Receive，事件用 AgentEvent::{Handoff, Error, Warning, ToolResult}，外部协议用 StdioRequest 的 mailbox 字段；src/approval.rs 的有界 Condvar 可复用为等待/唤醒模型。建议 DAGNodeId、parent_id: Option<DAGNodeId>、children: Vec<DAGNodeId> 放入 session/tree 状态（现有 Turn.parent_id 在 src/core.rs:140-L175 可作为父子关系样板），并由 src/session.rs journal 持久化节点状态和消息序号。
5. 会话父子关系：每个 worker session 记录 parent_session_id 与 root_session_id；直接 sib

## AC-046 opencode/packages/opencode/src/acp/event.ts
### 可复用通信原语与现有落点

- event.ts 的 Event 分派、sessionUpdate 与 per-session idle waiter 可映射到 src/core.rs 的 AgentEvent（TurnAccepted、AssistantMessage、ToolCall、ToolProgress、ToolResult、Provider、Error、Warning，L281-L325）和 src/headless.rs 的 AsyncEventBuffer。后者按请求保存 provider/agent 事件，按数量与字节上限丢弃并保留 admission/tool 生命周期事件（headless.rs:L814-L869,L940-L1061），比无界广播更适合 DAG。
- src/runtime.rs 的 BackgroundRunner 已提供有界 command/event channel、FIFO pending follow-up、JobId、Accepted/Started/Queued/CancelRequested/Completed/Closed（runtime.rs:L38-L40,L49-L99,L156-L199,L272-L365）。可把 ACP 的 runUntilIdle 变成 await_node_idle(node_id)，把 RuntimeEvent::Completed 作为一次 worker turn 结束，而不是 DAG 节点可关闭。
- src/session.rs 的 SessionMailbox/LiveSessionRegistry 是可复用的持久消息/邮箱原语：消息按 digest、sender/recipient、TTL 入队并经过 Queued -> Acknowledged/Claimed -> Succeeded/Failed 转换（session.rs:L3563-L3592,L3617-L3697,L3743-L3795）；注册、heartbeat、claim_next、finish_claim 绑定 owner_epoch（session.rs:L3285-L3438）。它可承载 parent、grandparent、直接 sibling、直接 child 的定址消息，避免进程内裸指针。
- src/session.rs::SessionStore 的 append-only append_event 与 ReconnectJournal 的 checksum/sequence WAL 可分别持久化 DAG 状态和断线事件（session.rs:L1205-L1225,L1908-L2021）。src/protocol.rs 已有 MailboxRequest、StdioEvent、StdioResponse 与严格字段校验（protocol.rs:L81-L110,L882-L923,L994-L1023），可扩展 DagMessage/DagStatus 而不另造无校验 JSON。
- src/approval.rs::ApprovalCoordinator 是等待/唤醒、一次性可见、首个响应获胜、持久化失败回撤、cancel_all 的条件变量 rendezvous（approval.rs:L126-L168,L194-L249,L294-L375,L409-L457）。它适合 DAG worker 的权限消息，但不能替代节点完成判定。
- src/tool_runtime.rs::execute_tool_batch 已有有界批量、准备阶段、取消传播、并行 worker join、未确定副作用阻止后续调用（tool_runtime.rs:L157-L176,L219-L249,L252-L346），可复用为新派生 worker 的工具执行边界；provider 适配器通过 backend::ProviderEvent 的 text/reasoning/tool delta 与取消检查产生流式事件（backend.rs:L238-L286,L384-L413），src/providers/** 只应继续负责协议/流解析，不直接操纵 DAG

## AC-047 opencode/packages/opencode/src/acp/permission.ts
### 可复用原语与会话关系

zenpi 已有的 `src/protocol.rs` 提供版本化 JSONL 协议（`StdioRequest`/`Command`）及 `MailboxRequest::{Send,Receive,Acknowledge,Claim,Complete}`（`src/protocol.rs:L87-L113`），适合作为 permission 事件的线协议；`src/runtime.rs` 的 `BackgroundRunner`、有界 `sync_channel`、`RuntimeEvent::{Accepted,Started,Queued,CancelRequested,Completed,Closed}`（`src/runtime.rs:L156-L190,L236-L365`）可承载异步询问、结果和关闭事件。`src/core.rs` 的 `AgentEvent`、`take_events`、live tool mailbox（约 `L300-L325,L846-L889,L1390-L1391`）可把权限询问广播给 headless host，再把决定回送核心。

会话身份应使用 `SessionStore::session_id()` 和现有 `SessionMailbox` 的 sender/recipient 校验（`src/session.rs:L3171-L3219,L3260-L3476,L3574-L3608`）。父子 DAG 关系使用 `src/dag.rs` 的 `DagNode.parent/children`，不要把 transcript 的 `SessionTree.parent_id` 当成进程父子：`SessionTree` 维护记录分支和回放，DAG worker 维护执行节点、邮箱和进程生命周期。若一个 DAG 节点对应一个 session，建议在节点记录 `session_id` 与 `parent` 的双向索引，向上/横向/向下通信均通过节点关系解析。

### 对 permission.ts 的 Rust 类型/函数落点

- 在 `src/approval.rs` 增加 ACP/host 适配层（例如 `PermissionRequest`、`PermissionOption`、`PermissionReply`）。`ApprovalCoordinator::request/respond/respond_with_request/cancel_all`（`src/approval.rs:L131-L216,L293-L345,L409-L447`）已经提供 pending、首个决定获胜、取消唤醒和 fail-closed 基础；把 `once/always/reject` 映射到 `ApprovalDecision`，并保持 worker 决定不可记忆的既有约束。
- 在 `src/core.rs` 增加 `PermissionEvent` 到 `AgentEvent` 的构造函数：复用 `AgentEvent::ToolCall/ToolResult/Provider/Error` 的关联 `turn_id` 设计，把 `session_id`、`request_id`、`tool_call_id`、`metadata` 和候选选项写入事件；权限决定先经 `ApprovalCoordinator`，再由 core 在进入工具副作用前持久化。
- 在 `src/headless.rs` 的异步 dispatch/事件回放位置接入权限请求和决定。使用现有有界事件 mailbox、请求 ID replay 和 JSONL `StdioResponse/Event`，确保 UI 掉线时不把未确认请求当 allow；对应 `permission.ts` 的异常吞并应在 Rust 中变为可观测的 `Rejected`/`Error` 事件，同时默认拒绝。
- 在 `src/protocol.rs` 增加明确的 `Permission` 命令或扩展 `MailboxRequest`：载荷至少包含 `session_id`、`request_id`、`tool_

## AC-048 opencode/packages/opencode/src/acp/profile.ts
### 可直接复用的原语与现有落点

- **消息/邮箱**：`src/protocol.rs:L88-L109` 的 `MailboxRequest` 已覆盖 `Send`、`Receive`、`Acknowledge`、`Claim`、`Complete`，并在 `L544-L585` 限制 ID、文本、TTL 和分页；`src/session.rs:L3171-L3268` 的 `MailboxMessage`/`MailboxAction` 与 `L3574-L3950` 的 `SessionMailbox` 提供有界、带 digest、带序列、持久化 JSONL mailbox。DAG 节点间的 parent、grandparent、直接 sibling、直接 child 通信应复用该邮箱，不应为每种关系新造 transport。
- **事件**：`src/core.rs:L273-L325` 的 `AgentEvent` 可表达 turn、tool、provider、error；`src/runtime.rs:L158-L203` 的 `RuntimeEvent` 提供 `Accepted/Started/Queued/CancelRequested/Completed/Closed` 生命周期；`src/protocol.rs:L882-L918` 的 `StdioEvent` 提供带 `sequence`、`request_id`、`turn_id` 的 JSONL 事件封套。`src/headless.rs:L861-L1041` 的 `AsyncEventBuffer` 已按 provider/agent 分邮箱、按数量和字节限流，并优先保留 admission/tool 终态事件。
- **协议**：`src/protocol.rs` 是版本化 JSONL 边界（`PROTOCOL_VERSION` 为 2，`L20-L36`），`MailboxRequest`、`TreeRequest`（约 `L1161-L1210`）和 `OutputRequest` 可继续承载 DAG 控制，但应新增严格的 `WorkerRequest`/`WorkerEvent` 类型而不是把关系信息塞入自由文本。
- **会话父子关系**：`src/core.rs:L140-L175` 的 `Turn.parent_id` 只保证对话 turn 的父关系；`src/session.rs:L3274-L3359` 的 `LiveSessionRegistry` 管理 `session_id`、`owner_epoch`、workspace 与心跳；`L3361-L3488` 提供 claim/finish，`L3490-L3559` 可向原 sender 回信。这些是实现 worker 通信的基础，但当前没有显式的“grandparent/sibling/child DAG 节点”类型，不能把 `Turn.parent_id` 直接当作完整 worker 图。
- **取消与调度**：`src/runtime.rs:L49-L100` 的 `CancellationToken` 是可复用的协作取消原语；`BackgroundRunner` 在 `L236-L430` 提供有界 command/event channel、`try_submit`、`try_cancel` 和有界 shutdown grace。`src/headless.rs:L4467-L4636` 负责把异步请求接入 runner，`L6000-L6485` 等路径处理取消与 mailbox 命令。`src/core.rs:L3666-L3745` 的 `run_active_turn_cancelable_with_events` 还能在持久化最终答案前阻止晚到取消泄漏。
- **保活与租约**：`src/session.rs:L3274-L3355` 的 `register`/`heartbeat`/`active` 已能以 `owner_epoch + TTL` 判断在线；`src/core.rs:L2870-L2895` 的 blueprint worker

## AC-049 opencode/packages/opencode/src/acp/service.ts
### 可直接复用的通信原语

1. **消息/邮箱**：直接复用 `src/protocol.rs:L81-L113` 的 `MailboxOutcome` 与 `MailboxRequest::{Send,Receive,Acknowledge,Claim,Complete}`，以及 `src/protocol.rs:L479-L484` 的解析入口；不要另造一套 worker RPC。现有 `src/session.rs:L3171-L3272` 的 `MailboxStatus`、`MailboxMessage`、`MailboxAction` 已有 queued→acknowledged→claimed→succeeded/failed 的状态机、TTL 与 result。
2. **持久邮箱**：`src/session.rs:L3570-L3697` 的 `SessionMailbox::open/enqueue` 已做同 workspace、session identity、request id 幂等、digest、TTL 与 archived 检查；`L3700-L3795` 的 `list/update` 提供有界分页与不可隐式重试的状态转移。worker 之间的“请求新工作/询问父状态/回报绿或失败”应均落成 mailbox message，不用共享可变内存直接互调。
3. **活性/保活**：`src/session.rs:L3274-L3359` 的 `LiveSessionRegistry::{register,heartbeat,unregister,active}` 是最小 liveness lease。它目前只描述 session owner，不描述 DAG 节点；建议新增 `LiveDagWorker`（`node_id,parent_node_id,session_id,owner_epoch,last_seen_ms,expires_at_ms,state`）或将 node identity 作为 session owner 的受约束扩展。`heartbeat` 必须是幂等的；过期只阻止 claim/派生，不自动把工作标成成功。
4. **事件**：复用 `src/core.rs:L281-L325` 的 `AgentEvent` 与 `src/protocol.rs:L882-L917` 的异步事件 envelope，把 `DagWorkerStarted/Heartbeat/MessageReceived/Green/Failed/KeepAlive/Derived/CloseDeferred` 作为事件 payload。事件是可观察流，结论仍要先写 `SessionStore`；不能把“收到 Green 事件”当成 durable commit。
5. **协议**：复用 `src/protocol.rs:L152-L209` 的 `StdioRequest` 公共 envelope、`L211-L263` 的 `Command`、`L265-L517` 的版本/字段校验。建议在现有 `Command::Mailbox` 旁新增 `Command::Dag(DagRequest)`，或者先把 DAG 控制面编码为受限 mailbox payload；必须用 `MAX_ID_BYTES`、`MAX_MAILBOX_TEXT_BYTES`、TTL 上限（`L32-L37`）并拒绝未知字段。`headless.rs:L5323-L5503` 是异步请求解析/dispatch，`L6392-L6558` 已有 mailbox/cancel/resume 路由，适合挂 DAG 查询、keepalive 与 derive。

### 会话父子关系与 DAG 邻接

源 `forkSession`（`L362-L407`）虽然创建了 fork，却没有在 `service.ts` 保存 `parentSessionId`；zenpi 不能照搬这个隐式关系。建议在 `src/session_tree.rs` 或新 `src/dag.rs` 建立持久的 `DagNode`：`node_id`、`session

## AC-050 opencode/packages/opencode/src/acp/session.ts
### 可直接复用的通信原语

本文件的“会话对象 + 有地址的元数据索引”在 zenpi 中不应照搬成进程内无界共享 Map。最接近且已经存在的原语是 `src/session.rs` 的 `SessionMailbox`：`MailboxMessage` 带 sender/recipient/session/request、digest、TTL、sequence、claim token、status/result（L3170-L3194），`MailboxAction` 定义 `Acknowledge/Claim/Complete/Fail`（L3258-L3265），每次变更在 inode 锁下恢复并追加同步 journal（L3570-L3579、L3743-L3795）。它能同时充当消息、邮箱和可审计事件协议。发送/接收/认领/完成由 `src/protocol.rs` 的 `MailboxRequest`（L79-L113）和 `validate_mailbox`（L544-L583）承载；`src/headless.rs::execute_mailbox` 已把它接到 JSONL（L8954-L9093），但返回值明确 `execution_started: false`，不会自动调度 worker。

### 会话父子关系与 DAG 差异

`src/core.rs::Turn` 的 `parent_id`（L140-L175）只表达会话内 conversation turn 的父项，不能直接等同 DAG 节点父子；当前 `src/session.rs` 的 mailbox 也只按显式 session ID 寻址，没有 grandparent/sibling/child 关系解析。建议在 `src/session.rs` 的 durable event/journal 层增加可验证的 `DagNodeRecord`（`node_id`、`session_id`、`parent_node_id`、`depth`、直接 children、状态、`generation`），并提供纯查询函数 `parent(node)`、`grandparent(node)`、`direct_siblings(node)`、`direct_children(node)`、`descendants(node)`；关系变更先写事件再更新内存投影。验证要求是同一 workspace、父引用存在、无环、child 只能有一个直接 parent，且 session `Turn.parent_id` 仍只用于转录。

### 面向 zenpi DAG 的最小机制

1. **通信**：worker 负责节点 `N` 时，由关系查询把 parent、grandparent、直接 sibling、直接 child 映射成允许的 session ID，再复用 `SessionMailbox::enqueue/list/find_message/update`。所有消息必须带 `node_id`、`in_reply_to`、`generation` 和 `request_id`；只允许这些关系集合，不能让 worker 任意指定 workspace 路径。跨节点回执使用现有 `finish_claim_with_reply` 的“先完成接收方、再向原 sender 投递 reply”语义（L3490-L3559），delivery 失败必须返回可重试错误而不是伪造成功。
2. **保活**：`src/session.rs::LiveSessionRegistry` 已有 `register/heartbeat/unregister/active`（L3274-L3359），用 `owner_epoch + last_seen_ms + ttl_ms` 做最小 liveness；`src/core.rs` 暴露 `register_live_owner/heartbeat_live_owner/unregister_live_owner`（L819-L844）。DAG worker 的保活循环应只做 heartbeat，并在 lease 将过期前调用 `Agent

## AC-051 opencode/packages/opencode/src/acp/tool.ts
### 可复用原语与现有落点

1. ACP 的 `ToolCall` 快照可映射为 `protocol.rs` 的可序列化事件/响应，再由 `core.rs::AgentEvent`（`src/core.rs:L283-L325`）承载 `ToolCall`、`ToolProgress`、`ToolResult`。`tool_runtime.rs::ToolBatchOutcome` 已保证每个输入 call 一个按源顺序的终态（`src/tool_runtime.rs:L151-L168`），适合承接本文“pending→running→completed/failed”的协议投影；建议增加纯函数 `acp_tool_kind`, `acp_locations`, `acp_tool_update`，保持无执行副作用。
2. 通信首选现有持久 mailbox：`session.rs` 的 `MailboxMessage`/`MailboxStatus`（`src/session.rs:L3171-L3268`）已有 sender、recipient、request_id、digest、TTL、claim token、result；`LiveSessionRegistry::claim_next/claim_message/finish_claim/finish_claim_with_reply`（`src/session.rs:L3363-L3525`）已提供心跳、领取、完成、回复和 owner epoch 防旧进程完成。它正是 worker 与 parent/child/sibling 间的消息/邮箱原语，不应另造无界 channel。
3. 事件原语用 `SessionStore::append_event`/`events`（`src/session.rs:L1207-L1223`）记录 `dag_node_*`、`worker_*`、`mailbox_*`、`close_blocked`、`worker_derived`；实时投影用 `AgentEvent` 和 `runtime.rs` 的有界 `RuntimeEvent`。协议入口沿 `protocol.rs::MailboxRequest`（`L81-L113`、校验 `L544-L578`）和 `headless.rs::mailbox_slash_view`（`L2065-L2112`）扩展，加入受限的 DAG relation，而不是让客户端任意伪造 sender。
4. 会话父子关系不能只借用 `Turn.parent_id`（`src/core.rs:L143-L203`）：它表达 turn/steer 关系，不表达 session DAG。建议在 `session.rs` 增加不可变 `DagNodeRef { node_id, session_id, parent_node_id, grandparent_node_id, generation }`，以 `dag_node_attached` 事件持久化；direct sibling 由相同 `parent_node_id` 计算，direct child 由 `parent_node_id == self.node_id` 查询，grandparent 沿 parent 指针上溯一次。所有关系查询必须限定同一 workspace，并验证目标仍是已注册节点。

### 针对 zenpi DAG 需求的最小机制

- 新增建议类型（可放 `protocol.rs` 或专门的 `dag.rs`）：`DagRelation::{Parent,Grandparent,DirectSibling,DirectChild,SelfNode}`、`DagMessage { message_id, from_node, to_node, relation, kind, correlation_id, generation, lease_id, payload, expires_at_ms }`、`DagNodeState::{Queued,Running,Green,Blocked,Cancelled,Closed}`

## AC-052 opencode/packages/opencode/src/acp/usage.ts
### 可直接复用的通信原语

1. `src/protocol.rs` 已有版本化 JSONL 信封、`MAX_ID_BYTES`、`MAX_MAILBOX_TEXT_BYTES`、`MAX_MAILBOX_TTL_MS`，以及 `MailboxRequest::{Send, Receive, Acknowledge, Claim, Complete}` 和 `MailboxOutcome::{Succeeded, Failed, Abandoned}`（L20-L35、L79-L113）。它适合作为 DAG 控制面协议；不要让客户端直接提交 sender identity 或权限，当前协议也明确把身份留给 mailbox owner。
2. `src/session.rs` 的 `SessionMailbox` 是可复用的持久邮箱：`MailboxMessage` 带 sender/recipient、request ID、digest、sequence、TTL、status、claim token、result（L3171-L3253）；`enqueue/list/update` 使用锁、边界校验和追加 transition（L3574-L3790）。`LiveSessionRegistry` 提供 `register`、`heartbeat`、TTL `active`、`claim_next`、`claim_message`、`finish_claim`（L3278-L3529），这正是最小保活/派生前的 live-owner 租约基础。
3. `src/headless.rs` 已把协议 mailbox 接到 session owner：`mailbox_slash_view` 映射 slash action（L2065-L2114），`execute_mailbox` 只允许同 session directory 中唯一的 recipient，并处理 send/receive/claim/complete 与 reply（L8940-L9088）。父、祖父、直接 sibling、直接 child 都可以先表现为已验证的 `session_id` 地址；关系解析需另加 DAG 拓扑层，不能从文件名推断。
4. 实时事件可复用 `core::AgentEvent` 的 `TurnAccepted`、`Handoff`、`ToolCall`、`ToolProgress`、`ToolResult`、`Error`（`src/core.rs` L273-L325），以及 `headless.rs::AsyncEventBuffer` 的按事件数和字节双预算、优先保留 admission/tool lifecycle、显式 dropped 计数（L861-L1060）。持久 mailbox 负责可靠命令，AgentEvent 负责 live progress，二者不能混为一个无界队列。
5. `src/runtime.rs::BackgroundRunner` 提供 bounded command/event channels、FIFO pending、`Accepted/Started/Queued/Completed/CancelRequested/Closed` 事件和 `CancellationToken`（L1-L18、L143-L198、L250-L365）。它适合作为一个 DAG worker 的执行壳；`CancellationToken` 是协作式的，不能杀死任意线程，超时后 detach 也不能回滚副作用（L327-L341、L383-L420）。

### 会话父子关系与 DAG 状态建议

现有 `core::Turn.parent_id` 只表达一次 steering 或上下文父 turn：`Turn::new` 无父，`Turn::with_parent` 设置父，`validate` 只校验 ID 形状（`src/core.rs` L140-L205）；它没有祖父、sibling、child 集合，也没有 DAG 完成判定。建议新增 `src/agent_graph.rs` 或 `src/session_

## AC-053 opencode/packages/opencode/src/cli/cmd/run/subagent-data.ts
### 先划清可复用边界

源文件的可复用核心不是“subagent UI tab”，而是四个数据流习惯：稳定 session ID 关联、事件 reducer、有限队列/帧保留、可重放快照。它没有父/祖父/兄弟/子节点授权、DAG 完成判定、keepalive 或 worker spawn；`children` 只用于过滤父 task 产出的已知 child（L709-L737），不能直接满足 zenpi 的 DAG 编排。

1. **消息/邮箱原语**：优先复用 `src/protocol.rs` 的 `MailboxRequest::{Send,Receive,Acknowledge,Claim,Complete}`，其中 Send 带 `recipient_session_id`、`message_id`、正文和 TTL，Receive 用 sequence cursor 分页，Claim/Complete 提供所有权与结果（`src/protocol.rs:L81-L113`）。验证和 TTL/文本边界已在 `validate_mailbox` 中实现（`src/protocol.rs:L544-L590`）。它适合 parent、grandparent、直接 sibling、直接 child 的点对点控制/结果消息；但必须由 graph owner 证明 recipient 是允许的邻接点，不能把任意合法 `recipient_session_id` 当成拓扑授权。
2. **事件原语**：内部 worker 生命周期复用 `src/runtime.rs` 的 `RuntimeEvent::{Accepted,Started,Queued,CancelRequested,Completed,Rejected,Closed}` 与有界 channel；对外进度复用 `StdioEvent` 的 sequence、request_id、turn_id、event envelope（`src/runtime.rs:L171-L190`，`src/protocol.rs:L882-L918`）。建议将 `NodeEvent` 放进 `event` payload，保留 `node_id`、`parent_id`、`generation`、`kind`、`monotonic_seq`，让断线恢复只回放 durable journal cursor，不把 UI snapshot 当权威状态。
3. **阻塞/协议原语**：权限问题不要另造等待机制；`ApprovalCoordinator` 已是 worker-host rendezvous，worker 等待 bounded condition variable，host 可 drain/answer（`src/approval.rs:L126-L190`），并且持久化失败会 retract，避免未审计批准进入副作用（`src/approval.rs:L370-L390`）。`StdioRequest -> Command` 的 bounded validation、request ID 和 `Mailbox` 路由可作为 headless 控制面（`src/protocol.rs:L265-L267`、`L479-L484`）。

### 建议的 DAG 类型与会话父子关系

建议新增 `src/agent_graph.rs`（或将同等类型并入既有 `session_tree`，但不要把 DAG 语义埋进 UI reducer）：

```rust
type NodeId = String;
struct DagNode {
    node_id: NodeId,
    session_id: String,
    parent_id: Option<NodeId>,
    children: BTreeSet<NodeId>,
    status: NodeStatus,
    generation: u64,
    last_heartbeat_ms: u64,
}
enum NodeStatus { Running, Blocked, Red, Green, 

## AC-054 opencode/packages/opencode/src/sync/README.md
总体结论：README 的可复用核心不是某个具体 TS API，而是“结构化消息/事件 + 单写者全序 + projector/状态消费者 + 兼容投影”。zenpi 已有的持久邮箱、JSONL session journal、有界 runtime 和 host approval 足以组成最小 DAG 通信层，但现有 `Turn.parent_id` 不能单独承担 worker DAG 拓扑。

### 通信原语与 DAG 语义

- 消息/邮箱：优先复用 `src/session.rs:L3171-L3794` 的 `MailboxMessage`、`MailboxAction`、`SessionMailbox`、`LiveSessionRegistry`。它已有 `sender_session_id`、`recipient_session_id`、`request_id`、不可变 `digest`、TTL、`Queued/Acknowledged/Claimed/Succeeded/Failed/Expired` 状态、claim token、分页和同 workspace 校验。它对应源文的“事件/记录可回放”思想，但当前 mailbox 是请求状态机，不是广播事件流；应在 `payload` 中承载 `DagMessage { kind, node_id, parent_id, target, generation, reply_to, body }`，保留原有 digest/TTL/claim 约束。
- 事件：复用 `src/core.rs:L283-L325` 的 `AgentEvent` 作为进程/host 事件出口，复用 `SessionStore` 的 JSONL 记录作为 durable event log。建议新增明确的 `DagEvent`/`DagEventKind`（例如 `WorkAccepted`、`NodeGreen`、`ChildSpawned`、`CloseDeferred`、`LeaseRenewed`、`WorkArrived`），由唯一 session owner 写入，再由 headless/UI projector 转换为 `AgentEvent`；不要让多个 worker 直接共同写同一 journal。
- 协议：复用 `src/protocol.rs:L27-L115` 的版本化 JSONL、`MailboxRequest`、`MailboxOutcome` 与 `src/protocol.rs:L1121-L1233` 的 session-scoped request 校验。最小可执行扩展是 `dag_send`、`dag_receive`、`dag_status`、`dag_spawn`、`dag_close` 五类请求，字段必须带 `session_id`、`node_id`、`request_id`、`schema_version`，并把 sender 身份留给 host/session owner 解析，不能信任客户端自报 sender。
- 父子会话：新增持久的 `DagNodeMeta { node_id, session_id, parent_node_id: Option<NodeId>, children: BTreeSet<NodeId>, state, generation, owner_epoch }`，写入 node 的 session journal 或受同一 writer 保护的 DAG 索引。`Turn.parent_id`（`src/core.rs:L143-L207`）只表示对话/steer 的记录关联；它可作为展示关联，但不能替代 worker parent/grandparent/child 拓扑。`src/session_tree.rs` 的 `TreeEntry.parent_id` 是 transcript tree 索引，也应明确与 worker DAG 分离。
- 定向关系：worker 节点要找 parent、grandparent、直接 sibling、直接 child，先从 `DagNodeMeta` 得到直接 parent

## AC-055 opencode/packages/opencode/src/sync/schema.ts
### 可复用原语与现有落点

1. **ID/schema 原语**：在 `src/protocol.rs` 或新建 `src/dag.rs` 定义 `pub struct EventId(String)` 与 `TryFrom<String>`/`serde` 实现，校验至少为非空、`evt` 前缀、长度和控制字符；提供 `EventId::ascending(given: Option<&str>) -> Result<Self, DagError>`。它对应 `EventID`（L6-L11），但 Rust 不能只靠 TypeScript brand，要把不变量放进构造器。随机/时间生成应复用 zenpi 的 ID 约定，并用单调序列或 `JobId` 关联；不要把现有 `StdioEvent.sequence: u64`（`src/protocol.rs:L885-L900`）误当作跨会话 `EventId`。
2. **消息/邮箱**：优先复用 `src/protocol.rs:L81-L105` 的 `MailboxOutcome`/`MailboxRequest` 和 `Command::Mailbox`（`L211-L263`），底层复用 `src/session.rs:L3171-L3265` 的 `MailboxMessage`、TTL、claim token、digest 与状态机，以及 `SessionMailbox`（`src/session.rs:L3574-L3655`）。这提供 durable message、ACK、claim、complete/fail；发送者只提供 recipient/request/payload/ttl，权限仍由 owner 检查。
3. **事件/协议**：短生命周期进度使用 `RuntimeEvent`（`src/runtime.rs:L171-L190`）和 `StdioEvent`（`src/protocol.rs:L882-L918`），事件与 terminal response 分离，能关联 `request_id`/`turn_id` 并按 sequence 重放。跨进程入口继续走 `parse_line`/`Command`（`src/protocol.rs:L795-L851`），不要把 DAG 进度伪装成第二个命令结果。
4. **会话父子关系**：`src/core.rs:L140-L205` 的 `Turn.parent_id` 可保留对话父链，`InputBoundary.context_parent`（`src/runtime.rs:L769-L791`）可表达下一模型边界；但二者都不是完整 DAG 拓扑。新增持久化的 `DagSessionRelation { node_id, session_id, parent_session_id, edge_kind }`，并在 `src/session.rs` 的 session 记录中保存直接 parent/children 边。grandparent 通过 parent 的 parent 得到；direct sibling 通过相同 parent 查询；direct child 通过 children 查询。真正发送仍按目标 `session_id` 调 `SessionMailbox`，不能凭 `parent_id` 猜收件人。
5. **保活与派生最小机制**：复用 `src/session.rs:L3274-L3335` 的 `LiveSessionOwner`/`LiveSessionRegistry` 做 owner 注册和 heartbeat；其注释明确它只是 admission boundary，不会启动 scheduler。为 DAG 增加最小 `WorkerLease { lease_id, node_id, session_id, owner_epoch, heartbeat_deadline_ms, status }`，把 heartbeat 作为 mailbox/protocol 的 typed action 或 `RuntimeEvent::Heartbeat`。派生时由 

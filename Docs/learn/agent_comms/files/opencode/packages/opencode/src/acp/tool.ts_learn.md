# AC-051 — opencode/packages/opencode/src/acp/tool.ts

source_id: `opencode/packages/opencode/src/acp/tool.ts`
item_id: `AC-051`
source_path: `opencode/packages/opencode/src/acp/tool.ts`
source_hash: `decfa2d51b4edcf5a2259b754e1d6528b4d42aeff1309981c5ec19ff09d45e3e`
source_bytes: `10665`
source_lines: `364`
coverage: 字节 `1-10665`，行 `L1-L364`，已按源文件顺序完整读取（含注释、类型、导出与私有函数）。

## 完整行为复盘

文件是 OpenCode 工具状态到 ACP `ToolCall`/`ToolCallUpdate` 的纯同步适配层：只做分类、路径定位、文本/差异/图片投影，不执行工具。

- 类型 `ToolInput`（`L4`）是 `Record<string, unknown>`；`ToolAttachment`（`L6-L10`）允许可选 `mime`、`url` 及任意字段；`CompletedToolState`、`RunningToolState`、`ErrorToolState`（`L12-L31`）分别固定 `status` 为 `completed`/`running`/`error`，输入均为 `ToolInput`，完成态带 `output`，错误态带 `error`；`metadata`/`attachments` 是可选的。`ImageAttachment`（`L33-L36`）是已解码的 `mimeType`+base64 `data`。这些类型无运行时校验、无默认构造和错误返回。
- `toToolKind(toolName)`（`L38-L71`）：先 `toLocaleLowerCase()`；`bash`/`shell`→`execute`，`webfetch`→`fetch`，`edit`/`apply_patch`/`patch`/`write`→`edit`，`grep`/`glob`/`context`/两个 `context7_*`→`search`，`read`→`read`，`task`→`think`，其余→`other`。未知工具不会报错；输入必须是字符串，函数无共享状态、无并发等待。
- `toLocations(toolName, input, cwd?)`（`L73-L101`）：shell 调 `shellWorkdir`，有路径才返回单元素 `{path}`；`read`/`edit`/`write` 取 `filePath ?? filepath`；`external_directory` 合并 `filePath ?? filepath`、`parentDir`、`directories`；搜索类取 `path`；其余为空数组。它本身不去重，实际由 `locationFrom` 去重；普通文件路径不解析为绝对路径，只有 shell 工作目录会解析。缺值、非字符串、空字符串被忽略。
- `completedToolContent(toolName, state)`（`L103-L122`）：首项始终是文本 content；工具名为 `read` 时优先 `readDisplayText(state.metadata)`，取不到才用 `state.output`。若 `toToolKind` 为 `edit`，追加 `diffContent(state.input)`；最后追加 `imageContents(state.attachments ?? [])`。顺序严格为文本→diff→图片；缺 diff 或非图片附件静默跳过。
- `pendingToolCall(input)`（`L124-L138`）：输出 `ToolCall`，状态为 `pending`，复制 `toolCallId`，用 `toolTitle` 生成标题，用 `toToolKind` 生成 kind，用 `toLocations` 生成 locations，用 `rawInput` 生成 rawInput。没有校验 ID、输入或 cwd，也不执行调用。
- `runningToolUpdate(input)`（`L140-L168`）：输出状态 `in_progress` 的 `ToolCallUpdate`，包含 id、kind、标题、locations、rawInput；仅当 `input.output` 为真值时加入单个文本 content，因此空字符串会被省略。它是重建快照，不是增量合并；重复调用无内部去重。
- `duplicateRunningToolUpdate(input)`（`L170-L184`）：与 running 版本相同，但永不带 content，适合重复发布运行态元数据；输入的 `cwd` 仍参与位置/rawInput 推导。
- `completedToolUpdate(input)`（`L186-L199`）：输出状态 `completed`，可选 title 仅在真值时写入，content 来自 `completedToolContent`，rawOutput 来自 `completedToolRawOutput`。形参 `cwd` 未使用；完成态不再重新附带 kind、locations、rawInput。
- `errorToolUpdate(input)`（`L201-L228`）：输出状态 `failed`，kind/标题/locations/rawInput 按工具重建；content 是一项错误文本；rawOutput 总是含 `error` 和 `metadata` 键，即使 metadata 为 `undefined`。不抛出业务错误，不做重试或回滚。
- `completedToolRawOutput(state)`（`L230-L236`）：总含 `output`；`metadata !== undefined` 才加入 metadata；`attachments?.length` 为真才加入 attachments。空数组被省略，非空附件原样透传，不筛图片。
- `imageContents(attachments)`（`L238-L249`）：先 `extractImageAttachments`，每项转成 ACP image content，保留顺序；非图片/非 data URL 不产生输出。
- `extractImageAttachments(attachments)`（`L251-L256`）：逐附件调用 `dataUrlImage` 并 `flatMap`，只保留成功项。不会校验 base64 内容本身，也不会下载 URL。
- `shellOutputSnapshot(state)`（`L258-L261`）：metadata 必须是非空 object；读取其 `output`，仅字符串返回，否则 `undefined`。数组也属于 object，但没有字符串 output 即无结果。
- 私有 `toolTitle`（`L263-L268`）让 shell 标题优先显示 `command ?? cmd`，再 fallback，再工具名；非 shell 用 `fallback || toolName`，空标题会退回工具名。`rawInput`（`L270-L277`）只为 shell 补 cwd：已有真值 `input.cwd` 或 `input.workdir` 时原样返回，否则把解析出的 workdir 写入新对象；非 shell 原样返回同一个 input 引用。`cwd`/`workdir`（`L279-L288`）优先 workdir，再 cwd；相对路径以传入 cwd 或 `process.cwd()` 解析，绝对路径原样保留；无值时返回传入 cwd（也可能为 `undefined`）。`shellCommand`（`L290-L292`）优先 command，再 cmd；`isShell`（`L294-L297`）只认 bash/shell，大小写不敏感。
- 导出别名（`L299-L308`）是同一函数引用：`mapToolKind`、`extractLocations`、`buildCompletedToolContent`、`buildCompletedRawOutput`、`extractShellOutputSnapshot`、`buildPendingToolCall`、`buildRunningToolUpdate`、`buildDuplicateRunningToolUpdate`、`buildCompletedToolUpdate`、`buildErrorToolUpdate`，没有额外语义。
- 私有 `locationFrom`（`L310-L323`）接受多个未知值：数组只保留非空字符串，标量经 `stringValue` 过滤；`Set` 去重后按首次出现顺序转 `{path}`。`diffContent`（`L325-L338`）要求 oldString 和 newString/content 均为字符串，否则空数组；新文本优先 newString，路径只取 `filePath`，缺失时为 `""`。`readDisplayText`（`L340-L350`）要求 metadata.display 为对象：file 返回字符串 text；directory 要求 entries 数组，过滤字符串后用换行连接；其他 display 类型返回 `undefined`。`dataUrlImage`（`L352-L360`）只接受 `data:<mime>;...;base64,<data>`，URL 中 mime 优先于附件 mime，且 mime 必须以大小写敏感的 `image/` 开头；没有匹配到 base64 数据即使附件 mime 是图片也返回空。`stringValue`（`L362-L364`）只接受原生字符串。

所有函数都是同步、无锁、无全局可变状态；数组/对象主要是新建快照，但 `rawInput` 在非 shell 或已有工作目录时会原样返回 input。并发语义因此是“调用方可并发调用但本模块不提供排序、去重、取消或原子提交”。

## 状态、取消、恢复与副作用

本源文件没有取消 token、超时、重试、持久化、网络/进程/文件写入或工具执行；所有“错误路径”都是空数组、`undefined`、默认 ACP kind，或把错误文本投影为 `failed`。它不维护状态机，`pending`/`in_progress`/`completed`/`failed` 只是输出协议状态。恢复依赖上游重新传入 `state`，本模块不保存 checkpoint；附件只做内存引用/透传，图片只解析 data URL，不产生外部副作用。注意 `completedToolUpdate` 的 `cwd` 没有作用，取消/超时只能由调用它的事件层决定。

## 源内测试与行为判据

配套测试为 `packages/opencode/test/acp/tool.test.ts`（已完整读取）。`L15-L31` 验证工具分类；`L33-L51` 验证文件、搜索、external directory、shell 工作目录的绝对/相对路径和缺省空结果；`L53-L96` 验证文本→diff→图片顺序及只接受图片 data URL；`L98-L114` 验证缺 old/new 文本时不生成 diff；`L116-L193` 验证 pending/completed update 字段；`L195-L233` 验证 read 的干净 display 文本优先、raw output 仍保留原 output/metadata；`L235-L266` 验证可选 metadata/attachments 的省略规则；`L268-L291` 验证 MIME 与 base64 过滤；`L293-L297` 验证 shell output snapshot 只接受字符串。可独立验证命令是：在 OpenCode 包目录运行 `bun test test/acp/tool.test.ts`。测试未覆盖的边界包括空 shell command、重复 location、多级 data URL 参数、非字符串 `ToolInput` 字段、locale 特殊大小写和 `completedToolUpdate` 的未使用 `cwd`；这些应按上述源码判据补测。

## zenpi Rust 映射

### 可复用原语与现有落点

1. ACP 的 `ToolCall` 快照可映射为 `protocol.rs` 的可序列化事件/响应，再由 `core.rs::AgentEvent`（`src/core.rs:L283-L325`）承载 `ToolCall`、`ToolProgress`、`ToolResult`。`tool_runtime.rs::ToolBatchOutcome` 已保证每个输入 call 一个按源顺序的终态（`src/tool_runtime.rs:L151-L168`），适合承接本文“pending→running→completed/failed”的协议投影；建议增加纯函数 `acp_tool_kind`, `acp_locations`, `acp_tool_update`，保持无执行副作用。
2. 通信首选现有持久 mailbox：`session.rs` 的 `MailboxMessage`/`MailboxStatus`（`src/session.rs:L3171-L3268`）已有 sender、recipient、request_id、digest、TTL、claim token、result；`LiveSessionRegistry::claim_next/claim_message/finish_claim/finish_claim_with_reply`（`src/session.rs:L3363-L3525`）已提供心跳、领取、完成、回复和 owner epoch 防旧进程完成。它正是 worker 与 parent/child/sibling 间的消息/邮箱原语，不应另造无界 channel。
3. 事件原语用 `SessionStore::append_event`/`events`（`src/session.rs:L1207-L1223`）记录 `dag_node_*`、`worker_*`、`mailbox_*`、`close_blocked`、`worker_derived`；实时投影用 `AgentEvent` 和 `runtime.rs` 的有界 `RuntimeEvent`。协议入口沿 `protocol.rs::MailboxRequest`（`L81-L113`、校验 `L544-L578`）和 `headless.rs::mailbox_slash_view`（`L2065-L2112`）扩展，加入受限的 DAG relation，而不是让客户端任意伪造 sender。
4. 会话父子关系不能只借用 `Turn.parent_id`（`src/core.rs:L143-L203`）：它表达 turn/steer 关系，不表达 session DAG。建议在 `session.rs` 增加不可变 `DagNodeRef { node_id, session_id, parent_node_id, grandparent_node_id, generation }`，以 `dag_node_attached` 事件持久化；direct sibling 由相同 `parent_node_id` 计算，direct child 由 `parent_node_id == self.node_id` 查询，grandparent 沿 parent 指针上溯一次。所有关系查询必须限定同一 workspace，并验证目标仍是已注册节点。

### 针对 zenpi DAG 需求的最小机制

- 新增建议类型（可放 `protocol.rs` 或专门的 `dag.rs`）：`DagRelation::{Parent,Grandparent,DirectSibling,DirectChild,SelfNode}`、`DagMessage { message_id, from_node, to_node, relation, kind, correlation_id, generation, lease_id, payload, expires_at_ms }`、`DagNodeState::{Queued,Running,Green,Blocked,Cancelled,Closed}`、`WorkerLease { node_id, generation, expires_at_ms }`。`relation` 只能由服务端根据拓扑验证，不能信任客户端字段。
- `src/core.rs` 增加 `Agent::send_dag_message`, `claim_dag_message`, `complete_dag_message`, `dag_snapshot`；底层调用现有 `finish_live_mailbox_with_reply`。发送目标只允许 parent、grandparent、直接 sibling、直接 child 或 self；跨工作区、过期 lease、旧 owner epoch、未知 node 都 fail closed。消息 payload 必须有界，并使用现有 digest/TTL/claim 语义。
- 保活最小闭环放在 `src/runtime.rs` 与 `src/core.rs`：复用 `BackgroundRunner` 的 bounded command/event channel、`JobId`、`CancellationToken`（`src/runtime.rs:L35-L150`、`L190-L280`），增设 `Heartbeat { node_id, generation, now_ms }` 和 `Derive { parent_node_id, work_digest }` 命令；`LiveSessionRegistry::heartbeat` 只证明 owner 活着，不等于 DAG 节点完成，因此必须另记 `dag_node_heartbeat` 与 lease expiry。
- close 判定必须递归而非仅检查当前节点：`can_close(node)` 当且仅当自身为 `Green` 且所有 direct child 及其递归 descendants 均为 `Green`；存在 `Running/Queued/Blocked/Unknown/Expired` 时返回 `close_blocked`，保持 parent worker/owner 活着。`src/headless.rs` 已有“有 admitted work 时拒绝 close”的边界（约 `L5585-L5592`），应把 DAG 聚合判定接在该 close admission 前；不要仅调用现有 `Agent::close`。
- 新工作派生走显式 `derive_worker(parent_node_id, work_item)`：先在 `SessionStore` 写 `worker_derivation_intent`，再创建带 `parent_node_id`/`generation + 1` 的 child session、绑定 `WorkerExecutionBinding`（`src/core.rs:L1543-L1701`），最后写 `worker_admitted` 并提交 runtime job。任一持久化步骤失败都不得报告 child 已运行；旧 worker 保持 alive，等待重试/人工恢复。`core.rs` 的 `append_runtime_intent` 本身明确“不启动 scheduler 或 nested worker”（`src/core.rs:L5463-L5480`），所以只能把它作为意图日志，不能冒充派生完成。

### 文件级落点与差异清单

- `src/headless.rs`：在 mailbox 协议映射、运行时事件处理（约 `L4952-L5200`）、close 命令和 `execute_mailbox` 附近接入 DAG 命令；输出 `dag_message_queued/claimed/completed`、`close_blocked`、`worker_derived`。现有 mailbox 入口有了，但缺 relation authorization、递归绿检查和 derive admission。
- `src/core.rs`：扩展 `AgentEvent`、`AgentSnapshot` 和 `Agent` 的 DAG API；复用 `Turn.parent_id` 仅做 turn 级链路，新增 session/node 拓扑索引。现有 `WorkerExecutionBinding`/`admit_blueprint_worker` 可复用 lease、policy digest、过期和 durable reservation，但它不会建立父子节点或自动派生。
- `src/session.rs`：复用 `SessionMailbox` 的同 inode lock、digest、TTL、claim/complete、reply；新增节点注册、父指针、generation、状态聚合事件和 `dag_descendants_green` 查询。现有 mailbox 只按 recipient session 寻址，不能表达 sibling/ancestor 关系，必须在 owner 侧补拓扑校验。
- `src/tool_runtime.rs`：工具执行仍是叶节点工作，复用 `execute_tool_batch` 的有界并发、全批取消、按源顺序结果；不要把 DAG 通信塞进工具 handler。它当前“不重试 interrupted call”，派生新 worker 应是上层新 job/新 call_id，而不是隐式重试同一 call。
- `src/runtime.rs`：复用 `BackgroundRunner` 的 bounded queue、`RuntimeEvent`、`CancellationToken` 和 shutdown grace；增加 DAG heartbeat/derive/close-check 事件。现有 cancellation 是合作式且取消不是 rollback，需把 `Unknown`/未确认副作用保留为非绿，不能因 cancel 直接 close。
- `src/protocol.rs`：复用 `MailboxRequest::{Send,Receive,Acknowledge,Claim,Complete}`、`MailboxOutcome` 和字段校验；新增 `Dag` 请求并限制 relation、node ID、generation、TTL、payload 大小。当前协议只提供 session mailbox，不提供拓扑或递归 close 语义。
- `src/approval.rs`：DAG 派生/通信若带工具副作用，继续走 `ApprovalRequest` 的 `request_id/turn_id/call_id/lease_id` 和 `ApprovalCoordinator`；worker 的 approval 必须带 policy 与 lease，不能把 mailbox 消息当授权。现有取消 epoch、fail-closed 持久化可复用。
- `src/providers/**`：provider 只负责模型路由、能力（尤其 tools/streaming）和请求事件；不要让 provider 维护 DAG 拓扑或决定 close。派生 worker 时由 `core.rs` 选择经过 `providers/registry.rs` 能力校验的 backend/model，并将 provider stream 映射为 runtime/AgentEvent；通信和 liveness 留在 session/runtime。

可执行验证顺序：先为 `DagNodeRef` 建立同 workspace 的 parent/grandparent/sibling/child 关系测试；再测试 mailbox claim 的旧 epoch、TTL、重复 complete 和回复寻址；随后构造 child 全绿、child blocked、grandchild running 三种树验证 close；最后验证 close blocked 后 `derive_worker` 写入顺序、lease 过期、cancel 后 Unknown 不被误判为 Green。现有 `cargo test` 应覆盖新增纯判定，headless 集成测试覆盖协议边界。

## 未决问题

1. `tool.ts` 本身无法确认 ACP 消费方是否要求 locations 必须绝对路径，也无法确认 `ToolKind` 的未来枚举是否会新增值；当前实现对未知工具固定为 `other`。
2. 源文件没有定义 DAG、worker 派生或 close 规则；上述 parent/grandparent/sibling/child 拓扑、递归 Green 聚合和保活派生均是面向 zenpi 需求的映射设计，不是 OpenCode 源行为。

# AC-052 — opencode/packages/opencode/src/acp/usage.ts

```yaml
source_id: opencode/packages/opencode/src/acp/usage.ts
item_id: AC-052
source_path: opencode/packages/opencode/src/acp/usage.ts
source_hash: 09b2c0ec9a31a78cf85d5810d8bc7aa8d9b0fa460b16f4d0008d7d5b4d3fc38b
source_bytes: 8730
source_lines: 243
coverage:
  bytes: 0-8729（完整读取，共 8730 字节）
  lines: L1-L243（完整读取，含全部导入、类型、导出符号和实现）
learn_mode: understand
```

## 完整行为复盘

该文件是 ACP 使用量适配层，不负责生成模型回答，也不负责调度 worker。它把 OpenCode assistant message 的 token/cost 形状转换成 ACP `Usage`，从 session 读取最新 assistant message，再按 provider/model 查询 context limit，最后向 `AgentSideConnection.sessionUpdate` 发送 `usage_update`。文件没有注释；以下按源文件顺序覆盖全部声明。

### 类型、接口与服务声明

- `AssistantTokenCost`（L13-L13）是 `OpenCodeAssistantMessage` 的 `cost` 与 `tokens` 两字段投影；`AssistantMessage`（L15-L17）在此基础上要求 `role`，并允许 `providerID`、`modelID` 缺省。输入数据没有本文件自己的数值校验。
- `SessionMessage`（L19-L21）只保留 `info`，其最小形状是有 `role` 的消息，或完整 `AssistantMessage`；`MessagesInput`（L23-L26）把 `sessionID` 与 `directory` 作为读取键。
- `SDK`（L28-L35）约束 `session.messages(parameters, { throwOnError: true })` 返回一个 Promise，响应的 `data` 可以不存在或为 `null`。`MessageLoaderInterface`（L37-L39）将其暴露为 `Effect.Effect<readonly SessionMessage[], unknown>`；`ContextLimitLoaderInterface`（L41-L43）按 directory 返回 provider 信息表。
- `UsageConnection`（L45-L45）只取 `AgentSideConnection.sessionUpdate`，因此服务依赖的是单向通知能力，不持有连接生命周期。`Interface`（L47-L61）公开 `buildUsage`、`latestAssistantMessage`、`totalSessionCost`、Effect 化的 `contextLimit` 和 `sendUpdate`。
- `MessageLoader`、`ContextLimitLoader`、`Service`（L63-L71）是 Effect Context 服务，服务键分别为 `@opencode/ACPUsageMessageLoader`、`@opencode/ACPUsageContextLimitLoader`、`@opencode/ACPUsage`。它们是依赖注入边界，不是进程间通信通道。

### 逐函数与逐层行为

- `messageLoaderFromSDK(sdk)`（L73-L82）构造 `MessageLoaderInterface`。`messages(input)` 用 `Effect.promise` 调 SDK，并把 `response.data ?? []` 归一化为空数组；SDK Promise rejection 会作为 Effect 失败向上传播，没有重试、超时或日志。`messageLoaderLayer(sdk)`（L84-L84）用 `Layer.succeed` 固化这个 loader。
- `contextTokens(message)`（L86-L88）返回 `tokens.input + tokens.cache.read + tokens.cache.write`。它刻意不计 `output` 与 `reasoning`，因为该数值代表当前 context 占用；负数或异常数值不会被修正。
- `buildUsage(message)`（L90-L103）输出 `inputTokens`、`outputTokens`，并将 input、output、reasoning、cache read、cache write 五者相加为 `totalTokens`。`thoughtTokens`、`cachedReadTokens`、`cachedWriteTokens` 只有大于 0 才出现在结果中；零会被省略，负数也会被省略但仍影响 total。`cost` 不进入 ACP `Usage`。
- `latestAssistantMessage(messages)`（L105-L109）按 `info.role === "assistant"` 过滤，并取最后一个匹配项；没有 assistant 时返回 `undefined`。它依赖数组顺序，不按时间戳排序，也不复制消息。
- `totalSessionCost(messages)`（L111-L115）只对 assistant 消息的 `info.cost` 求和；用户消息被忽略，空数组结果为 `0`。它不做货币换算、上限裁剪或去重。
- `findContextLimit(providers, providerID, modelID)`（L117-L123）通过 `providers[providerID]?.models[modelID]?.limit.context` 查找；provider、model、limit 任一缺失时返回 `undefined`，不会抛错。
- `contextLimitLoaderLayer`（L125-L140）依赖 `InstanceStore.Service` 与 `Provider.Service`（L127-L130）。其 `providers(directory)` 先 `store.load({ directory })` 得到实例上下文，再以 `Effect.provideService(InstanceRef, ctx)` 执行 `provider.list()`（L132-L137）。实例加载或 provider 列表失败原样成为 Effect 失败，由上层缓存逻辑转为未知 limit。
- 私有 `layer`（L142-L231）组装完整 `Service`。它取得两个 loader，并创建 `SynchronizedRef` 保存 `Map<string, Effect.Effect<number | undefined>>`（L144-L148）。私有 `cachedLimit`（L149-L173）使用 `SynchronizedRef.modifyEffect` 串行修改缓存：键是 `directory`、`providerID`、`modelID` 用 NUL 字符拼接（L157）；命中则复用既有 Effect（L158-L159）；未命中则用 `Effect.cached` 包装 provider 查询、`findContextLimit` 映射，并在异常处记录 `failed to get providers for usage context limit` 后转为 `undefined`（L160-L169），再保存新键（L170）。因此同一服务实例内相同三元组共享查询和失败结果，失败不会自动重试；缓存没有失效/刷新入口。
- `contextLimit(input)`（L175-L181）通过 `yield* yield* cachedLimit(input)` 执行缓存中保存的 Effect，返回 `number | undefined`。`Effect.fn` 只提供追踪命名 `ACPUsage.contextLimit`，没有额外业务状态。
- `sendUpdate(input)`（L183-L221）先调用 `messageLoader.messages`，失败时记录 `failed to fetch messages for usage update` 并转成 `undefined`（L188-L195）；没有消息、没有 assistant、最新 assistant 缺 provider/model 时均静默返回（L195-L200）。随后按该消息的 provider/model 查 context limit；未知或为 0 时返回（L201-L207）。成功时通过 `Effect.promise` 调 `connection.sessionUpdate`，负载为 `sessionId`、`sessionUpdate: "usage_update"`、`used: contextTokens(message)`、`size`、以及所有 assistant 的 USD 总成本（L208-L217）。`sessionUpdate` rejection 被 `.catch(() => {})` 吞掉（L218-L219），所以通知失败不会使 Effect 失败。
- `Service.of`（L223-L229）只把 `buildUsage`、`latestAssistantMessage`、`totalSessionCost`、`contextLimit`、`sendUpdate` 暴露出去。`messageLoaderNode`（L233-L233）是全局但未绑定的 `LayerNode`；`contextLimitLoaderNode`（L235-L239）是全局节点，依赖 `Provider.node` 和 `InstanceStore.node`；`node`（L241-L241）再依赖两个 loader。最后 `export * as UsageService from "./usage"`（L243-L243）提供命名空间导出。

并发语义的核心是两个层次：缓存写入由 `SynchronizedRef` 串行化，具体 provider 查询由共享的 `Effect.cached` 复用；`sendUpdate` 本身没有去重锁，多个调用可并发读取和发送通知，通知顺序不由本文件保证。`messageLoaderFromSDK` 的 SDK Promise 也没有取消适配。

## 状态、取消、恢复与副作用

- 本文件只有服务实例内存状态：context-limit `Map` 和其 Effect 缓存。它不修改 session，不写磁盘，不保存 usage snapshot，也没有跨进程状态。
- 源实现没有显式取消、超时、重试或恢复协议。`Effect.promise` 包住的 SDK 查询和 `sessionUpdate` Promise 没有本地 timeout；查询错误只记录日志并把 limit 变成永久缓存的 `undefined`，消息读取错误只跳过本次 update；通知错误直接忽略。
- 外部副作用只有两类：通过 SDK 读取 session/provider 信息，以及调用 ACP `sessionUpdate`。后者只发送 usage 事件，不执行工具、不改变 provider 配置、不产生 worker。
- 对 zenpi DAG 的含义：不能把本文件的“查询失败后返回 undefined”照搬成节点完成；DAG 状态必须区分 `unknown`、`failed`、`green` 和 `closed`。尤其子孙未全绿时，父节点必须保活；取消只能阻止后续工作，不可证明已回滚外部副作用。

## 源内测试与行为判据

`usage.ts` 源文件本身未包含测试。仓库的直接对应测试位于 `opencode/packages/opencode/test/acp/usage.test.ts`：

- L129-L148 验证 `buildUsage` 会把 reasoning/cache 计入 `totalTokens` 并保留正数可选字段；L151-L167 验证零值字段省略。
- L169-L177 验证只取最新 assistant，以及只累计 assistant cost。
- L179-L208 验证 context limit 按 `directory/provider/model` 缓存，同一键只调用一次 provider loader。
- L210-L249 验证 `sendUpdate` 的 `used` 包括 cache read/write，`size` 来自模型 context，`cost.currency` 固定为 `USD`。
- L251-L317 验证消息读取失败、没有 assistant、assistant 缺 provider/model、未知 context size 时都不发送 update。

可独立验证的最小判据：构造一个含 user、两个 assistant 的列表，最后 assistant 的 `input=10、cache.read=5、cache.write=7` 应得到 `used=22`，总 cost 是两个 assistant cost 之和；同一 directory/provider/model 连续两次 `contextLimit` 只能触发一次 provider 查询；任何缺少必要关联数据的 `sendUpdate` 都应产生零条通知。zenpi 迁移后还应验证：祖先或任一 child/grandchild 非绿时禁止 close，所有后代绿且自身绿时只产生一次 close 事件，重复 claim/complete 必须幂等或 fail-closed。

## zenpi Rust 映射

### 可直接复用的通信原语

1. `src/protocol.rs` 已有版本化 JSONL 信封、`MAX_ID_BYTES`、`MAX_MAILBOX_TEXT_BYTES`、`MAX_MAILBOX_TTL_MS`，以及 `MailboxRequest::{Send, Receive, Acknowledge, Claim, Complete}` 和 `MailboxOutcome::{Succeeded, Failed, Abandoned}`（L20-L35、L79-L113）。它适合作为 DAG 控制面协议；不要让客户端直接提交 sender identity 或权限，当前协议也明确把身份留给 mailbox owner。
2. `src/session.rs` 的 `SessionMailbox` 是可复用的持久邮箱：`MailboxMessage` 带 sender/recipient、request ID、digest、sequence、TTL、status、claim token、result（L3171-L3253）；`enqueue/list/update` 使用锁、边界校验和追加 transition（L3574-L3790）。`LiveSessionRegistry` 提供 `register`、`heartbeat`、TTL `active`、`claim_next`、`claim_message`、`finish_claim`（L3278-L3529），这正是最小保活/派生前的 live-owner 租约基础。
3. `src/headless.rs` 已把协议 mailbox 接到 session owner：`mailbox_slash_view` 映射 slash action（L2065-L2114），`execute_mailbox` 只允许同 session directory 中唯一的 recipient，并处理 send/receive/claim/complete 与 reply（L8940-L9088）。父、祖父、直接 sibling、直接 child 都可以先表现为已验证的 `session_id` 地址；关系解析需另加 DAG 拓扑层，不能从文件名推断。
4. 实时事件可复用 `core::AgentEvent` 的 `TurnAccepted`、`Handoff`、`ToolCall`、`ToolProgress`、`ToolResult`、`Error`（`src/core.rs` L273-L325），以及 `headless.rs::AsyncEventBuffer` 的按事件数和字节双预算、优先保留 admission/tool lifecycle、显式 dropped 计数（L861-L1060）。持久 mailbox 负责可靠命令，AgentEvent 负责 live progress，二者不能混为一个无界队列。
5. `src/runtime.rs::BackgroundRunner` 提供 bounded command/event channels、FIFO pending、`Accepted/Started/Queued/Completed/CancelRequested/Closed` 事件和 `CancellationToken`（L1-L18、L143-L198、L250-L365）。它适合作为一个 DAG worker 的执行壳；`CancellationToken` 是协作式的，不能杀死任意线程，超时后 detach 也不能回滚副作用（L327-L341、L383-L420）。

### 会话父子关系与 DAG 状态建议

现有 `core::Turn.parent_id` 只表达一次 steering 或上下文父 turn：`Turn::new` 无父，`Turn::with_parent` 设置父，`validate` 只校验 ID 形状（`src/core.rs` L140-L205）；它没有祖父、sibling、child 集合，也没有 DAG 完成判定。建议新增 `src/agent_graph.rs` 或 `src/session_tree.rs` 的持久类型：

```text
WorkerNode { worker_id, session_id, parent_id: Option<WorkerId>, state, lease, provider, model }
WorkerEdge { parent_id, child_id, relation: Child }
WorkerState { Pending, Running, Green, Failed, Cancelled, Closed }
```

建议每个节点只保存一个直接 `parent_id` 和有界的直接 child 引用；祖父是沿 parent 链上溯，直接 sibling 是同一 parent 的 child 集合，直接 child 是 edge 表的一跳。发送前由 `Topology::resolve(worker, Relation::{Parent, Grandparent, Sibling, Child})` 解析为 session mailbox recipient，并拒绝越界、歧义、跨 workspace 或环。这样复用 `SessionMailbox` 的地址/权限校验，同时显式满足 worker 与 parent/grandparent/直接 sibling/直接 child 通信。

### 保活、派生与 close 的最小机制

建议在 `src/session.rs` 扩展持久事件（或新建 graph journal），至少有 `worker_spawned`、`worker_heartbeat`、`worker_status`、`worker_close_requested`、`worker_closed`、`worker_derived`。最小状态机应是：

1. `register_live_owner` + `heartbeat_live_owner` 维护 `owner_epoch`、`last_seen_ms` 和租约 TTL；租约失效只意味着不能 claim 新工作，不自动把执行结果判绿。
2. 派生新 worker 前，在父 session 追加带 `parent_id`、`item_id`、`lease_id`、policy/provider/model 快照的 `worker_spawned`，再经 `BackgroundRunner::try_submit` 或 `spawn` 启动。`worker_derived` 与 mailbox request 必须用稳定 `request_id`/digest 去重；不要隐式 retry，未知副作用沿用现有 journal/recovery 原则。
3. worker 完成一次工作只报告自身状态。`close_allowed(node)` 必须同时满足 `node.state == Green` 且所有直接 child 递归结果为 Green；任一 child/grandchild 是 Pending/Running/Failed/Unknown/Expired，节点不得 close，应保持 live owner 和可接收 mailbox。
4. 保活期间收到新工作时，追加 `worker_derived` 并启动新 child；没有新工作时只 heartbeat，不派生空 worker。全部自身及 descendants 绿后追加一次 `worker_closed`，关闭 live owner；重复 close 应返回既有 receipt，不重复外部通知。

### 对既有模块的可执行落点与差异清单

- `src/headless.rs`：保留当前 `mailbox_slash_view`/`execute_mailbox` 作为 JSONL/slash 入口；增加 `DAG` 或 `Worker` command，将 relation 解析、heartbeat、derive、status、close 映射到 session owner。复用 `AsyncEventBuffer` 分离 live progress 与 admission/terminal 事件。验证点：慢消费者下 mailbox 不无界增长，close receipt 可由 session cursor 重放。
- `src/core.rs`：在 `Agent` 增加 `worker_identity`、`topology` 或 `WorkerExecutionBinding` 扩展。现有 `WorkerExecutionBinding` 已绑定 blueprint/goal/item/lease/policy 并检查 expiry（L35-L82），适合成为派生 worker 的不可变执行绑定；`AgentPhase::{Idle,Running,Closed}`（L273-L279）需要与 DAG `Green/Blocked/Closed` 分层，不能把单 Agent 的 `Closed` 当作全子孙 green。
- `src/session.rs`：以 append-only `SessionStore::append_event`/`append_owned_event`（L1207-L1224）记录 worker graph、heartbeat、status 和 close receipt；把 `SessionMailbox` 作为节点之间的可靠消息面。新增 graph recovery 时须沿 sequence 重放、拒绝非法 parent/child 环、对重复 `request_id` 保持幂等。不要修改现有 mailbox 的 digest/TTL/claim transition 语义。
- `src/tool_runtime.rs`：复用 `MAX_BATCH_CALLS=32`、`MAX_BATCH_BYTES=256*1024` 和 `mpsc`/scoped worker 的 bounded 执行方式（L21-L22、L157-L167、L272-L344）。DAG 派生不应绕过 tool approval、operation journal 或未知结果恢复；该模块明确“不隐式重试中断调用”，因此新 worker 的重试必须是显式新 attempt。
- `src/runtime.rs`：`BackgroundRunner` 是 worker 线程调度落点；给 job input 加 `WorkerSpec`，给 output 加 `WorkerResult`，并把 `RuntimeEvent::Completed` 转换成自身绿/失败，而不是直接 close。新增 `derive` 只在父节点 live 且新工作已持久化后提交；若 command queue 满，返回 `QueueFull`，不得丢失派生意图。
- `src/protocol.rs`：在既有 `MailboxRequest` 旁增加严格的 `WorkerRequest`/`WorkerRelation`/`WorkerState` 或复用统一 envelope；使用 `deny_unknown_fields`、版本和 bounded ID。补充 `Heartbeat`、`Derive`、`Status`、`Close`、`SendRelation` 的结构化动作，禁止客户端伪造 sender、owner epoch、green receipt。
- `src/approval.rs`：该模块是工具副作用审批，不是 worker 路由。`ApprovalCoordinator` 的 condition-variable 等待、取消、重复响应 fail-closed、`persist_accepted` 先持久化再放行（L194-L245、L295-L390）应包住派生 worker 的 side-effect/tool actions；不能因 worker 是 child 就自动获得持久 grant，尤其 `BlueprintWorker` 的 remember 已被禁止（L320-L327）。
- `src/providers/**`：`src/providers/registry.rs::ModelDescriptor` 提供 `context_window`、`max_output_tokens`、capabilities 和 `context_budget` 裁剪（L74-L127），`ModelRegistry::resolve/list` 按精确 `(provider,id)` 解析（L311-L401）；`src/providers/connection.rs` 负责验证 model/provider identity、route、streaming/capabilities（L166-L230、L288-L365）。可把 opencode 的 `findContextLimit` 映射为读取 `ModelDescriptor.context_window` 的纯函数，但 provider/model 快照应在 `WorkerSpec` 中持久化，防止运行中配置漂移。其余 `anthropic.rs`、`codex.rs`、`deepseek.rs`、`google.rs`、`openai.rs` 的 provider definition 只决定 wire route/capability，不应承载 DAG 父子关系。

差异清单：opencode 有 Effect service/layer 与内存 context-limit cache，zenpi 是 Rust synchronous core + bounded background runner；opencode 的关系仅由 sessionID 调用方提供，zenpi 需要持久 DAG topology；opencode 的 usage update 失败可静默跳过，zenpi 的 worker 状态和 close 必须可审计、可恢复、不可把未知当 green；opencode 没有 keepalive/derive/descendant-close 规则，zenpi 需要新增状态事件、租约和递归 green 判定。

建议验证命令（实现后执行，不代表本次已修改代码）：`cargo test --lib session::`、`cargo test --lib runtime::`、`cargo test --lib protocol::`，并新增针对 `parent/grandparent/sibling/child` 路由、TTL 过期、重复 complete、child 未绿禁止 close、全 descendants green 允许 close、队列满不丢派生意图的单元测试。

## 未决问题

1. `@agentclientprotocol/sdk` 的 `Usage` 是否拒绝负数、是否要求 `totalTokens` 与服务端重新计算一致，源文件无法确认。
2. `Effect.promise` 被运行时中断时，底层 SDK Promise 是否真正取消，源文件没有提供 adapter 或 timeout 证据。
3. provider 列表变化后 context-limit cache 是否应失效、何时重建 `Service`，源文件只展示了单实例永久缓存。
4. zenpi 当前 `src/session.rs` 的 session 目录是否已经有统一的 DAG topology 持久格式，现有 `parent_id` 只能确认 turn 级父关系，不能从本文件推断 worker 图格式。

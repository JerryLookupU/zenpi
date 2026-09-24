# AC-041 — opencode/packages/opencode/src/acp/agent.ts

source_id/item_id: `opencode/packages/opencode/src/acp/agent.ts` / `AC-041`  
source_path: `opencode/packages/opencode/src/acp/agent.ts`  
source_hash: `3bf7c0a31bbf8c3dd5b3d06ddf38eff0c988858e5df6bb46d3abecc906f15a57`  
source_bytes: `2627`  
source_lines: `95`  
coverage: 已按文件顺序读取完整字节范围 `[0,2627)` 与行范围 `L1-L95`，包括导入、类型、注释、类方法、错误转换和重导出。

## 完整行为复盘

- `L1-L22` 导入 ACP SDK 的 `RequestError`、`ACPAgent`、`AgentSideConnection` 以及全部请求类型（`InitializeRequest`、`AuthenticateRequest`、`NewSessionRequest`、`LoadSessionRequest`、`ListSessionsRequest`、`ResumeSessionRequest`、`CloseSessionRequest`、`ForkSessionRequest`、`SetSessionConfigOptionRequest`、`SetSessionModeRequest`、`SetSessionModelRequest`、`PromptRequest`、`CancelNotification`）。同时引入 `Effect`、`OpencodeClient`、`ACPError` 和 `ACPService`。这些导入表明本文件是协议适配层：请求类型由 ACP 约束，业务实现全在 `ACPService.Interface`。
- `L24-L30` 导出 `init({ sdk: _sdk }: { sdk: OpencodeClient })`。输入必须是含 `sdk` 的对象；函数返回只含 `create` 的工厂对象。`create(connection: AgentSideConnection)` 将闭包保存的 `_sdk` 与新连接交给 `ACPService.make({ sdk: _sdk, connection })`，再包装成 `new Agent(...)`。没有默认 SDK、连接替换、缓存或校验；每次调用 `create` 都建立一个新的 `Agent`/service 组合，连接是该实例的外部回调通道。
- `L32-L33` 导出 `class Agent implements ACPAgent`。构造函数只接收 `ACPService.Interface`，以 `private readonly service` 保存；没有自身可变状态、锁或生命周期钩子。`implements ACPAgent` 要求下面的方法签名与 ACP 代理协议一致。
- `L35-L37` `initialize(params: InitializeRequest)`：把请求原样传给 `this.service.initialize(params)`，再交给 `run`。输出是 `run` 返回的异步结果（成功值由 service 决定）；参数验证、协议版本与初始化副作用不在此层。
- `L39-L41` `authenticate(params: AuthenticateRequest)`：同样是 service 委托。认证失败走 `ACPService.Error` 到 `RequestError` 的映射；本层不保存凭据。
- `L43-L45` `newSession(params: NewSessionRequest)`：调用 `service.newSession` 并统一运行/错误处理。session ID、父会话或持久化语义由 service 决定。
- `L47-L49` `loadSession(params: LoadSessionRequest)`：委托加载既有 session；不存在、损坏或权限错误均由 service 错误转换处理。
- `L51-L53` `listSessions(params: ListSessionsRequest)`：委托枚举；本层不分页、不排序，也不加并发限制。
- `L55-L57` `resumeSession(params: ResumeSessionRequest)`：委托恢复活动；恢复冲突、失效 session 或取消由 service 表达。
- `L59-L61` `closeSession(params: CloseSessionRequest)`：委托关闭。此方法本身没有“所有子节点完成后才能关闭”的判断，因此该门禁若需要必须放入 service/上层状态机。
- `L63-L65` `unstable_forkSession(params: ForkSessionRequest)`：将 ACP 的不稳定扩展操作映射到 `service.forkSession`。名称中的 `unstable_` 是协议兼容信号，不改变调用语义；派生失败仍由 `run` 统一转换。
- `L67-L69` `setSessionConfigOption(params: SetSessionConfigOptionRequest)`：委托配置选项修改；没有本地默认值或回滚。
- `L71-L73` `setSessionMode(params: SetSessionModeRequest)`：委托模式切换；模式合法性由 service/ACP 协议层决定。
- `L75-L77` `unstable_setSessionModel(params: SetSessionModelRequest)`：委托模型切换；本层不检查当前会话是否忙碌。
- `L79-L81` `prompt(params: PromptRequest)`：委托一次提示处理。流式更新、工具副作用、会话事件应由 `ACPService` 使用保存的 `AgentSideConnection` 发出；本方法只返回终态 Promise。
- `L83-L85` `cancel(params: CancelNotification)`：委托取消通知。没有 token、超时或强制中断逻辑；取消是否可达、取消哪个运行中的操作由 service 实现。
- `L88-L92` 私有泛型 `run<A>(effect: Effect.Effect<A, ACPService.Error>)` 是全类共用的边界。先执行 `effect.pipe(Effect.mapError(ACPError.toRequestError))`，再用 `Effect.runPromise` 转成 Promise。已知的 `ACPService.Error` 变成 ACP `RequestError` 并以 rejected Promise 返回。若 Promise 链捕获到 defect：`L90` 已经是 `RequestError` 的直接重新抛出，保持原 code/message/data；否则 `L91` 先经 `ACPError.fromUnknownDefect(defect)` 生成安全内部错误，再经 `toRequestError` 转成协议错误。因而未知异常不会泄漏任意 defect 文本，也不会静默成功。
- `L95` `export * as ACP from "./agent"` 以命名空间再次导出本模块，允许调用者通过 `ACP.init`、`ACP.Agent` 访问同一适配层；不引入额外状态。

并发语义：每个 ACP 方法都会独立调用 `Effect.runPromise`，函数本身不串行化请求，也没有共享可变字段（service 引用是只读）。同一 `Agent` 上并发调用的顺序、session 锁、连接事件顺序和取消竞态只能由 `ACPService`/`AgentSideConnection` 保证；该文件只保证每次 effect 的错误类型边界一致。没有本地默认参数、重试、分页或输入截断。

## 状态、取消、恢复与副作用

本文件的唯一持有状态是构造时注入的 `service`；`init` 的 `_sdk` 只通过闭包传给 `ACPService.make`。没有持久化、checkpoint、重连 WAL、超时计时器或 retry loop。`cancel` 只是把 `CancelNotification` 交给 service，不能从代码本身推断取消是否立即生效；`Effect.runPromise` 也没有在此处接收外部 `AbortSignal`。`closeSession`、`forkSession`、`prompt` 等可能产生会话、模型、工具或连接事件副作用，但实际写盘/网络/通知均在 `ACPService` 与 `AgentSideConnection` 中完成。已知业务错误被稳定映射，未知 defect 被安全归一化；这层不自动重试，因此重试不会被意外重复触发。

## 源内测试与行为判据

`src/acp/agent.ts` 未包含测试；同目录对应的测试主要覆盖 `ACPService` 和 `ACPError`（例如 `test/acp/service-session.test.ts`、`test/acp/error.test.ts`），没有发现直接实例化本文件 `Agent` 并验证所有方法的专门测试。可独立验证的判据是：构造假的 `ACPService.Interface`，确认每个 ACP 方法收到完全相同的 `params` 且只调用一次；让 effect 成功时 Promise 返回成功值；让 service 返回 `ACPService.Error` 时得到 `RequestError`；让 effect 抛出已有 `RequestError` 时对象不被二次包装；让 effect 抛出普通 `Error`/非 Error 值时得到 `ACPError.fromUnknownDefect` 产生的安全 `RequestError`。并发验证应检查 wrapper 不添加锁，调用顺序与 service 的调度一致。

## zenpi Rust 映射

### 可复用原语与协议落点

1. **消息/邮箱**：优先复用 `src/protocol.rs` 的版本化 JSONL 与 `MailboxRequest`（`Send`、`Receive`、`Acknowledge`、`Claim`、`Complete`，`L79-L113`），而不是另造 ACP 专用队列。`Command::Mailbox`（`L242-L255`、解析在 `L479-L484`）是 host 入口；`validate_mailbox` 的 ID、文本、TTL 和分页上限（`L544-L590`）可作为 DAG 消息的硬边界。`src/session.rs` 的 `SessionMailbox` 是追加式、同 inode 锁、每次 mutation reload+append 的持久邮箱（`L3570-L3591`）；消息自带 sender/recipient/session、request ID、digest、TTL 和状态，`enqueue/list/update` 位于 `L3617-L3795`。因此 worker 节点可用 `session_id` 作为地址，`message_id`/digest 作为幂等键。
2. **事件/流**：`src/protocol.rs` 的 `StdioEvent`（`L885-L918`）适合承载节点 `accepted/started/progress/green/blocked/closed` 事件；`src/core.rs` 的 `AgentEvent`（`L283-L325`）已有 `TurnAccepted`、`ToolProgress`、`Handoff`、`Error` 等语义。`src/headless.rs` 负责把 agent/provider 事件转成关联的 JSONL 响应，适合作为 ACP `AgentSideConnection.sessionUpdate` 的 Rust 等价宿主。
3. **运行协议/取消**：`src/runtime.rs` 的 `BackgroundRunner` 以有界 command/event channel 启动 worker（`L272-L306`），`RuntimeEvent` 提供 `Accepted`、`Started`、`Queued`、`CancelRequested`、`Completed`、`Rejected`、`Closed`（`L171-L193`）；`CancellationToken` 是协作式、幂等取消（`L49-L99`）。这对应 TS 的 `run`/`cancel` 边界，但必须由 worker 主动轮询 token，不能假设线程被强杀。
4. **会话关系**：`src/core.rs::Turn` 的 `parent_id`（`L146-L191`）和 `src/session.rs` 的 tree ancestry/fork（`tree_ancestry_turns`、`fork_at_tree_leaf`，约 `L1038-L1136`）可提供历史上的父链。`TreeAction::Fork`（`src/protocol.rs:L1161-L1207`）以及 `Agent::tree_request`（`src/core.rs:L922-L1001`）已有显式 fork/选择接口；它们应成为 DAG 节点建立或派生 worker 的审计入口，而不是在 `ACP Agent` 包装层隐式 fork。
5. **保活**：`src/session.rs::LiveSessionRegistry`（`L3274-L3355`）已提供带 `owner_epoch` 的 `register`、`heartbeat`、TTL `active` 和 `unregister`；`claim_next`/`claim_message`（`L3361-L3438`）只做一次认领、不自动启动调度器。worker lease 还可用 `src/core.rs::renew_blueprint_worker`（`L2846-L2868`），取消用 `cancel_blueprint_worker`（`L2870-L2885`）。

### 面向 DAG 的建议类型和最小机制

- 在 `src/session.rs` 或独立 `src/dag.rs` 增加持久化 `DagNodeRecord { node_id, session_id, parent_id, children: BTreeSet<String>, state, generation, last_heartbeat_ms, lease_id }` 与 `DagNodeState::{Running, AwaitingChildren, Green, Failed, Closed}`。关系索引必须同时记录直接 parent、直接 child；通过 parent 的 children 集合计算直接 siblings，通过 parent.parent_id 寻址 grandparent。不要仅凭 session 文件名推断关系。
- 在 `src/protocol.rs` 增加严格的 `DagMessage`/`DagRelation`（`Parent`、`Grandparent`、`Sibling`、`Child`）或将 relation 放入现有 mailbox payload；校验目标必须是同一 DAG、关系必须可由持久节点记录证明，沿用 `MAX_ID_BYTES`、`MAX_MAILBOX_TEXT_BYTES`、TTL 和 `deny_unknown_fields`。Rust 接口可收敛为 `DAGComms::send(from, to, relation, message_id, payload, ttl)`、`receive(node, cursor, limit)`、`claim`、`complete`，内部调用 `SessionMailbox`。
- 在 `src/headless.rs` 的 `execute_mailbox`/`mailbox_response`（约 `L8943-L9088`）增加 relation 授权和当前 owner 检查；`Send` 只写 durable mailbox，`Claim` 只有 live owner 能做，`Complete` 必须匹配 claim token。对 parent/grandparent/sibling/child 的路由应在此处或 DAG registry 统一校验，避免任意 session ID 发送。
- 在 `src/runtime.rs` 增加 `DagWorkerRequest`（节点 ID、上下文 parent、lease、工作负载）适配 `BackgroundRunner::try_submit`（`L308-L315`）；以 `InputBoundaryGate::with_context_parent`（`L823-L830`）携带父上下文。派生的最小步骤是：持久化 child record/关系和 operation intent；`SessionStore::fork_at_tree_leaf` 或新 session 建立 child session；提交 `BackgroundRunner`；收到 `Started` 后登记 owner/heartbeat。派生失败必须留下可恢复的 durable 状态。
- 关闭门禁应由 `src/core.rs::Agent` 的新 `close_dag_node(node_id)` 实现，而不是直接调用普通 `close`：读取自身及递归全部 child/grandchild 的状态，只有“自身 terminal-green 且每个后代均 terminal-green”才写 `Closed` 并停止 runner；任何未完成、失败、过期 claim 或未知 outcome 都返回 `AwaitingChildren/NotReady`，保持 owner heartbeat。等待期间可再次 `try_submit` 派生新 worker，且新 child 必须挂到同一 node record 并带 generation/lease，不能覆盖未结算 operation。
- `src/core.rs` 的 `AgentPhase`（`Idle/Running/Closed`，`L273-L279`）可继续表示单个执行器，但 DAG 层需另有 `AwaitingChildren`，否则 `AgentPhase::Closed` 会过早表示整棵子树完成。`AgentEvent`/`StdioEvent` 应发出 `child_spawned`、`child_green`、`close_deferred`、`heartbeat_expired` 等可重放事件。
- `src/approval.rs` 的 `ApprovalCoordinator` 仍是 side-effect 工具的 rendezvous；worker 派生不能绕过 `ApprovalRequest` 的 `policy_digest`/`lease_id` 校验（`ApprovalMode::WorkerAllowAfterPreflight`）。`src/tool_runtime.rs` 的 `execute_tool_batch` 已提供有界并行、取消轮询和不隐式重试（`L157-L176`、`L250-L347`），适合作为节点内部工作执行器。
- `src/providers/**` 只负责 provider 连接/流和模型能力；DAG 通信、关系授权、保活和关闭判定不应放进 provider。`src/core.rs` 负责把 provider `ProviderEvent` 变成节点事件，`src/headless.rs` 负责传输，`src/session.rs` 负责 durable graph/mailbox，`src/runtime.rs` 负责 worker 生命周期。

### 与本文件的差异清单（可执行、可验证）

1. TS `Agent` 是薄 ACP façade，Rust 目前没有一一对应的 ACP façade；新增 `src/acp.rs`/`src/dag.rs` 的 façade 后，应让每个协议方法只做 `service` 委托与 `AgentError -> RequestError` 映射，并用表格测试逐项覆盖。
2. TS 用 `Effect.runPromise` 捕获 typed error/defect；Rust 用 `Result<T, AgentError>`，需要显式 `DAGError -> protocol error` 和 `from_panic` 边界，避免 panic 穿过 JSONL host。
3. TS `closeSession`/`forkSession` 没有子树绿度规则；Rust 必须增加递归绿度判定、`AwaitingChildren`、heartbeat/lease 保活，并测试“任一孙节点未绿则 close 被拒绝且可派生新 worker”。
4. TS 的父子关系由 ACP service 定义；Rust 应以 durable `DagNodeRecord` 加 `Turn.parent_id`/tree fork 双重证据建立 parent、grandparent、直接 sibling、直接 child 路由，并测试越权目标、过期 TTL、旧 owner_epoch claim 均失败。
5. TS 仅委托取消；Rust 还必须验证 `CancellationToken`、`RuntimeEvent::Completed` 与 session operation terminal marker 的顺序，确保取消不是回滚，未知副作用不会自动重试。

## 未决问题

无法从 `agent.ts` 确认 `ACPService.make` 是否为每个连接创建独立 session owner、`AgentSideConnection` 的事件顺序保证、ACP `ForkSessionRequest` 的父子语义，以及 service 是否自行实现超时/重试/持久化；这些点需继续阅读 `src/acp/service.ts` 与 ACP SDK 定义后才能确定。源文件本身没有 DAG 绿度或祖孙通信规则。

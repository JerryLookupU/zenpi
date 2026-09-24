# AC-054 — opencode/packages/opencode/src/sync/README.md

source_id/item_id: AC-054
source_path: opencode/packages/opencode/src/sync/README.md
source_hash: a4b011597b8bfe1dfaf167bbae538dac1c7d36dc862cf9562a0fb416e42a0b8f
source_bytes: 9181
source_lines: 179
coverage: 字节范围 `0-9181`（完整文件，含末尾范围）；行范围 `L1-L179`（完整文件，含注释、示例与全部文字）

## 完整行为复盘

这是一份设计说明，不是 `SyncEvent` 的实现文件；源内没有 Rust/TypeScript 的导出声明表。以下按文档中出现的每个 API 符号和行为逐一复盘，不能从 README 推断出的返回类型或具体错误码明确标为未定义。

- `SyncEvent.run(Definition, payload)`：`Definition` 的 `schema` 约束第二参数，示例中的 `Updated` 要求 `{ sessionID, info }`，且字段会做类型检查（L1-L5、L83-L91、L119-L121）。行为上它类似 `Bus.publish`，但进入 event sourcing 流程；同步事件会先于状态 mutation 发出，再由 projector 执行效果（L41-L41、L83-L91）。成功返回值未在源中说明；schema 不匹配时文档只确认会被类型系统拦截，没有运行时错误形状。事件会自动重新发布为 bus event，因此有一次记录/投影路径和一次兼容监听路径（L91-L93）。
- `SyncEvent.define`：输入是对象，至少包括 `type`、`version`、`aggregate`、`schema`；输出是供 `run`、`Bus.subscribe` 使用的同步事件定义（L67-L81）。`version` 在示例中显式为 `1`，没有默认值；`aggregate` 示例为 `sessionID`，用于事件聚合标识。`schema` 是事件数据的类型边界。可选 `busSchema` 描述向旧 bus 消费者暴露的 `properties` 形状（L145-L165）；其目的不是改变同步事件的数据，而是让旧 API 在编译期得到兼容类型。`busSchema` 与运行时 `convertEvent` 的实际转换并不会自动互相校验（L163-L165），所以转换不匹配是文档明确提示的边界风险。
- `BusEvent.define`：输入事件名和 schema，例如 `"session.diff"`、`Schema.Struct(...)`，输出旧 `Bus` 可发布/订阅的定义（L53-L63）。`Bus.publish(Diff, payload)` 写入 bus，`Bus.subscribe(Diff, handler)` 逐事件接收（L65-L65）。README 没有规定返回值、异常、队列容量或并发顺序。
- `SyncEvent.subscribeAll(handler)`：注册一个接收所有同步事件的记录型监听器；回调的通用事件保证有 `id`、`seq`，但因为没有缩小到具体定义，`data` 是未知类型（L7-L15、L101-L104、L123-L131）。它不是按类型订阅 API，主要服务于客户端记录事件。事件顺序由同步事件的序列号承载；具体监听器是否阻塞发布没有说明。
- `Bus.subscribe(Updated, handler)`：对单个同步事件定义订阅，回调读取兼容 bus 形状的 `event.properties.info.title`，类型检查完整（L20-L24、L103-L107、L136-L140）。它是监听单个事件的推荐路径；不能用 `SyncEvent` 直接逐事件订阅。`client.subscribe("session.updated", handler)` 是客户端同类旧接口，仍读取 `properties`（L23-L24、L136-L140）。
- `Bus.publish(Updated, payload)`：虽然目前可以把同步事件定义传入旧 publish API 且能类型检查（L17-L18、L133-L134），文档明确这是兼容过渡行为，不应作为同步事件发布路径，未来应成为类型错误；调用者也不应绕过 projector 直接改数据库或手工处理事件（L105-L107）。
- `SyncEvent.init`：系统安装 projector 的初始化入口；说明中的安装位置是 `server/projectors.js`。它还安装运行时 `convertEvent` hook（L109-L113）。这意味着初始化必须早于依赖投影的事件运行；README 未说明重复初始化、初始化失败或返回值。
- `convertEvent`：把同步事件在发布到 bus 前重塑为旧 bus 形状，原则上应避免，仅为临时向后兼容保留（L111-L115）。已知例子是 `session.updated`：同步事件只含变更字段，转换器补成完整 session 对象（L115-L115）。运行时转换不是 `busSchema` 的自动校验对象，形状契约由定义中的 `busSchema` 在类型层提供（L163-L165）。

行为主线是“单写者 + 可回放日志 + projector”：一个设备控制并修改 session，其他设备读取事件日志并本地重放（L27-L33）；每生成一个事件，数字 `seq` 加一，因只有一个 writer，不需要分布式时钟或因果排序（L31-L33）。同步事件的事件形状是 `type/id/seq/aggregateID/data`，bus 事件是 `type/properties`，转换后 `data` 与 `properties` 语义大体相同（L95-L99）。单个同步事件不直接订阅，统一记录用 `subscribeAll`，面向业务消费仍走 `Bus.subscribe`（L101-L107）。

## 状态、取消、恢复与副作用

源只定义了 event sourcing 的状态顺序和回放目标，没有定义取消、超时、重试或持久化文件格式。可确认的持久化语义是：事件需要被记录、客户端通过事件日志重放 session；`seq` 是单写者产生的全序游标（L27-L33）。可确认的副作用顺序是“先发同步事件，后由 projectors 处理 mutation”（L39-L41）；因此 projector 是状态变更边界，而不是订阅回调中任意直接写库。

README 没有取消 token、deadline、超时默认值、重试策略、幂等键、事务回滚、断线恢复握手或并发锁语义。唯一相关的边界是单 writer 假设：如果未来允许多个 writer，当前简单递增 `seq` 不再足以解决排序；这是由 L31-L33 的前提直接推出的限制。`convertEvent` 是发布兼容副作用，不应被当作新的业务 mutation。`busSchema` 也只保证类型层的旧接口形状，转换实现本身仍可能漂移（L163-L165）。

## 源内测试与行为判据

源内未包含测试；该目录的 README 只有示例、设计约束和替代方案讨论，没有 test case 或测试命令。可独立验证的行为判据如下：

1. 对一个 `SyncEvent.define` 定义，合法 payload 可通过 `SyncEvent.run`，非法字段在类型检查阶段被拒绝；同一事件可被 `Bus.subscribe` 以 `properties` 形状读取（L1-L5、L20-L24、L117-L141）。
2. 每个事件具有 `type/id/seq/aggregateID/data`，同一单 writer 连续生成事件时 `seq` 单调递增；回放端按日志顺序重放（L27-L33、L95-L99）。
3. `SyncEvent.subscribeAll` 能观测全部事件且至少访问 `id`、`seq`；个别事件必须通过 `Bus.subscribe`，不能依赖 `SyncEvent` 的逐事件订阅（L101-L107、L123-L131）。
4. `session.updated` 的同步增量经过 `convertEvent` 后仍向旧 bus 客户端呈现完整旧形状；定义中的 `busSchema` 应与该转换结果一致（L109-L115、L145-L165）。
5. 不能把 `Bus.publish(Updated, ...)` 当作正式同步写入入口；正式路径必须经过 `SyncEvent.run` 和 projector（L83-L91、L105-L107、L133-L134）。

## zenpi Rust 映射

总体结论：README 的可复用核心不是某个具体 TS API，而是“结构化消息/事件 + 单写者全序 + projector/状态消费者 + 兼容投影”。zenpi 已有的持久邮箱、JSONL session journal、有界 runtime 和 host approval 足以组成最小 DAG 通信层，但现有 `Turn.parent_id` 不能单独承担 worker DAG 拓扑。

### 通信原语与 DAG 语义

- 消息/邮箱：优先复用 `src/session.rs:L3171-L3794` 的 `MailboxMessage`、`MailboxAction`、`SessionMailbox`、`LiveSessionRegistry`。它已有 `sender_session_id`、`recipient_session_id`、`request_id`、不可变 `digest`、TTL、`Queued/Acknowledged/Claimed/Succeeded/Failed/Expired` 状态、claim token、分页和同 workspace 校验。它对应源文的“事件/记录可回放”思想，但当前 mailbox 是请求状态机，不是广播事件流；应在 `payload` 中承载 `DagMessage { kind, node_id, parent_id, target, generation, reply_to, body }`，保留原有 digest/TTL/claim 约束。
- 事件：复用 `src/core.rs:L283-L325` 的 `AgentEvent` 作为进程/host 事件出口，复用 `SessionStore` 的 JSONL 记录作为 durable event log。建议新增明确的 `DagEvent`/`DagEventKind`（例如 `WorkAccepted`、`NodeGreen`、`ChildSpawned`、`CloseDeferred`、`LeaseRenewed`、`WorkArrived`），由唯一 session owner 写入，再由 headless/UI projector 转换为 `AgentEvent`；不要让多个 worker 直接共同写同一 journal。
- 协议：复用 `src/protocol.rs:L27-L115` 的版本化 JSONL、`MailboxRequest`、`MailboxOutcome` 与 `src/protocol.rs:L1121-L1233` 的 session-scoped request 校验。最小可执行扩展是 `dag_send`、`dag_receive`、`dag_status`、`dag_spawn`、`dag_close` 五类请求，字段必须带 `session_id`、`node_id`、`request_id`、`schema_version`，并把 sender 身份留给 host/session owner 解析，不能信任客户端自报 sender。
- 父子会话：新增持久的 `DagNodeMeta { node_id, session_id, parent_node_id: Option<NodeId>, children: BTreeSet<NodeId>, state, generation, owner_epoch }`，写入 node 的 session journal 或受同一 writer 保护的 DAG 索引。`Turn.parent_id`（`src/core.rs:L143-L207`）只表示对话/steer 的记录关联；它可作为展示关联，但不能替代 worker parent/grandparent/child 拓扑。`src/session_tree.rs` 的 `TreeEntry.parent_id` 是 transcript tree 索引，也应明确与 worker DAG 分离。
- 定向关系：worker 节点要找 parent、grandparent、直接 sibling、直接 child，先从 `DagNodeMeta` 得到直接 parent 和 children；grandparent 是 parent 的 parent；直接 sibling 是 parent children 集合减去自身；直接 child 是 children 集合。通信统一经过 addressed mailbox，不新增 provider 特殊通道。若收件方不在线，消息留在 TTL mailbox；在线 owner 通过 `LiveSessionRegistry::heartbeat` 表示可收取，但 `claim_next` 只 claim、不会自动启动 worker（`src/session.rs:L3295-L3415`），启动必须由 host/runtime 显式完成。

### 保活、派生与 close 的最小机制

1. **保活**：每个节点有 `owner_epoch`、`last_seen_ms` 和 lease expiry；使用 `LiveSessionRegistry::register/heartbeat/active`（`src/session.rs:L3295-L3360`）。`runtime.rs` 的 driver 每个 poll 周期检查取消、lease 和 mailbox，并持久化 `LeaseRenewed`。epoch 变化时旧 worker 的 claim/result 必须拒绝，沿用现有 token 前缀和 owner-epoch 绑定。
2. **派生**：新工作进入某节点 mailbox 后，owner 先 durable append `WorkArrived`/`ChildSpawnRequested`，再调用 `BackgroundRunner::try_submit` 或 `BackgroundRunner::spawn`（`src/runtime.rs:L279-L359`）。派生出的 child 必须获得新的 `node_id`、`session_id`/owner epoch、`parent_node_id` 和有限 budget；父节点记录 child，child 完成后回送 `NodeGreen`/结果。`src/tool_runtime.rs:L142-L233` 的 bounded batch 可复用来限制一个 worker 同时处理的 provider/tool 调用，但它不是 DAG scheduler，也不会为新 mailbox 消息自动派生 worker。
3. **close 判定**：定义 `is_green(node) = node.self_state == Green && node.children.iter().all(|child| is_green(child))`，递归覆盖全部 child/grandchild。只有自身和全部后代都为 Green、没有未完成 claim、没有未处理新 work、且 durable `NodeGreen` 已落盘，才允许写 `NodeClosed` 并释放 lease。任一 child 为 queued/claimed/running/failed/expired/unknown，或新消息在 close race 中到达，都保持父节点 Alive；把新消息转成新一代 child worker，并写 `CloseDeferred`，不能把“当前任务完成”误当成“整个子树完成”。
4. **取消与副作用**：复用 `src/runtime.rs:L44-L104` 的 `CancellationToken`。取消只能请求 worker 在边界退出，不能回滚已发生副作用；对 provider/tool 的 unknown outcome 继续留在 durable ledger，不能自动重试。close、settle、spawn 的顺序应先写 journal，再改变内存状态或释放 lease，失败则保持保活/可恢复状态。

### 现有模块落点与差异清单

- `src/headless.rs`：`run_async_streams`/`run_async_stdio` 已使用 `BackgroundRunner`、有界 `AsyncEventBuffer`，并在 mailbox 路由处调用 `SessionMailbox`（`src/headless.rs:L806-L1080`、`L8950-L9130`）。落点是增加 DAG JSONL command 的解析/响应、将 `DagEvent` 转成可回放 `StdioEvent`，以及把 close/heartbeat/派生操作限制在 session owner。现有 `execute_mailbox` 返回 `execution_started: false`，这正好保留“邮箱入队与 worker 启动分离”；需要新增明确的 host dispatch，而不是让 `SessionMailbox` 隐式启动。
- `src/core.rs`：`AgentEvent`、`Turn`、`run_active_turn_cancelable_with_events` 提供事件关联、同步 Agent 所有权和取消边界（`src/core.rs:L143-L325`、`L3660-L3915`）。落点是增加 DAG node context/worker binding 和 durable lifecycle event；不要把 `Turn.parent_id` 直接解释为 grandparent/sibling 关系。已有 `settle_blueprint_worker`/`renew_blueprint_worker`/`cancel_blueprint_worker`（`src/core.rs:L2810-L2905`）可挂接预算、lease 与 close 判定，但 settlement 仍需等待全部子节点。
- `src/session.rs`：是持久化、锁、邮箱、owner epoch 的核心落点。现有 mailbox 已拒绝过期任务的隐式 retry，并在同 workspace 校验 sender/recipient（`src/session.rs:L3574-L3794`）；差异是缺少 DAG 元数据、递归 descendant green 判定、worker 状态事件和“新工作到达时延期 close”的原子事务。应在同一 journal writer 下增加这些记录，避免并行 writer 破坏源文要求的 total order。
- `src/runtime.rs`：`RuntimeConfig` 默认 command capacity 32、event capacity 128、pending 32、poll interval 10ms，并提供 `try_submit`、`try_cancel`、`try_shutdown_with_grace`、FIFO follow-up 与 `RuntimeEvent`（`src/runtime.rs:L106-L201`、`L279-L359`）。可直接承载一个 node worker 的最小 driver；差异是现有 runner 是单 active job + follow-up FIFO，不知道 DAG 邻接关系，需要在上层维护 `DagNodeMeta` 和 `ChildCompletion` 聚合。
- `src/tool_runtime.rs`：`execute_tool_batch` 已限制最多 32 calls/256 KiB、支持 sequential/parallel、5ms polling、panic 转内部错误，且取消不会 detach（`src/tool_runtime.rs:L142-L233`、`L260-L349`）。它适合作为 worker 节点内部的 side-effect 执行器；不要用它实现跨节点通信，也不要把一个工具 batch 的成功当作 node green。节点 green 还必须等待所有 child/grandchild。
- `src/protocol.rs`：现有 `MailboxRequest` 的 `Send/Receive/Acknowledge/Claim/Complete` 和 TTL/ID 校验是最小 wire primitive（`src/protocol.rs:L81-L115`、`L527-L590`）。差异是协议目前以 text/result 为主，没有 DAG target kind、parent/child topology、status snapshot、spawn/close 原语；扩展需保持 `PROTOCOL_VERSION`/legacy projection 兼容、限制 frame/text 大小，并区分 durable session cursor 与 stdout event cursor。
- `src/approval.rs`：`ApprovalCoordinator` 是 worker-host 的进程内 rendezvous，50ms 条件变量等待、`cancel_all`、`emergency_cancel`、先持久化 accepted decision 再释放 worker（`src/approval.rs:L126-L242`、`L348-L460`）。它可复用为 DAG worker 的人工批准通道；但 approval 只决定 side effect，不能授予 parent/grandparent/sibling/child 的通信权限。worker 的记忆授权已有 fail-closed 约束，应继续禁止 worker 自行扩大权限。
- `src/providers/**`：`src/providers/mod.rs:L1-L160` 及各 provider 定义只负责 provider protocol/dialect/endpoint/header/capability 路由，不负责 session event、mailbox、DAG 生命周期。无需把通信 primitive 放入 provider；worker 通过 `core`/`tool_runtime` 调 provider，provider delta 仍由 `headless`/`AgentEvent::Provider` 投影。可验证差异是 provider failure/unknown outcome 只能产生 worker 状态事件，不能绕过 DAG close 判定；任何 provider retry 必须带 node/operation idempotency 记录。

最小验收序列：创建 root、child、grandchild 三个 `DagNodeMeta`；root 通过 mailbox 向 parent/grandparent/sibling/child 发带 digest 的消息；child heartbeat 超时后拒绝旧 claim；child 与 grandchild 都写 `NodeGreen` 后 root 才能 `NodeClosed`；若 root close 前收到新 work，则 root 保持 Alive、产生新 child/generation，并且重启后从 session journal 重建同一结论。该序列同时验证单 writer 全序、消息幂等、保活、派生和递归 close。

## 未决问题

1. README 没有给出 `SyncEvent.run`、`SyncEvent.subscribeAll`、`SyncEvent.init`、`convertEvent` 的实际返回类型、异常类型、背压和线程安全保证，不能从源确认。
2. README 只说“事件日志可回放”，没有规定日志存储介质、崩溃恢复、重复 projector 的幂等方式、版本迁移算法或多 writer 方案。
3. `busSchema` 与 `convertEvent` 的运行时一致性需要实现侧测试；源明确说不会自动校验，不能假设类型通过就等于转换安全（L163-L165）。
4. 对 zenpi 而言，现有 `SessionTree` 是否最终承载 worker DAG 仍无法由本 README 确认；当前映射按 transcript tree 与 worker DAG 分离处理。

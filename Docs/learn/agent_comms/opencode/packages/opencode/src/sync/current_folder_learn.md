# `opencode/packages/opencode/src/sync` 目录级学习汇总

## 目录职责

`sync` 是 opencode 会话同步的事件溯源边界。它把“会改变 session 状态的动作”先包装为可记录、可回放、可排序的 sync event，再由 projector 执行状态 mutation，并自动转换成既有 `Bus` 能理解的事件形状，以兼容旧消费者。设计前提是单 writer：每生成一个事件就递增数字 `seq`，其他设备只读取事件日志并按序重放，因此不需要分布式时钟或因果排序。sync event 的完整形状是 `type/id/seq/aggregateID/data`，传统 bus event 则是 `type/properties`；`convertEvent` 负责临时兼容转换，`busSchema` 只提供编译期的旧形状约束，运行时不会自动验证转换结果。

## 模块清单

- `README.md`：设计契约和使用示例，定义/说明 `SyncEvent.define`、`SyncEvent.run`、`SyncEvent.subscribeAll`、`SyncEvent.init`、`BusEvent.define`、`Bus.publish`、`Bus.subscribe` 与 `convertEvent` 的职责；它不是实现文件，没有实际导出表。业务发布应走 `SyncEvent.run`，个别事件监听仍走 `Bus.subscribe`，全量记录走 `SyncEvent.subscribeAll`。
- `schema.ts`：唯一实际导出为 `EventID`。它以 `Schema.String.check(Schema.isStartsWith("evt"))` 做字符串约束，用 `Schema.brand("EventID")` 做名义类型，并通过 `statics` 附加 `EventID.ascending(id?: string)`；生成委托给 `Identifier.ascending("event", id)`，本文件不负责持久化、通信或事件分发。

## 运行时数据流与控制流

正式路径是：调用者用事件定义的 `schema` 构造 payload，调用 `SyncEvent.run(Definition, payload)`；系统生成 `id`、单调 `seq` 和 `aggregateID`，先把 sync event 进入可记录的事件流，再由 projector 执行 mutation，随后自动重发布为 bus event。记录端使用 `SyncEvent.subscribeAll` 获取通用事件（可可靠读取 `id`、`seq`，但 `data` 为未知类型）写入日志；业务端对单个事件使用 `Bus.subscribe(Definition, handler)`，从 `event.properties` 读取兼容形状。`SyncEvent.init` 安装 projector 与 `convertEvent` hook；例如 `session.updated` 的 sync event 只含增量字段，转换后仍可向旧客户端提供完整 session。`Bus.publish(Definition, payload)` 虽暂时可用且能类型检查，但只是兼容过渡，不能绕过 event sourcing 直接写库或手工处理事件。

`EventID` 处于上述流的标识层：校验时要求值以 `evt` 开头并赋予品牌类型，`ascending()` 生成事件 ID。它没有跨进程唯一性、日志落盘或重放语义，连续升序的具体保证来自外部 `Identifier` 的模块级时间/计数器实现。

## 错误、取消与恢复语义

源目录没有定义取消 token、deadline、超时、重试、背压、事务回滚或线程安全规则；也没有测试、持久化格式、崩溃恢复握手或多 writer 方案。可确认的边界只有：schema 不匹配会在类型/解码层失败；`EventID` 的普通非法值校验失败，而显式非法 ID 可能由 `Identifier.ascending` 抛错；projector 是状态 mutation 的边界；事件日志按 `seq` 回放。README 未规定 `run`、`init`、`subscribeAll` 或转换器的返回值和异常形状。单 writer 前提若被打破，简单递增 `seq` 不足以解决并发排序。`busSchema` 通过不等于 `convertEvent` 运行时结果正确，二者必须另行测试。

## 与 zenpi Rust 的映射建议

### 通信原语、父子关系与协议

优先复用 `src/session.rs` 的 `MailboxMessage`、`MailboxAction`、`SessionMailbox`、`LiveSessionRegistry`：已有 `sender_session_id`、`recipient_session_id`、`request_id`、digest、TTL、claim token 以及 `Queued/Acknowledged/Claimed/Succeeded/Failed/Expired` 状态，足以作为 durable 的点对点消息/邮箱原语。消息 payload 可定义为 `DagMessage { kind, node_id, parent_id, target, generation, reply_to, body }`，但 mailbox 当前是请求状态机而非广播事件流；全量事件仍应进入 session 的 JSONL journal。`src/core.rs` 的 `AgentEvent` 和 `src/runtime.rs` 的 `RuntimeEvent` 适合作为短生命周期的进度出口，不能替代 durable journal。

协议层复用既有 `src/protocol.rs` 的版本化 JSONL、`MailboxRequest`/`MailboxOutcome`、ID/TTL/文本大小校验，并增补 `dag_send`、`dag_receive`、`dag_status`、`dag_spawn`、`dag_close` 等 typed action；字段至少包含 `session_id`、`node_id`、`request_id`、`schema_version`，sender 身份由 host/session owner 解析，不信任客户端自报。可以新增 Rust `EventId` newtype，以 `TryFrom`/`serde` 强制非空、`evt` 前缀、长度和控制字符约束；不要把现有 `StdioEvent.sequence` 当作跨会话事件 ID。

worker DAG 必须与对话 transcript tree 分离。新增持久化 `DagNodeMeta { node_id, session_id, parent_node_id, children, state, generation, owner_epoch }` 或等价记录；`Turn.parent_id` 只表示 turn/steer 关联，`session_tree::TreeEntry.parent_id` 只表示 transcript 树，均不能独自表达 worker 的 parent、grandparent、direct sibling、direct child。由直接边计算关系：grandparent 是 parent 的 parent，direct sibling 是 parent 的 children 去掉自身，direct child 是当前节点的 children；所有定向通信均通过目标 `session_id` 的 mailbox。

### 保活、派生和递归 close 的最小机制

每个节点至少保存 `owner_epoch`、`last_seen_ms`、lease expiry 和状态；用 `LiveSessionRegistry::register/heartbeat/active` 做存活准入，epoch 改变时拒绝旧 worker 的 claim/result。runtime driver 每个 poll 周期检查取消、lease 和 mailbox，并把 `LeaseRenewed` 之类事件 durable append。邮箱收到新工作时，session owner 先记录 `WorkArrived`/`ChildSpawnRequested`，再调用 `BackgroundRunner::try_submit` 或 `BackgroundRunner::spawn`；child 获得新的 `node_id`、`session_id`、`parent_node_id`、owner epoch 和有限 budget，完成后回送 `NodeGreen`/结果。`LiveSessionRegistry::claim_next` 只负责 claim，不会自动启动 worker，因此 host/runtime 必须显式 dispatch；`QueueFull`、`Closed`、lease 过期都必须是可观察结果。

close guard 应是持久化前的单写者原子判定：`is_green(node) = self_state == Green && every descendant (child/grandchild/全部后代) == Green`，同时没有未完成 claim、未处理 mailbox work 或活动 lease，且 `NodeGreen` 已落盘，才允许追加一次 `NodeClosed` 并释放 lease。任一后代为 queued/running/failed/expired/unknown，或 close 竞争窗口有新消息到达，都保持节点 Alive；追加 `CloseDeferred`，续租，并把新工作派生为新 generation 的 child。`CancellationToken` 只提供协作式边界取消，不能回滚已经发生的 provider/tool 副作用；unknown outcome 应留在 durable ledger，不得假装成功或无条件重试。

### 指定文件的实际落点

- `src/headless.rs`：在 `run_async_streams`/`run_async_stdio` 的 bounded event 输出和现有 mailbox 路由旁增加 DAG command/event 的 JSON 解析、响应、heartbeat、spawn、close 投影；`execute_mailbox` 保持“入队与启动分离”，由 host 明确 dispatch，headless 不绕过 core 的 close guard。
- `src/core.rs`：扩展 `AgentEvent`、worker binding 和 durable lifecycle event，增加 `descendants_green`、`close_node_if_green`、`keep_alive_or_derive` 等纯判定/编排入口。已有 `Turn`、cancelable turn、`settle_blueprint_worker`/`renew_blueprint_worker`/`cancel_blueprint_worker` 可接入，但 settlement 必须等待整个后代子树，不能把 `Turn.parent_id` 解释为 DAG 拓扑。
- `src/session.rs`：作为 DAG 元数据、边、lease 变化、派生 receipt、mailbox 结果和 JSONL 顺序的权威存储；恢复时重放 journal，把未完成 lease 恢复为 Alive/NeedsRetry 检查态。应在同一 writer 保护下实现“新工作延期 close”的原子性，沿用 `Expired` 是时钟视图而非成功 receipt 的语义。
- `src/runtime.rs`：以 `BackgroundRunner`、bounded command/event channel、`try_submit`、`RuntimeEvent`、`CancellationToken` 承载单节点 driver；上层额外维护 `DagNodeMeta` 与 child completion 聚合。`src/tool_runtime.rs` 可继续限制单 worker 的 tool batch，但 tool batch 成功不等于 node green。

最小验收闭环是：建立 root、child、grandchild 及 sibling 拓扑；从节点向 parent、grandparent、direct sibling、direct child 各发带 digest 的消息；验证 heartbeat 过期会拒绝旧 claim；只有自身及全部传递后代写入 `NodeGreen` 后 root 才产生一次 `NodeClosed`；close 前新 work 到达时 root 保活并派生新 child/generation；重启后从 journal 重建相同结论。

## 未决问题

1. opencode 源文没有给出 `SyncEvent.run`、`SyncEvent.init`、`subscribeAll`、`convertEvent` 的具体返回值、异常、队列容量、并发和重复初始化语义。
2. 事件日志的实际介质、崩溃恢复、重复 projector 的幂等策略、版本迁移和多 writer 演进方案未在本目录定义。
3. `busSchema` 与 `convertEvent` 的一致性没有自动校验，必须由实现侧测试覆盖；`EventID` 是否要求跨进程唯一、是否允许裸值 `evt` 也需要明确。
4. zenpi 的 DAG 拓扑权威存储仍需在 `SessionStore` journal、独立 DAG 索引与 `SessionTree` 之间做最终选择；当前建议是 worker DAG 独立于 transcript tree，但复用同一 session writer 和恢复游标。

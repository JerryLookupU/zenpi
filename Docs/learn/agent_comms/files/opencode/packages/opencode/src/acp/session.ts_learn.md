# AC-050 — opencode/packages/opencode/src/acp/session.ts

source_id/item_id: `opencode/packages/opencode/src/acp/session.ts` / `AC-050`
source_path: `opencode/packages/opencode/src/acp/session.ts`
source_hash: `cc73f70f7d7071fd6bf808871c65bcb87d9f737f9e292be5e279ee51f5c9507e`
source_bytes: `7944`
source_lines: `232`
coverage: 已按源文件顺序完整读取；字节范围 `0-7943`（7944 字节），行范围 `L1-L232`（232 行）。

## 完整行为复盘

这是一个基于 `Effect` 的 ACP 会话内存存储层，不连接文件、数据库或 provider。导入只引入类型 `McpServer`、`Message`、`Part` 以及 `LayerNode`、`ProviderV2`、`ModelV2`、`Context/Effect/Layer/Ref` 和 `ACPError`（L1-L7）。

导出数据类型如下：

- `SelectedModel`（L9-L12）是 `{ providerID, modelID }`，两个字段分别采用 `ProviderV2.ID` 与 `ModelV2.ID`。
- `KnownMessagePartMetadata`（L14-L22）以 `messageId`、`partId` 为必填键，`partType`、`role`、`ignored`、`toolCallId`、任意 `metadata` 均可选；它保存的是消息 part 的索引元数据，不保存消息正文。
- `Info`（L24-L33）是会话快照：`id`、`cwd`、只读 `mcpServers`、`createdAt`、可选模型/variant/mode，以及 `knownParts: ReadonlyMap`。
- `StoreInput`（L35-L43）创建/加载输入；只有 `id`、`cwd` 必填，`mcpServers` 默认空数组，`createdAt` 默认当前时间，其余配置可缺省。
- `RecordPartMetadataInput`（L45-L54）带 `sessionId`、消息/part 标识及同一组可选元数据；`PartMetadataLookupInput`（L56-L60）只带三段定位键。
- `Interface`（L62-L91）声明完整服务面：`create/load/list/get/tryGet/remove`，模型、variant、mode 的读写，以及 part 元数据的写入、严格读取和容错读取。除 `tryGet`、`remove`、`tryGetPartMetadata` 外，按 ID 查找失败统一产生 `ACPError.SessionNotFoundError`。
- `Service`（L93）是 `Context.Service<Service, Interface>()`，键为 `@opencode/ACP/Session`；`State`（L95）是 `Map<string, Info>`。

实现主体是 `layer`（L97-L202）。`Effect.gen` 内先用 `Ref.make<State>(new Map())` 建立进程内状态（L99-L100）；因此所有方法都必须在该 Layer 提供的同一 `Ref` 上运行。

- 内部 `store(input)`（L102-L106）调用 `makeSession`，再用 `Ref.update` 复制旧 `Map`、按 `session.id` 写入新会话，并返回 `snapshot(session)`。`create` 与 `load` 在服务导出时都指向它（L169-L171），所以当前实现的 `load` 不是从持久化介质恢复，而是覆盖/写入一个新内存会话；同 ID 是最后一次写入胜出，没有唯一性检查。
- 内部 `tryGet(sessionId)`（L108-L112）从 `Ref` 读取；不存在返回 `undefined`，存在返回快照，调用者不能直接拿到内部 `Info`。
- 内部 `get(sessionId)`（L114-L118）复用 `tryGet`；不存在时构造 `SessionNotFoundError({ sessionId })` 并以 Effect 失败结束，存在时返回快照。
- 内部 `update(sessionId, fn)`（L120-L129）使用 `Ref.modify` 原子读取。找不到返回原 state 并在 L127-L129 转为 `SessionNotFoundError`；找到则调用 `fn(session)` 得到新 `Info`，以新 `Map` 写回，并返回 `snapshot(next)`。这要求更新函数按不可变值风格工作；并发更新不会原地改共享 `Map`，而是由 `Ref` 串行化每次修改。
- 内部 `remove(sessionId)`（L131-L139）也是原子 `Ref.modify`：不存在返回 `undefined` 且保留原 map；存在则复制 map、删除键、返回被删除会话的快照。删除不存在不是错误。
- `setModel`、`setVariant`、`setMode`（L141-L151）分别通过 `update` 做浅对象展开，仅替换对应字段；传入 `undefined` 即清除字段，其他字段和 `knownParts` 保持原引用/值语义。会话不存在时沿用 `update` 的错误。
- `recordPartMetadata(input)`（L153-L167）先按输入拼出 `metadata` 对象（包括值为 `undefined` 的可选字段），再更新指定会话的 `knownParts`：复制旧 Map，以 `partMetadataKey(input)` 覆盖同键，最后 `Effect.as(metadata)` 返回刚记录的对象。相同 `messageId:partId` 是覆盖写；不校验 part 是否真实存在。会话不存在失败。
- `list(cwd?)`（L172-L177）读取当前 map 的所有值；`cwd` 缺省或为空字符串时不过滤，否则仅保留 `session.cwd === cwd` 的会话；对每个值做 `snapshot`，再按 `createdAt.getTime()` 降序 `toSorted`。没有分页、去重、ID 校验或错误路径。
- `getModel`、`getVariant`、`getMode`（L182-L191）先 `get` 再取字段，因此不存在会失败，存在但字段缺省则返回 `undefined`。
- `getPartMetadata`（L194-L196）严格通过 `get` 获取会话后按复合键查 Map；`tryGetPartMetadata`（L197-L199）通过 `tryGet`，会话不存在和键不存在都返回 `undefined`。

`node`（L204）用 `LayerNode.make({ service: Service, layer, deps: [] })` 导出无依赖节点，供 ACP 依赖图装配。`makeSession`（L206-L217）建立初始值：复制 `mcpServers`（缺省 `[]`）、把给定 `createdAt` 重新 `new Date` 或取当前时间，保留可选模型/variant/mode，并总是创建空 `knownParts`。`snapshot`（L219-L225）浅复制 `Info`，再复制数组、`Date` 和 `knownParts Map`；嵌套对象和 `metadata: unknown` 不做深拷贝。`partMetadataKey`（L228-L230）直接返回 ``${messageId}:${partId}``，未转义分隔符，调用者若在 ID 中自行使用 `:` 可能发生键碰撞。L232 的 `export * as ACPSession from "./session"` 提供模块命名空间再导出；它不是第二份状态。

## 状态、取消、恢复与副作用

- 状态只有 `Ref<State>` 中的 `Map`（L95-L100），生命周期绑定 Layer/进程；没有文件持久化、恢复日志、跨进程锁或网络副作用。`createdAt` 只是值对象，不触发时钟之外的外部动作。
- `Ref.update/modify/get` 提供 Effect 侧的原子读改写；`snapshot` 隔离了顶层对象、数组、日期和 Map，避免调用者通过返回值直接篡改内部容器。由于是浅拷贝，`mcpServers` 元素和 `metadata` 内嵌对象仍可能共享引用。
- 源文件没有取消 token、超时、重试、后台任务、Promise 并发控制或错误恢复。Effect 失败仅用于缺失会话的 `get/update` 系列；`create/load/list/tryGet/remove` 的接口声明没有错误通道。
- 没有 session close、续租或派生 worker 语义；`remove` 是立即从内存 Map 删除，不能恢复被删状态。`load` 也不会读取既有状态，因此不能解释为恢复。

## 源内测试与行为判据

源文件内未包含测试；同目录未发现 `session` 测试文件。可独立验证的判据：在同一 `layer` 中 `create` 后 `get` 应得到等值但容器独立的快照；`setModel(id, undefined)` 应清除模型；`get(missing)` 应失败为 `SessionNotFoundError` 而 `tryGet(missing)` 为 `undefined`；`list(cwd)` 应只返回精确 cwd 且按创建时间倒序；重复 `recordPartMetadata` 的复合键应后写覆盖；`remove` 后 `tryGet` 与 `tryGetPartMetadata` 均应为空。并发验证应检查多次 `update` 不丢失 `Ref` 的原子写入，而不能假定 Map 的原地可变性。

## zenpi Rust 映射

### 可直接复用的通信原语

本文件的“会话对象 + 有地址的元数据索引”在 zenpi 中不应照搬成进程内无界共享 Map。最接近且已经存在的原语是 `src/session.rs` 的 `SessionMailbox`：`MailboxMessage` 带 sender/recipient/session/request、digest、TTL、sequence、claim token、status/result（L3170-L3194），`MailboxAction` 定义 `Acknowledge/Claim/Complete/Fail`（L3258-L3265），每次变更在 inode 锁下恢复并追加同步 journal（L3570-L3579、L3743-L3795）。它能同时充当消息、邮箱和可审计事件协议。发送/接收/认领/完成由 `src/protocol.rs` 的 `MailboxRequest`（L79-L113）和 `validate_mailbox`（L544-L583）承载；`src/headless.rs::execute_mailbox` 已把它接到 JSONL（L8954-L9093），但返回值明确 `execution_started: false`，不会自动调度 worker。

### 会话父子关系与 DAG 差异

`src/core.rs::Turn` 的 `parent_id`（L140-L175）只表达会话内 conversation turn 的父项，不能直接等同 DAG 节点父子；当前 `src/session.rs` 的 mailbox 也只按显式 session ID 寻址，没有 grandparent/sibling/child 关系解析。建议在 `src/session.rs` 的 durable event/journal 层增加可验证的 `DagNodeRecord`（`node_id`、`session_id`、`parent_node_id`、`depth`、直接 children、状态、`generation`），并提供纯查询函数 `parent(node)`、`grandparent(node)`、`direct_siblings(node)`、`direct_children(node)`、`descendants(node)`；关系变更先写事件再更新内存投影。验证要求是同一 workspace、父引用存在、无环、child 只能有一个直接 parent，且 session `Turn.parent_id` 仍只用于转录。

### 面向 zenpi DAG 的最小机制

1. **通信**：worker 负责节点 `N` 时，由关系查询把 parent、grandparent、直接 sibling、直接 child 映射成允许的 session ID，再复用 `SessionMailbox::enqueue/list/find_message/update`。所有消息必须带 `node_id`、`in_reply_to`、`generation` 和 `request_id`；只允许这些关系集合，不能让 worker 任意指定 workspace 路径。跨节点回执使用现有 `finish_claim_with_reply` 的“先完成接收方、再向原 sender 投递 reply”语义（L3490-L3559），delivery 失败必须返回可重试错误而不是伪造成功。
2. **保活**：`src/session.rs::LiveSessionRegistry` 已有 `register/heartbeat/unregister/active`（L3274-L3359），用 `owner_epoch + last_seen_ms + ttl_ms` 做最小 liveness；`src/core.rs` 暴露 `register_live_owner/heartbeat_live_owner/unregister_live_owner`（L819-L844）。DAG worker 的保活循环应只做 heartbeat，并在 lease 将过期前调用 `Agent::renew_blueprint_worker`（L2846-L2868）；过期或 epoch 不匹配时拒绝 claim/complete。
3. **派生**：先在 `src/session.rs` 追加“child admitted/derived”事件并建立 node record，再用 `src/runtime.rs::BackgroundRunner::try_submit`（L308-L316）提交 child job。`BackgroundRunner` 当前是一个 active + FIFO pending（L443-L577），适合做每个节点的 bounded executor；`RuntimeEvent::Accepted/Started/Queued/Completed/CancelRequested/Closed`（L171-L193）可复用为调度事件。派生不是隐式重试：child 的 request_id、generation、lease 必须新建，父节点收到 mailbox reply 后显式更新 child outcome。
4. **关闭判据**：新增 `DagNodeStatus::{Running,Green,Failed,Cancelled,NeedsWork,Closed}` 和 `subtree_green(node)`；只有节点自身为 `Green` 且全部 direct child/grandchild 的递归结果为 Green，才允许结算并 close。当前 `Agent::try_close` 只关闭 extensions、写 `skill_session_close`、清空运行态并设置 `Closed`（L5711-L5734），不会检查 children；因此 DAG supervisor 必须在调用它前阻挡 close。若自身或任一后代非绿，保留 live owner/lease，写 `NeedsWork`，生成新 generation 并派生 worker；子节点完成但父/祖父尚未绿时不能关闭任一仍需通信的 owner。

### 指定模块落点对照

- `src/session.rs`：承载 `DagNodeRecord`/状态事件、关系索引和现有 durable mailbox；复用 TTL、digest、claim token、幂等 request ID 与 `LiveSessionRegistry`，并为父/祖父/兄弟/子节点地址提供受限查询。`SessionStore` 的追加日志和恢复模型比源文件的内存 `Ref<Map>` 更适合保活和崩溃恢复。
- `src/core.rs`：在 `Agent` 上增加“节点上下文/当前 generation”以及 `subtree_green` 前置检查；复用 `WorkerExecutionBinding`（L37-L81）绑定 `blueprint_id/goal_id/item_id/lease_id/policy_digest/expires_at_ms`，用 `admit_blueprint_worker` 的原子 gate+lease+journal 流程（L1578-L1697），用 `settle_blueprint_worker` 在观察 terminal 且 children 已 reap 后结算（L2823-L2843）。注意工具完成不等于 worker terminal，源码已明确要求等待 worker 与 children（L5342-L5344）。
- `src/headless.rs`：作为 host/wire owner 负责 mailbox 命令、heartbeat、事件回放和派生结果的 JSONL 投影；现有有界 `AsyncEventBuffer` 按数量/字节丢弃普通进度、保留 admission/tool 边界（L833-L1061），可承载 DAG lifecycle event。必须把 `execute_mailbox` 的 `execution_started: false`（L8982-L8985）扩展为“显式 claim 后调度”的宿主路径，并把 parent/child reply 作为独立 `StdioEvent`，不能混成 terminal response。
- `src/runtime.rs`：复用 `CancellationToken` 的协作取消与 `mark_completed` 终态竞态规则（L49-L99）、bounded command/event channel、FIFO follow-up 和 shutdown grace（L101-L193、L327-L421）。建议在此处或紧邻模块落一个 `DagSupervisor`，维护 `node_id -> children/parent/status/lease`，但不得把 `BackgroundRunner` 的 FIFO pending 误认为 DAG 拓扑；child 派生、等待回执、保活和 `subtree_green` 应在 supervisor 层完成。
- `src/tool_runtime.rs`：它只执行一个 provider tool batch，最大 32 calls/256 KiB，并行 handler 在 owner unwind 时先置 cancellation 且不会 detach（L157-L176、L252-L347）；可复用 bounded concurrency、取消轮询和“副作用结果未知时不得自动重试”的判据，但不能把 tool batch 当作 DAG child。源代码还明确 executor 不写第二份 journal、不重试中断调用（L1-L7）。
- `src/protocol.rs`：复用版本化 JSONL、`MailboxRequest`、`StdioEvent` 的 sequence/request_id/turn_id envelope（L882-L918），在 `MailboxRequest` 或新增 DAG control action 中加入 node relation/generation 字段并做 `MAX_ID_BYTES`、TTL、文本和 action 严格校验；解析错误应保持“不修改 session”的协议边界（L735-L851）。
- `src/approval.rs`：不是 DAG 路由器，但可复用其 `ApprovalCoordinator` 的 pending/visible/accepted 交握；响应先持久化再释放 worker，持久化失败会 retract、保持 fail-closed（L356-L390），`cancel_all/emergency_cancel` 负责唤醒等待者（L409-L446）。所有派生 worker 的 side effect 仍应通过既有 approval/gate，而不是因父子关系自动授权。
- `src/providers/**`：`providers::registry::ModelDescriptor` 提供模型能力、上下文预算和 digest（L74-L136）；`providers::connection` 负责 route/provider/model/auth 校验，且明确“不做 session mutation”（L1-L1、L166-L200）。DAG 节点可沿用 `core::provider_request_scope` 传递 `session_id/operation_id/lease_id/policy_digest`（L530-L564），但 parent/child/sibling 通信应留在 session/protocol，不能塞进 provider route；provider adapter 的取消、timeout、retry 只决定一次模型调用，不决定 DAG close。

### 差异清单与可执行验证

源 `session.ts` 是无持久化、无关系图、无取消、无调度、最后写入胜出的单进程快照服务；zenpi 是追加日志、跨进程 mailbox、live owner/lease、bounded runtime 和 fail-closed approval。实现映射后至少验证：同一 node 的重复 request 幂等；只允许四类关系收发；heartbeat 过期后 claim 被拒；child/grandchild 任一非绿时 `try_close` 不被调用；全子树绿且 terminal/children 已 reap 后才 `settle_blueprint_worker` 与 `Agent::try_close`；派生 worker 使用新 generation/request ID，旧未知副作用不会自动重放；取消只阻止尚未执行的工作，不能声称已回滚 provider/tool side effect。

## 未决问题

源文件无法确认 `load` 是否在更高层被赋予“恢复”语义；就本文件实现它与 `create` 完全相同。`snapshot` 是否需要深拷贝 `McpServer` 和 `metadata` 也未规定。对 zenpi 而言，现有源码未定义 DAG 节点记录、grandparent/sibling 关系或自动派生 supervisor；上述字段、事件名称及 `DagSupervisor` 是满足目标需求的明确落点建议，不是 `session.ts` 已实现的行为。

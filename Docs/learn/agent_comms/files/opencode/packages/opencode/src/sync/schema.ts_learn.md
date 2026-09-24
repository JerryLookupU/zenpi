# AC-055 — opencode/packages/opencode/src/sync/schema.ts

source_id/item_id: AC-055  
source_path: opencode/packages/opencode/src/sync/schema.ts  
source_hash: 4c3366bb2b30e0e1f7868c3d0237293d1f8bbbc60d168916f7383014f0695e14  
source_bytes: 330  
source_lines: 11  
coverage: 完整读取 bytes 0-329、lines 1-11（含全部注释、导入、类型构造与导出符号）。

## 完整行为复盘

源文件只有一个导出符号 `EventID`，没有导出函数、类或运行时服务。

- `import { Schema } from "effect"`（L1-L1）：引入 Effect 的 schema 组合器；本文件把它用作字符串约束、品牌化和静态构造器的载体。
- `import { Identifier } from "@/id/id"`（L3-L3）：引入 ID 生成命名空间；本文件只调用 `Identifier.ascending("event", id)`，不自行实现时间、随机数或排序逻辑。
- `import { statics } from "@opencode-ai/core/schema"`（L4-L4）：引入给 schema 对象附加静态方法的辅助器。该辅助器的实现是把回调返回的成员挂到 schema 上，因此最终 `EventID` 同时是 schema 与带 `ascending` 方法的对象；这属于外部 helper 行为，本文件未重复实现。
- `export const EventID = ...`（L6-L11）：先以 `Schema.String` 接受字符串，再用 `Schema.isStartsWith("evt")`（L6-L6）增加前缀约束；输入不是字符串或不以 `evt` 开头时，schema 解码/校验失败。之后 `Schema.brand("EventID")`（L7-L7）添加 TypeScript 名义品牌，使通过校验的普通字符串在类型层面成为 `EventID`，不会自动改变字符串值。最后 `pipe(statics(...))`（L6-L11）附加静态方法。
- `ascending: (id?: string) => s.make(Identifier.ascending("event", id))`（L8-L10）：`ascending` 的参数可省略。省略时委托 `Identifier.ascending` 生成一个事件前缀的升序 ID，再由 `s.make` 构造出品牌 schema 对应的值；传入 `id` 时，生成器会复用它，但若显式 ID 不满足 `event` 前缀，生成器可抛出错误（该错误来自 `Identifier`，不是此文件捕获的错误）。正常结果应满足 `evt` 前缀检查并可作为 `EventID` 使用。注意：`Schema.isStartsWith("evt")` 只表达 `evt` 开头，不在此处限制长度、下划线后缀、字符集或全局唯一性；具体 `evt_...` 形态来自 `Identifier`。

边界与并发语义：空字符串、`"ev"`、非字符串和任意不以 `evt` 开头的值被 schema 拒绝；`"evt"` 本身按“starts with”规则可通过，是否符合更严格 ID 形态取决于外部 `Identifier` 生成/使用处。`EventID` 构造本身是同步、无 I/O、无 Promise、无取消点；`ascending()` 依赖 `Identifier` 模块的共享生成状态（其 `lastTimestamp`/`counter` 用于单进程内升序生成），本文件没有锁、事务或并发协调，也没有声明跨线程/跨进程唯一性。辅助器 `statics` 不做异步调度。

## 状态、取消、恢复与副作用

本文件没有取消、超时、重试、持久化、恢复或网络/文件/进程副作用。schema 定义在模块加载时建立，校验失败只返回 Effect schema 的失败结果；`ascending()` 的生成路径会读取时间并使用随机性/模块级计数器，这是唯一可见的运行时副作用来源，且没有本地错误恢复。显式非法 `id` 的异常也未在本文件转成 `Either`/`Option`。事件 ID 一旦生成，如何落盘、重放、确认、过期均由调用方负责。

## 源内测试与行为判据

源内未包含测试。可独立验证的判据：

1. 对 `EventID` 解码合法字符串 `evt_abc` 应成功并获得品牌类型；`"event_abc"`、`"ev"`、空串和非字符串应失败。
2. `EventID.ascending()` 应返回以 `evt` 开头的值；连续调用应保持 `Identifier.ascending` 约定的升序排序语义（同一时间片的排序以生成器实现为准）。
3. `EventID.ascending("evt_existing")` 应复用合法前缀 ID；传入不以 `event` 开头的显式值应在 `Identifier.ascending` 处失败，而不是静默修正。
4. 验证应区分“schema 校验失败”和“生成器抛错”，并确认本文件不写文件、不发消息、不触发 provider 调用。

## zenpi Rust 映射

### 可复用原语与现有落点

1. **ID/schema 原语**：在 `src/protocol.rs` 或新建 `src/dag.rs` 定义 `pub struct EventId(String)` 与 `TryFrom<String>`/`serde` 实现，校验至少为非空、`evt` 前缀、长度和控制字符；提供 `EventId::ascending(given: Option<&str>) -> Result<Self, DagError>`。它对应 `EventID`（L6-L11），但 Rust 不能只靠 TypeScript brand，要把不变量放进构造器。随机/时间生成应复用 zenpi 的 ID 约定，并用单调序列或 `JobId` 关联；不要把现有 `StdioEvent.sequence: u64`（`src/protocol.rs:L885-L900`）误当作跨会话 `EventId`。
2. **消息/邮箱**：优先复用 `src/protocol.rs:L81-L105` 的 `MailboxOutcome`/`MailboxRequest` 和 `Command::Mailbox`（`L211-L263`），底层复用 `src/session.rs:L3171-L3265` 的 `MailboxMessage`、TTL、claim token、digest 与状态机，以及 `SessionMailbox`（`src/session.rs:L3574-L3655`）。这提供 durable message、ACK、claim、complete/fail；发送者只提供 recipient/request/payload/ttl，权限仍由 owner 检查。
3. **事件/协议**：短生命周期进度使用 `RuntimeEvent`（`src/runtime.rs:L171-L190`）和 `StdioEvent`（`src/protocol.rs:L882-L918`），事件与 terminal response 分离，能关联 `request_id`/`turn_id` 并按 sequence 重放。跨进程入口继续走 `parse_line`/`Command`（`src/protocol.rs:L795-L851`），不要把 DAG 进度伪装成第二个命令结果。
4. **会话父子关系**：`src/core.rs:L140-L205` 的 `Turn.parent_id` 可保留对话父链，`InputBoundary.context_parent`（`src/runtime.rs:L769-L791`）可表达下一模型边界；但二者都不是完整 DAG 拓扑。新增持久化的 `DagSessionRelation { node_id, session_id, parent_session_id, edge_kind }`，并在 `src/session.rs` 的 session 记录中保存直接 parent/children 边。grandparent 通过 parent 的 parent 得到；direct sibling 通过相同 parent 查询；direct child 通过 children 查询。真正发送仍按目标 `session_id` 调 `SessionMailbox`，不能凭 `parent_id` 猜收件人。
5. **保活与派生最小机制**：复用 `src/session.rs:L3274-L3335` 的 `LiveSessionOwner`/`LiveSessionRegistry` 做 owner 注册和 heartbeat；其注释明确它只是 admission boundary，不会启动 scheduler。为 DAG 增加最小 `WorkerLease { lease_id, node_id, session_id, owner_epoch, heartbeat_deadline_ms, status }`，把 heartbeat 作为 mailbox/protocol 的 typed action 或 `RuntimeEvent::Heartbeat`。派生时由 host 在 `src/runtime.rs:L237-L315` 的 `BackgroundRunner::spawn` 创建新 worker，输入携带 parent/node/lease；`try_submit` 的 `QueueFull`/`Closed` 必须成为可观察拒绝，不能静默丢工作。
6. **取消与副作用闸门**：`src/runtime.rs:L49-L99` 的 `CancellationToken` 是协作取消且幂等，`src/core.rs:L3662-L3712` 的 cancelable turn 能在持久化前阻止迟到结果；`src/approval.rs:L22-L38,L41-L104,L126-L200` 的 `ApprovalCoordinator`/`ApprovalMode::WorkerAllowAfterPreflight` 适合约束 worker 工具副作用。`WorkerExecutionBinding`（`src/core.rs:L37-L81`）应随 lease 传递并校验 digest/过期时间。`src/tool_runtime.rs:L137-L178` 的 `ToolBatchDecision`/`execute_tool_batch` 继续作为工具执行边界，不让 DAG mailbox 直接执行工具。

### DAG 规则的可执行落点

- 在 `src/core.rs` 增加纯函数 `descendants_green(graph, node_id)` 和事务式 `close_node_if_green(session, node_id)`：只有当前 node 为 `Green` 且所有 child、grandchild 及全部传递后代均为 `Green`，才写 `Closed`。任一后代为 `Pending/Running/Failed/Cancelled/Expired` 或存在未完成 lease，都返回保活结果并保持节点可调度；此判定不能只看当前 `AgentPhase`。
- 在 `src/core.rs` 增加 `keep_alive_or_derive(node, work)`：先更新 lease/heartbeat，再把新工作包装为 `WorkerExecutionBinding`，调用 `BackgroundRunner::try_submit` 或 `spawn` 派生 worker；派生 worker 的 `parent_session_id` 必须是当前节点 session，grandparent/sibling/child 的通信则统一通过 `SessionMailbox` 的精确 recipient。返回值应区分 `KeptAlive`、`Derived(JobId)`、`QueueFull`、`Closed`、`LeaseExpired`。
- 在 `src/headless.rs` 接入 host 命令与可重放事件：现有 `run_blueprint_next`（约 `L3175`）、`sync_goal_status_after_execution`（约 `L3628`）、`execute_mailbox`（约 `L8954`）是 DAG 调度、状态同步、邮箱执行的实际入口；增加节点状态/heartbeat/derive/close 的 JSON 命令或事件，并通过既有 `run_async_streams`/bounded event mailbox 输出。headless 层只编排和呈现，不绕过 `core` 的 close guard。
- 在 `src/session.rs` 持久化 `DagNodeState`、边、lease 变化、派生 receipt 和 mailbox 结果；恢复时重放 journal，把未完成 lease 标为 `NeedsRetry`/`AliveCheck`，而不是假定已 close。现有 `MailboxStatus::Expired` 已表明过期是时钟派生视图，不应伪造成功执行 receipt（`src/session.rs:L3229-L3253`）。
- 在 `src/protocol.rs` 为 DAG 增加严格的 `DagAction`/`DagEvent` 类型，沿用 `deny_unknown_fields`、长度校验、相关 ID 和 sequence；`StdioEvent` 负责观察，`MailboxRequest` 负责命令投递，`CheckpointCursor` 负责恢复游标，三者不可混为一套状态。
- `src/providers/**` 不承载 DAG 拓扑或 close 判定。现有 provider 的 `Protocol`、`RouteRule`、`ProviderDefinition`（`src/providers/mod.rs:L13-L152`）只负责服务路由、认证、能力声明；worker 的 provider 流事件可经 `AgentEvent`/`StdioEvent` 转发，但 provider 重试不得改变 DAG lease 状态，lease 续期/派生由 `core`/`runtime` 管理。

### 差异清单与验证方式

- 源实现是 Effect 的运行时 schema + TypeScript brand；zenpi 是 `serde`/Rust newtype，必须显式实现解析、序列化和错误类型。
- 源实现只有 `evt` 前缀事件 ID，没有 parent、child、sibling、lease 或 close 规则；zenpi 需要新增拓扑和生命周期记录，不能把 `EventId` 当 DAG 状态。
- 源实现没有持久化/恢复/取消；zenpi 已有 mailbox journal、checkpoint、`CancellationToken` 和 approval，所以应组合现有能力而不是新增第二套邮箱。
- 验证最小闭环：创建 parent、grandparent、两个 sibling 与 child session；从任一节点向四类 recipient 各发一条 mailbox，检查 digest/ACK/claim/complete；让一个 grandchild 非 `Green`，断言 `close_node_if_green` 拒绝并 heartbeat 保活；标绿全部传递后代，断言只产生一次 `Closed`；在 `QueueFull`、lease 过期、cancel 和进程恢复后，断言新工作仍可派生或明确返回拒绝，且没有伪造成功 receipt。

## 未决问题

1. 仅凭本源文件无法确认 `Schema.brand` 在项目编译配置中的最终静态类型展示，以及 `s.make` 对已校验值是否再次运行完整校验。
2. `EventID` 本身没有说明事件是否需要跨进程唯一、是否允许 `evt` 裸值；严格长度、字符集和持久化约束需由调用方确认。
3. zenpi 当前 `LiveSessionRegistry` 不提供 DAG 祖先/兄弟索引，现有 `Turn.parent_id` 也不能替代持久 DAG 边；应在实现前确定拓扑数据的权威存储与跨 workspace 通信策略。

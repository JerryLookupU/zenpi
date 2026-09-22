# OC-033 — packages/opencode/src/session/todo.ts

## 元信息

~~~yaml
source_id: OC-033
item_id: OC-033
source_path: packages/opencode/src/session/todo.ts
source_hash: efebbc2397e659fc122d8606c42dfd42b4400313ff6537429938b9610e6cf825
source_bytes: 2480
source_lines: 74
coverage:
  bytes: "[0, 2480)"
  lines: "L1-L74"
  read_order: "按文件顺序完整读取，含全部注释、import、类型、实现和导出符号"
~~~

## 完整行为复盘

- 依赖与导出别名（L1-L9）：文件引入 `LayerNode`、`SessionID`、Effect 运行时的 `Effect/Layer/Context`、数据库服务 `Database`、Drizzle 的 `eq/asc`、`TodoTable`、`EventV2Bridge` 与共享 schema 命名空间 `SessionTodo`。本文件不自行定义 todo 字段校验、数据库表结构或事件载荷，均委托这些依赖。
- `Info` 常量与类型（L11-L12）：`export const Info = SessionTodo.Info` 原样暴露 schema；`export type Info = SessionTodo.Info` 暴露其 TypeScript 类型。交叉读取的 schema 显示字段为 `content`、`status`、`priority` 三个字符串；注释语义建议状态为 `pending/in_progress/completed/cancelled`、优先级为 `high/medium/low`，但这里没有运行时枚举或长度限制，实际约束由外部 schema 决定。
- `Event`（L14-L14）：原样导出 `SessionTodo.Event`。对应共享 schema 的 `Updated` 事件类型是 `todo.updated`，载荷含 `sessionID` 与完整 `todos` 数组；本文件不改变事件名称或版本。
- `Interface`（L16-L19）：服务接口只有两个 Effect 操作。
  1. `update(input)` 接收 `{ sessionID: SessionID; todos: ReadonlyArray<Info> }`，成功输出 `Effect.Effect<void>`；输入数组是只读视图，更新语义是整表替换，不是增量 patch。
  2. `get(sessionID)` 接收会话 ID，成功输出 `Effect.Effect<Info[]>`；实现返回新数组，调用方不能据此直接修改数据库。签名未声明显式错误类型，底层失败会沿 Effect 通道传播，数据库失败在实现中又会被 `Effect.orDie` 转成 defect。
- `Service`（L21-L21）：`Context.Service<Service, Interface>()("@opencode/SessionTodo")` 定义 Effect 环境标签 `@opencode/SessionTodo`。缺少该服务或其 layer 时，调用方无法取得实现；没有默认空列表服务。
- layer 初始化（L23-L28）：私有 `layer` 通过 `Layer.effect` 构造服务。初始化 fiber 先取得 `EventV2Bridge.Service`（L26-L26），再取得 `Database.Service` 的 `db`（L27-L27）；任一依赖缺失或初始化失败，整个 layer 构造失败。没有额外状态缓存、锁、定时器或 worker。
- `update`（L29-L51）：`Effect.fn("Todo.update")` 为调用提供追踪名称，主体按顺序执行。
  1. L30-L48 调用 `db.transaction`，事务内先在 `TodoTable` 上按 `TodoTable.session_id = input.sessionID` 删除旧行（L33-L33）。
  2. 若 `input.todos.length === 0`（L34-L34），立即从事务返回；删除已发生，因此空数组明确表示清空会话 todo，不会插入占位行。
  3. 非空时用 `map((todo, position))`（L38-L44）按输入顺序生成行：同一 `session_id`、`content/status/priority` 原值，以及从 0 开始的 `position`。没有去重、排序、状态归一化或单一进行中任务规则；重复内容和任意 schema 允许的字符串都会写入。
  4. 插入通过 `tx.insert(TodoTable).values(...).run()`（L35-L46）完成。删除和插入处于同一数据库事务，插入失败时事务应由数据库层回滚，不能留下本次替换的一半结果。
  5. `.pipe(Effect.orDie)`（L49-L49）把数据库 Effect 的失败转为不可恢复 defect；本函数没有 catch、重试或错误转换。
  6. 事务成功提交后才执行 `events.publish(Event.Updated, input)`（L50-L50）。事件携带调用者原始 `input`，而不是从数据库重新读出的行。事件发布失败会使 `update` 整体失败，但已提交的数据库状态不会被本文件回滚；这形成“持久化成功、通知失败”的可观察边界。
  7. 只有事务和事件发布都完成，效果才以 `void` 成功；没有返回插入计数、版本或提交回执。
- `get`（L53-L66）：`Effect.fn("Todo.get")` 先从 `db.select().from(TodoTable)` 查询，再按会话 ID 过滤（L54-L58），以 `asc(TodoTable.position)` 升序返回（L58-L58），调用 `.all()` 取得全部行（L59-L59）。数据库读取失败同样在 L60-L60 经 `Effect.orDie` 变成 defect。L61-L65 将每行投影为只含 `content/status/priority` 的 `Info`，故不会向调用者泄漏 `session_id`、`position` 或时间戳。无匹配行时返回空数组；没有分页、上限、默认 todo 或 session 存在性检查。
- layer 返回与节点（L68-L72）：L68-L68 以 `Service.of({ update, get })` 暴露恰好两个方法。L72-L72 的 `LayerNode.make` 导出 `node`，声明依赖 `EventV2Bridge.node` 与 `Database.node`，使应用组装时能按依赖图安装。
- 聚合导出（L74-L74）：`export * as Todo from "./todo"` 提供命名空间聚合，外部可通过 `Todo.Info`、`Todo.Service`、`Todo.Event`、`Todo.node` 访问本模块导出；不会复制或改变运行时对象。

输入/输出与并发边界：`SessionID` 的格式校验来自 `./schema`，此处没有额外检查；`ReadonlyArray` 只保证调用约定，不阻止调用者在异步执行期间改变底层对象引用。单次 `update` 内部是删除→空判断→按位置插入→提交→发布事件的严格顺序。文件没有显式互斥或串行队列；多个 fiber 同时更新同一 `sessionID` 时，数据库事务的实际提交顺序决定最终列表，事件发布时间可能与调用启动顺序不同。每个事务内部的原子性由 `Database` 实现保证，而不是由 `Effect.fn` 提供。

## 状态、取消、恢复与副作用

本模块没有内存状态机、缓存、版本号、乐观锁或恢复扫描。唯一持久状态是 `TodoTable` 中按 `session_id + position` 保存的当前快照；同一会话的历史列表不会在本模块保留。`update` 的数据库写入是持久化副作用，且删除/插入位于一个事务中；`get` 只读数据库。

源代码没有 `Effect.timeout`、`Effect.retry`、退避、重放、补偿或幂等键。Effect 在数据库事务或事件发布的 yield 点被取消时，运行时可中断该效果；文件没有显式 finalizer 来补发事件或恢复旧列表。若取消发生在事务提交前，数据库驱动通常会中止/回滚，但具体取消粒度属于 `Database.Service`；若提交已完成后才取消，旧状态可能已经持久化。若事件发布在提交后失败或被取消，调用者会收到失败而数据库仍是新列表，重试会再次删除/插入并再次尝试发布，可能产生重复的 `todo.updated` 事件。

事件是外部通知副作用：L50-L50 通过 `EventV2Bridge.Service` 发布，桥接层会把事件路由到实例/工作区并转发给总线订阅者。事件载荷使用原始输入数组和会话 ID；本文件不声明 durable 选项、顺序号或去重策略。进程重启后的恢复依赖数据库内容，不能从本文件恢复事件丢失期间的通知。

## 源内测试与行为判据

源文件及 `packages/opencode/src/session` 同目录未包含专门测试，因此源内未包含测试。上游共享实现有可核对的 `packages/core/test/session-todo.test.ts`（L40-L94）：安装 `Database.node`、事件服务和 `SessionTodo.node` 后，测试先写入两个 todo，断言 `get` 保持输入顺序与三字段（L55-L65），直接查询 `TodoTable` 断言位置为 0、1（L66-L74）；随后用单元素替换、空数组清空，并断言依次发布三次 `todo.updated`，事件数据分别等于三次输入（L76-L91）。

可独立验证的判据：

1. 在有效 `sessionID` 下更新两个不同元素，`get` 必须按输入顺序返回，数据库行的 `position` 必须是连续的 0、1；交换输入顺序后结果也必须交换。
2. 更新空数组后，旧行数必须为 0，`get` 必须返回 `[]`，且仍应发布一次包含空 `todos` 的 `todo.updated`。
3. 更新成功后事件只能在事务提交之后可观察；制造插入失败时，旧列表应保持完整，且不得发布更新事件。
4. 让 `events.publish` 失败：调用方应观察到失败，但随后 `get` 仍应看到已提交的新列表，验证通知不是事务的一部分。
5. 让数据库查询失败，调用应表现为 Effect defect（`orDie`），而非本文件定义的可恢复业务错误。
6. 对不存在的会话 ID 调用 `get`，预期是空数组；本文件不因会话不存在而报错。

## zenpi Rust 映射

- `src/session.rs`（主要落点）：当前 `SessionStore`（定义于 L146-L146，事件追加 API 在 L1207-L1207）是 append-only JSONL 日志，而源实现依赖可更新的 `TodoTable` 快照。建议增加可序列化的 `TodoInfo { content: String, status: String, priority: String }`、按会话归属的 `Vec<TodoInfo>` 投影，以及 `replace_todos(session_id, todos)` 与 `get_todos(session_id)`。替换应先构造完整新快照，再以一个受锁保护的提交边界写入（可采用事件 `todo_updated` 并在启动时重建），位置按输入索引保存。需明确事件追加成功与快照可见性的顺序；若选择 JSONL，可在同一 record 写入 `session_id`、`todos`，避免模拟 SQL 的逐行半提交。
- `src/core.rs`（服务/调用归属）：`Agent`（L403-L403）是当前会话行为 owner，`AgentEvent`（L283-L283）是观察出口。建议由 `Agent` 或独立 `TodoStore` 持有会话 ID 校验，在 todowrite 工具和协议命令进入时调用 replace/get；成功后发一个带 `session_id`、完整列表的 `AgentEvent::TodoUpdated` 或统一事件 Value。不要把 provider 参数直接写盘，必须在会话 admission 已确定且持久化成功后发事件。
- `src/tool_runtime.rs`（工具批处理）：把 todo 替换视为 mutating、全批次顺序操作，复用现有工具批执行的取消检查；同一 session 的两个写入需要明确串行化，否则最终列表只由竞态提交顺序决定。源文件没有自动重试，因此工具批处理不应在未知提交结果时隐式重放；若要重试，需新增幂等键和审计事件并作为差异测试。
- `src/runtime.rs`（取消/并发）：`CancellationToken`（L56-L56）可在事务开始前、快照提交前和事件发送前检查。建议采用单 owner worker 或 `Mutex` 保护每个 session 的 todo projection，并让取消只阻止尚未提交的更新；提交后晚到取消不得伪造“未写入”。源实现没有超时，Rust 若增加 deadline 必须测试“超时前回滚、提交后通知失败”的边界。
- `src/protocol.rs`（线协议）：`Command`（L214-L214）可增加版本化的 `TodoGet`/`TodoUpdate` 请求，字段为 `session_id` 与 `todos`，响应返回完整列表或明确错误。沿用现有 ID/JSONL 限制，规定数组顺序就是 `position` 顺序；不要在 headless 层复制存储逻辑。也可只暴露标准工具调用，避免新增协议面，但必须提供可验证的查询路径。
- `src/approval.rs`（权限）：`ApprovalCoordinator`（L131-L131）可为 todo 写入增加独立操作名（例如 `todowrite`），在调用 `replace_todos` 前完成批准。源文件本身没有权限步骤，因此这是 zenpi 的新增治理差异；拒绝或取消必须保证零持久化，且要记录原因，不能把审批通过当成数据库提交成功。
- `src/headless.rs`（宿主与事件）：headless 的请求/事件重放和有界邮箱适合转发 `todo.updated`，但只应消费 core/session 发出的 canonical event。把 `session_id` 和完整 todos 纳入 JSONL 事件，使用既有 request/replay 预算；不要在 stdout handler 中直接修改 `SessionStore`，否则会绕过 approval、事务和并发 owner。
- `src/providers/**`（provider 适配）：provider 模块（统一 `ProviderEvent` 定义在 `src/backend.rs` L238-L238）只负责把模型返回的工具调用转换成标准参数，不应持有 todo 状态或写会话。各 provider 的 wire schema 应保持三个字段及数组顺序；参数解码、状态字符串策略、权限、持久化和 `todo.updated` 发布集中在 core/tool/session。不存在的 provider、畸形字段或未知状态应在统一 schema 层拒绝或按明确兼容策略处理。

可执行差异清单：

1. 写 Rust 单测验证空列表清除、位置连续、重复内容保留、未知字符串是否按 schema 策略处理。
2. 用失败注入验证“快照提交失败不发事件；提交成功后事件失败仍保留新快照”。
3. 用两个并发更新验证每次事务内部原子、最终状态等于某个完整输入，而不是删除/插入交错的半列表；同时验证事件顺序按提交完成顺序。
4. 重启 `SessionStore` 后重建 todo projection，确认恢复的是最近一次成功快照；对取消、进程崩溃和未知提交结果禁止无证据自动重试。
5. 对 headless、工具调用和每个 provider adapter 做同一 JSON fixture，验证 `content/status/priority` 与数组顺序在协议转换中不变。

## 未决问题

1. `Database.Service` 的事务隔离级别、取消时驱动行为和具体回滚保证不在本文件中。
2. `EventV2Bridge.publish` 是否 durable、是否有序列号/去重、失败是否会被上层吞掉，无法仅凭本文件确认。
3. `SessionID` 的完整格式限制来自 `./schema`，本文件没有重复声明。
4. `SessionTodo.Info` 的未知状态、空字符串、长度和额外字段策略由外部 schema/解码器决定；本文件只转出其定义。
5. 同一 `sessionID` 并发 `update` 的锁定与提交顺序由数据库和 Effect 运行时决定，源文件没有明确答案。
6. 数据库已经提交但事件发布失败时，上层是否重试、如何处理重复通知，不在该服务中实现。
7. `LayerNode` 的实例作用域（全局、实例或工作区）由组装器决定，源文件只声明依赖关系。

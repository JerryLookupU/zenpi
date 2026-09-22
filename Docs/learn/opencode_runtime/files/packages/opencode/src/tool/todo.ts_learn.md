# OC-055 — packages/opencode/src/tool/todo.ts

## 元信息

~~~yaml
source_id: OC-055
item_id: OC-055
source_path: packages/opencode/src/tool/todo.ts
source_hash: f18fdedc754e0842660a968fa225ed961fb516a544eb04209d8b44c7673df270
source_bytes: 1338
source_lines: 46
coverage:
  bytes: "[0, 1338)"
  lines: "L1-L46"
  read_order: "按文件顺序完整读取，包含全部 import、类型、导出符号和内嵌 execute"
~~~

## 完整行为复盘

- 依赖导入（L1-L4）：引入 Effect、Schema，工具定义命名空间 Tool，静态说明文本 DESCRIPTION_WRITE（来自 ./todowrite.txt），以及 Todo.Info/Todo.Service。本文件不实现数据库、事件或权限策略，只把这些能力接到工具生命周期。
- 导出参数模式 Parameters（L6-L8）：Schema.Struct({ todos: ... }) 要求顶层存在 todos 字段；其值是 Schema.Array(Todo.Info)，再经 Schema.mutable 标记为可变数组，并附带描述 “The updated todo list”。这里没有长度上限、去重规则、必需的唯一 in_progress 规则或状态枚举约束；项目当前 Todo.Info 的字段形状由外部 schema 提供（content、status、priority 均为字符串）。输入解码失败会在工具框架的参数校验阶段结束，execute 不会被调用。
- 内部类型 Metadata（L10-L12）：只有 todos: Todo.Info[]。它是执行结果的元数据类型，不是持久化模型；数组会原样放入返回值。
- 导出 TodoWriteTool（L14-L46）：调用 Tool.define<typeof Parameters, Metadata, Todo.Service>，工具 ID 固定为 "todowrite"（L14-L15）。初始化体是 Effect.gen（L16-L18），从 Effect 环境取出 Todo.Service；缺少该服务时，初始化无法完成，文件内没有兜底实现或默认空服务。
  - 初始化返回工具描述（L19-L22）：description 使用完整的 DESCRIPTION_WRITE 文本，parameters 使用上面的 Parameters，execute 接收已解码的 params 与 Tool.Context<Metadata>。返回对象通过 satisfies Tool.DefWithoutID<typeof Parameters, Metadata>（L44）做静态结构检查；ID 由 Tool.define 外层绑定。
  - execute（L22-L43）：内部再次使用顺序执行的 Effect.gen。
    1. L24-L29 先调用 ctx.ask 请求权限，固定请求体为 permission: "todowrite"、patterns: ["*"]、always: ["*"]、metadata: {}。权限被拒绝、等待失败或 Effect 被中断时，后续更新不会执行；本文件不捕获错误，也不自动重试。
    2. L31-L34 调用 todo.update，把 ctx.sessionID 作为目标会话，把 params.todos 作为完整新列表。工具本身没有增量编辑语义，列表替换、排序、落盘由 Todo.Service 决定。
    3. L36-L42 构造成功结果：title 为“未完成数量 + " todos"”，未完成数量是 params.todos.filter((x) => x.status !== "completed").length；只有精确等于字符串 "completed" 才计为完成，空数组得到 "0 todos"，未知状态也会计入未完成。output 是 JSON.stringify(params.todos, null, 2) 的两空格格式 JSON；metadata.todos 保存同一批输入数组。
- 输入/输出边界：输入必须能通过 Parameters 解码，顶层额外字段是否保留由 Effect Schema.Struct 的默认策略决定，源文件未覆盖。输出只承诺 title、output、metadata 三项；没有附件、分页或摘要截断逻辑。数组中的对象来自 schema 解码，因此普通 JSON 圆环引用不可能由模型输入产生；序列化异常仍未在本文件处理。
- 顺序与并发：一次 execute 内严格为 ask → update → 构造结果，未创建并行 Fiber、锁、超时或重试。不同调用之间没有本文件级互斥；若上层并发调用同一会话，最终可见列表及事件顺序取决于 Todo.Service 与其数据库/事件实现。权限成功不等于更新已提交，更新失败不会返回成功结果。

## 状态、取消、恢复与副作用

本工具没有自己的状态机、缓存或恢复标记；每次调用携带一份完整列表。Tool.Context 类型虽通常包含 abort，本文件的 execute 没有读取它，也没有显式 Effect.timeout、Effect.retry、取消回调或重放逻辑。Effect 运行时若在 ask 前取消，通常不会产生更新；若 ask 已完成而 update 被取消，是否出现可见中间态由服务实现保证，源文件本身不声明补偿。

唯一明确的外部副作用是 L31-L34 的 todo.update。对应的 packages/opencode/src/session/todo.ts 实现（跨文件核对）会按 sessionID 删除旧行、按数组位置插入新行，并在事务后发布 todo.updated 事件；因此空数组表示清空列表，位置由输入顺序决定。源文件没有重试、幂等键、持久化确认回执或网络调用；恢复时只能由会话服务重新读取当前列表，不能从本工具推导历史版本。

权限请求是另一个外部边界：ctx.ask 以 todowrite 和通配模式请求授权，实际允许/拒绝、等待期间取消及审计记录由调用方权限系统负责。工具没有在更新前后主动写 session journal，也没有对并发调用声明顺序保证。

## 源内测试与行为判据

源文件及同目录 src/tool 未包含针对 todo.ts 的专门测试，故源内未包含测试。可独立验证的判据如下：

1. 用 Parameters 解码缺少 todos 或数组元素缺少 content/status/priority 的输入必须失败；三个字段均为字符串时应成功。项目已有 test/session/schema-decoding.test.ts:L255-L262 对 Todo.Info 三字段 round-trip 提供相邻 schema 判据。
2. 构造允许 ctx.ask 且 todo.update 成功的假服务：输入 2 个元素（1 个 completed、1 个 pending）时结果标题必须为 "1 todos"，输出必须是两空格缩进 JSON，metadata 与输入等价。
3. 输入空数组时必须仍先请求 todowrite 权限，再调用一次 update，并返回 "0 todos"；服务应观察到会话列表被清空。
4. 让 ctx.ask 拒绝或失败时，update 调用次数必须为 0；让 update 失败时，execute 不得伪造成功结果。
5. 用未知状态字符串（例如 "blocked"）验证计数：它不等于 "completed"，必须计入标题数字。并发调用的最终顺序应由服务层测试，而不能从本文件推断。

## zenpi Rust 映射

- src/tools.rs（核心落点）：新增 TodoItem { content: String, status: String, priority: String } 与 TodoWriteArgs { todos: Vec<TodoItem> }，为 ToolRegistry 注册 todowrite。输入 schema 要求对象和 todos 数组，保持源实现“不在工具层限制长度/状态枚举”的差异。工具执行先走现有 ToolContext/approval gate，再调用会话服务；成功返回 ToolResult::Success，output 用 serde JSON 两空格格式，metadata 携带列表，title 复现未完成计数。副作用分类应明确选择现有 ToolSideEffect::WorkspaceWrite，或新增专用 SessionWrite；不能误标为 ReadOnly。
- src/session.rs（持久化与事件）：增加按 session_id 替换 todo 列表的接口（建议 replace_todos），在同一持久化边界完成删除旧列表、写入带 position 的新列表，并追加可重放的 todo.updated 事件。现有 SessionStore::append_event（L228-L239）可作为审计通道，但要补充读取当前 todos 的接口，保证恢复和查询可验证。
- src/core.rs（注册和会话归属）：在 set_tools/工具运行时装配链（L1384-L1425）注册该工具，执行时从 ToolContext 取得并校验当前 session_id，再把工具调用/结果纳入既有 AgentEvent::ToolCall/ToolResult。若 agent 非 Idle 或会话已切换，应由现有 admission gate 拒绝，避免写入旧会话。
- src/tool_runtime.rs（批执行）：将 todowrite 标记为 whole-batch sequential/mutating，确保同一批及相邻只读调用不会并发重排列表。沿用 execute_tool_batch 的 cancellation predicate；源行为没有重试，因此不要为失败的 todo update 自动重放，且在取消时返回明确的取消错误。
- src/runtime.rs（取消边界）：把 CancellationToken 映射到工具执行前、权限等待中、update 前的检查；mark_completed 后的晚到取消不能把已提交结果改成 Cancelled。源文件没有 timeout，Rust 侧若增加超时必须记录为架构差异并验证事务回滚。
- src/protocol.rs（线协议）：优先复用现有 typed tool-call JSON；若要提供独立 todo 命令，加入带 session_id 的请求/结果类型并使用已有 identifier 校验。响应字段应能表达 title、pretty JSON output、todos metadata 和错误，不要把 provider 原始参数直接当作已授权写入。
- src/approval.rs（权限）：把工具名映射到 todowrite 权限决策，通配模式对应源里的 patterns:["*"] 与 always:["*"]。拒绝、取消、持久化失败应与现有 ApprovalError/事件审计衔接；不可因工具名自动推断允许。
- src/headless.rs（宿主/展示）：在 headless JSONL 和 TUI 事件投影中识别 todowrite 的 ToolCall/ToolResult，展示 title 和列表摘要；session owner 校验沿用现有 active session 逻辑。不要在 headless 层复制 update 事务，避免绕过 core 的 approval/cancellation。
- src/providers/**（provider 适配）：现有 anthropic、google、openai、codex、deepseek 等模块只负责协议路由、鉴权和模型响应，不应直接写 todo。它们只需接收注册后的工具 schema 并产生标准 tool call；参数解码、权限、会话副作用均留在 tools.rs/core。可验证差异是：同一 todowrite JSON 在各 provider 方言转换后仍保持数组顺序和字段值不变。

可执行验证集：Rust 单测覆盖 schema 缺字段、空数组、完成计数、权限拒绝零写入、update 失败无成功结果、同会话并发串行化、取消前后边界、session 重启后列表恢复；集成测覆盖每个 provider 产生相同 ToolCall。这些是建议的落点，源文件本身没有 Rust 类型或协议约束。

## 未决问题

1. 仅凭本文件无法确定 ctx.ask 是否监听 ctx.abort，以及权限请求的超时策略。
2. 无法从本文件确认 Todo.Service.update 的数据库隔离级别、事件发布失败后的处理；这些属于服务实现。
3. 无法确认 Tool.define 上层是否会截断 output、附加 tracing 或把 Effect defect 转成模型可见错误。
4. 无法确认同一 sessionID 的并发 todowrite 调用是否在调用方排队；文件内没有锁或顺序令牌。
5. 无法确认 provider 是否对工具参数做额外字段过滤；本文件只定义 Effect schema，不控制各 provider 的 wire 编码。


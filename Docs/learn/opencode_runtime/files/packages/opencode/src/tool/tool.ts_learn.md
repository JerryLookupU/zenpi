# OC-056 — packages/opencode/src/tool/tool.ts

- source_id/item_id：OC-056
- source_path：`packages/opencode/src/tool/tool.ts`
- source_hash：`fed2bc482a40aff4fe21a7939c753bc0f36110e5ec0e8dbba1410509d4deaf88`
- source_bytes：6130
- source_lines：183
- coverage：已从字节 0–6129、行 1–183 按顺序完整读取，包含全部注释、类型、导出符号。

## 完整行为复盘

- 导入与 `Metadata`（L1-L13）：依赖 `PermissionV1`、`SessionV1`、`Effect/Schema`、`JSONSchema7`、会话类型、`Truncate`、`Agent`。`Metadata` 是允许任意键值的索引接口，因而工具结果元数据可扩展但缺少静态字段约束。
- `DynamicDescription`（L15-L16）：导出类型，接收 `Agent.Info`，返回 `Effect.Effect<string>`；当前文件只声明类型，没有消费点。
- `InvalidArgumentsError`（L18-L34）：`Schema.TaggedErrorClass`，运行时 tag 为 `ToolInvalidArgumentsError`，字段 `tool`、`detail` 均为字符串。`message` 固定生成模型可读的纠错提示，要求重写输入以满足 schema（L31-L33）。它只在参数解码失败时构造，工具本体不会被调用。
- `Context`（L36-L46）：工具执行上下文包含 `sessionID`、`messageID`、`agent`、`AbortSignal abort`、可选 `callID`/`extra`、消息快照 `messages`。`metadata` 写入标题/元数据并返回 `Effect<void>`；`ask` 接受去掉 `id/sessionID/tool` 的 `PermissionV1.Request`，返回 `Effect<void>`，即工具通过上下文请求权限而非自行持有权限状态。
- `ExecuteResult`（L48-L53）：执行必须返回 `title`、泛型 `metadata`、文本 `output`；`attachments` 可选，为去掉身份字段的 `SessionV1.FilePart[]`。因此输出正文与附件是同一结果的两个通道。
- `Def`/`DefWithoutID`（L55-L69）：`Def` 描述一个已初始化工具：`id`、`description`、`parameters`（`Schema.Decoder`）、可选 `jsonSchema`，以及按解码后参数和 `Context` 执行的 `Effect<ExecuteResult<M>>`。可选 `formatValidationError` 将 schema 错误转为自定义字符串。`DefWithoutID` 用 `Omit` 移除 `id`，用于延迟初始化。
- `Info` 与 `Init`（L71-L82）：`Info` 保存稳定 `id` 与 `init(): Effect<DefWithoutID>`；`Init` 允许直接的 `DefWithoutID` 或返回它的 thunk。该联合决定初始化时是否还要执行一次 effect。
- `InferParameters`、`InferMetadata`、`InferDef`（L83-L97）：条件类型同时支持 `Info` 和 `Effect<Info,...>` 两种形态；分别提取 schema 解码后的参数、元数据类型和完整 `Def` 类型，其他输入得到 `never`。这是编译期映射，没有运行时副作用。
- `wrap`（L99-L149）：接收 `id`、`init`、`Truncate.Interface`、`Agent.Interface`，返回一个无参数 thunk（L99-L105）。内部 `Effect.gen` 在每次工具初始化时解析直接定义或执行 thunk，并复制成 `toolInfo`（L106-L107）。`Schema.decodeUnknownEffect(toolInfo.parameters)` 只创建一次闭包并复用，避免每次 LLM 调用重新闭包（L108-L112）。
  - 包装后的 `execute` 先构造 tracing 属性 `tool.name/session.id/message.id`，有 `callID` 才加入 `tool.call_id`（L113-L119），不记录原始参数。
  - 先 `decode(args)`；失败由 `Effect.mapError` 转换成 `InvalidArgumentsError`，detail 使用 `formatValidationError` 或 `String(error)`（L120-L129），所以解码失败是确定的模型反馈路径。
  - 解码成功后调用原始 `execute(decoded, ctx)`（L130）。若返回元数据已经存在 `truncated` 键（包括 `false`），直接返回，不重复截断（L131-L133）。否则按 `ctx.agent` 取 `Agent.Info`，调用 `truncate.output(result.output, {}, agent)`（L134-L135）；返回原结果并以截断内容替换 `output`，在 metadata 写入 `truncated`，截断时再写 `outputPath`（L136-L144）。
  - 整个 effect 使用 `Effect.orDie` 和 `Effect.withSpan("Tool.execute", { attributes })`（L145-L145）。因此内部可恢复错误被提升为 defect，span 覆盖解码、实际执行和截断；源码没有重试或并行调度。
  - 最后返回已替换 `execute` 的 `toolInfo`（L147-L148）。
- `define`（L151-L169）：泛型 `id` 保持字面量类型，输入是带环境 `R` 的 `Effect<Init, never, R>`。初始化时依次解析 `init`、`Truncate.Service`、`Agent.Service`，构造 `{id, init: wrap(...)}`（L160-L166）；返回 effect 的环境要求合并为 `R | Truncate.Service | Agent.Service`。`Object.assign` 额外把同步可读的 `.id` 附在 effect 对象上（L167-L168），方便注册表索引。定义阶段不执行工具。
- `init`（L171-L181）：接收 `Info`，执行其 `info.init()`，再把 `info.id` 合并回 `Def`（L174-L179）。它只负责把延迟初始化的无 ID 定义具体化，不重新包装 execute。
- `export * as Tool`（L183）：将本模块作为命名空间再次导出，便于以 `Tool.define`、`Tool.init` 等方式引用。

## 状态、取消、恢复与副作用

该文件自身没有持久化、超时计时器、重试计数器或并发池。每个 `wrap` thunk 初始化一次 `decode` 闭包；每次 `execute` 调用都创建新的 effect/span，工具定义对象在初始化后被原地替换其 `execute` 字段（L111-L114）。取消语义只通过 `Context.abort: AbortSignal` 向下传递，工具实现是否检查由实现决定；`Effect.orDie` 不会把取消变成此处定义的新结果类型。`ask`、`metadata`、工具原始 `execute`、`Truncate.output` 和 `Agent.get` 可能产生外部副作用，但本文件不落盘、不提交会话、不自动恢复。截断可能返回 `outputPath`（L141-L143），这是结果元数据中的外部产物引用。参数校验失败发生在原始执行和上述副作用之前；截断失败或原始执行失败会沿 effect 进入 `orDie` 缺陷路径。`result.metadata.truncated` 是幂等短路标记，可防止恢复/再次包装时重复截断。

## 源内测试与行为判据

源内未包含测试（`packages/opencode/src/tool` 未发现测试文件）。可独立验证的判据：使用一个必需字段的 `Schema.Decoder` 调用包装后的 `execute`，非法参数必须得到 tag 为 `ToolInvalidArgumentsError` 且 message 含工具名和 schema 修正提示；合法参数必须只调用一次原始 `execute`；返回 metadata 已有 `truncated` 时不得调用 `Truncate.output`；没有该键时输出应替换为 `truncate.output` 的 `content`，并按布尔值写入 `truncated`，仅在 true 时有 `outputPath`；`define` 的返回值应同时满足 effect 可执行和 `.id` 等于传入 ID。

## zenpi Rust 映射

- `src/tools.rs` 的 `ToolDefinition`、`ToolContext`、`ToolResult`、`ToolRegistry`（约 L505、L747、L1057 及 L1196-L1324）是最接近的运行时落点：将 `Def` 映射为含 `name/description/schema/execute` 的 trait 对象，将 `ExecuteResult` 映射为 `ToolResult { title, output, metadata, attachments }`，把 `Context.sessionID/messageID/callID` 映射到现有 `ToolContext` 的关联字段。
- `src/tool_runtime.rs` 的 `execute_tool_batch`（文件前部）已负责批量校验、顺序/并行模式、取消传播和结果按源顺序收集。建议新增 `ToolSpec`/`ToolInit` 或在 `ToolDefinition` 中加入一次性 JSON schema 编译结果；在 dispatch 前执行与 `Schema.decodeUnknownEffect` 等价的 `validate_call`，失败返回结构化 `InvalidArguments`，禁止进入 registry execute。
- `src/runtime.rs` 的 `CancellationToken`、`InputBoundaryGate`（L49-L100、L769-L910）可承接 `Context.abort`：建立 `AbortSignal` 等价的只读取消视图，在每个 provider chunk、重试前和工具边界检查；取消不应回滚已开始的副作用，符合现有 runtime 注释与 `JobOutcome::Cancelled` 语义。
- `src/approval.rs` 的 `ApprovalCoordinator::request/request_response`（L126-L220）对应 `Context.ask`。Rust 适配器应从 `ToolContext` 构造 `ApprovalRequest`，保留 `session_id/turn_id/call_id/tool` 关联，并在副作用前持久化决定；不要让 schema 解码错误绕过审批审计。
- `src/core.rs` 的工具调用准备、`ToolInvocationOutcome`、operation evidence 和 turn metadata（如工具调用处理区）对应 `Effect.orDie` 外层编排及 `ExecuteResult.metadata`。建议把 `truncated`、`output_path` 和附件写入 `Turn.metadata`/`SessionStore`，并在恢复时尊重已存在的 `truncated` 标记以保证幂等。
- `src/session.rs` 的 `Turn.metadata`、附件/操作 journal 与 `OperationRecovery` 是持久化落点；`tool.ts` 本身没有恢复算法，Rust 应由 session owner 记录 started/finished/unknown outcome，并在取消后保留未知副作用状态。
- `src/headless.rs` 与 `src/protocol.rs` 负责 JSONL 相关请求和错误投影。可将 `InvalidArguments` 序列化为带 `id/call_id/tool/detail` 的 correlated tool result，保持协议版本与请求 ID；不要把异常文本直接写 stdout 诊断通道之外。
- `src/providers/**`（尤其 `registry.rs`、`connection.rs` 及各 provider adapter）只应提供模型/流式事件和调用参数，不复制工具 schema 逻辑。provider 返回的 tool call 交给 `ToolRegistry` 解码，`Context.abort` 接到 provider stream 的取消；provider metadata 可并入 `ExecuteResult.metadata`，但截断与审批仍由 core/tool runtime 统一处理。

可执行差异清单：1）TypeScript 的 `Effect` 环境注入需在 Rust 中改为显式 `&ToolRegistry`、`&AgentCatalog`、`&Truncator` 参数或 trait；2）`Schema.Decoder`/`JSONSchema7` 双表示需统一验证入口并保留可选 schema；3）`orDie` 的 defect 语义需映射为可观测 `ToolError::Internal`，同时保留 `InvalidArguments` 的可恢复、模型可读错误；4）`Metadata` 的任意值需用 `serde_json::Map<String, Value>` 并限制大小；5）`Effect` 无内建并行，批量并行由 `tool_runtime.rs` 明确控制，且单个副作用失败后的停止规则要与现有 batch policy 对齐。

## 未决问题

- `Truncate.output` 的具体字节/行阈值、`outputPath` 的持久化位置不在本文件中。
- `Agent.Service.get` 对未知 `ctx.agent` 的错误类型与可恢复性需查 `agent/agent` 实现确认。
- `Effect.orDie` 在项目的统一运行时/日志层如何呈现 defect，单凭本文件无法确定。
- `PermissionV1.Request` 的剩余字段及 `Context.ask` 的审批超时由外部实现决定。

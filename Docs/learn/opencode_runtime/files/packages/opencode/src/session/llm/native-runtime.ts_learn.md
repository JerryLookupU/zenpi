# OC-016 — packages/opencode/src/session/llm/native-runtime.ts

- source_id: `packages/opencode/src/session/llm/native-runtime.ts`
- item_id: `OC-016`
- source_path: `packages/opencode/src/session/llm/native-runtime.ts`
- source_hash: `ad7ba806e0b9d429ccd93b02b3583754254e6fc540450cb97e16393c28d394c2`
- source_bytes: `8036`
- source_lines: `195`
- coverage: 已按源文件顺序读取完整字节范围 `0-8035`、完整行范围 `L1-L195`，包含导入、注释、类型、函数和导出符号。

## 完整行为复盘

### 文件边界与类型（L1-L44）

L1-L20 的导入表明本文件是 session 层到 `@opencode-ai/llm` 的适配器：`ProviderTransform` 负责消息和 provider options 降低，`LLMNative` 构造 `LLMRequest`，`LLMClientShape` 提供 native transport，`Effect`/`Stream`/`FiberSet`/`Queue` 负责资源、并发与事件流，`NativeTool`/`ToolRuntime` 负责工具定义和分发。`Auth`、`Provider`、AI SDK 的 `ModelMessage`/`Tool` 只作为输入边界类型。

L22-L24 导出 `RuntimeStatus` 是判定结果联合类型。`supported` 必须带 `apiKey`，可选 `baseURL`；`unsupported` 只带人类可读的 `reason`。L25-L27 导出 `StreamResult`：成功时带 `Stream.Stream<LLMEvent, unknown>`，失败时带原因。流元素错误类型声明为 `unknown`，所以 transport、provider 或工具失败可以在消费流时出现，而不是都转换成 `StreamResult`。

L29-L44 的 `StreamInput` 是完整请求输入：`model`/`provider`/`auth` 负责选择与认证，`llmClient` 是路由后的 native 客户端，`messages`、`tools`、`toolChoice`、`temperature`、`topP`、`topK`、`maxOutputTokens`、`providerOptions` 是模型参数，`headers` 是调用方覆盖头，`abort` 是工具执行取消信号。可选数值和 options 不在本文件补充数值默认值；未提供时继续传 `undefined` 或空对象。

### `status` 与 `statusWithFetch`（L46-L72）

导出 `status(input)`（L46-L48）只做纯门槛检查，先调用 `providerFetch(input)` 再调用 `statusWithFetch`；不会发 HTTP、不会修改 session。内部 `statusWithFetch`（L50-L72）按顺序执行：

1. L54-L56 检查 `model.providerID`。仅接受精确的 `"openai"`、`"anthropic"`，或以 `"opencode"` 开头的 provider；否则返回 `unsupported`，原因是 provider 不是 openai、opencode 或 anthropic。
2. L57-L59 检查 `model.api.npm`，仅接受 `@ai-sdk/openai`、`@ai-sdk/openai-compatible`、`@ai-sdk/anthropic`。provider ID 通过但 npm 不在白名单仍失败。
3. L60-L62 处理 OAuth：只要 `auth.type === "oauth"`，就要求 provider ID 精确为 `openai` 且 `fetch` 覆盖存在；否则返回 `OAuth auth requires a provider fetch override`。因此 Anthropic OAuth、opencode OAuth 和没有自定义 fetch 的 OpenAI OAuth 都不能走 native。
4. L64-L65 解析密钥：优先使用 `provider.options.apiKey` 的字符串值，否则使用 `provider.key`；不读取 `auth.key`，空字符串也因假值判断被视为未配置。没有密钥返回 `API key is not configured`。
5. L67-L71 返回 `supported`，`baseURL` 仅在 `provider.options.baseURL` 是字符串时保留，否则为 `undefined`。这里不校验 URL 格式，也不校验其他 provider options。

该门槛是逐条件短路的，最先失败的原因决定返回值；成功状态不携带 provider/package 的进一步能力信息。

### `stream`（L74-L146）

导出 `stream(input)`（L74-L146）先在 L75-L77 取得 OAuth fetch 覆盖并重跑同一门槛；若不支持，原样返回 `unsupported`，不会构造 request。支持时，L89 通过 `nativeTools(input.tools, input)` 将 AI SDK 工具转换为 native 工具表；L90-L102 调用 `LLMNative.request`：

- L91-L94 传入模型、门槛解析出的 `apiKey`/`baseURL`，并用 `ProviderTransform.message` 将 `ModelMessage[]` 按模型和 options 转成 native messages；缺省 options 使用 `{}`。
- L95-L99 原样透传 `toolChoice`、温度、`topP`、`topK`、最大输出 token，缺省值仍为 `undefined`。
- L100 将 `ProviderTransform.providerOptions` 的结果放入 native request。源码注释 L79-L88 明确要求两侧使用 OpenAI 官方 wire 字段名；若字段未来需要差异化，应在此边界集中翻译。
- L101-L102 将 `providerHeaders(input.provider.options.headers)` 与调用方 `input.headers` 合并，后者覆盖同名 provider 头。非字符串 provider 头会在 helper 中被过滤。

L103-L140 构造受 scope 管理的事件流。`Stream.scoped` 保证下游结束或失败时释放资源；`Stream.unwrap(Effect.gen(...))` 在建立流时创建 `FiberSet`（L106）和无界 `Queue<LLMEvent, Cause.Done>`（L107）。L108-L113 调用 `llmClient.stream`，并用 `LLMRequest.update` 把原 request 的工具与 `toDefinitions(tools)` 拼接；因此 native 工具既进入 provider 可见的 definitions，也保留本地执行闭包。

L114-L129 的 `Stream.flatMap` 是核心并发语义：非 `tool-call` 事件，或 `event.providerExecuted` 为真时（L116-L118），只发出原事件，不在本地再次执行。对未被 provider 执行的 tool call，先发出原始 `tool-call`（L118-L119），随后在 L120-L127 通过 `ToolRuntime.dispatch(tools, event)` 分发；分发结果的 `events` 被 `Queue.offerAll` 放入结果队列，错误 cause 通过 `Queue.failCause` 使队列失败。`FiberSet.run(..., { startImmediately: true })` 让每个本地工具调用立即进入 FiberSet；连续多个 tool call 可以重叠执行，而不是按 provider 事件串行等待。

L131-L135 在 provider 主流结束后追加等待：`FiberSet.awaitEmpty(settlements)` 等所有本地工具 fiber 结束，然后 `Queue.end(results)`。L137 将 provider 主流与 `Stream.fromQueue(results)` 串接，所以观察者先得到 provider 事件（包括 finish），再得到已入队的本地 tool-result 事件；队列结束才代表工具结算完成。L142-L145 返回门槛状态加事件流；有 OAuth fetch 时用 `Stream.provideService(FetchHttpClient.Fetch, fetch)` 注入 Effect HTTP 服务，没有时保持默认服务。`LLMNative.request` 或 schema 构造的同步异常不在此处捕获，会直接使调用 `stream` 抛出；provider 流本身的错误则作为流错误传播。

### 辅助函数与最终导出（L148-L195）

- `providerFetch`（L148-L153）仅当 provider ID 为 `openai` 且认证类型为 OAuth 时尝试读取 `provider.options.fetch`；非函数返回 `undefined`，函数以 `typeof globalThis.fetch` 断言返回。它不创建 wrapper，也不改变参数。
- `providerHeaders`（L155-L160）先用 `isRecord` 拒绝 null、数组及其他非 record；随后只保留值为字符串的键值对并以 `Object.fromEntries` 新建对象。非 record 返回 `undefined`，该值在对象展开时相当于不添加头。
- `nativeSchema`（L162-L167）把未知 schema 转为 `JsonSchema`：非 truthy 或非 object 使用 `{ type: "object", properties: {} }`；对象含有 object 类型的 `jsonSchema` 时直接复用；否则调用 `asSchema(...).jsonSchema`。转换异常不捕获。
- `nativeTools`（L169-L192）逐项映射 `Record<string, Tool>`，返回名称到 `NativeTool.make` 的对象。描述缺省为 `""`（L175-L177），输入 schema 走 `nativeSchema`。`execute`（L178-L189）用 `Effect.tryPromise` 包裹：没有 `item.execute` 时抛出带工具名的错误（L181）；有 handler 时传入原始 args，以及 `toolCallId: ctx?.id ?? name`、原始 `messages`、原始 `abort` signal（L182-L186）。同步抛错和 Promise rejection 都由 `catch` 转成 `ToolFailure({ message: errorMessage(error), error })`，不会静默跳过。
- L195 `export * as LLMNativeRuntime from "./native-runtime"` 将本文件全部导出为命名空间，供 session LLM 选择器调用。

## 状态、取消、恢复与副作用

本文件自身没有持久化状态、重试循环、超时计时器、文件写入或数据库副作用。`status` 是纯检查；`stream` 的外部副作用只有调用 `llmClient.stream` 触发 provider transport，以及 `ToolRuntime.dispatch` 触发本地工具。provider headers、API key、baseURL 只是 request 配置。

取消方面，`Stream.scoped` 的 Effect scope 取消会清理受 scope 管理的 stream/fiber；本文件没有显式 `AbortController.abort()`，也没有把 `input.abort` 传给 `LLMNative.request`。`input.abort` 只沿 `nativeTools` 传给每个 opencode `Tool.execute`，因此工具 handler 必须自行遵守 signal。`FiberSet.awaitEmpty` 在正常结束时等待所有工具 fiber；若 scope 被取消，具体中断行为由 Effect 和 `ToolRuntime.dispatch` 定义，源内没有“取消即回滚”的语义。

没有显式 timeout 或 retry。provider 重试、HTTP deadline 和 OAuth 刷新若存在，应属于 `LLMClient`/`FetchHttpClient`/上层 session；工具失败只进入 `ToolFailure` 和队列 failure，不会自动重试。无界 `Queue` 没有本文件级别的背压上限；长时间不消费结果可能增加内存。`providerExecuted` 是唯一的重复执行防线：native provider 已执行的工具调用不会交给 opencode 本地再次执行。

恢复方面，流消费失败后本文件不保存 cursor、request 或工具结果；恢复/重放必须由上层依据统一 `LLMEvent` 或 session journal 实现。OAuth 自定义 fetch 只在当前请求通过 service 注入，未持久化。

## 源内测试与行为判据

对应测试文件 `packages/opencode/test/session/llm-native.test.ts` 有可核对判据：

- L390-L456 验证 OpenAI、opencode、`@ai-sdk/openai-compatible` 的支持矩阵，Google provider、无 fetch 的 OAuth、不支持的 npm 包和缺少 key 的原因字符串。
- L458-L476 验证 Anthropic API-key 路径；L478-L503 验证 `options.apiKey` 优先于 `provider.key`，并回退到 provider key。
- L506-L524 验证 handler 抛出时得到 `ToolFailure` 且消息为 `boom`；L527-L541 验证缺失 `execute` 也得到带工具名的 `ToolFailure`。
- L543-L600 用两个被 gate 阻塞的 lookup 验证两个本地工具会并发启动；观察顺序是两个 `tool-call`、`finish`，释放 gate 后才追加两个 `tool-result`。
- L713-L760 验证 OpenAI OAuth 的自定义 fetch 被调用一次、请求为 Responses 路由且事件包含 text delta 与 finish。

若在 Rust 中独立验证，至少应断言：门槛条件按上述短路顺序返回稳定 reason；两个未 `providerExecuted` 的 tool call 在 provider finish 前可同时启动；provider 事件先于本地结果，所有本地 fiber 结束后流才结束；同步/异步工具错误都变成结构化失败；自定义 fetch 只对 OpenAI OAuth 生效。

## zenpi Rust 映射

- **请求与 provider 路由：** 建议在 `src/backend.rs` 的 `CompletionRequest`、`RequestControl`、`ProviderEvent`（当前定义见 `CompletionRequest` L174-L188、`ProviderEvent` L238-L276、取消/截止检查 L390-L415）上增加一个可选 native-stream 适配层；`Backend::complete_with_request_control` L495-L507 已有逐事件 sink，最接近 `Stream.Stream<LLMEvent>`。差异是 zenpi 当前 `Backend` 是同步 trait，返回 `Completion` 并通过 callback 发 `ProviderEvent`，没有 TypeScript 的可组合异步 Stream；可执行落点是用 callback 转有界 channel，再由 `runtime::BackgroundRunner` 暴露事件。
- **status 门槛：** 不要在 Rust 复制 npm 字符串白名单作为唯一能力判断。`src/providers/connection.rs` 的 `ValidatedRoute`（字段和 accessor L42-L110，能力与 streaming 校验 L288-L365）已经绑定 provider、URL、auth、`ProviderCapabilities` 和 route digest；`src/providers/registry.rs` 的 `ModelDescriptor::effective_capabilities` L70-L82 以及 `ModelRegistry::resolve` L366-L373 可实现“模型能力 ∩ wire 能力”的支持判定。建议新增 `NativeRuntimeStatus { api_key: SecretHandle/受控引用, base_url: Option<String> }` 和 `status_native(route, auth, fetch_override)`，明确 OAuth fetch override 的特例。与 TS 的差异：zenpi 还验证 destination、credential revision、streaming capability，并有 `BackendError::Configuration/Authentication`，不能只返回字符串 reason。
- **消息、headers、options：** TS 的 `ProviderTransform.message/providerOptions` 可落在现有 provider wire 编码器（`src/providers/openai.rs`、`anthropic.rs`、`google.rs`、`codex.rs`、`deepseek.rs`）和 `OpenAiWireApi`；`ValidatedRoute::protocol/dialect/options` 应作为单一翻译来源。`providerHeaders` 的字符串过滤可放在 route/header policy 层，避免把任意 JSON 头传入 transport。TS 允许调用方 headers 覆盖 provider headers；Rust 应在已验证的 `AuthHeaderPolicy` 之后应用非敏感 request headers，并保持审计字段不可覆盖。
- **工具桥：** `nativeTools` 对应 `src/tool_runtime.rs` 的 `execute_tool_batch`（入口 L168-L176，取消与 join 语义 L219-L331）和 `src/core.rs` 的 `ToolRuntime`/`AgentEvent::ToolCall`、`ToolProgress`、`ToolResult`（L283-L315）。建议新增 `NativeToolAdapter`：把 `ToolDefinition`/`ToolCall` 转为 native definition，handler 错误映射为 `ToolFailure`，将 `turn_id`、messages snapshot 和 cancellation predicate 放入 `ToolContext`。差异是 zenpi 的批处理限制最多 32 call、256 KiB，并区分 sequential/parallel；TS 使用无界 FiberSet/Queue，且单事件 dispatch 后允许重叠。
- **并发、取消与 shutdown：** `src/runtime.rs` 的 `CancellationToken` L56-L100、`BackgroundRunner::{try_cancel,try_shutdown,recv_timeout}` L309-L390 可承载 abort、关闭和事件读取；`InputBoundaryGate` L797-L910 可防止工具未结算时进入下一模型边界。实现时应把 provider callback 和工具结果都送入 bounded channel，保持“provider tool-call/finish 在先、tool-result 在后”，并在取消时让 `CancellationToken` 先置位，再 join cooperative workers。与 TS 的差异：zenpi 明确有 bounded queue、shutdown grace 和 `JobOutcome::Cancelled`，取消不回滚副作用；TS 本文件的 Queue 是 unbounded 且没有独立 shutdown grace。
- **approval 与外部副作用：** `src/approval.rs` 的 `ApprovalCoordinator`/`ApprovalPolicy`（请求、响应、持久化确认 L127-L220、L291-L446；策略 L466-L571）应在 `ToolRuntime` dispatch 前执行。TS native runtime 不做 approval；若直接映射会越权，因此 Rust 必须保留 zenpi 的 approval、policy digest、side-effect 分类和 durable ack，再调用 handler。
- **session 与恢复：** `src/session.rs` 的 `SessionStore::{begin_operation,finish_operation,mark_interrupted_operations,operation_recovery}`（约 L1410-L1527）以及 `append_event`/`events`（L1207-L1223）是 TS 缺少的持久化层。建议每个 native request 记录 operation intent、provider/tool event、terminal outcome；进程中断后由 operation recovery 标记“side effect unknown”，只允许显式 retry。TS 的 Stream scope 取消不能推导回滚，正好对应 zenpi 的恢复判据。
- **headless/protocol：** `src/protocol.rs` 的 `StdioEvent`/`StdioResponse`、`encode_line` L886-L995 将统一事件转 JSONL；`src/headless.rs` 的 replay journal、bounded event/terminal cache（常量 L33-L57）负责 request-id 重放和断线恢复。建议把 `ProviderEvent`、tool call/result 和错误映射成 `StdioEvent`，并由 headless 层持久化终端事件；native adapter 不应直接写 stdout 或 session journal。
- **providers/**：`src/providers/mod.rs` 的 `Protocol`/`AuthHeaderPolicy` 将 OpenAI Responses、Chat Completions、Anthropic Messages 等 wire 路由类型化；`src/providers/registry.rs` 的 `ReasoningLevel`、`ModelDescriptor` 可替代 TS 的自由形状 `providerOptions`。`src/providers/connection.rs` 已拒绝不支持 streaming 的 route（L313-L315）。需要补齐的差异是原生 `LLMClient` 的 `providerExecuted` 标记和统一 `LLMEvent` 模型；zenpi 目前 provider 事件为 `ProviderEvent::ToolCallDelta/ToolCallDone`，需在 core 或 backend adapter 中明确“本地执行 vs provider 已执行”字段。

建议的可执行落点：新增 `src/native_runtime.rs`（`NativeRuntimeStatus`、`NativeToolAdapter`、事件 channel adapter），由 `backend.rs` 的受控请求接口调用；在 `core.rs` 只消费统一 `AgentEvent`，在 `tool_runtime.rs` 复用 approval/batch，不把 HTTP、headers 或持久化写入 `native_runtime.rs`。验证步骤是新增纯单元测试覆盖 provider/auth gate、工具错误、并发事件顺序和取消，再以 `headless` JSONL 回放检查 terminal/event 顺序。

## 未决问题

1. `LLMNative.request` 和 `LLMClient.stream` 对 `input.abort` 的具体 signal 传播不在本文件内，无法确认 provider HTTP 是否会因该 signal 中止。
2. `ToolRuntime.dispatch` 返回的 `dispatched.events` 完整事件集合及其对失败 cause 的内部处理在本文件未定义，只能确认会被批量放入 `results` queue。
3. `ProviderTransform.message/providerOptions` 的具体字段转换、`toDefinitions` 的 schema 序列化和 `LLMRequest.update` 是否去重工具定义均需查看相邻模块才能确认。
4. `Stream.scoped` 被下游取消时，正在运行的 provider request 和 tool fiber 的最终中断/清理顺序由 Effect 实现决定，源码未给出本文件级保证。

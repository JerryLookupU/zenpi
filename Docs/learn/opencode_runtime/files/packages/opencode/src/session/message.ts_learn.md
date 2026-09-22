# OC-020 — packages/opencode/src/session/message.ts

> source_id/item_id: OC-020
> source_path: packages/opencode/src/session/message.ts
> source_hash: 0ece6d145ad543b16f45116d27ff37ac4b875f878ea05f940bfe365f26a64fc3
> source_bytes: 5041
> source_lines: 148
> coverage: 字节 1-5041；行 1-148（已按文件顺序完整读取，包含全部导入、注释、类型、导出符号）。

## 完整行为复盘

该文件没有运行时函数、类或状态机，核心行为是用 `effect` 的 `Schema` 声明消息协议，并导出对应的静态类型。调用方可把这些 schema 交给 Effect Schema 的解码/编码流程做运行时校验；本文件本身不执行 I/O 或转换。

- 导入与再导出（L1-L9）：`Schema` 来自 `effect`；`SessionID` 来自 `./schema`；`NonNegativeInt`、`ProviderV2`、`ModelV2` 来自 core；`MessageError` 及其两个错误类型来自 `./message-error`。L9 再导出 `AuthError`、`OutputLengthError`，所以使用者可从本模块取得同一错误类型。导入不产生副作用。
- `ToolCall`（L11-L18）：`Schema.Struct` 要求 `state` 精确为 `"call"`、`toolCallId`/`toolName` 为字符串、`args` 为 `Schema.Unknown`；`step` 是可缺省的 `NonNegativeInt`，存在时不能为负。没有长度、ID 唯一性或参数对象形状约束。`ToolCall` 类型由 schema 推导。
- `ToolPartialCall`（L20-L27）：结构与 `ToolCall` 相同，但 `state` 固定为 `"partial-call"`，表示参数仍可能未完成；缺省 `step` 与未知参数语义同上。
- `ToolResult`（L29-L37）：`state` 固定为 `"result"`，保留调用 ID、工具名和原始 `args`，并额外要求 `result` 为字符串。结果不是任意 JSON；结构化结果需先序列化成字符串。`step` 仍可缺省或为非负整数。
- `ToolInvocation`（L39-L43）：`Schema.Union([ToolCall, ToolPartialCall, ToolResult])` 用 `state` 作为 discriminator。解码时只能落入三个字面状态之一；同一调用可依次以 partial/call/result 形式出现，但该文件不检查跨消息顺序、ID 配对或重复。
- `TextPart`（L45-L49）：要求 `type: "text"` 与字符串 `text`；空字符串没有被排除。
- `ReasoningPart`（L51-L56）：要求 `type: "reasoning"`、字符串 `text`；`providerMetadata` 可缺省，存在时是字符串键到任意值的 `Schema.Record`。该字段用于保留提供商元数据，不限制具体提供商名称。
- `ToolInvocationPart`（L58-L62）：要求 `type: "tool-invocation"`，并嵌入前述 `ToolInvocation`；嵌套调用的 `state` 仍受 union 校验。
- `SourceUrlPart`（L64-L71）：要求 `type: "source-url"`、字符串 `sourceId` 与 `url`；`title`、`providerMetadata` 可缺省。URL 仅按字符串校验，本文件不做 URL 语法、协议或可达性检查。
- `FilePart`（L73-L79）：要求 `type: "file"`、字符串 `mediaType` 与 `url`；`filename` 可缺省。没有 MIME 白名单、文件大小、路径存在性或下载行为约束。
- `StepStartPart`（L81-L84）：只有字面量 `type: "step-start"`，无附加字段；它是消息 parts 中的步骤边界标记。
- `MessagePart`（L86-L94）：按 `type` discriminator 联合六种 part。`parts` 中每一项必须匹配一个已知字面量；未知类型、缺失必填字段或错误字段类型会使 schema 解码失败。未声明数组非空、顺序或重复限制。
- `Info`（L96-L146）：顶层要求字符串 `id`、`role`（仅 `"user"`/`"assistant"`）、`parts: Schema.Array(MessagePart)` 和完整 `metadata`。`parts` 可为空，数组项按上面的 union 校验。
  - `metadata.time`（L100-L105）要求非负 `created`，`completed` 可缺省但存在时也必须非负；`error` 可缺省，存在时必须符合 `MessageError.SharedSchema`，因此错误变体的实际边界由 `message-error.ts` 决定；`sessionID` 使用 `SessionID` schema。
  - `metadata.tool`（L107-L120）是字符串键到 `StructWithRest` 的记录。每个工具值必须有字符串 `title`，可选字符串 `snapshot`，以及 `time.start/end` 两个非负整数；rest schema 允许其他字符串键及任意值，因此可携带工具扩展元数据。这里没有要求 `end >= start`。
  - `metadata.assistant`（L121-L142）可缺省；存在时要求 `system` 字符串数组、`modelID: ModelV2.ID`、`providerID: ProviderV2.ID`、`path.cwd/root` 字符串、`cost` 有限数、可选布尔 `summary`，以及有限数 token 计数。`tokens.cache.read/write` 也必须是有限数。只检查 finite，不检查成本或 token 的非负性、加总关系或与 provider 一致性。
  - `metadata.snapshot`（L143-L145）可缺省字符串；整个 metadata 和 Info 分别标注 `MessageMetadata`、`Message`，用于 schema 标识、诊断与生成工具。
- `export type Info`（L146）：静态类型直接从 schema 推导，避免另写一份结构定义；运行时仍以 `Info` schema 为准。
- `export * as Message from "./message"`（L148）：把本模块命名空间作为 `Message` 导出，形成聚合访问入口；不新增数据或执行逻辑。该自引用式再导出依赖 TypeScript 模块解析，使用者可通过 `Message.Info`、`Message.TextPart` 等访问。

边界与默认值集中在 `Schema.optional`：缺省字段不会被自动补成空串、零或空数组；调用方必须区分缺省与显式值。`Schema.Unknown` 允许 `null`、数组、对象等任意值，而 `Schema.Finite` 排除 `NaN` 与正负无穷。所有 schema 声明为模块级不可变对象，多个并发解码调用之间没有共享可变消息状态。

## 状态、取消、恢复与副作用

本文件只描述数据形状，没有取消 token、超时、重试循环、并发调度、恢复游标或持久化调用。schema 解码失败是同步返回的校验错误路径；文件没有捕获、重试或把错误写入日志。声明本身不读写 session、provider、工具、网络或文件系统，也不会启动后台任务。`Info.metadata.time`、`snapshot`、`tool` 与 `assistant.tokens` 只是由上层写入的事实记录，不能据此推断本文件会自动维护完成状态。若上层在流式 provider 输出期间生成 `ToolPartialCall`，取消或断线后的重试语义必须由 `session`/`runtime`/provider owner 决定；本 schema 不会合并 partial、回滚副作用或保证幂等。并发下可安全共享 schema 定义，但单次解码结果是否持久化、是否接受 late result，完全由调用方控制。

## 源内测试与行为判据

源文件或同目录未包含测试文件（同目录清单没有 `*.test.*`/`*.spec.*`）。可独立验证的判据如下：

1. 用 Effect Schema 的 `decodeUnknown` 解码最小 `ToolCall`：`{state:"call", toolCallId:"c", toolName:"read", args:null}` 应成功；`state:"CALL"`、负 `step` 或缺 `toolName` 应失败。
2. 对 `ToolInvocation` 分别提交 `state:"partial-call"`、`"call"`、`"result"`，三者应按 discriminator 成功；`state:"other"` 应失败；`ToolResult.result` 为对象而非字符串应失败。
3. `MessagePart` 的六个 `type` 各用最小合法字段应成功；未知 `type`、缺 `sourceId`/`mediaType`/`toolInvocation` 应失败；`ReasoningPart.providerMetadata` 可省略且可携带任意 JSON。
4. 构造完整 `Info`，验证 role 只接受 `user`/`assistant`，`metadata.time.created` 与 token/cost 的 Infinity、NaN 被拒绝；省略可选的 `completed`、`assistant`、`snapshot` 仍可成功。额外验证 `metadata.tool.*` 的 `title` 和 `time.start/end` 是必填。
5. 对成功解码值执行 encode/decode 往返，字段值（特别是 discriminator、可选字段缺省状态和 Unknown 参数）应保持语义等价；验证多个并发调用不共享彼此的 `parts` 或 metadata。

## zenpi Rust 映射

建议新增 `src/message.rs`（由 `lib.rs` 或现有模块树公开），把该文件的协议形状与持久化/运行时分开；若必须沿用现有布局，则 `src/session.rs` 放持久化消息类型，`src/core.rs` 只消费其投影。建议落点如下：

- `ToolCall`/`ToolPartialCall`/`ToolResult`/`ToolInvocation` 对应 Rust `enum ToolInvocation` 的三个 `#[serde(tag = "state")]` 变体，`step: Option<u64>` 表达 `NonNegativeInt`，`args: serde_json::Value` 表达 `Schema.Unknown`，结果保留 `String`。现有 `src/tools.rs` 的 `ToolCall { id, name, arguments }` 与 `ToolResult::{Success,Error}` 可作为执行层对象，但字段名和“result 必须字符串”的 wire 契约不同，建议单独转换，不直接复用。
- `TextPart`、`ReasoningPart`、`ToolInvocationPart`、`SourceUrlPart`、`FilePart`、`StepStartPart` 对应 `#[serde(tag = "type")] enum MessagePart`。`ReasoningPart.provider_metadata` 与扩展字段用 `Option<BTreeMap<String, Value>>`；`SourceUrlPart`/`FilePart` 的 URL 仍是字符串。现有 `src/backend.rs::ProviderEvent::ReasoningDelta`、`ToolCallDelta`、`ToolCallDone` 可作为流式输入来源，`src/core.rs::AgentEvent::{ToolCall,ToolResult,Provider}` 可作为事件投影，但不能替代可恢复的 parts 数组。
- `Info` 建议为 `MessageInfo { id, role: MessageRole, parts, metadata }`；`MessageRole` 用 `serde(rename_all = ...)` 或显式 `user/assistant`，不要复用 `TurnRole` 的 `system/tool` 变体，除非做明确转换。`metadata.sessionID` 可关联 `src/session.rs::SessionHeader.session_id`；`metadata.time`、`snapshot`、`tool` 与 assistant token/cost 应写入 `SessionRecord.value` 的事件 JSON，以便恢复和审计。
- `assistant.modelID/providerID` 可映射 `src/backend.rs::CompletionRequest.model` 与 providers registry 的 provider/model 路由；`path.cwd/root` 应取 core/workspace 上下文。`cost` 与 token 计数可从 provider `Usage` 事件累计后落盘。`MessageError.SharedSchema` 应映射 `src/core.rs::AgentError`、`src/session.rs::OperationOutcome` 或一个专用可序列化 message error，而不是把错误只变成自由文本。
- 取消、重试和并发仍由现有 `src/runtime.rs::CancellationToken`/`JobOutcome`、`src/tool_runtime.rs::execute_tool_batch`、`src/approval.rs::ApprovalCoordinator`、`src/headless.rs` 的 admission/replay 机制实现。schema 层只在提交或恢复边界做校验；工具副作用前仍须走 approval 与 operation journal。

可执行差异清单：

1. TypeScript `Schema.Array`、`Schema.Record` 未设容量；Rust 端应在 `protocol.rs` 或 message validator 增加消息/parts/metadata 大小上限，避免 `serde_json::Value` 无界增长，并记录拒绝原因。
2. `NonNegativeInt` 在 Rust 用 `u64` 最接近，但若 JSON 数字先落 `Value`，需拒绝负数、浮点与溢出；`cost`/tokens 用 `f64` 后必须显式 `is_finite()`，不能仅依赖 serde。
3. TypeScript 的 `Schema.Unknown` 比现有 `tools::ToolCall.arguments` 的默认对象约束更宽；wire 解码应保留任意 JSON，执行层再按工具 schema 验证。
4. `StructWithRest` 允许工具元数据扩展键；Rust 不应对该内部对象使用 `deny_unknown_fields`，应采用扁平扩展 map；但顶层协议请求仍可沿用 `src/protocol.rs` 的严格字段策略。
5. 源 schema 不检查 ID 唯一性、URL 可达性、snapshot 内容、tool 时间单调性或 token 非负；若 zenpi 需要更强不变量，应在 `src/session.rs` 的恢复校验或 `src/core.rs` 的业务验证中新增并测试，不能声称它们来自本源文件。
6. 源 `ToolResult` 用字符串结果，而 zenpi 执行层结果是 `Value`/`ToolFailure`；转换时应保留结构化值的 JSON 文本及失败状态，确保 `call_id`/tool 名可关联，重试不得重复已审计副作用。

## 未决问题

1. `NonNegativeInt`、`SessionID`、`ModelV2.ID`、`ProviderV2.ID` 和 `MessageError.SharedSchema` 的精确约束定义在其他文件，本源只能确认其被引用，不能确认长度、格式或错误变体。
2. `export * as Message from "./message"` 在构建产物中的循环再导出形态、tree-shaking 结果及调用方实际使用方式需由 TypeScript 构建配置确认。
3. `Schema.StructWithRest` 对重复键、未知键的编码排序和解码细节由 Effect 版本实现决定，源文件未声明稳定顺序。

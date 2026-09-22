# OC-015 — packages/opencode/src/session/llm/native-request.ts

元信息：
- source_id/item_id：`OC-015`
- source_path：`packages/opencode/src/session/llm/native-request.ts`
- source_hash：`ee47e4430d7bb959f0ef672ca40ad9a45acd5ca4e0df72548e01825da3208432`
- source_bytes：`7953`
- source_lines：`196`
- coverage：已按顺序读取完整文件，字节范围 `[0,7953)`（即 `0-7952`），行范围 `L1-L196`，包含导入、注释、类型、私有函数和导出。

## 完整行为复盘

该文件是纯同步 lowering adapter：把 opencode/AI SDK 形状的输入转换为 `@opencode-ai/llm` 的 canonical `LLMRequest`，不发起网络请求，也不执行工具（边界职责由同目录 `native-runtime.ts` 承担，源文件注释和 `L183-L185` 明确了这一点）。

- `ToolInput`（`L16-L19`）是工具配置的最小形状，`description`、`inputSchema` 均可缺省且只读。导出的 `RequestInput`（`L21-L35`）要求 `model`、`messages`，其余均可选：API key/base URL、额外 system 文本、工具表、`toolChoice`（`auto|required|none`）、采样参数、最大输出 token、provider options 和 headers。
- `providerMetadata`（`L37-L43`）先用 `isRecord` 拒绝非对象；只保留值本身也是 record 的顶层键，空结果转为 `undefined`。因此数组、标量命名空间不会进入 native 元数据。`partProviderMetadata`（`L45-L48`）优先读取新字段 `providerMetadata`，缺失/无有效 record 时回退历史字段 `providerOptions`，保证 OpenAI/Anthropic 等续接数据可重放。
- `textPart`（`L50-L54`）固定输出 `type: "text"`；非字符串 `text` 降为空串，并附过滤后的 provider metadata。`mediaPart`（`L56-L65`）只接受字符串或 `Uint8Array` 的 `data`，否则立即抛错；`mediaType` 缺省为 `application/octet-stream`，非字符串 `filename` 变为 `undefined`，数据原样透传，不复制或持久化。
- `toolResult`（`L67-L78`）把非 record 的 `output` 包装为 `{type:"json",value:...}`；record 输出的 `type` 只识别 `text`（映射 `resultType:"text"`）和 `error-text`（映射 `"error"`），其他均为 `"json"`。`id/name` 非字符串时取空串；`result` 取 `output.value`（若存在）否则整个 output；布尔型 `providerExecuted` 才保留，最后带 provider metadata。
- `contentPart`（`L80-L100`）要求每个 part 是 record，否则抛“only supports object content parts”。按 `type` 分派：`text`、`file`、`reasoning`（字符串文本否则空串）、`tool-call`（空串 id/name 允许，`input` 原样透传）、`tool-result`；未知类型立即抛错。工具调用使用 `ToolCallPart.make`，结果使用上述 `toolResult`，因此构造失败会同步向上传播。
- `content`（`L102-L103`）将字符串消息包装为单个 text part；数组逐项经 `contentPart` 转换。`messages`（`L105-L118`）先把所有 `role:"system"` 的内容用 `SystemPart.make` 收集到 `system`，再为每个非 system 消息用 `Message.make` 保留原 role、转换 content，并且仅当 `message.providerOptions` 是 record 时写入 `native: {providerOptions: ...}`。系统消息不进入普通 `messages`，顺序分别保持原输入顺序；函数是纯 O(n) 转换，无共享可变状态和并发操作。
- `schema`（`L120-L124`）对非 record 返回空 object schema；若有 record 型 `jsonSchema` 则取该字段，否则直接把 record 当 schema。`tools`（`L126-L133`）对 `undefined` 视为空表，按 `Object.entries` 顺序构造 `ToolDefinition`；描述缺省空串，schema 经 `schema` 归一化。没有名称去重或额外校验，重复/非法值由下游 native 库处理。
- `generation`（`L135-L143`）把 `temperature/topP/topK/maxOutputTokens` 映射成 `temperature/topP/topK/maxTokens`；只有至少一个值非 `undefined` 时返回对象，否则返回 `undefined`。数值不在此处限幅或校验。
- `baseURL`（`L145-L146`）对 `RequestInput` 优先 `input.baseURL`，否则 `input.model.api.url`，空字符串转 `undefined`；对裸 `Provider.Model` 只读 `input.api.url`。`requireBaseURL`（`L148-L151`）在 URL 为空时抛出包含 `providerID/model.id` 的错误，仅被需要显式端点的 provider 分支调用。
- 导出 `model`（`L153-L179`）接受裸模型或 `RequestInput`，合并模型 headers 与调用 headers（调用值覆盖同名键；空合并结果为 `undefined`），设置可选 API key、baseURL，以及 `limits.context/output`。按 `model.api.npm` 路由：`@ai-sdk/openai`→`OpenAI.configure(...).responses`；`@ai-sdk/azure`→必须 base URL 的 `Azure...responses`；`@ai-sdk/anthropic`→`Anthropic...model`；`@ai-sdk/google`→`Google...model`；`@ai-sdk/amazon-bedrock`→`AmazonBedrock...model`；`@ai-sdk/openai-compatible`→必须 base URL、provider 为 `String(model.providerID)` 的 `OpenAICompatible...model`；`@openrouter/ai-sdk-provider`→`OpenRouter...model`。未知 npm 包在 `L178` 同步抛错。仅 `"model" in input` 且 `apiKey` 真值时才向 options 写 API key，裸模型调用不会凭空带凭据。
- 导出 `request`（`L181-L193`）先转换消息，再调用 `LLM.request`：显式 `system` 数组先映射 `SystemPart.make`，随后拼接消息中抽出的 system；传入转换后的普通消息、工具数组、原样 `toolChoice/providerOptions` 和 `generation`，模型通过 `model(input,input.headers)` 创建。函数不会执行 I/O、重试或工具副作用；任一 part、schema 或 provider 路由错误均在返回前同步抛出。文件末尾 `export * as LLMNative from "./native-request"`（`L196`）提供命名空间导出。

## 状态、取消、恢复与副作用

本文件没有状态机、`AbortSignal`、超时、重试、锁或异步 Promise；每次调用都从输入重新构造不可持久化的 request 对象。取消只能由调用它的 `native-runtime.ts`/`LLMClient` 处理，本适配器不会吞掉取消错误。没有 session journal 写入、token 计费或外部 HTTP；唯一可观察副作用是同步抛出输入/路由错误，以及把 `Uint8Array`/文件 data 和 provider continuation metadata 引用带入下游请求。`providerMetadata` 对 `providerOptions` 的兼容回退是“恢复”语义：它保留历史消息中的 provider-owned continuation state，但不会自行生成或更新状态。并发调用彼此独立；共享的 `model`/`messages` 只读使用，不做原地修改。

## 源内测试与行为判据

配套测试位于 `test/session/llm-native.test.ts`。`L157-L297` 验证 system 顺序、文本/文件/reasoning/tool-call/tool-result、headers、limits、generation、tool choice 和 metadata 映射；`L299-L333` 验证已持久化的 `providerMetadata` 保持原值。`L335-L379` 验证七类 provider package 的 route/base URL，`L381-L388` 验证未知 package 快速失败。更细的 OpenAI Responses 判据在 `L603-L618`（请求体字段与 stream）、`L620-L710`（无加密 reasoning 时省略、带加密状态时保留、空 reasoning 保留、可用 id 时生成 item reference），OAuth 自定义 fetch 的端到端判据在 `L713-L755`。可独立验证：构造 `ModelMessage` 覆盖每种 content part，调用 `LLMClient.prepare(LLMNative.request(input))`，断言 route、system、messages、tools、generation 和 body；再用未知 `npm`、缺失 Azure/compatible URL、非法 file data 断言对应错误。

## zenpi Rust 映射

建议新增纯函数模块 `src/native_request.rs`（或放入 `backend.rs` 的独立 `native` 子模块），定义 `NativeRequestInput`、`NativeMessage`/`NativeContentPart`、`NativeToolDefinition` 和 `lower_request(&NativeRequestInput) -> Result<NativeRequest, BackendError>`；测试应使用固定 JSON fixture 验证确定性。这一层只做 lowering，不能取得 `ApprovalCoordinator` 或执行工具。

- `RequestInput.model` 对应 `providers::registry::ModelDescriptor` 加 `providers::connection::ProviderConnection`；`model()` 的 npm 路由应落到 `providers::Protocol/EndpointOperation/resolve_connection`。现有 `src/providers/**` 已有 OpenAI Responses/Chat、Anthropic、Google、custom、Codex、DeepSeek 路由和认证策略，但没有源文件中的 Azure、Amazon Bedrock、OpenRouter 专用分支，需明确映射为 unsupported 或新增 definition，并为缺失 `base_url` 保持可验证错误。
- `messages()` 的 system 分离和结构化 parts 应落到 `src/backend.rs` 的 `CompletionRequest`/协议编码边界；当前 `src/core.rs` 在 `L4000-L4225` 以 `Turn { role, content: String }` 构造 `CompletionRequest`，缺少 reasoning/media/provider metadata。建议扩展 backend 内部 `MessagePart`（text/media/reasoning/tool-call/tool-result）并由各 `src/protocols/**` codec 编码，system 文本可暂时合并为 `instructions`。
- `tools()` 应从 `src/tools.rs::ToolDefinition`（含 `side_effect` 和 JSON `input_schema`）生成 provider-facing definitions；执行仍交给 `src/tool_runtime.rs`，该模块已经区分顺序/并行、审批前置、结果相关性和取消后的不确定副作用。不能把 native adapter 的工具定义当作授权。
- `src/core.rs` 的 `RequestControl`、`src/runtime.rs::CancellationToken` 和 backend 的 deadline/重试循环提供本文件没有的取消、超时、退避与 circuit breaker；lowering 只传递字段，不重复实现这些语义。`src/protocol.rs` 负责 JSONL `cancel` 等命令，`src/headless.rs` 负责有界事件/终端重放，均不是 request conversion 层。
- `src/session.rs` 是 append-only journal，`src/approval.rs` 持久化审批和撤销 epoch；它们应继续拥有恢复、审批、凭据/副作用审计。若要恢复 provider continuation metadata，应把受限 JSON metadata 放入 `Turn.metadata`/session record，再由 `native_request` 映射，禁止在 adapter 内写盘。

可执行差异清单：补齐结构化消息与 `providerMetadata` 的 Rust 类型；实现 file data 仅接受 `String`/`Vec<u8>` 且默认 MIME 的边界；实现工具 schema 的空 object 默认；为每种 Protocol 建立 provider package/URL 矩阵测试；验证 `headers` 合并覆盖规则和 generation 全空时为 `None`；增加未知 provider、缺 URL、非法 part 的 typed `BackendError` 测试。

## 未决问题

- `@opencode-ai/llm` 各 `configure/model` 实现如何进一步解释 `limits`、`providerMetadata` 以及 `toolChoice`，仅凭本文件无法确认，需查该包实现。
- `isRecord` 的精确定义来自 `@opencode-ai/tui/util/record`，本文件未展开；数组是否被排除需以该实现为准。

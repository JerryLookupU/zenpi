# OC-013 — packages/opencode/src/session/llm.ts

- source_id：OC-013
- item_id：OC-013
- source_path：packages/opencode/src/session/llm.ts
- source_hash：5f1dcfb734853e39760e4dd05470f0c7d8752fc5dcc64e118c065149602e402d
- source_bytes：15097
- source_lines：404
- coverage：已从字节 0–15096、行 1–404 按顺序完整读取，包含全部注释、类型、导出符号与文件末尾导出。

## 完整行为复盘

该模块把一次会话 LLM 请求封装成 Effect Layer，并把 native runtime 与 AI SDK 两种实现统一为 `Stream.Stream<LLMEvent, unknown>`。依赖由 `Auth`、`Config`、`Provider`、`Plugin`、`Permission`、`EventV2Bridge`、`LLMClientService`、`RuntimeFlags` 提供（L62-L84）。

- `OUTPUT_TOKEN_MAX`：直接别名 `ProviderTransform.OUTPUT_TOKEN_MAX`，供其他模块复用输出上限（L33-L33）。
- `StreamInput`：必填 `user/sessionID/model/agent/permission? /system/messages/tools`，可选 `parentSessionID/small/retries/toolChoice`；`tools` 是 AI SDK `Tool` 字典，`toolChoice` 仅允许 `auto|required|none`（L35-L48）。`StreamRequest` 在此基础上加入内部 `AbortSignal`（L50-L52）。
- `Interface`：只暴露 `stream(input)`，返回 Effect Stream 而非立即执行结果（L54-L56）。`Service` 是 Context 服务标签 `@opencode/LLM`，`use` 是其 `serviceUse` 访问器（L58-L60）。
- `live/run`：`live` 用 `Layer.effect` 构造服务；内部 `run(input: StreamRequest)` 是带追踪名 `LLM.run` 的 Effect 函数（L62-L85）。
  1. 先记录 provider/model、session、small、agent、mode；日志本身不改变请求（L85-L93）。
  2. 通过 `Effect.all(...,{concurrency:"unbounded"})` 并行获取语言模型、配置、provider 信息和认证信息；任一 Effect 失败都会让本次请求失败，其他并发获取不再作为成功结果使用（L95-L103）。
  3. 判断 `language instanceof GitLabWorkflowLanguageModel`，调用 `LLMRequestPrep.prepare` 合并系统提示、消息、插件变换后的参数、headers 和经权限过滤/排序的工具（L105-L113）。准备阶段的失败直接传播。
  4. 建立 `EffectBridge`（L115-L118）。对于 workflow 模型，写入 session/systemPrompt，并安装 `toolExecutor`：未知工具或缺少 `execute` 返回错误；参数用 `JSON.parse`，执行时传入 `toolCallId`、原始消息和 `input.abort`；字符串结果直接作为 result，结构化结果优先取 `output`，再序列化，并保留 metadata/title；解析或执行异常转成 `{result:"",error}`，不会抛出到 workflow 调用方（L119-L147）。
  5. workflow 权限把 agent 与输入 ruleset 合并；按最后匹配的 `Wildcard` 规则，将非 `ask` 工具预批准（L149-L153）。每个 session 另有 `approvedToolsForSession` 集合；`approvalHandler` 去重工具名，若全已批准立即成功，避免服务器 MCP 无限询问（L155-L163）。否则生成递增 `PermissionV1.ID`，监听匹配的 `Permission.Event.Replied`，把参数标题整理成去重 patterns，通过 `perm.ask` 发起 `workflow_tool_approval`，成功后把工具加入 session 预批准集合；任何异常返回 `approved:false`，`finally` 解除订阅（L164-L205）。监听器只按 requestID 过滤事件并触碰 reply；实际等待由 bridge/permission 协议完成。
  6. 若配置开启 OpenTelemetry，则读取可选 tracer，并用 Proxy 包装 `startSpan`，每个 span 自动写入 `session.id`；未配置 tracer 时传 undefined（L208-L221）。
  7. 当 `flags.experimentalNativeLlm` 为真，调用 `LLMNativeRuntime.stream`，传入模型/provider/auth、`llmClient`、准备后的消息和工具、toolChoice、temperature/topP/topK/maxOutputTokens、providerOptions、headers、abort（L224-L242）。返回 `supported` 时记录 native 选择并返回 `{type:"native",stream}`（L243-L253）；`unsupported` 时记录原因并继续 AI SDK fallback，同时额外写 fallback 日志（L254-L269）。native 不支持不是错误。
  8. 默认/回退 AI SDK 路径记录选择日志并调用 `streamText`（L271-L280）。`onError` 通过 bridge 异步 fork 错误日志，不阻塞流（L280-L292）；GitHub Copilot provider 才开启 `includeRawChunks` 以读取计费字段（L294-L295）。`experimental_repairToolCall` 先尝试把工具名转小写并在准备工具中命中；否则把原工具名和错误消息编码成 input，并改成 `invalid` 工具（L296-L311）。请求使用准备参数、经 `ProviderTransform.providerOptions` 变换的 providerOptions、排除 `invalid` 的 activeTools、原工具字典、toolChoice、maxOutputTokens、abortSignal、headers、`maxRetries: input.retries ?? 0`、准备消息（L313-L325）。模型由 `wrapLanguageModel` 包装：仅 stream 类型参数会用 `ProviderTransform.message` 变换 prompt（L325-L343）。telemetry 设置开关、`functionId:"session.llm"`、tracer、username/session 元数据（L344-L353）。返回 `{type:"ai-sdk",result}`。
- `stream(input)`：用 `Stream.scoped/Stream.unwrap` 管理生命周期；获取资源时创建 `AbortController`，释放时必定 `abort()`（L357-L366）。调用 `run` 后 native 直接返回其统一事件流（L368-L369）；AI SDK 则创建 `LLMAISDK.adapterState()`，将 `fullStream` 转 AsyncIterable，错误值统一为 Error，再逐事件 `toLLMEvents`，把一个适配结果展平为事件流（L370-L379）。因此订阅取消会触发 scope release，向 provider 传播 abort。
- `hasToolCalls`：直接导出 `LLMRequestPrep.hasToolCalls`（L387-L387）；实现语义是仅检查消息 content 为数组且 part.type 为 `tool-call` 或 `tool-result`，字符串内容、空数组、纯文本均为 false（源内测试 L89-L174；委托实现 packages/opencode/src/session/llm/request.ts L216-L224）。
- `node`：用 `LayerNode.make` 注册 Service、live 层及上述依赖节点；应用通过此节点获得完整 LLM 服务（L389-L402）。
- `export * as LLM from "./llm"`：把本模块以命名空间再次导出（L404-L404）。

边界与默认值：`permission`、`parentSessionID`、`small`、`retries`、`toolChoice` 均可省略；small 默认 false 只用于日志和 preparation 分支；retries 默认 0，意味着 AI SDK 不自动重试。native 只由显式 flag 选择，且能力判断失败会透明回退。工具执行 JSON 无效、工具未知、权限拒绝均被转换为工具/审批结果；provider 准备、认证、stream 建立、适配器转换错误则终止 Stream。

## 状态、取消、恢复与副作用

本文件没有 session transcript 持久化、重试记录或恢复状态写入；其直接副作用是 provider 网络请求、工具执行、权限请求、日志、OpenTelemetry span，以及 workflow 模型对象字段的临时注入（L119-L205、L208-L221、L280-L353）。`AbortController` 是请求级状态，作用域结束或消费者取消时 abort；工具收到同一个 `AbortSignal`（L133-L137），AI SDK/native provider 也收到该信号（L227-L242、L321-L322）。超时没有独立计时器，需由上层取消 signal 或 runtime 取消作业。AI SDK 的 `maxRetries` 只由调用者的 `retries` 指定；native 路径的重试策略不在此文件。取消不是回滚：已经开始的工具或 provider 副作用只能合作式停止，是否产生未知结果由上层处理。workflow approval 以 session 内工具名集合去重并复用批准，订阅在成功、拒绝或异常后 finally 清理（L155-L205）。没有断点恢复逻辑；恢复、持久化和未知副作用判定应由 session/core 层承担。

## 源内测试与行为判据

源内包含同目录测试引用：`packages/opencode/test/session/llm.test.ts` 与 `llm-native-recorded.test.ts`。可核对的判据如下：

- `hasToolCalls`：空/纯文本/字符串 content 为 false；tool-call、tool-result、文本混合工具调用为 true（llm.test.ts L89-L174）。
- AI SDK stream 应发送 provider 认证、model、temperature、top_p、stream、max token、reasoning 选项，并对 opencode provider 发送 parent session header（L759-L892）。
- provider 返回 `network_error` finish reason 应表现为可重试的 stream failure；service stream 被 Effect fiber interrupt 后，响应 body 的取消/abort 应在 500ms 竞争窗口内发生（L905-L940、L1154-L1204）。
- native flag 关闭时即使注入会失败的 native client，也必须走 AI SDK；flag 开启时 OpenAI 走 `/responses`，并携带 reasoning/include/system input（L1432-L1545）。
- native tool call 要生成工具定义并执行 AI SDK `Tool.execute`，传递原始 args 与 toolCallId；录制回放应先产生 toolCall/toolResult，再在第二轮得到最终文本且无第二轮 toolCall（L1547-L1643、llm-native-recorded.test.ts L343-L396）。
- 可独立验证：构造等价 `StreamInput`，替换 provider/LLMClient/RuntimeFlags 层，断言事件类型顺序、请求 headers/body、abort listener、工具执行参数和 native unsupported fallback；不应把取消误判为成功文本或隐含重试。

## zenpi Rust 映射

现有 zenpi 已有可复用边界，但语义粒度不同：

- provider 抽象落在 `src/backend.rs`：`CompletionRequest`（约 L182-L234）承载 turns/model/tools/instructions/token limit，`ProviderEvent`（L236-L294）提供 TextDelta、ReasoningDelta、ToolCallDelta/Done、Usage、Completed、Failed；`Backend::complete_with_control`（L492-L531）提供同步完成与取消检查。这对应本文件的统一 `LLMEvent` 流，但当前是 callback sink + 最终 `Completion`，没有 native/AI SDK 双 runtime。
- 会话编排落在 `src/core.rs`：`run_active_turn_cancelable_with_events`（L3674-L3787）建立 provider operation marker、检查取消、调用 `complete_with_tools`；`complete_with_tools`（L3929 起）循环 provider 与工具调用，最多 8 次迭代并持久化操作结果。建议新增 `llm_runtime.rs) 或在 core 前增加 `LlmRequest`/ `LlmEventStream`，把准备、runtime 选择、事件规范化从 core 抽出。
- 工具执行映射到 `src/tool_runtime.rs` 的 `execute_tool_batch`（L168-L347）和 `dispatch`（L349-L436），已有批量上限、并行度、取消、panic 转错误、审批后 preview 校验；对应 workflow `toolExecutor` 时应复用 registry，而不是直接执行任意 JSON handler。建议类型为 `ToolExecutor) trait：输入 `ToolCall{id,name,args}`、`CancellationToken)，输出 `ToolResult)，并把未知工具映射为稳定错误。
- 取消/并发映射到 `src/runtime.rs`：`CancellationToken`（L49-L99）是可克隆、幂等、合作式 token；`BackgroundRunner`（L236-L450）有有界队列、FIFO follow-up、取消事件、shutdown grace、非合作任务 detach。建议 `StreamRequest.abort` 对应 token clone，provider 每个 chunk/重试前检查 `is_cancelled`；native 与 fallback 选择应在一个 job 内完成，并发 provider metadata 获取可用独立线程或顺序读取，不能引入无界任务。
- 权限与 workflow approval 映射到 `src/approval.rs`：`ApprovalRequest/Response`（L41-L125）、`ApprovalCoordinator::request_response`（L181-L245）、`persist_accepted/mark_persisted`（L370-L403）已支持等待、取消、先持久化后放行和 first decision wins。建议把 `workflow_tool_approval` 作为 ToolOrigin/permission kind，按 `name:title` 生成 patterns，并用 session-local `BTreeSet<String>) 实现已批准工具缓存；不要照搬 TypeScript 中忽略 reply 的 listener。
- 持久化/恢复映射到 `src/session.rs`：`begin_operation_with_key`（L1424-L1451）在 dispatch 前写 marker，`finish_operation`（L1453-L1474）写 terminal，`operation_recovery`（L1492-L1522）保留 unknown/interrupted，明确禁止自动重试。provider 请求建议 kind=Provider、tool kind=Tool、idempotency key 由 turn/session 生成；取消、transport/partial response 分别映射 Cancelled/UnknownOutcome。
- wire/UI 事件映射到 `src/protocol.rs` 与 `src/headless.rs`：`ProviderEvent) 可封装为 `StdioEvent)，`Command::Cancel)/`Shutdown`（protocol L214-L265、L415-L425）驱动 runtime token；headless 的异步 provider/agent mailbox（headless L806-L1065）已有有界事件缓冲和丢弃报告，可承接流式 LLMEvent。
- provider 路由映射到 `src/providers/**`：`providers::Protocol/Dialect/AuthHeaderPolicy` 及 registry 负责 OpenAI Responses/Chat、Anthropic、Google、Codex、DeepSeek 路由；新增 `NativeRuntime) 时应把能力判断（provider、协议、auth、API key）落在 registry/connection，避免在 core 按字符串硬编码。现有 backend 已有 `BackendError::is_retryable`（backend.rs L450-L472）和 provider retry/backoff（backend.rs 约 L1774-L1820），可对齐 `input.retries`，但要保留“请求已到达 provider 时 unknown outcome”规则。
- 建议落点与差异清单：
  1. 新建 `src/llm_runtime.rs`：`LlmInput)、`LlmEvent)、`RuntimeChoice::{Native,Fallback})、`prepare_request)、`stream`；由 core 调用，headless 只消费事件。
  2. 新建 `LlmPreparation` 保存系统提示、过滤排序工具、headers、temperature/top_p/top_k/max_output_tokens、tool_choice；对应 TypeScript 的 `LLMRequestPrep.prepare`，当前 Rust `CompletionRequest` 缺少 top-k/tool-choice/headers 字段。
  3. 为 `Backend` 增加可选流式接口（或新 trait），把 callback `ProviderEvent` 适配成 pull/iterator；必须定义 finish/error/abort 顺序。
  4. 为 native provider 增加 capability probe 与 fallback reason，测试 flag off 不调用 native、unsupported 自动 fallback。
  5. 将 tool-call repair（大小写归一、invalid sentinel）与 `ToolRuntime) 错误结构化；当前 Rust 没有等价 `invalid` tool sentinel。
  6. 把 telemetry session.id 注入 tracing span；当前列出的文件未见等价的 span Proxy。
  7. 明确 `maxRetries=0) 默认与 provider 层重试的优先级，避免双重重试；所有重试前检查 `CancellationToken)，并对 unknown outcome 要求 `SessionStore::decide_operation_recovery)。

## 未决问题

- `LLMRequestPrep.prepare`、`LLMNativeRuntime.stream`、`LLMAISDK.toLLMEvents` 的完整 provider 特殊字段和事件映射不在本文件内，需结合各自源码确认最终 wire schema。
- workflow `Permission.Event.Replied` listener 中 `void data.reply` 与 `perm.ask` 的确切等待契约无法仅由本文件确认。
- native runtime unsupported 的所有具体原因集合由 `native-runtime.ts` 决定，本文件只记录并 fallback。
- 上层如何把 `LLMEvent` 持久化成 MessageV2、如何判定网络错误可重试，由 `session/processor.ts` 等调用方决定，本文件没有定义。


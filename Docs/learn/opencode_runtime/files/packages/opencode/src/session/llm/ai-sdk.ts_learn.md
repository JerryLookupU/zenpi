# OC-014 — packages/opencode/src/session/llm/ai-sdk.ts

- `source_id/item_id`: `OC-014`
- `source_path`: `packages/opencode/src/session/llm/ai-sdk.ts`
- `source_hash`: `3d2e653811abaf230b3fea9a54049499d1be2c09dabe017ed4eb99918ee7def5`
- `source_bytes`: `9524`
- `source_lines`: `291`
- `coverage`: 已从第 1 字节至第 9524 字节、从 `L1-L291` 按顺序完整读取，含导入、类型、注释、私有函数、导出符号和文件末尾 re-export。

## 完整行为复盘

该文件是 AI SDK `Result["fullStream"]` 到 `@opencode-ai/llm` `LLMEvent` 的有状态适配器。`Result`（`L7`）取 `streamText` 的异步结果，`AISDKEvent`（`L8`）从 `fullStream` 推导事件联合类型；因此 `toLLMEvents` 的输入由 AI SDK 类型驱动，输出是 `Effect.Effect<ReadonlyArray<LLMEvent>, unknown>`（`L77-L80`），每个输入块可产生 0 或 1 个规范事件。

- `adapterState()`（`L10-L20`）创建每条流独享的可变状态：`step/text/reasoning` 计数器均从 0 起；当前文本、推理块 ID 初始为 `undefined`；`toolNames` 是调用 ID 到工具名的映射；`copilotTotalNanoAiu` 初始无值。状态不是线程安全对象，调用方须按流事件顺序消费。
- `finishReason(value)`（`L22-L24`）用 `Schema.is(FinishReason)` 白名单校验；不合法或缺失一律返回字面量 `"unknown"`，不会抛错。
- `providerMetadata(value)`（`L26-L29`）对 `null/undefined` 或不符合 `ProviderMetadata` schema 的值返回 `undefined`，只有 schema 通过才透传。它是元数据边界，不会尝试修复对象。
- `copilotTotalNanoAiu(value)`（`L31-L43`）是临时 Copilot 计费桥：只接受对象；从顶层 `copilot_usage` 或对象型 `response.copilot_usage` 读取 `total_nano_aiu`。该字段必须是有限、非负 `number`，否则返回 `undefined`。成功值稍后并入 step 元数据；注释明确未来应迁移到 `@opencode-ai/llm` 原生运行时。
- `usage(value)`（`L45-L65`）接受未知值；非对象返回 `undefined`。从 `inputTokens/outputTokens/totalTokens`、`outputTokenDetails.reasoningTokens`（回退 `reasoningTokens`）、`inputTokenDetails.cacheReadTokens`（回退 `cachedInputTokens`）、`inputTokenDetails.cacheWriteTokens` 提取字段，删除值为 `undefined` 的条目并返回普通对象；全缺失时返回 `undefined` 而非 `{}`。数值类型、负数、有限性没有再次校验。
- `currentTextID(state,id)`（`L67-L70`）和 `currentReasoningID(state,id)`（`L72-L75`）实现块 ID 复用：有显式 `id` 就覆盖当前 ID；否则复用当前 ID；若仍无值则产生 `text-${state.text++}` 或 `reasoning-${state.reasoning++}`。显式 ID 不推进计数器，事件乱序时可能切换当前块，故要求上游保持顺序。

`toLLMEvents` 的分支（`L77-L289`）如下：

- `start`（`L81-L84`）输出空数组；`start-step`（`L85-L87`）输出 `LLMEvent.stepStart({index: state.step})`，此时不递增。
- `finish-step`（`L88-L112`）若 `rawFinishReason === "network_error"` 立即 `Effect.fail(new ProviderError.ResponseStreamError(...))`，不生成完成事件。否则在 `Effect.sync` 中校验元数据，并在存在 `state.copilotTotalNanoAiu` 时浅合并 `copilot.totalNanoAiu`（保留原有 `original?.copilot` 字段）；随后清空该计费值，输出 `stepFinish`，索引使用当前 `state.step` 后自增，原因经 `finishReason` 归一化，usage 经 `usage` 转换。
- `finish`（`L114-L127`）输出单个 `LLMEvent.finish`，原因来自 `event.finishReason`，usage 来自 `event.totalUsage`；只有事件含 `providerMetadata` 属性时才读取并校验它。生成事件后用 `Object.assign(state, adapterState())` 全量重置计数器、当前块 ID、工具名和 Copilot 暂存值，因此同一状态可以安全复用于后续流。
- 文本块 `text-start`（`L129-L138`）保存/生成 ID 并输出 `textStart`；`text-delta`（`L140-L147`）输出当前或生成的 ID、原始 `event.text` 和元数据；`text-end`（`L149-L159`）先解析 ID，再清空 `currentTextID`，输出 `textEnd`。缺失 ID 的 delta/end 仍会生成稳定的 `text-0`（或下一计数）。
- 推理块 `reasoning-start`、`reasoning-delta`、`reasoning-end`（`L161-L191`）与文本三分支完全对称，分别产生 `reasoningStart/Delta/End`，使用 `reasoning-*` 自动 ID，并在 end 后清空当前推理 ID。
- 工具输入 `tool-input-start`（`L193-L203`）先记录 `event.id -> event.toolName`，输出 `toolInputStart`；`tool-input-delta`（`L205-L212`）输出相同 ID，工具名从映射取值、缺失回退 `"unknown"`，文本为 `event.delta ?? ""`，且不携带 provider metadata；`tool-input-end`（`L214-L221`）同样回退工具名并输出结束事件。
- `tool-call`（`L223-L235`）记录 `toolCallId -> toolName`，输出 `toolCall`，保留 `input`、可选的 `providerExecuted`（用 `"providerExecuted" in event` 判断）及合法 provider metadata。
- `tool-result`（`L237-L250`）从映射取名、缺失回退 `"unknown"`，随后删除映射，使用 `ToolResultValue.make(event.output)` 包装结果；保留 `providerExecuted` 和 metadata。删除意味着同一调用 ID 的后续结果无法继续获得原工具名。
- `tool-error`（`L252-L265`）优先使用映射名，其次是事件中存在的 `toolName`，再回退 `"unknown"`；删除映射；`message` 为 `errorMessage(event.error)`，同时原始 `error` 也放入事件，便于保留 cause。
- `error`（`L267-L268`）直接 `Effect.fail(event.error)`，不转换为 `LLMEvent`。`abort/source/file/tool-output-denied/tool-approval-request`（`L270-L275`）明确忽略并返回空数组；适配器不执行审批、文件或工具副作用。
- `raw`（`L277-L281`）只在 `Effect.sync` 中提取 Copilot 计费：成功值覆盖暂存值，提取失败则保留旧值，输出空数组。`default`（`L283-L287`）以 `never` 编译期穷尽检查兜底，运行时返回空数组。
- `export * as LLMAISDK from "./ai-sdk"`（`L291`）将本文件命名空间重新导出，调用方通过 `LLMAISDK.adapterState` 与 `LLMAISDK.toLLMEvents` 使用。

## 状态、取消、恢复与副作用

状态只存在调用方传入的 `adapterState`，事件转换按 `Effect.succeed`/`Effect.sync` 执行；并发消费同一 state 没有锁或合并规则，必须单线程、顺序化地处理一个 `fullStream`。`finish` 是唯一全量复位点；`text-end/reasoning-end` 只清理各自当前 ID，`tool-result/tool-error` 只清理对应工具名，`finish-step` 只清理 Copilot step 暂存值。流在 `error`、`network_error` 或外部取消处中断时若未到 `finish`，计数器、当前 ID、工具映射可能残留在该 state 中。

本文件没有取消 token、超时计时器、重试循环、持久化、网络发送、工具执行或审批决策；这些属于上层 session/runtime/provider。`network_error` 被转成 `ProviderError.ResponseStreamError`，由上层决定是否可重试；普通 `error` 原样失败。原始 chunk 只读取计费元数据，没有外部写入。Copilot 数值在每个 `finish-step` 附加后清空，防止跨 step 泄漏；`finish` 的全量 reset 防止跨 stream 泄漏。

## 源内测试与行为判据

源文件本身未包含测试；同一 session LLM 测试套件在 `/Users/wangweiyang/GitHub/opencode/packages/opencode/test/session/llm.test.ts` 覆盖：完整文本/推理/工具/usage/metadata 映射（`L188-L305`）、缺失 ID 自动生成 `text-0/reasoning-0`（`L308-L322`）、忽略不可见 chunk（`L324-L335`）、保留 `tool-error` 原始 cause（`L337-L357`）、全字段缺失时 usage 为 `undefined`（`L359-L387`）、`finish` 后状态复用并从 step 0 重新开始（`L389-L450`）、Anthropic cache metadata 保留（`L452-L505`）、Copilot raw 计费只附加到当前 step 且下一 step 清除（`L507-L555`）。网络错误的集成判据是收到 `ProviderError.ResponseStreamError` 且消息精确为 `Provider finish_reason: network_error`（`L905-L955`）。可独立验证时，应顺序运行这些事件并断言事件类型、ID、usage 字段、metadata 及失败类型；还应验证不调用工具、不产生持久化写入。

## zenpi Rust 映射

- `src/backend.rs` 的 `ProviderEvent`（`ProviderEvent::TextDelta/ReasoningDelta/TextDone/ToolCallDelta/ToolCallDone/Usage/Completed/Failed`）是最接近的共享事件层；`src/core.rs` 的 `AgentEvent::Provider` 将 provider 事件按 `turn_id` 关联并送入 session。建议新增 `src/llm_adapter.rs`（或 `src/ai_sdk_adapter.rs`）中的 `AdapterState`、`ProviderChunk` 和 `to_provider_events(state, chunk) -> Result<Vec<ProviderEvent>, AdapterError>`，明确实现文本/推理 ID、工具名映射、usage 归一化和 `network_error` 错误。
- `src/core.rs`（`run_active_turn_cancelable_with_events`）应成为事件消费边界：provider sink 顺序调用映射器，完成后再持久化 assistant turn；映射器只产生事件，不绕过 core 的 admission、工具循环或错误状态。现有 `AgentEvent` 没有 step/block start/end 全套语义，若需要 1:1 保真，应扩展 `ProviderEvent` 或在 `AgentEvent::Provider` 的 `event` 中增加 `StepStart/StepFinish/TextStart/TextEnd/ReasoningStart/ReasoningEnd/ToolInputStart/ToolInputDelta/ToolInputEnd/ToolResult/ToolError`。
- `src/session.rs` 负责 append-only journal、`OperationKind::Provider`、`OperationOutcome` 和中断恢复；它可记录适配器失败、完成和未知结果，但不应把 `adapterState` 当作持久化状态。`network_error` 的重试必须新建 operation ID，并遵守已有 interrupted/unknown outcome 的显式确认规则。
- `src/runtime.rs` 的 `CancellationToken` 是取消落点；provider/backend 已在流块之间轮询取消。Rust 映射器本身只需在每个 chunk 边界检查 token（或让 backend 返回 `BackendError::Cancelled`），不要尝试线程强杀；取消不是副作用回滚。
- `src/tool_runtime.rs` 执行真实工具批次、审批后 dispatch、结果持久化和取消协调。适配器中的 `tool-input-*`/`tool-call` 只能描述 provider 输出，不能直接执行；应把完整 `ToolCall` 交给 core/tool runtime，沿用批次顺序、审批与 unknown-outcome 规则。
- `src/protocol.rs` 定义 headless JSONL 的版本、相关 ID、取消、checkpoint/mailbox 等线协议；若要向客户端暴露细粒度 LLM 事件，应为 `ProviderEvent` 增加可序列化字段并由 protocol 投影，不能让适配器自行写 stdout。
- `src/headless.rs` 管理 bounded mailbox、重放、请求指纹、终端响应和 reconnect journal。它适合承载适配器产生的有序事件与 terminal error，但不应保存临时 block ID state；重连应依靠 session/replay 序列，而非重新猜测 `text-0`。
- `src/approval.rs` 是主机审批协调器；AI SDK 的 `tool-approval-request` 在本源适配器中被忽略，zenpi 则应继续让 host 显式决定 `ApprovalDecision`，适配器不能从 provider 请求推断 Allow。
- `src/providers/**`（`connection.rs`、`registry.rs` 及 `openai.rs/anthropic.rs/codex.rs/google.rs/deepseek.rs`）负责路由、能力和原生 streaming 解码；建议把各协议 decoder 输出统一到 `ProviderChunk`，再经上述 adapter 归一化。`Usage` 当前只有 `u64 input/output/total`，与源的 reasoning/cache read/cache write、`ProviderMetadata`、Copilot `totalNanoAiu` 不等价，应扩展可选 usage/metadata，或在 `Completion.annotations`/事件 metadata 中保存，并对负数/非有限外部数值采用拒绝或丢弃策略。

差异清单：zenpi 当前没有 AI SDK 的 `fullStream` 事件联合与 `Effect` 错误语义；没有细粒度 text/reasoning block ID 生命周期；工具事件是 `ToolCall`/`ToolCallDone` 而非输入增量、结果、错误三套事件；provider usage 缺少 cache/reasoning/Copilot 字段；`ProviderEvent::Failed` 是消息型事件而源的 `error` 是 effect 失败；zenpi 已有更严格的取消、审批、operation journal 和 headless replay，需要在映射层保持这些约束。

## 未决问题

无法从本文件确认 `ProviderMetadata`、`FinishReason`、`ToolResultValue.make` 的完整 schema，以及上层收到 `ProviderError.ResponseStreamError` 后具体重试次数、退避和 session 持久化时机；这些行为需查 `@opencode-ai/llm` 定义与 `src/session/llm.ts`/processor。除此之外，适配器分支、默认值、状态清理和忽略事件均可由本文件及上述测试直接确认。

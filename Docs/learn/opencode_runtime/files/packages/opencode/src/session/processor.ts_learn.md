# OC-022 — packages/opencode/src/session/processor.ts

- source_id/item_id: `OC-022`
- source_path: `packages/opencode/src/session/processor.ts`
- source_hash: `0b31e207beda56bd9a4b9b88c9e42d89a0651dd011773748c8e2938dfc3dcb74`
- source_bytes: `27266`
- source_lines: `732`
- coverage: 已从字节 `0` 至 `27265`、行 `L1-L732` 按顺序完整读取，包含全部注释、类型、导出与尾部自导出。

## 完整行为复盘

文件把一次 LLM provider stream 转成可持久化的 `SessionV1` message parts，并以 Effect 服务提供会话级工具、取消、重试和副作用编排。`DOOM_LOOP_THRESHOLD=3` 是连续相同工具调用的熔断阈值；`Result` 只有 `"compact" | "stop" | "continue"`（L29-L30）。`Handle` 暴露当前 `message`、`updateToolCall`、`completeToolCall`、`process`；前两个写入工具 part，`process` 消费 `LLM.StreamInput` 并返回结果分类（L32-L48）。`Input` 要求 assistant message、`sessionID`、`Provider.Model`；`Interface.create` 异步返回 `Handle`（L50-L58）。内部 `ToolCall` 用 `Deferred<void>` 表示工具完成，`ProcessorContext` 保存工具表、快照、阻断/压缩标志、当前文本和 reasoning 映射（L60-L75）。`StreamEvent` 直接等同 `LLMEvent`（L77）；`Service` 是 `@opencode/SessionProcessor` 的 Effect Context 服务（L79）。

`layer` 注入 `Session`、`Config`、`Snapshot`、`Agent`、`LLM`、`Permission`、`Plugin`、`SessionSummary`、`Scope`、`SessionStatus`、`Image`、`EventV2Bridge`、`Database`（L81-L97）。`create` 首先在流开始前 `snapshot.track()`，避免 AI SDK 先执行工具再发 `step-start` 导致快照过晚；随后创建全局 `ctx`，`aborted=false`（L98-L116）。`parse` 将任意异常交给 `MessageV2.fromError`，附带 provider ID 和 aborted 状态（L117-L121）。

- `settleToolCall(toolCallID)` 取出并删除工具记录，若存在则成功 `Deferred`；重复 settle 安全无效（L123-L127）。
- `readToolCall(toolCallID)` 查表后从 `Session.getPart` 重新读取 part；不存在或类型不是 `tool` 时删除陈旧索引并返回 `undefined`，因此后续更新不会写错 part（L129-L142）。
- `updateToolCall` 先读取，再将调用者变换后的 part 写回；若 part ID、message ID 或 session ID 变化，会同步索引；找不到调用返回 `undefined`（L144-L158）。
- `completeToolCall` 只接受仍为 `running` 的 part，写入 `completed` 状态、原始 input、输出、metadata、title、结束时间和附件，然后 settle；已结束/不存在调用直接返回（L160-L184）。
- `failToolCall` 只处理 `running` part，保留运行时 metadata，写入错误文本和结束时间；`PermissionV1.RejectedError` 或 `Question.RejectedError` 会按当前 `shouldBreak` 设置 `blocked`，最后 settle 并返回是否处理（L186-L205）。
- `finishReasoning` 仅处理当前 reasoning ID，刷新文本以触发响应式更新，写结束时间、持久化后从 map 删除（L207-L214）。
- `ensureToolCall` 先复用现存调用；provider 后续补充 `providerExecuted` 时只补 metadata。新调用创建 `pending` tool part，默认 `input={}`、`raw=""`，必要时带 `providerExecuted`，并分配 `PartID.ascending()` 与 `Deferred`（L216-L253）。
- `isFilePart` 是 `Schema.is(SessionV1.FilePart)` 类型守卫（L255）。`toolResultOutput` 将 provider 返回归一化：若 value 是带 string `output` 的记录，title 默认工具名、metadata 默认空记录、附件只保留合法 `FilePart`；否则字符串原样输出，JSON 值序列化失败时用空串，JSON 对象作为 metadata（L257-L276）。

`handleEvent` 按 `value.type` 串行处理事件（L278-L551）。`reasoning-start` 忽略重复 ID，创建空文本 reasoning part 并持久化；`reasoning-delta` 没有对应 start 时静默丢弃，否则累加文本并发出 `updatePartDelta`，provider metadata 可覆盖旧值；`reasoning-end` 先更新 metadata，再完成 reasoning（L280-L313）。`tool-input-start`/`tool-call` 在 summary assistant 上抛错，前者禁止摘要期间工具调用；input start/delta/end 都确保工具记录（L315-L329）。`tool-call` 将非 record input 包装为 `{value}`，把 pending 转成 running 并记录开始时间，保留 providerExecuted 标记；随后读取该 assistant 的最近三 parts，若三次都是同名、非 pending 且 input JSON 相等，则向 `Permission.ask` 发 `doom_loop` 请求，带工具名、input、`always` 和 agent ruleset（L331-L381）。

`tool-result` 对未知 error 调用直接丢弃；已知 error 走 `failToolCall`。成功结果先归一化图片附件：`image.normalize` 失败若为 `Image.ResizerUnavailableError` 则保留原附件，其他失败计入 omitted；被省略图片数会追加明确提示，然后完成工具调用（L383-L414）。`tool-error` 将显式 error 或 `new Error(message)` 记为失败；`provider-error` 直接抛错，使外层进入重试/停止路径（L416-L423）。`step-start` 懒取快照（仅当初始快照不存在），写 `step-start` part（L424-L433）。`step-finish` 取完成快照、强制结束全部 reasoning；记录 Anthropic 被丢弃 thinking blocks 的 warning。使用 `Session.getUsage` 计算 tokens/cost，更新 assistant finish、cost、tokens，写 `step-finish` part 和 message；对快照生成 patch 并写 patch part，清空快照；summary 在 scope 中 fork 且错误被忽略；非 summary 且 `isOverflow` 为真则设置 `needsCompaction`（L435-L497）。

`text-start` 创建空 text part；`text-delta` 没有当前 text 时静默丢弃，否则累加并发出 delta；`text-end` 触发 `plugin.trigger("experimental.text.complete")`，允许插件改写最终文本，补结束时间/metadata、持久化并清空当前 text（L500-L547）。`finish` 是无副作用终止事件（L548-L550）。

`cleanup` 是所有 process 结束时的确保动作：未提交快照生成 patch；未结束文本补 end time；所有未结束 reasoning 补 end time 并写回；对每个工具并发等待其 Deferred，单个最多 250ms；仍残留的工具改成 `error`、错误为 `Tool execution aborted`、metadata 加 `interrupted=true` 并写结束时间；清空工具表、设置 assistant 完成时间并更新 message（L553-L611）。`halt` 记录错误日志和 stack，再经 `parse` 归一化。context overflow 在 `compaction.auto=false` 且非 summary 时直接给 assistant 设置 error/finish、发布 `Session.Event.Error`、置 idle；否则只置 `needsCompaction` 并发布错误，让 process 返回 `compact`。其他错误写 assistant error、发布错误事件并置 idle（L613-L639）。

`process` 先记录日志，清零 `needsCompaction`，并按配置将 `shouldBreak` 设为“拒绝后是否停止”（默认配置不是 `continue_loop_on_deny` 即为 true）。内层设置当前文本/reasoning、状态 busy，调用一次 `llm.stream`；`Stream.tap(handleEvent)` 串行处理事件，`takeUntil(() => needsCompaction)` 在 step 完成后停止，`runDrain` 消费至结束（L641-L660）。中断时设置 `aborted`，若尚无 error 则以 `AbortError` 调 `halt`；仅非纯中断 Cause 才转成失败；套用 `SessionRetry.policy`，每次 retry 通过 `SessionStatus` 发布 attempt/message/action/next；最终 `catch(halt)`，无论如何 `ensuring(cleanup())`（L661-L691）。返回优先级是 `compact`，其次 `stop`（blocked 或 assistant error），否则 `continue`（L693-L696）。返回对象通过 getter 暴露最新 ctx message，并满足 `Handle`（L699-L706）。`node` 用 `LayerNode.make` 声明服务及全部依赖（L713-L730），末尾 `export * as SessionProcessor from "./processor"` 提供命名空间投影（L732）。

## 状态、取消、恢复与副作用

状态仅在一次 `create` 生命周期内保存在 `ProcessorContext`；`toolcalls` 是调用 ID 到持久化 part 的可变索引，`reasoningMap/currentText` 是流中间态。`Deferred` 只用于协调 cleanup 与外部工具完成，不是 durable recovery。取消来自 Effect fiber interrupt：中断会转成 `MessageAbortedError`（由 `MessageV2.fromError` 产生），但 cleanup 仍执行；工具最多等待 250ms，超时后标记中断，不能回滚已经发生的工具副作用（L553-L611、L661-L673）。代码没有独立 timeout 或自动重复取消；重试由 `SessionRetry.policy` 决定，且每次重试重新清空 reasoning map/currentText，避免跨尝试拼接（L646-L687）。

持久化副作用包括 `Session.updatePart/updatePartDelta/updateMessage`、`Snapshot.track/patch`、`SessionSummary.summarize` 后台 fork、`EventV2Bridge.publish`、`SessionStatus.set`、`Permission.ask`、图片规范化和插件 text hook。快照在 stream 前预捕获，在 step 结束或 cleanup 时 patch；摘要 fork 是并发的 advisory 副作用，失败忽略。tool output 的 attachments 可能因图像尺寸限制被剔除并在文本中留下计数提示（L391-L410）。context overflow 的恢复是返回 `compact` 给上层 compaction 流程，而不是本文件直接重试压缩（L621-L631、L693-L695）。

## 源内测试与行为判据

同目录存在 `test/session/processor-effect.test.ts` 和 `test/session/compaction.test.ts`，并非“源内未包含测试”。可核对判据包括：普通文本一次 provider call 返回 `continue` 且 text part 持久化（`processor-effect.test.ts:L240-L283`）；reasoning start/delta/end 生成独立 reasoning 文本，重试后不会把两次内容拼成 `onetwo`（L421-L515）；识别的 429、midstream server error、`network_error` 会各重试一次并发布 retry attempt，未知 JSON 错误不重试（L517-L605、L607-L763）；token 或结构化 context overflow 返回 `compact`（L374-L419、L765-L805）；工具成功状态含 input/output/title/metadata/start/end（L808-L870）；中断时残留工具变成 `Tool execution aborted` 且 `interrupted=true`（L873-L939），普通中断写入 `MessageAbortedError`、状态回到 idle 并发布错误（L941-L1013）；provider tool error 只产生工具 error part，不产生 `session.next.*`（L1072-L1118）；orphan `reasoning-delta` 必须静默丢弃（`compaction.test.ts:L1299-L1315`）。独立验证时应断言事件顺序、part 的最终状态、provider 调用次数、重试状态和 cleanup 后 session idle。

## zenpi Rust 映射

- `src/core.rs` 的 `Agent::process_with_cancel_and_events`、`run_active_turn_cancelable_with_events` 是 `process` 的主要落点：前者负责 admission，后者负责 provider operation marker、取消、工具循环和 `ProviderEvent` sink（`src/core.rs:L5310-L5373`、`L3674-L3801`）。建议新增 `ProcessorState`（assistant turn、当前 text/reasoning、tool map、needs_compaction）及 `process_provider_events`，保持 `Agent` 只负责生命周期提交。
- `src/backend.rs` 的 `ProviderEvent` 只有 `TextDelta`、`ReasoningDelta`、`ToolCallDelta/Done`、`Usage`、`Completed/Failed` 等（`src/backend.rs:L236-L276`），对应 `LLMEvent` 时需补一个可验证的归一化层：`reasoning-start/end`、`text-start/end`、`step-start/finish`、provider tool error、metadata 和附件必须明确映射，不能把 delta 当完整 part。`Backend::complete_with_control` 已提供取消检查和事件 sink（`src/backend.rs:L492-L531`）。
- `src/session.rs` 是 append-only JSONL durable owner，已有 `OperationKind::{Provider,Tool,Compaction}`、`OperationOutcome`、`InterruptedOperation`/`OperationRecovery`；可将 `SessionV1` part 事件编码为 `SessionRecord`/`append_event`，并在 operation marker 缺终态时沿用 unknown outcome，不自动重放副作用（`src/session.rs:L49-L95`、`src/session.rs:L314-L325`）。这比当前 TS 的内存 `Deferred` 更耐崩溃。
- `src/tool_runtime.rs` 已定义有界 tool batch、source-order prepare/persist 和不自动重试中断调用；映射 `ensureToolCall/completeToolCall/failToolCall` 时应让 `ToolBatchDecision` 产生 pending/running/completed/error 事件，并把 `Tool execution aborted` 转成 `OperationOutcome::Cancelled/Interrupted`。批量并行的边界必须与 TS 的单调用事件顺序分开测试。
- `src/approval.rs` 的 `ApprovalCoordinator::request/request_response` 与取消 epoch 可承载 `doom_loop`、权限拒绝和 `Question.RejectedError` 的阻断语义；新增 `permission="doom_loop"` 的 typed request，并在三次同名同 input 判定上复用 `src/core.rs` 的工具调用历史（`src/approval.rs:L122-L241`）。
- `src/runtime.rs` 的 `BackgroundRunner`、`CancellationToken`、`JobOutcome` 适合承载 Effect fiber 外层：队列有界、取消合作式、shutdown grace 后可 detach，但必须像 TS cleanup 一样在 job 终态前写中断工具结果；runtime detach 不能声称回滚副作用（`src/runtime.rs:L49-L99`、`L156-L193`、`L272-L374`）。
- `src/headless.rs` 与 `src/protocol.rs` 只负责 JSONL admission、replay、`TurnMode`、事件相关性和 stdout 背压，不应直接执行 `processor` 逻辑；将 `compact/stop/continue` 投影为终端 `StdioResponse`，并以 request/turn ID 关联 provider/tool 事件。headless 的 replay journal 与 `src/session.rs` durable journal 要保持两套 cursor 语义。
- `src/providers/**` 负责 protocol/dialect/endpoint/auth 路由；provider-specific metadata、Anthropic thinking transformation、usage/cost 计算应放在 provider 归一化或 `core` 的纯函数中，而不是散落到 headless。当前 Rust `Usage` 是 token 三元组，TS 还计算 cost 与模型 limit overflow，需补 `Cost/ContextBudget` 适配并为 overflow→compaction 写集成测试。

可执行差异清单：①建立统一 `ProviderEvent -> PartEvent` 枚举并为每个事件写序列断言；②为 `ProcessorState` 提供 snapshot/patch、文本 hook、图片附件过滤接口；③将 retry policy 的可重试分类和 status attempt 暴露到 `AgentEvent`；④在 session journal 中持久化 tool started/finished/error 与 provider terminal marker；⑤实现三次 doom-loop 检测和 approval gate；⑥验证 cancellation、cleanup、unknown outcome、compaction 四条路径都不会重复执行外部工具。

## 未决问题

无。

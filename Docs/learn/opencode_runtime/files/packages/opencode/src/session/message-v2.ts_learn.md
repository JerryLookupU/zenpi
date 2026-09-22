# OC-019 — packages/opencode/src/session/message-v2.ts

- source_id/item_id：`opencode:packages/opencode/src/session/message-v2.ts` / `OC-019`
- source_path：`packages/opencode/src/session/message-v2.ts`
- source_hash：`bfeb41e03e3788c83d3a031cce0aa1cf19a82c024a09e2a6aadbebb7a3d40d53`
- source_bytes：`25944`
- source_lines：`741`
- coverage：已从字节 `1-25944`、行 `L1-L741` 按源文件顺序完整读取，包含注释、类型、内部函数和全部导出符号。

## 完整行为复盘

### 依赖、类型和基础导出（L1-L78）

文件把 `SessionV1` 的 `Info`、`Part`、`WithParts`、`User`、`Assistant` 结构，与 `MessageTable`、`PartTable`、`SessionTable` 的 Drizzle 查询、Effect 运行时和 provider 错误类型连接起来（L1-L37）。`FetchDecompressionError`（L39-L44）只是描述 Bun `fetch()` 在 gzip/br 流中途解压失败的错误形状：必须有 `code: "ZlibError"`，并携带 `errno`、`path`。

`SYNTHETIC_ATTACHMENT_PROMPT` 是固定文本 `Attached media from tool result:`；同时重新导出 `isMedia`（L46-L47）。`truncateToolOutput(text, maxChars?)` 在 `maxChars` 未提供、为假值或文本不超限时原样返回；超限时保留前 `maxChars` 个 JavaScript 字符，追加换行和省略字符数诊断（L49-L53）。

`Event` 把 `SessionV1.Event` 中的 `MessageUpdated`、`MessageRemoved`、`PartUpdated`、`PartDelta`、`PartRemoved` 暴露为统一事件表（L55-L61）。`Cursor` 由 `Schema.Struct` 定义，字段为 `MessageID id` 和非负有限数值 `time`；`cursor.encode` 对 JSON 做 `base64url` 编码，`cursor.decode` 反向解码并同步校验 schema，JSON/编码/schema 任一失败都会抛出异常（L63-L78）。

### 数据行转换和分页辅助（L80-L129）

`info(row)` 将 `MessageTable` 行的 `data` 展开，并用数据库列 `id`、`session_id` 覆盖/补充为 `id`、`sessionID`，强制视作 `Info`（L80-L85）。`part(row)` 同样展开 `PartTable.data`，补上 `id`、`sessionID`、`messageID`，形成 `Part`（L87-L93）。`older(row)` 是严格的向前游标谓词：`time_created < row.time`，或时间相等且 `id < row.id`；因此时间相同由 ID 作为确定性次序（L95-L96）。

`hydrate(db, rows)` 先提取消息 ID。若为空则不查 parts；否则一次查询所有匹配 `PartTable.message_id`，按 `message_id`、`id` 升序排列，放入 `Map<string, Part[]>`，数据库失败经 `Effect.orDie` 转成 defect。最后每行返回 `{info: info(row), parts: ...}`，缺 parts 时使用空数组，消息行顺序保持输入顺序（L98-L123）。`providerMeta(metadata)` 去掉 `providerExecuted` 保留其他元数据；剩余为空时返回 `undefined`，没有 metadata 也返回 `undefined`（L125-L129）。

### `toModelMessagesEffect` 与 `toModelMessages`（L131-L427）

`toModelMessagesEffect(input, model, options?)` 是核心转换器，输入 `WithParts[]`、当前 `Provider.Model`，可选 `stripMedia` 和 `toolOutputMaxChars`；输出通过 AI SDK `convertToModelMessages` 转成 `ModelMessage[]`（L131-L135、L410-L418）。它在一次调用内维护 `result: UIMessage[]` 与工具名集合 `toolNames`，不修改输入对象。

`supportsMediaInToolResult(attachment)` 按 `model.api.npm` 和模型 ID 判断工具结果能否直接携带媒体（L147-L163）：Anthropic、OpenAI、Bedrock Mantle、Google Vertex Anthropic 全部允许；普通 Bedrock 仅允许 `image/*` 且模型 ID 含 `anthropic.`、`nova`、`llama4` 或 `llama-4`；xAI 仅允许图片；Google 仅允许 ID 含 `gemini-3` 且不含 `gemini-2`；其余 provider 均不允许。该能力判断只影响工具结果附件的去留。

内部 `toModelOutput({toolCallId,input,output})` 将工具输出适配 AI SDK（L165-L197）：字符串变 `{type:"text", value}`；对象被视为含 `text` 与可选 `attachments` 的结构，只接收 URL 同时以 `data:` 开头且含逗号的附件，输出 `content` 数组（可选文本项与 `media` 项，media 的 data 去掉 `data:` 前缀至逗号后内容）；其他值转 `{type:"json", value}`。`toolCallId`、`input` 当前只用于形状兼容，没有被再次加工。边界上，JavaScript 的 `typeof null === "object"`，所以 null 会在读取 `attachments` 时抛 `TypeError`；附件元素若缺少字符串 `url` 也会在 `startsWith` 处抛错，源码依赖上游 tool output 形状保证。

遍历每条消息时，空 `parts` 的消息直接跳过（L199-L201）。用户消息创建 `{id, role:"user", parts:[]}`（L202-L207）：非 ignored 且非空的 text 产生文本 part；`file` 中 `text/plain` 和 `application/x-directory` 被忽略，其余文件在 `stripMedia && isMedia(mime)` 时替换为 `[Attached mime: filename]` 文本，否则保留 `{type:"file", url, mediaType, filename}`；`compaction` 变成 `What did we do so far?`；`subtask` 变成 `The following tool was executed by the user`（L208-L244）。最终没有有效 parts 的用户消息不入结果（L245）。

助手消息先计算当前模型是否不同于消息记录的 `providerID/modelID`，并收集待注入的媒体（L248-L252）。若有错误，且不是“仅包含 step-start/reasoning 的 aborted”，整条助手消息跳过；这避免把失败重放为正常回答，同时允许已有可见输出的中止回答继续转换（L253-L260）。

助手 parts 处理细节如下（L261-L381）。普通 text 原样保留；有签名 reasoning 时，空文本改为单个空格，以维持 Anthropic adaptive thinking 的 signed block 分隔位置（L266-L289）。`step-start` 转同名结构。已完成 tool：工具名加入集合；若 `time.compacted` 则输出文本为 `[Old tool result content cleared]` 且丢弃附件，否则按 `toolOutputMaxChars` 截断并按 `stripMedia` 决定是否保留附件（L290-L300）。媒体附件先筛 `isMedia`，provider 不支持的媒体移入 `media`，支持的媒体留在工具输出；最终输出是纯文本或 `{text, attachments}`，并映射为动态 `tool-${part.tool}`、`output-available`、call ID、输入、输出；`providerExecuted` 单独透传，只有同模型时才透传去除该标记后的 `callProviderMetadata`（L302-L327）。

工具状态为 `error` 时，若 metadata.interrupted 为真且 output 是字符串，则作为 `output-available`（保留已产生的中断输出）；否则为 `output-error` 并使用 `errorText`（L329-L351）。`pending`/`running` 工具被强制补成 `output-error`，文本为 `[Tool execution was interrupted]`，以满足 Anthropic 每个 `tool_use` 必须有对应 `tool_result` 的约束（L353-L364）。reasoning 在模型变化时只保留非空 trimmed 文本为普通 text，避免把旧 provider metadata 带到新模型；同模型时保留 reasoning 与 metadata（L366-L380）。有任何助手 parts 才入结果；如果有不支持的媒体，则追加一个新 ID（`MessageID.ascending()`）的合成用户消息，首 part 为固定提示，之后是文件 parts（L382-L405）。

最终过滤掉只有 `step-start` 的消息，再调用 `convertToModelMessages`；tools 映射每个见过的工具名到同一个 `toModelOutput`，并以 `@ts-expect-error` 说明 AI SDK 实际只需要 `tools[name]?.toModelOutput`（L408-L418）。`toModelMessages` 是 Promise 外壳，通过 `Effect.runPromise` 执行同一 Effect；Effect 失败以 rejected Promise 暴露（L421-L427）。

### 数据库查询导出：`page`、`stream`、`parts`、`get`（L429-L523）

`page({sessionID, limit, before?})` 从 `Database.Service` 取得 db；若有 before，先 `cursor.decode`，按当前 session 且 `older(before)` 查询，否则只按 session。消息按创建时间、ID 降序取 `limit + 1`，数据库错误 `orDie`（L429-L446）。零行时再次查询 `SessionTable` 以区分“空会话”和“会话不存在”：不存在抛 `NotFoundError("Session not found: ...")`，存在返回空 items、`more:false`（L447-L459）。有行时用多取一行判断 `more`，裁掉哨兵行，`hydrate` 后反转为页内时间正序；有更多时以最后一行的时间和 ID 编码 cursor，否则 cursor 为 `undefined`（L461-L471）。负数、零或极大 limit 未在本函数显式限制，行为取决于 SQL/调用方。

`stream(sessionID)` 固定页大小 50，循环调用 `page`，把 `NotFoundError` 视为空流；每页从末项向前压入结果，因此总体返回“最新在前”的数组。空页、无更多或无 cursor 即停止；分页期间 DB 失败（除 NotFound）继续向调用方失败（L473-L494）。`parts(messageID)` 查询该消息全部 parts，按 part ID 升序并映射 `part`；查询错误 `orDie`（L496-L508）。`get({sessionID,messageID})` 同时按消息 ID 与 session ID 查单行，找不到抛 `NotFoundError("Message not found: ...")`，找到后返回 `info` 与 `parts(messageID)`（L510-L523）；因此跨会话 ID 不会泄露数据。

### 压缩过滤和最新状态（L525-L608）

`filterCompacted(msgs)` 接受任意 `Iterable<WithParts>`。它先按输入顺序（通常是 `stream` 的最新到最旧）累积结果，用 `completed` 记录已见到成功完成的 summary assistant 的 `parentID`；遇到已完成 user 时，如果有 compaction part，则没有 `tail_start_id` 就停止，有则记下要保留的 tail ID，并在达到该 ID 时停止；随后反转为时间方向（L525-L547）。反转后找到带 `tail_start_id` 的 compaction、其对应 summary assistant、以及 tail 消息；若 tail 在 compaction 前且 summary 在 compaction 后，则输出顺序重排为 `[compaction, summary, retained tail..., summary 后续]`，否则原反转结果（L548-L576）。源码中紧接的第二个“user+completed+compaction”判断（L543-L547）在前一个同条件分支已 `continue/break`，正常路径不可达，但应在 Rust 映射中保留语义或用测试证明等价。

`filterCompactedEffect(sessionID)` 只是 `stream` 后调用上述纯函数（L578-L580）。注释明确警告该重排使数组位置不再代表时间（L582-L585）。`latest(msgs)` 不依赖数组顺序：用 `isAfter` 按 `time.created`、再按 ID 选择最新 user、最新 assistant、最新已 finish assistant；只收集 finish 之后的 compaction/subtask 任务（L586-L602）。`isAfter` 对无 other 恒真，时间不同比较时间，时间相同比较字符串 ID（L604-L608），这也是导入消息 ID 不单调时的确定性 tie-breaker。

### `fromError`、命名空间与 Layer（L610-L741）

`fromError(e, {providerID, aborted?})` 将未知错误归一为 `Assistant["error"]`（L610-L613），按顺序匹配：DOM `AbortError`→`AbortedError`；已有 `OutputLengthError` 原样返回；`LoadAPIKeyError`→含 provider ID 的 `AuthError`；`ECONNRESET`→可重试 `APIError` 并记录 code/syscall/message；`ZlibError` 在上下文 aborted 时为 `AbortedError`，否则为可重试“Response decompression failed”（L614-L659）。`ProviderError.HeaderTimeoutError` 和 `ResponseStreamError` 都映射为可重试 `APIError`，分别记录 timeoutMs 或错误名（L660-L682）。`APICallError` 交给 `ProviderError.parseAPICallError`：context overflow→`ContextOverflowError`，其他→带 status、retryable、headers/body/metadata 的 `APIError`（L683-L708）。普通 `Error`→`NamedError.Unknown` 且使用 `errorMessage`；非 Error 先尝试 `parseStreamError`，同样区分 context overflow/APIError，解析失败或无结果时使用 JSON.stringify 的 Unknown（L709-L737）。每个错误都保留原始 cause；解析过程的异常被空 catch 吞掉后走 Unknown。

文件末尾把自身命名为 `MessageV2` namespace export，并定义 `node = LayerNode.group([Database.node])`，说明所有 Effect 数据库导出需要 Database layer（L740-L741）。

## 状态、取消、恢复与副作用

本文件本身是消息读模型、模型消息适配器和错误归一化层，没有 INSERT/UPDATE/DELETE；`page`、`parts`、`get` 只有数据库读取，`hydrate` 也是纯组装，`node` 仅声明依赖。因此持久化副作用来自外部 session projector，而不是本文件。读取过程中 `Effect.orDie` 把 SQL 失败提升为不可恢复 defect；`NotFoundError` 是有意保留的业务错误，`stream` 将“会话不存在”折叠为空流。

没有显式超时、重试循环或取消 token。取消语义通过错误值进入：`fromError` 识别 `AbortError`；压缩失败若调用方标记 `aborted` 也识别为 aborted。`toModelMessagesEffect` 只做同步遍历加一次异步 `convertToModelMessages` Promise，Effect 的中断能力由调用它的运行时决定，源码没有自己的 finally/补偿动作。

恢复/重放的关键是模型输入重建：compacted tool 输出被替换为固定清除文本，pending/running tool 生成错误结果，已中断 tool 若有字符串 output 则保留该输出；不同模型不携带 provider metadata；Anthropic 签名 reasoning 的空分隔改成空格。媒体不兼容时创建新的合成 user message，使用递增 ID，属于内存中的外部输入副作用但不会写数据库。分页对并发写入没有事务快照承诺：每次 `page` 单独查询，后续页之间新增/删除记录可能改变可见集合；时间+ID 游标只保证稳定的“早于该边界”谓词。

## 源内测试与行为判据

源内包含直接测试，不能写作“源内未包含测试”。主要判据如下：

- `packages/opencode/test/session/message-v2.test.ts` 覆盖 `toModelMessages` 的 user/assistant/tool、媒体、provider 差异、截断和空结果（例如 L133-L374、L561-L860、L988-L1358），并覆盖 `fromError` 分支（L1366-L1551）以及 `latest` 与压缩重排回归（L1628-L1724）。可独立验证：构造最小 `WithParts[]` 与各 provider model，断言输出 `ModelMessage[]` 的 role、parts、metadata 和错误类型。
- `packages/opencode/test/session/messages-pagination.test.ts` 的 `MessageV2.page`（L130-L307）验证向后分页、opaque base64url cursor、分数时间、同时间 ID tie-breaker、空/不存在 session、limit 边界、跨 session 隔离和 parts hydrate；`stream`（L309-L374）验证最新优先与空流；`filterCompacted`（L599-L965）验证 compaction 边界、缺 summary、错误/未 finish summary、tail_start_id、fork 和任意 Iterable；cursor 与一致性（L971-L1052）验证 encode/decode、page/get/parts/stream 一致。
- 可独立判据：分页全量迭代应与 `stream` 返回相同 ID 序列；`get` 的 `info/parts` 应等于 `page` hydrate；同一时间戳按 ID 严格分页且不重复；不支持的工具媒体应只出现在合成 user 消息；所有 pending/running tool 都必须生成 `output-error`；每个 `fromError` 输出的 `isRetryable`、provider ID、cause 与分支一致。

## zenpi Rust 映射

建议把该文件拆成可执行、可测试的 Rust 层，而不是把所有逻辑塞进一个 `Agent` 方法：

- `src/session.rs` 已有 append-only `SessionStore`、`SessionRecord`、`Turn`、`OperationRecovery` 和 `SessionStore::records/turns`。可新增 `MessageInfo`、`MessagePart`、`WithParts` 以及 `MessageStore::page/get/parts/stream`；用 `(created_at_ms, id)` 游标替代 TS `Cursor`，serde + URL-safe base64 实现 `cursor.encode/decode`。验证点是复现 page 的 `limit+1`、反转和跨 session 过滤；不要把读取操作改成重写 JSONL。
- `src/core.rs` 的 `Turn`/agent 状态适合承载 `latest` 所需 role、parent、finish、summary 元数据；新增纯函数 `filter_compacted`、`latest`、`is_after`，以 `TurnRole` 和 typed part enum 表达 `compaction`/`subtask`。必须保留 `tail_start_id` 重排规则，并为导入 ID 非单调增加相同时间的 ID tie-breaker 测试。
- `src/tool_runtime.rs` 已有 `ToolBatchDecision`、取消和失败结果；可在 `src/message_runtime.rs`（建议新模块）实现 `to_model_messages`，将 `ToolResult` 映射为 `ToolOutput::{Text,Content,Json}`，对 pending/running 统一产生 interrupted error，对 compacted 采用固定清除文本。该模块只返回模型请求值，不直接执行工具。
- `src/runtime.rs` 的 `CancellationToken`、`BackgroundRunner` 可包住模型转换/请求；`message_runtime` 不自行创建线程或重试，只检查取消并让上层决定中断。`src/session.rs` 的 `OperationOutcome::{Cancelled,Interrupted,UnknownOutcome}` 可承载恢复判据，与 TS 的 `fromError` aborted 语义对齐。
- `src/protocol.rs` 适合定义 wire 层的 `ModelMessage`、分页 cursor 和错误 code；沿用现有 `MAX_ID_BYTES`、`MAX_TEXT_BYTES` 做输入上限。HTTP/JSONL 只负责序列化 `NotFound`、cursor 无效和 `more`，不要把 provider media 能力判断散落在协议解析中。
- `src/approval.rs` 与 `src/tool_runtime.rs` 的 side-effect/approval gate 对应 TS 工具结果的 providerExecuted 与中断安全边界：工具执行前记录 approval，工具结果转换只读取已审计结果；Rust 不应因 provider 返回的工具名自动放行副作用。
- `src/providers/mod.rs`、`src/providers/connection.rs`、`src/providers/registry.rs` 已有 provider/model identity、wire protocol 与 `ProviderCapabilities`。在 `ProviderCapabilities` 中显式增加 `tool_result_images`、`tool_result_files` 或等价能力，并实现 Anthropic/OpenAI/Bedrock/xAI/Google 的矩阵；`to_model_messages` 依据能力把不支持的 media 提取到合成 user message。`src/providers/anthropic.rs`、`google.rs`、`codex.rs` 的协议定义应提供默认能力，未知 provider 默认 false 或按 registry 明确覆盖。
- `src/headless.rs` 负责 host/命令边界，可调用 session page/stream 和 message runtime；不要在 headless 层复制 `fromError` 分支。建议在 `src/error.rs` 或新 `src/message_error.rs` 定义 `AssistantError`（`Aborted`、`Auth`、`Api{retryable}`、`ContextOverflow`、`Unknown`），把 `ECONNRESET`、解压失败、header timeout、stream error、APICallError 的解析集中到一个 `from_error`。

差异清单：TS 使用 Effect/Drizzle/AI SDK，Rust 需用 `Result`、数据库或 JSONL 投影和自有 `ModelMessage` wire 类型；TS 的 `MessageTable/PartTable` 是规范化表，zenpi 当前主要是 append-only turns，需决定是否建立内存索引或新增 durable part record；TS 允许 provider metadata 任意 JSON，Rust 应用 `serde_json::Value` 并过滤 `providerExecuted`；TS `MessageID.ascending()` 是外部 ID 生成器，Rust 要提供单调、session-scoped 的合成 ID；TS 对 `limit` 没有本地边界，zenpi 协议已有大小常量，映射时应明确拒绝负数/过大值；TS `Effect.orDie` 的 defect 与 Rust `SessionError` 的可恢复错误分类不同，必须在 API 层固定转换；TS 的分页跨查询无快照，若 zenpi 需要一致读取则应在 `SessionStore` 增加快照序列并记录该差异。

## 未决问题

- `MessageTable.data`、`PartTable.data` 的完整 schema 不在本文件中，无法仅凭本源确认所有 metadata 字段及 `time_created` 的物理精度。
- `convertToModelMessages` 对动态 `tools` 的最终 provider-specific 序列化由 AI SDK 决定，本文件只明确了输入适配形状。
- `ProviderError.parseAPICallError` 与 `parseStreamError` 的具体识别规则在外部模块，无法从本文件确认所有 HTTP 状态和响应 body 的映射。
- 分页期间并发写入是否需要事务快照由调用方/数据库层决定，本文件未作承诺。

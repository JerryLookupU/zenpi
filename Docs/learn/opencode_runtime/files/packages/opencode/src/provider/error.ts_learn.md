# OC-007 — packages/opencode/src/provider/error.ts

- source_id/item_id: `OC-007`
- source_path: `packages/opencode/src/provider/error.ts`
- source_hash: `eb04a3e8ec37aa50342a74d4d51b25d671fd30f032b170a0e9be9e0d32c0ddb4`
- source_bytes: `5912`
- source_lines: `195`
- coverage: 已按文件顺序读取完整字节范围 `0-5911`（共 5912 字节），对应行范围 `L1-L195`。

## 完整行为复盘

文件先导入 `ai` 的 `APICallError`、Node `http` 的 `STATUS_CODES`、同步 `iife`、`ProviderV2` 类型和 `isContextOverflow`（L1-L5）。实现是同步、无状态的 provider 错误归一化层：它创建错误对象、清洗展示文本，并把流式或一次性 API 错误转换成判别联合；没有网络调用或重试循环。

- `HeaderTimeoutError`（L7-L13）继承 `Error`，固定只读 `name = "ProviderHeaderTimeoutError"`，构造输入是毫秒数 `ms: number`，同时保留只读字段 `ms`，消息精确为 `Provider response headers timed out after ${ms}ms`。它只是“响应头超时”分类标记；构造不启动定时器、不取消请求，也不改变任何共享状态。
- `ResponseStreamError`（L15-L21）继承 `Error`，固定只读 `name = "ProviderResponseStreamError"`。构造接收 `message: string` 和可选 `ErrorOptions`，将 `options` 原样传给 `super`，因此可以保留 `cause`。没有默认消息、重试字段或流控制逻辑。
- `isOpenAiErrorRetryable`（L23-L28）接收 `APICallError`。先读 `statusCode`：没有状态码时返回 SDK 提供的 `e.isRetryable`；有状态码时，`404` 无条件判为可重试（代码注释说明 OpenAI 偶尔把实际可用模型报成 404），其他状态回退到 `e.isRetryable`。它不检查请求是否已产生副作用，也不等待或执行退避；只返回布尔建议。
- `message`（L30-L71）接收 `providerID: ProviderV2.ID` 与 `APICallError`，但当前实现没有使用 `providerID`。逻辑顺序如下。
  1. 取 `e.message`。若为空字符串：优先原样返回 `e.responseBody`；否则若有状态码，查 `STATUS_CODES[e.statusCode]` 并在存在时返回；两者都没有则返回 `Unknown error`（L33-L42）。最终统一执行 `.trim()`，所以首尾空白会被删除（L70-L71）。
  2. 非空消息且没有响应体，或状态码存在并且消息不等于该状态的标准短语时，直接返回原消息（L44-L46）。这意味着只有“消息正好是 HTTP 状态短语”且存在响应体时才继续丰富信息；未知状态码也会因不相等而直接返回消息。
  3. 尝试 `JSON.parse(e.responseBody)`，从解析对象取 `body.message || body.error || body.error?.message`；只有取到字符串才拼接为 `${msg}: ${errMsg}`（L48-L55）。解析失败静默吞掉。由于 `body.error` 若是对象本身为真值，后面的可选链不会再被求值，嵌套 `error.message` 只有在前两个候选值为空时才有机会被取到，这是当前确切的字段优先级行为。
  4. 若响应体匹配 HTML/DOCTYPE 开头（正则 `/^\s*<!doctype|^\s*<html/i`，L57-L59），状态 `401` 返回包含重新认证命令的网关/代理提示，状态 `403` 返回权限提示，其他状态仍返回 `msg`，从而避免把整页 HTML 倾倒给用户（L60-L67）。
  5. 以上都未命中时返回 `${msg}: ${e.responseBody}`（L69），再 trim。原始响应体可能包含敏感信息，调用方需要自行决定是否脱敏。
- `json`（L73-L87）是宽松解析器：字符串尝试 `JSON.parse`，仅当结果为非空对象才返回；JSON 原语、`null` 或解析异常返回 `undefined`（L74-L81）。非空对象输入直接返回原对象（数组也满足 `typeof === "object"`，L83-L85）；其余类型返回 `undefined`（L86）。它不复制对象、不校验字段，也不捕获对象分支之后的其他异常。
- `ParsedStreamError`（L89-L100）是导出类型联合。`context_overflow` 只有 `type`、`message`、`responseBody`；`api_error` 另有布尔 `isRetryable`。因此上下文溢出分支没有显式重试布尔值，调用方不能把两种成员当成完全同形结构。
- `parseStreamError`（L102-L154）接收任意 `input`，返回 `ParsedStreamError | undefined`。
  - 先用 `json(input)` 得到 `raw`；若 `raw.message` 是字符串，再把该字符串当 JSON 二次解析，成功结果作为 `body`，失败则退回 `raw`（L103-L105）。`raw` 为空即返回 `undefined`。
  - 对 `body` 做 `JSON.stringify` 作为 `responseBody`（L107），随后要求顶层 `body.type === "error"`，否则返回 `undefined`（L108）。没有对 `JSON.stringify` 本身做 try/catch；异常对象、不可序列化值属于可抛出边界。
  - `error.code = "context_length_exceeded"` 映射为 `context_overflow`，固定消息 `Input exceeds context window of this model`（L110-L116）。`insufficient_quota` 和 `usage_not_included` 映射为不可重试 `api_error`，消息分别是账单提示和 Plus 升级提示（L117-L130）。`invalid_prompt` 使用字符串类型的 `error.message`，否则使用 `Invalid prompt.`，且不可重试（L131-L136）。`server_is_overloaded` 与 `server_error` 使用可选服务端消息，否则 `Server error.`，并标记可重试（L138-L145）。
  - 未知或缺失的错误码走兜底 `api_error`，字符串错误消息优先，否则 `Server error.`，并默认 `isRetryable: true`（L148-L153）。这是一项重要默认值：未知错误按可重试处理。
- `ParsedAPICallError`（L156-L170）是一次性 API 错误联合。`context_overflow` 的 `responseBody` 可选；`api_error` 可带可选 `statusCode`、`responseHeaders`、`responseBody`、`metadata`（字符串键值映射），并要求 `isRetryable`。
- `parseAPICallError`（L172-L192）接收 `{ providerID, error }`，其中 `error` 是 `APICallError`，返回上述联合。先调用 `message` 生成清洗后的 `m`，再用 `json` 解析响应体（L173-L174）。满足任一条件即返回 `context_overflow`：`isContextOverflow(m)` 为真、HTTP 状态为 `413`、或 JSON 体的 `error.code` 为 `context_length_exceeded`（L175-L181）；此分支保留 `m` 和原始 `input.error.responseBody`，不产生重试字段。否则构造 `api_error`（L183-L192）：仅当 URL 存在时把它放入 `metadata = { url }`；状态码、响应头、响应体原样透传；`providerID.startsWith("openai")` 时使用前述 OpenAI 专用 404 规则，否则使用 SDK 的 `error.isRetryable`（L188）。
- `export * as ProviderError from "./error"`（L195）把本文件作为命名空间重新导出，形成外部调用的 `ProviderError.*` 入口；不会复制或包装值。

并发语义方面，所有函数都是同步纯计算（错误对象仅持有实例字段），没有锁、全局计数器、Promise、后台任务或共享可变缓存；同一输入可安全并发调用。`isRetryable` 只是结果数据，重试发生在上层；`HeaderTimeoutError` 和 `ResponseStreamError` 也不会自动终止流。

## 状态、取消、恢复与副作用

本文件没有持久化、日志、认证刷新、HTTP、文件或进程副作用。取消、超时与恢复只以错误分类/布尔建议的形式表达：`HeaderTimeoutError` 表示头部阶段超时，`ResponseStreamError` 表示流阶段失败；二者不携带取消令牌，也没有 deadline。`parseStreamError` 的未知错误默认可重试，`parseAPICallError` 的重试判定依赖 OpenAI 404 特例或 SDK 标记，但实现本身不执行退避、最大次数、熔断或幂等检查。上下文溢出被单独分类，避免被当成普通服务器错误重试；413 和上下文错误码同样走该分支。解析函数不会恢复或写回 session，`responseBody`/URL/headers 的透传可能形成敏感数据泄露面，调用方需在展示和持久化前脱敏。

## 源内测试与行为判据

源文件本身未包含测试，也没有同目录测试引用。可独立验证的判据如下：

1. `HeaderTimeoutError(250).name` 必须为 `ProviderHeaderTimeoutError`，消息必须为 `Provider response headers timed out after 250ms`；`ResponseStreamError` 必须保留传入 `cause`。
2. `APICallError` 无 `statusCode` 时返回其 `isRetryable`；OpenAI provider 且状态 `404` 时无论 SDK 标记如何均为可重试；非 OpenAI 404 不触发该特例。
3. 空消息依次验证响应体、`STATUS_CODES`、`Unknown error`；状态短语加 JSON `{"message":"detail"}` 应拼接；HTML 401/403 不应输出原始 HTML。
4. `parseStreamError` 对非 JSON、JSON 原语、顶层 `type` 非 `error` 返回 `undefined`；覆盖五个显式错误码、未知码默认可重试，以及字符串化 `message` 的二次 JSON 形态。
5. `parseAPICallError` 对消息包含上下文溢出、413、以及 `error.code=context_length_exceeded` 均返回 `context_overflow`；普通 OpenAI 错误的 `metadata.url` 只在 URL 存在时出现。

## zenpi Rust 映射

- 建议落点为新增 `src/providers/error.rs`（或 `src/backend/error.rs`，由 `src/providers/mod.rs` 导出），定义 `HeaderTimeoutError`、`ResponseStreamError`、`ParsedStreamError`、`ParsedApiCallError` 及 `parse_stream_error`/`parse_api_call_error`。Rust 联合可用 `enum`，字段用 `String`、`Option<u16>`、`BTreeMap<String, String>`，并派生 `Debug`；若要跨协议输出再派生 `Serialize`。
- `src/providers/mod.rs`、`src/providers/connection.rs` 和各 `src/providers/**` 定义了 provider ID、协议、路由和能力；它们可提供 `provider_id` 与最终 URL。OpenCode 的 `providerID.startsWith("openai")` 应落为显式 `provider.starts_with("openai")` 分支；路由的 `route_digest`/`identity_scope` 可作为 `metadata` 的安全内部替代，避免把凭证写入错误文本。
- 现有 `src/backend.rs` 的 `BackendError` 已有 `Configuration`、`Transport`、`HttpStatus`、`InvalidResponse`、`Cancelled`、`DeadlineExceeded`、`Authentication` 与 `is_retryable()`。建议增加 `ContextOverflow { message, response_body }`，并让 HTTP 原始错误在映射前保留可选 headers/body/url；当前 `HttpStatus` 只保留 status 与 retry-after，无法完整承载本文件的 `ParsedAPICallError`。现有 `ProviderEvent::Failed`/`Warning` 可承载清洗后的展示消息，但不应替代结构化错误。
- `src/protocols/**` 的流式响应解码是 `parseStreamError` 的最佳调用点：在 SSE/JSON 错误帧完成后先构造结构化错误，再产生 `ProviderEvent::Failed`。`src/backend.rs` 的请求控制和重试循环已有“取消前检查、已发出事件后不自动重试、退避和熔断”语义，接入时应保留这些条件；只有在尚未发出业务事件且错误的 `is_retryable()` 为真时才允许重试。
- `src/core.rs` 的 `AgentError::Backend` 与稳定 `code()` 应保留 `backend_context_overflow`/`backend_api_error` 等机器码，不要把上下文溢出降成普通字符串；一个失败 turn 的 session 变更仍遵循 Agent 现有 admission 规则。
- `src/headless.rs` 将 `AgentEvent`/`ProviderEvent` 投影为 JSONL；当前 `provider_event_block` 对 `Failed` 固定 `retryable: true`。应改为读取结构化 `is_retryable` 和 `BackendError::code()`，并通过 `StdioResponse.error_code` 输出稳定码；错误响应要沿用其重放/请求 ID 账本，避免客户端重试造成重复 turn。
- `src/runtime.rs` 的 `CancellationToken` 是取消映射：在每个流块、头部等待和重试退避前调用 `is_cancelled()`；`Cancelled`、`DeadlineExceeded` 不应进入普通 API 重试。其 `JobOutcome::Cancelled` 说明取消不是回滚，因此 provider 已产生的外部副作用不能由错误解析器假设已撤销。
- `src/session.rs` 负责 JSONL 会话恢复和未知结果审计。provider 错误可作为受限事件/turn 错误持久化，但不要无界写入原始 `responseBody` 或认证 HTML；若请求在取消/断线时结果未知，应沿用 session 的显式 retry/abandon 规则，而不是依据 `isRetryable` 自动重放。
- `src/tool_runtime.rs` 的错误与取消边界针对工具副作用，不应把 provider `api_error` 当作工具可重试；provider 失败发生在工具调用之前可映射为 agent/backend error，工具已执行后的不确定结果必须遵循现有 operation journal 和显式 retry 约束。
- `src/protocol.rs` 已有 `ProtocolError`、`StdioResponse.error`/`error_code` 和 JSONL 编码。建议新增稳定错误码映射（至少 `context_overflow`、`provider_api_error`、`provider_header_timeout`、`provider_stream_error`），保持 `error` 人类可读、`error_code` 机器可分支；不要把 `responseBody` 直接放进公共协议。
- `src/approval.rs` 没有 provider 解析职责。其等待取消、首次响应获胜和持久化确认语义应保持不变：provider 超时/流错误不能伪造 approval decision；若取消与审批响应竞态，仍由 coordinator 的 cancellation epoch 决定，错误映射只消费最终决定。

可执行验证顺序：先为 `src/providers/error.rs` 写纯函数单元测试覆盖上述五组判据，再让 `src/protocols/**` 的流式错误样本经过解析并断言 `BackendError::code()`；最后用 headless 重放相同 request ID，确认错误响应可重放且不会在 session 中自动新增第二个 turn。

## 未决问题

1. `APICallError` 的 `responseBody`、`statusCode`、`responseHeaders` 和 `url` 在当前安装的 `ai` 版本中是否始终具有本文假设的类型与可选性，源文件自身未声明。
2. `isContextOverflow` 的具体匹配规则不在本文件中，无法仅凭本源确认哪些自然语言消息会被判定为上下文溢出。
3. `ProviderV2.ID` 的实际字符串规范及 `startsWith("openai")` 是否覆盖所有 OpenAI 兼容 provider，需要查看 provider 注册表才能完全确认；本文件只给出调用方式。

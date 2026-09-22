# OC-025 — packages/opencode/src/session/retry.ts

- source_id/item_id: `OC-025`
- source_path: `packages/opencode/src/session/retry.ts`
- source_hash: `7e24b96622f7b46586eabb68d3c2464d1c68f5cc58262f2d305878d5e55b9aed`
- source_bytes: `8139`
- source_lines: `209`
- coverage: 已从字节 `0-8138`、行 `L1-L209` 按源文件顺序完整读取，包含全部导入、注释、类型、常量、私有函数、导出符号和文件末尾重导出。

## 完整行为复盘

### 类型、常量与匹配规则

`Err` 是 `ReturnType<NamedError["toObject"]>`，即 retry 层接收的统一错误对象形状（L1-L8）。`GO_UPSELL_MESSAGE` 与 `GO_UPSELL_URL` 是免费额度弹窗的固定文案和链接（L10-L11）。`RetryReason` 允许 `"free_tier_limit"`、`"account_rate_limit"` 以及任意非空的开放字符串交集；`Retryable` 至少有 `message`，可附带 `action`，其中 action 带原因、provider、标题、文案、按钮文字和可选链接（L12-L24）。

导出的退避常量分别是 `RETRY_INITIAL_DELAY=2000`、`RETRY_BACKOFF_FACTOR=2`、`RETRY_JITTER_FACTOR=0.25`、`RETRY_MAX_DELAY_NO_HEADERS=30_000`、`RETRY_MAX_DELAY=2_147_483_647`、`RETRY_MAX_RETRIES=5`（L26-L31）。它们分别表示初始毫秒、倍增、抖动、无头部上限、计时器上限和最大重试数。`RETRYABLE_MESSAGE_PATTERNS` 共六类正则：HTTP/状态码 `429|500|502|503|504|524`，速率限制，过载/服务不可用/服务器错误，网络连接失败，超时，以及“稍后再试/资源耗尽/容量不足”（L33-L41）。匹配大小写不敏感，数组顺序只影响首次命中，不改变结果结构。

### `cap`、`exponential` 与 `delay`

`cap(ms)` 只执行上限截断 `Math.min(ms, RETRY_MAX_DELAY)`，没有下限截断（L43-L45）。因此异常的负 `retry-after` 或负 `attempt` 可能产生负延迟，这是源实现的可观察边界。

`exponential(attempt, random)` 先计算 `base = 2000 * 2^(attempt-1)`，再加 `base * 0.25 * random` 并向上取整（L80-L83）。`random` 默认由 `Math.random()` 提供，但调用者可注入任意数字；源代码不校验范围。`attempt` 正常从 `1` 开始，非正数会按幂函数自然计算。

`delay(attempt, error?, random = Math.random())` 的优先级是供应商头部优先、否则指数退避（L47-L78）：

1. 有 `error.data.responseHeaders` 时，先读取 `"retry-after-ms"`。非空字符串经 `Number.parseFloat` 解析，非 `NaN` 即直接返回 `cap(parsedMs)`；包括 `"0"`，所以可返回 `0`（L49-L57）。
2. 没有可用毫秒头时读取 `"retry-after"`。先按秒解析，成功后换算为毫秒并 `Math.ceil`，再经 `cap` 返回（L59-L65）。`parseFloat` 可接受带尾随文本的数字前缀。
3. 秒解析失败时按 HTTP 日期 `Date.parse(retryAfter) - Date.now()` 解析；只有结果为正且非 `NaN` 才返回向上取整后的剩余毫秒（L66-L70）。过去日期、非法日期会落入指数退避。
4. 头部存在但所有提示无效时，使用 `cap(exponential(attempt, random))`，允许带头部时超过 `30s`（L71-L74）。
5. 没有错误或没有响应头时，指数退避再额外受 `RETRY_MAX_DELAY_NO_HEADERS` 限制，即不超过 `30s`（L75-L78）。因此同一次 attempt 有头部和无头部的上限不同；头部提示过大仍只被 `2^31-1` 限制。

函数本身是纯计算（时间读取仅用于 HTTP 日期），没有睡眠、网络访问或持久化；实际等待由调用方的 `Effect` schedule 完成。

### `retryable`

`retryable(error, provider)` 将 `Err` 分类为 `Retryable | undefined`（L85-L155）。第一道硬拒绝是 `SessionV1.ContextOverflowError.isInstance(error)`，上下文溢出永不重试（L86-L87）。

对于 `SessionV1.APIError`（L88-L146）：

- 只有在 `error.data.isRetryable` 为真、HTTP 状态码为 `500+`、`message` 命中正则、或 `responseBody` 命中正则之一时才进入可重试分支；若四者皆否，返回 `undefined`（L89-L98）。`500+` 即使 SDK 标记 `isRetryable=false` 也强制重试，低于 `500` 的 `4xx` 不因状态码单独重试。
- 响应体包含 `FreeUsageLimitError` 时返回固定 `GO_UPSELL_MESSAGE`，并构造 `free_tier_limit` action；action 中保留传入 `provider`，按钮为 `subscribe`，链接为 `GO_UPSELL_URL`（L99-L110）。该检查发生在通用可重试门槛之后。
- 响应体包含 `GoUsageLimitError` 时尝试 `parseJSON`，读取 `metadata.workspace`、`metadata.limitName` 和响应头 `retry-after`（L112-L117）。`resetIn` 将秒数取 `max(0, ceil(seconds))`，依次格式化为天/小时/分钟；天有余数小时才同时显示小时，小时有余数分钟才同时显示分钟，小于一分钟显示 `less than a minute`，复数由局部 `unit` 处理（L117-L128）。随后拼出 usage limit 文案和 `https://opencode.ai/workspace/${workspace}/go`，返回 `account_rate_limit` action、按钮 `open settings`（L130-L143）。缺少 workspace 或限额名不会抛错，而是产生空 workspace 或通用 `Usage limit` 文案；若没有可解析的 `retry-after`，`resetIn` 为空字符串，最终文案仍会保留 `reset in ` 前缀。
- 其他 API 错误返回原始 `error.data.message`；仅当消息大小写敏感地包含 `"Overloaded"` 时改写为 `Provider is overloaded`（L145-L146）。

非 API 错误路径只接受 `error.data` 是 record 且其中 `message` 为字符串，否则返回 `undefined`（L148-L150）。消息转小写后，含 `too_many_requests` 返回规范化 `Too Many Requests`，含 `exhausted` 或 `unavailable` 返回 `Provider is overloaded`，否则若命中 `RETRYABLE_MESSAGE_PATTERNS` 返回原文，全部不命中则返回 `undefined`（L150-L155）。JSON 字符串不会在这里递归解析字段；它是作为整体字符串做关键字匹配。

### 私有解析函数与模块重导出

`matchesRetryableMessage(value)` 仅在 `value` 为字符串时对六组正则逐个 `.test`（L157-L159）；`str(value)` 把 `undefined/null` 转为空串，其余调用 `String`（L161-L164）；`num(value)` 先 `str` 再 `Number.parseFloat`，`NaN` 转成 `undefined`，负数和无穷等非 `NaN` 值不额外拒绝（L166-L170）。`parseJSON(value)` 通过 `iife` 包住解析：非字符串或 `JSON.parse` 抛错均返回 `undefined`，不会向上抛（L172-L181）。文件最后 `export * as SessionRetry from "./retry"`，为测试和调用者提供命名空间式重导出（L209-L209）。

### `policy`

`policy(opts)` 接收 `provider`、将 `unknown` 转为 `Err` 的 `parse`，以及写入重试状态的 `set({ attempt, message, action?, next }) => Effect.Effect<void>`（L183-L187），返回 `Schedule.fromStepWithMetadata(...)`（L188-L206）。每次 schedule step：先对 `meta.input` 调 `parse`，再调用 `retryable`（L189-L191）；不可重试时以 `Cause.done(meta.attempt)` 终止，不等待、不调用 `set`（L192）。当 `meta.attempt > RETRY_MAX_RETRIES`（即大于 5）时同样终止，因此 attempt `1..5` 可写状态，第 6 次停止（L193）。

可重试时进入 `Effect.gen`：计算等待时间；只有 `SessionV1.APIError.isInstance(error)` 才把错误传给 `delay`，其他可重试错误没有 headers，使用无头部指数退避（L194-L196）。读取 `Clock.currentTimeMillis`，调用 `opts.set` 写入本次 attempt、最终消息、可选 action 和绝对时间 `next = now + wait`（L197-L202），然后返回 `[meta.attempt, Duration.millis(wait)]`，供 `Schedule` 安排下一次执行（L203-L204）。`set` 失败、`parse` 抛异常或时钟 effect 失败都会使该 step 失败；源代码没有把它们转换成另一次重试。并发上，attempt 计数属于每个 `Schedule` 的 metadata，不是模块全局计数；多个独立 policy 可同时执行，`set` 的并发安全和顺序由调用者服务决定。

## 状态、取消、恢复与副作用

该文件没有自己的可变状态、锁、全局重试计数器、文件/数据库写入或网络副作用。`delay` 只读 `Date.now()`/随机数，`policy` 只通过调用者提供的 `set` effect 发布状态；在 `processor.ts` 中，`Effect.retry(SessionRetry.policy(...))` 包裹 LLM stream，`set` 实际将 `{ type: "retry", attempt, message, action, next }` 写入 `SessionStatus`（processor.ts L641-L690）。因此状态展示是外部副作用，retry 模块本身不保证持久化。

取消依赖 Effect 运行时中断：源文件没有 `AbortSignal`、取消 token 或显式取消检查；等待由 `Duration.millis(wait)` 交给 Effect 调度，外部中断可停止等待和后续重试。已有流在 `processor.ts` 的 `Effect.onInterrupt` 中处理 abort，随后 retry schedule 不再继续（processor.ts L661-L689）。超时是否可重试取决于上游将错误转换为 `APIError` 且消息命中超时模式；纯 `DeadlineExceeded` 等非该结构错误不会被本文件识别。

重试只重新执行被 `Effect.retry` 包裹的 provider stream；它不回滚已经发送的请求、流式 token、工具调用或其他外部副作用。达到第 5 次可重试之后，下一次 step 返回 `Cause.done`，最终错误交给 processor 的 `halt`/错误事件处理。`retry-after` 是建议等待时间而非恢复承诺；带头部时可等待数天级毫秒值（仅受 32 位 `setTimeout` 上限），没有头部时最多 30 秒。

## 源内测试与行为判据

对应测试文件 `/Users/wangweiyang/GitHub/opencode/packages/opencode/test/session/retry.test.ts` 已完整核对：

- `delay` 测试覆盖无头部序列 `2000,4000,8000,16000,30000`、抖动、`retry-after-ms`、秒数、HTTP date、非法/过去日期、带头部超过 10 分钟以及 32 位上限（L35-L95）。独立判据：固定 `random=0/1` 后结果必须确定，提示无效必须回到指数退避。
- `policy` 测试证明两次 step 后状态 attempt 为 `2` 且 message 保留，连续 `RETRY_MAX_RETRIES+1` 次只调用 `[1,2,3,4,5]`（L97-L149）。独立判据：第 6 次不调用 `set` 且返回完成原因。
- `retryable` 测试覆盖 JSON/纯文本速率限制、过载、网络/超时/容量正则、API response body、context overflow 禁止、500/502/503 强制、400 非重试、ZlibError、两类 Go upsell 文案（L151-L428）。独立判据：分类结果只能是精确 message/action 或 `undefined`，不得抛出 numeric code 或坏 JSON。
- `MessageV2.fromError` 集成测试覆盖 ECONNRESET、header timeout、WebSocket stream error、OpenAI 404 和 `server_error` stream chunk 转成可重试 `APIError`（L430-L521），证明本文件依赖上游错误归一化，而不是直接识别所有原始异常。

## zenpi Rust 映射

现有 zenpi 的 `src/backend.rs` 已有最接近的落点：`BackendError` 用结构化 `Transport`、`HttpStatus { status, retry_after_ms }`、`CircuitOpen`、`Cancelled`、`DeadlineExceeded` 等类型表达错误，并用 `is_retryable()` 判定 Transport、408/409/425/429/5xx（backend.rs L417-L472）。provider 请求内部已有最多 `max_retries` 的指数等待、`retry_after_ms` 取最大值、每 10ms 轮询取消（backend.rs L1774-L1825），HTTP `Retry-After` 解析支持秒和 HTTP date，且上限为 60 秒（backend.rs L1873-L1890）。这与 TS 的 2 秒起步、25% jitter、无头 30 秒上限、带头 32 位上限存在明确差异。

建议的可执行 Rust 映射与差异清单：

1. 新增 `src/retry.rs`（或将策略拆到 `backend.rs` 的独立纯模块），定义 `RetryReason`、`RetryAction`、`Retryable { message, action }`、`RetryPolicyConfig`，实现 `delay(attempt, &BackendError, random)`、`classify_retryable(&AgentError, provider)`；用 `Result<Option<Retryable>, RetryError>` 或纯 `Option` 对应 TS 的 `Retryable | undefined`。先写表格驱动测试复刻 retry.test.ts 的 L35-L95、L151-L336、L338-L427 判据。
2. `src/backend.rs` 的 `BackendError::is_retryable` 目前是状态码白名单，需决定是否扩展到 response body/transport 文本正则和 `ContextOverflow` 明确拒绝。建议保留结构化类型优先，增加 `RetryHint { after: Option<Duration> }`，并让 `retry-after-ms` 优先于秒/date；将现有 60 秒上限改为与需求一致的“无头 30 秒、带头 32 位毫秒上限”，或显式记录这是 zenpi 的产品差异。
3. 在 `src/core.rs` provider turn 边界接入 policy：每次失败先分类，再在可重试且未超过 5 次时写 retry 状态，实际 sleep 用 `CancellationToken` 每 10ms 检查。现有 `AgentError::Backend` 映射和 provider event 流（core.rs 中 `ProviderEvent` 处理及操作结果归类）可作为输入输出边界；不要在已有 unknown outcome 操作上自动重放外部副作用。
4. `src/runtime.rs` 已提供 `CancellationToken`，且文档明确 backend 在每次重试前检查（runtime.rs L49-L99）；将它传入新的 retry sleep。`BackgroundRunner` 的有界提交、取消和关闭语义（L308-L350）负责并发，不应把 attempt 计数放进全局 runner。
5. `src/session.rs` 已定义 `OperationOutcome::{Succeeded,Failed,Cancelled,Interrupted,UnknownOutcome}` 与 `retry_requires_confirmation`（session.rs L49-L95），并有 operation marker/recovery API（session.rs L1443-L1514）。TS retry 只更新 `SessionStatus`，没有 durable journal；若 zenpi 要恢复 UI，应新增明确的 `retry_scheduled`/`retry_finished` 事件，但 provider/tool 的 `UnknownOutcome` 必须继续要求显式确认，不能套用自动 retry。
6. `src/headless.rs` 已将队列压力标成可重试并释放 request ID reservation（L3870-L3901），并将 provider `Failed` 映射为 `retryable: true`、拒绝映射为 false（L4339-L4353）。可把 `Retryable.action` 投影到 headless 错误响应/状态，但需要保留 request ID 幂等与缓存规则；不要因 transient admission 失败写 terminal cache。
7. `src/protocol.rs` 的 `Command::Cancel { target_id }`（L211-L235）是显式取消入口；可增加可选 `retry_after`/`attempt` 的只读状态字段，但 retry 逻辑不应绕过协议校验。`src/approval.rs` 的取消检查和 50ms 条件变量等待（L167-L245）说明审批等待也必须在重试前后尊重取消，且 `emergency_cancel` 是不可清除的 epoch（L438-L446）。
8. `src/providers/**`（`anthropic.rs`、`google.rs`、`openai.rs`、`codex.rs`、`deepseek.rs`、`connection.rs`、`registry.rs`）当前主要负责协议、路由、认证、能力和模型元数据；`providers/mod.rs` 将 provider 映射为 `Protocol`/`AuthHeaderPolicy`（L1-L10、L13-L49），没有 TS 那样的统一文本分类或 Go upsell action。建议 provider 只填充结构化状态、headers、body，统一策略留在 `retry.rs`，并为 `opencode`/`opencode-go` 的 `FreeUsageLimitError`、`GoUsageLimitError` 增加 provider-specific action 映射测试。
9. `src/tool_runtime.rs` 的批量工具执行已把取消、线程 join 和未知副作用分开（L157-L161、L223-L243）；它适合承载“失败但可安全重试”的工具级策略，却不能把 TS 的 provider retry 直接用于 shell/tool。`src/approval.rs` 的 `ApprovalError::Cancelled` 应永远映射为不可重试取消。

验证步骤：新增纯函数测试后运行 `cargo test retry`（若没有过滤目标则运行 `cargo test`），再运行 headless/runtime/session 相关测试，检查 retry 状态写入、取消期间不新增 attempt、5 次上限、`Retry-After` 解析和 unknown outcome 阻断均成立；最后用重复 request ID 测试 `src/headless.rs` 的 replay 不产生第二次 provider side effect。

## 未决问题

1. `SessionStatus.set` 的具体存储介质和进程重启后的保留策略不在 `retry.ts` 内，需查看其 service 实现才能断言 retry 状态是否 durable。
2. `Schedule.InputMetadata.attempt` 的初始值和 Effect 版本的精确计数契约由 `effect` 库提供；本文件只通过测试确认首次可写 attempt 为 `1`、第六次停止。
3. `responseHeaders` 的实际 schema 是否始终是字符串值、以及 provider 是否会同时提供 `retry-after-ms` 与 `retry-after`，源文件未定义；代码仅按 truthiness 和 `parseFloat` 处理。
4. `GoUsageLimitError` 缺少 `workspace` 时生成的双斜杠链接是否被 UI 接受，源文件没有校验或回退策略。

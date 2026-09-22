# OC-018 — packages/opencode/src/session/message-error.ts

- `source_id/item_id`: `OC-018`
- `source_path`: `packages/opencode/src/session/message-error.ts`
- `source_hash`: `0a57851de3412cff21c9d2c679788fcaf6c835155b567e900b4873a08016f750`
- `source_bytes`: `519`
- `source_lines`: `14`
- `coverage`: 已完整读取字节范围 `[0,519)`、行范围 `L1-L14`（包含全部注释、导入、类型相关表达式和导出符号）。

## 完整行为复盘

该文件没有自定义函数或运行时流程，职责是把会话消息错误定义成可实例化、可编码和可解码的 `NamedError` 类型，并集中导出联合 schema。`L1` 从 `effect` 引入 `Schema`；`L2` 引入 `@opencode-ai/core/util/error` 的 `NamedError`。因此字段约束和错误对象的 `name/data` 形状由这两个库提供，文件本身不保存可变状态。

- `OutputLengthError`（`L4`）：调用 `NamedError.create("MessageOutputLengthError", {})` 生成错误类。构造输入是空对象 `{}`，没有业务字段；输出是名为 `MessageOutputLengthError` 的错误实例，其结构化数据为 `{}`。边界是任何额外字段是否被接受取决于 `NamedError`/`Schema.Struct({})` 的解码规则，源文件没有自定义默认值、截断或重试。该错误用于表达消息输出达到长度限制，而不是携带 provider 或文本信息。
- `AuthError`（`L6-L9`）：调用 `NamedError.create("ProviderAuthError", { providerID: Schema.String, message: Schema.String })` 生成错误类。构造输入必须同时提供字符串 `providerID` 与字符串 `message`；没有默认值，也没有空串、长度或 provider 白名单检查。成功输出为 `name = "ProviderAuthError"`、`data = { providerID, message }` 的 `NamedError` 实例；字段缺失或非字符串时，实例化/Schema 解码会走 schema 错误路径。错误名称是稳定分类键，provider 的认证细节只放在 `message`。
- `Shared`（`L11`）：按固定顺序建立只读 tuple：`AuthError.EffectSchema`、`NamedError.Unknown.EffectSchema`、`OutputLengthError.EffectSchema`。它把认证错误、通用未知错误和输出超长错误纳入同一个消息错误集合；`as const` 保留 tuple 的字面量/顺序类型。这里没有执行校验、网络访问或副作用。
- `SharedSchema`（`L12`）：以 `Schema.Union(Shared)` 构造联合解码器。输入必须匹配上述三个 schema 之一；不匹配时返回 `effect` schema 解码错误。联合顺序是源中唯一可观察的候选顺序，不能推断额外的错误类型或自动转换。
- `MessageError` 命名空间导出（`L14`）：`export * as MessageError from "./message-error"` 将本模块再次作为命名空间暴露，消费者可用 `MessageError.AuthError`、`MessageError.OutputLengthError`、`MessageError.SharedSchema` 等路径访问同一导出集合；不会复制实例、注册全局单例或触发异步任务。

从并发角度看，所有导出均为类定义、schema 常量或模块别名；没有计数器、缓存、锁、Promise、取消 token 或 worker。多个会话并发创建错误只产生独立的不可变错误值，`Shared`/`SharedSchema` 可被并发读取。

## 状态、取消、恢复与副作用

源内没有状态机。错误实例可能携带标准 `ErrorOptions` 的 `cause`（由 `NamedError` 生成类提供），但本文件没有设置或持久化 cause。没有取消、超时、重试、恢复分支，也没有文件、网络、日志、凭据或 provider 调用。`OutputLengthError` 与 `AuthError` 只是对上游结果的分类；是否重试、提示登录、终止 turn 必须由调用方决定。将其放入消息 metadata 后可随消息序列化，但该持久化行为属于调用方 schema，而非本文件。

## 源内测试与行为判据

源文件自身没有测试（源内未包含测试）。仓库级 `packages/opencode/test/util/error.test.ts` 提供了可核对判据：`new MessageError.AuthError({ providerID: "anthropic", message: "boom" })` 应是 `NamedError` 实例且 `toObject()` 为 `{ name: "ProviderAuthError", data: { providerID: "anthropic", message: "boom" } }`；`new MessageError.OutputLengthError({}).toObject()` 应为 `{ name: "MessageOutputLengthError", data: {} }`。独立验证还应检查：缺少任一 `AuthError` 字符串字段时 schema 解码失败；`SharedSchema` 接受三类 `name/data` 形状并拒绝未知名称；`MessageError` 命名空间能访问同一构造器。

## zenpi Rust 映射

- `src/core.rs` 当前 `AgentError`（`L354-L400`）已有 `Backend`、`Recovery`、`Approval` 等分类，但没有消息级 provider 认证/输出长度变体。建议新增可序列化的 `MessageError` enum：`ProviderAuth { provider_id: String, message: String }`、`Unknown { message: String, reference: Option<String> }`、`OutputLength`，并在 `AgentError::code()` 中给出稳定代码；保持 admission 失败不写 turn 的既有约束（`L1-L6`）。
- `src/providers/**` 的 `AuthHeaderPolicy` 和 provider definition（`src/providers/mod.rs:L72-L100,L146-L160`）已经知道 provider ID 与认证头。各适配器（`anthropic.rs`、`openai.rs`、`codex.rs`、`deepseek.rs`、`google.rs`）应在认证失败边界统一转换为 `MessageError::ProviderAuth { provider_id, message }`，不要把 API key 写入 `message`；可验证判据是相同 HTTP 认证失败在不同 provider 下仍保留正确 `provider_id`。
- `src/protocol.rs`（`L20-L37` 的大小常量及 `L43-L55` 的 turn 模式）负责 JSONL 输入约束。建议在 `StdioResponse` 的错误 payload 中使用显式 `kind`/`data` 标签序列化上述 enum，并复用 `MAX_TEXT_BYTES` 约束 `message`；未知标签返回 `ProtocolError`，不要静默降级为成功。
- `src/headless.rs` 将 `AgentError` 转成相关响应（`HeadlessError` 在 `L539-L551`，主循环从 `L553` 起）。映射时应保留 `ProviderAuth` 的 provider ID、`OutputLength` 的稳定代码，并让 replay 使用同一终态，避免重放改变错误类别。
- `src/session.rs` 已有 append-only JSONL 记录与 `OperationOutcome::{Succeeded,Failed,Cancelled,Interrupted,UnknownOutcome}`（`L49-L95`）。消息错误可作为 turn/assistant metadata 的结构化 `error` 字段持久化；认证错误或输出超长不应被误标成 `Cancelled`，重启恢复时仍应能区分 `Failed` 与 `UnknownOutcome`。
- `src/runtime.rs` 的 `CancellationToken`（`L49-L99`）和 `JobOutcome`（`L156-L169`）只描述工作生命周期。provider 返回 `MessageError` 时应走 `JobOutcome::Failed(E)`；仅 token 被观察到才走 `Cancelled`。不要在取消竞态中把已发布的认证失败或输出超长改写为取消。
- `src/tool_runtime.rs` 的执行器声明“不重试中断调用”（文件头 `L1-L7`），因此不应把 provider 消息错误塞进工具重试逻辑。若工具输出受到消息长度限制，应在 owner 的输出压缩/持久化边界生成 `OutputLength`，并留下可审计的失败结果。
- `src/approval.rs` 的 `ApprovalError`（`L573-L589`）处理审批无效、未知请求和取消；它与 `ProviderAuth` 语义不同。认证错误不应触发自动 `ApprovalDecision`，审批等待取消仍保持 `ApprovalError::Cancelled`。

可执行差异清单：一是在 `core.rs` 定义并测试三类 Rust 错误及稳定 code；二是在所有 `providers/**` 适配器加入统一认证错误转换；三是在 `protocol.rs`/`headless.rs` 加入带 `kind/data` 的 JSONL 编解码和 replay 判据；四是在 `session.rs` 增加错误 metadata 的 round-trip 测试；五是在 `runtime.rs` 验证失败、取消、未知结果三者在竞态下不互相改写。

## 未决问题

无法从该源文件确认：`NamedError` schema 对未知字段的精确解码策略、调用方何时把认证失败标为可重试、输出长度阈值的数值、错误 metadata 的最终持久化版本，以及 `NamedError.Unknown` 的完整字段语义；这些均应查调用方或 `NamedError` 实现后再定 Rust 兼容细节。

# Docs/learn/opencode_runtime/packages/opencode/src/session — 目录汇总（master 聚合）

- folder_path: `Docs/learn/opencode_runtime/packages/opencode/src/session`
- files: 20
- total_source_bytes: 268662
- 说明：本汇总由主控从各文件 1:1 笔记确定性聚合（不新增未在笔记中出现的语义）。

## 模块清单与要点

- `packages/opencode/src/session/compaction.ts` (21236 B) -> `Docs/learn/opencode_runtime/files/packages/opencode/src/session/compaction.ts_learn.md`
  - 要点：导出 `Event = SessionCompactionEvent`（`L26`）暴露 `Compacted` 事件定义。`PRUNE_MINIMUM=20_000`、`PRUNE_PROTECT=40_000` 是工具输出清理的 token 阈值；单个工具输出最多序列化 `2_000` 字符；`skill` 工具受保护；自动保留尾部 token 默认限制在 `2_000..15_000`（`L28-L33`）。`Turn`、`Tai
- `packages/opencode/src/session/instruction.ts` (8581 B) -> `Docs/learn/opencode_runtime/files/packages/opencode/src/session/instruction.ts_learn.md`
  - 要点：relative(instruction) 读取当前 InstanceState.context。正常情况下调用 fs.globUp(instruction, ctx.directory, ctx.worktree)；若 Flag.OPENCODE_DISABLE_PROJECT_CONFIG 为真，则改从 global.config 到 global.config 查找。两条路径都用 Effect.catch 把任何 glob 错误折
- `packages/opencode/src/session/llm.ts` (15097 B) -> `Docs/learn/opencode_runtime/files/packages/opencode/src/session/llm.ts_learn.md`
  - 要点：`OUTPUT_TOKEN_MAX`：直接别名 `ProviderTransform.OUTPUT_TOKEN_MAX`，供其他模块复用输出上限（L33-L33）。
- `packages/opencode/src/session/message-error.ts` (519 B) -> `Docs/learn/opencode_runtime/files/packages/opencode/src/session/message-error.ts_learn.md`
  - 要点：`OutputLengthError`（`L4`）：调用 `NamedError.create("MessageOutputLengthError", {})` 生成错误类。构造输入是空对象 `{}`，没有业务字段；输出是名为 `MessageOutputLengthError` 的错误实例，其结构化数据为 `{}`。边界是任何额外字段是否被接受取决于 `NamedError`/`Schema.Struct({})` 的解码规则，源文件
- `packages/opencode/src/session/message-v2.ts` (25944 B) -> `Docs/learn/opencode_runtime/files/packages/opencode/src/session/message-v2.ts_learn.md`
- `packages/opencode/src/session/message.ts` (5041 B) -> `Docs/learn/opencode_runtime/files/packages/opencode/src/session/message.ts_learn.md`
  - 要点：导入与再导出（L1-L9）：`Schema` 来自 `effect`；`SessionID` 来自 `./schema`；`NonNegativeInt`、`ProviderV2`、`ModelV2` 来自 core；`MessageError` 及其两个错误类型来自 `./message-error`。L9 再导出 `AuthError`、`OutputLengthError`，所以使用者可从本模块取得同一错误类型。导入不产生副作用。
- `packages/opencode/src/session/overflow.ts` (1313 B) -> `Docs/learn/opencode_runtime/files/packages/opencode/src/session/overflow.ts_learn.md`
  - 要点：输入对象是 `{ cfg: ConfigV1.Info, model: Provider.Model, outputTokenMax?: number }`；读取 `model.limit.context`（`L11`）。
- `packages/opencode/src/session/processor.ts` (27266 B) -> `Docs/learn/opencode_runtime/files/packages/opencode/src/session/processor.ts_learn.md`
  - 要点：`settleToolCall(toolCallID)` 取出并删除工具记录，若存在则成功 `Deferred`；重复 settle 安全无效（L123-L127）。
- `packages/opencode/src/session/prompt.ts` (64944 B) -> `Docs/learn/opencode_runtime/files/packages/opencode/src/session/prompt.ts_learn.md`
  - 要点：模块级常量与判定函数：`decodeMessageInfo`/`decodeMessagePart` 在 `L63-L64` 以 `Schema.decodeUnknownExit` 做保存前诊断；MCP 附件上限 `MAX_MCP_RESOURCE_BLOB_BYTES=10 MiB`、允许 MIME 集合在 `L65-L72`；结构化输出说明和系统提示在 `L74-L82`。`mcpResourceBase64Size(value)
- `packages/opencode/src/session/reminders.ts` (3373 B) -> `Docs/learn/opencode_runtime/files/packages/opencode/src/session/reminders.ts_learn.md`
  - 要点：1. 旧计划模式分支：当 `flags.experimentalPlanMode` 为假时进入 L26-L49。若当前 `input.agent.name === "plan"`，就直接向最后用户消息的 `parts` 推入一个合成文本 part（L27-L35）：`id` 由 `PartID.ascending()` 生成，`messageID`/`sessionID` 取用户消息元数据，`type` 为 `"text"`，文本为完整
- `packages/opencode/src/session/retry.ts` (8139 B) -> `Docs/learn/opencode_runtime/files/packages/opencode/src/session/retry.ts_learn.md`
  - 要点：1. 有 `error.data.responseHeaders` 时，先读取 `"retry-after-ms"`。非空字符串经 `Number.parseFloat` 解析，非 `NaN` 即直接返回 `cap(parsedMs)`；包括 `"0"`，所以可返回 `0`（L49-L57）。
- `packages/opencode/src/session/revert.ts` (5659 B) -> `Docs/learn/opencode_runtime/files/packages/opencode/src/session/revert.ts_learn.md`
  - 要点：1. **输入模式与公共契约（L1-L26）**
- `packages/opencode/src/session/run-state.ts` (5506 B) -> `Docs/learn/opencode_runtime/files/packages/opencode/src/session/run-state.ts_learn.md`
  - 要点：`Interface`（`L11-L25`）定义四个公开能力：`assertNotBusy(sessionID)` 只返回 `void` 或 `Session.BusyError`；`cancel(sessionID)` 请求取消且没有业务错误类型；`ensureRunning(sessionID, onInterrupt, work)` 确保工作运行并返回 `SessionV1.WithParts`；`startShell(sessi
- `packages/opencode/src/session/schema.ts` (814 B) -> `Docs/learn/opencode_runtime/files/packages/opencode/src/session/schema.ts_learn.md`
  - 要点：`SessionID` 导出常量（`L7`）：直接赋值为 `SessionV2.ID`，因此运行时校验、编码/解码规则和生成能力完全继承 `@opencode-ai/core/session` 的定义，本文件没有再加前缀、长度或格式约束。它的同名类型导出（`L8`）是 `Schema.Schema.Type<typeof SessionID>`，用于把 schema 的静态类型暴露给 TypeScript；它不产生新的运行时值。输入若不
- `packages/opencode/src/session/session.ts` (35643 B) -> `Docs/learn/opencode_runtime/files/packages/opencode/src/session/session.ts_learn.md`
  - 要点：`createNext(input)` 从 `InstanceState.context` 取得 project，生成 descending `SessionID`、随机 slug、安装版本、目录/路径/父子关系；标题默认是前缀加 `new Date().toISOString()`；permission 复制数组，cost/tokens 初始化为零，created/updated 为 `Date.now()`。先日志再发布 `Sess
- `packages/opencode/src/session/status.ts` (1971 B) -> `Docs/learn/opencode_runtime/files/packages/opencode/src/session/status.ts_learn.md`
  - 要点：`Info` 常量和 `Info` 类型别名（`L8-L9`）直接转出 `SessionStatusEvent.Info`。可接受值由被引用的 schema 定义：`{type: "idle"}`、`{type: "busy"}`，或 `retry` 状态（含非负 `attempt`、字符串 `message`、可选 `action` 结构和非负 `next`）。本文件不重新校验字段，也不提供默认 retry 参数。
- `packages/opencode/src/session/summary.ts` (5196 B) -> `Docs/learn/opencode_runtime/files/packages/opencode/src/session/summary.ts_learn.md`
  - 要点：`unquoteGitPath(input: string)`（`L10-L64`）是本文件唯一的路径解码函数。输入不同时以双引号开头和结尾时原样返回（`L11-L12`），所以普通路径、单边引号或空字符串都不会被改写。对包在双引号中的 Git quoted path，它去掉首尾引号后逐字符扫描（`L13-L16`）：普通字符按 `charCodeAt(0)` 加入 `bytes`（`L17-L20`）；末尾反斜杠没有后继字符时保留反斜
- `packages/opencode/src/session/system.ts` (6462 B) -> `Docs/learn/opencode_runtime/files/packages/opencode/src/session/system.ts_learn.md`
  - 要点：`provider(model: Provider.Model)`（`L28-L51`）按固定优先级返回只含一个字符串的数组。先检查 `model.api.id` 是否含 `muse`：`muse-glimmer` 使用 `PROMPT_META` 将所有 `{{MODEL_NAME}}` 替换为 `Muse Glimmer`，其他 Muse 使用 `Muse Spark`（`L29-L32`）。随后 `gpt-4`、`o1`、`o3`
- `packages/opencode/src/session/todo.ts` (2480 B) -> `Docs/learn/opencode_runtime/files/packages/opencode/src/session/todo.ts_learn.md`
  - 要点：依赖与导出别名（L1-L9）：文件引入 `LayerNode`、`SessionID`、Effect 运行时的 `Effect/Layer/Context`、数据库服务 `Database`、Drizzle 的 `eq/asc`、`TodoTable`、`EventV2Bridge` 与共享 schema 命名空间 `SessionTodo`。本文件不自行定义 todo 字段校验、数据库表结构或事件载荷，均委托这些依赖。
- `packages/opencode/src/session/tools.ts` (23478 B) -> `Docs/learn/opencode_runtime/files/packages/opencode/src/session/tools.ts_learn.md`

## zenpi Rust 映射（聚合）

- `packages/opencode/src/session/compaction.ts`: `src/context.rs` 保持 token 预算、whole-turn cut、`SemanticCheckpoint` 与摘要验证；若要对应 `preserve_recent_tokens/tail_turns`，新增纯函数 `select_recent_tail(turns, TailPolicy)`，并用测试证明边界和重复 compaction 不把 prior summary 算入 tail。
- `packages/opencode/src/session/instruction.ts`: src/core.rs:L4050-L4137 已把技能、persona、resource 和 selected skills 合成 instructions，按 instructions.len().div_ceil(4) 扣减 context budget，再通过 CompletionRequest::with_instructions 注入 provider。建议新增 src/instruction.rs 的 Instructio
- `packages/opencode/src/session/llm.ts`: provider 抽象落在 `src/backend.rs`：`CompletionRequest`（约 L182-L234）承载 turns/model/tools/instructions/token limit，`ProviderEvent`（L236-L294）提供 TextDelta、ReasoningDelta、ToolCallDelta/Done、Usage、Completed、Failed；`Backend::compl
- `packages/opencode/src/session/message-error.ts`: `src/core.rs` 当前 `AgentError`（`L354-L400`）已有 `Backend`、`Recovery`、`Approval` 等分类，但没有消息级 provider 认证/输出长度变体。建议新增可序列化的 `MessageError` enum：`ProviderAuth { provider_id: String, message: String }`、`Unknown { message: String,
- `packages/opencode/src/session/message-v2.ts`: `src/session.rs` 已有 append-only `SessionStore`、`SessionRecord`、`Turn`、`OperationRecovery` 和 `SessionStore::records/turns`。可新增 `MessageInfo`、`MessagePart`、`WithParts` 以及 `MessageStore::page/get/parts/stream`；用 `(created_a
- `packages/opencode/src/session/message.ts`: `ToolCall`/`ToolPartialCall`/`ToolResult`/`ToolInvocation` 对应 Rust `enum ToolInvocation` 的三个 `#[serde(tag = "state")]` 变体，`step: Option<u64>` 表达 `NonNegativeInt`，`args: serde_json::Value` 表达 `Schema.Unknown`，结果保留 `String
- `packages/opencode/src/session/overflow.ts`: `src/context.rs` 已有 `ContextBudget { max_tokens, reserved_output_tokens }`、`estimate_tokens` 和上下文准备逻辑；建议新增纯函数 `usable_context_tokens(model: &ModelDescriptor, budget: ContextBudget, output_token_max: Option<u64>) -> u64` 
- `packages/opencode/src/session/processor.ts`: `src/core.rs` 的 `Agent::process_with_cancel_and_events`、`run_active_turn_cancelable_with_events` 是 `process` 的主要落点：前者负责 admission，后者负责 provider operation marker、取消、工具循环和 `ProviderEvent` sink（`src/core.rs:L5310-L5373`、`L3
- `packages/opencode/src/session/prompt.ts`: `PromptInput`/`CommandInput`/`LoopInput`/`ShellInput` 应落到 `src/protocol.rs` 的 `Command`、`TurnMode`、`UserShellRequest`（`L46-L119`、`L214-L510`），继续使用 `MAX_TEXT_BYTES`、控制字符和 ID 校验；命令模板插值可落在 `core.rs` 的 admission 辅助函数。当前 Rust
- `packages/opencode/src/session/retry.ts`: 1. 新增 `src/retry.rs`（或将策略拆到 `backend.rs` 的独立纯模块），定义 `RetryReason`、`RetryAction`、`Retryable { message, action }`、`RetryPolicyConfig`，实现 `delay(attempt, &BackendError, random)`、`classify_retryable(&AgentError, provider)`；用
- `packages/opencode/src/session/revert.ts`: 1. **会话与回退状态：src/session.rs**。新增可序列化 RevertState { message_id: String, part_id: Option<String>, snapshot_id/digest: String, summary: DiffSummary }，通过 append-only event（如 session_revert_set/session_revert_clear）持久化；实现 Ses
- `packages/opencode/src/session/run-state.ts`: `src/runtime.rs` 的 `CancellationToken`（`L56-L91`）、`BackgroundRunner`（`L237-L319`）和 `RuntimeEvent`（`L158-L184`）最接近 `Runner`：建议新增 `SessionRunState`，内部用 `HashMap<SessionId, SessionRunner>`，`SessionRunner` 持有 `JobId`、取消 toke
- `packages/opencode/src/session/schema.ts`: `src/session.rs` 是最接近的落点：已有 `SessionHeader.session_id`、`SessionStore::session_id()`、`SessionSummary` 和 append-only JSONL 恢复。建议新增集中式 `SessionId`, `MessageId`, `PartId` newtype（或放入 `src/session.rs` 的 `ids` 子模块），实现 `Deseria
- `packages/opencode/src/session/session.ts`: `zenpi/src/session.rs` 的 `SessionStore`（JSONL header/turn/event、`SessionError`、`fork_to`、archive/catalog 与 `list_sessions/search_sessions`，如 `L327-L434`、`L1410-L1601`、`L1655-L1724`）是 `Info`/`create/get/list/fork/remove` 
- `packages/opencode/src/session/status.ts`: `src/core.rs`：`AgentPhase::{Idle,Running,Closed}`、`AgentSnapshot` 和 `AgentEvent`（尤其 `TurnAccepted`、`Provider`、`Error`）是现有状态投影。建议新增 `SessionStatusInfo`（`Idle|Busy|Retry{attempt,message,action,next}`）及 `SessionStatusStore`
- `packages/opencode/src/session/summary.ts`: 对照 `src/session.rs`：将 `summarize` 的清零和最终写回实现为一个可验证的 `update_message_summary(session_id, message_id, summary)`；写入应沿 `append_json`/事件日志路径，保证“先持久化后更新内存投影”的习惯（现有 `append_turn` 的行为可作判据，`src/session.rs:L861-L891`）。可将快照 ID 与 `T
- `packages/opencode/src/session/system.ts`: **模型与 provider prompt 路由**：源的字符串启发式应落在新模块 `src/system_prompt.rs` 的 `provider_prompt(&ModelDescriptor) -> &'static str`（Muse 另返回 `String`）。zenpi 的 `src/providers/registry.rs` 用精确 `(provider,id)` 的 `ModelDescriptor`、`resol
- `packages/opencode/src/session/todo.ts`: `src/session.rs`（主要落点）：当前 `SessionStore`（定义于 L146-L146，事件追加 API 在 L1207-L1207）是 append-only JSONL 日志，而源实现依赖可更新的 `TodoTable` 快照。建议增加可序列化的 `TodoInfo { content: String, status: String, priority: String }`、按会话归属的 `Vec<TodoIn
- `packages/opencode/src/session/tools.ts`: 1. 先在 `src/tool_runtime.rs`补一个纯函数 `format_mcp_resource_content`及 `base64_size/format_bytes`，覆盖空内容、非法 record、padding、10 MiB 边界和五种允许 MIME。

## 未决问题（聚合）

- `packages/opencode/src/session/compaction.ts`: 
- `packages/opencode/src/session/instruction.ts`: withTransientReadRetry 的具体重试次数、退避和哪些状态码属于 transient 不在本文件中定义。
- `packages/opencode/src/session/llm.ts`: `LLMRequestPrep.prepare`、`LLMNativeRuntime.stream`、`LLMAISDK.toLLMEvents` 的完整 provider 特殊字段和事件映射不在本文件内，需结合各自源码确认最终 wire schema。
- `packages/opencode/src/session/message-error.ts`: 
- `packages/opencode/src/session/message-v2.ts`: `MessageTable.data`、`PartTable.data` 的完整 schema 不在本文件中，无法仅凭本源确认所有 metadata 字段及 `time_created` 的物理精度。
- `packages/opencode/src/session/message.ts`: 1. `NonNegativeInt`、`SessionID`、`ModelV2.ID`、`ProviderV2.ID` 和 `MessageError.SharedSchema` 的精确约束定义在其他文件，本源只能确认其被引用，不能确认长度、格式或错误变体。
- `packages/opencode/src/session/overflow.ts`: 1. `ProviderTransform.maxOutputTokens` 在 `outputTokenMax` 未提供、模型没有输出上限或配置非法时的精确返回/抛错规则不在本文件内。
- `packages/opencode/src/session/processor.ts`: 
- `packages/opencode/src/session/prompt.ts`: 1. `Session.updateMessage/updatePart` 的事务/幂等保证不在本文件中，无法确认并发写入失败时是否原子回滚。
- `packages/opencode/src/session/reminders.ts`: `Session.plan(input.session, ctx)`的确切路径规则、是否始终位于 workspace 内，无法从本文件确认。
- `packages/opencode/src/session/retry.ts`: 1. `SessionStatus.set` 的具体存储介质和进程重启后的保留策略不在 `retry.ts` 内，需查看其 service 实现才能断言 retry 状态是否 durable。
- `packages/opencode/src/session/revert.ts`: Snapshot.Patch 的具体 patch 方向、哈希校验和 Snapshot.Service 对未跟踪/删除文件的精确处理不在本文件中，需继续核对 src/snapshot 实现与测试。
- `packages/opencode/src/session/run-state.ts`: 1. `Runner.make` 的 `ensureRunning`、`startShell`、`busy` 字段以及 `onInterrupt` 的确切触发时机不在本文件中，无法确认工作是否排队、替换还是拒绝。
- `packages/opencode/src/session/schema.ts`: 1. `SessionV2.ID` 的具体格式、是否自带 `.ascending`、长度和解码错误结构不在本文件内，需继续读取 `@opencode-ai/core/session` 才能完全对齐。
- `packages/opencode/src/session/session.ts`: 1. `EventV2Bridge` 如何把 publish 映射到 `SessionTable`/`PartTable` 的最终写入、是否事务化，源文件未定义。
- `packages/opencode/src/session/status.ts`: 1. `InstanceState.make` 的具体生命周期和并发保证不在本文件中，无法仅凭 `status.ts` 判断 Map 是否会跨 workspace 共享。
- `packages/opencode/src/session/summary.ts`: 1. `Snapshot.diffFull` 的具体快照存储、差异排序、`FileDiff` 字段和错误类型不在本文件中，无法确认空文件、删除文件或重命名的精确表示。
- `packages/opencode/src/session/system.ts`: 
- `packages/opencode/src/session/todo.ts`: 1. `Database.Service` 的事务隔离级别、取消时驱动行为和具体回滚保证不在本文件中。
- `packages/opencode/src/session/tools.ts`: 1. `MCP.Service`、`McpCatalog.convertTool`以及 `ToolRegistry`各自对工具超时、连接断开和错误值的精确类型未在本文件定义；只能确认本文件会传播失败，无法确认是否自动重连或重试。

## 覆盖

- 本目录 20 个源文件均有 1:1 笔记；`file_learn_index.tsv` 与本清单一一对应。

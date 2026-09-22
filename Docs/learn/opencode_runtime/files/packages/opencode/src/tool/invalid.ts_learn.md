# OC-041 — packages/opencode/src/tool/invalid.ts

```yaml
source_id: packages/opencode/src/tool/invalid.ts
item_id: OC-041
source_path: packages/opencode/src/tool/invalid.ts
source_hash: 2cc8ba3e9e2422a12eaeec33433ff143fdd27876b98268b9217d0245fed5c0f8
source_bytes: 531
source_lines: 21
coverage: bytes 0-530（完整 531 字节）；lines 1-21（按源文件顺序完整读取，含注释、类型、导出符号）
```

## 完整行为复盘

- `import { Effect, Schema } from "effect"`（L1）引入 Effect 运行时构造器和 Effect Schema 解码器；`import * as Tool from "./tool"`（L2）引入本地工具定义 API。文件没有其它隐式依赖或全局状态。
- `export const Parameters`（L4-L7）是 `Schema.Struct`，字段 `tool: Schema.String`（L5）和 `error: Schema.String`（L6）都是必填字符串。它定义了 `invalid` 工具的输入形状：输入必须能被 Effect Schema 解码为包含这两个字符串的结构。源内没有长度上限、正则、枚举、非空检查、默认值或显式的额外字段策略；未知字段究竟被接受、丢弃还是拒绝，应以 Effect Schema 的实际解码行为验证，不能从本文件臆断。`tool` 是原始工具名的诊断字段，`error` 是修复失败原因；二者都只在解码成功后传给执行函数。
- `export const InvalidTool = Tool.define(...)`（L9-L21）把工具 ID 固定为字符串 `"invalid"`（L10）。初始化参数是 `Effect.succeed({...})`（L11），因此初始化立即成功，不需要环境、服务、文件或网络资源。定义的 `description` 为 `"Do not use"`（L12），`parameters` 指向上面的 `Parameters`（L13）。
- `execute: (params: { tool: string; error: string }) => ...`（L14-L15）只接收已通过 Schema 的参数；它没有声明 `Tool.Context` 参数，因此不会读取会话、调用 ID、`abort`、权限询问或消息历史。执行体也是 `Effect.succeed`（L15），没有异步等待、分支、抛错或重试。注意 `params.tool` 被校验但未用于输出；实际文本只使用 `params.error`。
- 成功值固定包含 `title: "Invalid Tool"`（L16）、`output: \`The arguments provided to the tool are invalid: ${params.error}\``（L17）、`metadata: {}`（L18）。`params.error` 通过模板字符串原样插入，源码没有截断、转义、脱敏或换行规范化；只要 Schema 接受该字符串，空字符串也没有被本文件禁止。每次调用都构造新的结果对象，调用之间没有共享可变状态，因而在宿主允许的情况下可并发执行。
- `Tool.define` 在本文件外把定义包装成工具信息（相关实现会再次解码参数，并在执行后做统一输出截断/遥测）；因此缺失字段或非字符串输入的错误来自包装层，而不是 L14-L19 的 `execute`。Opencode 的注册表将该工具始终放入 builtin 集合（`registry.ts` L209-L245），LLM 修复回调在工具名大小写修复失败时把原始 `toolName` 与错误消息编码成 `{tool,error}`，再把调用名改为 `invalid`（`session/llm.ts` L296-L323）。它从 `activeTools` 中排除（同处 L317），所以主要是修复/回显目标，不是模型正常选择的业务工具。

## 状态、取消、恢复与副作用

该文件没有模块级状态、缓存、计数器、锁或持久化句柄。初始化和执行都使用 `Effect.succeed`（L11、L15），没有文件、进程、网络、数据库、审批、权限或工作区写入副作用；唯一成本是构造输入和结果字符串/对象。

源码没有读取 `Tool.Context.abort`，也没有 `Effect.timeout`、轮询取消、超时分支或补偿逻辑。宿主可以在 Effect 层中断整个计算，但本工具自身没有可观测的取消点；取消不会触发回滚，因为它从未启动外部副作用。源码也没有 retry/backoff/idempotency key。恢复语义同样不存在：若宿主在调用前后记录操作，恢复只能由宿主决定，不能从此文件推导“已执行”或“可安全重试”。由于执行是纯的诊断结果，重复调用在语义上是幂等的；但调用方仍应保持原始 `call_id` 关联，避免把一次修复结果误配给另一调用。

## 源内测试与行为判据

源文件本身没有测试（“源内未包含测试”）。同目录工具参数测试在 `/Users/wangweiyang/GitHub/opencode/packages/opencode/test/tool/parameters.test.ts` 的 `invalid` 小节（L166-L174）验证：`{ tool: "foo", error: "bar" }` 可解析；缺少任一字段均不可解析。JSON Schema 快照测试也包含 `Invalid`（L37-L54），可检查对 provider 暴露的对象 schema。可独立验证的判据如下：

1. 用 `Schema.decodeUnknownSync(Parameters)` 解码完整的两个字符串字段应成功；缺 `tool` 或 `error`、或将任一字段改成非字符串应失败。
2. 初始化 `InvalidTool` 后执行 `execute({tool: "foo", error: "bar"}, ctx)`，应得到 `title === "Invalid Tool"`、`output === "The arguments provided to the tool are invalid: bar"`、`metadata` 为空对象；`tool` 的值改变时输出仍只随 `error` 改变。
3. 在 LLM 修复路径中，原始工具调用应被重写为工具名 `invalid`，并且 `invalid` 不出现在 `activeTools`；这对应 `session/llm.ts` L296-L323 的行为判据。

建议在 opencode 包根目录运行 `bun test packages/opencode/test/tool/parameters.test.ts`，再用一个最小 Effect 运行器验证第 2 条；测试不应把 `invalid` 当作普通业务工具调用。

## zenpi Rust 映射

- **工具定义落点：** zenpi 的 `src/tools.rs` 已有 `ToolDefinition`、`Tool` trait、`ToolRegistry` 和 `ToolResult`。建议新增 `InvalidTool`（可与其它 builtin 放在同文件），`definition()` 返回 `name: "invalid"`、`description: "Do not use"`、对象型 `input_schema`（必需字符串 `tool`/`error`），`side_effect: ToolSideEffect::ReadOnly`；`invoke()` 仅校验对象字段并返回 `json!({"title":"Invalid Tool","output":...,"metadata":{}})`。若希望直接调用 `ToolRegistry::with_read_only_builtins()` 得到等价注册表，应在那里注册它，并用 `ToolDefinition::validate` 保证定义合法（现有定义/结果类型见 `src/tools.rs`，与 `src/tool_runtime.rs` L16-L19 的使用一致）。
- **调用与结果差异：** zenpi `ToolResult` 以 `call_id`、`tool`、`output: Value` 或 `ToolFailure` 表示（`src/tool_runtime.rs` L16-L19、L446-L455），没有 opencode `ExecuteResult` 的一等 `title`/`metadata` 字段。因此把三字段对象放入 `output` 是兼容性最高的映射；若 headless 客户端只期待文本，则应在协议投影层读取对象的 `output`，不能静默丢掉 `title`/`metadata`。
- **修复入口：** opencode 的关键语义是“坏工具调用 → 合成 `invalid` 调用”，而 zenpi 当前 registry 未知工具通常走 `ToolError::UnknownTool`。建议在 `src/core.rs` 的工具调用准备/`invoke_tool` 前增加 `repair_invalid_tool_call(call, error)`：保留原 `call.id`，生成 `ToolCall { name: "invalid", arguments: json!({"tool": original_name, "error": redacted_error}) }`，再走普通 registry；仅对参数解析/工具调用格式错误启用，不能把权限拒绝、取消或未知副作用错误伪装成参数错误。`src/core.rs` 已有 `PreparedTool`/`ToolInvocationOutcome`（L120-L129）和单调用批执行入口（L4928-L4935、L5086-L5099），适合放置该转换并保留审计证据。
- **批执行、并发与取消：** `src/tool_runtime.rs` 的批执行器规定结果按源顺序返回、批量取消时不执行待处理调用，并对非协作处理器执行 join（L151-L176、L438-L455）。`invalid` 是只读、无状态的诊断工具，可标记为 `Parallel` 或沿用默认 `Sequential`；为保持简单和与修复调用的调用序一致，建议先沿用默认顺序模式。它不应引入额外线程、超时或重试。宿主取消仍由 `cancelled` 谓词传播；取消发生在调用前应得到 `ToolErrorCode::Cancelled`，不能产出伪造的“invalid 参数”成功结果。
- **运行时取消边界：** `src/runtime.rs` 的 `CancellationToken` 是协作式、幂等取消，并要求工作在重试/流式边界检查（L49-L90）。这与源工具不主动读取 `abort` 的差异应明确记录：Rust 适配器可以在进入 `invoke` 前检查 token，但 `InvalidTool` 本身没有长任务，不需要新增 polling；完成标记与迟到取消的竞争由 runtime 统一处理。
- **审批与副作用：** `src/approval.rs` 将审批请求与取消等待分开（L109-L135、L167-L245）。将 `invalid` 声明为 `ReadOnly` 后，它应绕过写入审批；不要为它创建 `ApprovalRequest`，也不要把 `tool` 字段当作真实待执行工具名。若修复前的原调用是写入工具，原调用的审批/预览仍应在修复前失败路径中记录，合成诊断调用不能绕过安全门。
- **持久化与恢复：** `src/session.rs` 将工具操作区分为 `Succeeded/Failed/Cancelled/UnknownOutcome`，并规定未知结果必须显式恢复决定、重试需新操作（L49-L95、L1525-L1588）。源 `invalid.ts` 没有持久化；Rust 侧只需把一次诊断结果作为普通已完成工具事件写入，不能把它标成原始副作用已成功。若原始调用在重写前已留下 operation marker，仍应保留 `UnknownOutcome` 证据，避免用 `invalid` 结果覆盖原调用的不确定性。
- **协议与 headless 投影：** `src/protocol.rs` 以有界 JSONL 传输（L1-L37），因此合成的 `error` 必须经过既有文本/字节上限与脱敏规则，不能无限复制 provider 错误。`src/headless.rs` 将 `ToolCall`/`ToolResult` 作为高优先级事件（L907-L920），并把 provider 的 `ToolCallDone` 映射成 queued 工具状态（L4339-L4364）；建议让修复后的 `invalid` 调用仍沿用原 `call_id`，在事件中标明原工具名和修复原因，避免 UI 只看到无关联的 `invalid`。
- **Provider 对照：** `src/providers/mod.rs` 的 `OptionPolicy` 有 `tool_choice` 开关（L121-L144），`src/providers/connection.rs` 会在能力交集中裁剪 `tools`/`tool_choice`（L290-L325），`src/providers/registry.rs` 的模型覆盖也有 `tools: Option<bool>`（L125-L140）。这些模块只决定 provider 是否支持工具，不实现 invalid repair。建议在 provider 请求前按能力决定是否发送工具列表：与 opencode 一样，`invalid` 可存在于可调用定义集合但不进入“主动工具”选择集合；若 provider 不支持 tools，直接走现有文本/错误路径，不合成无法发送的工具调用。

## 未决问题

1. `Schema.Struct` 对未知字段的精确策略（保留、剥离或拒绝）及其版本差异，无法仅由 `invalid.ts` 确认。
2. 工具错误文本是否已在 LLM/provider 层脱敏、截断或带有调用上下文，需结合完整 session/provider 实现确认；本文件只保证模板字符串原样插入 `params.error`。
3. zenpi 是否要把 `title`/`metadata` 提升为 `ToolResult` 的独立字段，或统一保留在 `output` 对象中，取决于现有 headless 客户端契约；本源文件不能决定该协议选择。

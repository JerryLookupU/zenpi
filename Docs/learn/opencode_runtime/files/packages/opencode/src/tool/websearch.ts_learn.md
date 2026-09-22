# OC-060 — packages/opencode/src/tool/websearch.ts

## 元信息

- source_id/item_id：OC-060
- source_path：packages/opencode/src/tool/websearch.ts
- source_hash：edb175726b7830d242f59417ce6961f44d39e500bccc068bf2dddbf61d5ca92a
- source_bytes：5189
- source_lines：143
- coverage：已从首字节到末字节完整读取，bytes 0-5188（5189/5189），lines 1-143（143/143），包括 import、注释、Schema、类型、私有函数和导出符号。

## 完整行为复盘

文件导入 `Effect`/`Schema`、`HttpClient`、本地 `Tool` 与 `McpWebSearch`，并读取描述文本、`checksum`、`InstallationVersion` 和 `RuntimeFlags`（L1-L8）。它把两个远程 MCP 搜索服务包装成一个名为 `websearch` 的工具；本文件不实现 HTTP 解析，实际请求和响应解析委托给 `mcp-websearch.ts`。

- `Parameters`（L10-L25）是导出的输入 Schema。必填 `query: string`。`numResults`、`livecrawl`、`type`、`contextMaxCharacters` 可省略；其中 `livecrawl` 只接受 `fallback|preferred`，`type` 只接受 `auto|fast|deep`，其余两个是数字。注解给出了文档默认值：结果数 8、livecrawl 为 fallback、type 为 auto、上下文上限 10000；Schema 本身只表达可选性和类型，默认值在调用 Exa 时通过 `||` 补上，Parallel 路径不使用这些字段（L12-L24、L66-L96）。缺少 query 或字面量/数字类型不符会在工具调用进入执行函数前被 Schema 拒绝。

- `WebSearchProviderSchema` 与导出的 `WebSearchProvider`（L27-L28）把提供方封闭为 `"exa" | "parallel"`。这使选择结果和 MCP 分支都具有有限域；运行时从环境读取的未知字符串不会被接受为覆盖值。

- `selectWebSearchProvider(sessionID, flags)`（L30-L37）按固定优先级选择提供方：先看 `OPENCODE_WEBSEARCH_PROVIDER`，仅精确等于 `exa` 或 `parallel` 才生效；否则 `flags.parallel` 优先于 `flags.exa`；两个 flag 都假时，对 `checksum(sessionID) ?? "0"` 按 36 进制解析，偶数选 Exa，奇数选 Parallel。默认 flags 是两个 false。相同 session ID 在无环境覆盖和同一 flag 集合下稳定；若解析结果是 NaN，奇偶比较为假而落到 Parallel。环境覆盖属于进程全局，优先级高于运行时 flag（L30-L36）。

- `webSearchProviderLabel(provider)`（L39-L43）把 `parallel` 映射为 `Parallel Web Search`，`exa` 映射为 `Exa Web Search`，任何未知值（包括 `undefined`）回退为 `Web Search`。它不抛错，适合展示层处理不可信 metadata。

- `webSearchModelName(extra)`（L45-L52）从 `Tool.Context["extra"]` 提取给 Parallel analytics 的模型名。只有 `extra.model` 是对象才继续；若 `model.api.id` 是字符串，优先使用它，否则使用 `model.id`；最终截断到前 100 个 UTF-16 code unit。所有缺失、非对象、非字符串路径返回 `undefined`，不会阻止搜索。API 模型 ID 优先于显示/内部模型 ID（L47-L51）。

- 私有 `parallelAuthHeaders()`（L54-L58）总是生成 `User-Agent: opencode/<InstallationVersion>`。若没有 `PARALLEL_API_KEY`，只返回该头；有值时追加 `Authorization: Bearer <key>`。密钥只进入请求头，不进入 metadata/result；没有显式格式校验，空字符串因 JS 的 falsy 规则会走“无 key”分支（L55-L57）。

- 私有 `callProvider(http, provider, params, ctx)`（L60-L97）统一分派 MCP 调用。Parallel 分支调用 `McpWebSearch.call`，地址 `PARALLEL_URL`，工具名 `web_search`，参数为 `objective=params.query`、单元素 `search_queries=[params.query]`、`session_id=ctx.sessionID`、`model_name=webSearchModelName(ctx.extra)`，超时输入为 `"25 seconds"`，并传入 `parallelAuthHeaders()`（L66-L80）。Exa 分支使用 `EXA_URL`、工具名 `web_search_exa`，参数为 query、`type || "auto"`、`numResults || 8`、`livecrawl || "fallback"` 和原样可选的 `contextMaxCharacters`，同样 25 秒但不传自定义 headers（L83-L96）。因此数值 0 或空字符串会被当作未提供并替换默认值；Parallel 完全忽略这四个 Exa 参数。单次调用只选一个分支，没有本地并发、fallback 重试或双提供方竞速。

- 导出的 `WebSearchTool`（L99-L143）通过 `Tool.define("websearch", Effect.gen(...))` 注册。构造阶段从 Effect 环境取得 `HttpClient.HttpClient` 和 `RuntimeFlags.Service`（L101-L104）；工具描述是惰性 getter，每次读取都把 `DESCRIPTION` 中的 `{{year}}` 替换为当前年份（L105-L109）。`execute(params, ctx)` 先以 `ctx.sessionID` 和 `flags.enableExa/enableParallel` 调用选择器，再生成提供方标签，调用 `ctx.metadata` 写入标题 `<label> "<query>"` 与 `{ provider }`（L110-L117）。随后调用 `ctx.ask`，权限名为 `websearch`，patterns 为 query，always 为 `["*"]`，metadata 完整携带五个参数和 provider（L119-L131）；这是网络搜索前的显式授权边界。授权成功后才调用远程提供方（L133）。结果返回对象：`output` 使用远程字符串，若为 nullish 则使用 `No search results found. Please try a different query.`；`title` 为 `<label>: <query>`；metadata 仅保留 provider（L135-L139）。执行 Effect 末尾 `.pipe(Effect.orDie)` 将失败通道转为 defect，而非返回结构化的工具错误（L140）；调用方是否捕获 defect 不在本文件内。

## 状态、取消、恢复与副作用

本文件没有持久化状态、缓存、重试计数、恢复游标或自建并发池。提供方选择只依赖进程环境、flag、session ID 和 checksum；同一会话的默认分桶是稳定的，但环境变量改变会改变所有会话。每次执行依次经历 metadata、ask、一次 MCP HTTP 请求，多个调用之间没有并发语义；并发由外层工具运行时决定。

取消/超时由 Effect 运行时和 `McpWebSearch.call` 承担：本文件把 25 秒时限传下去，但没有显式取消 token、finally 清理或重试。源内 MCP 实现对 HTTP 状态执行 `filterStatusOk`，超时以 defect 结束；因此网络失败、HTTP 非 2xx、Schema 解码失败和权限拒绝都不会在本文件生成备用结果。外部副作用是向 Exa/Parallel 发起带 query、session ID（Parallel）和可能的模型名的 POST；Parallel API key 和可能的 Exa key（由导入模块构造 URL）是凭据边界。metadata/ask 是宿主 UI、审批和审计副作用，但没有本地文件写入。

## 源内测试与行为判据

同目录 `test/tool/websearch.test.ts` 覆盖：同一 session 的选择稳定性（L12-L15）；环境覆盖 Parallel/Exa 优先（L17-L30）；单独 flag 路由（L32-L38）；`webSearchProviderLabel` 三种输出（L48-L52）；Parallel 使用 `model.api.id`（L54-L63）。`test/tool/parameters.test.ts:L276-L280` 验证只给 query 可通过 Schema。该测试还以 `mcp-websearch.ts` 的 `parseResponse` 验证普通 JSON-RPC、SSE data 帧和忽略 `[DONE]`（L66-L99），间接给出 `callProvider` 的响应判据。源文件本身没有直接测试 `WebSearchTool.execute` 的 ask 顺序、metadata、25 秒超时或 nullish 输出回退。

可独立验证的判据：给定固定 session，连续调用选择器结果相等；设置/清除 `OPENCODE_WEBSEARCH_PROVIDER` 后检查覆盖优先级；分别启用两个 flag 检查 Parallel 优先；传入 `{model:{id,api:{id}}}` 检查前 100 字符的 api ID；以缺失可选参数构造 Exa 请求并断言 type/numResults/livecrawl 默认值；用假的 HttpClient/审批实现记录顺序必须为 metadata→ask→HTTP，并检查结果 title、provider metadata 和 nullish fallback。

## zenpi Rust 映射

- `src/tools.rs` 的 `Tool`、`ToolDefinition`、`ToolContext`、`ToolRegistry`、`ToolResult` 是最直接落点：新增 `WebSearchTool` 实现 `definition()` 与 `invoke_cancellable()`，输入 JSON 对应 `Parameters`，输出对应 `ToolResult::Success { output }`。建议新建 `src/tools/websearch.rs`（或当前扁平工具模块中的同名类型）并在注册表注册 `"websearch"`；使用现有 `ToolError::{InvalidArguments, Io, Json, CommandTimeout, Cancelled, Unsupported}` 映射错误，禁止把网络失败变成 panic/defect。
- `src/tool_runtime.rs:L157-L176,L219-L249` 已提供批量顺序执行、取消轮询和结果按 source order 返回。将 websearch 标为顺序工具；`invoke_cancellable` 在请求前、读取响应期间和解析 SSE 每个帧之间检查 `cancelled`。没有源内重试，应保持一次调用一次请求；若未来增加重试，必须在 operation journal 中显式记录。
- `src/approval.rs:L40-L56,L167-L245` 的 `ApprovalRequest`/`ApprovalCoordinator` 对应 `ctx.ask`。为 websearch 生成稳定 call/request ID，把 query 和 provider 放进 arguments/metadata；在 Rust 的 `SideEffectPolicy` 中决定它属于网络只读还是需要显式批准。若沿用现有 ReadOnly 枚举，仍应保留 `websearch` 的 per-tool 审批，以复刻源行为。
- `src/core.rs` 负责把工具调用绑定到 turn、policy 和 `ToolContext`；应从 Agent 当前 session ID、当前模型 provider/id 构造 Parallel 的 `session_id`/model_name，并把 provider 选择结果写入 `AgentEvent::ToolResult` 或 metadata。不要把 websearch 当作 `src/providers/**` 的 LLM wire route。
- `src/runtime.rs:L49-L99` 的 `CancellationToken` 是外层取消边界；网络客户端应使用可中断/短超时请求，并在 token 取消后返回 `ToolError::Cancelled`。运行时 shutdown 只能停止接收结果，不能假设已发出的远程 POST 被回滚。
- `src/session.rs:L45-L96` 的 `OperationKind::Tool`、`OperationOutcome::{Succeeded,Failed,Cancelled,UnknownOutcome}` 可记录一次搜索。远程请求发出而进程中断时不得自动判定成功或隐式重试；按现有 recovery 规则标为 unknown，显式新 operation 才能重试。源工具本身无持久化，因此这是 Rust 为可恢复性增加的宿主层约束。
- `src/headless.rs` 的 JSONL owner/replay/cancel/approval 通道应只传递工具事件和终态；将 query、provider、错误码放入现有事件 payload，遵守请求/响应关联与重放预算。取消命令应触发 token，不能由 headless 层猜测远程请求是否已完成。
- `src/protocol.rs` 当前有版本化 `StdioRequest` 和工具/审批相关载荷，但没有 websearch 专用 schema；新增字段时保持 deny-unknown-fields、大小上限和 v1/v2 兼容，不把 API key 放进协议。至少为参数对象增加 query 非空、字符串枚举和数字范围校验。
- `src/providers/**` 目前是 OpenAI/Anthropic/Google 等模型路由、认证头和能力目录；它们不应承载 Exa/Parallel MCP。可复用其 ureq/HTTP 错误处理和 provider 配置风格，但建议在独立 `websearch` 模块保存 `EXA_URL`、`PARALLEL_URL`、`OPENCODE_WEBSEARCH_PROVIDER`、`PARALLEL_API_KEY`，并对允许的网络 host 做显式白名单。Cargo 已有 `ureq`，可实现 25 秒总超时和 JSON/SSE 解析。
- 可执行验证顺序：注册工具后运行 registry definition/schema 测试；用本地 HTTP mock 断言 Exa/Parallel 的 URL、JSON-RPC method、参数默认值、Authorization/User-Agent；用 `CancellationToken` 在响应读取中取消；模拟 approval deny、HTTP 500、超时、坏 JSON 和进程中断，分别断言 `ToolResult::Error`、session unknown outcome 和不可自动重试。

主要差异：TypeScript 的 Schema 注解默认值不等同于解析默认值；`||` 会吞掉 0/空字符串；`Effect.orDie` 偏向 defect，而 zenpi 应返回结构化错误；源工具每次显式 ask 且网络只调用一次；zenpi 还要求批量边界、取消 token、journal/replay、字节上限和 redaction；源选择器依赖 `checksum` 的 base36 算法，Rust 必须先复刻该算法才能保证分桶一致。

## 未决问题

1. 本文件未给出 `checksum` 的具体算法；无法仅凭本源确认其对任意 session ID 的 base36 输出，Rust 映射必须读取 `@opencode-ai/core/util/encode` 后做跨语言 fixture。
2. `Tool.Context["extra"]` 的完整运行时形状、`RuntimeFlags.enableExa/enableParallel` 的来源及 `ctx.ask` 的拒绝错误类型不在本文件定义。
3. `McpWebSearch.call` 的具体 HTTP/SSE 解析、`EXA_URL` 的 API key 拼接和 Effect 中断细节属于被导入文件，不能由本文件单独证明；已在测试与映射中标明依赖边界。

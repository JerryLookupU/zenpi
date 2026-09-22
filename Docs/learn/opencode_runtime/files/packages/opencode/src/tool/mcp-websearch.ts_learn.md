# OC-044 — packages/opencode/src/tool/mcp-websearch.ts

- source_id/item_id：OC-044
- source_path：packages/opencode/src/tool/mcp-websearch.ts
- source_hash：44cec72f6a8995ff172bc774e4a2e689fd1aa77fb9cfb9575d66a0e4bab24a88
- source_bytes：2998
- source_lines：96
- coverage：已从字节 0 读到字节 2997（完整文件），行范围 L1-L96，包含全部导入、注释/空行、类型模式、常量、函数与导出符号。

## 完整行为复盘

- 模块导入（L1-L2）：依赖 effect 的 Duration、Effect、Schema，以及 effect/unstable/http 的 HttpClient、HttpClientRequest。HTTP 请求、解码和超时都在 Effect 计算中完成。
- EXA_URL（L4-L6，导出）：模块加载时读取 process.env.EXA_API_KEY。有值时生成 https://mcp.exa.ai/mcp?exaApiKey=加上 encodeURIComponent 后的密钥；无值时为 https://mcp.exa.ai/mcp。一次性初始化，运行中修改环境变量不会更新；密钥位于 URL，属于外部请求副作用和潜在日志敏感信息。
- PARALLEL_URL（L7，导出）：固定为 https://search.parallel.ai/mcp，没有配置覆盖或校验。
- McpResult（L9-L18，内部 schema）：要求对象含 result 对象，后者含 content 数组；数组每项必须有字符串 type 与字符串 text。未声明字段不会在此处被业务使用，但结构校验失败会使解码 Effect 失败。
- decode（L20）：Schema.fromJsonString(McpResult) 先把字符串解析为 JSON，再按 McpResult 校验，并包装为 Schema.decodeUnknownEffect。非法 JSON、缺字段、错误类型都走失败通道，没有本地恢复。
- parsePayload(payload: string)（L22-L28，内部函数）：先 trim（L24）。若结果不以字面量 { 开始，立即返回 undefined（L25），因此数组、纯文本、SSE 前缀和其他格式不会继续尝试。否则调用 decode（L26）；解码失败直接传播。成功后在 data.result.content 中按顺序寻找第一个 item.text 为真值的项（L27），返回该文本；空字符串会被跳过，全部为空时返回 undefined。无并发、不访问持久化状态。
- parseResponse(body: string)（L30-L41，导出）：由 Effect.fn("McpWebSearch.parseResponse") 包装为命名 Effect。先对完整响应 trim，非空时把它作为直接 JSON 尝试（L31-L33）；拿到真值文本即返回。若直接对象可解码但没有非空文本，继续逐行扫描原始 body.split("\n")（L35）。只有以精确前缀 "data: " 开始的行才处理（L36），去掉前 6 个字符后再走 parsePayload（L37）；遇到第一个真值文本立即返回（L38）。全部没有命中则返回 undefined（L40）。行前有空格、data: 后无空格或使用 \r 时可能不匹配；直接 JSON 以 { 开头但损坏时会在 L32 失败并终止，不会回退到 SSE；任一被选中的 data: JSON 损坏也会传播解码错误。
- SearchArgs（L43-L49，导出 schema）：要求 query: string、type: string、numResults: number、livecrawl: string；contextMaxCharacters?: number 可省略。没有默认值、范围约束或未知字段策略声明，实际编码由通用 McpRequest 负责。
- ParallelSearchArgs（L51-L56，导出 schema）：要求 objective: string、search_queries: string[]；session_id?: string、model_name?: string 可选。数组元素必须是字符串；没有默认值、长度约束或重试语义。
- McpRequest<F>（L58-L67，内部泛型函数）：接受 Schema.Struct<F>，返回固定 JSON-RPC 请求 schema：jsonrpc 必须是 "2.0"，id 必须是数字字面量 1，method 必须是 "tools/call"；params.name 为字符串，params.arguments 使用传入的参数 schema。请求标识固定，不能为一次调用生成不同请求 ID。
- call<F>(...)（L69-L96，导出泛型函数）：输入为 HttpClient.HttpClient、目标 url、工具名 tool、参数 schema args、已类型化参数值 value、timeout: Duration.Input，以及可选 headers?: Record<string,string>（L69-L76）；输出是一个 Effect，成功值为 parseResponse 的 string | undefined。执行顺序为：
  1. L79-L88 用 HttpClientRequest.post(url) 创建 POST；声明可接受 application/json, text/event-stream（L80），合并自定义头（无头时 {}，L81），用 schemaBodyJson(McpRequest(args)) 编码固定 JSON-RPC 请求体（L82-L87）。schema 编码或 URL/头处理失败会在构造阶段失败。
  2. L89-L93 通过 HttpClient.filterStatusOk(http).execute(request) 发出请求；非 2xx 状态进入 HTTP 错误。随后 Effect.timeoutOrElse 施加调用方提供的 timeout；超时时执行 Effect.die(new Error(工具名 + " request timed out"))（L92），这是 defect/非正常失败路径，不是可继续的重试结果。
  3. L94 读取 response.text；读取失败传播。L95 将完整文本交给 parseResponse，解析成功返回首个非空结果文本，否则返回 undefined；解析 schema 错误传播。
  函数内部没有循环、重试、缓存或并发调度；单次调用是“建请求→HTTP→读正文→解析”的顺序链。多个 call 是否并行完全由调用者组合 Effect 决定。headers 仅覆盖/附加请求头，代码未自动注入认证头；EXA 密钥由 URL 常量携带。

## 状态、取消、恢复与副作用

本文件没有可变业务状态、数据库/会话持久化、checkpoint 或恢复逻辑。唯一模块级读取是加载时读取 EXA_API_KEY（L4-L6）。call 产生真实外部 HTTP POST（L79-L95），可能触发远端 MCP 工具副作用；代码自身不记录请求、响应或工具结果，也不做幂等键管理。Effect 的中断可在请求执行或正文读取处传播，但没有显式 cancellation token 或 finally 清理。超时只在 timeoutOrElse 处转换成带工具名的致命 defect（L92），没有自动重试、退避或降级；取消/超时发生后远端已提交的副作用不保证回滚。无并发控制、限流、熔断和持久化恢复。

## 源内测试与行为判据

源文件及同目录可见内容未包含测试。可独立验证的判据如下：

1. 设定/清除 EXA_API_KEY 后重新加载模块，分别断言 EXA_URL 为带 encodeURIComponent 密钥的 URL 或裸 https://mcp.exa.ai/mcp；PARALLEL_URL 恒为指定字符串。
2. 用 parseResponse 验证：合法直接 MCP JSON 返回首个非空 text；直接 JSON 无文本时扫描 data: 行；非对象、损坏 JSON、错误字段类型产生失败；空文本不被选中；不精确前缀不命中。
3. 用假的 HttpClient 捕获 call 请求，断言 POST、Accept 值、自定义 headers、JSON-RPC 固定字段和参数 schema；模拟非 2xx、正文读取失败、超时，分别观察失败/defect；模拟 SSE 正文断言返回解析文本。
4. 组合两个独立 call 时由测试显式选择并发或顺序，确认本模块没有隐式并行。

## zenpi Rust 映射

- 建议新增 src/mcp_websearch.rs（或放入现有工具模块）承载 ExaUrl/ParallelUrl 常量、McpResult、SearchArgs、ParallelSearchArgs、McpRequest<F> 的 serde 类型，以及 parse_payload、parse_response、call。用 serde/serde_json 做结构校验，用现有 HTTP 客户端抽象（若无统一抽象则在 providers 层补充）发送 POST。
- 对照 src/headless.rs：headless 已有工具生命周期事件、取消传播和外部证据/资源控制入口；MCP 调用应在既有工具工作 lane 中发出并把开始、完成、失败、取消映射到现有事件，而不是在 headless 中直接散落 HTTP。
- 对照 src/core.rs：Agent/ToolRuntime 与 AgentEvent::ToolCall/ToolResult 是注册远端 MCP 工具和回传结果的合适边界。可增加封装工具或 provider-backed handler，把 ToolResult::Success/Error 与 string | undefined 明确转换；保留 schema 校验失败和 HTTP 错误的可诊断错误码。
- 对照 src/session.rs：源 TS 不持久化；zenpi 若将调用纳入 agent 操作，应沿现有 operation/tool journal 记录 dispatch、finish、cancelled 和“副作用未知”状态，以支持恢复检查，不能把取消误当成远端回滚。
- 对照 src/tool_runtime.rs：该文件已有批处理、顺序/并行模式、ToolErrorCode::Cancelled 和可取消执行。将 call 作为单个可取消 handler；批量并行由 tool_runtime 决定，保持 TS 的“本函数无内部并发”语义；超时错误应落到明确的工具错误，而非 Rust panic/abort。
- 对照 src/runtime.rs：使用 CancellationToken 和 runtime job deadline 包裹 HTTP future/阻塞调用；区分“请求取消”“请求超时”“响应解析失败”。runtime 的 cooperative cancellation 不能假设中断远端请求已撤销。
- 对照 src/protocol.rs：若由 headless 客户端配置/调用 MCP，增加受限命令或工具参数载荷的 serde/长度校验；固定 JSON-RPC 字段可在内部构造，避免让 wire 输入任意修改 method 或 id。
- 对照 src/approval.rs：远端 MCP 属于外部副作用候选，应在 dispatch 前生成包含工具名、目标 origin、参数摘要的 ApprovalRequest，遵守现有 ApprovalPolicy；批准结果需和一次调用/operation 关联，不能因 URL 或工具名自动推断信任。
- 对照 src/providers/**：现有 providers/connection.rs 已集中 URL、HTTPS、凭据和允许目的地校验，MCP endpoint 应复用它或新增等价 allowlist；providers/* 目前主要描述模型端点，没有 Exa/Parallel MCP 适配器。建议新增 provider kind/adapter，仅允许配置的 EXA_URL/PARALLEL_URL，将 API key 放入受控 secret/header 或 URL 生成器并避免日志泄露。
- 可执行差异清单：TypeScript Effect schema → Rust serde + 显式错误枚举；Effect.die 超时 → ToolErrorCode::CommandTimeout/专用 McpTimeout；SSE 解析必须保留精确 data: 前缀和首个 truthy 文本规则；固定 id=1 的请求格式需测试；Rust 需补充 URL 安全、响应大小上限、取消竞态和 journal 记录，这些均不是源文件提供的能力。

## 未决问题

1. HttpClient.filterStatusOk 对非 2xx 的具体错误类型和是否保留响应正文，源文件未说明。
2. Duration.Input 的允许单位/零值行为由 effect 版本实现决定，源文件只表明由调用方传入。
3. MCP 服务端是否把 type 字段用于其他语义、是否可能返回多条可用文本，当前实现只取首个非空 text，无法从本文件确认。


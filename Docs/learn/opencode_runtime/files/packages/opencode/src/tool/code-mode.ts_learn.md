# OC-036 — packages/opencode/src/tool/code-mode.ts

- source_id/item_id: `OC-036`
- source_path: `packages/opencode/src/tool/code-mode.ts`
- source_hash: `9a95ddba2ea9ef15c9276ee07cf88bf729b11ade6619ed03c5c8690b46e402aa`
- source_bytes: `11808`
- source_lines: `310`
- coverage: 已从字节 `0-11807`、行 `L1-L310` 按顺序读完，包含导入、注释、类型、常量、内部函数和导出符号。

## 完整行为复盘

文件把一个受限 JavaScript 编排器注册成名为 `execute` 的工具。它把当前权限可见的 MCP 工具投影为 `tools.<server>.<local>`，在沙箱内执行用户脚本，并把子调用状态、文本/结构化结果、附件和日志合并回工具结果。

- `CODE_MODE_TOOL`（L12）是导出常量 `"execute"`；`DESCRIPTION`（L14）是静态描述。`Parameters`（L16-L20）是 Effect `Schema.Struct`，只接受必填字符串 `code`，并为该字段写入脚本用途说明；缺失或非字符串由 schema 解码失败。
- `CallEntry`、`Metadata`、`Attachment`、`CatalogEntry`（L22-L37）定义内部状态：每个子调用有 `running/completed/error`，可带对象输入；元数据还可标记 `error`；目录项同时保留显示路径、扁平 MCP key、server/local 名称和原始 `MCP.McpTool`。
- `groupByServer`（L39-L56）先按 server 名长度降序，再按 MCP key 字典序遍历。对每个 key，优先选最长的 `server + "_"` 前缀；找不到时取第一个下划线前部分，没有下划线则整个 key 作为 server。`local` 去掉该前缀，`path` 形如 `server.local`，结果按 server 分组并保持 key 排序。空工具表返回空 `Map`；同名 server 的项追加。
- `describeCatalog`（L58-L65）把分组目录转换为 `CodeMode.make(...).instructions()`，但给每个条目绑定一个总是 `Effect.fail(toolError("Tool preview is not executable."))` 的运行器，因此只生成说明/签名而不能执行预览。输入 schema、输出 schema、description 原样交给 `toolTree`；目录为空仍能生成基础 instructions。
- `lastSegment`（L67-L71）先截断 `?` 或 `#` 后内容，再去掉末尾斜杠并取最后 `/` 段；空字符串返回 `undefined`。`dataUrl`（L73）无校验地拼接 `data:<mime>;base64,<base64>`。
- `projectMcpResult`（L75-L116）逐个处理 MCP `CallToolResult.content`：`text` 累积换行文本；`image/audio` 转为 `file` attachment，二进制不进入脚本；`resource` 若含 `text` 则进入文本，否则以资源 MIME（默认 `application/octet-stream`）转 data URL，并用 `lastSegment(uri)` 设 filename；`resource_link` 只变成 `name: uri` 文本，因为链接不可直接抓取。先返回非空 `structuredContent`，否则返回拼接文本；无文本但有文件返回 `[N image(s)/file(s) attached to the result]`，文件数等于图片数才用 image；全空返回 `null`。每次文件都会先计数再调用传入的 `collect`，所以附件即使最终脚本返回别的值也会累积。
- `Run`（L118）是未知输入到 Effect 结果的函数类型。`toolTree`（L120-L131）为每个目录项建立 `SandboxTool.make`：description 缺省为空串，输入 schema 强制转换为 `JsonSchema`，输出 schema 可选，运行器由调用方提供；server/local 命名空间通过对象键构造。目录项的 server/local 名称因此直接影响沙箱 API。
- `invokeChildTool`（L134-L186）是带 span 的 Effect 函数。先触发插件 `tool.execute.before`，事件中使用扁平 MCP key、sessionID、子 callID；然后调用 `ctx.ask`，权限名为扁平 key，`patterns/always` 都是 `["*"]`。MCP SDK `callTool` 传原始工具名和 args，开启 `resetTimeoutOnProgress`、`ctx.abort` signal、工具自身 timeout，并提供空 `onprogress` 以获得进度 token。若 `raw.isError`，只抽取非空文本并抛出 `Error`，没有文本时用 `MCP tool returned an error`；否则返回 schema 校验后的 `CallToolResult`。span 属性含工具名、call id、session id、message id。成功后触发 `tool.execute.after` 并返回结果；before、ask、transport、after 任一失败都会使 Effect 失败，after 不会在失败 transport 上执行。
- `CodeModeTool`（L188-L310）通过 `Tool.define` 注册。构建时注入 `MCP.Service`、`Agent.Service`、`Session.Service`、`Plugin.Service`（L190-L195）。其 `execute`（L199-L307）行为如下：
  1. 若 `ctx.abort.aborted`，立即返回标题 `execute`、空 `toolCalls`、`error: true` 和 `Execution cancelled.`（L200-L205），不读取 agent/session，也不调用 MCP。
  2. 读取 agent、session；session 获取用 `Effect.orDie`。合并 agent/session 权限，调用 `Permission.visibleTools` 隐藏硬拒绝工具；从 MCP client 名称经 `McpCatalog.sanitize` 得到 server 列表，再以 `groupByServer` 展平目录（L207-L213）。
  3. 建立 `calls`、`attachments`；`publish` 对当前 calls 做浅拷贝并经 `ctx.metadata` 发布（L214-L218）。`childCalls` 从 0 开始；每次 `callTool` 递增并生成 `${ctx.callID ?? entry.key}/${childCalls}`，缺省输入按 `{}` 处理，调用 `invokeChildTool` 后投影结果并追加附件（L219-L230）。`Effect.catchCause` 将纯中断保持为 `Effect.interrupt`，其他 Cause 压扁成 `toolError`，因此脚本可捕获普通子调用错误。
  4. 创建 `CodeMode` runtime。`onToolCallStart`（L241-L253）把 `null/undefined` 视为无 input；非数组对象仅在非空时记录原对象；其他值包成 `{input}`，并以 index 写入 `running` 后发布。`onToolCallEnd`（L254-L259）若 index 仍有记录，则按 `outcome === "success"` 改为 `completed`，否则 `error`，再发布。并发脚本（例如 `Promise.all`）由解释器决定，多个回调可能交错；index 保证每条状态关联对应调用。
  5. `abort` Effect（L262-L267）在 signal 已中止时立即完成，否则注册一次性 abort listener，并在资源释放时移除。`cancelled` 结果（L268-L272）固定为 `ok:false`、`ExecutionFailure`、并保留已见子调用名称。
  6. `Effect.raceFirst(runtime.execute(params.code), abort.pipe(...))`（L274）使脚本执行与取消竞争；取消获胜不等待解释器正常返回。读取 `result.logs`，`withLogs`（L275-L279）只在有日志时追加 `Logs:` 段，错误和成功路径都保留日志。
  7. 失败结果（L281-L290）：若此时 signal 已中止，返回取消结果和当前 calls；否则去重掉已包含在主错误消息中的 suggestions，并 `Effect.fail(new Error(...))`。成功结果（L293-L305）中字符串原样输出，其他值 `JSON.stringify(value, null, 2)`；若 stringify 返回 `undefined` 则退回 `String(value)`。返回标题、calls 元数据、带日志输出，以及非空 attachments。最外层 `Effect.orDie`（L306）把未恢复的执行错误提升为 defect。

## 状态、取消、恢复与副作用

状态只在一次 execute 调用内维护：`calls` 是顺序索引数组，`attachments` 是所有成功投影的文件累积，`childCalls` 是合成子调用 ID 的单调计数器；没有跨请求持久化、重试计数或恢复快照。`ctx.metadata` 是外部进度副作用，开始和结束各发布一次（异常/取消可能留下 `running`）；`Plugin.trigger` before/after 是插件副作用。真正外部副作用来自 MCP `client.callTool`，每个调用先经过 `ctx.ask` 权限检查；`Permission.visibleTools` 在目录生成前屏蔽硬拒绝工具。

取消是协作式而非强杀：初始已中止直接短路；执行中通过 `AbortSignal` 竞争并传给 MCP transport。`raceFirst` 只保证 execute 工具尽快返回取消结果，已发出的 MCP 请求可能仍在远端运行；源码没有自动重试，MCP timeout 由 `entry.tool.timeout`，进度回调会重置超时。中断 Cause 原样中断，普通错误转成可被脚本 `try/catch` 捕获的 `toolError`；顶层未捕获错误变成失败工具调用。没有 session、文件或数据库写入，附件仅随当前 `Tool.ExecuteResult` 返回。

## 源内测试与行为判据

源文件本身未包含测试；同目录测试明确覆盖行为：`test/tool/code-mode.test.ts` L97-L137 验证参数 schema、最长 server 前缀和原始 schema；L195-L250 验证大目录的 partial/search、`$codemode.search` 和元数据；L252-L337 验证普通返回、命名空间、结构化结果及 `Promise.all` 并发；L339-L392 验证脚本错误、未知工具、MCP 错误和逐调用权限；L394-L430 验证插件 hook 与合成 call id；L647-L729 验证日志、硬拒绝隐藏、ask 权限可见性。真实 MCP 集成测试 `test/tool/code-mode-integration.test.ts` L176-L251 验证真实 transport、结构化结果、图片 data URL/attachment 和并发附件，L254-L330 验证 `isError`、日志以及 running/completed 元数据。可独立验证判据：输入 `{code:"return 1+2"}` 输出 `"3"`；一个 `image/png` content 产生一个 `type:file` attachment 且脚本只看到 `[1 image attached to the result]`；被硬拒绝的扁平 key 不出现在目录且调用得到 `Unknown tool`；`Promise.all` 两个调用最终均为 completed。

## zenpi Rust 映射

- **工具执行落点**：`src/tool_runtime.rs` L157-L176 的 `execute_tool_batch` 已有批量、顺序/并行、调用前 `prepare`、结果按源顺序归并、取消轮询和 panic 转内部错误；可新增 `code_mode.rs`，将脚本调用编译为 `ToolCall`，把 `toolTree` 的 namespace/path 映射为 `ToolRegistry` 名称。其 L177-L217 的批量上限和全批次无副作用校验可作为脚本批量边界；不要绕过 L349-L425 的 registry policy recheck。
- **取消与运行时**：`src/runtime.rs` L49-L99 的 `CancellationToken`、L308-L350 的 `try_cancel/try_shutdown_with_grace`、L640-L653 的 outcome 分类对应 `AbortSignal + raceFirst`。建议 `CodeModeJob` 在脚本解释器和每次 MCP/工具调用前后检查 token；把取消映射为 `ToolErrorCode::Cancelled`，遵循 L157-L165 所述“取消不等于回滚”，不可安全终止的远端调用需留下不确定结果。
- **协议/流式元数据**：`src/protocol.rs` L152-L194 的 `StdioRequest`、L795-L1003 的解析/响应编码可承载 `execute` 请求和终端结果；建议新增事件类型 `tool_call_start/tool_call_end`，携带 `tool`, `status`, 可选 JSON input，与 TS 的 `ctx.metadata` 对齐。已有 `cancel` 命令和 schema 版本校验可承接 abort；需明确脚本输出、日志、attachments 的 JSON 字段上限。
- **审批**：`src/approval.rs` L41-L104 的 `ApprovalRequest::validate` 和 L167-L245 的 `ApprovalCoordinator` 对应每个子调用先 `ctx.ask`；用扁平 MCP key 作为 `tool`、脚本父 turn/call 作为 `turn_id/call_id`。`persist_accepted`（L370-L390）要求批准先持久化再执行，强于 TS 当前仅调用 ask；`cancel_all/emergency_cancel`（L409-L446）可覆盖等待中的审批。必须保留 hard deny 优先级，不能把脚本可见性当作执行授权。
- **会话/恢复**：`src/session.rs` L1410-L1590 的 operation begin/finish/recovery 明确要求中断操作新 operation ID、未知结果不能隐式重试；可把每个有副作用的 MCP 子调用记录为 operation，`attachments`/日志写入 terminal record。TS 当前没有持久化，Rust 映射若启用恢复会改变语义，应在协议中标记 `unknown outcome` 并要求人工确认。
- **headless 与外部输出**：`src/headless.rs` L33-L78 的有界事件/终端预算、L334-L380 的 request admission/replay 可限制脚本日志、子调用事件和大附件；L266-L282 的 WAL 机制适合持久化终端结果，但不能把 data URL 无界写入 replay。`src/headless.rs` 的 stdin EOF/显式 shutdown 区分可对应脚本自然完成和取消。
- **providers**：`src/providers/**` 目前是 Anthropic/OpenAI/Codex/Google/DeepSeek 路由与 capability registry，没有 MCP client/catalog。建议新增 `src/mcp.rs`（`McpTool`, `McpClient`, `McpCatalog`, `CallToolResult` 投影）并由 `src/providers/connection.rs` 的 capability `tools/structured_output`（现有 provider route 约束）决定是否暴露 code mode；不要把 MCP transport 逻辑塞入 provider encoder。
- **建议可验证差异**：TS 使用 Effect sandbox、动态 `tools.<namespace>`、结构化内容优先、图片/音频转附件、每次调用 timeout/progress reset、取消 race 且无 durable retry；zenpi 需补脚本解释器或受限 DSL、MCP transport、`project_mcp_result` 等价投影、日志/附件预算、子调用状态事件和 operation journal。验证顺序应是：目录分组/权限过滤单测 → 文本/structured/image/error 投影单测 → 并行调用与取消集成测 → 审批持久化和 unknown outcome 恢复测。

## 未决问题

1. `@opencode-ai/codemode` 解释器的资源限制（CPU、内存、脚本最大时长、`$codemode.search` 的完整实现）不在本源文件内，无法从本文件确定。
2. `McpCatalog.sanitize` 的具体改名规则、`Permission.visibleTools` 对非硬拒绝规则的完整优先级来自外部模块；本文件只能确认调用位置和输入输出。
3. `Tool.Context.ask` 的 UI/协议实现及 `ctx.metadata` 是否持久化不在本文件内；这里只能确认它们被逐子调用触发/发布。

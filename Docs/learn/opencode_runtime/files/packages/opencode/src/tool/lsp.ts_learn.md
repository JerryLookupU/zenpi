# OC-043 — packages/opencode/src/tool/lsp.ts

## 元信息

- `source_id/item_id`: `OC-043`
- `source_path`: `packages/opencode/src/tool/lsp.ts`
- `source_hash`: `993e1316f12a44cde46eb8af34868a1ea2077a6e4ec749c25ec8b913a2e46da5`
- `source_bytes`: `4329`
- `source_lines`: `113`
- `coverage`: 已顺序读取完整文件，字节范围 `0-4328`（全量 4329 字节），行范围 `L1-L113`（113/113）。

## 完整行为复盘

文件导入 `Effect`、`Schema`、`Tool`、`LSP`、`InstanceState`、`FSUtil` 及路径/外部目录辅助函数（`L1-L9`）。该工具是 Effect 服务包装器，不直接创建 LSP 进程；真正的客户端生命周期由 `LSP.Service` 提供。

- `operations`（`L11-L21`）是 `as const` 的九项白名单：`goToDefinition`、`findReferences`、`hover`、`documentSymbol`、`workspaceSymbol`、`goToImplementation`、`prepareCallHierarchy`、`incomingCalls`、`outgoingCalls`。它同时驱动 Schema 字面量校验和后续 `switch`，因此未知操作在进入执行函数前即被拒绝。
- `Parameters`（`L23-L35`）要求 `operation`、`filePath`、`line`、`character`；`line` 与 `character` 必须是整数且 `>=1`，语义是编辑器展示的一基坐标。`query` 可选字符串，仅用于 `workspaceSymbol`；注释明确空字符串代表请求全部符号。即使 `documentSymbol`/`workspaceSymbol` 不使用游标，Schema 仍要求这两个字段，调用方不能省略。
- `LspTool` 导出定义（`L37-L43`）名称为 `lsp`，描述来自 `./lsp.txt`，参数为 `Parameters`。构造器通过 Effect 环境取得 `LSP.Service` 与 `FSUtil.Service`（`L39-L42`）；缺少任一服务属于环境错误。
- `execute(args, ctx)`（`L45-L110`）按单次调用顺序执行，没有 `fork` 或并行分支：
  1. 从 `InstanceState.context` 取当前实例目录（`L47`）。绝对 `filePath` 原样使用；相对路径以 `instance.directory` 为基准拼接（`L48`）。随后调用 `assertExternalDirectoryEffect(ctx, file)`（`L49`），先执行工作区外路径保护。
  2. 构造审批元数据 `meta`（`L50-L55`）：`workspaceSymbol` 只放 `operation`；`documentSymbol` 放 `operation` 与解析后的绝对 `filePath`；其余操作再带原始一基 `line`、`character`。通过 `ctx.ask` 请求权限 `lsp`，`patterns` 与 `always` 均为 `['*']`（`L56-L61`），因此权限检查发生在文件存在性和 LSP 可用性检查之前；拒绝/中断会使后续步骤不执行。
  3. 将文件转为 `pathToFileURL(file).href`（`L63`）。建立给位置型请求使用的 `position`：保留绝对 `file`，并把行、字符各减一转换为 LSP 常见的零基坐标（`L64`）。以 `instance.worktree` 计算相对路径 `relPath`（`L65`）。标题 `title`（`L66-L72`）规则为：`workspaceSymbol` 仅操作名；`documentSymbol` 为 `operation + ' ' + relPath`；其他为 `operation + ' ' + relPath + ':' + 原始line + ':' + 原始character`。
  4. `fs.existsSafe(file)` 检查目标存在（`L74`）；假值抛出 `File not found: ${file}`（`L75`）。再调用 `lsp.hasClients(file)`（`L77`）；假值抛出 `No LSP server available for this file type.`（`L78`）。这说明 `filePath` 对 `workspaceSymbol` 虽不进入 `workspace/symbol` 请求，仍用于选择/启动匹配语言服务器。调用 `lsp.touchFile(file, 'document')`（`L80`）在查询前确保文档已被 LSP 客户端接触/同步。
  5. `switch`（`L82-L103`）按操作一一映射：`goToDefinition`→`lsp.definition(position)`（`L84-L85`）；`findReferences`→`lsp.references(position)`（`L86-L87`）；`hover`→`lsp.hover(position)`（`L88-L89`）；`documentSymbol`→`lsp.documentSymbol(uri)`（`L90-L91`）；`workspaceSymbol`→`lsp.workspaceSymbol(args.query ?? '')`（`L92-L93`，缺省查询严格变成空字符串）；`goToImplementation`→`lsp.implementation(position)`（`L94-L95`）；`prepareCallHierarchy`→`lsp.prepareCallHierarchy(position)`（`L96-L97`）；`incomingCalls`→`lsp.incomingCalls(position)`（`L98-L99`）；`outgoingCalls`→`lsp.outgoingCalls(position)`（`L100-L101`）。结果统一收集为 `unknown[]`，所以本文件不规范化 LSP 返回对象。
  6. 成功返回 `{title, metadata: {result}, output}`（`L105-L109`）。空数组输出 `No results found for ${operation}`；非空数组以 `JSON.stringify(result, null, 2)` 美化输出。`metadata.result` 保留原数组供程序消费。
- 执行 Effect 在 `L110` 通过 `Effect.orDie` 包装：服务失败、显式 `throw` 的文件/服务器错误会被提升为不可恢复 defect，而不是在此转换成结构化工具错误。文件没有显式重试、超时或结果截断逻辑。

## 状态、取消、恢复与副作用

该文件只维护一次调用的局部值（解析路径、位置、标题、结果），没有跨调用状态、缓存或持久化（`L45-L109`）。外部副作用依次是权限请求 `ctx.ask`（`L56-L61`）、可能触发客户端启动/文档同步的 `lsp.touchFile`（`L80`）以及各 LSP 查询的进程/IPC 通信；没有写入工作区文件的代码。`fs.existsSafe` 是只读检查。

没有本地超时和重试；`Effect` 的取消依赖运行时中断语义以及底层 `LSP.Service` 是否合作，`ctx.abort` 仅存在于上下文类型中，本文件没有主动轮询或调用它。取消发生在 `ctx.ask` 或任意 LSP Effect 等待期间时，后续顺序步骤不会开始；已经发出的 `touchFile`/请求不保证回滚。不存在崩溃恢复、幂等键或结果持久化，因此重试只能由上层重新发起整次工具调用，并可能再次请求权限、触碰文档和发起 LSP 请求。

## 源内测试与行为判据

源文件本身未包含测试；同项目 `test/tool/lsp.test.ts` 覆盖了可核对判据：位置型操作的权限元数据含绝对 `filePath`、一基 `line`/`character`，且标题如 `goToDefinition test.ts:3:7`（测试 `L99-L120`）；`documentSymbol` 元数据省略游标且标题为相对路径（`L123-L142`）；`workspaceSymbol` 元数据省略文件和游标、标题仅为 `workspaceSymbol`（`L145-L165`）；查询参数原样传递，省略时传空字符串，断言为 `['TestSymbol', '']`（`L167-L181`）。`test/tool/parameters.test.ts` 另验证 `line=0`、`character=0`、未知 operation 均被 Schema 拒绝（`L176-L190`）。可独立验证时应再检查：相对路径基于 `InstanceState.directory`，`workspaceSymbol` 仍先执行 `hasClients(file)`，不存在文件和无客户端分别得到两条精确错误文本，空结果得到精确的 `No results found...` 输出。

## zenpi Rust 映射

- `src/tools.rs`：建议新增只读 `LspTool` 实现，或拆出 `src/lsp.rs` 后由 `ToolRegistry` 注册。定义 `LspOperation`（九个枚举变体）、`LspRequest { operation, file_path, line, character, query }` 与 `LspResult { title, result: serde_json::Value, output }`；在 `ToolDefinition.input_schema` 保留 `line/character` 最小值 1，并将 `side_effect` 设为 `ToolSideEffect::ReadOnly`。`ToolContext` 的路径解析可承接工作区边界，但需额外允许“用于选服务器的存在文件”语义。
- `src/core.rs`：在工具预检/执行路径接入 LSP，复用现有 `ToolCall`、`ToolRegistry`、`ApprovalRequest` 和 `ToolExecutionEvidence`。差异是 TypeScript 每次都调用 `ctx.ask(permission='lsp')`，而 zenpi 的 `ApprovalPolicy` 可能自动放行只读工具；若要求严格一致，应新增 `lsp` 专用权限类别或在核心中强制产生可审计的只读审批记录。
- `src/tool_runtime.rs`：LSP 查询是单个有序调用，建议登记为 `ToolExecutionMode::Sequential`，通过 `execute_tool_batch` 的 `prepare` 阶段完成路径/权限/服务器可用性检查；取消时返回 `ToolErrorCode::Cancelled`，不要把“已发出请求”误判为可回滚。
- `src/runtime.rs`：若 LSP 客户端为长生命周期后台进程，可由 `BackgroundRunner` 承载启动与请求队列，使用 `CancellationToken` 在请求前、响应后和重试前检查；当前源文件没有重试，Rust 初版应保持一次提交一次终态。
- `src/protocol.rs` 与 `src/headless.rs`：若要暴露 JSONL 接口，增加受限的 `lsp` 工具调用或 `Command` 分支，复用现有 ID、路径长度和错误编码；`headless.rs` 的重放/终端缓存可记录最终结果，但必须明确这是 zenpi 扩展，因为源实现没有持久化。路径解析和 `file not found`/`no server` 应在命令进入 provider 前完成。
- `src/session.rs`：源实现无会话记录。若 zenpi 需要审计，使用 `OperationKind::Tool` 的 `begin_operation_with_key`/`finish_operation` 包围一次 LSP 请求；崩溃后按现有 `UnknownOutcome` 规则要求显式恢复决策，不能自动重放可能已抵达语言服务器的请求。
- `src/approval.rs`：映射 `ctx.ask` 的权限请求、`patterns=['*']` 和 `always=['*']` 到 `ApprovalRequest` 的工具名/参数/预览字段；保持只读副作用分类，且取消时清除 pending 请求。若采用默认只读自动批准，要在行为判据中记录与 opencode 的差异。
- `src/providers/**`：`anthropic.rs`、`openai.rs`、`google.rs`、`deepseek.rs`、`codex.rs` 及 `connection.rs` 只负责模型 provider 协议，不能把 LSP 操作编码成 provider 请求。应新增独立本地 LSP client/transport（例如 `src/lsp.rs`），由工具层调用；provider registry 仅继续提供模型能力，不参与 `workspaceSymbol` 等请求。

建议的可执行验证顺序是：先实现九项枚举和 Schema 约束，再加入工作区外路径、文件存在、客户端可用性三道门；用假的 `LspService` 断言九个方法的参数和顺序，测试 `line/character` 的 1→0 转换、`query.unwrap_or_default()`、标题/空结果文本，最后用 `CancellationToken` 和会话故障注入验证取消及未知结果不自动重试。

## 未决问题

1. `LSP.Service` 各方法的具体返回类型、服务器选择算法、`touchFile` 是否启动进程，无法仅由本文件确认。
2. `Effect.orDie` 在当前宿主中如何被上层序列化为工具错误，需结合 Effect 运行入口验证。
3. `assertExternalDirectoryEffect` 是否解析符号链接、是否允许工作树外的只读路径，源文件只展示调用点，具体规则需查其实现。

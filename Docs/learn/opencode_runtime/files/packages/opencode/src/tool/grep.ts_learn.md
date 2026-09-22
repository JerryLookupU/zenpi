# OC-040 — packages/opencode/src/tool/grep.ts

## 元信息

- source_id/item_id: `OC-040`
- source_path: `packages/opencode/src/tool/grep.ts`
- source_hash: `f7bbab8ae3fabfe3dd78b5728a283a56bff318d5b1f278d15d1c637227ff660f`
- source_bytes: `4072`
- source_lines: `115`
- coverage: 已从字节 `0..4072`、行 `L1-L115` 按顺序读取，包含全部 import、注释、Schema、导出符号和闭包实现；哈希与任务给定值一致。

## 完整行为复盘

1. **模块依赖与导出参数 Schema（L1-L18）**  
   `path`、`Effect`、`Schema`、`InstanceState`、`FSUtil`、`Ripgrep`、`assertExternalDirectoryEffect`、描述文本 `DESCRIPTION` 与 `Tool` 被导入（L1-L8）。导出 `Parameters` 是一个结构体 Schema（L10-L18）：`pattern` 为必填字符串，描述为“在文件内容中搜索的正则表达式”；`path` 为可选字符串，语义是搜索目录，默认当前工作目录；`include` 为可选字符串，语义是文件过滤模式，例如 `*.js` 或 `*.{ts,tsx}`（L10-L17）。Schema 层只声明类型和说明，未在此处给出长度、正则可编译性或路径存在性约束。

2. **导出 `GrepTool` 定义（L20-L27）**  
   `GrepTool = Tool.define("grep", Effect.gen(...))` 注册工具名 `grep`（L20-L22）。构造时从 Effect 环境取得 `FSUtil.Service` 与 `Ripgrep.Service`（L23-L24），返回包含 `description: DESCRIPTION`、`parameters: Parameters` 和 `execute` 函数的定义对象（L25-L28）。服务获取发生在工具定义效果执行时；没有本地缓存或初始化副作用的代码。

3. **`execute` 输入、空结果和必填检查（L28-L37）**  
   `execute` 接受 `{ pattern: string; path?: string; include?: string }` 与 `Tool.Context`，返回一个 `Effect`（L28-L29）。每次调用先构造 `empty`：`title` 为原始 `params.pattern`，`metadata.matches` 为 `0`、`metadata.truncated` 为 `false`，`output` 为 `"No files found"`（L30-L34）。随后用 `if (!params.pattern)` 检查空字符串、`undefined`（若上游绕过类型）等假值；失败时抛出 `Error("pattern is required")`（L35-L37）。空白字符串不会被此检查拒绝，会继续交给 ripgrep。

4. **权限询问顺序与内容（L39-L48）**  
   通过 `ctx.ask` 请求 `permission: "grep"`，`patterns` 只包含当前正则，`always` 为 `["*"]`，表示该权限可被记忆为全局模式（L39-L42）。`metadata` 原样记录 `pattern`、`path`、`include`，包括可选字段的 `undefined`（L43-L47）。权限询问在路径解析和文件系统访问之前，因此拒绝会短路搜索，不触碰后续 `stat` 或 ripgrep。

5. **工作目录、相对路径和外部目录边界（L50-L59）**  
   从 `InstanceState.context` 取得实例状态（L50）。`requested` 的规则是：当 `params.path`（或实例 `ins.directory`）是绝对路径时直接使用；否则以 `ins.directory` 为基准将 `params.path`（缺省时 `"."`）拼接（L51-L53）。先对 `requested` 做 `fs.stat`，异常被吞掉并转成 `undefined`，因此不存在、无权限等情况不会在这里直接失败（L54）。随后调用 `assertExternalDirectoryEffect(ctx, requested, { bypass:false, kind: ... })`；已知类型为目录则声明 `directory`，否则声明 `file`，未知类型也按 `file` 进入边界检查（L55-L58）。该断言是允许/拒绝工作区外路径的安全闸门，且本工具明确不绕过（L55-L58）。

6. **ripgrep 工作根、搜索调用和上限（L60-L69）**  
   `FSUtil.resolve(requested)` 产生用于搜索的解析路径 `search`（L60）。再次 `fs.stat(search)`，错误转为 `undefined`；若其类型是目录则 `cwd = search`，否则使用 `path.dirname(search)`（L61-L63）。因此传入文件路径时，ripgrep 的当前目录是其父目录，而不是文件本身；实际文件筛选依赖 `Ripgrep.Service.grep` 的实现。调用参数为 `cwd`、原始 `pattern`、可选 `include` 和固定 `limit: 100`（L63-L68）。结果数组为空时立即返回前述 `empty`（L69）。没有显式传递取消令牌、超时或排序选项。

7. **结果投影、绝对路径和截断判定（L71-L86）**  
   每个 ripgrep 项目被映射成 `{ path, line, text }`（L71-L78）。路径使用 `path.resolve`：若最初 `requestedInfo` 的类型为目录，以 `requested` 为基准；否则以 `path.dirname(requested)` 为基准，再拼接 `item.entry.path`（L72-L75）。这使输出路径绝对化，并保留文件输入时的父目录语义。`line` 和 `text` 完全来自 ripgrep（L76-L77）。再次固定 `limit = 100`，`truncated = rows.length === limit`，`final = rows`（L80-L82）；由于 `rows` 刚由结果生成，`final.length===0` 只在未来修改映射逻辑时有意义，当前实现不会在非空 `result` 下命中（L83）。`total` 是当前返回行数；`hasMore` 为 `truncated || result.length === limit`，所以服务恰好返回 100 条时会提示可能还有更多，即使映射后的行数异常变化（L85-L87）。

8. **文本输出格式（L87-L111）**  
   输出首行是 `Found N matches`，若 `hasMore` 为真则追加 ` (more matches available)`（L87-L88）。遍历 `final`，按连续路径分组：路径变化时（首个路径除外）先插入空行，再插入 `absolute/path:` 标题（L89-L95）；每行匹配格式为两个空格、`Line <line>: <text>`（L96）。只有 `truncated` 为真时，末尾再加入空行和 `(Results truncated. Consider using a more specific path or pattern.)`（L99-L102）。最终返回 `title` 为 pattern，`metadata.matches` 为 total、`metadata.truncated` 为 truncated，`output` 为以换行连接的字符串（L104-L111）。输出没有单独的文件计数、搜索耗时或原始 ripgrep 错误字段。

9. **错误边界与并发语义（L112-L115）**  
   内层 Effect 通过 `.pipe(Effect.orDie)` 处理（L112）：Effect 失败会转为不可恢复的 defect，而不是在此函数返回一个带错误值的 Result。外层 `Effect.gen` 的依赖和内层步骤严格按书写顺序执行：取服务、校验 pattern、询问权限、两次 `stat`、断言目录、ripgrep、映射输出；没有 `Effect.all`、`fork` 或显式并发。工具注册器是否并行调度多个只读工具由宿主决定，本文件单次调用本身是串行的；`Ripgrep.Service.grep` 内部并发/线程数不由本文件控制。`limit` 固定为 100，无法由调用者调大。

## 状态、取消、恢复与副作用

本文件只有调用局部状态：`empty`、`requested`、`requestedInfo`、`search`、`info`、`cwd`、`result`、`rows`、`limit`、`truncated`、`current` 和 `output`（L30-L110）；没有跨调用缓存、全局可变状态或持久化句柄。`fs.stat` 失败被转成 `undefined`，但后续 `FSUtil.resolve`、外部目录断言或 ripgrep 仍可能失败（L54-L63）。未实现显式取消检查、超时、重试、断点续搜或恢复；取消只能由 Effect 运行时/下层 `Ripgrep.Service` 间接实现，源码没有传递 cancellation signal（L60-L68）。搜索是读取副作用：两次 `stat`、路径解析和 ripgrep 读取文件；没有写文件、网络、进程启动或 session 持久化代码。`ctx.ask` 是唯一明显的外部交互，并可能持久化 “always” 授权，具体持久化由 `Tool.Context` 实现负责（L39-L47）。`Effect.orDie` 使失败进入运行时 defect 通道，调用者不能从本文件获得结构化重试建议（L112）。

## 源内测试与行为判据

源目录中只有 `grep.ts` 和 `grep.txt`，未包含 `grep` 专用测试文件。可独立验证的判据如下：

- 给定非空 `pattern` 且权限允许，`Parameters` 应接受字符串 `pattern`，`path`/`include` 缺省合法；空 pattern 应得到精确错误 `"pattern is required"`（L10-L18、L35-L37）。
- 权限请求必须先于文件系统访问，权限参数中的 `patterns[0]`、`metadata` 应与输入一致，`always` 必须为 `["*"]`（L39-L48）。
- 相对 `path` 应相对 `InstanceState.context.directory` 解析，绝对 `path` 应原样使用；外部目录断言的 `bypass` 必须为 `false`（L50-L58）。
- stub `Ripgrep.Service.grep` 返回空数组时，输出应为 `title=pattern`、`matches=0`、`truncated=false` 和 `"No files found"`（L30-L34、L69）。
- 返回 1 条结果时应生成绝对路径标题与 `Line n: text`；返回恰好 100 条时 metadata.truncated 为真且输出含“more matches available”和截断提示（L63-L111）。
- 模拟 `stat` 拒绝可确认其被吞掉为 `undefined`，但外部目录断言仍被调用；模拟断言拒绝或 ripgrep 失败可确认 `.orDie` 将失败升级为 defect（L54-L59、L112）。

## zenpi Rust 映射

建议把该工具映射为 `src/tools.rs` 中的只读内建 `GrepTool`（或将现有 `SearchTextTool` 拆出兼容别名），而不是修改 provider 层。现有 `ToolDefinition` 要求名称、说明、对象 Schema 并校验大小（`src/tools.rs:L502-L535`）；`ToolRegistry::with_read_only_builtins` 已注册 `SearchTextTool`/`FindFilesTool`（`src/tools.rs:L1075-L1082`），可在同一处注册 `GrepTool`。建议落点与差异如下：

- **类型/Schema**：新增 `GrepTool`、`GrepArguments { pattern: String, path: Option<String>, include: Option<String> }` 或直接沿用 JSON `ToolCall`。Schema 名称可取 `"grep"`，`pattern` 必填，`path` 默认 `"."`，`include` 为 glob；保留 `ToolSideEffect::ReadOnly`。与 opencode 的正则默认语义一致时，不能直接把现有 `search_text` 的默认字面量语义当作等价物（`src/tools.rs:L1711-L1732`）。
- **执行函数**：实现 `Tool::execution_mode -> Parallel`、`invoke_cancellable`，先调用 `context.admit_builtin`、拒绝未知字段，再做非空 pattern 检查；路径用 `ToolContext::resolve_existing` 限定在 canonical workspace（`src/tools.rs:L745-L777` 及现有 `SearchTextTool` 的校验路径 `L1743-L1777`）。`resolve_existing` 对“不存在”通常直接返回 `NotFound`，与 opencode 先吞掉 `stat` 错误的宽松行为不同，应在兼容层明确是否保留该差异。
- **ripgrep 后端**：优先复用 `src/search.rs:SearchBackend`。该后端发现宿主提供的绝对 `rg`、有 10 秒 deadline、取消轮询、输出/文件/深度上限并清理子进程（`src/search.rs:L1-L31`、`L89-L190`、`L386-L520`）；它目前返回结构化 `matches/context/truncated`，而 opencode 要求最多 100 行、绝对路径和文本分组输出。可增加一个 `grep_regex` 适配函数，把 `SearchOptions { regex:true, glob:include, max_matches:100 }` 的 matches 映射为 `{path,line,text}`，再生成 opencode 兼容的 `output` 字符串。
- **并发与取消**：把 `invoke_cancellable` 的 `cancelled` 闭包传入 `SearchBackend::search`，避免 opencode 中未显式暴露的取消缺口；`tool_runtime.rs` 已在批处理前后及 worker 线程中传播取消，并确保线程 join（`src/tool_runtime.rs:L221-L249`、`L252-L347`）。该实现比源文件的单次 Effect 串行语义更强，但可验证且不会引入悬挂 ripgrep。
- **权限/外部目录**：opencode 的 `ctx.ask(permission:"grep")` 应映射到 `src/approval.rs` 的 `ApprovalRequest`/`ApprovalCoordinator`；只读调用可由 `ApprovalPolicy::ReadOnly` 自动放行，若产品要求每次询问则使用 `request` 并记录 `tool="grep"`（`src/approval.rs:L40-L56`、`L167-L201`）。workspace 外路径由 `ToolContext::resolve_existing`/Blueprint gate 拒绝；不要实现 `bypass:true`，对应源中的 `bypass:false`（L55-L58）。
- **结果与错误**：`ToolResult::Success { output: Value }` / `ToolResult::Error { ToolFailure }` 是 Rust 的结构化替代（`src/tools.rs:L652-L741` 附近）；不要照搬 `Effect.orDie` 的 panic/defect，应把空 pattern、无 ripgrep、正则错误、路径越界、超时、取消分别编码为 `ToolErrorCode`。保留 `matches`、`truncated`、`output` 三字段可让 headless 协议稳定消费。
- **运行时/协议/持久化**：`src/core.rs` 的 `ToolRuntime` 负责把 `ToolCall` 送入工具并产出 `AgentEvent::ToolResult`；`src/runtime.rs` 的 `BackgroundRunner` 可承载取消和 join；`src/protocol.rs` 的 `StdioRequest/Command` 目前没有专门 grep 命令，建议把 grep 作为普通 tool call 或增加严格的 `Command::Tool` 变体，而不要让 provider 文本触发文件搜索。`src/session.rs` 的 append-only journal 可记录工具开始、成功、失败、取消与未知结果，但 grep 只读且可重复，默认无需持久化结果；若记录审计，重试应产生新的 operation/call ID。`src/headless.rs` 只负责 JSONL transport/replay，应透传 ToolResult，不能在 headless 层重新解析正则。
- **providers/**：`src/providers/**` 仅负责模型连接和 provider 事件，不应实现 grep、路径授权或 ripgrep；工具调用仍由 core/tool registry 执行。Provider 断线/重试不应隐式重复一次已开始的搜索，需依靠 `session`/`tool_runtime` 的 call correlation 判断。
- **可执行验证**：添加 Rust 单测覆盖空 pattern、相对/绝对路径、include glob、0/1/100/101（应截断为 100）匹配、无 `rg`、非法 regex、workspace 外路径、取消和 10 秒超时；集成测试从 headless `StdioRequest` 发出 `grep` ToolCall，断言 `AgentEvent::ToolResult` 的 `matches/truncated/output`，并检查 session journal 没有写入文件副作用。

## 未决问题

- `Ripgrep.Service.grep` 的具体 JSON 字段、排序、正则错误类型和是否自身截断，无法仅由本文件确认（调用点只使用 `item.entry.path/line/text`，L63-L78）。
- `assertExternalDirectoryEffect` 对允许的外部目录、路径白名单和 `kind:"file"` 的精确策略未在本文件定义（L55-L58）。
- `FSUtil.resolve` 是否 canonicalize、如何处理符号链接与不存在路径，无法由调用点确定（L60-L63）。
- `ctx.ask` 的授权持久化和取消行为由 `Tool.Context` 实现决定；本文件只声明请求载荷（L39-L48）。

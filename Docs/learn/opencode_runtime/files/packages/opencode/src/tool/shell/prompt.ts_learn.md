# OC-052 — packages/opencode/src/tool/shell/prompt.ts

- source_id/item_id：OC-052
- source_path：`packages/opencode/src/tool/shell/prompt.ts`
- source_hash：`f3c6bdb216a9df2dd871d3a786b2abe3a20ea3e44b9121ce0610c648256fb2b6`
- source_bytes：16779
- source_lines：293（文件以换行结束；逐行读取范围为 L1-L293）
- coverage：已读取字节 `1-16779`、行 `L1-L293`，含全部 import、注释、类型、常量、私有函数、导出符号和文件末尾 re-export。

## 完整行为复盘

### 模块级依赖与分类常量（L1-L8）

文件导入 `Schema`、`DESCRIPTION`（`./shell.txt`）、`PositiveInt`、`Global` 和 `ShellID`（L1-L5）。`PS` 是只含 `powershell`、`pwsh` 的 `Set`，`CMD` 是只含 `cmd` 的 `Set`（L7-L8）；集合使用精确字符串匹配，大小写或别名不会自动归一化。该文件本身只生成描述和参数 schema，不执行 shell。

### `Limits`（L10-L13）

导出类型 `Limits` 有两个必需数值字段：`maxLines`、`maxBytes`（L10-L13）。源码不在此处检查正数、上限或二者关系；调用方传入什么数值，就插入提示文本。

### `parameterSchema()`（L15-L23）与 `Parameters`（L25-L26）

导出函数 `parameterSchema()` 每次调用都构造新的 `Schema.Struct`（L15-L23），字段如下。

- `command`：必需 `Schema.String`，描述为 “The command to execute”（L16-L18）；空字符串、shell 语法和路径安全性不由此 schema 判定。
- `timeout`：可选 `PositiveInt`，描述为毫秒级可选超时（L18）；因此类型层面拒绝非正整数，但默认值不在 schema 中提供。
- `workdir`：可选 `Schema.String`，描述为工作目录，缺省使用当前目录，并要求用该参数代替 `cd`（L19-L22）。空路径仍是字符串层面的合法输入。

导出常量 `Parameters` 在模块加载时保存一次 `parameterSchema()` 的结果；导出同名类型 `Parameters` 是 `Schema.Schema.Type<typeof Parameters>`（L25-L26），即从 schema 推导出的 `{ command: string; timeout?: positive-int; workdir?: string }` 形状。该类型和 schema 都不携带执行、取消或持久化语义。

### `renderPrompt(template, values)`（L28-L34）

私有函数对模板做全局正则替换：`/\$\{(\w+)\}/g` 只识别 `${` 后接一个或多个单词字符再 `}` 的占位符（L28-L30）。回调从 `values[key]` 取值；值为 `undefined` 时立即抛出 `Error("Missing shell prompt value: <key>")`（L30-L32），不会返回部分结果。已提供的值按原样插入，不做转义；未在模板中出现的键被忽略。正则不覆盖带连字符、空格或点号的键，因此这类文本会原样保留。替换同步完成，无异步等待、并发或重试。

### `shellDisplayName(name)`（L36-L41）

私有函数将三个精确名称映射为展示名：`pwsh`→`PowerShell (7+)`、`powershell`→`Windows PowerShell (5.1)`、`cmd`→`cmd.exe`（L36-L40）；其他名称原样返回（L40-L41）。它只影响介绍句，不改变后续分支判定。

### `powershellNotes(name)`（L43-L63）

私有函数为 `pwsh` 返回 PowerShell 7+ 说明，为 `powershell` 返回 Windows PowerShell 5.1 说明，否则返回空字符串（L43-L63）。两种说明都指导双引号插值、单引号原样字符串、完整 cmdlet 名、`$(...)`/`@(...)`、带空格可执行文件用 `&` 调用以及反引号转义（L45-L60）。差异是 `pwsh` 明确支持 `&&`/`||`（L46-L47），而 5.1 使用 `cmd1; if ($?) { cmd2 }`（L54-L55）。该函数不验证平台，`name` 匹配即返回对应文本。
### `chainGuidance(name)`（L65-L76）

该私有函数生成“依赖命令如何串联”的一段自然语言（L65-L76）。`powershell` 单独提示不要使用不受支持的 `&&`，改用 `cmd1; if ($?) { cmd2 }`（L66-L68）；`PS` 中的 `pwsh` 提示用一次 Bash tool call 和 `&&`，并举 `git add . && git commit ...` 例子（L69-L71）；`CMD` 提示使用 `&&`（L72-L74）；其余 shell 默认采用 Bash 文案（L75）。这里是给模型的指导文字，不会解析或执行命令，也不对 `name` 做大小写转换。

### `bashCommandSection(chain, limits, defaultTimeoutMs)`（L78-L119）

此函数返回完整 Bash 命令区段模板（L78-L119）。内容固定要求执行前先用 `ls` 验证将要创建的父目录（L79-L84），路径带空格必须双引号，并给出 `mkdir`、`python` 的正确/错误例子（L85-L94）。用参数插入三类运行约束：命令必填、未指定 `timeout` 时使用 `${defaultTimeoutMs}ms`、输出超过 `limits.maxLines` 或 `limits.maxBytes` 时截断且完整输出写文件（L95-L99）。同时明确应通过 Read/Grep 读取完整输出，避免 `head`/`tail` 等截断。

L100-L106 的工具选择规则要求优先专用的 Glob、Grep、Read、Edit、Write 和直接输出，除非确有必要才在 Bash 中使用 `find`、`grep`、`cat` 等。L107-L118 规定独立命令可并行发出多个 tool call，依赖命令使用传入的 `chain`，不关心失败时可用 `;`，禁止用换行分隔命令，并用 `workdir` 参数取代 `cd`。该函数不检查 `limits` 或 `defaultTimeoutMs` 是否合理；任何字符串均直接插入模板。

### `powershellCommandSection(name, chain, pathSep, limits, defaultTimeoutMs)`（L121-L170）

该函数先插入 `powershellNotes(name)`，再生成 PowerShell 命令区段（L121-L129）。目录验证改用 `Test-Path -LiteralPath`，示例中的路径分隔符来自 `pathSep`（L130-L135）；路径带空格使用双引号，脚本路径通过 `&` 调用（L136-L145）。超时与输出限制说明与 Bash 对应，但读取完整输出时禁止 `Select-Object -First/-Last`（L146-L150）。专用工具名称改成 Glob、Grep、Read、Edit、Write 的 PowerShell 对照说明（L151-L157）。多命令规则仍插入 `chain`，但统一使用反引号包围 `;`，禁止命令中自行换目录，示例在 `powershell` 下用 `Set-Location ...; if ($?) { ... }`，其他 PowerShell 名称用 `Set-Location ... && ...`（L158-L169）。`pathSep` 仅影响示例文本，不影响实际进程。

### `cmdCommandSection(chain, limits, defaultTimeoutMs)`（L172-L219）

该函数生成 `cmd.exe` 专用模板（L172-L219）。开头说明双引号路径、`%VAR%` 环境变量、`if exist` 存在性检查和跨批处理调用时的 `call`（L173-L178）。目录验证示例为 `if exist "foo\\" dir "foo"`（L179-L184），文件路径、`mkdir`、`call` 示例说明不加引号会被拆分（L185-L194）。超时和输出限制插值同前，但建议避免 `more` 分页（L195-L199）。专用工具映射为 Glob/Grep/Read/Edit/Write，命令串联使用传入 `chain`，不关心失败时可用 `&`，禁止换行分隔命令及在命令中切换目录（L200-L218）。所有规则是提示文本，不构成执行器的解析器或沙箱。

### `profile(name, platform, limits, defaultTimeoutMs)`（L221-L271）

`profile` 先计算 `isPowerShell = PS.has(name)` 与 `chain = chainGuidance(name)`（L221-L223）。分支优先级为 `CMD`、PowerShell、默认 Bash：即使未来名称同时进入集合，也会先走 CMD（L224-L235）。

- `cmd` 分支返回对象：介绍句使用 `shellDisplayName`，工作目录说明要求使用 `workdir`，命令区段调用 `cmdCommandSection`；`gitCommands` 与限制均为 `git commands`；PR 指令要求临时 body 文件，示例用括号和 `echo` 再 `gh pr create --body-file`（L224-L235）。
- PowerShell 分支适用于 `pwsh` 与 `powershell`（L236-L255）。`pathSep` 在 `platform === "win32"` 时为反斜杠，否则为斜杠（L241-L246）；命令区段调用 `powershellCommandSection`。PR 指令使用 `gh pr create` 的 PowerShell here-string（L248-L254）。
- 其余名称走 Bash 默认对象（L257-L270）：介绍强调 persistent shell session；工作目录文案禁用 `cd <directory> && <command>`；命令区段调用 `bashCommandSection`；`gitCommands` 为 `bash commands`，限制为 `git bash commands`；PR 示例使用 `$(cat <<'EOF'` HEREDOC（L257-L270）。

`profile` 只组装字符串和函数结果；没有网络、进程、文件写入、超时计时器或并发控制。`platform` 只在 PowerShell 示例路径中生效。
### `render(name, platform, limits, defaultTimeoutMs)`（L273-L290）与 `ShellPrompt` re-export（L293）

导出函数 `render` 先调用 `profile`（L273-L274），再调用 `renderPrompt(DESCRIPTION, values)`。传入值完整对应模板占位符：`intro`、平台字符串 `os`、shell 名 `shell`、`Global.Path.tmp`、工作目录段、命令段、git 文案、`ShellID.ToolID`、git 限制、PR 指令与示例（L275-L287）。若 `DESCRIPTION` 引用了未提供的合法占位符，`renderPrompt` 会抛出缺值错误；若模板没有这些键则对应值被忽略。成功结果是 `{ description: string, parameters: parameterSchema() }`（L275-L290），其中 `parameters` 每次 render 都是新 schema 实例，而非复用导出常量 `Parameters`。函数同步返回，不启动命令、不等待超时、不读取会话状态。

L293 的 `export * as ShellPrompt from "./prompt"` 将本模块命名空间重新导出；它不增加运行逻辑，但使调用者可以通过 `ShellPrompt.render`、`ShellPrompt.Parameters` 等访问导出成员。由于是自引用模块命名空间导出，应由 TypeScript 模块系统处理，源码没有额外初始化副作用。

## 状态、取消、恢复与副作用

该文件是纯提示构造层。唯一可抛出的显式错误来自 `renderPrompt` 的缺失占位符（L28-L33）；schema 构造本身同步完成。`timeout` 只作为输入字段约束和提示文案中的默认毫秒数（L18、L97-L98、L148、L197），没有计时器、超时中断或重试实现。`workdir` 只进入参数 schema 和提示，未在此处切换目录（L19-L22、L112-L118、L163-L169、L212-L218）。

没有取消 token、AbortSignal、并发任务、锁、队列或后台 worker；因此取消/超时只能由上层 shell 执行器解释本文件生成的契约。没有持久化、日志、环境变量写入、子进程、网络请求或外部文件副作用；`Global.Path.tmp` 与 `ShellID.ToolID` 仅被读取并插入描述（L275-L285）。没有恢复点或重试状态；重复调用 `render` 只会重新生成同类字符串和 schema。模板文本中的“并行 tool call”“输出写文件”等是给模型的操作约束，不是本模块执行的副作用（L95-L118、L146-L169、L195-L218）。

## 源内测试与行为判据

源文件及同目录已读内容中未包含测试；源内未包含测试。可独立验证的判据如下：

1. 对 `parameterSchema()` 做 schema 解码：`{command:"x"}` 成功；缺 `command`、非字符串 `command`、非正整数 `timeout` 应失败；`workdir` 与 `timeout` 可省略（L15-L23）。
2. 对 `renderPrompt("${a}-${b}", {a:"1"})` 应抛出 `Missing shell prompt value: b`；带连字符的占位符如 `${a-b}` 不应被该正则替换（L28-L34）。
3. `render("cmd", "win32", limits, 1000)` 的描述应含 `cmd.exe`、`if exist`、`1000ms` 和 `git commands`；`render("powershell", "win32", ...)` 应含 5.1 说明和反斜杠示例；`render("pwsh", "linux", ...)` 应含 PowerShell 7+、斜杠示例和 `&&` 链接指导；未知名称应走 Bash 分支（L36-L40、L43-L76、L221-L290）。
4. 检查描述中 `${tmp}`、`${toolName}` 等实际模板占位符被 `Global.Path.tmp`、`ShellID.ToolID` 替换，缺任意键时调用抛错；重复 `render` 返回的 `parameters` 应是可独立构造的 schema（L273-L290）。
5. 输入极大 `limits.maxLines`、负数 `defaultTimeoutMs` 或空 `workdir` 不应在本文件内被拒绝，因为这里没有额外数值/路径校验（L78-L119、L121-L219）。
## zenpi Rust 映射

zenpi 已把“提示生成”和“执行/治理”分层：`prompt.ts` 的可移植核心应落在一个纯函数模块，执行语义继续由现有宿主负责。

- `src/core.rs` 的 `Agent::run_user_shell_with_cancel`（约 L2961 起）是最直接的执行落点：它校验单个 `!`、要求 `ToolRuntime` workspace/policy、创建 approval、在 spawn 前 `begin_operation` 并调用 `RunCommandTool::invoke_user_shell_with_cancel`。对应关系是：`command`/`workdir`/`timeout` 的请求形状映射到 `ToolCall` 参数；`render` 生成的说明可由 `ShellPrompt` 纯模块提供给 UI/模型。差异是 opencode 的 `render` 同时按 shell 名称生成 Bash/PowerShell/cmd 文案，而当前 Rust user-shell 路径主要执行本地工具，不显示同等完整的 shell-specific guidance。
- `src/tool_runtime.rs` 的 `MasterSessionCommand`、`classify_master_session_input`（L37-L108）负责 `!command` 与 steer 的分类及输入上限；`execute_tool_batch`（L157 起）负责顺序/并行、取消轮询、批量限制和结果关联。它可承接 prompt 中“依赖命令用 `&&`/条件串联”的治理前置，但应把文字指导与真正的执行模式分开：prompt 的 `chainGuidance` 不应改变 `ToolBatchOptions`。
- `src/runtime.rs` 的 `CancellationToken`、`JobOutcome`、有界队列和 `recv_timeout`（L49-L93、L174-L364）对应上层取消、超时等待和后台作业生命周期。差异是源文件没有任何 cancellation 实现；Rust 必须在 `RunCommandTool`/host 侧将 token 传入，不能从提示字符串推导取消。
- `src/approval.rs` 的 `ApprovalRequest`、`ApprovalCoordinator` 与 `ApprovalPolicy`（L39-L180）对应命令执行前的显式授权。源提示只说“after host approval/ security measures”，未定义批准状态、记忆策略或取消错误；Rust 现有实现还会持久化 accepted approval，因此不能把 `render` 当作批准。
- `src/session.rs` 是 append-only JSONL 持久化，含 `OperationKind::Tool`、`OperationOutcome::{Succeeded,Failed,Cancelled,Interrupted,UnknownOutcome}` 和中断恢复投影（L1-L145）。这比源文件更强：opencode prompt 仅指导超时、输出截断和完整输出文件，未定义重启恢复；Rust 应继续在 dispatch 前写 operation marker、结束后写 terminal record，未知结果要求显式确认重试。
- `src/headless.rs` 将正常 provider turn 与本地 shell 结果统一到 headless 事件/会话流（约 L823、L1561、L3996、L4606、L5721）。可在 headless 输出描述或能力查询中调用纯 Rust `shell_prompt::render`，但 `UserShellRequest` 的解析本身不授予执行权，仍须经过 `Agent` 的 owner、approval 与 side-effect gate。
- `src/protocol.rs` 的 `MAX_USER_SHELL_BYTES`、`UserShellRequest` 及 `Command::UserShell` 路由（约 L37、L115-L116、L486-L510、L689 起）可承接 `command` 输入边界；差异是 `prompt.ts` 的 `command` schema 没有字节上限，Rust 协议层已有限制，映射时应以协议上限为准，避免把 schema 的“字符串合法”误当作无限长度。
- `src/providers/**`（尤其 `providers/registry.rs` 与 `backend::Backend`）只描述 provider/model 能力、身份、超时与连接，不应承载本地 shell prompt。provider 侧可以消费 `description` 作为模型上下文，但 `workdir`、命令执行、输出截断和审批都属于 `core`/`tools`/`runtime`，不要放入 provider registry 或凭模型名推断 shell 能力。

建议的可执行 Rust 落点与差异清单：

1. 新增纯模块 `src/shell_prompt.rs`（或 `src/tools/shell_prompt.rs`），定义 `ShellKind`/字符串名称、`Limits { max_lines, max_bytes }`、`ShellParameters { command, timeout_ms: Option<NonZeroU64>, workdir: Option<PathBuf> }`；实现 `parameter_schema` 等价的 serde/校验函数和 `render(name, platform, limits, default_timeout_ms)`。保持函数无 I/O、无 token、无 session 依赖，单元测试覆盖四类 shell 与缺占位符错误。
2. 将 `core::run_user_shell_with_cancel` 的真实 workspace、approval、operation journal、`CancellationToken` 传给执行器；`render` 只在构造 provider/tool 描述时调用。验证 `timeout_ms` 转换到实际 `RunCommandTool` timeout，输出限制与 `tool_output`/journal 的实际字段一致；当前源提示提到“完整输出写文件”，Rust 必须确认 `tool_output` 是否真的保存并能通过 Read/Grep 访问。
3. 在 `protocol.rs` 保留 `MAX_USER_SHELL_BYTES` 和显式 `UserShellRequest` 校验；在 `tool_runtime.rs` 复用 `MasterSessionCommand` 分类，不新增第二套 `!` 解析。把 `chainGuidance` 作为描述文本，顺序/并行由 `ToolBatchOptions` 和工具 execution mode 决定。
4. 在 `headless.rs` 增加可观测字段时沿用现有 `user_shell_input`、`tool_execution_started/finished` 与 cancellation 事件；不要把提示文本当作持久化事实。用 `session.rs` 的 `InterruptedOperation` 验证崩溃后 retry 需要确认，使用 `approval.rs` 验证拒绝、取消和 remember 语义。
5. 差异验证：对 Windows `platform == win32` 使用 `\\` 示例，对 Unix 使用 `/`；`powershell` 与 `pwsh` 的链式命令文案必须不同；未知 shell 仍回退 Bash 文案；Rust provider 模块不得改变 shell 选择。运行 `cargo test` 的新增纯函数测试，再用已有 user-shell/headless 测试确认审批、取消、输出持久化和恢复没有回归。

## 未决问题

1. `DESCRIPTION`（`./shell.txt`）的完整占位符集合不在本源文件内；仅能确认 `render` 显式提供的 11 个键（L275-L287），无法仅凭本文件确认模板是否还有其他占位符。
2. `Global.Path.tmp` 的具体路径来源、`ShellID.ToolID` 的具体字符串，以及 `RunCommandTool` 如何落实超时、输出截断和 persistent shell session，均由导入模块/调用方决定，不能从本文件进一步确认。
3. `limits.maxLines` 与 `limits.maxBytes` 的实际截断算法、完整输出文件命名及 Read/Grep 接口契约不在本文件中。

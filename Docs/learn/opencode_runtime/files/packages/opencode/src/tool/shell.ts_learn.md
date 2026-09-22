# OC-050 — packages/opencode/src/tool/shell.ts

## 元信息

- source_id/item_id：OC-050
- source_path：packages/opencode/src/tool/shell.ts
- source_hash：342d742ae324782d222465c202dcdfcb7cc35a4cecf04e87f9881f1794d921ac
- source_bytes：20439
- source_lines：645
- coverage：已按源文件顺序读取完整内容，覆盖字节范围 1-20439、行范围 L1-L645（包含注释、类型、导入和导出）。

## 完整行为复盘

- 导入与导出（L1-L25）：依赖 Effect/Stream、ChildProcess、web-tree-sitter、文件流、路径、配置、插件、权限和截断服务；唯一显式导出是从 ./shell/prompt 转出的 Parameters（L25）。
- 常量与内部类型（L27-L82）：MAX_METADATA_LENGTH=30000 限制状态元数据预览；CWD 是不会按普通命令申请 shell 权限的目录切换命令集合；FILES、CMD_FILES 描述 Bash/PowerShell/CMD 中会触及路径的命令；FLAGS 是 PowerShell 后跟路径值的参数，SWITCHES 是无路径值开关。内部 Part 保存 tree-sitter 节点类型和文本，Scan 聚合外部目录、命令模式和“始终允许”模式，Chunk 保存输出片段及 UTF-8 字节数。
- resolveWasm（L84-L89）：将 file://、绝对路径、Windows 盘符路径或相对 URL 转成本地 WASM 文件路径；相对资源以 import.meta.url 为基准，URL/路径异常直接抛错。
- parts（L91-L117）：遍历一个 Node 的直接子节点。对 command_elements 递归取子项，跳过参数分隔符和重定向；其他只接受命令名、单词、字符串、裸字符串、拼接节点。输出顺序保持语法树顺序，未知节点被丢弃，因此重定向本身不进入路径扫描。
- source（L119-L121）：若节点父级是 redirected_statement，取父节点全文，否则取自身文本，最后 trim()；用于权限模式的原始命令展示。
- commands（L123-L125）：返回根节点所有后代 command 节点，并以类型守卫过滤空值；多个管道/复合命令会逐个扫描。
- unquote（L127-L133）：仅当首尾同为单引号或双引号且长度至少 2 时去掉一层引号；不处理转义、嵌套引号或不成对引号。
- home（L135-L139）：把 ~、~/...、~\... 展开为 os.homedir() 下的路径；其他文本原样返回，使用平台 path.join。
- envValue（L141-L145）：非 Windows 直接查 process.env[key]；Windows 做不区分大小写的键查找，返回实际环境变量值，找不到为 undefined。
- auto（L147-L152）：为变量展开提供 HOME、PWD、PSHOME 的自动值，分别是用户主目录、当前工作目录和 shell 所在目录；未知键无值。
- expand（L154-L160）：先 unquote，再替换 PowerShell 的 env:KEY、$env:KEY、$HOME/$PWD/$PSHOME，缺失变量替换为空，最后执行波浪号展开。它不执行命令替换。
- provider（L162-L172）：识别 filesystem::path 并返回路径；其他双冒号 provider 返回空（拒绝）；单字母前缀如 Windows C:\ 保持原文；一般 name:path 也返回空，未匹配时返回输入。
- dynamic（L174-L179）：检测 (...)、@(...)、$(...)、${...}、反引号和美元变量。PowerShell 对除 $env: 外的美元表达式都视为动态；Bash 只要含美元符号即动态。动态路径不进入外部目录授权。
- prefix（L181-L186）：找到第一个 ?*[ 通配符时截取其前缀；通配符在首位或正则无匹配时返回 undefined/原文，用于把 glob 映射成可检查目录。
- pathArgs（L188-L218）：Bash/CMD 模式取首个命令之后、不以 - 开头的参数；CMD 还排除 / 开关，chmod +... 排除权限标志。PowerShell 模式逐项处理：SWITCHES 跳过，FLAGS 的下一项作为路径，其余参数作为路径；这是保守扫描，可能收集非路径参数。
- preview（L220-L223）：文本不超过 30000 字节则原样，否则保留末尾 30000 个 JavaScript 字符并加 ... 标记；该函数按字符而非 UTF-8 字节裁剪。
- tail（L225-L255）：同时受最大行数和最大 UTF-8 字节数限制。未超限返回 cut:false；超限从末尾倒序收集完整行。若单行本身超过字节上限，按字节截取并跳过 UTF-8 continuation byte，返回可解码尾部和 cut:true。
- parse（L257-L261）：懒加载的 parser 解析 Bash 或 PowerShell；解析失败（无树）抛出 Failed to parse command。WASM 加载、语法错误异常都会阻止执行和授权扫描。
- ask（L263-L291）：先对 scan.dirs 请求 external_directory，每个目录生成 dir/*（Windows 先 FSUtil.normalizePathPattern），并将 globs 同时放入 patterns/always。随后若有命令模式，请求 ShellID.ToolID，always 使用 scan.always，元数据带原始命令。两个请求按顺序发生，任一拒绝都会短路执行。
- cmd（L293-L310）：Windows PowerShell 使用显式 -NoLogo -NoProfile -NonInteractive -Command 参数；其他情况把命令交给 ChildProcess.make(command, [], {shell,...})。stdin 永远忽略；非 Windows 设置 detached:true，Windows 不分离。
- parser lazy 单例（L311-L336）：动态加载通用 web-tree-sitter.wasm、Bash 和 PowerShell WASM，并行加载两种语言，创建两个 Parser；首次调用并发共享同一个 Promise，避免重复初始化。
- 导出 ShellTool（L338-L345）：通过 Tool.define(ShellID.ToolID, Effect.gen(...)) 注册工具，并注入 Config.Service、ChildProcessSpawner、FSUtil.Service、Truncate.Service、Plugin.Service、RuntimeFlags.Service。默认超时为 flags.bashDefaultTimeoutMs ?? 120000（L347）。
- 内部 cygpath（L349-L356）：Windows 下通过 shell 执行 cygpath -w -- "$1"，取首行并规范化；任何失败被吞掉并返回空，因此路径解析可退化到普通 Windows 解析。
- 内部 resolvePath（L358-L367）：Windows POSIX shell 且以 / 开头时尝试 cygpath；否则用 FSUtil.windowsPath 与 path.resolve(root,...)。非 Windows 直接 path.resolve(root,text)，结果为绝对路径。
- 内部 argPath（L369-L376）：PowerShell 参数先 expand，Bash/CMD 只做去引号和 home 展开；取 glob 前缀，动态表达式或 provider 拒绝时返回空；否则交给 resolvePath。
- 内部 collect（L378-L414）：初始化三个集合，按 shell 名称判定 CMD。对文件命令的每个路径参数解析并记录日志；若路径在当前 InstanceContext 内则跳过，否则检查是否目录并把目录本身或其父目录放入 dirs。每个非 CWD 命令都加入原始源码模式及 BashArity.prefix(tokens)+" *" 的宽松 always 模式。返回集合去重，遍历顺序稳定。
- 内部 shellEnv（L416-L426）：触发插件 shell.env，传入 cwd、sessionID、callID，默认环境为空；将插件环境覆盖合并到 process.env，插件异常向上失败。
- 内部 run（L428-L595）：先取截断上限，保留最近约 2*maxBytes 的 Chunk 队列，同时以 preview 更新元数据。启动子进程后 fork 一个作用域内的流消费者读取 handle.all；未达到上限时累积 full，超过上限调用 trunc.write 持久化文件并切换到追加写入 sink。初始和每个片段都会调用 ctx.metadata({output})。作用域 finalizer 关闭写流（L450-L473）。退出、abort、timeout 三者竞速（L542-L546）；超时实际等待 input.timeout+100ms，abort/timeout 都以 forceKillAfter: "3 seconds" 杀进程。最终用 tail 再次裁剪；若裁剪但尚无文件则写完整 raw；无输出显示 (no output)。输出被裁剪时附带保存路径，超时/用户取消追加 <shell_metadata>；返回命令标题、exit code、truncated/outputPath 元数据和展示文本。子进程、流读取、文件写入均在 Effect scope 中，流消费者与主流程可并发运行，但作用域退出会等待/清理资源。
- 返回的执行闭包（L597-L643）：读取配置并用 Shell.acceptable 选择 shell，渲染 ShellPrompt 描述/参数，记录日志。执行时从 InstanceState.context 取得目录；workdir 经 resolvePath 解析，否则用实例目录。只拒绝负 timeout（L615-L617），零和正数允许；缺省使用默认超时。解析树用 acquireRelease 保证 tree.delete()，收集并在 cwd 位于实例外时补充 cwd 目录授权，然后调用 ask。审批通过后构造插件环境并运行命令。解析或审批失败不会创建子进程。

## 状态、取消、恢复与副作用

- 状态仅存在一次执行闭包内：输出片段队列、full/last/file、裁剪标记、超时和 abort 标记；没有跨调用的重试计数或恢复点。
- 取消由 ctx.abort 事件驱动，与退出码和 timeout 竞争；取消/超时都杀子进程，但杀进程失败通过 orDie 变成致命 defect。取消不是回滚，命令已产生的文件、网络、数据库等副作用保留。
- 没有自动重试；超时元数据明确建议调用方以更大毫秒值重试。命令启动前的解析、路径扫描、审批是可取消的 Effect，但源中没有持久化其状态。
- 副作用包括执行任意选定 shell、插件修改环境、写截断输出文件、创建/关闭 createWriteStream；外部目录与命令权限必须分别通过 ctx.ask。
- 只有输出 artifact 路径通过 Truncate.Service 持久化；session、provider 或 turn journal 不由本文件直接写入。解析树在释放阶段删除，WASM parser 由 lazy 单例复用。

## 源内测试与行为判据

源文件未包含测试。可独立验证的判据：1）给定 Bash/PowerShell 命令，tree-sitter 能解析且每个 command 被扫描；2）外部文件参数触发一次 external_directory，非 CWD 命令触发 ShellID.ToolID；3）负 timeout 在进程启动前报错，超时输出含 <shell_metadata>；4）超过 Truncate.Service 限制时结果含 Full output saved to: 且 truncated=true；5）abort 后子进程被 kill、exit 为 null、输出含 User aborted the command；6）多字节单超长行的 tail 输出仍是合法 UTF-8；7）插件 shell.env 的键覆盖同名 process.env。

## zenpi Rust 映射

- src/tools.rs::RunCommandTool（现有 L2219 起）是最接近落点：它已有 invoke_with_cancel/invoke_user_shell_with_cancel、命令超时、进程组清理、stdout/stderr 限制和 artifact capture。应新增独立 ShellTool 或扩展该类型，加入 tree-sitter 等价扫描、PowerShell/CMD 分支、合并输出和工作目录审批。
- src/tool_runtime.rs 负责批量准备、顺序/并行执行和取消传播；把本文件的“审批后才 spawn”对应到 ToolBatchDecision::ExecuteApproved，将超时映射为 ToolErrorCode::CommandTimeout，将 kill 后不确定结果记录为取消/未知，而非假设回滚。
- src/approval.rs 的 ApprovalCoordinator/ApprovalRequest 对应 ctx.ask；建议把命令模式、外部目录 glob、原始 command 放入 arguments/preview，并维持“目录授权”和“命令授权”两个可审计请求。
- src/runtime.rs 的 CancellationToken、BackgroundRunner 对应 Effect scope、raceAll 和子进程监督；需要在 worker 中轮询 token，并在超时/取消时先终止 process group 再 join 输出读取线程。
- src/core.rs 负责 turn/tool admission、ToolContext 和副作用策略；在这里绑定 session/call ID、workspace root、policy digest，拒绝路径越界和未审批 command。
- src/headless.rs 是 JSONL wire owner；将 shell 的中间 output metadata 映射为有界 StdioEvent，将最终 title/output/exit/truncated/outputPath 映射为终端响应，避免把诊断写入 stdout。
- src/protocol.rs 已有 UserShellRequest、MAX_USER_SHELL_BYTES 和版本化事件；若要暴露 workdir/timeout，应新增显式字段及校验，拒绝未知字段、负数和超出策略上限的值。
- src/session.rs 适合持久化命令 operation marker、取消/超时结论和 output artifact 引用，支持 crash 后 UnknownOutcome；本 TypeScript 文件本身没有恢复语义，Rust 不应把取消误报为成功。
- src/providers/**（openai、anthropic、google、deepseek、codex、registry）只负责 provider 请求/流，不应直接执行 shell；provider 返回的 tool call 应经 core -> approval -> tool_runtime -> RunCommandTool 链路。
- 可执行验证差异清单：为 Bash/PowerShell/CMD 各建路径扫描测试；验证 ~、环境变量、filesystem::、动态表达式和 glob 前缀；验证外部目录越界审批；验证 UTF-8 尾部裁剪；验证超时/取消下 process group 无遗留；验证 artifact 写入和 session 恢复；验证插件环境覆盖；验证 Windows cygpath 分支（Rust 当前 RunCommandTool 主要是 Unix /bin/sh -c，存在平台差异）。

## 未决问题

无。


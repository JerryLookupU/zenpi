# OC-039 — packages/opencode/src/tool/glob.ts

## 元信息

- source_id/item_id：packages/opencode/src/tool/glob.ts / OC-039
- source_path：packages/opencode/src/tool/glob.ts
- source_hash：a5069377ae916495a72be319d218878136fc13f18551f3e16981d8c692c68a4a
- source_bytes：2895
- source_lines：76
- coverage：已完整读取字节 0-2894（2895 字节）与行 L1-L76，包含全部导入、注释、类型注解、导出符号及闭合括号。

## 完整行为复盘

### 模块依赖与导出参数契约（L1-L15）

- L1-L8 导入 Node 的 path，Effect 的 Effect 与 Schema，实例上下文 InstanceState，文件系统服务 FSUtil，ripgrep 服务 Ripgrep，外部目录检查 assertExternalDirectoryEffect，工具描述文本 DESCRIPTION 和工具定义 API Tool。该文件本身不实现遍历算法，而是组合这些服务。
- L10-L15 导出 Parameters。它是 Schema.Struct：
  - pattern：必填 Schema.String，描述为用于匹配文件的 glob pattern；类型层面只保证是字符串，不在此处限制长度、语法或是否为空。
  - path：Schema.optional(Schema.String)，省略时使用当前工作目录；描述特别要求默认行为下直接省略，不要传入字符串 undefined 或 null，提供时必须是有效目录路径。
  - Schema 只定义输入形状；文件存在性、是否为目录、路径边界和 glob 语法在执行阶段处理。
- 参数快照测试确认 pattern 是 required、path 是可选字符串（test/tool/parameters.test.ts:L140-L150，生成的 JSON Schema 快照在 test/tool/__snapshots__/parameters.test.ts.snap:L76-L94）。因此空对象拒绝，{pattern:"**/*.ts"} 与附带 path 的调用接受。

### GlobTool 工厂与服务获取（L17-L24）

- L17-L24 导出 GlobTool，通过 Tool.define("glob", Effect.gen(...)) 注册工具名 glob。
- L19-L21 在 Effect 环境中取得 FSUtil.Service 与 Ripgrep.Service。工具初始化依赖这两个服务；缺少服务属于环境构造/运行时失败，不会被本文件转换为业务结果。
- L22-L24 返回定义对象：description 使用 glob.txt 的说明，parameters 使用上述 Parameters。描述强调按文件名模式快速查找、支持如 **/*.js 或 src/**/*.ts 的模式、返回匹配路径，并建议开放式多轮搜索使用 Task。

### execute(params, ctx)（L25-L73）

输入是 { pattern: string; path?: string } 与 Tool.Context；输出是包含 title、metadata、output 的对象。实现整体由 L26-L73 的嵌套 Effect.gen 顺序执行。

1. 取得实例边界（L27）：从 InstanceState.context 取得 ins。后续默认搜索根是 ins.directory，结果标题以 ins.worktree 计算相对路径。这两个基准有意分离，不能假定 directory 一定等于 worktree。
2. 工具权限请求（L28-L36）：先调用 ctx.ask，再做路径检查或启动搜索。请求字段固定为：
   - permission: "glob"
   - patterns: [params.pattern]
   - always: ["*"]，表示该请求的持久化/始终允许模式是全局通配符
   - metadata: { pattern: params.pattern, path: params.path }，省略 path 时 metadata 中的值仍是 undefined，而不是补默认目录。
   权限拒绝、上下文取消或 ask 的其他失败会在此处终止执行。
3. 搜索根解析（L38-L39）：params.path ?? ins.directory 使用空值合并；只有 undefined/省略会回退，空字符串会继续参与解析。若 search 已是绝对路径则保留，否则用 path.resolve(ins.directory, search) 相对实例目录解析。因此相对参数不是相对当前进程 cwd，也不是必然相对 worktree。
4. 目录类型预检（L40-L43）：调用 fs.stat(search)，并用 Effect.catch(() => Effect.succeed(undefined)) 吞掉所有 stat 失败，把结果视为未知。若 stat 明确报告 info?.type === "File"，抛出普通 Error，消息模板为 "glob path must be a directory: " 加上已解析的 search 路径；目录、不存在、权限错误或其他类型不会在这里直接拒绝。
5. 外部目录授权（L44-L47）：调用 assertExternalDirectoryEffect(ctx, search, { bypass:false, kind:"directory" })。实例目录内的路径通常不再提示；实例外路径会按目录范围构造外部目录权限请求（同目录测试验证了 kind:"directory" 使用 target/*，见 test/tool/external-directory.test.ts:L81-L95）。该授权是第二个可能阻断点，即使前面的 glob 权限已经允许也必须通过。
6. 固定结果上限与 ripgrep 调用（L49-L51）：limit = 100 为硬编码上限，调用 ripgrep.glob({ cwd: search, pattern: params.pattern, limit })。本层未传 hidden、follow 或 signal，所以采用 Ripgrep.Service 默认行为；适配器会以 ripgrep --files --glob=<pattern> 搜索、排除 .git，将返回项表示成相对于 cwd 的文件路径。ripgrep 失败或模式解析失败向上失败，没有本地降级。
7. 截断判定（L51）：truncated = files.length === limit。达到 100 即标记截断，即使真实匹配恰好只有 100 个也无法区分“刚好达到上限”和“还有更多”。
8. 输出拼装（L53-L63）：
   - 初始化数组 output。
   - 无匹配时只加入精确字符串 "No files found"（L54），最终输出没有尾随换行。
   - 有匹配时逐项执行 path.resolve(search, file.path)（L56），因此对 ripgrep 返回的相对路径再次变成绝对路径，并按数组原顺序以换行连接。
   - 若 truncated，先追加空行，再追加精确提示 "(Results are truncated: showing first 100 results. Consider using a more specific path or pattern.)"（L57-L61）。
9. 成功结果结构（L65-L72）：
   - title: path.relative(ins.worktree, search)；搜索根与 worktree 相同会得到空字符串，位于 worktree 外可能得到带 .. 的相对路径。
   - metadata.count = files.length，计数最大为 100；metadata.truncated = truncated。
   - output = output.join("\n")，是给模型/界面的纯文本路径列表或“无结果/截断”文本。
10. 失败通道（L73）：整个执行 Effect 接 .pipe(Effect.orDie)，把内部失败通道转成 defect/die。也就是说调用者不能从该函数得到结构化的 ToolError 失败值；普通 Error、权限失败、外部目录失败、ripgrep 失败都会沿 Effect 运行时的 defect 处理路径暴露。源码没有 try/catch、重试或“部分成功”返回。

### 并发语义

L19-L73 的 generator 内部步骤是顺序的：权限请求完成后才 stat，stat/外部目录检查完成后才启动 ripgrep，ripgrep 完成后才构造结果。单次调用不创建线程、不 fork Fiber、不批量并发多个模式；跨调用是否并行由 Tool.define/上层工具调度器决定，本文件没有声明 execution_mode。

## 状态、取消、恢复与副作用

- 取消/超时：execute 没有读取 ctx.abort，也没有向 Ripgrep.glob 传 AbortSignal（虽然底层服务接口支持 signal）。因此取消是否能打断等待，取决于 Effect 运行时对外层 fiber 的中断；源码没有在 stat、权限等待或 ripgrep 子进程之间增加显式轮询。没有工具级超时常量。
- 重试：没有重试、退避、幂等键或“已执行但结果未知”状态。失败直接 die；重新调用会重新请求 glob 权限并重新运行搜索。
- 恢复/持久化：没有写文件、会话记录、缓存或 checkpoint。成功结果只在内存中生成。权限系统可能自行记住 always:["*"]，但这属于 ctx.ask 的外部实现，不是本文件持久化。
- 外部副作用：搜索本身是只读；外部目录授权会产生权限提示/策略状态变化；Ripgrep.Service 会启动子进程并读取目录。未传 follow，本层没有显式跟随符号链接的意图。stat 错误被吞掉可能把不存在路径留给后续外部授权或 ripgrep 处理。
- 边界：只显式拒绝“stat 明确为普通文件”的搜索根。不存在路径、特殊文件、stat 权限错误及 glob 非法模式由外部目录检查或 ripgrep 层决定；路径是否越出实例边界也由 assertExternalDirectoryEffect 与权限系统决定，而不是由 GlobTool 自己 canonicalize。

## 源内测试与行为判据

源文件本身未包含测试。可核对的行为判据如下：

- test/tool/parameters.test.ts:L140-L150 验证 pattern-only、可选 path、缺失 pattern 拒绝；快照 L76-L94 验证描述和 required 字段。
- test/tool/external-directory.test.ts:L81-L95 验证外部目录调用使用目录本身加 * 的权限范围；L53-L61 验证实例目录内路径不提示。
- test/session/prompt.test.ts:L881-L917 是端到端判据：在实例目录创建 probe.txt，发起 glob({pattern:"**/*.txt"}) 后，工具 part 必须完成、输出包含该绝对文件路径，且不能出现 “No context found for instance”。这能独立验证 InstanceState.context 被正确保留以及默认目录搜索成立。
- 可独立验证的补充判据：构造 101 个匹配文件时，输出应包含前 100 个绝对路径、metadata.count===100、metadata.truncated===true 和精确截断提示；传入 stat 为文件的 path 应得到 glob path must be a directory；零匹配应得到精确 No files found。

## zenpi Rust 映射

- 核心落点：zenpi 的最接近实现是 src/tools.rs 的 FindFilesTool，其定义声明工具名 find、只读副作用、Parallel 执行模式，并通过 ToolContext::resolve_existing 限制在固定 workspace_root。建议新增独立 GlobTool（或给 FindFilesTool 增加兼容别名 glob），输入字段保持 pattern 与可选 path，输出增加与源一致的 paths/count/truncated/title 字段；若要 1:1 保持文本协议，则将绝对路径换行串放入 ToolResult::Success.output，同时保留结构化 metadata。
- src/tool_runtime.rs：以 ToolBatchDecision::Execute 进入 execute_tool_batch；声明只读工具可并行，但单次 glob 仍应是一个原子调用。使用现有批次预算、结果按 source order 返回的约束，不复制一个新执行器。若 ripgrep 子进程启动计数需要治理，可沿 search_process_budget("find", ...) 方式为 glob 预留固定预算。
- src/approval.rs：源行为有 glob 权限和外部目录权限两个边界。zenpi 的只读策略可由 ApprovalPolicy::ReadOnly 自动放行工具本身，再由 ToolContext 的 workspace/path gate 处理越界；若需要交互提示，应构造 ApprovalRequest，把 pattern、解析后的 path 和目标目录作为 arguments/preview 审计，不能仅凭工具名推断外部目录授权。
- src/runtime.rs：把 CancellationToken::is_cancelled 传入 glob 实现，在启动搜索前、遍历批次间和收集结果时检查；优先让搜索 backend 接收取消，而不是像源文件一样完全省略 signal。超时应由宿主的 job/runtime deadline 负责，结束时返回 ToolError::Cancelled 或 CommandTimeout 的明确结果。
- src/core.rs：工具调用由 ToolInvocation/ToolInvocationOutcome 统一记录。建议把 glob 作为只读 ToolResult::Success，失败映射 ToolErrorCode::InvalidArguments/NotADirectory/NotFound/Cancelled/LimitExceeded，不要模拟 TypeScript 的 Effect.orDie 让进程 panic；title 可用 pathdiff::diff_paths(search, worktree)（或等价实现）复刻 path.relative(ins.worktree, search)。
- src/session.rs：源没有持久化，但 zenpi 会为工具操作写 operation marker/terminal record。glob 不产生工作区写入，成功可记录终态结果；若取消后后台子进程仍未 join，按现有 Interrupted/UnknownOutcome 规则处理，重试是否需要确认由宿主策略决定。
- src/headless.rs 与 src/protocol.rs：当前 JSONL 层已有通用工具调用、取消和终态回放机制，不需要新增 provider 专用命令。通过既有 tool-call 路径暴露 glob/兼容 find；终态响应应包含 call id、结构化 output 和 truncated，并让 reconnect journal 重放同一终态。若新增直接的 glob 命令，必须在 Command、解析校验和响应编码中增加 bounded pattern/path 字段，但这会偏离现有通用工具边界。
- src/providers/**：provider 模块只描述模型路由、协议和 capabilities；providers/connection.rs/registry 的 capabilities.tools 决定模型是否能收到工具 schema，不能承担文件系统访问。保持 glob 实现在 core/tools 层，向各 provider 统一注册 schema；不要在 openai.rs、anthropic.rs、deepseek.rs、google.rs 或 codex.rs 分别实现路径解析。若某路由不支持 tools，应沿现有 capability 交集逻辑拒绝工具调用。
- 差异清单：Opencode 默认根是 InstanceState.directory、标题基于 worktree，zenpi 是单一 workspace_root；Opencode 上限固定 100 且输出绝对路径纯文本，zenpi find 默认 limit 100 但输出为 JSON；Opencode 先请求 glob 再请求外部目录，zenpi 目前以 side-effect/path gate 为主；Opencode stat 失败被吞掉且文件根用普通 Error，zenpi 应返回可序列化错误；Opencode 没有显式取消 signal，zenpi 已具备合作式取消；Opencode 底层 ripgrep 排除 .git，zenpi search backend 的 ignore/hidden 选项需明确对齐。

## 未决问题

- Tool.define 对 Effect.orDie defect 的最终捕获、展示和工具消息编码未在本文件中给出。
- InstanceState.directory 与 InstanceState.worktree 在所有项目布局下是否相同无法由本文件确认。
- Ripgrep.Service.glob 对非法 pattern、权限错误、符号链接和排序的完整约定属于依赖模块；本文件只提供 cwd/pattern/limit。
- ctx.ask 中 always:["*"] 的持久化语义、权限请求是否可被缓存以及取消时的清理行为由调用方实现，源文件未说明。


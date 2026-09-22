# OC-012 — packages/opencode/src/session/instruction.ts

## 元信息

- source_id/item_id：OC-012
- source_path：packages/opencode/src/session/instruction.ts
- source_hash：fee79c023eae44f720d4d600f334a987ad5f7615c4a008a20c89131a92684a8f
- source_bytes：8581
- source_lines：237
- coverage：已从首字节到末字节完整读取，字节范围 1-8581（0-based offset 0-8580）；行范围 L1-L237，包括注释、类型、导入、闭包实现、导出和 layer 定义。

## 完整行为复盘

### 模块导入与 extract（L1-L32）

模块依赖 LayerNode、httpClient、SessionV1、Effect 的 Layer/Context、Config、InstanceState、RuntimeFlags、Flag、FSUtil、Global，并引入 MessageV2 类型（L1-L16；该类型在本文件中没有继续使用）。extract(messages) 输入 SessionV1.WithParts[]，逐消息、逐 part 扫描：只接受 part.type === "tool"、part.tool === "read"、part.state.status === "completed" 的读取结果；若 part.state.time.compacted 为真则跳过；metadata.loaded 必须存在且为数组，数组内只有字符串才加入 Set<string>（L17-L31）。因此输出天然去重，非字符串、缺元数据、未完成或已 compact 的 read 都不会贡献路径。它不检查路径是否存在，也不规范化路径。

### Interface 与 Service（L34-L46）

Interface 暴露五个 Effect API：clear(messageID) 清理消息声明；systemPaths() 返回全局/项目/配置本地指令的去重绝对路径集合；system() 返回带来源前缀的文本数组；find(dir) 返回目录内第一个指令文件或 undefined；resolve(messages, filepath, messageID) 返回要随文件读取而附加的局部指令记录（L34-L44）。除 clear 外，类型错误被声明为 FSUtil.Error。Service 是 Context service，键为 @opencode/Instruction，供 Effect layer 注入（L46）。

### layer 初始化与静态候选（L48-L77）

layer 依赖 FSUtil.Service | Config.Service | Global.Service | HttpClient.HttpClient | RuntimeFlags.Service，构造时取得 cfg/fs/global/flags，再把 HttpClient 包成 HttpClient.filterStatusOk(withTransientReadRetry(...))（L48-L59）。全局候选按顺序为全局配置目录下的 AGENTS.md，以及在 disableClaudeCodePrompt === false 时的 global.home/.claude/CLAUDE.md（L60-L63）。项目候选按 AGENTS.md、可选的 CLAUDE.md、已弃用的 CONTEXT.md 顺序建立（L64-L68）。InstanceState.make 建立实例内存状态，claims: Map<MessageID, Set<string>> 记录某条 assistant message 已附加过哪些附近文件；源码注释明确这是 per assistant message，不是持久化状态（L70-L77）。

### 内部 relative、read、fetch（L79-L103）

- relative(instruction) 读取当前 InstanceState.context。正常情况下调用 fs.globUp(instruction, ctx.directory, ctx.worktree)；若 Flag.OPENCODE_DISABLE_PROJECT_CONFIG 为真，则改从 global.config 到 global.config 查找。两条路径都用 Effect.catch 把任何 glob 错误折叠为空数组（L79-L89）。这意味着项目配置禁用时，配置中的相对路径不会从工作区向上搜索。
- read(filepath) 调用 fs.readFileString；任何读取错误都转为空字符串（L91-L93）。空文件和读取失败在上层都表现为“没有可附加文本”。
- fetch(url) 发出 HttpClientRequest.get(url)，外层 Effect.timeout(5000)，超时或 HTTP 执行错误变为 null，再变为空字符串；响应 arrayBuffer 失败也变成空 ArrayBuffer，最后用 TextDecoder 解码（L95-L103）。重试由 withTransientReadRetry 提供，源码未指定次数；只处理传入的 HTTP client，且只用于这里的 GET。

### clear（L105-L108）

clear(messageID) 取出实例状态并删除 claims[messageID]（L105-L108）。返回 Effect<void>，不触碰文件、会话 journal 或网络。删除后同一消息可再次附加附近指令；不存在的 key 是无害操作。

### systemPaths（L110-L153）

函数先读取配置和实例上下文并创建 Set<string>（L110-L114）。

1. 遍历 globalFiles，只要 fs.existsSafe(file) 为真就加入 path.resolve(file) 并立即 break；所以全局候选是“第一个存在者”，不会同时堆叠两个全局文件（L115-L120）。
2. 若项目配置未禁用，按 instructionFiles 顺序调用 fs.findUp(file, ctx.directory, ctx.worktree)。错误转为空数组；第一个得到非空匹配的文件名获胜，把该数组全部规范化加入集合，然后停止尝试后续文件名。注释明确不会从每个祖先同时叠加 AGENTS.md/CLAUDE.md（L122-L132）。
3. 遍历 config.instructions。以 http:// 或 https:// 开头的项跳过，因为它们由 system 单独作为远程项处理（L135-L137）。~/ 展开为 global.home + raw.slice(2)；绝对路径用 basename 在 dirname 中执行 fs.glob(..., { absolute: true, include: "file" })；其他项走 relative。单项 glob 错误转空数组，所有命中都以 path.resolve 加入集合（L138-L149）。
4. 返回 Set；因此全局、项目和配置本地重复路径只保留一次，插入顺序也成为后续文本输出顺序（L152-L153）。

### system（L155-L169）

system() 再次取配置并调用 systemPaths，同时保留配置中所有 HTTP(S) URL 的原始顺序（L155-L160）。文件读取通过 Effect.forEach(Array.from(paths), read, { concurrency: 8 })，远程获取通过 Effect.forEach(urls, fetch, { concurrency: 4 })；结果数组仍与输入顺序对齐（L162-L164）。输出先是非空文件项，再是非空远程项，每项格式为 Instructions from: <path-or-url>\n<content>；空内容被过滤（L165-L168）。本函数的并发边界是本地最多 8、远程最多 4，不保证文件与 URL 两组交叉并发，因为两组调用是先后执行的。

### find（L171-L177）

find(dir) 仅检查传入目录的直接子路径，不向祖先搜索；按 instructionFiles 的当前顺序拼接并 path.resolve，第一个 existsSafe 成功的路径立即返回，全部不存在则返回 undefined（L171-L177）。它受 layer 初始化时的 Claude flag 影响，也包含已弃用 CONTEXT.md。

### resolve（L179-L221）

输入是历史消息、正在读取的 filepath 和 messageID（L179-L183）。先计算系统级路径集合 sys，用 extract(messages) 得到历史 read metadata 中已加载路径，建立结果数组，取得共享 claims 状态，并把实例目录解析为 root（L184-L189）。目标路径与当前目录都做 path.resolve，从目标文件的父目录开始向上走（L190-L192）。

循环条件为 current.startsWith(root) && current !== root，所以只在实例 root 内、且不处理 root 本身；每层调用 find(current)（L193-L195）。发现为空、等于当前目标文件、已经在 sys、或已经在历史 already 中时跳过并继续父目录（L196-L199）。否则为 message 建立 Set；若该路径已被此 message 的 claims 占用则跳过（L201-L209）。源码在读取前先 set.add(found)，然后读取；非空内容才追加 { filepath: found, content: "Instructions from: " + found + "\n" + content }（L211-L215）。即使读取失败或空文件，claim 仍会抑制本消息后续重复附加。最后继续到父目录，按“近到远”的遍历顺序返回结果（L217-L220）。该函数每次调用内部是串行向上扫描；共享 Map 没有显式锁或持久化机制。

### 导出符号（L227-L237）

loaded(messages) 是 extract 的公开别名，直接返回已加载路径 Set（L227-L229）。node 用 LayerNode.make 暴露 Service 和 layer，依赖 Config.node、FSUtil.node、Global.node、RuntimeFlags.node、httpClient（L231-L235）。文件末尾 export * as Instruction from "./instruction" 提供命名空间导出（L237）。

## 状态、取消、恢复与副作用

- 状态只有实例内存中的 claims；它按 MessageID 隔离，clear 删除整条消息的声明。源码没有向 session、JSONL、数据库或配置文件写入 claims，因此进程重启会丢失去重状态。
- 文件副作用全是读取：existsSafe、findUp、globUp、glob、readFileString。读取错误大多折叠为空结果；没有写文件、修改工作区或 approval 流程。
- 网络副作用是对配置 URL 的 HTTP GET。请求有 5 秒 timeout，底层 client 使用 withTransientReadRetry；响应解码失败视为空字符串。没有显式的持久化、幂等键或远程结果缓存。
- systemPaths/system/resolve 本身没有显式取消 token；它们依赖 Effect 运行时取消传播。每个本地读取和远程读取的错误捕获可能把失败转成正常的空项；system 的并发任务仍受 Effect 取消影响。远程超时是唯一明确的时间边界。
- 没有恢复或自动重试指令附加语义。重复调用 resolve 时以 claims、extract(messages)、systemPaths 作为抑制条件；调用 clear 才能重新声明。配置/flag 在 layer 构造时捕获，运行中改变依赖服务不会重建这些静态候选数组。
- 路径安全边界是“从 root 向上走”的字符串前缀判断和 path.resolve；本文件没有显式拒绝绝对配置路径、符号链接或 ..，绝对配置路径反而被专门支持。这个行为与更严格的宿主路径策略需要在 Rust 映射时明确取舍。

## 源内测试与行为判据

同目录测试文件 packages/opencode/test/session/instruction.test.ts 有可执行判据：

- Instruction.resolve 测试（L114-L144）验证 root 的 AGENTS.md 已在 systemPaths 时不重复返回，子目录 AGENTS.md 会在读取嵌套文件时返回，直接读取 instruction 文件本身不会再次附加。
- claims 与恢复行为（L160-L207）验证同一 messageID 的第二次 resolve 返回空，clear 后可以重新附加，历史 read part 的 metadata.loaded 会抑制重复附加。
- 远程行为仍是 test.todo("fetches remote instructions from config URLs via HttpClient")（L209），因此 timeout、重试次数和响应错误的测试覆盖不足。
- Instruction.system（L212-L249）验证全局与项目 AGENTS.md 同时加载、输出顺序与精确前缀，以及 disableClaudeCodePrompt 为真时跳过项目和全局 CLAUDE.md。
- 全局配置目录来源测试（L251-L264）验证 Global.Service.config 下的全局 AGENTS.md 会被使用。
- 可独立验证的补充判据：对同一 config.instructions 放入重复本地路径应只输出一次；放入空文件应不出现在 system()；放入无法连接的 HTTP URL 应在约 5 秒边界后产生空项而不使整体失败；同时配置 9 个慢本地读应观察最多 8 个并发。上述判据直接对应 L91-L103、L152-L168 的实现。

## zenpi Rust 映射

对照现有 zenpi：

- src/core.rs:L4050-L4137 已把技能、persona、resource 和 selected skills 合成 instructions，按 instructions.len().div_ceil(4) 扣减 context budget，再通过 CompletionRequest::with_instructions 注入 provider。建议新增 src/instruction.rs 的 InstructionService，在这里合并 system() 文本并计入同一个 token 预算；保留 core.rs:L3981-L4000、L4063-L4065 的取消轮询。
- src/backend.rs:L176-L213 已有 CompletionRequest.instructions: Option<&str>，不需要改变请求模型。provider wire 层已经正确承载：Chat Completions 把文本插入首个 system message（src/protocols/chat.rs:L759-L762），Responses 写入 instructions（src/protocols/responses.rs:L88-L90），Anthropic 和 Google 分别生成 system text（src/protocols/anthropic.rs:L323-L328、src/protocols/google.rs:L280-L285）。src/providers/** 的 connection.rs:L166-L220 只负责 provider/model 路由，应保持与 instruction 搜索解耦。
- src/headless.rs:L1148-L1210 的 owner_workspace 与 resolve_workspace_path_at 提供 workspace 所有权、canonicalize、拒绝绝对路径/父遍历/符号链接。建议 InstructionService 接受已确认的 workspace root；对来自配置的 ~/、绝对路径是否保留兼容性要单独开关。若复用该安全策略，应把 TypeScript 的绝对 instruction glob 行为记录为有意差异，并用组件级 Path::starts_with 避免字符串前缀误判。
- src/session.rs:L1205-L1219 的 append-only journal 与 L1410-L1521 的 operation recovery 明确“中断操作不自动重试”。instruction claims 不应写入 operation marker；建议作为 Agent 或 workspace session 生命周期内的 HashMap<MessageId, HashSet<PathBuf>>，对应 TS 的 InstanceState。若要跨重启去重，才另行设计事件类型，当前源没有此要求。
- src/tool_runtime.rs:L1-L7 将调用者责任限定为 approval、journal、持久化和重试；指令读取是只读准备步骤，不应进入 ToolRegistry、ApprovalCoordinator 或 side-effect gate。其 L313-L328、L357-L425 的取消传播可作为读取/附加前的统一取消回调。
- src/runtime.rs:L49-L99 的 CancellationToken 是合作式取消，L318-L350 提供非阻塞 cancel/shutdown。建议远程 fetch 使用同一 token；5 秒 timeout 触发空结果，不能把取消误报为成功附加。若并行读取，使用 bounded worker 或已有 runtime 队列实现 8/4 上限，结果按输入顺序重排。
- src/protocol.rs:L1-L24 与 StdioRequest:L152-L209 是版本化 JSONL 控制协议，已有 text/path/attachments，没有 instruction 专用字段。建议先从 workspace/config 装载，不把 AGENTS.md 内容直接扩展为客户端可注入的协议字段；如必须支持远程配置，再新增经校验的配置动作而非复用 prompt 文本。
- src/approval.rs:L122-L187 把 approval 保持为工具副作用的同步 rendezvous，且取消可中断等待。instruction 文件和 HTTP GET 属于上下文读取，不应请求 approval；若产品政策把网络读取视为外部副作用，则需另加明确 policy，而不能从 provider 输出推断。
- 建议的可执行接口：InstructionConfig { global_config, home, instructions, disable_claude_code_prompt, disable_project_config }；InstructionService::system_paths(&WorkspaceContext) -> Result<Vec<PathBuf>, InstructionError>；system() -> Result<Vec<String>, _>；find(&Path) -> Option<PathBuf>；resolve(messages, filepath, message_id) -> Vec<ResolvedInstruction>；以及纯函数 extract_loaded。测试应覆盖层级优先级、Set 去重、claims/clear、已读 metadata、HTTP timeout/retry、并发上限和路径边界。
- 关键差异清单：TypeScript 的 FS/HTTP 错误多数变为空内容，Rust 需决定是否完全兼容；TypeScript 支持绝对配置 glob 与 ~/，headless Rust 默认拒绝绝对/符号链接路径；TypeScript 的 findUp/glob 行为依赖 worktree，Rust 需明确 workspace 与 worktree 的等价字段；TypeScript 在 resolve 中先 claim 后 read，Rust 必须保持该顺序才能避免空文件重复；TypeScript 本地/远程读取上限分别为 8/4，且输出按输入顺序稳定；Rust provider 路由和协议已支持 instruction 字段，无需在 src/providers/** 复制加载逻辑。

## 未决问题

- withTransientReadRetry 的具体重试次数、退避和哪些状态码属于 transient 不在本文件中定义。
- FSUtil.globUp/findUp 遇到符号链接、权限错误和 worktree 边界时的精确语义无法仅由本文件确认。
- InstanceState.make 对并发 resolve 调用的原子性未在本文件展示；源码只显示共享 Map 的读写，没有锁或去重竞态测试。
- 配置 instructions 的来源、刷新时机和 cfg.get() 的失败类型由 Config.Service 决定，本文件未展开。


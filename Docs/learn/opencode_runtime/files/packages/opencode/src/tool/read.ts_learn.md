# OC-047 — packages/opencode/src/tool/read.ts

```yaml
source_id: OC-047
item_id: OC-047
source_path: packages/opencode/src/tool/read.ts
source_hash: afec8294965cbd9b9e29d3453b682164f0ecdba26574ebc048e9ba21af209c68
source_bytes: 13126
source_lines: 386
coverage: bytes [0, 13126)；lines 1-386（按源文件顺序完整读取，含注释、类型与导出符号）
```

## 完整行为复盘

- 依赖与常量（L1-L19）：使用 `effect` 的 `Effect/Option/Schema/Scope/Stream`，`FSUtil`、`LSP`、`Instruction`、`InstanceState`、外部目录检查和媒体嗅探。`DEFAULT_READ_LIMIT=2000` 行，单行最多 `MAX_LINE_LENGTH=2000` 字符，输出总字节上限 `MAX_BYTES=50*1024`（标签为 `50 KB`），采样 `SAMPLE_BYTES=4096`；图片只接受 JPEG/PNG/GIF/WebP。
- `ReadStop`（L21）：`Schema.TaggedErrorClass` 定义的内部控制错误，仅用于达到字节上限时中止上游流，不是用户可见错误。
- 导出 `Parameters`（L23-L36）：输入是 `filePath: string`（声明为绝对路径，但执行层仍把相对路径解析到实例目录），可选 `offset`、`limit` 均为 `NonNegativeInt`。`offset` 文档为 1 起始行，`limit` 缺省 2000；注释说明不再把字符串运行时强制转换成数字，JSON Schema 仍是 number。
- `Display` 与 `Metadata`（L38-L62）：目录展示含 `entries/offset/totalEntries/truncated`；文件展示含文本、起止行、总行数和截断标志。所有结果 metadata 都有 `preview/truncated/loaded`，文件或目录再带 `display`。
- 导出 `ReadTool` 的构造（L64-L75、L379-L386）：通过 `Tool.define` 注册名称 `read`，依赖 `FSUtil.Service | Instruction.Service | LSP.Service | Scope.Scope`；描述来自 `read.txt`，参数为 `Parameters`，执行函数调用 `run(...).pipe(Effect.orDie)`，因此未捕获的 Effect 缺陷会终止为 die。
- `miss(filepath)`（L76-L99）：取父目录和 basename，读取目录后做大小写不敏感的双向包含匹配，最多建议 3 个候选并拼回完整路径；目录读取失败按空建议处理。找到候选时报 `File not found: ...\n\nDid you mean...`，否则只报 `File not found`。这是失败值，没有重试。
- `list(filepath)`（L101-L115）：读取目录项；普通目录追加 `/`，普通非符号链接保持名称，符号链接通过 `fs.stat` 判断目标是否目录，stat 失败则按非目录处理。每项处理以 `Effect.forEach(...,{concurrency:"unbounded"})` 并发执行，最后用 `localeCompare` 排序，保证展示顺序稳定但不保证 stat 顺序。
- `warm(filepath)`（L117-L120）：调用 `lsp.touchFile`，用 `Effect.ignoreCause` 忽略后台缺陷并 `Effect.forkIn(scope)` 异步派生；LSP 预热不会让成功读取变失败，也不等待其完成。
- `readSample(filepath,fileSize,sampleSize)`（L122-L135）：空文件直接返回空 `Uint8Array`；否则在 `Effect.scoped` 中以只读方式打开，读取 `min(sampleSize,fileSize)`，`Option` 缺值也转为空字节。作用域结束关闭文件。
- `lines(filepath,{limit,offset})`（L137-L180）：把 1-indexed offset 转为 `start=offset-1`，通过文件字节流、手工 `TextDecoder("utf-8", {stream:true})` 和 `Stream.splitLines` 逐行处理。手工 decoder 保留无换行结尾行；`ReadStop` 在字节封顶后停止上游。`flags.count` 统计已见行（包括 offset 前和 limit 后），offset 前只计数不保存；保存行数达到 limit 后置 `more=true`，仍继续扫描以得到总行数。每行先截到 2000 字符并追加 `... (line truncated to 2000 chars)`，再按 UTF-8 字节计数，后续行计一个换行字节；若加入会超过 50 KB，设置 `cut/more/done` 并抛 `ReadStop`。返回 `{raw,count,cut,more,offset}`。`limit=0` 会保存零行但扫描全文件；`offset=0` 不会在调用处生效，因为调用方用 `|| 1`。
- `isBinaryFile(filepath,bytes)`（L182-L227）：先按扩展名判定压缩、可执行、办公文档、对象文件等二进制；空采样不是二进制。否则扫描采样，遇 NUL 立即判定；统计控制字节（小于 9，或 13 与 32 之间），非打印比例严格大于 30% 才判定。它只检查 4096 字节样本，不能证明后续内容没有二进制数据。
- `run(params,ctx)` 路径与权限（L229-L263）：取得 `InstanceState.context`；非绝对路径相对 `instance.directory` 解析，Windows 调 `FSUtil.normalizePath`，标题是相对 `instance.worktree` 的路径。`fs.stat` 仅把 `NotFound` 转成 `undefined`，其他 I/O 错误继续失败。随后调用 `assertExternalDirectoryEffect`，`ctx.extra.bypassCwdCheck` 可绕过外部目录检查，kind 依据 stat 是否为目录；再以 `ctx.ask` 请求 `read` 权限，pattern 是相对 worktree 路径，`always:["*"]`。权限在缺失文件检查之前发生。
- 目录分支（L264-L297）：stat 为目录时调用 `list`；`limit=params.limit ?? 2000`，`offset=params.offset || 1`，按 `items.slice(offset-1, offset-1+limit)` 分页。`truncated` 表示仍有未显示项；输出包含 `<path>、<type>directory、<entries>`，并明确显示数量及下一 offset。metadata preview 最多 20 项，loaded 为空，display 保存完整切片与总数。
- 文件准备与附件（L300-L325）：先由 `instruction.resolve(ctx.messages,filepath,ctx.messageID)` 得到要附加的指令内容，再读 4 KB 样本，用 `sniffAttachmentMime(sample,FSUtil.mimeType(filepath))` 判 MIME。图片或 PDF 会完整 `fs.readFile`；返回成功短语、`truncated:false`、已加载指令路径，并以 `data:<mime>;base64,...` attachment 返回原始字节。此分支不执行 `isBinaryFile` 与 LSP warm。
- 二进制拒绝（L327-L329）：普通二进制文件直接失败 `Cannot read binary file: <filepath>`，不会生成部分文本。
- 文本结果与边界（L331-L377）：调用 `lines`，若 `count < offset` 且不是空文件 offset=1，失败 `Offset ... is out of range ...`。正文逐行前缀为实际行号；`cut` 优先提示达到 50 KB 并建议 `offset=next`，否则 `more` 提示 limit 分页，均给出范围；未截断提示 EOF 和总行数。之后异步 `warm`。若 `loaded` 非空，把指令内容包在 `<system-reminder>` 追加到输出，但 metadata.preview 只取前 20 行；display.text 是未加行号的原始裁剪行。

## 状态、取消、恢复与副作用

源文件没有持久化、重试或显式超时参数；`Effect` 的取消可沿文件流、scoped 文件句柄和权限等待传播，但 `lines` 的 `ReadStop` 是内部正常截断控制流（L142-L177）。目录项 stat 采用无界并发（L103-L114），文本流是单次顺序扫描；图片/PDF 一次性把整个文件读入内存（L306-L323）。读权限、外部目录检查和 instruction/LSP 服务是外部交互；实际文件操作均为只读。`warm` 派生任务受 `Scope` 管理，忽略其失败（L117-L120）；没有恢复游标或 journal，调用方只能使用提示的 `offset` 重新读取。

## 源内测试与行为判据

源文件及其同目录内容未包含测试引用，故为“源内未包含测试”。可独立验证：1）对不存在路径创建相似 basename，错误最多列 3 个建议；2）目录含目录、文件、指向目录和坏符号链接时检查 `/` 标记及排序；3）构造超过 2000 字符的行、UTF-8 多字节行和无末尾换行文件，核对行后缀、50 KB 截断和总行数；4）用 `offset=0/limit=0/offset` 超范围检查默认值与错误；5）对图片/PDF、扩展名二进制、含 NUL 或控制字节样本分别核对 attachment、二进制拒绝和文本路径；6）验证权限拒绝先于缺失文件错误，且 LSP 失败不改变成功结果。

## zenpi Rust 映射

- `src/tools.rs` 的 `ReadFileTool`（L1515-L1589）是最直接落点：对应 `ToolDefinition{name:"read_file", side_effect:ReadOnly}`、`ToolContext::resolve_existing`、UTF-8 校验和 bounded read。建议新增 `ReadTool` 或扩展 `ReadFileTool` 参数为 `path/offset/limit`，返回结构化 `path/type/content/line_start/line_end/total_lines/truncated`；保留 `ToolResult` 错误码（L548-L718）而不要把错误 `panic`。
- 路径安全应复用 `ToolContext` 固定 workspace（`src/tools.rs:L745-L773`）及 `src/headless.rs` 的相对路径、无 symlink、canonical 边界检查（L1159-L1210）。这与源文件允许绝对路径、并可由 `bypassCwdCheck` 绕过的语义有差异，必须在 Rust API 明确开关并默认拒绝越界。
- `src/tool_runtime.rs` 的 `execute_tool_batch`（L157-L249、L252-L346）提供并发批处理、源顺序结果和合作式取消；`ReadTool` 可声明 `ToolExecutionMode::Parallel`，但其内部目录 stat 的无界并发应改为有界线程/同步迭代以符合 zenpi 资源预算。`src/runtime.rs` 的 `CancellationToken`（L49-L87）可在逐块读取与逐行扫描时检查；取消不是回滚证据。
- `src/approval.rs` 的 `ApprovalPolicy` 将 read-only 自动 Allow、Headless 可拒绝（L490-L529），对应源 `ctx.ask(read)`；`src/core.rs` 的 tool batch/approval 编排（约 L4472-L4576、L5133-L5217）负责把失败包装为 `ToolResult`，不应让 `Effect.orDie` 的致命语义直接移植。
- `src/protocol.rs` 已有 `Resources` 命令（L236-L241、L424-L425）和 `ToolOutput`（L255-L260、L1237-L1245），但没有逐文件分页 read wire 类型。建议在 `src/protocol.rs` 增加 `ReadFileRequest{path,offset,limit}` 与结构化响应，复用严格字段/长度校验；目录资源快照可参考 `src/headless.rs:L1104-L1125`，但它是资源统计而非文件内容。
- `src/session.rs` 负责 journal/tree 和取消传入，适合作为可选 `read` 审计事件落点；当前源 read 本身无持久化，故不要把每个读取自动写 session。`src/headless.rs` 只应做 workspace 解析、JSONL 响应和 bounded snapshot，不复制 provider 调用。
- `src/providers/**`（`openai.rs`、`anthropic.rs`、`google.rs`、`codex.rs`、`deepseek.rs`、`connection.rs`、`registry.rs`）只描述模型能力、连接和文件/图片输入能力；read 的本地 FS 语义应停留在 `tools.rs`/`tool_runtime.rs`，仅在 provider 内容编码层把 `attachments` 映射为各协议的 file/image block。图片/PDF data URL 是源行为，Rust 需按 provider capability（registry 的 `files/images`）决定可否转发，不能让 provider 直接读取任意路径。

差异清单：源支持绝对路径和外部目录 bypass，zenpi 默认 workspace-relative；源有目录输出、50 KB/2000 字符行截断、MIME 附件和 instruction reminder，现有 Rust `read_file` 只有单一 UTF-8 字节上限；源 LSP warm 与 `Effect` 作用域无直接对应；源错误以字符串/Effect die 传播，zenpi 应使用 `ToolErrorCode`；源 `offset` 允许 schema 的 0 但执行归一化为 1，Rust 校验需固定该兼容规则。

## 未决问题

1. `read.txt` 的完整 description 文本未在本文件内展开，无法仅凭 `read.ts` 确认最终展示文案。
2. `FSUtil.stream` 的 chunk 大小、`Stream.splitLines` 对 CRLF 的精确规范化，以及 `sniffAttachmentMime` 的优先级需查看依赖实现才能完全确认。
3. `Instruction.resolve` 返回内容的来源、去重和安全过滤不在本文件定义；只能确认其结果会被追加到 system-reminder。

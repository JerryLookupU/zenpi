# OC-037 — packages/opencode/src/tool/edit.ts

- source_id: `packages/opencode/src/tool/edit.ts`
- item_id: `OC-037`
- source_path: `packages/opencode/src/tool/edit.ts`
- source_hash: `f84d9d242137e1f18ce912188efb7a97b71bf4f255e0a69f16a1fe9d1ff236d4`
- source_bytes: `24530`
- source_lines: `737`
- coverage: 已按源文件顺序读取字节 `1-24530`、行 `L1-L737`，包含全部注释、类型、导入、导出符号与函数体。

## 完整行为复盘

### 依赖、纯辅助函数与锁

- 文件开头的注释说明算法来源于 Cline/Gemini 的 diff edit 方案（`L1-L4`）。导入 `path`、Effect 的 `Schema`/`Semaphore`、`Tool`、`LSP`、`diff`、`FileSystem`/`Watcher`、`EventV2Bridge`、`Format`、`InstanceState`、`Snapshot`、外部目录检查、`FSUtil` 与 `Bom`，分别支撑路径、Effect 服务、工具 schema、补丁、事件、格式化、实例工作区、文件差异、路径治理和 BOM（`L6-L20`）。
- `normalizeLineEndings(text)` 将全部 `\\r\\n` 变成 `\\n`，不处理孤立 `\\r`（`L22-L24`）。`detectLineEnding(text)` 只要文本含一个 CRLF 就选择 `\\r\\n`，否则选择 `\\n`（`L26-L28`）；因此混合换行文件会统一按 CRLF 重建替换片段。`convertToLineEnding(text, ending)` 在目标为 LF 时原样返回，在 CRLF 时把 LF 换成 CRLF，调用方必须先标准化（`L30-L33`）。
- `locks` 是进程内 `Map<string, Semaphore.Semaphore>`；`lock(filePath)` 先用 `FSUtil.resolve` 得到键，命中则复用已有信号量，否则用 `Semaphore.makeUnsafe(1)` 创建单许可锁并永久放入 map（`L35-L45`）。它串行化同一解析路径的编辑，锁不会清理，也不能跨进程协调。
- 导出的 `Parameters` 是 `Schema.Struct`：`filePath` 绝对路径描述、`oldString`、`newString`（描述要求与旧值不同）为必填字符串，`replaceAll` 为可选布尔且语义默认 false（`L47-L56`）。schema 本身没有 `.default`，运行时实际默认由 `replace` 参数默认值提供。

### `EditTool` 与文件编辑主流程

`EditTool = Tool.define("edit", Effect.gen(...))` 在构造时取得 `LSP.Service`、`FSUtil.Service`、`Format.Service`、`EventV2Bridge.Service`，返回 `description`、`parameters` 和 `execute`（`L58-L69`）。`execute(params, ctx)` 的完整顺序如下（`L69-L215`）：

1. `filePath` 空字符串立即抛出 `filePath is required`；`oldString === newString` 抛出无变化错误（`L71-L77`）。从 `InstanceState.context` 取得实例目录；绝对路径原样使用，相对路径通过 `path.join(instance.directory, params.filePath)` 解析，并以 `assertExternalDirectoryEffect(ctx, filePath)` 检查外部目录（`L79-L83`）。
2. 在共享变量 `diff`、`contentOld`、`contentNew` 中保存本次结果；整个读、询问、写、格式化、事件发布过程包在 `lock(filePath).withPermits(1)(Effect.gen(...))` 内，且内层 pipe `Effect.orDie`（`L85-L89`、`L172-L173`）。因此同一路径的审批等待也会阻塞后续编辑；内层失败被转成 defect，调用者不能依赖普通 typed error 恢复。
3. **新建文件分支（`oldString === ""`）**：先 `afs.existsSafe`。若文件已存在，抛出要求提供精确旧文本、或改用 `write` 的错误（`L90-L96`）；不存在时用 `Bom.split(newString)` 分离 BOM 和文本，设置旧内容为空、新内容为无 BOM 文本，生成 `createTwoFilesPatch` 后经 `trimDiff`（`L97-L101`）。随后 `ctx.ask` 请求 `permission: "edit"`，pattern 为相对 `instance.worktree` 的路径，`always: ["*"]`，metadata 含绝对 `filepath` 与 diff（`L102-L110`）。获准后 `afs.writeWithDirs(filePath, Bom.join(contentNew, desiredBom))` 创建目录并写入；若 `format.file` 返回真，再以 `Bom.syncFile` 读取/同步格式化后的内容；发布 `FileSystem.Event.Edited`，再发布 `Watcher.Event.Updated` 且 `event: "add"`（`L111-L119`）。`return` 只结束锁内 effect，外层仍会汇总差异、触发 LSP 并返回结果。
4. **已有文件分支**：`afs.stat` 的失败被吞掉为 `undefined`，从而统一报 `File ... not found`；目录报 `Path is a directory, not a file`（`L123-L126`）。用 `Bom.readFile` 读取文本和 BOM；根据原文件换行风格，把 `params.oldString`、`params.newString` 先 `normalizeLineEndings` 再转换为原风格（`L126-L132`）。调用 `replace(contentOld, old, replacement, params.replaceAll)`；对结果再 `Bom.split`，优先保留源 BOM，否则采用新文本携带的 BOM（`L133-L135`）。写前 diff 对两侧都标准化为 LF 后生成，询问结构同新建分支（`L137-L153`）。获准后写入、可选格式化与 BOM 同步，发布 Edited 和 Updated（此处 `event: "change"`），最后重新计算 diff，使返回补丁反映格式化后的 `contentNew`（`L155-L171`）。
5. 出锁后用 `diffLines(contentOld, contentNew)` 累加 `change.added/count` 与 `change.removed/count`，形成 `Snapshot.FileDiff {file, patch, additions, deletions}`（`L175-L186`）。调用 `ctx.metadata` 写入 diff、filediff 和空 diagnostics（`L188-L194`）。随后 `lsp.touchFile(filePath, "document")`，读取 `lsp.diagnostics()`，以 `FSUtil.normalizePath` 查找当前文件诊断，通过 `LSP.Diagnostic.report` 生成可读块；若有块，在成功文本后追加 “LSP errors detected...” 和错误内容（`L196-L201`）。最终返回 metadata（diagnostics/diff/filediff）、相对工作树的 title 和 output（`L203-L211`）。

### 替换器类型与逐导出符号行为

- `export type Replacer = (content, find) => Generator<string, void, unknown>` 定义惰性候选生成器协议（`L217`）。相似度阈值 `SINGLE_CANDIDATE_SIMILARITY_THRESHOLD`、`MULTIPLE_CANDIDATES_SIMILARITY_THRESHOLD` 均为 `0.65`（`L219-L221`）。
- `levenshtein(a,b)` 为空串时返回另一方长度；否则分配 `(a.length+1)*(b.length+1)` 矩阵，按字符替换/插入/删除取最小值，返回编辑距离（`L223-L242`），时间和空间均为 O(|a|·|b|)。
- `SimpleReplacer` 无条件 yield `find`（`L244-L246`），只让后续 `indexOf` 判断精确命中。
- `LineTrimmedReplacer` 按 LF 分行，去掉搜索串末尾空行；滑窗逐行 `trim()` 比较，命中后用原行长度重新计算字符区间并 yield 原始（未裁空格）片段，可能产生多个候选（`L248-L286`）。
- `BlockAnchorReplacer` 要求至少三行，去掉搜索尾空行，以首尾行（trim 后）作锚点；候选块长度与搜索块长度差不得超过 `max(1,floor(size*0.25))`，每个首锚点只取之后第一个尾锚点（`L288-L327`）。单候选时只比较中间行，按 Levenshtein 相似度平均，达到 0.65 可提前接受；无中间行仅凭锚点接受，并 yield原字符区间（`L329-L373`）。多候选时计算每个平均相似度，保留首次达到最大值的候选，最大值至少 0.65 才 yield（`L375-L425`）。空中间行仍占分母，可能拉低相似度。
- `WhitespaceNormalizedReplacer` 把连续空白压成一个空格并 trim；先逐行整行匹配，失败时对规范化子串按搜索词构造转义正则（词间 `\\s+`），在原行中 yield 实际子串，非法正则被 catch 跳过；搜索含多行时再做等行数窗口的整体规范化匹配（`L427-L469`）。
- `IndentationFlexibleReplacer` 对非空行求最小前导空白，移除共同缩进而保留空行；对等行数窗口比较去缩进后的块，yield原块（`L471-L497`）。全为空行时返回原文，因而不改变匹配内容。
- `EscapeNormalizedReplacer` 的内部 `unescapeString` 仅解释 `\\n`、`\\t`、`\\r`、单双引号、反引号、反斜杠、反斜线换行和 `\\$`，未知转义保留（`L499-L525`）。若内容直接含解码后的 find 先 yield 解码串；再对窗口解码，解码块相等时 yield 原始转义块（`L527-L546`），可能有重复候选。
- `MultiOccurrenceReplacer` 用 `indexOf` 从上次命中末尾继续，yield 每个非重叠精确出现；`find` 为空会无限循环，但公共 `replace` 已拒绝空旧串（`L548-L560`）。
- `TrimmedBoundaryReplacer` 只在 `find` 本身带边界空白时工作；先尝试全文包含 `find.trim()`，再按原行数窗口比较 `block.trim()`，yield裁剪版或原块（`L562-L586`）。
- `ContextAwareReplacer` 至少需要三行，去掉尾空行，以首尾 trim 行定位块；只有块行数完全相等且中间非空行至少 50% trim 后相等才 yield。每个首锚点遇到第一个尾锚点就停止继续搜索；内层成功后只 break 内层，外层仍可能在后续首锚点再次 yield（`L588-L644`），实际不保证全局唯一。

### diff 与最终 `replace`

- `trimDiff(diff)` 保留 diff 中以 `+`、`-`、空格开头的内容行，排除 `---`/`+++` 文件头；若无内容、无非空缩进或最小缩进为 0，则原样返回。否则计算非空内容行的最小前导空格，并从所有内容行（含空行）删除该数量，文件头和其它行不改（`L646-L680`）。
- `replace(content, oldString, newString, replaceAll=false)` 先拒绝相等旧/新值及空 `oldString`，错误消息分别指出无变化、现有文件编辑不能用空旧串（`L682-L690`）。它按 `SimpleReplacer → LineTrimmedReplacer → BlockAnchorReplacer → WhitespaceNormalizedReplacer → IndentationFlexibleReplacer → EscapeNormalizedReplacer → TrimmedBoundaryReplacer → ContextAwareReplacer → MultiOccurrenceReplacer` 顺序消费候选（`L692-L705`）。每个候选必须能被 `content.indexOf` 找到；先用 `isDisproportionateMatch` 防止宽松候选远大于请求跨度（`L706-L713`）。`replaceAll` 为真时立即 `content.replaceAll(search,newString)`；否则只有 `index === lastIndex` 的全局唯一命中才拼接替换，重复命中会继续寻找下一策略（`L714-L721`）。全无候选抛“Could not find oldString...”精确匹配错误；曾找到但始终不唯一则抛“Found multiple matches...”要求更多上下文（`L723-L728`）。
- `isDisproportionateMatch(search, oldString)` 以行数和字符数双重限制宽松匹配：若候选行数至少 `max(oldLines+3, oldLines*2)` 拒绝；单行旧串不做字符长度限制；多行时若 trim 后候选长度大于 `max(old+500, old*4)` 拒绝（`L731-L737`）。

## 状态、取消、恢复与副作用

- **状态与并发**：可变状态只在一次 `execute` 局部变量（`diff`、`contentOld`、`contentNew`）和模块级 `locks` 中；同一 `FSUtil.resolve` 键由单许可 `Semaphore` 串行，跨不同文件可并行，跨进程无锁。锁 map 无淘汰，长时间运行会积累路径键（`L35-L45`、`L85-L89`）。审批请求在锁内发出，所以等待人工审批期间同文件调用排队。
- **取消/超时**：源码没有显式 `Effect.timeout`、取消 token、重试循环或 deadline。Effect 的运行时取消可在任意 yield 点中断；`ctx.ask`、文件 I/O、格式化、事件发布和 LSP 服务是否响应取消由依赖实现决定。`Effect.orDie` 会把锁内失败转为 defect；没有把已写文件回滚的逻辑（`L88-L89`、`L172-L173`）。
- **恢复/重试**：没有 session journal、operation id、幂等键或自动 retry。写入已经发生但后续 `format.file`、`Bom.syncFile`、事件发布、LSP touch 或诊断失败时，文件可能已改变而工具调用整体失败；再次调用可能重复替换或因 oldString 不再存在而失败（`L111-L120`、`L155-L171`）。
- **持久化与外部副作用**：`ctx.ask` 是编辑许可副作用；`afs.writeWithDirs` 改变磁盘并创建父目录；`Format.Service` 可能再次修改文件；`EventV2Bridge` 发布 Edited/Updated；`ctx.metadata` 只写调用上下文；`LSP.touchFile`/`diagnostics` 更新语言服务视图。`Snapshot.FileDiff`、diff 和 additions/deletions 只作为返回元数据，没有本文件内的 durable snapshot（`L102-L119`、`L188-L211`）。
- **边界风险**：源文件以 `Bom.readFile`/`Bom.join` 保留 BOM，以原文件的首个 CRLF 选择换行；格式化后重新同步 BOM，但没有并发外部进程变更检测或 source hash 校验。`assertExternalDirectoryEffect` 只提供路径治理入口，实际规则在依赖模块（`L79-L83`）。

## 源内测试与行为判据

源文件及 `src/tool` 同目录没有测试块；仓库检索没有发现对这些导出 replacer 或 `replace` 的同目录测试，因此结论为“源内未包含测试”。可独立验证的判据如下：

1. 对既有文件验证 LF、CRLF、混合换行及 UTF-8 BOM：替换后应保持源 BOM，并将旧/新串转换为检测到的换行风格；`createTwoFilesPatch` 的返回 diff 经 `trimDiff` 后只去公共缩进（`L22-L33`、`L126-L171`、`L646-L680`）。
2. 验证参数错误：空 `filePath`、相同 old/new、既有文件空 `oldString`、不存在文件、目录路径分别命中源码中的固定错误文本（`L71-L77`、`L90-L96`、`L123-L126`）。
3. 构造一个 oldString 精确出现两次的文件：`replaceAll=false` 应报多匹配；`replaceAll=true` 应替换该候选的所有非重叠出现。构造仅有缩进/空白差异的文本，按 replacer 顺序验证唯一候选与宽松匹配拒绝（`L692-L728`）。
4. 审批后检查写入前后事件顺序：`ctx.ask` 必须先于 `afs.writeWithDirs`；新建分支 Updated 的 `event` 为 `add`，已有文件为 `change`，格式化成功后最终 diff 反映格式化内容（`L102-L119`、`L145-L171`）。
5. 以并发任务同时编辑同一绝对/相对别名路径，确认 `FSUtil.resolve` 后共用 semaphore；不同文件可以同时运行。再在 LSP 返回诊断时确认 output 追加诊断而不撤销写入（`L35-L45`、`L196-L211`）。
6. 为每个导出 Replacer 提供生成器测试：尾随空行、三行锚点、Levenshtein 0.65 边界、正则转义、未知 escape、重叠 occurrence、trimmed boundary 和多处 ContextAware 首锚点，确保实际 yield 数量与上述逐符号规则一致。

## zenpi Rust 映射

### 现有执行链与可落点

- `zenpi/src/tools.rs` 已有最接近的落点：`EditFileTool` 定义为 `WorkspaceWrite`，schema 是 `path/old/new` 且只允许一次精确 occurrence（`src/tools.rs:L2108-L2170`）；`approval_preview` 读取有界文本、计算 source SHA-256 和 unified diff（`L2111-L2149`），`invoke` 在 `context.admit_builtin`、路径解析和 `check_path_gate` 后用 `read_bounded_text`、`matches(old).count()==1`、`replacen`、`atomic_write_text`（`L2173-L2215`）。建议新增 `src/edit.rs` 或扩展 `tools.rs`：把本文件的 `normalizeLineEndings`/`detectLineEnding`/`Bom` 规则映射为 Rust helper；把九个 `Replacer` 实现为 `trait Replacer { fn candidates(...) -> Vec<String> }` 或迭代器；把 `replace` 和 `trim_diff` 作为纯函数并为每个阈值写单元测试。若保留 zenpi 当前“exactly once”安全策略，应把宽松 fallback 设为显式兼容模式，避免改变现有拒绝多匹配语义。
- `src/core.rs` 的 `ToolRuntime` 保存 `registry/context/policy/approval`（`L448-L459`），`set_tools` 安装 runtime 并从 session 事件恢复 remembered approvals（`L1413-L1437`）。`prepare_tool` 先校验 skill、doom-loop、Blueprint gate、`SideEffectPolicy` 和 `ApprovalPolicy`，再生成 preview、等待 `ApprovalCoordinator`、持久化 approval 事件并写 `tool_execution_started`（`L4674-L4778`、`L4780-L4864`、`L4940-L4972`）。这是 `ctx.ask` 的 Rust 落点：编辑 preview 应由 `EditFileTool::approval_preview` 产生，审批通过后再 dispatch。
- `src/tool_runtime.rs` 的 `execute_tool_batch` 对批次做大小/重复 ID/参数截断校验，顺序模式在取消后停止，线程模式轮询取消并 join 全部 worker；非只读工具遇到 IO、timeout、cancel 会阻止后续调用（`L157-L247`、`L252-L347`）。`edit.ts` 的 per-file semaphore 应落在 `EditFileTool` 或 `ToolContext` 的 `Arc<Mutex<HashMap<PathBuf, Semaphore>>>`，并在整个 preview/approval/write 区间持锁；批次 executor 只负责调用级并发，不能替代路径锁。
- `src/core.rs` 在 `run_tool_batch` 之前为每个 call 写 operation marker，调用 `execute_tool_batch`，按源顺序 `persist_tool_invocation`；结果压缩失败会标为 `UnknownOutcome`（`L4470-L4627`、`L5197-L5244`），写结果和 `tool_execution_finished` 再完成 session operation（`L5264-L5344`）。这比 `edit.ts` 更强：Rust 映射应把 edit 的写前 preview、写后 diff、格式化失败都纳入 operation outcome，不能直接把取消当回滚。

### 取消、恢复、协议与宿主对照

- `src/runtime.rs` 的 `CancellationToken` 是协作式、幂等 cancel；`is_cancelled` 读取 atomic，`mark_completed` 防止完成后的迟到取消改写成功结果（`L49-L99`），`JobOutcome::Cancelled` 明确不是 rollback（`L158-L168`）。映射 `EditTool` 时把 token 传入读、审批、写、format、LSP 适配器，并在写入前后分别检查；无法中断的文件系统调用必须 join，不应 detached。
- `src/protocol.rs` 已有 `Command::Approve`、`Command::Cancel`，审批字段 `approval_id/decision/remember` 经 `into_command` 校验（`L211-L263`、`L501-L513`）；`StdioResponse`/`StdioEvent` 提供相关 request/turn/sequence 的 JSONL envelope（`L854-L900`）。因此不应为 edit 添加专用协议；沿用 `ToolPreview::Diff`、approval event 和工具结果即可。
- `src/approval.rs` 的 `ApprovalRequest` 包含 `side_effect`、原始 arguments、可选 `ToolPreview`、origin、policy/lease，并验证 preview、worker correlation（`L40-L104`）；`ApprovalCoordinator::request_response` 可被取消，接受的 response 在持久化前保留（`L126-L200`）。`ApprovalPolicy` 对只读自动 allow，对写入按模式/每工具决定，remembered events 只恢复人类选择（`L472-L577`）。这正好承接 `ctx.ask` 的 edit permission，但 Rust 应在 preview 中记录 source digest、before/after bytes，并在执行前复核 preview，防止审批后工作区变化。
- `src/headless.rs` 的同步 `run_headless` 有 frame/UTF-8/协议校验和 replay admission（`L556-L709`）；异步 stdio 将输入读取与有界 runtime 分离（`L786-L812`）。`drain_approval_events` 将 pending approval 变成带 `approval_id`、tool、arguments、turn/request correlation 的事件（`L4177-L4248`），`approve` 命令调用 coordinator（`L5767-L5835`），cancel 会同时 `runner.try_cancel` 与 `approval.emergency_cancel`（`L6038-L6063`）。Rust edit 的审批等待应接入这些既有宿主通道，而非阻塞 stdin。
- `src/session.rs` 将 Tool operation 建模为 `OperationKind::Tool`，结果有 `Succeeded/Failed/Cancelled/Interrupted/UnknownOutcome`，marker 的 `retry_requires_confirmation` 可为真（`L49-L95`）。`operation_recovery` 对无 durable terminal record 的工具保留 UnknownOutcome，`decide_operation_recovery` 只记录显式决定，retry 要求新 operation（`L1492-L1589`）。这弥补 `edit.ts` 没有恢复/重试的差异：Rust edit 写入前先 `begin_operation`，写后 durable result，再 finish；崩溃时禁止自动重复替换。

### provider 层差异

- `src/providers/registry.rs` 的 `ModelDescriptor/ProviderCapabilities` 记录 `tools`、`files`、streaming 等能力（`L75-L158`、`L195-L231`），未知模型也有保守能力默认（`L405-L451`）；`src/providers/connection.rs` 将模型能力与 wire API 求交，缺少显式证据时关闭 rich fields（`L288-L377`）。`src/providers/anthropic.rs`、`codex.rs`、`deepseek.rs`、`google.rs`、`openai.rs` 只提供各 provider 的协议/能力常量，没有本地文件编辑实现。故 edit 应由本地 `ToolRegistry` 执行，provider 只负责声明/传输工具 call；在 provider 不支持 `tools` 时，调用前由 route capability 拒绝，不把编辑降级成 prompt 指令。
- 可执行验证：给 `EditFileTool` 加 schema/preview/execute 测试，接入 `ToolRegistry::approval_preview` 与 `execute_cancellable`；在 `core` 测试中模拟审批后修改源文件，必须返回 `StalePreview` 或等价拒绝；在 `session` 测试中模拟写入后进程中断，`operation_recovery` 必须是 `UnknownOutcome` 且 retry 只允许新 call；在 `runtime` 测试中取消审批等待，确认没有写盘；在 `headless` 测试中验证 approval/cancel JSONL correlation；provider 测试只需确认 `tools` capability gating。

## 未决问题

1. `assertExternalDirectoryEffect` 的外部目录允许/拒绝规则不在本文件，无法仅凭 `edit.ts` 确认是否允许绝对路径越出 worktree（调用点见 `L79-L83`）。
2. `FSUtil.resolve` 是否做真实 canonicalization、符号链接处理及跨平台大小写折叠未在本文件定义，因此锁键是否覆盖所有路径别名需要依赖实现确认（`L37-L44`）。
3. `afs.writeWithDirs` 是否原子写入、崩溃时是否可能留下半写文件，以及 `Bom.syncFile` 的冲突处理策略未在本文件确认（`L111-L113`、`L155-L157`）。
4. `Effect.orDie` 在当前宿主如何序列化 defect、`ctx.ask`/LSP/Format 是否可取消、取消发生在写入之后时宿主如何报告，源文件没有说明（`L88-L89`、`L102-L113`、`L196-L201`）。
5. `Format.Service.file` 返回布尔值的含义、是否保证读取到的内容已经是最终格式化版本，以及格式化失败后的持久化语义需查看 `Format` 实现；本文件只能确认它成功为真时调用 `Bom.syncFile`（`L112-L114`、`L156-L158`）。
6. `ctx.ask` 中 `always: ["*"]` 的权限记忆/范围和 `metadata.diff` 的大小限制不由本文件定义；Rust 映射应以 `ApprovalCoordinator`/`ToolPreview` 的有界校验为准，而不能假设两者完全等价。

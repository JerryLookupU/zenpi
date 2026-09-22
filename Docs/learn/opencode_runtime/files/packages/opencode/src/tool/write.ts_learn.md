# OC-061 — packages/opencode/src/tool/write.ts

source_id/item_id: `OC-061`
source_path: `packages/opencode/src/tool/write.ts`
source_hash: `861de91cc67849e138c32a697c8eb1abab8e4912bfd97811ea1866b8e9af4096`
source_bytes: `3957`
source_lines: `104`
coverage: 字节 `0-3956`（完整 3957 字节）；行 `L1-L104`（从首行 import 到末行 `)`，含全部注释、类型和导出符号）。

## 完整行为复盘

- `L1-L16` 是依赖边界：`Schema`/`Effect` 提供参数校验和 Effect 组合，`path` 做路径运算，`Tool` 定义工具契约；`LSP`、`FileSystem`、`Watcher`、`EventV2Bridge`、`Format`、`FSUtil` 是注入服务；`Bom` 处理字节顺序标记；`createTwoFilesPatch` 与 `trimDiff` 生成审批 diff；`DESCRIPTION` 来自 `write.txt`。
- `MAX_PROJECT_DIAGNOSTICS_FILES`（`L18`）固定为 `5`，只限制“其他文件”的诊断数量，不限制当前文件诊断。
- 导出 `Parameters`（`L20-L25`）是 `Schema.Struct`：`content` 必须是字符串，`filePath` 必须是字符串。描述要求绝对路径且不能相对，但这只是 schema 描述，执行层对相对路径仍有明确回退逻辑（见下文），没有在此处强制拒绝。
- 导出 `WriteTool`（`L27-L104`）通过 `Tool.define("write", Effect.gen(...))` 注册名为 `write` 的工具。初始化阶段（`L29-L34`）从 Effect 环境取得 `LSP.Service`、`FSUtil.Service`、`EventV2Bridge.Service`、`Format.Service`；缺少任一服务会使构造 Effect 失败。返回对象（`L35-L38`）暴露说明文本、`Parameters` 和 `execute`。
- `execute` 的输入类型为 `{ content: string; filePath: string }` 与 `Tool.Context`（`L38-L39`）。首先读取 `InstanceState.context`（`L40`）。路径若已绝对则原样使用，否则拼接 `instance.directory`（`L41-L43`）；因此运行时接受相对路径。`assertExternalDirectoryEffect(ctx, filepath)`（`L44`）先做外部目录/访问边界检查，失败即停止。
- 文件快照阶段（`L46-L52`）：`fs.existsSafe` 判断存在性；存在时用 `Bom.readFile` 得到 `{ bom, text }`，不存在则采用 `{ bom:false, text:"" }`。新内容经 `Bom.split` 拆为 BOM 标志和正文；`desiredBom = source.bom || next.bom` 表示旧文件有 BOM 时优先保留，否则沿用新输入 BOM。`contentOld`/`contentNew` 只用于正文 diff 和写入。
- 审批预览（`L53-L62`）先用同一路径作为 old/new 文件名调用 `createTwoFilesPatch`，再 `trimDiff`。随后 `ctx.ask` 请求 `permission: "edit"`；`patterns` 是相对 `instance.worktree` 的路径，`always: ["*"]`，metadata 同时携带绝对 `filepath` 与 diff。拒绝、审批服务错误或 Effect 取消都会阻止后续写入；源内没有再次比较快照的逻辑。
- 写入与格式化（`L64-L67`）严格按顺序发生：`fs.writeWithDirs(filepath, Bom.join(contentNew, desiredBom))` 会写正文并按需创建父目录；若 `format.file(filepath)` 返回真值，再调用 `Bom.syncFile(fs, filepath, desiredBom)` 同步/修正 BOM。格式化是写入后的第二次潜在修改，因此最终内容可能与 `contentNew` 不同。
- 事件副作用（`L68-L72`）在写入/格式化成功后发布 `FileSystem.Event.Edited`，再发布 `Watcher.Event.Updated`；后者的 `event` 为既有文件的 `"change"`，新文件为 `"add"`。两次 publish 依次执行，任一失败都会使整个 Effect 失败，但已完成的文件写入不会回滚。
- 结果组装（`L74-L100`）：初始输出为 `Wrote file successfully.`；先 `lsp.touchFile(filepath, "document")`，再取得完整 `lsp.diagnostics()`。路径以 `FSUtil.normalizePath` 归一化（`L75-L77`）；遍历诊断映射（`L79-L90`）时当前文件总是报告，其他文件最多报告五个，空报告块跳过。当前文件追加“`LSP errors detected in this file, please fix:`”，其他文件追加“`LSP errors detected in other files:`”。返回值（`L92-L100`）含相对 worktree 的 `title`、`diagnostics`/绝对 `filepath`/原始 `exists` 的 metadata，以及最终 output。
- `execute` 以 `Effect.orDie` 收尾（`L101`）：未被业务转换的失败会升级为 defect，而不是结构化工具错误；`WriteTool` 的外层 Effect 在 `L103-L104` 闭合。

## 状态、取消、恢复与副作用

该源没有显式超时、重试、恢复令牌或会话持久化。Effect 的合作式取消只能在各个 `yield*` 边界生效；取消发生在 `writeWithDirs` 或格式化之后不会撤销文件、目录、事件或 LSP 状态。审批等待期间可取消，但 `ctx.ask` 的超时/取消具体语义未在本文件定义。没有锁或版本号，审批 diff 与真正写入之间存在外部修改竞态。持久化副作用仅是目标文件及可能新建的父目录；`Bom.syncFile`、格式化器、`events.publish` 和 `lsp.touchFile`/`diagnostics` 都是外部服务副作用。`Effect.orDie` 也意味着失败后的重试策略由上层决定，本函数不会自动重试；部分成功后重试可能再次格式化并重新发布事件。

## 源内测试与行为判据

源文件没有测试导入或测试块；在同目录测试 glob 中也未找到 `WriteTool`、`write_file` 或 `write.ts` 的测试引用，故为“源内未包含测试”。可独立验证的判据：

1. 给定不存在的相对路径，审批 metadata 的 `filepath` 应为 `instance.directory` 下的绝对路径，写入后 `Watcher.Event.Updated.event` 应为 `add`。
2. 给定已有 BOM 文件和无 BOM 新内容，最终文件仍应保留 BOM；给定无 BOM 旧文件和带 BOM 新内容，最终文件应带 BOM。
3. 审批拒绝时不得调用 `writeWithDirs`；审批通过后应依次出现文件写入、可选格式化、`Edited`、`Updated`、LSP touch/diagnostics。
4. 诊断输出必须包含当前文件全部可报告错误，其他文件最多五个，返回 metadata 的 `exists` 必须反映写入前状态。

## zenpi Rust 映射

- 主落点是现有 `src/tools.rs::WriteFileTool`：`definition`/`invoke` 对应 `Parameters` 与 `execute`，`ToolPreview::Diff` 对应 `createTwoFilesPatch`/`trimDiff` 的审批预览；`ToolContext::resolve_for_write`、`atomic_write_text` 已提供工作区约束、临时文件写入、`sync_all`、rename 和目录落盘。若要完整复刻 BOM，建议在 `src/tools.rs` 或独立 `src/file_write.rs` 增加 `BomState`、`split_bom`、`join_bom`，并在 `WriteFileTool::invoke` 与 `approval_preview` 共用。
- `src/approval.rs::ApprovalCoordinator` 和 `ToolBatchDecision::ExecuteApproved`（`src/tool_runtime.rs`）承接 `ctx.ask`；Rust 预览带 `source_sha256`，执行前会重新生成预览并拒绝 `StalePreview`，比源实现多出明确的快照竞态保护。写工具应保持 `ToolSideEffect::WorkspaceWrite`，不能因 provider 名称自动放行。
- `src/tool_runtime.rs::execute_tool_batch` 提供取消轮询、顺序执行和 join 语义；`src/runtime.rs::CancellationToken` 是宿主取消入口。建议在 `WriteFileTool::invoke_cancellable` 的文件写入前后检查 token，但要记录“取消不是回滚”，与 Rust 现有语义一致。
- `src/core.rs` 负责工具注册、`AgentEvent::ToolCall/ToolResult`、审批策略及错误归类；若需要 `output` 文本，可由 `ToolResult::Success` 生成“Wrote file successfully.”并附诊断摘要。`src/headless.rs` 将这些结果编码到 JSONL/事件流，适合承接源中的事件通知。
- `src/session.rs` 是追加式 JSONL 持久化，当前写工具本身不写会话；若产品要求恢复审计，应在 core 的操作开始/结束处记录 operation，而不是把文件内容写入 session。`src/protocol.rs` 的 `tool_output`、`approve`、`cancel`、`resume` 命令可暴露审批/结果/取消边界。
- `src/providers/**`（`anthropic`、`openai`、`codex`、`deepseek`、`google` 等）只负责模型协议、路由与认证，不应实现文件写入；provider 返回的 tool call 仍由 `src/core.rs` → `src/tool_runtime.rs` → `src/tools.rs` 本地校验和执行。

关键差异清单：源接受绝对路径并仅由 `assertExternalDirectoryEffect` 判定外部目录，zenpi 默认要求 workspace-relative 且拒绝凭据目录、符号链接和私有输出 inode；源未见 `MAX_WRITE_BYTES`/diff 上限，zenpi 已有 512 KiB 写入与 64 KiB diff 限制；源使用 `fs.writeWithDirs`，zenpi 的 `atomic_write_text` 具有临时文件和目录 fsync 的更强原子/耐久语义；源显式保留/同步 BOM、格式化并触发 LSP/Watcher，zenpi 现有 `WriteFileTool` 主要返回 bounded diff，需要补 BOM、formatter、LSP 诊断和等价事件适配；源的 `Effect.orDie` 把未处理错误变成 defect，zenpi 应映射为 `ToolErrorCode`；源只限制其他文件诊断五个，Rust 侧若无 LSP 服务应明确返回空诊断而非伪造结果。

## 未决问题

1. `assertExternalDirectoryEffect` 对 workspace 外路径的精确允许规则，以及 `ctx.ask` 的 `always: ["*"]` 是否会持久化授权，无法从本文件确认。
2. `FSUtil.existsSafe`、`writeWithDirs`、`Bom.syncFile` 是否保证原子替换、权限继承和目录 fsync 未在本文件定义。
3. `format.file` 的真值含义、格式化失败后的文件状态，以及 `lsp.diagnostics()` 是否等待索引完成无法从本文件确认。
4. 两类事件的 publish 是否可靠投递、是否可重复，以及 `Effect` 取消恰好发生在各副作用中间时的上层恢复协议无法从本文件确认。

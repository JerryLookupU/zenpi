# OC-035 — packages/opencode/src/tool/apply_patch.ts

## 元信息

- source_id/item_id：OC-035
- source_path：packages/opencode/src/tool/apply_patch.ts
- source_hash：9bc5a1384a5e1b1cdd21943cb529dcde6e5e596706dee2111ccdb999d6b0ba22
- source_bytes：11051
- source_lines：313
- coverage：完整读取字节 `0-11050`（11051 字节）与行 `L1-L313`，包含全部 import、注释、类型、导出及闭合括号。

## 完整行为复盘

1. **导出与依赖（L1-L33）**
`L1-L16` 导入 `path`、Effect/Schema、`Tool`、`EventV2Bridge`、`Watcher`、`InstanceState`、`Patch`、`diff`、外部目录校验、`trimDiff`、`LSP`、`FSUtil`、`DESCRIPTION`、`FileSystem`、`Format`、`Bom`。补丁语法由 `Patch.parsePatch` 与 `Patch.deriveNewContentsFromChunks` 提供（依赖 `packages/opencode/src/patch/index.ts:L185-L241,L307-L484`）。`L18-L20` 导出 `Parameters`，要求对象字段 `patchText: string`，无默认值；参数测试确认缺失或非字符串被拒绝（`test/tool/parameters.test.ts:L94-L106`）。
`L22-L29` 导出 `ApplyPatchTool`，工具 ID 为 `"apply_patch"`，初始化取得 `LSP.Service`、`FSUtil.Service`、`Format.Service`、`EventV2Bridge.Service`。`L30-L33` 定义 `ApplyPatchTool.execute` 的 `run(params, ctx)`；`L306-L311` 返回描述、参数和 `execute`，并以 `Effect.orDie` 把失败提升为 defect。外层 Tool 框架仍负责 Schema 解码、输出截断和 span。

2. **输入、解析与验证（L34-L75）**
`L34-L36` 空 `patchText` 立即失败：`patchText is required`。`L38-L45` 调 `Patch.parsePatch`，异常统一包装为 `apply_patch verification failed: <error>`。`L47-L53` 无 hunk 时先把 CRLF/CR 归一为 LF 并 trim；精确的 `*** Begin Patch\\n*** End Patch` 报 `patch rejected: empty patch`，其他无 hunk 报 `...no hunks found`。因此完整解析/验证在写入前完成。`L55-L70` 获取 `InstanceState.context`，准备内存 `fileChanges` 和累计 `totalDiff`，每条保存绝对源路径、旧/新内容、`add/update/delete/move`、可选 `movePath`、diff、增删行数、BOM。`L72-L75` 按 hunk 顺序用 `path.resolve(instance.directory, hunk.path)` 定位，并调用 `assertExternalDirectoryEffect`；工作区外目录会请求 `external_directory` 权限。

3. **add/update/delete 预检（L77-L191）**
add：`L77-L103` 旧内容为空；非空且无结尾 LF 时追加 LF（`L79-L80`）；`Bom.split` 让 diff 不含 BOM，`diffLines` 统计增删（`L81-L89`），保存 `bom` 并累计 diff。
update：`L106-L113` stat，缺失或目录失败并报告 `Failed to read file to update`。`L115-L119` 用 `Bom.readFile` 读 UTF-8；`L120-L131` 以 `Bom.join` 的原文调用 `Patch.deriveNewContentsFromChunks`。该依赖按 chunk 顺序匹配，支持上下文、EOF、纯插入、精确/去尾空白/trim/Unicode 归一；匹配失败包装为 verification failed。`L133-L140` 生成 diff 与行数；`L142-L158` 解析移动目标、再次做外部目录校验，带 `move_path` 的类型为 `move`，否则为 `update`。
delete：`L161-L170` 读文件失败统一包装；`L171-L187` 生成到空串的 diff，删除计数为 `contentToDelete.split("\\n").length`，保存源 BOM。所有这些只读/计算，尚未写文件。

4. **审批元数据（L193-L215）**
`L193-L203` 生成 files 元数据：`relativePath` 相对 `instance.worktree`，移动使用目标；Windows `\\` 归一为 `/`；包括 patch、additions、deletions 和可选绝对 `movePath`。`L205-L215` 以所有源路径构造一次 `ctx.ask({ permission: "edit", patterns: relativePaths, always: ["*"], metadata })`，metadata 含 filepath、完整 totalDiff、files。审批拒绝/取消会阻止写入；移动的 permission pattern 是旧路径，UI relativePath 是新路径。

5. **写入、格式化、事件（L217-L271）**
`L217-L220` 建立 updates，按 fileChanges 顺序串行处理，无并行、锁或事务。add `L221-L228`：`writeWithDirs(Bom.join(...))`，记录 add；update `L230-L233`：覆写原路径，记录 change；move `L235-L243`：先写目标再 remove 源，记录 unlink/add，源删除失败不会回滚已写目标；delete `L246-L249`：remove 并记录 unlink。
对非 delete，`L252-L257` 调 `format.file(edited)`；若真则 `Bom.syncFile`，随后发布 `FileSystem.Event.Edited`。`L260-L263` 再逐条发布 `Watcher.Event.Updated`。`L265-L271` 对非 delete 目标调用 `lsp.touchFile(target, "document")`，再一次性取得 diagnostics；delete 不 touch。

6. **摘要和返回值（L273-L311）**
`L273-L293` 按 worktree 相对路径生成 `A/D/M` 摘要；output 以 `Success. Updated the following files:` 开头，诊断非空时追加 `LSP errors detected in <rel>, please fix:`。`L295-L303` 返回 `{ title: output, metadata: { diff: totalDiff, files, diagnostics }, output }`；`L306-L311` 暴露工具定义。多 hunk 在本调用内严格源顺序执行。

## 状态、取消、恢复与副作用

- 持久状态只在调用内的 `fileChanges`、`totalDiff`、`updates`；没有 session journal、checkpoint、幂等键、重试计数或恢复记录。
- `Tool.Context` 有 `abort: AbortSignal`，但 `L30-L304` 不读取它；是否取消由 Effect 宿主决定。取消可能发生在 stat/read/ask/write/format/LSP yield 处，已完成前序文件不回滚。
- 无显式 timeout、retry 或补偿事务。验证失败通常在任何文件写入前结束；写入中失败可留下部分 patch，尤其 move 是先写目标后删源。
- 外部副作用是 `ctx.ask`（`edit`、`external_directory`）、FS 写入/删除、格式化/BOM 同步、文件事件和 LSP 触碰/诊断（`L72-L75,L142-L143,L205-L215,L223-L271`）。本文件不保证不同并发 tool 调用之间串行。

## 源内测试与行为判据

源文件自身未包含测试；对应测试为 `packages/opencode/test/tool/apply_patch.test.ts`。判据：空参数/非法格式/空 patch（`L89-L109`）；单 patch add/update/delete、一次权限请求、A/D/M 摘要（`L130-L174`）；move 元数据和跨目录移动（`L178-L205,L281-L297`）；多 hunk、BOM diff、尾换行、覆盖目标（`L207-L245,L263-L332`）；缺失 update/delete、目录删除、非法 header、验证失败无创建副作用（`L335-L402`）；EOF、上下文消歧、heredoc、空白及 Unicode 匹配（`L404-L549`）。参数 Schema 判据见 `test/tool/parameters.test.ts:L94-L106`。独立验证应检查审批拒绝无写入、验证失败保持所有原文件、move 源消失且目标正确、metadata 可 JSON 编码。

## zenpi Rust 映射

- **工具落点**：建议新增 `src/tools/apply_patch.rs` 并由 `src/tools.rs` 注册。现有 `ToolDefinition`、`ToolCall`、`ToolResult`、错误码、`ToolPreview::Diff` 在 `src/tools.rs:L502-L572,L642-L718`；`with_all_builtins` 当前注册 write/edit/run command（`L1075-L1091`）。新增 `ApplyPatchTool`，schema 为 `{ patchText: string }`，side effect 为 `WorkspaceWrite`，结果为 JSON 摘要和每文件统计。
- **路径和解析**：实现 Begin/End、Add/Delete/Update/Move、chunk 上下文、EOF、空白/Unicode 兼容和 BOM；先解析全部 hunk、读取并验证所有源文件，再 dispatch。每个源/目标路径走 `ToolContext::resolve_existing/resolve_for_write` canonical 边界（`src/tools.rs:L745-L772,L814-L867`），比 TypeScript 的 `path.resolve+external_directory` 更严格；工作区外路径映射显式审批。
- **审批/预览**：`core.rs:L4662-L4774` 的 `prepare_tool` 做 gate/policy/preview 和 `ApprovalRequest`，`L4775-L4859` 等待并持久化决定。当前 `ToolPreview::Diff` 是单 path（`src/tools.rs:L574-L640`），多文件 patch 应增加受界限的 `MultiDiff` 或聚合 diff，并为每个源文件保存 SHA-256；执行前重新验证，复用 `execute_approved` 的 stale-preview 拒绝（`src/tools.rs:L1243-L1280`）。
- **并发批处理**：`core.rs:L4424-L4455` 只有 Parallel 只读工具并行；`tool_runtime.rs:L157-L168,L203-L263` 要求 source-order prepare、边界和取消检查。apply patch 应是 Sequential 的单一 workspace write，hunk 保持源顺序。
- **journal/恢复**：`core.rs:L4486-L4604` 为 tool call 写 `InterruptedOperation` 并持久化结果；`session.rs:L49-L95` 区分 `Succeeded/Failed/Cancelled/Interrupted/UnknownOutcome`，缺失终态不等于成功。首次写入前应追加 `tool_execution_started`，记录已写数量/路径；中断标 `UnknownOutcome`，要求显式 Retry/Abandon，不能自动重放 move/delete。
- **取消/运行时**：`runtime.rs:L49-L99` 的 `CancellationToken` 是 cooperative，`L318-L350` 提供 cancel/shutdown，非合作任务可在 grace 后 detach（`L383-L420`）。Rust handler 应在解析后、每个 hunk 写前、格式化前检查 token；取消不是 rollback 证据。源 TypeScript 不检查 `ctx.abort`，这是明确差异。
- **协议/headless**：`protocol.rs:L219-L263` 已有 `Cancel`、`ToolOutput`、`Approve`、`Shutdown`；apply patch 应作为 provider 的 `ToolCall` 进入工具循环，而不是新建文件写命令。headless 审批事件及有界 mailbox/replay 在 `headless.rs:L4177-L4248` 等处处理；preview、approval_request、ToolResult 需带既有 request/turn ID。
- **写入与事件差异**：zenpi `WriteFileTool` 已有 workspace policy、diff preview、路径 gate、原子替换（`src/tools.rs:L2058-L2105,L2839-L2898`），但没有多文件事务、move/delete、BOM 保持、`Format.Service`、LSP touch/diagnostics。可复用 `atomic_write_text` 并新增 BOM helper；多文件仍按顺序提交和记录 partial state，格式化/LSP 只能接入可选 hook或明确空实现。
- **providers/**：`src/providers/mod.rs:L13-L20,L146-L159` 只负责协议与 provider 路由；`src/providers/connection.rs:L296-L327` 计算 `tools` 等 capabilities。`anthropic.rs`、`codex.rs`、`deepseek.rs`、`google.rs`、`openai.rs` 不应承担 patch 解析或文件副作用，只需在 capabilities.tools 可用时发送 schema。

## 未决问题

- 无法从本文件确认 `Format.Service` 是否原子、`LSP.diagnostics()` 是否阻塞、`EventV2Bridge.publish` 的失败策略；实现位于导入模块。
- `Patch.parsePatch` 对重复路径、绝对路径和空 `Move to` 的最终限制不在本文件内；Rust 映射需结合依赖实现和测试确认。

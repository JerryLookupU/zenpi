# OC-057 — packages/opencode/src/tool/truncate.ts

- `source_id`: `OC-057`
- `item_id`: `OC-057`
- `source_path`: `packages/opencode/src/tool/truncate.ts`
- `source_hash`: `d3dc9e7402de74a7652ceb9f7796c970efc4958597b51a94fa264697366ccea5`
- `source_bytes`: `6152`
- `source_lines`: `156`
- `coverage`: 已按源文件顺序完整读取；字节范围 `0-6151`（共 6152 字节），行范围 `L1-L156`。

## 完整行为复盘

1. **依赖与全局常量**：`L1-L10` 导入 Effect 运行时、`FSUtil`、权限 `evaluate`、`Config`、`ToolID` 与 `TRUNCATION_DIR`。`NodePath` 在本文件中没有被后续表达式使用。`RETENTION` 是 7 天；`MAX_LINES=2000`、`MAX_BYTES=50*1024`；`DIR` 是截断目录，`GLOB` 是该目录下的 `*` 路径（`L12-L17`）。
2. **数据类型与权限判定**：`Result` 是判别联合：未截断时只有 `{content, truncated:false}`，截断时还必须有 `outputPath`（`L19-L19`）。`Options` 的三个字段均可省略，方向只能是 `"head"` 或 `"tail"`（`L21-L25`）。私有函数 `hasTaskTool(agent)` 在 agent、permission 缺失时返回 `false`；否则调用 `evaluate("task", "*", agent.permission)`，只有 action 不是 `"deny"` 才返回 `true`（`L27-L30`）。
3. **服务契约**：`Interface` 暴露 `cleanup`、`write`、`output`、`limits` 四个 Effect 函数；`output` 的注释明确说超限时保存完整文本并返回预览和检查提示，`limits` 从 `tool_output` 配置解析上限（`L32-L44`）。`Service` 是 Context service，键为 `@opencode/Truncate`（`L46-L46`）。
4. **层初始化与文件系统服务**：`layer` 在 `L48-L52` 通过 `Effect.gen` 取得 `FSUtil.Service`，所有后续操作共享这个服务，但本文件没有额外的内存缓存或锁。
5. **`cleanup`**：`L53-L66` 计算 `Date.now()-7天` 的截止时间，读取 `TRUNCATION_DIR`；目录读取失败被吞掉并当作空列表。只保留名字以 `tool_` 开头的条目。逐项 `stat`，统计失败视作无信息；没有 mtime 或 mtime `>= cutoff` 的条目跳过，严格早于截止时间的条目调用 `fs.remove`。删除失败也被吞掉，因此清理是尽力而为，不向调用者报告失败。
6. **`write(text)`**：`L68-L73` 用 `path.join(TRUNCATION_DIR, ToolID.ascending())` 生成文件名，先 `ensureDir`，再 `writeFileString` 写入原始完整文本并返回路径。两个 I/O 均使用 `Effect.orDie`，失败会升级为 defect（中止该 Effect），而不是返回可恢复的业务错误；ID 由 `ToolID` 产生，调用者不提供路径。
7. **`limits()`**：`L75-L83` 尝试取得可选的 `Config.Service`。没有配置服务时返回默认常量；有服务但 `get()` 失败时也回退默认值。读取成功后逐字段使用 nullish fallback：`cfg?.tool_output?.max_lines` 和 `max_bytes` 缺失才分别回退 `2000` 与 `51200`。代码没有验证零、负数或其他异常数值，运行时传入的数值会直接参与截断。
8. **`output(text, options={}, agent?)`**：主体在 `L85-L141`。先解析 limits，再以 `??` 解析 `maxLines`、`maxBytes`，方向默认 `"head"`；`text.split("\\n")` 产生行数组，`Buffer.byteLength(text,"utf-8")` 计算完整 UTF-8 字节数（`L86-L92`）。只有行数不超过上限且总字节数不超过上限时原样返回未截断结果（`L93-L95`）。
   - 头部方向 `L102-L111` 从第 0 行向后遍历，最多取 `maxLines` 行；每行大小为 UTF-8 字节数，并为非首行加 1 个 `\\n` 字节。下一行会使累计值超过 `maxBytes` 时停止并置 `hitBytes=true`，所以单行大于字节上限时预览可以为空。
   - 尾部方向 `L113-L121` 从最后一行向前扫描，最多取 `maxLines` 行，用 `unshift` 保持原顺序；分隔符同样按 1 字节计入。任何下一行超限都会停止并标记按字节截断。
   - `L124-L126` 在字节截断时以 `totalBytes-bytes` 作为省略量、单位为 `bytes`；否则以 `lines.length-out.length` 作为省略量、单位为 `lines`。预览是 `out.join("\\n")`。尾随换行会产生额外空行，CRLF 中的 `\\r` 属于行内容并计入字节。
   - `L127-L140` 无论方向都先把完整 `text` 写入文件。若 agent 具有 task 权限，提示要求用 Task 工具委派 explore agent，并用 Grep/Read 的 offset/limit 查看；否则提示直接用 Grep 或带 offset/limit 的 Read。头部结果格式是“预览 + 截断标记 + hint”，尾部结果格式是“截断标记 + hint + 预览”；均返回 `truncated:true` 与 `outputPath`。写文件失败会在返回前以 defect 结束。
9. **启动后台清理与导出**：`L143-L148` 在层建立时先安排 `cleanup()`，随后按每小时间隔重复；整体首延迟 1 分钟，并用 `Effect.forkScoped` 放入当前 Scope。清理失败通过 `Effect.catchCause` 记录 `truncation cleanup failed` 和 `Cause.pretty(cause)`，不会杀死周期任务。`L150-L152` 返回四个方法的 service 实现；`L154` 用 `LayerNode.make` 暴露节点并声明 `FSUtil.node` 依赖；`L156` 将自身命名空间导出为 `Truncate`。

## 状态、取消、恢复与副作用

状态只存在于截断目录中的完整文本文件、文件 mtime，以及 Effect 层持有的周期 fiber；服务本身没有结果缓存、操作日志或持久化索引。`write` 的副作用是创建目录并写文件，`cleanup` 的副作用是删除超过 7 天且名字以 `tool_` 开头的条目；配置读取和权限评估是外部服务调用。`ToolID.ascending()` 使并发输出通常获得不同文件名，但实现没有显式互斥，清理 fiber 可与写入并发。

源代码没有独立的取消 token、超时、重试或恢复协议。取消由 Effect 运行时决定；`forkScoped` 使 Scope 关闭时可中断周期清理。`Effect.orDie` 把写入失败变成 defect，调用方没有业务级重试点。清理每小时重复属于运行期重试，但错误只记录并继续；进程重启后必须重新构造 layer 才会再次安排清理。不存在截断文件的恢复元数据，也没有对半写文件的校验或回滚。

## 源内测试与行为判据

源文件及同目录检索未包含测试（源内未包含测试）。可独立验证的判据：

- 输入同时满足 `lines.length <= maxLines` 且 UTF-8 字节数 `<= maxBytes` 时，结果必须原样、`truncated:false`，且不应创建文件（对应 `L90-L95`）。
- 构造多字节文本（如中文或 emoji）验证限制按 `Buffer.byteLength` 而非 JavaScript 字符数计算；分别验证 `head`、`tail`、单行超限、尾随换行和 `maxLines=0`（对应 `L97-L126`）。
- 超限时检查 `outputPath` 文件内容等于原始 `text`，头尾两种方向的标记顺序、`removed` 数值与 `bytes/lines` 单位符合 `L127-L140`；带允许 `task` 的 agent 时 hint 必须改变。
- 注入无 `Config.Service`、配置读取失败、只设置一个 `tool_output` 字段的场景，验证逐字段默认值（`L75-L83`）。创建旧 mtime、近期 mtime、非 `tool_` 名称和不存在目录，验证清理的过滤、跳过和吞错规则（`L53-L65`）。

## zenpi Rust 映射

zenpi 已有最接近的落点是 `src/tool_output.rs`（由 `src/lib.rs` 导出）：`OutputLimits`、`SessionOutputStore`、`CommandOutputCapture`、`OutputProgress` 和 `OutputError` 已提供受限文件存储、TTL、取消读取与恢复清理；`src/core.rs:L993-L1148` 的 `tool_output_control`、`read_tool_output`、`cleanup_tool_output` 负责协议入口和带 journal receipt 的清理。建议把本文件的“预览算法”单独落在 `src/tool_output.rs` 或新建 `src/tool_truncate.rs`：定义 `TruncateOptions { max_lines: Option<usize>, max_bytes: Option<usize>, direction: HeadTail }`、`TruncateResult { content, truncated, output_path }`、`resolve_limits(config)`、`truncate_preview(text, options)` 与 `write_full_output(...)`，并用 Rust `text.as_bytes().len()`、`char_indices`/安全 UTF-8 边界实现可验证的字节裁剪。

- `src/core.rs`：在 `run_command` 捕获流程（现有 `begin_output_capture`/`persist_output_capture`）之后生成 bounded preview；保留现有 `OutputError`、磁盘治理和 `tool_output_captured` 事件，不直接复刻 TypeScript 的裸路径字符串。
- `src/tool_runtime.rs`：执行器已有批量边界、策略和“不中途重试”语义；可让截断作为结果格式化阶段，不能让 preview 截断改变工具执行或 side effect 判定。
- `src/runtime.rs`：`CancellationToken` 是取消映射点。预览和写入前后检查 `is_cancelled()`；取消不得伪装成成功的 `TruncateResult`，并遵循现有 worker 的 cooperative cancellation 与 detach 语义。
- `src/headless.rs` 与 `src/protocol.rs`：已有 `ToolOutput(OutputRequest)`、`read_tool_output` 控制命令、JSONL 关联响应和事件字节上限（`src/headless.rs` 的异步 mailbox 上限）。建议新增显式 `preview` 字段或复用 `OutputSnapshot`，不要把绝对内部路径直接暴露到 provider/协议层；若暴露 `outputPath`，应改为现有 artifact ID/受信引用。
- `src/session.rs`：现有 journal 大小和恢复边界可承载 `tool_output_captured`、cleanup intent/receipt；应把截断文件生命周期与 session 绑定，并复用已存在的 pending cleanup 恢复，而不是依赖每小时 fiber 才能找回状态。
- `src/approval.rs`：该文件处理 side-effect approval；截断写盘属于宿主内部输出保存，不应绕过工具审批把它当作新的 workspace write。若策略要求审批，复用既有 `ToolSideEffect` 判定，不能从 provider 名称推断权限。
- `src/providers/**`：provider 模块（Anthropic/Codex/DeepSeek/Google/OpenAI、connection、registry）负责模型身份、协议和能力，不应实现截断算法。只需确保 provider 请求/响应的 bounded text 使用 `TruncateResult` 的预览，并保留 provider 原始调用 ID 以便 `ArtifactRef` 关联；provider 配置中没有 TypeScript `tool_output.max_lines/max_bytes` 的直接对应项，需在 zenpi config 中新增并做范围校验。

关键差异：TypeScript 默认是 2000 行/50 KiB、7 天全局目录清理、成功截断后返回可读提示；zenpi 当前 artifact 默认是每 session 私有目录、24 小时 TTL、64 KiB view/read 预算，并有私有 inode、哈希、完成标记、持久化 receipt、取消和恢复机制。Rust 映射应保留这些更强的安全与恢复约束，不能简单复制 `path.join(TRUNCATION_DIR, ToolID.ascending())` 的任意路径模型；同时补齐 `head`/`tail` 的精确字节与行判据。

## 未决问题

1. `TRUNCATION_DIR`、`ToolID.ascending()` 和 `Config.Service` 的具体实现及路径权限不在本文件中，无法确认目录是否跨进程共享、ID 是否必然以 `tool_` 开头。
2. Effect `Schedule.spaced` 在该版本中的首次 tick 时序、以及 `catchCause` 对 fiber interruption 的具体传播，需要结合运行时版本验证。
3. 配置允许零或负 `max_lines`/`max_bytes` 是否是有意行为，源文件没有约束或错误分支。

# OC-038 — packages/opencode/src/tool/external-directory.ts

- source_id/item_id: `OC-038`
- source_path: `packages/opencode/src/tool/external-directory.ts`
- source_hash: `b86477bd434aef57caf9fb1d3d72249be37c66e299aa1a864a601e1600e36bda`
- source_bytes: `1380`
- source_lines: `49`
- coverage: 已按顺序读取完整文件，字节 `0-1379`（共 1380 字节），行 `L1-L49`（含导入、注释、类型、导出函数和包装函数）。

## 完整行为复盘

1. `L1-L6` 导入 `path`、`Effect`、`InstanceState`、`Tool.Context` 类型、`containsPath` 和 `FSUtil`。本文件不访问文件系统，也不直接持久化；路径拼接和 Windows 规范化由 `path`/`FSUtil` 完成。
2. `type Kind = "file" | "directory"`（`L8`）限定检查目标的解释方式。`type Options`（`L10-L13`）只有可选 `bypass` 和 `kind`；调用者不传 options 时两者均无值。
3. `assertExternalDirectoryEffect`（`L15-L45`）由 `Effect.fn("Tool.assertExternalDirectory")` 包装，输入是 `ctx: Tool.Context`、可选 `target: string`、可选 `options: Options`，输出是成功时的 `Effect`，值为 `boolean`。
   - `L20`：`!target` 直接返回 `false`，因此 `undefined` 或空字符串不读取 `InstanceState`、不发请求。源码没有存在性、绝对路径、长度或字符校验。
   - `L22`：`options?.bypass` 为真时直接返回 `false`，跳过实例状态读取和权限询问。按 TypeScript 类型它应是布尔值；若运行时传入其他 truthy 值，JavaScript 条件仍会绕过。
   - `L24-L26`：通过 `yield* InstanceState.context` 取得实例上下文。Windows (`process.platform === "win32"`) 先以 `FSUtil.normalizePath(target)` 得到 `full`，其他平台保持原字符串。`containsPath(full, ins)` 为真时返回 `false`，表示目标在当前实例的 project directory/worktree 边界内，不需要 external permission。`containsPath` 的实现说明其会检查 `ctx.directory`，并在 `worktree !== "/"` 时检查 worktree（外部 helper 行 `L14-L23`）；本文件只依赖布尔结果。
   - `L28-L29`：`kind` 默认是 `"file"`。`"directory"` 时 `dir = full`；其他值（类型上不应出现）均走文件分支 `path.dirname(full)`，不会抛出 kind 错误。
   - `L30-L33`：构造单个目录通配模式。Windows 使用 `FSUtil.normalizePathPattern(path.join(dir, "*"))`；非 Windows 使用 `path.join(dir, "*").replaceAll("\\", "/")`，把反斜杠统一为正斜杠。通配符只覆盖目标目录的直接子项，不递归展开。
   - `L35-L43`：调用 `yield* ctx.ask`，请求体固定为 `permission: "external_directory"`，`patterns: [glob]` 与 `always: [glob]` 使用同一个单元素数组；`metadata.filepath` 是规范化后的 `full`，`metadata.parentDir` 是 `dir`。`ask` 成功后 `L44` 返回 `true`，该值表示已完成外部目录权限询问，不表示文件已存在或已被读写。
   - `ctx.ask` 失败、权限拒绝、`InstanceState.context` 缺失或路径工具抛错时，函数没有本地捕获，错误沿 `Effect` 失败通道传播；不会返回 `false`。
4. `assertExternalDirectory`（`L47-L49`）是异步适配器，接收同样的 `ctx/target/options`，调用 `Effect.runPromise(assertExternalDirectoryEffect(...))`。因此调用者得到 `Promise<boolean>`；Effect 的取消/失败会表现为 Promise rejection，成功值保持上述 `false` 或 `true`。
5. 并发语义：函数没有共享可变状态、锁、去重或合并请求；多个调用可同时等待各自的 `ctx.ask`。`InstanceState.context` 是当前 Effect 环境读取，不能从本文件推断跨调用缓存。源码没有重试、超时、文件检查或实际外部副作用，唯一外部动作是发出权限请求。

## 状态、取消、恢复与副作用

- 状态：只读取 `InstanceState.context`，不改变实例状态；局部变量 `full/dir/glob` 在一次调用内有效。
- 取消：函数没有读取 `ctx.abort`，也没有显式 `Effect.timeout`、`retry` 或 `onInterrupt`。Effect 运行时可以取消正在执行的 `ctx.ask`，但取消后的具体清理由 `ask` 实现负责；包装函数只把取消作为 Promise rejection 暴露。
- 超时/重试：本文件无超时和重试策略。权限系统若重试，属于 `ctx.ask` 的上层语义，不能归因于本文件。
- 持久化/恢复：不写 session、journal 或 permission store；`always` 只是请求字段，是否记忆授权由权限服务决定。进程恢复不会从本函数恢复任何中间状态。
- 外部副作用：`ctx.ask` 可能向 UI/主机发起 `external_directory` 请求；本函数不会创建、读取、删除目录或文件，也不会验证目标类型。`true` 只代表询问成功返回。

## 源内测试与行为判据

同目录测试 `test/tool/external-directory.test.ts` 覆盖了可独立核对的判据：

- `L42-L51`：空 target 不产生请求，结果等价于无操作。
- `L53-L62`：实例目录内路径不产生请求。
- `L64-L79`：实例外文件目标产生一个 `external_directory` 请求，`patterns` 和 `always` 都严格等于目标父目录加 `*` 的单元素数组。
- `L81-L96`：`{ kind: "directory" }` 使用目标目录本身加 `*`，而不是其父目录。
- `L98-L106`：`{ bypass: true }` 不产生请求。
- Windows 分支 `L108-L154`：不同盘符/斜杠写法归一为同一个 glob；盘根文件使用 `root/*`。测试辅助 `L27-L28` 与源码的 Windows/非 Windows glob 规则一致。
- 可独立验证的最小判据：给定假的 `ctx.ask` 记录请求，分别传 `undefined`、实例内路径、实例外文件、实例外目录和 `bypass`，检查返回值及请求字段；再在 Windows 下检查 `FSUtil.normalizePathPattern` 结果。测试没有覆盖 `ctx.ask` 失败、缺少 `InstanceRef`、非法运行时 kind 或并发取消，这些仍是实现边界。

## zenpi Rust 映射

- `src/tools.rs` 的 `ToolContext::new`（`L747-L773`）负责 canonical workspace root，`resolve_existing`（`L979-L1003`）负责 workspace 内路径。建议在 `src/tools.rs` 增加纯函数 `external_directory_request(target: Option<&str>, kind: ExternalDirectoryKind, bypass: bool, workspace: &Path, worktree: Option<&Path>) -> Result<Option<ExternalDirectoryRequest>, ToolError>`，或单列 `src/external_directory.rs`；它应只做 lexical path normalization/边界判断和 glob 构造，不调用 `resolve_existing`，因为源实现允许不存在的外部目标且允许绝对路径。
- `src/core.rs` 的 `prepare_tool` 审批前置流程（约 `L4674-L4978`，请求构造和 `request_response` 在约 `L4800-L4850`）是最接近 `ctx.ask` 的落点。把目标参数解析成 `ExternalDirectoryRequest { permission, patterns, always, filepath, parent_dir }`，在真正执行工具前提交 `src/approval.rs` 的 `ApprovalCoordinator::request_response`；拒绝映射为 `ToolErrorCode::PolicyDenied`，取消映射为 `Cancelled`，成功后才继续现有 preview/execute。
- `src/approval.rs` 的 `ApprovalRequest` 字段与校验（`L40-L104`）可承载 `tool`, `arguments`, `origin`, `policy_digest`, `lease_id`；建议增加明确的 permission/pattern metadata，而不是把路径 glob 拼进 tool 名称。`ApprovalCoordinator` 的 pending/visible/decision 状态（`L130-L370`）可复用等待、去重和取消唤醒语义；源文件本身不要求记忆授权，是否写入 policy 应保持由 Rust 审批策略决定。
- `src/tool_runtime.rs` 的批执行器（`L160-L270`、`L280-L455`）规定顺序/并发、取消检查和线程 join。权限请求应在 `prepare` 阶段完成；不要在 `dispatch` 中新增隐式重试。源函数无共享锁，Rust 可保持每个 call 独立；若批中出现顺序工具，沿用现有全批 barrier。
- `src/runtime.rs` 的输入边界批处理（工具 batch 约 `L833-L900`）只负责工具批生命周期；映射时让 approval wait 期间的输入泵/取消信号中断等待，不能把 external-directory 请求当成 provider 并发能力。`src/session.rs` 的 `append_event`（`L1205-L1221`）和操作恢复判定（`L1492-L1510`、`L2138-L2153`）可审计 approval 事件，但这是宿主增强；源实现没有持久化，不能声称源会自动恢复权限请求。
- `src/headless.rs` 的 `resolve_workspace_path_at`（`L1159-L1210`）故意拒绝绝对路径、父遍历和 symlink。它适合 workspace 内 host inspection，不应直接复用来实现本功能，否则会错误拒绝源函数应询问的外部绝对路径；建议另写不打开文件的外部路径分类器，并在 Windows `cfg(windows)` 下实现与 `FSUtil.normalizePath`/`normalizePathPattern` 等价的盘符和分隔符归一化。
- `src/protocol.rs` 当前工具/输出协议校验（工具输出读取约 `L1252-L1309`）没有 `external_directory` 专用请求类型。若 headless/TUI 需要跨进程呈现，应增加带 `permission`, `patterns`, `always`, `filepath`, `parent_dir` 的受限消息，并复用现有控制字符、长度和 JSON object 校验；若仅进程内审批，则无需改协议。
- `src/providers/**` 的 `OptionPolicy.tool_choice` 与 capability 交集（`src/providers/mod.rs:L121-L144`、`src/providers/connection.rs:L299-L325`）只决定 provider 是否能接收工具调用。`assertExternalDirectory` 是 provider-independent 的宿主安全门，应在 provider dispatch 之前执行；无需修改 provider capability。验证时可用任一支持 tools 的 provider 发起外部路径工具调用，确认请求先于 provider side effect 出现。

差异清单与可执行验证：Rust 需要显式 `Option<&str>`/enum，JavaScript 的 `undefined`、truthy bypass 和运行时错误 kind 行为需测试固定；Rust 的 `canonicalize` 对不存在路径会失败，而源函数不要求目标存在；Rust 现有 workspace resolver 拒绝绝对路径，必须与本功能分开；源的 `always`/`patterns` 是权限规则字段，Rust 应保留单一 glob、父目录 metadata 和 Windows 归一化。建议添加纯单元测试覆盖空目标、bypass、workspace/worktree 内外、file/directory glob、Windows 盘根，并添加审批集成测试覆盖拒绝、取消、成功后才 dispatch；用 `cargo test` 验证，不改 provider 行为。

## 未决问题

1. 本文件无法确认 `ctx.ask` 对 `always` 的具体持久化、重复请求合并、UI 展示和拒绝错误类型；这些属于 `Tool.Context`/permission 实现。
2. 本文件无法确认 `FSUtil.normalizePath` 是否解析 `~`、环境变量、symlink 或仅做分隔符/盘符规范化；只能确认它在 Windows 分支被调用。
3. `containsPath` 的 project/worktree 边界来自外部 helper；若实例上下文不存在，`InstanceState.context` 的 defect 行为由 Effect 环境决定，不在本文件内定义。

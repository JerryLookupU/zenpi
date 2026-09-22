# OC-031 — packages/opencode/src/session/summary.ts

- source_id/item_id：`OC-031`
- source_path：`packages/opencode/src/session/summary.ts`
- source_hash：`116bcdfe4467a384920582688fe37739318817e2867000c733519b1783652fe8`
- source_bytes：`5196`
- source_lines：`160`
- coverage：已读取源文件全部字节 `0-5195`、全部行 `L1-L160`（包含导入、注释、类型、实现与导出）。

## 完整行为复盘

文件先在 `L1-L8` 引入 `LayerNode`、Effect 运行时、`SessionV1` 消息类型、`EventV2Bridge`、`Snapshot`、`Session`、`SessionID`/`MessageID` 和 `Config`。因此本模块是一个由 Effect Layer 组装的会话文件变更摘要服务：消息由 `Session` 提供，文件快照差异由 `Snapshot` 提供，结果事件由 `EventV2Bridge` 发布，是否启用快照由 `Config` 决定。

- `unquoteGitPath(input: string)`（`L10-L64`）是本文件唯一的路径解码函数。输入不同时以双引号开头和结尾时原样返回（`L11-L12`），所以普通路径、单边引号或空字符串都不会被改写。对包在双引号中的 Git quoted path，它去掉首尾引号后逐字符扫描（`L13-L16`）：普通字符按 `charCodeAt(0)` 加入 `bytes`（`L17-L20`）；末尾反斜杠没有后继字符时保留反斜杠字节（`L23-L27`）。后继字符是 `0`-`7` 时，最多取三个八进制数字、以 8 为基数解析并跳过已消费字符（`L29-L39`）；代码中的 `!match` 是防御分支，发生时保留后继字符并额外前进一位（`L31-L35`）。其余转义只识别 `n/r/t/b/f/v`、反斜杠和双引号，未知转义按后继字符本身处理（`L42-L60`）。最后用 `Buffer.from(bytes).toString()` 转成 UTF-8 字符串（`L63-L64`）；因此非法 UTF-8 的替换行为由 Node `Buffer` 决定，函数不抛出显式异常。注意它按 JavaScript 字符码组装字节，非 ASCII 字符在此解码路径中的精确性取决于输入形态。

- `Interface`（`L66-L70`）导出三项 Effect API：`summarize({sessionID,messageID})` 返回 `Effect<void>`；`diff({sessionID,messageID?})` 返回 `Effect<Snapshot.FileDiff[]>`；`computeDiff({messages})` 同样返回文件差异数组。接口没有显式错误类型，具体 Effect 失败会沿调用链传播。

- `Service`（`L72`）是 `Context.Service<Service, Interface>()("@opencode/SessionSummary")`，为依赖注入提供稳定 service key；它本身不保存状态。

- `layer`（`L74-L146`）是私有 `Layer.effect`。构造时依次取得 `Session.Service`、`Snapshot.Service`、`EventV2Bridge.Service`、`Config.Service`（`L76-L81`），并在同一个 Effect 作用域内创建三个带追踪名字的函数（`L82`、`L102`、`L129`）。这些函数在一次调用中按源码顺序执行，没有并发合并或锁语义；Effect 的取消会在各个 `yield*` 边界生效。

- `computeDiff`（`L82-L100`）输入有序的 `SessionV1.WithParts[]`。`from` 初始未定义；遍历每条消息时，只要尚未找到起点，就在 parts 中寻找第一个带 `snapshot` 的 `step-start`，找到后停止继续寻找起点（`L83-L93`）。同一遍历会把每一个带 `snapshot` 的 `step-finish` 写入 `to`，所以最终 `to` 是输入序列中最后一个完成快照（`L94-L97`）。只有 `from` 与 `to` 都存在才调用 `snapshot.diffFull(from,to)` 并把其结果作为 Effect 结果（`L98`）；缺起点、缺终点或消息为空均返回新建空数组（`L99`）。`diffFull` 的失败不被捕获，会使该调用失败；没有显式重试、超时或去重。

- `summarize`（`L102-L127`）要求 `sessionID` 与 `messageID`。第一步无条件把会话 summary 写成 `additions:0`、`deletions:0`、`files:0`（`L106-L113`），随后发布 `Session.Event.Diff`，载荷是同一 `sessionID` 与空 `diff`（`L114`），这两个副作用先于任何快照开关和消息校验。读取配置后若 `snapshot === false` 立即返回（`L115`），因此关闭快照时保留清零结果和空事件。否则读取该会话全部消息，并用 `Effect.orDie` 将消息读取失败转为不可恢复缺陷（`L116`）；空会话直接返回（`L117`）。消息筛选保留目标 `messageID` 本身，或 `role === "assistant"` 且 `parentID === messageID` 的助手消息（`L119-L121`）。随后只接受精确目标存在且角色为 `user` 的情况（`L122-L123`）；目标不存在、目标是 assistant 或其他角色时不写入差异。对筛选后的用户/助手集合调用 `computeDiff`（`L124`），把目标原有 summary 展开并仅覆盖 `diffs` 字段（`L125`），再通过 `sessions.updateMessage(target.info)` 持久化（`L126`）。若计算或更新失败，之前的清零和空 Diff 事件可能已经可见，源码没有事务回滚。

- `diff`（`L129-L142`）是读取接口。缺少可选 `messageID` 时立刻返回空数组（`L130`）。否则读取会话消息并用 `.find` 精确匹配 ID（`L131-L133`）；找不到或角色不是 `user` 时返回空数组（`L134`）。从 `message.info.summary?.diffs` 取数据，缺省为 `[]`（`L135`）。逐项处理时，没有 `file` 的差异原样返回（`L136-L138`）；有文件名时调用 `unquoteGitPath`，若解码前后相同则复用原项（`L138-L139`），改变时浅拷贝差异对象，仅替换 `file`（`L140-L141`）。该函数不重新计算磁盘状态，也不写事件或持久化。

- `Service.of({ summarize, diff, computeDiff })`（`L144`）暴露实现；`Layer` 在 `L145-L146` 闭合。

- `DiffInput`（`L148-L152`）是 Schema 对象：`sessionID` 必填，`messageID` 用 `Schema.optional`，并导出相应 `Schema.Type` 类型别名。它只描述输入验证，不改变 service 函数的 Effect 错误语义。

- `node`（`L154-L158`）调用 `LayerNode.make` 注册 `Service`、实现 layer 和四项依赖节点 `Session.node`、`Snapshot.node`、`EventV2Bridge.node`、`Config.node`。最后 `export * as SessionSummary from "./summary"`（`L160`）提供命名空间再导出，便于其他 session 模块引用。

## 状态、取消、恢复与副作用

本文件没有模块级可变状态；`from`、`to`、筛选消息和差异数组都是单次调用局部值。源码没有显式超时、重试、指数退避或恢复分支。Effect 运行时可在 `yield* sessions.messages`、`snapshot.diffFull`、事件发布和消息更新之间取消，但取消点的精确行为由 Effect/各服务实现决定。`summarize` 的副作用顺序是“清零 summary → 发布空 Diff → 读取配置/消息 → 计算 → 更新目标消息”（`L106-L126`）；任何后续失败都不会自动撤销前两步。`sessions.setSummary` 与 `sessions.updateMessage` 是持久化写入，`events.publish` 是外部事件副作用；`diff` 与 `computeDiff` 本身是只读（后者可能访问快照存储）。没有并发协调：两个 `summarize` 调用可能交错清零、发布空事件和写回同一消息，最后完成的 `updateMessage` 可能覆盖先前结果。`config.snapshot === false` 是唯一明确的功能开关（`L115`），不是错误或恢复状态。

## 源内测试与行为判据

源文件及同目录检索未包含针对 `summary.ts` 的测试，源内未包含测试。可独立验证的判据如下：

1. 给 `computeDiff` 一组 parts，首个 `step-start.snapshot` 应成为 `from`，最后一个 `step-finish.snapshot` 应成为 `to`；缺任一端必须得到长度为零的新数组（对应 `L82-L100`）。
2. 用 `summarize` 调用关闭快照配置，必须观察到 `setSummary` 的三项零值和一次空 `Session.Event.Diff`，且不应读取/更新目标消息（`L106-L117`）。
3. 目标不是 user、ID 不存在或没有 `messageID` 时，`diff` 必须返回 `[]`；带 Git 八进制转义的引号路径在 `diff` 返回值中应变成解码后的 `file`（`L129-L142`）。
4. 让 `diffFull` 或 `updateMessage` 失败，验证清零/空事件可能已经发生，说明不存在事务回滚（`L106-L126`）。

## zenpi Rust 映射

zenpi 的 `src/session.rs` 已有会话持久化与 `SessionSummary`（`src/session.rs:L280-L297`），但它是路径、会话 ID、turn/handoff/event 计数和恢复警告的会话级目录摘要；`summary.ts` 的 `additions/deletions/files/diffs` 属于单个 user message，不能直接复用同名结构。建议新增 `src/session_summary.rs`（或将下列类型放入 `src/session.rs` 的独立区域）：`FileDiff { file: Option<String>, additions: u64, deletions: u64, ... }`、`MessageSummary { additions, deletions, files, diffs: Vec<FileDiff> }`、`SessionSummaryService`，并以 `SessionStore` 的单写者 API 持久化 message metadata/event。

- 对照 `src/session.rs`：将 `summarize` 的清零和最终写回实现为一个可验证的 `update_message_summary(session_id, message_id, summary)`；写入应沿 `append_json`/事件日志路径，保证“先持久化后更新内存投影”的习惯（现有 `append_turn` 的行为可作判据，`src/session.rs:L861-L891`）。可将快照 ID 与 `Turn.metadata` 或专门 `snapshot_step` 事件关联；当前 `SessionStore` 没有 `messages()`、`updateMessage()` 或 `Snapshot.Service` 等价物，需要新增索引或由 `Agent` 提供投影。
- 对照 `src/core.rs`：`Agent::snapshot()` 只产出会话/运行状态（`src/core.rs:L1337-L1346`），可在 Agent 的 turn 完成路径调用新 service；已有语义压缩/`append_semantic_checkpoint_for_operation`（`src/session.rs:L1231-L1290`、`src/context.rs`）是上下文恢复，不等价于文件 diff，应保持两套数据模型。
- 对照 `src/headless.rs`：已有 cancellable worker、请求重放和资源/文件差异命令（例如 `src/headless.rs` 中 `/diff` 的 `diff_value_at` 路由），可把 `SessionSummaryService::diff` 暴露为一个明确的 headless command；必须复用 headless 的 `CancellationToken` 检查和有界响应策略，不要把文件内容或差异缓存无限放入 replay WAL。
- 对照 `src/tool_runtime.rs` 与 `src/approval.rs`：这些模块负责工具执行、审批、取消和副作用审计。快照采集若调用工具或工作区写操作，应先走 `ApprovalCoordinator`/`SideEffectPolicy`；纯读取 diff 可以作为只读工具，不应从 provider 名称或响应推断批准。工具取消只代表“不再启动/停止等待”，不能假设外部文件变更已回滚。
- 对照 `src/runtime.rs`：用 `CancellationToken::is_cancelled()` 在遍历消息、快照读取和每次 diff 计算间隙轮询；成功结果发布前调用 `mark_completed()`，避免晚到取消把已完成摘要改成 cancelled。不要创建脱离现有 `BackgroundRunner` 的线程。
- 对照 `src/protocol.rs`：新增命令输入可采用 `session_id`、可选 `message_id` 的 serde 结构，沿现有 `Command` 校验 ID 长度、控制字符和未知字段；`message_id` 缺失时返回空 diff，保持源行为，而不是协议错误。
- 对照 `src/providers/**`：provider registry/connection 只描述模型、协议、路由和认证（`src/providers/mod.rs:L13-L50`、`src/providers/connection.rs:L18-L60`），没有工作区快照能力。`Snapshot` 应由本地 workspace/session 层实现，provider 仅产生带 `step-start`/`step-finish` 元数据的 turn 事件；禁止把文件 diff 计算下沉到 Anthropic、Google、OpenAI 等 provider 适配器。

可执行验证：新增 Rust 单元测试覆盖“首个 start + 最后 finish”“缺端返回空”“user/assistant parent 过滤”“Git quoted path 八进制解码”“取消发生在 diff 前后”“写入失败后事件顺序”；再以 headless JSONL 命令读取同一 `message_id`，比较持久化摘要与读取接口结果。差异清单是：zenpi 当前无 `Snapshot.Service`/`Snapshot.FileDiff`、无按 message 的 summary diffs、无 `Session.Event.Diff` 同名事件、无 `Session.messages/updateMessage` 投影 API，也没有本文件的 Git path 解码器。

## 未决问题

1. `Snapshot.diffFull` 的具体快照存储、差异排序、`FileDiff` 字段和错误类型不在本文件中，无法确认空文件、删除文件或重命名的精确表示。
2. `sessions.setSummary`、`sessions.updateMessage` 是否原子写入、是否有并发版本检查，源文件未说明；因此并发 `summarize` 的最后写入胜出只是基于调用顺序的风险判断。
3. `Effect.orDie` 产生的缺陷如何被上层记录、是否自动重试，需查看 `Session.Service` 与运行时宿主实现。

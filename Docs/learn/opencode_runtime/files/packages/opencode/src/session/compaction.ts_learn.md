# OC-011 — packages/opencode/src/session/compaction.ts

## 元信息

- source_id/item_id: `OC-011`
- source_path: `packages/opencode/src/session/compaction.ts`
- source_hash: `8d478570a7e4ad32b746030d4f86a1c673949b1e2259bd716b3885d99283289a`
- source_bytes: `21236`
- source_lines: `608`
- coverage: 已顺序读取完整文件，字节范围 `0-21235`，行范围 `L1-L608`（含导入、注释、类型、常量、导出与文件末尾 re-export）。

## 完整行为复盘

文件以 Effect service 组织会话压缩。导入的 `SessionV1` 是消息/part 的持久模型，`MessageV2` 负责模型消息转换，`Token` 做近似 token 估算，`SessionProcessor` 执行摘要模型请求，`Provider`/`Agent` 选择模型，`Plugin` 提供扩展钩子，`Session` 写回消息，`EventV2Bridge` 发布事件（`L1-L24`）。

- 导出 `Event = SessionCompactionEvent`（`L26`）暴露 `Compacted` 事件定义。`PRUNE_MINIMUM=20_000`、`PRUNE_PROTECT=40_000` 是工具输出清理的 token 阈值；单个工具输出最多序列化 `2_000` 字符；`skill` 工具受保护；自动保留尾部 token 默认限制在 `2_000..15_000`（`L28-L33`）。`Turn`、`Tail`、`CompletedCompaction` 是内部索引结构：turn 以消息区间 `[start,end)` 表示，tail 记录起点消息 ID，completed 记录 compaction user/assistant 的数组下标与摘要（`L34-L49`）。
- `truncate(value)`：长度不超过 `2_000` 原样返回，否则保留前缀并追加换行和 `[truncated]`（`L51-L52`）；按 JavaScript 字符串长度工作。
- `serialize(message)`：user 消息只取未 ignored 的 `text` part，过滤空串并以换行合并，文件 part 变成 `[Attached mime: filename]`（缺名使用 `file`），前面分别加 `[User]:`；assistant 消息保留 text、reasoning，tool 先输出调用名和 JSON input，completed tool 再输出附件及结果，已被压缩的结果显示 `[Old tool result content cleared]`，否则经 `truncate`，error 输出 `[Tool error]`，其他状态只保留调用行（`L54-L85`）。这决定摘要请求看到的文本边界和工具结果压缩形态。
- `summaryText(message)`：收集所有 text parts，trim 后以空行连接；空结果返回 `undefined`（`L87-L95`）。
- `completedCompactions(messages)`：先把含 `compaction` part 的 user 消息 ID 映射到数组下标，再寻找其 `parentID` 匹配、`summary=true`、有 `finish` 且无 `error` 的 assistant；返回 user/assistant 下标及摘要，失败或无对应 user 的摘要被忽略（`L97-L113`）。
- `preserveRecentBudget({cfg,model})`：优先使用 `cfg.compaction.preserve_recent_tokens`；否则计算 `floor(usable(input)*0.25)`，再夹在 `MIN_PRESERVE_RECENT_TOKENS` 与 `MAX_PRESERVE_RECENT_TOKENS`（`L115-L120`）。因此默认值依赖模型可用上下文，不直接等于模型窗口。
- `turns(messages)`：每个不含 compaction part 的 user 消息开启一个 turn，初始 end 为消息总数，随后用下一个 user 的起点截断；compaction marker user 不会开启新 turn（`L122-L138`）。
- `splitTurn(...)`：预算小于等于 0 或 turn 只有 user 一条时返回 `undefined`；从 turn 的第二条消息起逐一尝试，把尾部切片送入异步 `estimate`，第一个不超过预算的起点即返回 `Tail`，全失败返回 `undefined`（`L140-L163`）。估算是 Effect，调用按顺序进行。
- 导出 `Interface` 定义四个 Effect 方法：`isOverflow`（按 token/model 判定）、`prune(sessionID)`、`process(parentID,messages,sessionID,auto,overflow?)` 返回 `"continue"|"stop"`、`create(sessionID,agent,model,auto,overflow?)`（`L165-L185`）。导出 `Service` 是 `Context.Service`，`use` 是 `serviceUse(Service)`（`L187-L189`）。
- `layer`（`L191-L605`）注入 Config、Session、Agent、Plugin、SessionProcessor、Provider、EventV2Bridge、RuntimeFlags，构造服务实现：
  - `isOverflow`（`L203-L213`）读取配置和 `flags.outputTokenMax`，调用 `overflow`；配置中的 `compaction.auto=false`、模型输入/输出限制等由 `overflow` 决定，返回布尔值，不写会话。
  - `estimate`（`L215-L221`）将完整消息转换为 model messages，再对 JSON 字符串调用 `Token.estimate`；转换失败会使 Effect 失败。
  - `select`（`L223-L269`）选择摘要 head 与保留 tail。`tail_turns<=0` 直接不压缩；否则按 `turns` 构造列表，从最新 turn 向前惰性估算，直到累计不超过 `preserveRecentBudget`；超预算的当前 turn 尝试 `splitTurn`，无可行切分且此前没有 keep 时记录 `tail fallback`。没有 keep 或 keep 起点为 0 时返回全部消息且无 `tail_start_id`，否则返回 `messages.slice(0,keep.start)` 与起点 ID。重复 compaction 时调用方会先隐藏旧 compaction 对应的 user/assistant，因此旧摘要不会占用尾部预算（`L364-L370`）。
  - `prune`（`L271-L317`）仅在 `cfg.compaction.prune` 为真时执行；读取 session，`NotFoundError` 转为空操作。倒序扫描消息，遇到第二个 user 前不处理；遇到带 summary 的 assistant、已 compact 的工具结果或保护工具 `skill` 即停止。只收集 completed tool，按输出 token 累加，超过 `PRUNE_PROTECT` 后加入待清理列表；仅当可清理量严格大于 `PRUNE_MINIMUM`，才把 `part.state.time.compacted` 写成当前时间并逐 part `session.updatePart`。清理是持久化副作用，但没有事务回滚；单次失败会中断 Effect。
  - `processCompaction`（`L319-L557`）是主流程。先以 `parentID` 找到最后一个 user；不存在或角色错误立即抛出 `Compaction parent must be a user message`（`L326-L329`）。读取 parent 的 compaction part（`L331`）。若 `overflow=true`，向前找最近的普通 user 作为 `replay`，并把摘要输入截到它之前；若截断后没有任何普通 user，则撤销 replay 并使用原消息（`L340-L355`）。选择名为 `compaction` 的 agent，并优先使用该 agent 的模型，否则沿用 user 模型；provider 找不到通过 `Effect.orDie` 转为致命失败（`L358-L362`）。若当前 parent 就是 compaction marker 的末条消息，则从 history 排除 marker；用 `completedCompactions` 得到旧摘要，隐藏旧 user/assistant，下游 `select` 只对可见 history 选择 head/tail（`L363-L371`）。
  - 主流程随后触发 `experimental.session.compacting`，允许插件返回额外 `context` 或完全替换 `prompt`；对 selected head 做 `structuredClone`，再触发 `experimental.chat.messages.transform`（`L372-L380`）。将消息 `serialize` 后以双换行组成 conversation；没有插件 prompt 时使用 `buildPrompt({previousSummary,context:[conversation]})` 并拼接插件 context，空字符串过滤（`L381-L391`）。创建 summary assistant：新 `MessageID`、`summary=true`、`mode/agent="compaction"`、继承 user variant、路径来自 `InstanceState.context`，cost/token counters 全零、provider/model 来自所选模型、created 时间为当前时间；先 `session.updateMessage` 持久化（`L392-L419`）。
  - 创建 `SessionProcessor` 并以只有一个 text user prompt 的输入运行，prompt 为 `nextPrompt`；插件自定义 prompt 时额外附加“conversation history”和序列化 conversation（`L420-L449`）。processor 返回 `"compact"` 表示上下文仍超限：给 assistant 写 `ContextOverflowError`，`finish="error"`，更新消息并返回 `"stop"`（有 replay 与否产生两种不同错误文案，`L450-L459`）。若旧 compaction part 的 `tail_start_id` 变化，更新该 part（`L461-L466`）。
  - 当结果为 `"continue"` 且 `auto=true`，若有 replay，则新建 user message，复制原 user 的 agent/model/format/tools/system，并复制其 parts；跳过 compaction part，媒体 file 改为附加说明 text，其余 part 重新生成 `PartID` 后写入（`L468-L495`）。无 replay 时触发 `experimental.compaction.autocontinue`，默认 `{enabled:true}`；插件允许时创建新的 user message，并写 synthetic text part，overflow 情况先写媒体过大指导，再写继续或请求澄清的指导，带内部 metadata `{compaction_continue:true}`（`L497-L549`）。
  - 若 processor message 有 error 返回 `"stop"`；否则 result 为 `"continue"` 时发布 `Event.Compacted`（含 sessionID），最后返回 processor result（`L552-L557`）。发布只发生在成功继续路径，不为 stop 发布。
  - `create`（`L559-L582`）只创建一个 user message（新 ID、传入 agent/model/session、当前时间），再写一个 compaction part，保存 `auto` 和可选 `overflow`；不调用 provider。`Service.of` 返回四个方法（`L584-L589`）。
- 导出 `node = LayerNode.make(...)`（`L593-L605`）声明上述八个依赖；文件末尾 `export * as SessionCompaction from "./compaction"` 使整个模块可命名空间导入（`L608`）。

## 状态、取消、恢复与副作用

状态以 session 中的 user compaction part、assistant summary、tool part 的 `time.compacted` 以及 `tail_start_id` 表示；没有独立内存状态，重复压缩通过扫描历史恢复。`completedCompactions` 只承认已 finish 且无 error 的 summary，故半成品不会被当作 prior summary（`L97-L113`）。`processCompaction` 先写 summary assistant，再让 `SessionProcessor` 更新其 parts；中途失败可能留下未完成 assistant，后续扫描会因缺 `finish`/有 `error` 而忽略。成功后 tail 边界和 synthetic continue 会再次持久化。

Effect 的 fiber interruption 是主要取消语义：所有 `yield*`、provider/processor 调用和 plugin trigger 都可被中断；源文件没有显式 `CancellationToken`、超时或重试循环。重试/退避属于 `SessionProcessor`/provider 层；源内不对 `process` 做 retry，也不在取消时回滚已写入的消息或 part。`prune` 作为 Effect 可被调用方 fork，但自身按顺序写每个 part。并发上，单个 Effect 流程内选择、写消息、处理器和事件发布是串行的；`structuredClone` 只隔离插件变换输入，不能提供跨 fiber 事务锁。

外部副作用包括 provider 摘要请求、plugin hooks、session message/part 持久化、事件总线发布和时间戳写入。`overflow` replay 会把媒体 file 降级成文字提示，避免再次把大媒体送入 provider；自动继续的 metadata 明确标记为内部、不稳定插件契约（`L528-L540`）。源文件没有持久化操作 ID、幂等键或恢复日志；恢复依赖历史消息结构及 `tail_start_id`。

## 源内测试与行为判据

源文件本身未包含测试；同目录测试位于 `packages/opencode/test/session/compaction.test.ts`。可核对判据包括：`isOverflow` 的输入/缓存 token、模型 input cap、auto=false 边界（`L382-L565`）；`create` 必须产生 compaction user/part（`L566-L625`）；`prune` 只清理旧 completed tool 且跳过 `skill`（`L626-L813`）；`process` 的错误 parent、`Compacted` 事件、`compact` 结果 stop、auto synthetic continue、tail_start_id 与预算切分（`L814-L1125`）；overflow replay/媒体降级、重试退避中中断、处理器建立前中断不留 summary、禁止工具调用、只摘要 head、重复摘要锚定 prior summary、插件 context 顺序及重复 compaction 序列化（`L1125-L1670`）。独立验证时至少应断言：给定相同消息与配置，`select` 返回的 head/tail 起点稳定；`prune` 在清理量不超过 `20_000` token 时不写 part；processor 返回 `compact` 时 summary assistant 为 error 且结果为 `stop`；成功 auto 流程发布一次 `SessionCompaction.Event.Compacted` 并产生 synthetic continue 或 replay user。

## zenpi Rust 映射

zenpi 已有语义压缩骨架：`src/core.rs:2080-2420` 的 `prepare_semantic_provider_context` 在预算不足时选择安全 cut、调用 `prepare_semantic_compaction`、向 provider 请求摘要、校验 `MAX_SUMMARY_BYTES`、写 `semantic_summary_response`，最后 `append_semantic_checkpoint_for_operation` 并恢复 checkpoint；`compact_context_with_control` 在 `src/core.rs:2428` 起提供 idle 检查、取消回调和 `CompactionReport`。建议落点如下。

- `src/context.rs` 保持 token 预算、whole-turn cut、`SemanticCheckpoint` 与摘要验证；若要对应 `preserve_recent_tokens/tail_turns`，新增纯函数 `select_recent_tail(turns, TailPolicy)`，并用测试证明边界和重复 compaction 不把 prior summary 算入 tail。
- `src/session.rs:1231-1290` 是持久化落点；继续使用 append-only `semantic_checkpoint` 与 `OperationKind::Compaction`，可增加等价 `tail_start_id`/tool-pruned 标记，而不要改写旧记录。`latest_semantic_checkpoint`（`L1317-L1391`）已经提供分支与源摘要校验，可替代 TS 的 `completedCompactions` 恢复扫描。
- `src/core.rs` 应承载 `processCompaction` 的编排：新增 `compact_with_options(auto, overflow, cancelled)`，把摘要 provider、预算、operation marker、finish outcome 和 `CompactionReport` 统一串起；`compact_context_view`（`src/headless.rs` 的本地 compact 投影）继续作为 headless/TUI 入口。
- `src/protocol.rs:214-263` 的 `Command` 当前没有 `Compact` 变体；建议新增受限 `Compact { auto: bool, overflow: bool }` 或复用本地 slash 路由，并在 `into_command` 做字段校验。可验证条件是 headless 请求得到带 `durable=true` 的报告且错误不进入 provider prompt。
- `src/runtime.rs:49-99` 的 `CancellationToken` 与 job deadline 是 TS Effect interruption 的对应物；provider 适配器应在发送前、流式 delta 间、retry/backoff 前检查 token，并在 summary 成功写 checkpoint 前调用 `mark_completed`，避免晚到 cancel 改写成功结果。
- `src/tool_runtime.rs:157-176` 已规定取消时 join worker、不得 detach；摘要阶段应禁止 tool calls，沿用 `ToolBatchDecision::Reject`/只读策略。`src/approval.rs` 不需要为摘要模型产生用户审批；若未来允许插件工具，必须显式拒绝 compaction origin，而不是从模型响应推断授权。
- `src/providers/**`：`providers/registry.rs:60-109` 的 `ModelDescriptor::context_budget` 提供窗口/output clamp；摘要请求应使用同一 descriptor、`RequestPurpose::SemanticCompaction` 和 provider 适配器（`anthropic.rs`、`openai.rs` 等）完成流式响应、usage 与 deadline。不要照搬 TS 的 `Agent` 专用模型而绕过 registry 身份校验。
- 差异清单与可执行验证：① TS 有 `tail_turns`、按工具输出的 `prune`，zenpi 当前主要是 whole-turn semantic checkpoint；新增 `TailPolicy` 和 `prune_tool_output` 后测试 token 阈值与 `skill` 等价保护规则。② TS 有 plugin prompt/context/transform hooks，zenpi 没有同名插件 API；可在 `core.rs` 定义受限 `CompactionHook`，测试 prompt 注入不改变已序列化 source digest。③ TS 有 overflow replay 与 synthetic continue，zenpi `CompactionReport` 仅报告结果；新增协议字段和 replay turn 前必须做附件降级、幂等 journal 记录。④ TS 依赖消息/part 可更新投影，zenpi session 是 append-only；所有新状态应写事件并可由 `latest_semantic_checkpoint` 重放。⑤ TS 的 `process` 返回 `continue|stop`，zenpi 通过 `AgentError`/`CompactionReport` 表达；建立映射表并测试 provider context overflow、取消、transport failure 分别为 `InvalidTurn`、`Cancelled`、`Interrupted`。⑥ TS 没有操作恢复日志，zenpi 已有 `OperationKind::Compaction` 与 recovery veto（`core.rs:2089-2100`）；保留 zenpi 的 fail-closed 语义。

## 未决问题

无。源文件可确认的模型选择、尾部预算、工具输出清理、插件钩子、overflow replay、持久化顺序、事件发布和 Effect 中断路径均已覆盖；未从源文件确认的 provider 具体 retry/backoff 实现属于 `SessionProcessor`/provider 模块边界，已在映射中明确为外部依赖。

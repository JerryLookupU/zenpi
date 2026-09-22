# OC-026 — packages/opencode/src/session/revert.ts

- source_id/item_id: OC-026
- source_path: packages/opencode/src/session/revert.ts
- source_hash: c7e578bc0e8d01e551a06ce91a611c974e2e0e6ce59c5f6578922ba162536879
- source_bytes: 5659
- source_lines: 136
- coverage: 已顺序读取完整字节范围 0–5658（5659 字节）与行范围 L1-L136，包含全部 import、类型、实现、依赖层和导出。

## 完整行为复盘

1. **输入模式与公共契约（L1-L26）**
   - RevertInput 是 Schema.Struct：必填 sessionID: SessionID、必填 messageID: MessageID，partID: PartID 可选（L13-L18）。缺少会话或消息标识应在 schema 解码阶段失败；省略 partID 表示按消息边界回退。
   - Interface 暴露三个 Effect 函数：revert(input)、unrevert({sessionID})、cleanup(session)，前两者只声明 Session.BusyError，cleanup 返回 void（L20-L24）。Service 用 Context.Service 注册名 @opencode/SessionRevert（L26）。
   - layer 通过 Effect.gen 注入 Session.Service、Snapshot.Service、Storage.Service、EventV2Bridge.Service、SessionSummary.Service、SessionRunState.Service（L28-L37）；node 在 L130-L134 以同一依赖集合构造运行时节点，L136 将模块重新导出为 SessionRevert。

2. **revert（L38-L89）**
   - 先调用 state.assertNotBusy(input.sessionID)（L38-L40），活动运行中的会话被拒绝，避免和 prompt/工具并发修改。随后读取完整消息序列 all 和当前 session；两处 .pipe(Effect.orDie) 把底层读取失败提升为不可恢复缺陷，而不是返回声明中的 BusyError（L40-L43）。
   - 算法按 all 的既有顺序扫描，而不是按 ID 字符串排序。每遇到 user 消息就更新 lastUser（L46-L48）。在尚未命中回退点时，逐 part 维护当前消息的 remaining；命中条件是“消息 ID 等于 input.messageID 且未提供 partID”，或“part ID 等于 input.partID”（L49-L57）。
   - 命中 part 时，只有该消息此前已有 text 或 tool part，才保留 partID；否则把 partID 置为 undefined（L56-L61）。这使得消息开头的 step/patch 等非文本工具 part 回退到整条消息。命中后，当前 part 之后及所有后续消息的 patch part 都收集到 patches，前缀不会收集（L50-L53、L62-L65）。
   - 找不到目标时直接返回原 session，不写状态、不改工作区、不发事件（L68-L68）。找到后，rev.snapshot 复用已有 revert snapshot，否则调用 snap.track() 建立基线（L70-L70）。已有基线时先 snap.restore(session.revert.snapshot)，再对新收集的 patches 调用 snap.revert(patches)；这保证连续选择更晚消息时先恢复到原始基线、再重放较短后缀（L71-L72）。
   - 若存在 snapshot，调用 snap.diff(rev.snapshot) 保存从基线到当前回退工作区的差异（L73-L73）。以 rev.messageID 在消息数组中定位起点，缺失则使用空 range；summary.computeDiff({messages: range}) 计算消息侧 diff（L74-L76）。
   - 将消息 diff 写入 Storage 的键 ["session_diff", input.sessionID]，并用 Effect.ignore 有意吞掉该写入失败；随后发布 Session.Event.Diff，载荷含 sessionID 与 diffs（L77-L78）。最后 sessions.setRevert 持久化 rev 和汇总：additions、deletions 求和，files 为 diff 数组长度（L79-L87），再重新 sessions.get 返回更新后的 Session.Info（L88-L88）。
   - 默认值/边界：无 partID 即消息级，但匹配发生在 part 循环内，因此目标消息若没有任何 part 也会静默 no-op；有 partID 但找不到目标时同样 no-op；有旧 snapshot 时不会重新 track；空 patches 仍会执行 snapshot 恢复/状态更新流程。该实现没有显式排序、分页或重试。

3. **unrevert（L91-L99）**
   - 记录 info 日志，先执行同样的 assertNotBusy，再读取会话（L91-L94）。没有 session.revert 时原样返回，不产生任何副作用（L95-L95）。
   - 存在 snapshot 则恢复它，随后 sessions.clearRevert 清除回退标记，最后重新读取并返回会话（L96-L98）。因此 unrevert 只恢复工作区和状态，不删除消息；恢复失败会中断后续清除。

4. **cleanup（L101-L124）**
   - 输入是已取得的 Session.Info；没有 revert 状态立即返回（L101-L103）。有状态时读取消息，按 revert.messageID 找索引（L104-L107）。
   - 无 partID：msgs.slice(index) 从目标消息开始删除到末尾；有 partID：msgs.slice(index + 1) 只删除目标消息之后的消息（L108-L109）。索引不存在时删除集合为空，因此不会误删全部记录。
   - 逐条调用 sessions.removeMessage（L109-L111）。若是 part 级回退且目标消息存在，找到 part 索引后把 target.parts 截断为前缀，并逐 part 调用 sessions.removePart 删除从该 part 开始的后缀（L112-L121）；找不到 part 时不删除 part。最后无条件 sessions.clearRevert(sessionID)（L123-L123）。
   - 该函数是实际的消息/part 清理阶段，与 revert 的“只移动工作区并标记状态”分离；可重复调用，清理后第二次会成为 no-op。

## 状态、取消、恢复与副作用

- 忙状态：revert/unrevert 在任何读取或工作区操作前调用 SessionRunState.assertNotBusy（L39-L40、L92-L94），形成串行化边界；cleanup 自身不检查 busy，调用方必须提供合适的生命周期时机。
- 取消/超时：本文件没有 Effect.timeout、Effect.retry、取消 token 或 finally 补偿。Effect 运行时取消可能打断任意 yield；已经完成的 snap.restore、snap.revert、消息删除、事件发布不会被自动回滚，可能留下工作区与 session.revert 不一致的中间状态。
- 恢复：重复 revert 通过旧 session.revert.snapshot 先 restore，再应用新的 patch 后缀（L70-L73）；unrevert restore 同一 snapshot 并清标记（L95-L98）。cleanup 不恢复文件，只清理对话尾部并清标记（L108-L123）。
- 持久化：sessions.setRevert/clearRevert 持久化回退元数据；Storage.write(["session_diff", ...]) 写消息 diff 但错误被忽略（L77-L87）。EventV2Bridge.publish 是外部事件副作用，可能在存储写入忽略后继续发生（L78）。
- 外部副作用：Snapshot.Service 修改工作区文件（track/restore/revert/diff）；Session.Service 修改消息与 part；EventV2Bridge 向订阅者广播；没有 provider 网络调用。多步操作之间没有事务封装或幂等键。

## 源内测试与行为判据

源内包含测试，主要位于 packages/opencode/test/session/revert-compact.test.ts：
- revert 后消息仍完整、cleanup 才删除目标及后续消息，并清除 revert 状态（L244-L267）。
- 回退到用户消息后 cleanup 会清空该消息及之后内容（L341-L355）；带 partID 时只保留该 part 之前的 part（L363-L395）；消息级回退保留更早消息、删除目标及其后消息（L400-L436）。
- 测试用时间字段验证按时间/数组顺序处理混合 ID，而非按 ID 字典序（L441-L469）。
- 连续对不同 turn 回退会按顺序恢复文件；unrevert 恢复最后一次回退前的文件并清除标记（L499-L591、L596-L679）。
- 无 revert 状态的 cleanup 是 no-op（L474-L495）。
- schema-decoding.test.ts 验证 messageID 必填、partID 可选，缺失 messageID 必须抛错（L201-L212）。
可独立验证的判据：构造三条带 patch 的消息，依次调用 revert(A)、revert(B)、unrevert，检查文件内容依次为基线、A 后状态、B 后状态、最终原状态；再调用 cleanup，确认消息/part 后缀及 revert 元数据被移除而前缀保留。

## zenpi Rust 映射

对照结果显示 zenpi 当前没有 SessionRevert、Snapshot.Service、session_diff 或 provider 级工作区回退 API；rg 在 src/headless.rs、src/core.rs、src/session.rs、src/tool_runtime.rs、src/runtime.rs、src/protocol.rs、src/approval.rs、src/providers/** 未发现同名实现。

建议落点与可执行验证：

1. **会话与回退状态：src/session.rs**。新增可序列化 RevertState { message_id: String, part_id: Option<String>, snapshot_id/digest: String, summary: DiffSummary }，通过 append-only event（如 session_revert_set/session_revert_clear）持久化；实现 SessionStore::set_revert、clear_revert、revert_state、cleanup_revert。用现有 SessionRecord 回放验证崩溃后状态可恢复。现有 SessionStore::append_turn 只追加 turn（L863-L889），没有 removeMessage/removePart，所以 cleanup 应追加“隐藏/截断”事件并在投影层过滤，或明确实现新的原子重写协议。
2. **工作区快照：新增 src/workspace_snapshot.rs（或并入 src/core.rs）**。实现 track、restore、revert_patches、diff，优先以受控 git tree/index 或带 SHA-256 的文件清单保存基线；验证连续回退的 restore-then-apply 顺序与路径越界拒绝。该能力不应放入 src/providers/**，因为 provider 只负责模型传输/重试。
3. **核心门禁：src/core.rs**。在 Agent 增加 revert_session、unrevert_session、cleanup_revert，要求 phase == AgentPhase::Idle，否则返回既有 AgentError::NotIdle（Agent::snapshot 暴露 phase，L1341-L1350）。将 session journal、workspace snapshot、diff summary 按明确顺序提交；对 Storage.write 被忽略这一点，Rust 版本应改为显式 Result 并记录可恢复错误。
4. **传输与 headless：src/protocol.rs、src/headless.rs**。在 Command 增加 Revert { session_id, message_id, part_id }、Unrevert、Cleanup 或一个带 action 的结构；在 StdioRequest::into_command 做 ID/长度校验，然后由 headless 的 session owner 分发。现有 Cancel 只请求运行时取消（Command::Cancel，L220-L251），不能替代回退；session slash dispatch 当前只有 list/search/open/fork 等生命周期动作（L7697-L7772）。
5. **取消、并发与审批：src/runtime.rs、src/tool_runtime.rs、src/approval.rs**。复用 CancellationToken 的协作式语义（L49-L99）和 Runtime::try_cancel 非阻塞请求（L318-L325），但在回退步骤间检查取消并把未完成状态写成 interrupted；不要把取消误当成文件 rollback。工具批处理明确规定取消不是回滚证据（tool_runtime.rs L157-L167、L236-L239），应作为新 API 的错误判据。ApprovalCoordinator::emergency_cancel 只取消等待中的审批（L438-L443），不应承担回退。
6. **providers/**：无需修改。src/backend.rs/src/providers/connection.rs 的取消、流式传输、retry-after 只影响模型请求；回退的文件副作用应由 workspace/session owner 管理。可验证差异是 provider 测试无需知道 RevertState，而 core 集成测试要覆盖 provider turn 完成后回退。

关键差异清单：TypeScript Effect 依赖层与 orDie 缺陷语义对应 Rust 的显式 Result；OpenCode 的可变消息/part 存储对应 zenpi 的 append-only JSONL；OpenCode 按消息数组顺序扫描并可物理删除，zenpi 现有 session tree 更适合追加分支/投影；OpenCode 忽略 diff storage 写失败，zenpi 应保留错误并可重放；OpenCode 没有本文件级取消/重试，zenpi runtime 已有协作式取消且 provider 有 retry，因此不能自动推断“取消即恢复”。

## 未决问题

- Snapshot.Patch 的具体 patch 方向、哈希校验和 Snapshot.Service 对未跟踪/删除文件的精确处理不在本文件中，需继续核对 src/snapshot 实现与测试。
- sessions.setRevert、Session.Info.revert、SessionSummary.computeDiff 的持久化格式和并发锁细节由依赖模块定义，本文件只能确认调用顺序与错误处理。

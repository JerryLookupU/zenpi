# AC-028 — codex/codex-rs/core/src/tasks/ghost_snapshot.rs

- source_id/item_id：`codex/codex-rs/core/src/tasks/ghost_snapshot.rs` / `AC-028`
- source_path：`/Users/wangweiyang/GitHub/codex/codex-rs/core/src/tasks/ghost_snapshot.rs`
- source_hash：`dbfc0e2734f96511fdffc3646ba2fbaac7905a1d07aa3aad95513dadd43928da`
- source_bytes：`10217`
- source_lines：`254`
- coverage：已从首字节到末字节顺序读取，覆盖字节 `1-10217`、行 `L1-L254`；同目录测试 `ghost_snapshot_tests.rs` 亦顺序读取 `L1-L31`。

## 完整行为复盘

- `GhostSnapshotTask`（`pub(crate)`，`L23-L25`）只保存一个 `codex_utils_readiness::Token`；`SNAPSHOT_WARNING_THRESHOLD`（`L27-L27`）固定为 `240s`，是慢任务提示阈值而非取消超时。
- `impl SessionTask for GhostSnapshotTask`（`L29-L161`）：`kind`（`L31-L33`）恒返回 `TaskKind::Regular`；`span_name`（`L35-L37`）恒返回 `"session_task.ghost_snapshot"`。
- `run`（`L39-L160`）输入为 `Arc<SessionTaskContext>`、`Arc<TurnContext>`、未使用的 `Vec<UserInput>` 与 `CancellationToken`，输出始终为 `None`（`L158-L160`）。它先 `tokio::task::spawn` 一个后台任务并立即把控制权交回调用者，因此调用者不会等待快照完成。
  - 后台任务复制 `token`，依据 `ctx.ghost_snapshot.disable_warnings` 得出 `warnings_enabled`（`L46-L48`），创建 `oneshot` 完成信号（`L49-L51`）。开启提示时再派生一个 Tokio 任务（`L53-L59`），用 `tokio::select!` 竞争 `240s` 睡眠、`snapshot_done_rx` 或取消（`L60-L73`）；睡眠胜出则通过 `session_for_warning.session.send_event` 发出一条 `EventMsg::Warning`，内容指出大型未跟踪/忽略文件会拖慢快照并建议 `.gitignore` 或关闭 `undo`（`L61-L69`）。完成信号或取消只让提示任务静默退出；禁用提示时直接丢弃 receiver（`L75-L77`）。
  - 主任务再用 `tokio::select!` 竞争取消与快照工作（`L79-L145`）。工作复制 `cwd`、`ghost_snapshot` 配置（`L83-L86`），必须在 `spawn_blocking` 专用阻塞池中执行 `CreateGhostCommitOptions::new(&repo_path).ghost_snapshot(...)` 和 `create_ghost_commit_with_report`（`L87-L92`）。取消先到则 `cancelled=true`，不会等待该阻塞句柄；因此底层阻塞 Git 操作可能在取消后继续产生外部副作用。
  - 成功分支 `Ok(Ok((ghost_commit, report)))`（`L94-L118`）先记录完成日志；若开启提示，按 `format_snapshot_warnings`（`L97-L109`）逐条异步发送 `WarningEvent`；然后把 `ResponseItem::GhostSnapshot { ghost_commit }` 写入 `record_conversation_items`（`L111-L115`），最后记录 commit id（`L117-L117`）。这表示会话持久化只发生在成功且返回 commit 时。
  - `GitToolingError::NotAGitRepository` 仅记 `info!` 并跳过（`L119-L123`）；其他 Git 错误只 `warn!`（`L124-L129`），不向调用者返回错误。`spawn_blocking` join 错误（通常 panic）记警告并通过 `notify_background_event` 发出“Snapshots disabled after ghost snapshot panic”消息（`L131-L141`）。
  - 任务结束后无论成功、失败还是取消，都尝试 `snapshot_done_tx.send(())`（`L147-L147`）；取消时额外记录 `ghost snapshot task cancelled`（`L149-L151`）。最后调用 `ctx.tool_call_gate.mark_ready(token).await`（`L153-L157`）：`Ok(true)` 表示门闩首次就绪，`Ok(false)` 表示已就绪，`Err` 只记录警告。该顺序保证后台准备任务不会永久阻塞后续 tool gate。
- `GhostSnapshotTask::new`（`L163-L167`）是构造器，直接保存传入 `Token`，无默认值、无校验、不会启动任务。
- `format_snapshot_warnings`（`L169-L184`）输入两个可选阈值和 `&GhostSnapshotReport`，固定先追加目录警告再追加文件警告，返回 `Vec<String>`；两个 helper 均无异常返回，缺少阈值或报告为空时返回空向量。
- `format_large_untracked_warning`（`L186-L207`）报告目录为空即 `None`（`L190-L192`）；阈值为 `None` 也返回 `None`（`L193-L193`）。阈值存在时最多展示 `MAX_DIRS=3` 个 `path (file_count files)`（`L194-L198`），超过三项追加 `N more`（`L199-L202`），消息说明目录被排除出 snapshot 与 undo cleanup，并指向 `ghost_snapshot.ignore_large_untracked_dirs`（`L203-L206`）。
- `format_ignored_untracked_files_warning`（`L209-L237`）先要求阈值存在（`L213-L213`），再要求 `ignored_untracked_files` 非空（`L214-L216`）；最多展示三项路径和 `format_bytes(byte_size)`（`L218-L225`），多余项同样压缩为 `N more`（`L227-L230`）。消息明确文件在 undo cleanup 中保留但内容不进 snapshot，并指向 `ghost_snapshot.ignore_large_untracked_files` 与 `.gitignore`（`L232-L236`）。
- `format_bytes`（`L239-L250`）采用二进制单位整数整除：`>=1 MiB` 为 `"N MiB"`，否则 `>=1 KiB` 为 `"N KiB"`，其余为 `"N B"`；不足一个单位时向下取整，负数也落入 `B` 分支。`#[cfg(test)] mod tests`（`L252-L254`）把同目录 `ghost_snapshot_tests.rs` 编入本模块。

## 状态、取消、恢复与副作用

状态主要在 Tokio 任务栈、`CancellationToken`、`oneshot` 完成信号和 `tool_call_gate` 中；`GhostSnapshotTask` 本身无持久状态机。取消是合作式竞速：外层 `select!` 可立即结束等待，但 `spawn_blocking` 中已开始的 Git 操作不可强杀，可能晚于取消完成。240 秒只触发提示，不会自动重试、回滚或超时失败。代码没有重试/恢复分支；非 Git 目录与普通 Git 错误都被记录后结束，panic 通过背景事件提示快照被禁用。成功快照会创建 ghost commit（仓库外部副作用）并将 `ResponseItem::GhostSnapshot` 追加到会话记录；警告事件和 panic 通知也是外部可见副作用。没有单独的 checkpoint 或 undo 持久化协议，真正的撤销语义由 `codex_git` 完成。完成信号与取消信号存在竞态，提示任务以先观察到的分支为准；`mark_ready` 是最终释放点，即使前面失败也会执行。

## 源内测试与行为判据

源文件只通过 `#[path = "ghost_snapshot_tests.rs"]` 引入测试（`L252-L254`），同目录测试 `ghost_snapshot_tests.rs`：`large_untracked_warning_includes_threshold`（`L6-L18`）构造一个 `models`、250 文件的报告，断言阈值 200 的目录警告包含 `>= 200 files`；`large_untracked_warning_disabled_when_threshold_disabled`（`L20-L31`）断言阈值 `None` 时返回 `None`。可独立验证的判据还包括：关闭 `disable_warnings` 时不创建有效慢任务告警；成功报告按目录后文件顺序发 warning 并记录一个 `ResponseItem::GhostSnapshot`；取消后最终仍能观察到 `mark_ready`；非 Git 路径不产生错误返回；`format_bytes(1024/1048576)` 分别为 `1 KiB/1 MiB`。

## zenpi Rust 映射

zenpi 可直接复用的通信原语有三层。持久、可寻址的节点间消息使用 `src/session.rs` 的 `SessionMailbox` 与 `MailboxMessage`/`MailboxStatus`（`Queued`、`Acknowledged`、`Claimed`、`Succeeded`、`Failed`、`Expired`）：`enqueue` 具备 request id 幂等、64 KiB payload、最多 7 天 TTL，`list/update` 在 inode 锁下追加同步记录；`src/protocol.rs` 的 `MailboxRequest::{Send,Receive,Acknowledge,Claim,Complete}` 是 JSONL 协议面，`src/headless.rs` 的 `mailbox_slash_view`/`execute_mailbox` 是统一入口。瞬时进程内通知复用 `core::AgentEvent`、`runtime::RuntimeEvent` 与 `headless` 的有界 event mailbox；`InputBoundaryGate`/`CancellationToken` 提供边界取消。provider 流事件仍走 `src/providers/**` 对应的 backend route，不应承担 DAG 拓扑或持久队列。

会话父子关系目前只有 `core::Turn { id, parent_id }`（`Turn::with_parent` 可建直接父链），以及 `WorkerExecutionBinding { blueprint_id, goal_id, item_id, lease_id, expires_at_ms }` 的工作关联；它们不足以表达完整 DAG。建议新增 `src/dag.rs`（或 `src/domain_execution` 的持久类型）`DagNode { node_id, session_id, parent_id, child_ids, status, generation, lease_id }`、`DagStatus::{Pending,Running,Green,Blocked,Closed}` 和 `DagEnvelope { request_id, sender_node, recipient_node, relation, payload, ttl_ms }`，将 `parent_id` 递归索引为 grandparent，将同一 `parent_id` 的其他节点解析为直接 sibling，将 `child_ids` 解析为直接 child；grandchild 是 child 的 child。持久记录应追加到 `SessionStore`/执行日志，不能只依赖内存 `LiveSessionRegistry`。

最小可执行的保活/派生机制：worker 获得带 `lease_id`/`expires_at_ms` 的 `WorkerExecutionBinding` 后，以固定间隔向 `Agent::heartbeat_live_owner` 或一条 mailbox heartbeat 发送保活；`LiveSessionRegistry::{register,heartbeat,active}`（`src/session.rs`）可作为进程内租约门槛，`Agent::{claim_live_mailbox,finish_live_mailbox_with_reply}` 负责领取与回执。worker 在每个节点完成时写 `DagNode.status=Green`，聚合 `self`、全部 child、grandchild 的状态；只有全绿才写 `Closed` 并释放 lease，否则保持 `Running/Blocked`、继续 heartbeat。发现新工作时，通过 `runtime::BackgroundRunner::try_submit` 派生最小新 job，并监听 `RuntimeEvent::{Accepted,Queued,Started,Completed,Rejected,CancelRequested}`；派生请求必须带新 `node_id`、父节点和幂等 request id，不能在 `ghost_snapshot` 式取消后假设副作用已回滚。

具体落点与差异：`src/headless.rs` 已有有界异步 worker、mailbox 路由、`blueprint_handoff_next`，应增加 DAG relation 校验、状态聚合和派生响应；`src/core.rs` 的 `Agent`、`WorkerExecutionBinding`、`admit_blueprint_worker`、`heartbeat_live_owner` 适合承载节点租约与准入，但当前只校验单个 Blueprint item，没有“全部后代全绿才 close”；`src/session.rs` 负责 `DagNode`/信封/回执的 append-only 持久化、TTL、幂等和恢复扫描；`src/runtime.rs` 的 bounded `BackgroundRunner` 是唯一派生/取消入口，现有 `Cancelled` 明确不等于副作用回滚；`src/protocol.rs` 可扩展 `MailboxRequest` 或新增 `DagRequest`，保留 `MAX_ID_BYTES`、`MAX_MAILBOX_TEXT_BYTES` 和版本化 JSONL；`src/approval.rs` 的 `ApprovalMode::WorkerAllowAfterPreflight`、`ApprovalCoordinator` 与取消 epoch 应继续约束派生 worker，不能把 mailbox 到达当成 approval；`src/tool_runtime.rs` 的 `execute_tool_batch` 已保证批量边界、顺序结果和取消协作，节点状态应在其持久化完成后再判绿；`src/providers/**` 只负责 `Protocol`/route/model/auth 能力解析，DAG worker 应经 `Agent` 调用 provider，禁止直接在 provider 模块创建线程或绕过 approval。差异清单是：缺少 DAG 节点/边持久模型、grandparent/sibling/descendant 查询、全后代聚合 close 规则、lease 心跳与过期后的安全恢复、派生请求幂等键及父链验证；这些均可用上述现有边界加可测试纯函数补齐。

## 未决问题

- 源文件未定义 `codex_git::create_ghost_commit_with_report` 的原子性、undo 清理细节或阻塞操作被取消后的最终完成时刻；只能确认本文件不会主动回滚。
- `SessionTaskContext::tool_call_gate.mark_ready` 在重复 token、会话关闭或错误时的具体持久语义不在本文件中。
- zenpi 的 DAG 状态持久化格式、节点数量上限、heartbeat 周期、全绿判定是否要求忽略 `Expired/Abandoned` 尚未由现有列出的模块明确规定，需在新增协议/测试中决定。

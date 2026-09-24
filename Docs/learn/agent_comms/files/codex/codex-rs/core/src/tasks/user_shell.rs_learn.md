# AC-033 — codex/codex-rs/core/src/tasks/user_shell.rs

- source_id/item_id: `AC-033`
- source_path: `codex/codex-rs/core/src/tasks/user_shell.rs`
- source_hash: `f87841aadd7ec30bd62c34ed0618403c311b9f01107af3f3d53bbdd1b72ace98`
- source_bytes: `13341`
- source_lines: `357`
- coverage: 已按文件顺序读取完整内容；字节范围 `1-13341`，行范围 `L1-L357`。

## 完整行为复盘

文件头 `L1-L40` 只建立执行所需依赖：`Arc`/`Duration`、`async_trait`、`CancellationToken`、`Uuid`，以及 `TurnContext`、`Session`、`ExecRequest`、`EventMsg`、`SandboxPolicy`、`ResponseItem` 等。通信方向是 `Session::send_event` 发出生命周期/执行事件，`StdoutStream` 通过 `tx_event` 转发流式输出；持久化通过 `Session` 完成，而非本文件直接写文件。

常量 `USER_SHELL_TIMEOUT_MS` 在 `L42` 固定为 `60 * 60 * 1000`（1 小时）。它是 `ExecRequest.expiration` 的默认硬上限；注释明确这是暂时的“大超时”，未来应改用 `ExecExpiration::Cancellation`。

`UserShellCommandMode`（`L44-L52`）是 `pub(crate)`、`Copy` 的二值策略：`StandaloneTurn` 表示独立 turn，由任务生命周期发出 `TurnStarted/TurnComplete`；`ActiveTurnAuxiliary` 表示已有 turn 中的辅助命令，不能重复发同一对生命周期事件。枚举本身不携带命令或会话状态。

`UserShellCommandTask`（`L54-L57`）只保存一个 `command: String`，`new`（`L59-L63`）无校验地接收并移动字符串。实际边界检查、shell 选择和执行由后续路径负责，因此空命令、超长命令是否被接受不在此构造函数决定。

`impl SessionTask for UserShellCommandTask`（`L65-L92`）的三个导出行为如下：

- `kind`（`L67-L69`）恒返 `TaskKind::Regular`。
- `span_name`（`L71-L73`）恒返指标名 `"session_task.user_shell"`。
- `run`（`L75-L91`）接收 `Arc<Self>`、`Arc<SessionTaskContext>`、`Arc<TurnContext>`、被忽略的 `Vec<UserInput>` 和 `CancellationToken`。它克隆 `Session`，克隆命令，以 `StandaloneTurn` 调用 `execute_user_shell_command` 并等待完成；无论执行成功、失败或取消，任务都返回 `None`（`L82-L90`），所以结果通过事件/会话记录传递，不通过返回值传递。

`execute_user_shell_command`（`L94-L319`）是主执行函数，返回 `()`，但会产生遥测、事件、外部进程和会话记录副作用：

1. `L101-L104` 递增 `codex.task.user_shell` 计数器。`L106-L120` 仅当 `mode == StandaloneTurn` 时发送 `EventMsg::TurnStarted`，事件字段来自 `turn_context.sub_id`、`model_context_window()`、`collaboration_mode.mode`。辅助模式跳过该事件，避免 active turn 出现重复生命周期。
2. `L122-L133` 选择用户默认 shell：`use_login_shell = true`，通过 `session.user_shell()` 和 `derive_exec_args(&command, true)` 生成展示/执行参数；随后调用 `maybe_wrap_shell_lc_with_snapshot`，将当前工作目录 `turn_context.cwd` 与 `shell_environment_policy.r#set` 交给 snapshot 包装器。注释说明支持管道、`&&`、重定向等 shell 语法，但不主动 source rc 文件，也不重新格式化脚本。
3. `L135-L139` 为一次调用生成随机 `Uuid` 字符串 `call_id`，保留未包装的 `raw_command`，克隆 `cwd`，并对 `display_command` 调用 `parse_command`。解析结果只是事件字段；解析失败的具体行为取决于 `parse_command`，本函数没有 `Result` 分支。
4. `L140-L154` 发送 `ExecCommandBegin`。事件绑定 `call_id`、`turn_id`、`cwd`、原始展示命令、`parsed_cmd`，来源固定为 `ExecCommandSource::UserShell`，`process_id` 与 `interaction_input` 为 `None`。事件发送是异步等待的，因而事件总在启动执行请求前发出。
5. `L156-L180` 构造 `ExecRequest`。`sandbox_policy` 固定 `DangerFullAccess`，同时 `sandbox = SandboxType::None`；文件与网络策略从该值转换，`sandbox_permissions = UseDefault`。环境由 `create_env(&shell_environment_policy, Some(session.conversation_id))` 生成；工作目录、网络配置、Windows 沙盒级别/私有桌面、命令包装结果均来自 `TurnContext`。`expiration` 为 1 小时，`justification`/`arg0` 为 `None`。
6. `L182-L186` 创建 `StdoutStream`，绑定 `sub_id`、`call_id` 与会话事件发送端；因此子进程 stdout 的增量可以和本次调用及 turn 关联。
7. `L188-L195` 调用 `execute_exec_request(exec_env, &sandbox_policy, stdout_stream, None)`，再用 `.or_cancel(&cancellation_token)` 等待。返回值是三层语义：外层 `Err(CancelErr::Cancelled)` 表示取消；`Ok(Ok(output))` 表示执行请求完成（无论退出码是否为 0）；`Ok(Err(err))` 表示执行器错误。这里没有重试，也没有并发启动第二个命令。
8. 取消分支 `L197-L238` 构造固定输出：`exit_code=-1`、空 stdout、stderr/aggregated output 为 `"command aborted by user"`、`duration=Duration::ZERO`、`timed_out=false`。先调用 `persist_user_shell_output`（`L208-L215`），再发送 `ExecCommandEnd`，状态固定 `Failed`，格式化输出就是中止消息。也就是说取消仍会留下可恢复的失败记录和终止事件。
9. 成功执行分支 `L239-L272` 先发送 `ExecCommandEnd`，保留真实 stdout/stderr/aggregated output、退出码和耗时；退出码为 0 时状态 `Completed`，否则为 `Failed`。`formatted_output` 使用 `format_exec_output_str(&output, turn_context.truncation_policy)`，随后持久化原始命令和完整 `ExecToolCallOutput`。
10. 执行器错误分支 `L273-L318` 记录 tracing `error!`，把错误包装成 `execution error: {err:?}`，构造同样的 `exit_code=-1`/零时长输出；发送 `ExecCommandEnd`（状态 `Failed`）后持久化。该错误不会向调用者抛出，因为函数返回 `()`。

`persist_user_shell_output`（`L321-L357`）先在 `L328` 调用 `user_shell_command_record_item(raw_command, exec_output, turn_context)`，把命令和执行结果转换成 `ResponseItem`。其模式差异是核心持久化契约：

- `StandaloneTurn`（`L330-L337`）：把单个 `output_item` 通过 `record_conversation_items` 写入会话，然后显式 `ensure_rollout_materialized`。这是因为独立 shell turn 可能早于普通用户 turn，必须立即物化 rollout；随后返回。
- `ActiveTurnAuxiliary`（`L340-L357`）：要求 `output_item` 是 `ResponseItem::Message`，否则 `unreachable!`；转换成 `ResponseInputItem::Message` 并调用 `inject_response_items`，让结果进入当前 active turn 的模型上下文。如果注入返回 `Err(items)`，函数把这些退回的输入转换回 `ResponseItem`，再用 `record_conversation_items` 记录，形成注入失败时的持久化兜底。该路径不调用 `ensure_rollout_materialized`。

## 状态、取消、恢复与副作用

状态边界由 `UserShellCommandMode`、`TurnContext.sub_id` 和 `Session` 所有权决定；本文件不维护自己的 worker 状态机。`StandaloneTurn` 负责完整 turn 起点和独立 rollout，`ActiveTurnAuxiliary` 复用已有 turn，避免重复 `TurnStarted/TurnComplete`。所有 `send_event` 都是 `await`，因此事件顺序是：独立起点（若有）→ `ExecCommandBegin` → 流式 stdout（由 `StdoutStream` 异步发出）→ `ExecCommandEnd` → 会话记录。

取消是协作式的：`CancellationToken` 传给 `.or_cancel`，取消后本函数生成失败结果并继续事件/持久化收尾。1 小时 `expiration` 是执行器层超时上限；本文件没有单独 timeout 事件、重试计数或退避。`timed_out` 在本文件自行构造的取消/错误结果中为 `false`，真实执行输出的超时字段由 `execute_exec_request` 决定。

副作用包括：执行用户 shell 脚本（且策略是 `DangerFullAccess`）、写 stdout/stderr 事件、写 `ExecCommandBegin/End`、递增 telemetry、写/注入会话历史。没有显式 approval、网络请求或持久化重试。辅助模式的 `inject_response_items` 失败仍会写记录，但不会在本文件重新执行命令。恢复依赖 `Session` 的 durable rollout/会话记录；独立模式显式调用 `ensure_rollout_materialized`，辅助模式则依赖当前 turn 的注入或 fallback 记录。

并发语义是“任务可并存但一次命令有一个调用上下文”：`Arc<Session>`/`Arc<TurnContext>` 允许共享，`call_id` 用于隔离事件；stdout 通过事件 channel 流式发送。源代码未声明全局串行锁，是否限制同一 session 的多个命令由上层 `SessionTask` 调度器负责。

## 源内测试与行为判据

源文件本身没有 `#[cfg(test)]`，`src/tasks/` 同目录也没有针对 `user_shell.rs` 的专门测试（`mod.rs` 仅在 `L56-L58` re-export）。因此“源内未包含测试”。可独立验证的行为判据如下：

1. `StandaloneTurn` 只产生一组 turn 生命周期事件；`ActiveTurnAuxiliary` 不产生第二个 `TurnStarted/TurnComplete`。
2. 每次执行严格产生一个 `ExecCommandBegin` 和一个 `ExecCommandEnd`，`call_id`/`turn_id`/`cwd` 一致；退出码 0 为 `Completed`，非 0、取消和执行器错误为 `Failed`。
3. 取消输出必须为 `exit_code=-1`、stderr `command aborted by user`，且仍有会话记录。
4. 独立模式写入记录并调用 rollout materialization；辅助模式优先注入，注入失败时 fallback 到 `record_conversation_items`。
5. 相关行为判据可由相邻源树测试复核：`codex-rs/core/src/user_shell_command_tests.rs:L16-L55` 验证输出记录格式、退出码/时长与 `aggregated_output` 优先级；`codex-rs/core/src/codex_tests.rs:L4049-L4076` 验证独立 shell 不改变旧的 reference context item。

## zenpi Rust 映射

zenpi 已有的 `src/dag.rs` 是最直接的目标落点。其模块注释 `L1-L10` 已明确“一 worker 一个 DAG 节点”，可与 parent、grandparent、direct sibling、direct child 通信；只有节点自身及全部 descendant 为 green 才能 close，否则保活并派生新 worker。`DagNode`（`src/dag.rs:L57-L77`）有 `parent`、`children`、`status(open|green|red)`、`worker` 和 `worker_heartbeat_ms`；`DagRelation`（`L79-L112`）将邻居关系类型化。`DagStore::recipients`（`L332-L400`）的 `all` 正好解析为 parent + grandparent + direct siblings + direct children，`send/inbox`（`L402-L437`）是消息/邮箱原语，文件锁和有界消息体（`MAX_DAG_BODY_BYTES`）保证并发快照不会互相覆盖。

可复用的通信原语应分三层：一是 `DagStore::send`/`inbox`/`wait_for_message`（`src/dag.rs:L600-L618`）作为跨进程 durable mailbox/event；二是 `src/session.rs` 的 `SessionMailbox`，它在 `L3570-L3592` 定义私有、按 recipient session 定址的 JSONL 邮箱，在 `L3640-L3697` enqueue、`L3700-L3740` 分页 list、`L3743-L3795` 做 acknowledge/claim/complete/fail 状态迁移；三是 `src/runtime.rs` 的 `BackgroundRunner` bounded `mpsc` 与 `CancellationToken`（`L236-L254`、`L712-L759`），作为进程内 worker 事件流。`src/core.rs:L281-L325` 的 `AgentEvent`、`LiveToolEventSink`（`L474-L503`）可承载进度/结果事件，但不能替代 durable mailbox。

会话父子关系要明确分层：`src/core.rs:L140-L204` 的 `Turn.parent_id` 只表示对话/turn 相关性；`src/session_tree.rs:L50-L73` 的 `TreeEntry.parent_id` 是 transcript entry graph；DAG worker 拓扑应继续由 `DagNode.parent/children` 表达，不能把 `Turn.parent_id` 当作 worker parent。建议在 `DagNode` 增加 `session_id`、`lease_id`、`generation`，建立 node→session 的不可变绑定；`src/session.rs:L3274-L3355` 的 `LiveSessionRegistry` 可保存 session owner、`owner_epoch`、workspace 和 heartbeat，并拒绝过期 owner。

最小保活/派生机制已有大部分零件：`DagStore::touch_worker`（`src/dag.rs:L320-L329`）写心跳，`wait_for_message` 最多轮询 30 秒且接受取消，`spawn_worker`（`L499-L550`）启动 `zenpi-dev --auto --mode headless`，保留 stdin 以便 `send_to_worker`（`L571-L585`）发送 follow-up；`status_view` 会展示 heartbeat/未完成 descendant。建议在 worker 主循环固定每 `T` 秒调用 `touch_worker`，收到取消或父节点关闭后退出；新工作先 `send` 到已有 direct child，只有无可用 child 才 `spawn_worker`。`can_close`（`L448-L477`）已经实现“自身 green 且递归所有 child/grandchild green”的 gate；当返回 `false` 时，worker 必须保持运行并根据 unfinished 列表派生/发送新工作，而不是标记 close。

对照各既有文件的可执行落点：

- `src/headless.rs:L553-L581` 的 `run_headless` 已注册 `LiveSessionOwner`、读取有界 JSONL、EOF 时注销并关闭 Agent；`L2065-L2110` 的 `mailbox_slash_view` 已把协议 mailbox 动作映射到 owner。应在 headless worker 路由加入 `dag_status`、`dag_recv`、`dag_send`、`dag_keepalive`、`dag_spawn`、`dag_close`，并让 worker 事件循环在未 close 时继续读 stdin/邮箱。
- `src/core.rs:L402-L442` 的 `Agent` 与 `AgentPhase`（`L273-L279`）适合作为单 worker 的执行状态；`WorkerExecutionBinding`（`L37-L81`）已有 blueprint/goal/item/lease/policy digest/expiry 校验。建议新增 `DagWorkerContext { node_id, session_id, lease_id, parent, children }`，由 `run_active_turn_cancelable`/`run_user_shell_with_cancel`（现有入口在 `L2961` 附近）把 cancellation 与 DAG lease 绑定，并把每个节点状态/结果写 `AgentEvent`。
- `src/session.rs:L861-L891` 的 `append_turn` 保证先 durable write 后更新内存，`L1205-L1224` 的 `append_event` 提供不透明事件记录，适合写 `dag_message_sent/received/heartbeat/close_blocked/worker_spawned`；`LiveSessionRegistry::claim_next/finish_claim`（`L3361-L3488`）可复用为“活 worker 领取新任务并以 owner epoch 完成”。
- `src/tool_runtime.rs:L157-L176` 的 `execute_tool_batch` 明确有界、可取消、不自动重试，`L31-L42` 的 `MasterSessionCommand` 已把 `!command` 分成 Bash 与 steer；DAG worker 的 shell/tool 执行应复用此批处理和 approval/gate，而不是直接绕过 owner 另起无限线程。
- `src/runtime.rs:L49-L99` 的 `CancellationToken` 是最小取消原语，`L101-L123` 的 `RuntimeConfig` 给出 bounded command/event/pending 队列，`L655-L693` 在结束时先发 terminal 再取消 pending。可将 DAG worker 的保活任务作为同一 `BackgroundRunner` job 的周期性 side task；取消时先发 `CancelRequested`，再写 `red`/`close_blocked` 结果，绝不强杀任意 Rust 线程。
- `src/protocol.rs:L81-L119` 已有 `MailboxOutcome`、`MailboxRequest`、`UserShellRequest`，`Command::Mailbox` 及 `StdioRequest::into_command` 的 mailbox 校验在 `L479-L485`、`L544-L585`。建议新增带 `node_id`、`relation: DagRelation`、`request_id`、`lease_id`、`ttl_ms` 的 typed DAG command/event；复用现有 bounded identifier/text 校验和 `StdioEvent`/`StdioResponse`，确保每条发送/接收都有可重放相关 ID。
- `src/approval.rs:L126-L151` 的 `ApprovalCoordinator` 是进程内“请求—响应—取消”条件变量邮箱；`request_inner` 在 `L194-L245` 支持取消等待，`persist_accepted` 在 `L370-L390` 要求先持久化再放行副作用，`emergency_cancel` 在 `L438-L446` 可停止正在执行的 worker。DAG worker 应使用 `WorkerExecutionBinding` + `ApprovalMode::WorkerAllowAfterPreflight`，禁止把一个 worker 的记忆授权扩散给 sibling/child。
- `src/providers/**`（例如 `src/providers/mod.rs:L1-L50`、`connection.rs:L40-L60`、`registry.rs:L1-L16`）只负责 provider protocol、route、auth、model capability 和本地 catalogue；它们不是 worker IPC。保留 provider stream 为 `AgentEvent::Provider`（`src/core.rs:L313-L316`），DAG mailbox/heartbeat/close 仍由 `dag.rs`、`session.rs`、`headless.rs` 管理，避免把 provider 重试或 HTTP 生命周期误当成 DAG 节点完成。

差异清单与验证动作：

1. 当前 `DagStore::can_close` 是只读检查，检查与真正 close 之间可能竞态；应添加锁内 `try_close(id, expected_generation)`，原子复核自身/全部 descendant green 后写 closed 事件，否则返回排序后的 unfinished IDs。测试两个并发 worker 不能双重 close。
2. 当前 `spawn_worker` 有进程内 `SPAWNED` 表，但没有 durable spawn lease/idempotency key；为每个派生请求写 `spawn_request_id`，重启后同 request ID 不重复派生。验证重放同一请求只得到一个 child。
3. 当前 `DagMessage.body` 是截断字符串（`L114-L122`），可新增 `kind/request_id/reply_to/ttl/attempt` 的 `DagEnvelope`，保留 4096 字节上限；验证 parent↔grandparent、sibling、child 的地址解析和错误 recipient 不产生消息。
4. 当前 heartbeat 没有统一过期阈值；把 `worker_heartbeat_ms` 与 `LiveSessionRegistry::active` 的 TTL 绑定，过期只能标为 stale，不能自动假定 green。验证 stale worker 的 claim 被拒绝且可由新 owner epoch 接管。
5. 将 `can_close=false` 的 unfinished 列表接入 worker 主循环：保活、向已有 child `send_to_worker`、必要时 `spawn_worker`，再回到 `wait_for_message`；验收条件是“节点自身及全部 child/grandchild 全绿后才 close，否则 worker 仍存活并产生新 worker/消息”。

## 未决问题

1. `execute_exec_request` 在 `or_cancel` 触发时是否终止底层子进程、是否等待其退出，无法由本文件确认。
2. 真实执行器如何设置 `ExecToolCallOutput.timed_out`、如何区分超时与取消，需查看 `exec` 模块才能确认。
3. `inject_response_items` 返回 `Err(items)` 的具体失败原因及其与 rollout materialization 的事务边界不在本文件定义。
4. zenpi 当前 DAG 的“检查后关闭”是否已有跨进程原子 gate、派生是否有 durable idempotency，需以运行时实现和集成测试进一步核对；本笔记给出的 `try_close`/spawn lease 是可执行的补齐方案。

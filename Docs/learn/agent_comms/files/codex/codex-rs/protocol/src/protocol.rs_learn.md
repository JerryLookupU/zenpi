# AC-039 — codex/codex-rs/protocol/src/protocol.rs

- source_id/item_id: `codex/codex-rs/protocol/src/protocol.rs` / `AC-039`
- source_path: `/Users/wangweiyang/GitHub/codex/codex-rs/protocol/src/protocol.rs`
- source_hash: `dd9fb47475b0740800460353ce0798e1611430ea68f78f29e237386262713706`
- source_bytes: `155755`
- source_lines: `4429`
- coverage: 已按源文件顺序读取完整字节范围 `1-155755`、行范围 `L1-L4429`（含注释、导入、导出、类型、实现及测试）。

## 完整行为复盘

文件定义 Codex 会话的 SQ/EQ 异步协议：`Submission { id, op, trace }` 是带可选 `W3cTraceContext` 的请求信封，`Event { id, msg }` 是关联响应信封（L1-L119、L1125-L1136）。Serde 的 tagged enum、`JsonSchema` 与 `TS` 派生同时约束 JSON、Schema 和 TypeScript 外形；大量 `skip_serializing_if`/`default` 明确了兼容性默认值。L60-L80 重新导出 approvals、permissions、request 相关类型，调用方可从本模块建立统一协议面。

常量 L84-L98 是用户、环境、apps、skills、plugins、协作、实时会话和用户消息的 XML/文本边界标记，值固定且无运行时副作用。

请求域（L102-L552）：

- `W3cTraceContext` 的 `traceparent/tracestate` 均为可省略字符串；`McpServerRefreshConfig` 携带任意 JSON 配置；`ConversationStartParams` 有 `prompt` 与可选 `session_id`。实时类型 `RealtimeAudioFrame` 保存 base64 音频字符串、采样率、声道、可选每声道采样数和 item id；`RealtimeTranscriptDelta/Entry/HandoffRequested/InputAudioSpeechStarted/ResponseCancelled` 表示增量、转交和取消信息；`RealtimeEvent` L177-L194 覆盖会话更新、音频/文本增量、输出、取消、item 完成、handoff 与错误；`ConversationAudioParams`/`ConversationTextParams` 分别包裹帧/文本（L124-L205）。
- `Op` L210-L499 是提交操作全集：中断（保留后台终端）、清理后台终端、实时会话 start/audio/text/close、旧 `UserInput`、带 cwd/审批/沙箱/model/effort/summary/service tier/final schema/collaboration/personality 的 `UserTurn`、持久上下文覆盖、exec/patch/elicitation/user-input/permissions/dynamic-tool 回复、历史写入与查询、MCP/配置/skills/custom prompts、compact/memory、线程命名/撤销/回滚/review/shutdown/用户 shell/model 列表。`UserTurn` 的嵌套 `Option<Option<T>>` 语义是“设置/清除/保持”；`ListSkills` 空 cwd 表示会话 cwd，`force_reload` 默认 false；所有请求均可序列化为 snake_case tag。`Op::kind` L501-L552 对每个变体返回稳定静态名称，未知变体在编译期由穷尽匹配暴露。
- `AskForApproval` L558-L590 默认 `OnRequest`；`UnlessTrusted` 只自动放行已知安全只读命令，`OnFailure` 已弃用，`Granular` 按字段控制 sandbox/rules/skill/request_permissions/MCP elicitation，`Never` 不询问。`GranularApprovalConfig::allows_*` L608-L631 逐字段返回布尔值，缺失的新增字段（`skill_approval`、`request_permissions`）默认为 false。
- `NetworkAccess` L636-L645 默认 `Restricted`，`is_enabled` 仅 `Enabled` 为 true。`ReadOnlyAccess` L657-L718 有 `Restricted { include_platform_defaults=true, readable_roots }` 与默认 `FullAccess`；`has_full_disk_read_access`、`include_platform_defaults` 为判定器，`get_readable_roots_with_cwd` 在受限模式复制根、追加 cwd、忽略非法绝对路径并去重，FullAccess 返回空列表表示调用方应授予全盘读取。
- `SandboxPolicy` L722-L1013 四类：`DangerFullAccess`、带 `ReadOnlyAccess`/网络的 `ReadOnly`、带网络的 `ExternalSandbox`、带显式写根/只读根/网络/tmp 排除项的 `WorkspaceWrite`。`FromStr` L817-L839 直接走 `serde_json`，错误为 JSON 错误。`new_read_only_policy`/`new_workspace_write_policy` L841-L868 分别生成全盘读无网、cwd+tmp 可写无网的默认策略；`has_full_disk_read_access`、`has_full_disk_write_access`、`has_full_network_access`、`include_platform_defaults`、`get_readable_roots_with_cwd`、`get_writable_roots_with_cwd` L869-L1013 是纯判定/派生函数。WorkspaceWrite 总是尝试加入 cwd、Unix `/tmp`、可选 `TMPDIR`，非法路径只记录 tracing error；读根与写根最终去重。
- `WritableRoot::is_path_writable` L792-L815 要求 path 在 root 下且不在任一 read-only 子路径；越界或 carve-out 均 false。`default_read_only_subpaths_for_writable_root` L1015-L1056 将存在的 `.git`（含 pointer file 解析后的 gitdir）、`.agents`、`.codex` 加入保护集合并去重；`is_git_pointer_file` L1058-L1060 检查普通 `.git` 文件；`resolve_gitdir_from_file` L1062-L1121 读取 `gitdir:` 指针、按父目录解析绝对路径、检查存在性，任何 I/O/格式/路径失败均记录错误并返回 None。这里存在真实文件系统读取副作用。

事件域（L1125-L1520）：

- `EventMsg` L1138-L1339 是响应变体：错误/警告、实时生命周期与流、模型改路、上下文压缩、回滚、turn 开始/完成、token 统计、agent/user/reasoning 文本及增量、MCP/web/image/exec/terminal/view-image、各类 approval、dynamic tool、elicitation、guardian、弃用、后台、undo、stream error、patch、diff、历史/MCP/custom prompt/skills 列表、plan、abort、shutdown、review、raw/item/hook，以及完整 collab spawn/interaction/wait/close/resume 生命周期。`TurnStarted/TurnComplete` 采用 `task_started/task_complete` 的 wire 名并接受 v2 别名 `turn_started/turn_complete`；事件不含可选顶层字段的约束见注释，保证扩展代码生成稳定。
- hook 枚举 `HookEventName/HookHandlerType/HookExecutionMode/HookScope/HookRunStatus/HookOutputEntryKind` L1343-L1391 分别描述触发点、处理器、同步性、线程/turn 作用域、运行状态和输出类别；`HookRunSummary`、`HookStartedEvent`、`HookCompletedEvent` L1393-L1433 带时间戳、状态、entries。实时事件版本 L1435-L1456 默认 `V1`，started/realtime/closed 分别携带 session/version、payload、可选 reason。
- L1458-L1519 的九个 `From<Collab...> for EventMsg` 实现仅做一一变体包装，无校验和副作用。`AgentStatus` L1522-L1542 默认 `PendingInit`，还可为 Running/Interrupted/Completed(可选最终消息)/Errored/Shutdown/NotFound；这是后续状态判定的离散来源。
- `CodexErrorInfo` L1544-L1569 覆盖上下文/额度/服务过载/HTTP/SSE/沙箱/回滚/其他；`affects_turn_status` L1571-L1591 只让 `ThreadRollbackFailed` 不使回放中的 turn 失败，其余错误为 true。事件载荷 `RawResponseItemEvent`、`ItemStartedEvent`、`ItemCompletedEvent`、各 delta 结构和 `ExitedReviewModeEvent` 位于 L1593-L1725。
- `HasLegacyEvent` L1627-L1701 是兼容投影 trait：`ItemStartedEvent` 仅把 web search/image generation 转为 begin；`ItemCompletedEvent` 委托 `TurnItem::as_legacy_events`；message/reasoning delta 转换成旧 delta 事件；`EventMsg` 对五种新 item/delta 变体转发，其余返回空 Vec。参数 `show_raw_agent_reasoning` 只由底层 item 投影使用。

状态、统计、历史与持久化符号（L1728-L2609）：

- `ErrorEvent::affects_turn_status` L1728-L1742 对缺失 `codex_error_info` 采用 true；`WarningEvent`、`ModelRerouteReason/Event`、`ContextCompactedEvent`、`TurnCompleteEvent`、`TurnStartedEvent` L1744-L1779 是轻量载荷，`TurnStarted` 的 `model_context_window`/`collaboration_mode_kind` 允许兼容缺省。
- `TokenUsage`、`TokenUsageInfo`、`TokenCountEvent`、`RateLimitSnapshot/Window`、`CreditsSnapshot` L1781-L1895 保存累计/最近 token、上下文窗口、限流和额度。`TokenUsageInfo::new_or_append` L1803-L1822 在两输入均 None 时返回 None，否则克隆已有 info 或从零创建，追加 last 并用新窗口覆盖（无新窗口则保留）；`append_last_usage` 做逐字段累计并替换 last；`fill_to_context_window` 将 total 设为窗口、last 设为窗口与旧 total 的非负差；`full_context_window` 构造并填满。`TokenUsage::is_zero/cached_input/non_cached_input/blended_total/tokens_in_context_window/percent_of_context_window_remaining/add_assign` L1899-L1953 处理负数截断、固定 `BASELINE_TOKENS=12000` 和四舍五入百分比；窗口不大于基线直接 0%。`FinalOutput::from` 与 `Display` L1955-L1994 生成带千位分隔、cached/reasoning 括号的摘要字符串。
- `AgentMessageEvent/UserMessageEvent/...Reasoning...` L1996-L2055 是 UI 消息载荷；`UserMessageEvent` 保留 URL images、仅用于 UI 重挂载的 local paths 和 text spans。`McpInvocation`、MCP begin/end、`DynamicToolCallResponseEvent` L2057-L2113 关联 call id、参数、结果、成功标志、duration；`McpToolCallEndEvent::is_success` 仅在 `Ok` 且 `is_error` 未置 true 时成功。web/image 事件 L2115-L2146 只携带调用及结果字段。
- 历史：`ConversationPathResponseEvent`、`ResumedHistory`、`InitialHistory` L2148-L2165。`InitialHistory::forked_from_id/session_cwd/get_rollout_items/get_event_msgs/get_base_instructions/get_dynamic_tools` L2167-L2257 分别从 New/Resumed/Forked 分支读取 session meta、事件、基础指令和动态工具；New 返回 None/空，Resumed/Forked 克隆或筛选，不改变原历史。私有 `session_cwd_from_items` L2259-L2266 找第一条 `SessionMeta`。
- `SessionSource`/`SubAgentSource` L2269-L2296 记录 CLI/VSCode/Exec/MCP/子代理/未知来源；`Display` 与 `get_nickname/get_agent_role` L2298-L2359 为线程派生和 memory consolidation 提供稳定展示名/角色（Morpheus、memory builder）。`SessionMeta`、`SessionMetaLine`、`RolloutItem`、`CompactedItem` L2361-L2446 是 JSONL 持久化模型；`SessionMeta::default` L2388-L2407 以空字符串/默认 ThreadId/VSCode/None 初始化，`CompactedItem -> ResponseItem` 将摘要变成 assistant output text。
- `TurnContextNetworkItem`、`TurnContextItem`、`TruncationPolicy`、`RolloutLine`、`GitInfo` L2448-L2519 持久化一次真实 turn 的 cwd、审批/沙箱、网络域、model、personality、协作、实时、推理、指令和截断策略；`serde(default)` 使缺失网络/turn/trace 等字段可恢复。review 类型 `ReviewDelivery/ReviewTarget/ReviewRequest/ReviewOutputEvent/ReviewFinding/ReviewCodeLocation/ReviewLineRange` L2521-L2609 支持未提交树、分支、commit 或自定义指令，`ReviewOutputEvent::default` 为空 findings、空文本、置信度 0。

工具、MCP、配置与协作载荷（L2610-L3345）：

- `ExecCommandSource/Status` L2610-L2625 默认来源 Agent，状态 Completed/Failed/Declined；`ExecCommandBeginEvent/EndEvent` L2627-L2690 通过 call/process/turn id 配对，记录命令、cwd、解析结果、交互输入、stdout/stderr/聚合输出、exit code、duration、模型格式化输出和状态。`ExecOutputStream`、`ExecCommandOutputDeltaEvent`、`TerminalInteractionEvent` L2701-L2730 对 stdout/stderr 原始 bytes 使用 base64，终端 stdin 保持字符串。
- `BackgroundEventEvent/DeprecationNoticeEvent/UndoStartedEvent/UndoCompletedEvent/ThreadRolledBackEvent/StreamErrorEvent/StreamInfoEvent` L2732-L2780 是后台提示、弃用、撤销、回滚和流错误；`PatchApplyBegin/EndEvent`、`PatchApplyStatus`、`FileChange`、`Chunk`、`TurnDiffEvent` L2782-L2830 描述 patch 的变更 map、stdout/stderr、成功和 unified diff，旧数据的 `turn_id` 默认空字符串。
- `GetHistoryEntryResponseEvent`、`McpListToolsResponseEvent`、`McpStartupUpdate/Status/Complete/Failure`、`McpAuthStatus` L2830-L2903 提供历史按 offset/log_id 查询、工具/资源/模板和认证状态；`McpAuthStatus::Display` 输出人类可读英文。`ListCustomPromptsResponseEvent`、`ListSkillsResponseEvent`、`Product`、`SkillScope`、`SkillMetadata/Interface/Dependencies/ToolDependency/ErrorInfo/SkillsListEntry` L2905-L3008 支持技能发现，新增可选元数据保持旧 JSON 可读。
- `SessionNetworkProxyRuntime` 与 `SessionConfiguredEvent` L3010-L3074 是 configure ack：session/fork id、thread name、model/provider、tier、审批、沙箱、cwd、reasoning、历史计数、初始事件、代理地址、rollout 路径。`ThreadNameUpdatedEvent` L3076-L3084 是持久命名事件。
- `ReviewDecision` L3086-L3115 默认 `Denied`；`to_opaque_string` L3117-L3138 将决策映射为不含 PII 的稳定审计字符串（网络 allow/deny 分支不同）。`TurnAbortedEvent/TurnAbortReason` L3162-L3174 表示 interrupted/replaced/review-ended。
- DAG/协作所需的现成协议在 L3176-L3345：`CollabAgentSpawnBeginEvent` 含 sender、prompt、model、reasoning；`CollabAgentRef/StatusEntry` 含 receiver thread、可选 nickname/role、`AgentStatus`；spawn end 记录新 thread、有效模型/推理和最终状态；interaction begin/end 以 sender/receiver/call_id/prompt 配对；waiting begin/end 批量等待 receiver 并返回 `HashMap<ThreadId, AgentStatus>` 与带角色的 status entries；close/resume begin/end 记录 sender/receiver、角色和关闭/恢复前后状态。所有结构仅是可序列化事实，不自动调度、保活或递归派生。

函数与错误边界汇总：公共方法均为纯构造/判定/转换，除 sandbox 的 `std::fs` 探测和历史数据克隆外不启动线程；Serde 失败沿 `serde_json` 返回。`EventMsg` 的事件枚举穷举保证新增变体必须更新 kind/兼容投影；缺省字段由 `default`/`skip_serializing_if` 决定线协议稳定性。

## 状态、取消、恢复与副作用

本源没有通用 cancellation token、超时计时器或重试循环。取消语义由协议操作和载荷表达：`Op::Interrupt` 只请求当前 turn 中止且保留后台终端，`CleanBackgroundTerminals` 才终止后台进程；实时流有 `RealtimeResponseCancelled`，turn 有 `TurnAbortedEvent`，流故障由 `StreamErrorEvent` 记录可重试/最终断开信息（L210-L499、L172-L205、L2765-L2775）。`ReviewDecision::Abort`、`AgentStatus::Interrupted/Shutdown` 是状态事实，不是本文件内的执行器。

持久化模型由 `RolloutItem::SessionMeta/ResponseItem/Compacted/TurnContext/EventMsg` 和 `TurnContextItem` 提供；`InitialHistory` 的恢复、fork、事件筛选只读克隆，缺失旧字段走默认。`CompactedItem` 代表上下文压缩后的替代历史；`ThreadRollback` 注释明确不回滚磁盘修改，只有客户端负责外部撤销（L350-L484、L2418-L2446）。

可观察外部副作用集中在 `SandboxPolicy::get_writable_roots_with_cwd` 的目录/环境读取、`.git` pointer 读取和 tracing error（L1015-L1121）；协议层本身不写文件、不执行命令、不访问网络。`ExecCommand*`、`PatchApply*`、MCP 结果和 approval 事件只是对外报告，真实执行/审批在其他 crate。没有“子孙全绿才 close”的内建约束；`AgentStatus::Completed` 可携带空最终消息，不能单独证明后代完成。

## 源内测试与行为判据

源内测试位于同文件 `#[cfg(test)] mod tests`，范围 `L3348-L4429`，覆盖以下可核对判据：

- `external_sandbox_reports_full_access_flags`、`read_only_reports_network_access_flags` 验证 ExternalSandbox 的磁盘/网络标志及 ReadOnly 网络布尔值（L3601-L3640）。
- 四个 `granular_approval_config_*` 测试验证 MCP、skill、request_permissions 字段逐字段生效，缺省新增字段反序列化为 false（L3641-L3689）。
- `workspace_write_restricted_read_access_includes_effective_writable_roots`、`restricted_file_system_policy_*`、`legacy_workspace_write_nested_readable_root_stays_writable`、`file_system_policy_rejects_legacy_bridge_for_non_workspace_writes`、`legacy_sandbox_policy_semantics_survive_split_bridge` 验证根权限、carve-out、cwd/tmp/.agents/.codex、legacy bridge 等效性和非法外部写入拒绝（L3690-L4000）。独立判据：对同一 cwd 比较 `has_full_*`、平台默认、read/write root 及 `WritableRoot::is_path_writable`。
- `item_started_event_from_web_search_emits_begin_event`、`...non_web_search...`、`...image_generation...` 与 completed image 测试验证 legacy event 投影的一一映射及非目标空输出（L4001-L4060）。
- `rollback_failed_error_does_not_affect_turn_status` 与 `generic_error_affects_turn_status` 验证 `CodexErrorInfo::affects_turn_status`（L4061-L4080）。
- `conversation_op_serializes_as_unnested_variants` 验证实时 Op 的 snake_case tag、帧字段和往返解码；三个 `user_input_*` 与 `user_input_text_serializes_empty_text_elements` 验证可选 schema 和空数组的兼容序列化（L4081-L4210）。
- `user_message_event_serializes_empty_metadata_vectors`、`turn_aborted_event_deserializes_without_turn_id`、`turn_context_item_*network*` 验证旧 JSON 缺省字段和网络字段出现条件（L4211-L4320）。
- `serialize_event` 验证 Event 信封只有一层 `msg.type` 且默认字段不泄漏；`vec_u8_as_base64_serialization_and_deserialization` 验证原始 bytes 的 base64；两个 MCP startup 测试验证 tagged status 与 aggregate 列表（L4321-L4398）。
- 两个 `token_usage_info_new_or_append_*` 验证提供新上下文窗口会覆盖、不提供则保留（L4399-L4429）。

可独立运行判据：在 `codex-rs/protocol` crate 执行 `cargo test -p codex-protocol protocol::tests`（或按测试名过滤），并额外对每个公开 enum 做 `serde_json` round-trip；检查输出 JSON 的 `type`、省略字段、错误码和 legacy 投影长度。源内未测试实时取消、collab DAG 关闭条件、消息超时/重试；这些需在 zenpi 层补充。

## zenpi Rust 映射

zenpi 已有模块的可复用落点（按实际接口核对）：

- `src/protocol.rs` 已有 JSONL `StdioRequest/Command`、`MailboxRequest`、`CheckpointRequest`、`InputQueueRequest`、`TreeRequest`、`OutputRequest`、`StdioResponse/StdioEvent`、`parse_line/encode_line/command_name` 和严格 `ProtocolError`（当前约 `L1-L1309`）。它对应源的 `Submission/Op`、`Event/EventMsg`、稳定错误码和 tagged payload。应新增 `DagNodeId`、`DagRelation`、`DagMessageKind`、`DagMailboxRequest/Reply`、`DagEvent`，沿现有 `serde(deny_unknown_fields)` 与版本校验；复用 `StdioEvent.sequence`、request/turn id 关联，避免另造无序 stdout 通道。
- `src/session.rs` 已有 append-only journal、`SessionStore`、`LiveSessionRegistry` 与持久 `SessionMailbox`：邮箱消息有 sender/recipient session id、request id、digest、TTL、Queued/Acknowledged/Claimed/Succeeded/Failed/Expired 状态、claim token、幂等冲突检测和锁（约 L3171-L3560）。这是源 `Event` correlation、`SessionMeta/InitialHistory`、`Collab*` 的最佳基础。建议在 mailbox payload 中增加 `dag_node_id`、`parent_id`、`relation`、`generation`、`work_epoch`；每次 claim/finish 都追加 journal 事件，利用现有 digest/TTL/transition 规则实现崩溃恢复。
- `src/core.rs` 的 `Turn { id, parent_id, role, content, metadata }`、`Turn::with_parent/validate`（约 L141-L205）已经表达会话父子关系；`AgentEvent` 有 `TurnAccepted/AssistantMessage/ToolCall/ToolResult/Provider/Error/Warning`，`AgentPhase::{Idle,Running,Closed}`，且 `Agent::register_live_owner/heartbeat_live_owner/claim_live_mailbox/finish_live_mailbox(_with_reply)`（约 L819-L910）能把工作和 mailbox 绑定。建议新增 `DagNodeState { node_id, parent, children, status, green_epoch }` 与 `DagCoordinator`，在 `AgentEvent::Handoff/ToolResult` 处发出 DAG 事件。
- `src/runtime.rs` 的 `CancellationToken` 是最小协作取消原语；`BackgroundRunner` 使用有界 command/event `sync_channel`，提供 `try_cancel/next_event/recv_timeout/try_next_event/join/close_with_grace`，事件序列为 Accepted/CancelRequested/Completed/Closed（约 L40-L430）。可直接把每个 DAG worker 作为一个 `BackgroundRunner` job：mailbox receive 是 command，`DagEvent` 是 event；TTL 或 `recv_timeout` 驱动保活检查，关闭时先 cancel、短 grace、再发 Closed。
- `src/headless.rs` 已有有界 `AsyncEventBuffer`、request mailbox、事件 sequence/replay journal、重连恢复和显式 close（约 L39-L70、L806-L1050）。建议把 DAG 事件接入同一 replay buffer；每个事件同时写 `StdioEvent { sequence, request_id, turn_id, event }` 和 session journal，防止 stdout 断线丢失。沿现有 bounded mailbox 的丢失告警/优先级规则保护 `NodeGreen`、`NodeFailed`、`CloseDeferred` 等生命周期事件。
- `src/tool_runtime.rs` 的 `execute_tool_batch` 已实现 bounded worker、取消轮询、审批后再检查取消、失败相关性和 owner join（约 L157-L347），可作为 DAG 节点内部并发工具调用的底层执行器；不要让 DAG coordinator 直接执行工具。
- `src/approval.rs` 的 `ApprovalCoordinator` 有取消 epoch、`cancel_all/emergency_cancel`、持久 remembered decisions 和 fail-closed 错误（约 L131-L470）；将 `DagMessageKind::ApprovalRequest/Response` 映射到这里，确保 worker 被 cancel 后挂起 approval 不会误放行。`src/providers/**` 只负责 provider 连接/流，不应承载 DAG 拓扑或邮箱一致性；provider stream 的重试/断开应转换为 `DagEvent::WorkerRetrying/WorkerFailed`。
- 现有 `src/protocol.rs` 已有 `MailboxRequest::{Send/List/Claim/Complete/Fail...}`（以实际定义为准），因此最小增量可复用其 request parser，仅在 payload 中加入拓扑关系；若需要类型安全，再添加 `DagMailboxRequest` 而保留旧 mailbox 兼容分支。

DAG 需求的明确协议设计：

1. **通信原语**：worker 与 parent、grandparent、直接 sibling、直接 child 均通过同一持久 `SessionMailbox` 发送 `DagMessage`；`DagMessage` 必须含 `message_id/digest`, `sender`, `recipient`, `node_id`, `relation`（Parent/Grandparent/Sibling/Child）, `kind`（Work/Progress/Green/Failure/Cancel/KeepAlive/CloseQuery）, `payload`, `generation`, `ttl_ms`。短期流式通知走 `BackgroundRunner` event channel，断线后由 mailbox journal 重放；不要用裸 `mpsc` 作为唯一事实源。
2. **会话父子关系**：创建节点时在 `SessionMeta` 或既有 session journal 写 `parent_session_id`, `parent_node_id`, `depth`, `children`；可复用 `Turn.parent_id` 表示 turn 级父子，但 DAG 节点关系必须独立字段，避免一个 turn 同时有多个 child 时丢拓扑。Sibling 地址由 coordinator 根据同 parent 的 child 集合解析；grandparent 是 parent 的 parent，不允许任意跨树寻址。
3. **最小保活机制**：每个 running node 持有 `CancellationToken`、`owner_epoch`、`lease_expires_at` 和 `last_progress_seq`；每 `heartbeat_interval` 向 parent 发 `KeepAlive`，parent 用 `LiveSessionRegistry::heartbeat` 更新租约。`recv_timeout` 到期先发一次 `CloseQuery`；租约过期将 node 标为 `Lost`，其未完成 mailbox 可按 digest 幂等重派。所有状态更新 journal 化，以便 restart 后依据最后 lease/status 恢复。
4. **close/派生判定**：节点只能在自身状态 `Green` 且其直接 child、递归 grandchild 的聚合状态均为 `Green`（无 `Running/Queued/Failed/Lost`、无未完成 mailbox、所有子树 `green_epoch >= current work_epoch`）时发送 `CloseBegin` 并最终 `CloseEnd`。否则保持 `Running`/`WaitingChildren`，发送 `CloseDeferred`，并由 coordinator 派生一个新 worker（记录 `derived_from`, `reason`, `generation+1`）处理新工作；关闭请求必须幂等，只有同一 claim token 的 owner 可完成。父节点在收到 child green/failure 后重新计算聚合，不把单个 `AgentStatus::Completed` 当作全绿。
5. **可执行验证**：新增单元测试覆盖四种邻接寻址、重复 request digest、TTL 过期重派、父节点等待孙节点、child failure 导致 `CloseDeferred`、全树 green 才 `CloseEnd`、cancel 后 approval 不放行、重启从 journal 恢复 lease/claim。端到端测试经 `run_headless` 注入 mailbox JSONL，检查 `StdioEvent.sequence` 连续且可重放。

差异清单：源协议拥有丰富事件和 collab payload，但没有 DAG 拓扑字段、递归绿状态、lease/heartbeat、超时重派、close gate 或 worker 派生执行器；zenpi 需要在 `src/protocol.rs` 增加 typed DAG envelopes，在 `src/session.rs` 增加拓扑/lease 持久投影，在 `src/core.rs` 增加 `DagCoordinator` 与 close gate，在 `src/runtime.rs` 复用取消/有界队列，在 `src/headless.rs` 接入 replay 与事件优先级，在 `src/approval.rs` 复用 fail-closed cancel，在 `src/providers/**` 仅报告 provider 失败/重试。所有新增函数都应先写 journal intent，再做外部执行，再写 completion，保持恢复可判定。

## 未决问题

无法从源文件确认：Codex 实际 mailbox/协作调度器如何选择模型与 worker 数量；`AgentStatus::Completed` 是否在其他 crate 被视为绿；子代理 session 的实际持久化路径与租约时钟；zenpi 是否允许跨 workspace 的 grandparent/sibling 通信；provider 重试次数和退避参数。这些点需结合调用方实现或产品约束补充，不能由本源 `protocol.rs` 单独推出。

逐函数输入/输出与并发要点：`Op::kind`、所有 `is_*`/`has_*`/`include_*`、`get_*`、`to_opaque_string`、`command` 兼容转换函数均以借用值输入并返回静态字符串、布尔值、克隆集合或 `Option`；边界行为已由上文逐项注明（空集合、负数截断、非法路径、缺省字段）。`FromStr`、Serde 解码和 git pointer 解析是唯一显式 `Result` 错误边界，错误不改变协议对象；legacy 投影返回新 `Vec<EventMsg>`，不会修改事件。所有结构派生的 `Clone` 使跨线程发送可以复制值，但本文件没有 `Arc/Mutex/channel/spawn`，因此并发顺序、背压、取消和超时完全由调用方借助 `id`、`turn_id`、call id 与外部队列实现。`Event.id` 只提供关联键，不保证顺序或 exactly-once；`Submission.trace` 仅传播 W3C carrier，不参与重试判定。

导出符号逐项索引（便于核对完整覆盖）：常量 `USER_INSTRUCTIONS_*`、`ENVIRONMENT_CONTEXT_*`、`APPS_INSTRUCTIONS_*`、`SKILLS_INSTRUCTIONS_*`、`PLUGINS_INSTRUCTIONS_*`、`COLLABORATION_MODE_*`、`REALTIME_CONVERSATION_*`、`USER_MESSAGE_BEGIN`（L84-L98）；结构 `Submission/W3cTraceContext/McpServerRefreshConfig/ConversationStartParams/RealtimeAudioFrame/RealtimeTranscriptDelta/RealtimeTranscriptEntry/RealtimeHandoffRequested/RealtimeInputAudioSpeechStarted/RealtimeResponseCancelled/ConversationAudioParams/ConversationTextParams`（L102-L205）；枚举 `RealtimeEvent/Op/AskForApproval/NetworkAccess/ReadOnlyAccess/SandboxPolicy/EventMsg`（L177-L205、L210-L552、L558-L590、L636-L718、L722-L789、L1138-L1339）。

其余协议结构逐项为：`GranularApprovalConfig`（L592-L631）、`WritableRoot`（L792-L815）、`Event`（L1125-L1136）、`HookOutputEntry/HookRunSummary/HookStartedEvent/HookCompletedEvent`（L1393-L1433）、`RealtimeConversationStartedEvent/RealtimeConversationRealtimeEvent/RealtimeConversationClosedEvent`（L1442-L1456）、`AgentStatus/CodexErrorInfo`（L1522-L1591）、`RawResponseItemEvent/ItemStartedEvent/ItemCompletedEvent/AgentMessageContentDeltaEvent/PlanDeltaEvent/ReasoningContentDeltaEvent/ReasoningRawContentDeltaEvent/ExitedReviewModeEvent`（L1593-L1725）、错误/模型/token 结构（L1728-L1995）、消息/MCP/web/image 结构（L1996-L2146）、历史/来源/session/rollout 结构（L2148-L2519）、review 结构（L2521-L2609）、exec/patch/undo/stream 结构（L2610-L2830）、MCP/skills/session 配置结构（L2830-L3084）、`ReviewDecision/FileChange/Chunk/TurnAbortedEvent`（L3086-L3174）和全部 `Collab*` 结构（L3176-L3345）。私有辅助函数 `default_include_platform_defaults`、`default_read_only_subpaths_for_writable_root`、`is_git_pointer_file`、`resolve_gitdir_from_file`、`session_cwd_from_items` 的输入输出和失败分支分别见 L647、L1015-L1121、L2259-L2266。

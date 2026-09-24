# codex-rs/protocol/src 目录级学习汇总

范围：本汇总对应 `AC-034`—`AC-040` 的 7 个源文件；逐文件笔记位于 `Docs/learn/agent_comms/files/codex/codex-rs/protocol/src/`，并已用源文件 hash 复核。

## 目录职责

该目录是 Codex 的协议数据契约层，负责把请求、事件、回合 item、审批、用户输入、计划和历史编码成稳定的 Rust/JSON/JSON Schema/TypeScript 形状。核心设计是 serde tagged enum、兼容性的 `#[serde(default)]`/`skip_serializing_if`、稳定的 `id`/`turn_id`/`call_id` 关联键，以及旧事件投影；它描述事实和边界，不负责线程、邮箱、调度、取消计时器、重试或副作用。`protocol.rs` 还集中定义 `Submission`/`Event`、`Op`/`EventMsg`、sandbox/approval、turn/history、MCP、工具和 `Collab*` 生命周期载荷。尤其是 `CollabAgentSpawn*`、interaction、waiting、close、resume 事件已经能表达协作事实，但不会自动做 DAG 拓扑聚合、保活或递归派生。

## 模块清单

- `approvals.rs`：定义权限升级和人工决策 DTO；关键导出为 `Permissions`、`EscalationPermissions`、`ExecPolicyAmendment`、`ExecApprovalRequestEvent`、`GuardianAssessmentEvent`、`ElicitationRequest/Event`、`ElicitationAction`、`ApplyPatchApprovalRequestEvent`，只做序列化与默认决定列表计算。
- `items.rs`：定义带 `type` 标签的回合 item 及 legacy 适配器；关键导出为 `TurnItem`、`UserMessageItem`、`AgentMessageItem`、`PlanItem`、`ReasoningItem`、`WebSearchItem`、`ImageGenerationItem`、`ContextCompactionItem`，以及 `id()`、`as_legacy_events()` 等纯转换方法。
- `lib.rs`：protocol crate 的根命名空间装配器；公开重导出/模块包括 `approvals`、`items`、`message_history`、`plan_tool`、`protocol`、`request_user_input` 等，并私有加载 `thread_id` 后公开 `ThreadId`，自身不承载运行时逻辑。
- `message_history.rs`：定义最小历史记录 `HistoryEntry { conversation_id, ts, text }`，为历史消息进入传输或邮箱提供可序列化值对象。
- `plan_tool.rs`：定义 TODO/计划工具参数；关键导出为 `StepStatus::{Pending, InProgress, Completed}`、`PlanItemArg`、`UpdatePlanArgs`，用 `snake_case` 和 `deny_unknown_fields` 约束输入，但不实现状态转移或计划执行。
- `protocol.rs`：目录的主协议文件；关键导出包括 `Submission`、`Event`、`Op`、`EventMsg`、`AgentStatus`、`CodexErrorInfo`、`SandboxPolicy`、`AskForApproval`、`TurnItem` 相关事件、历史/rollout 模型以及全部 `Collab*` spawn/interaction/wait/close/resume 载荷和 legacy 投影。
- `request_user_input.rs`：定义用户问答协议；关键导出为 `RequestUserInputQuestionOption`、`RequestUserInputQuestion`、`RequestUserInputArgs`、`RequestUserInputAnswer`、`RequestUserInputResponse`、`RequestUserInputEvent`，保留 `isOther`、`isSecret`、缺失 `turn_id` 的兼容默认。

## 运行时数据流与控制流

典型链路是：调用方构造 `Submission { id, op, trace }`，`Op` 表达 turn、工具、审批、用户输入、历史或协作操作；执行层产生 `Event { id, msg: EventMsg }`，`EventMsg` 携带 turn 开始/完成、文本增量、工具/MCP、审批、错误、`AgentStatus` 或 `Collab*` 事实。`TurnItem` 将用户/代理消息、计划、推理、搜索和图像结果作为结构化流，再按需要通过 `as_legacy_events()` 投影为旧事件。`SessionMeta`、`RolloutItem`、`InitialHistory` 和 `CompactedItem` 构成 JSONL 历史恢复/压缩链；`HistoryEntry` 是更轻的历史载荷。审批 DTO 由 host/coordinator 决策，`RequestUserInputEvent` 发起等待，响应以问题 id 到答案的 `HashMap` 回来；`plan_tool` 只提供计划快照。

所有数据对象大多是 `Clone` 的纯值，协议层不保证顺序、exactly-once、背压或身份真实性；`Event.id`/`call_id`/`turn_id` 只提供关联键。Serde 失败发生在非法 JSON、未知 enum、缺失必填字段、类型不匹配或 `deny_unknown_fields`；兼容字段缺失时使用空字符串、`None`、`false` 或空集合。源代码内的副作用主要是 `SandboxPolicy` 为 `.git` pointer、cwd、`.agents`、`.codex` 等读取文件系统以推导路径规则；命令、patch、网络、MCP 和图像结果只是事件报告。

## 错误、取消与恢复语义

`CodexErrorInfo`/`ErrorEvent::affects_turn_status` 区分会影响 turn 的错误与 `ThreadRollbackFailed` 这类例外；`AgentStatus` 可为 `PendingInit`、`Running`、`Interrupted`、`Completed`、`Errored`、`Shutdown`、`NotFound`。取消由 `Op::Interrupt`、实时 `ResponseCancelled`、`TurnAbortedEvent`、`StreamErrorEvent`、`ReviewDecision::Abort` 等表达，但这些类型不会中断任务、撤销磁盘修改、清理后台进程或传播到子代理；`CleanBackgroundTerminals` 才是另一项清理操作。恢复依赖持久化的 `RolloutItem`/session history 和兼容默认，fork/resume 是只读筛选/克隆；重复提交、超时、重试、claim、lease 和幂等均交给上层。

对 zenpi 的 DAG 来说，`Completed`、`StepStatus::Completed` 或单个 `AgentStatus::Completed` 都不能直接等价于可关闭。取消、失败、过期、`UnknownOutcome`、未回答 approval 和未结算 mailbox 都必须阻止标绿或 close；取消只停止合作式计算，不能回滚已经发送的消息或外部工具副作用。审批应沿用先持久化 accepted decision、再唤醒 worker 的 fail-closed 语义。

## zenpi Rust 映射建议

通信原语统一复用现有 `src/protocol.rs` 的 `MailboxRequest::{Send, Receive, Acknowledge, Claim, Complete}`、`Command::Mailbox`、`StdioEvent { sequence, request_id, turn_id, event }` 和严格校验；在 payload 上增加类型安全的 `DagMessage`/`DagEvent`，至少携带 `message_id`、`digest`、`sender_session_id`、`recipient_session_id`、`node_id`、`relation`、`kind`、`generation`、`ttl_ms` 和 payload。`relation` 只允许 `Parent|Grandparent|Sibling|Child`：worker 发给 parent、grandparent、直接 sibling、直接 child 都走同一持久 mailbox；短时进度可走 `runtime` event channel，但唯一事实源必须是 session journal/mailbox，不能只靠裸 `mpsc` 或 stdout。由 registry 注入 sender 身份并校验 workspace/邻接关系，客户端不可伪造路由。

会话父子关系落在 `src/session.rs` 的可恢复拓扑投影，而不只放在 turn 文本。建议持久化 `DagNodeRecord { node_id, worker_session_id, parent_node_id, child_ids, state, generation, lease_id, revision }` 及 `parent_session_id`/`depth`/`role`；parent 由一条边获得，grandparent 沿 parent 上溯一跳，直接 sibling 是同 `parent_id` 的节点，直接 child 由反向 children 索引获得，全部 child/grandchild 用递归闭包查询。`src/core.rs::Turn { parent_id }`、`Turn::with_parent/validate` 继续表示 turn 级因果链，但不能代替 DAG 图。

四个既有落点的职责应保持清晰：

1. `src/protocol.rs` 增加 `DagNodeId`、`DagRelation`、`DagMessageKind`、`DagMailboxRequest/Reply`、`DagEvent`、计划/用户输入 DTO，复用 ID/文本/TTL/版本校验和旧字段默认。
2. `src/session.rs` 负责 append-only journal、拓扑索引、`SessionMailbox` 的 `Queued/Acknowledged/Claimed/Succeeded/Failed/Expired`、claim token、digest 幂等和恢复；每次 spawn、状态、heartbeat、claim、complete、close-deferred 都先记录事件。
3. `src/core.rs` 增加 `DagCoordinator`、`DagNodeState::{Pending, Running, Waiting, Green, Failed, Cancelled, Closed}` 和 `close_gate/can_close`。节点自身必须 `Green`，且递归全部后代（包含直接 child 与 grandchild）为 `Green`、当前 `work_epoch` 已收敛、无活动 lease/claimed mailbox/未知 operation，才允许 `Closing -> Closed`；否则写 `CloseDeferred`，保持活跃并向相关邻接节点发送状态。
4. `src/runtime.rs` 复用 `CancellationToken`、`BackgroundRunner::try_submit`、有界 command/event channel、`RuntimeEvent::{Accepted, Started, Queued, CancelRequested, Completed, Rejected, Closed}`。每个 node worker 持有 `owner_epoch`、`lease_id`、`generation`、`last_heartbeat` 和 token；heartbeat 通过 `LiveSessionRegistry` 续租。发现新工作时先 append `derive_intent`，再 `try_submit` 新 worker；`QueueFull/Closed` 不能静默丢弃，稍后重试仍保持 lease。取消停止计算但不把结果伪装成 Green。

`src/headless.rs` 是 JSONL I/O、事件缓冲、request mailbox、sequence/replay 和重连适配层：将 `NodeGreen`、`CloseDeferred`、`Heartbeat`、`WorkerSpawned`、mailbox receipt 投影到现有 `StdioEvent`/replay ledger，并在断线后只重放已持久化事实，不能从最后一行 stdout 推断 close。`src/tool_runtime.rs` 继续负责有界工具批处理和副作用边界；`src/approval.rs` 继续承载人工审批、`policy_digest`/`lease_id` 绑定和取消 fail-closed；`src/providers/**` 只报告流、重试和失败，不维护 DAG 拓扑或 close 判定。

最小机制可压缩为四件事：`DagNodeRecord`、定向持久 `MailboxRequest`、可重放 `PlanEvent`/journal、`LeaseHeartbeat + DeriveWorker`。`evaluate_close(node)` 的不变量等价于：`self.green && descendants(node).all(|d| d.green)`；否则 `heartbeat(node.lease_id)`，若有新工作则以新的 `generation`/`lease_id` 派生 child。所有 send/claim/complete/close 都带幂等 operation/message id 和 revision，旧 generation 不得覆盖新状态。

## 未决问题

源协议没有定义 DAG 节点 ID、失败/取消是否终态、`AgentStatus::Completed` 是否在其他 crate 被视为 green、lease 时长与 heartbeat 周期、派生上限、provider 重试/退避和 mailbox 超时。zenpi 还需确定跨 workspace 的 grandparent/sibling 是否允许、`RequestUserInput` 的答案数量/未知 key/secret 处理、approval amendment 的合并规则、未知副作用如何进入人工恢复，以及 session journal 与 mailbox sidecar 的一致性策略。实现前应补充四类邻接寻址、父等待孙节点、child failure、TTL 重派、重复 complete、祖先取消、重启恢复、队列满保活和“全树 green 才 close”的单元与端到端测试。

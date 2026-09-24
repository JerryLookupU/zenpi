# AC-021 — codex/codex-rs/core/src/agent/guards_tests.rs

- source_id/item_id：codex/codex-rs/core/src/agent/guards_tests.rs / AC-021
- source_path：codex/codex-rs/core/src/agent/guards_tests.rs
- source_hash：7bdfa8b238e430526c4812e5f55a7bda155452c5ee2aa111e1b51cff3b5d41a9
- source_bytes：8395
- source_lines：243
- coverage：已按文件顺序读取完整内容，字节范围 1-8395、行范围 L1-L243；未跳过注释、导入、类型引用或测试函数。

## 完整行为复盘

该文件没有生产实现，也没有 public 导出；use super::* 引入被测模块的 format_agent_nickname、session_depth、next_thread_spawn_depth、exceeds_thread_spawn_depth_limit、Guards、ThreadId、SessionSource、SubAgentSource、CodexErr 等符号。文件没有 public 导出，包含 12 个私有 #[test] 函数；它们以断言规定 guard 的默认值、RAII 槽位和昵称池行为。

- format_agent_nickname_adds_ordinals_after_reset（L5-L12）：直接验证输入昵称和从零开始的序号映射。0 返回原名，1/2/10/20 分别产生 the 2nd/3rd/11th/21st；输出是字符串，覆盖序数后缀的 2、3、11、21 边界，未覆盖负数（类型应为无符号）或更大的英语后缀。
- session_depth_defaults_to_zero_for_root_sources（L14-L17）：SessionSource::Cli 是根来源，session_depth 输出 0。这是根 worker 的默认深度判据，没有错误返回。
- thread_spawn_depth_increments_and_enforces_limit（L19-L30）：构造 SubAgentSource::ThreadSpawn，父 ThreadId::new()、depth: 1，昵称和角色均为 None；next_thread_spawn_depth 输出 2，再以最大深度 1 调用 exceeds_thread_spawn_depth_limit 得 true。说明限制检查作用于派生后的深度，而不是当前深度。
- non_thread_spawn_subagents_default_to_depth_zero（L32-L38）：SubAgentSource::Review 不携带 thread-spawn 深度，session_depth 输出 0，next_thread_spawn_depth 从该默认值推导 1；上限为 1 时不超限。非线程型子代理不会继承未知深度。
- reservation_drop_releases_slot（L40-L48）：Arc<Guards>::default() 在上限 Some(1) 下成功预留槽位；直接 drop(reservation) 后再次预留成功。RAII 未提交预留会释放并发名额，且无外部副作用。
- commit_holds_slot_until_release（L50-L71）：预留后以新 ThreadId commit。第二次预留返回 CodexErr::AgentLimitReached { max_threads: 1 }；release_spawned_thread(thread_id) 后再次预留成功。提交把临时槽位转为活动线程占用，释放是显式的；错误路径只返回 typed error，不会静默超卖。
- release_ignores_unknown_thread_id（L73-L96）：提交真实 ID 后释放一个未知 ID，不改变占用；再次预留仍得到 AgentLimitReached { max_threads: 1 }。随后释放真实 ID 才能复用槽位。释放操作必须按注册 ID 匹配，未知 ID 是幂等无动作。
- release_is_idempotent_for_registered_threads（L98-L127）：释放 first_id 后槽位可被第二线程复用；再次释放已移除的 first_id 不会错误释放 second_id 的槽位，第二线程仍使上限报错；释放 second_id 后才可再次预留。判据是按 ID 集合删除，而不是简单计数减一，避免重复释放造成并发额度失真。
- failed_spawn_keeps_nickname_marked_used（L129-L144）：未 commit 的 reservation 先保留候选 alpha 后 drop；下一次候选 alpha,beta 必须选择 beta。槽位随 drop 释放，但昵称的“已用”标记跨失败派生保留，避免失败重试立即复用同名。
- agent_nickname_resets_used_pool_when_exhausted（L146-L169）：第一活动线程占用 alpha；第二次只有 alpha 时触发昵称池重置并返回 alpha the 2nd，active_agents.nickname_reset_count == 1。重置是耗尽候选后的默认策略，序号持续递增而不是回到无后缀；测试在两个 reservation 均存活时观察共享状态。
- released_nickname_stays_used_until_pool_reset（L171-L207）：第一线程用 alpha 后释放，第二次候选 alpha,beta 仍选 beta，证明线程释放不等于昵称释放；第二线程释放后第三次触发池重置，结果必须是 alpha the 2nd 或 beta the 2nd，且重置计数为 1。昵称历史和活动线程集合是两个独立状态。
- repeated_resets_advance_the_ordinal_suffix（L209-L243）：连续三轮同名 Plato，分别得到 Plato、Plato the 2nd、Plato the 3rd；每轮提交后释放，最终 nickname_reset_count == 2。重复耗尽不会覆盖已用历史，序号单调递增。

并发语义由 Arc<Guards> 与内部共享状态体现；测试在 L164-L168、L202-L206、L238-L242 通过 active_agents.lock() 读取计数，并在锁中毒时用 PoisonError::into_inner 恢复。源文件没有显式并行线程，但要求 guard 的槽位、活动 ID、昵称集合在多个 owner 间可安全共享；预留对象的 drop 是失败路径的自动清理边界。

## 状态、取消、恢复与副作用

测试覆盖“预留中→提交→释放”生命周期，以及提交失败时槽位回收、昵称历史保留、未知/重复释放的幂等性。没有取消 token、超时、重试调度、磁盘持久化、网络调用、provider/tool 外部副作用或事务回滚；所有数据均为进程内 Guards 状态。唯一可视为恢复的是锁中毒后的 into_inner 读取，以及对失败 spawn 的槽位 RAII 回收。昵称池重置不是清空历史，而是增加 ordinal 基数；活动线程释放也不会清除昵称占用。源内没有验证 panic、线程崩溃、进程重启后恢复或时钟行为，这些必须由上层实现和独立测试补足。

## 源内测试与行为判据

源内确实包含测试，即上述 12 个 #[test]，没有额外 fixture 或异步测试。可独立验证的判据是：运行 cargo test 后，所有断言通过；重点断言包括根深度为 0、派生深度递增、超限返回 CodexErr::AgentLimitReached、未知/重复 ID 释放不改变真实占用、未提交 reservation 的槽位可重用但昵称不可立即重用、昵称池重置次数与 the Nth 后缀同步递增。若将这些判据移植到 zenpi，应另测并发抢占、取消、TTL 过期、进程重启和 DAG 闭合条件。

## zenpi Rust 映射

对照现有代码，最接近的通信原语已经存在：

- src/protocol.rs 的 MailboxRequest::{Send,Receive,Acknowledge,Claim,Complete}（L79-L113）和 Command::Mailbox（L214-L263）提供带 recipient_session_id、message_id、ttl_ms、分页 cursor 的消息/邮箱协议；validate_mailbox（L544-L582）限制 ID、正文、TTL 和分页边界。它适合作为 parent、grandparent、直接 sibling、直接 child 之间的持久消息入口。
- src/session.rs 的 MailboxMessage/MailboxStatus（L3170-L3255）把消息建模为 Queued→Acknowledged/Claimed→Succeeded/Failed，并以 digest、claim token、过期时间保证幂等和 owner epoch 约束；LiveSessionRegistry 的 register/heartbeat/active（L3274-L3355）可作为 worker 保活租约。claim_next 明确只 claim、不启动 scheduler（L3361-L3410），因此“派生新 worker”应由上层编排器显式执行。
- src/headless.rs 的 AsyncEventBuffer（L861-L1021）是有容量和字节上限的事件邮箱，优先保留 admission/tool 生命周期事件；PendingSteer/enqueue_pending_steer（L1064-L1101）提供 bounded steer 队列和 backpressure。它适合承载 DAG 节点状态、心跳、派生、关闭事件，但必须为终态事件保留优先级。
- src/runtime.rs 的 BackgroundRunner、JobId、CancellationToken 和 RuntimeEvent::{Accepted,Started,Queued,CancelRequested,Completed,Closed}（约 L40-L203、L236-L365）可复用为每个 worker 的执行壳。取消是合作式的；Completed 必须先于 runner Closed，且超时后不能假定任意已发生的外部副作用被回滚。
- src/session_tree.rs 已有 TreeEntry { id, parent_id, branch_id, depth }（L48-L60）、ancestry（L245-L268）、children 计数和深度/字节预算（L303-L362）。它能持久化会话 turn 的父链和分支，但当前模型是 transcript entry tree，不等价于 worker DAG：没有 sibling 消息路由、节点完成聚合或全子孙绿色判定。
- src/core.rs 的 Agent 持有 SessionStore、events、worker_budget 与 live_owners（L402-L442），并提供 live mailbox 的 heartbeat、claim、finish、reply（L836-L910）；with_live_tool_events 通过恢复 guard 清理 request-scoped sink（L505-L520）。这里适合放 DAG admission 与节点状态投影。AgentEvent 的 Tool/Provider/Warning/Error 变体（L281-L325）可作为节点进度和失败通知。
- src/tool_runtime.rs 的 execute_tool_batch（L157-L180、L252-L330）使用共享取消位、5ms 轮询、scoped worker 并在 owner unwind 前先 stop；它说明 DAG 节点内的 tool 工作必须可取消且 join，不应 detach。src/approval.rs 的 ApprovalCoordinator 是 worker-host 的受控 rendezvous；审批等待取消、首次响应获胜并可持久化，适合把需要人工批准的节点标成非绿色。
- src/providers/**（例如 src/providers/mod.rs L1-L10、L13-L49，registry.rs L74-L99）只负责 provider protocol、连接策略、模型能力和目录；provider 事件经 Backend/ProviderEvent 进入 core。不要把 parent/child 路由或 close 判定塞进 provider adapter。

建议的可执行落点与最小机制：

1. 在 src/core.rs 或独立 src/worker_dag.rs 增加 WorkerNodeId、WorkerNode { id, session_id, parent_id, state, generation, heartbeat_deadline }、WorkerState::{SpawnReserved,Running,Waiting,Green,Blocked,Closed} 和 WorkerDag。parent_id 直接支持 parent；沿 parent 链解析 grandparent；同一 parent 的 children 集合派生 direct siblings；children 集合即 direct children，递归索引得到 grandchildren。
2. 在 src/session.rs 复用 SessionStore JSONL 事件和 mailbox sidecar，新增 worker_dag_node、worker_state、worker_heartbeat、worker_spawn、worker_close 事件投影；用 MailboxMessage 保存跨会话消息，消息 envelope 增加 relation/in_reply_to/dag_generation（需协议版本化）。不要用仅内存的 Vec 作为恢复真相。
3. 在 src/protocol.rs 扩展 MailboxRequest 的可选 DAG 元数据或新增 DagRequest::{SendToParent,SendToGrandparent,SendToSibling,SendToChild,Status,Close}，继续复用现有 ID/TTL/限长校验；保持 MailboxRequest::Send 可作为底层原语，关系校验由 session graph owner 执行。
4. 在 src/runtime.rs 增加类似 guards_tests 的 DagSpawnGuard：reserve_spawn_slot(max_active, depth_limit) 返回 RAII reservation；commit(node_id) 才进入活动集合；Drop 只释放未提交槽位；按 WorkerNodeId 删除实现未知/重复释放幂等；深度限制在 parent.depth + 1 上检查。派生新工作只允许通过该 guard + BackgroundRunner::try_submit，失败时槽位回收而消息/昵称类历史按事件保留。
5. 保活最小闭环是：worker 每次执行/等待周期调用 Agent::heartbeat_live_owner；超 TTL 由 LiveSessionRegistry::active 视为离线，父节点收到失联事件并保持 Waiting/Blocked；新工作进入 mailbox 后由编排器 claim，再显式创建 BackgroundRunner job。没有新任务时不应自发 fork。
6. 关闭判定函数应是可验证的纯函数：仅当节点自身状态为 Green 且其全部 direct children 递归到 Green（无 Running/Waiting/Blocked/Expired/Failed 后代）才返回 Closable；否则保持 heartbeat、发送缺口消息并可为新任务派生 child。祖先只能在所有后代满足条件后逐级变绿，避免“自身完成但孙节点仍活跃”时误关。
7. 在 src/headless.rs 映射 DAG 命令、事件和 backpressure；在 src/core.rs 维护 WorkerDag 与 AgentEvent 投影；在 src/session.rs 做 durable recovery；在 src/runtime.rs 承担取消、join、spawn 上限；在 src/tool_runtime.rs 复用工具取消/结果聚合；在 src/approval.rs 将待审批节点阻止为非绿色；src/providers/** 只消费节点提供的 request scope，不管理 DAG。
8. 可验证差异清单：现有 SessionTree 只有 turn parent/branch，缺 worker node state、全子孙聚合和关系路由；现有 mailbox 有 TTL/claim/complete，缺关系授权和 DAG generation；现有 runtime 有单 runner 的 queue/cancel/close，缺按 DAG 深度与活动节点计数的 RAII guard；现有 live owner 有 heartbeat，缺心跳失联后的节点重派生策略；现有 approval/tool/provider 流程有取消与持久化边界，但没有“全绿才 close”判定。应为这些新增纯函数、事件恢复、未知/重复释放、TTL/重启及多级 parent/sibling/child 通信分别补 Rust 单测和端到端 headless 测试。

## 未决问题

1. 源文件未给出 Guards、ActiveAgents、昵称池或深度限制的具体实现，因此无法确认线程安全原语是 Mutex、RwLock 还是原子计数，也无法确认上限为 None 时的精确语义。
2. 源文件只证明线程 spawn 深度和昵称分配，不证明真实 worker 的消息拓扑、超时策略、全子孙 close 规则或持久化格式；这些需由 zenpi DAG 设计另行确定。
3. SessionTree 的 transcript parent 是否应与 worker DAG parent 共享 ID、是否允许一个 session 承载多个 worker 节点，无法从本源确认。


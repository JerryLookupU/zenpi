# AC-020 — codex/codex-rs/core/src/agent/guards.rs

- source_id/item_id: `codex/codex-rs/core/src/agent/guards.rs` / `AC-020`
- source_path: `codex/codex-rs/core/src/agent/guards.rs`
- source_hash: `5c934424258cd8a31fb1b7ebc11360229f91e1c4a0620509fa30d9ee401a665a`
- source_bytes: `7520`
- source_lines: `230`
- coverage: 已从头到尾读取字节 `0-7519`、行 `L1-L230`，包含注释、导入、私有类型、`pub(crate)` 导出、`Drop` 与测试模块声明。

## 完整行为复盘

文件先导入 `CodexErr/Result`、`ThreadId`、`SessionSource/SubAgentSource`、随机选择器，以及 `HashMap/HashSet`、`Arc/Mutex/AtomicUsize/Ordering`（`L1-L12`）。整体目标是给同一用户 session 内共享的多 agent 能力加上总线程数与昵称限制；`Guards` 由 `Arc` 共享，内部 `active_agents: Mutex<ActiveAgents>` 保存身份集合，`total_count: AtomicUsize` 保存已预留或已提交的 agent 数（`L14-L24`）。

`ActiveAgents` 是 `Default` 私有状态（`L26-L32`）：`threads_set` 记录已 commit 的 `ThreadId`，`thread_agent_nicknames` 建立线程到昵称映射，`used_agent_nicknames` 是曾经分配过的昵称池，`nickname_reset_count` 记录池重置次数。它把“当前活跃线程”与“历史已用昵称”分开；释放线程不会自动释放昵称。

`format_agent_nickname(name, nickname_reset_count) -> String`（`L34-L51`）在计数为 0 时原样返回 `name`；否则把 `reset_count + 1` 作为序号，按 `%100` 特判 11–13 为 `th`，其余按 `%10` 选择 `st/nd/rd/th`，生成 `"{name} the {value}{suffix}"`。计数溢出会在 `reset_count + 1` 处按 Rust release/debug 语义处理，源码没有额外饱和；正常路径由 `usize` 递增。

`session_depth(&SessionSource) -> i32`（`L53-L59`）只从 `SessionSource::SubAgent(SubAgentSource::ThreadSpawn { depth, .. })` 读取 `depth`；其他 `SubAgent` 变体和根来源都默认为 0。`next_thread_spawn_depth`（`L61-L63`）调用前者并用 `saturating_add(1)`，因此达到 `i32::MAX` 时仍为最大值，不回绕。`exceeds_thread_spawn_depth_limit(depth, max_depth) -> bool`（`L65-L67`）严格判断 `depth > max_depth`，等于上限允许，负数也按普通整数比较。

`Guards::reserve_spawn_slot(&Arc<Self>, max_threads) -> Result<SpawnReservation>`（`L69-L86`）是两阶段 spawn 入口。给出 `Some(max_threads)` 时调用 CAS 循环封装的 `try_increment_spawned`；失败返回 `CodexErr::AgentLimitReached { max_threads }`，不产生 reservation。给出 `None` 时直接 `fetch_add(1, AcqRel)`，表示不设上限。成功后返回 `SpawnReservation { state: Arc::clone(self), active: true, reserved_agent_nickname: None }`；此时只占配额，还没有线程 ID 或昵称登记。

`release_spawned_thread(&self, thread_id)`（`L88-L101`）先锁住 `active_agents`，移除 `thread_id`，并无条件清理其昵称映射；只有 `threads_set.remove` 真正返回 `true` 时才以 `AcqRel` 对 `total_count` 减一。因此未知 ID、重复释放都不会误减计数，且不会清空 `used_agent_nicknames`。锁被 poison 时用 `PoisonError::into_inner` 继续操作，避免因 poison 永久失效。

私有 `register_spawned_thread(&self, thread_id, agent_nickname)`（`L103-L117`）在同一 mutex 临界区插入线程；若有昵称，同时写入历史昵称集合和线程昵称映射。`HashSet::insert` 的返回值被忽略，所以相同 `ThreadId` 的重复 commit 不会使集合计数增加，但调用方若重复 commit 同一 reservation 并不可行（commit 会消费 reservation）。

私有 `reserve_agent_nickname(&self, names, preferred) -> Option<String>`（`L119-L157`）在 mutex 内完成选择和占用。`preferred` 有值时直接采用该字符串，不检查它是否在 `names` 或池中可用；随后仍插入 `used_agent_nicknames`。无 preferred 且 `names` 为空立即返回 `None`，不会插入任何值。否则把每个基础名按当前 reset 次数格式化，过滤历史已用名，用随机 `choose(&mut rand::rng())` 取一个。候选耗尽时清空已用池、递增 reset 计数，并向可用 OpenTelemetry 全局 metrics 的 `codex.multi_agent.nickname_pool_reset` counter 增加 1（指标失败被忽略）；再从原始 `names` 随机选名并格式化。原始切片非空时 `?` 理论上必成功。最终无论 preferred 还是随机路径都把昵称标记为 used 并返回 `Some`。随机性意味着同一输入不能保证固定昵称顺序；mutex 保证并发 reservation 不重复观察同一候选。

私有 `try_increment_spawned(&self, max_threads) -> bool`（`L159-L175`）先用 `Acquire` 读取当前值；循环中若 `current >= max_threads` 返回 `false`，否则 `compare_exchange_weak(current, current+1, AcqRel, Acquire)` 成功返回 `true`，失败把 `current` 更新为竞争者值后重试。它只限制“预留/提交”计数，不检查 `threads_set`，因此 reservation 的 RAII 回滚必须与该计数配套。

`SpawnReservation`（`L178-L182`）持有共享 `Arc<Guards>`、`active` 标记和可选预留昵称。`reserve_agent_nickname(&mut self, names)`（`L184-L187`）只是以 `preferred=None` 调用下一函数。`reserve_agent_nickname_with_preference`（`L189-L202`）调用 Guards 的选择器；无名可用时把 `None` 转成 `CodexErr::UnsupportedOperation("no available agent nicknames")`，成功则把结果保存在 reservation 内并返回副本。注意昵称一旦预留，即使 reservation 随后失败 drop，也仍留在 used 池中。

`commit(self, thread_id)`（`L204-L206`）以无昵称参数转到 `commit_with_agent_nickname`。后者（`L208-L217`）优先采用本 reservation 已预留的昵称，否则采用传入昵称；调用 `register_spawned_thread`，最后把 `active=false`。它消费 `self`，提交后由外部在真实线程结束时调用 `release_spawned_thread`。

`impl Drop for SpawnReservation`（`L220-L226`）提供失败回滚：若仍 `active`，以 `AcqRel` 对 `total_count` 减一；commit 后 active 已清零，不会二次扣减。这里没有线程 join、取消或持久化，只回滚 admission 计数。末尾 `#[cfg(test)] #[path="guards_tests.rs"] mod tests`（`L228-L230`）把同目录测试接入编译。

并发语义可归纳为：总配额用无锁原子 CAS 抢占，活跃线程/昵称映射用可恢复 poison 的 mutex；reservation 是“先占槽、成功 commit、异常 drop 自动释放”的线性资源。昵称池的随机选择在 mutex 内进行，指标调用也发生在锁内；源码没有跨进程协调，`Arc` 只覆盖同一进程用户 session。

## 状态、取消、恢复与副作用

本文件没有取消 token、超时、重试策略或恢复日志。取消/异常的最小语义是 reservation 尚未 `commit` 时析构自动 `fetch_sub`；已经 `commit` 的线程必须显式 `release_spawned_thread`，否则配额一直被占用。`release_spawned_thread` 可重复调用且对未知 ID 无害，但不会恢复昵称可用性；昵称直到整个池耗尽并 reset 才可用带序号的新名字。`session_depth` 只计算调用方传入的 session 元数据，不保存深度。

持久化方面没有文件、数据库或网络写入；唯一外部可见副作用是可选的 OpenTelemetry counter `codex.multi_agent.nickname_pool_reset`（`L140-L145`）。随机数来自 `rand::rng()`，因此重启后不保证选择序列。mutex poison 被吞并继续使用可能保留部分状态，但没有 fail-closed 日志。计数使用 `fetch_add/fetch_sub`，源码未为无上限模式的 `usize` 溢出定义业务策略。

## 源内测试与行为判据

同目录 `guards_tests.rs` 由 `L228-L230` 引入，测试覆盖以下可独立验证判据：`format_agent_nickname` 的 0、2、3、11、21 序数后缀（`guards_tests.rs:L5-L12`）；根 session 深度为 0，`ThreadSpawn` 深度递增且超过上限，其他 sub-agent 深度按 0 起算（`L14-L38`）；reservation drop 释放槽、commit 保持槽直到真实 `release_spawned_thread`、未知 ID 不释放、重复释放幂等（`L40-L127`）。昵称行为包括：失败 spawn 仍占用旧昵称、池耗尽后产生 `the 2nd`、线程释放后昵称仍保持 used、重复 reset 递增到 `the 3rd`，并检查 `nickname_reset_count`（`L129-L243`）。这些断言共同规定“配额按线程集合计数，昵称按历史池计数”的双状态模型。未见 guards 源内针对 mutex 并发竞态、`preferred` 与重复名、`usize` 溢出或 metrics 不可用的测试；可补充的独立判据是多线程并发 `reserve_spawn_slot(Some(N))` 成功数不超过 N，所有未 commit reservation drop 后可再次抢占，且每个已 commit ID 只释放一次。

## zenpi Rust 映射

zenpi 已有的 `protocol.rs` 提供可复用的有界消息协议：`MailboxRequest::{Send, Receive, Acknowledge, Claim, Complete}` 与 `MailboxOutcome`（`src/protocol.rs:L81-L117`），并限制 mailbox 文本、TTL、ID；它适合作为 worker 节点给 parent、grandparent、直接 sibling、直接 child 发消息的持久化邮箱命令。建议增加 `DagNodeId`、`DagRelation` 和带 `sender/recipient/session_id/message_id/causal_parent/ttl` 的 `DagMailboxMessage`，在 `validate_mailbox` 的现有边界（`src/protocol.rs:L544-L585`）之外拒绝越权关系；`Receive` 返回事件序号，`Claim/Complete` 保证一次消费和显式成功/失败/放弃。

通信层可组合三种现有原语：一是 `protocol.rs` 的 JSONL mailbox（跨进程、可重连）；二是 `runtime.rs` 的 `BackgroundRunner` 有界 `sync_channel`、`RuntimeEvent` 和 `JobId`（`src/runtime.rs:L49-L118`、`L171-L195`、`L236-L285`），作为进程内 worker command/event mailbox；三是 `core.rs` 的 `AgentEvent`（`TurnAccepted/ToolProgress/ToolResult/Handoff/Error`，`src/core.rs:L275-L326`）作为生命周期事件流。`headless.rs` 已经把 stdin/stdout JSONL、相关 ID、事件缓冲和关闭/重放串起来（`src/headless.rs:L1-L66`、`L561-L580`），应把 DAG 状态事件编码成 `StdioEvent`，而不是让 worker 直接写 stdout。

会话父子关系目前只有可复用的 `Turn.parent_id`（`src/core.rs:L141-L204`）以及 `SessionStore` 的 append-only records、tree 与 handoff 投影（`src/session.rs:L144-L160`、`L321-L329`、`L1142-L1168`）。建议在 `session.rs` 的 session tree 记录中补充 `dag_node_id`, `parent_node_id`, `root_node_id`, `depth`, `state`, `direct_children`，并让一次 worker spawn 产生不可变的 parent link；grandparent 通过重复 parent link 得到，direct sibling 通过共同 parent 索引得到，direct child 通过 child 索引得到。不要只把这些关系塞进自由格式 `Turn.metadata`，因为 close 判定和恢复需要可验证索引。`append_runtime_intent` 已明确“只持久化请求，不启动 scheduler/worker/process/network”（`src/session.rs:L1170-L1185`），可作为派生意图的 durable receipt；真正派生仍由 runtime owner 执行。

建议在 `src/core.rs` 增加 `DagNodeState::{Pending,Running,Waiting,Green,Blocked,Failed,Closing,Closed}`、`DagNodeSnapshot`、`DagWorkerContext` 与 `Agent::derive_worker/receive_dag_event/try_close_node`。`try_close_node` 的可验证规则是：自身达到 `Green`，且 direct children 集合非空时每个 child 均为 `Green`/`Closed`，递归祖先汇总表确认全部 child/grandchild 均为绿；任一 `Failed/Blocked/Running/Waiting` 都返回保活结果而非关闭。关闭应写入 session event，并要求 child completion/ack 的 message ID 已 `Complete(Succeeded)`，避免仅凭内存事件误关。

“保活与派生”的最小机制可直接仿照本文件：在 `core.rs` 或新 `dag.rs` 放置 `Arc<DagGuards>`，其中 `Mutex<HashMap<DagNodeId, NodeLease>>` 保存节点、父子边、heartbeat/last_progress，`AtomicUsize` 限制全局/每 session worker 数；`DagSpawnReservation` 实现 `Drop` 回滚、`commit(child_id)` 登记边。保活是一个不关闭的 `NodeLease` 加周期 heartbeat event；当新工作到达或 child 不绿时，保留 lease 并用 `BackgroundRunner::try_submit` 派生新 worker，受 `RuntimeConfig.command_capacity/max_pending`（`src/runtime.rs:L103-L132`）限制，队列满返回可重试的 `SubmitError::QueueFull`。派生前把 `append_runtime_intent` 或等价 `dag_spawn_reserved` 事件落盘，派生成功后写 `dag_worker_started`，失败则写 `dag_worker_failed`，恢复时依据 journal 重建 lease，而不是假定线程仍存在。

`runtime.rs` 的 `CancellationToken` 是取消边界（`src/runtime.rs:L49-L100`）：worker 在 provider chunk、mailbox wait、重试前检查 token；`mark_completed` 后晚到 cancel 不应把成功改写成取消。`tool_runtime.rs` 已有并发 batch、取消前置检查、join 与“取消不是 rollback”的语义（`src/tool_runtime.rs:L157-L168`、`L275-L349`），DAG worker 的工具调用应复用它，节点关闭只在工具结果已完成且审计事件写入后进行。`approval.rs` 的 `ApprovalCoordinator` 用 `Arc<(Mutex, Condvar)>`、pending/visible/accepted 状态并在取消时清理（`src/approval.rs:L126-L147`、`L194-L243`），适合把 parent/child 需要人工确认的消息作为有界 rendezvous；批准响应必须先持久化再放行副作用，沿用 `persist_accepted/mark_persisted` 语义。

`providers/**`（`openai.rs`、`anthropic.rs`、`google.rs`、`deepseek.rs`、`codex.rs`、`connection.rs`、`registry.rs`）应保持 provider 适配职责：把流式 token/错误映射到 `AgentEvent::Provider` 或 runtime 输出，并接受 `CancellationToken`；不得自行决定 DAG close。`headless.rs` 负责把节点事件、mailbox receipt、keepalive/close 结果编码到 JSONL 并做重放；`core.rs` 负责 DAG 状态机和 close 判定；`session.rs` 负责父子边、状态转移、spawn/heartbeat/complete 事件的追加与恢复；`runtime.rs` 负责有界派生和取消；`protocol.rs` 负责跨进程消息 schema；`approval.rs` 负责需要人类决策的闸门；`tool_runtime.rs` 负责工具批次的并发与 join。

与源文件的差异清单（可执行、可验证）：(1) Codex `Guards` 只统计同 session 的线程和昵称，zenpi 需按 DAG 节点、边、祖先完成度统计，并加每节点/每深度配额；(2) Codex 释放依赖显式 thread ID，zenpi 需持久化 lease/heartbeat，使进程重启可恢复；(3) Codex nickname 是随机临时标识，zenpi 节点 ID、parent link、message ID 必须稳定且可校验；(4) Codex 没有邮箱，zenpi 应为四类邻接关系定义授权矩阵并测试 sibling 越权被拒；(5) Codex 无 close 状态机，zenpi 应测试“自身绿但任一孙节点非绿时保持活跃并成功派生新 worker”；(6) Codex 无超时/取消传播，zenpi 应验证 parent cancel 能唤醒 mailbox/approval/runtime，并确认已发生的外部副作用不被伪称 rollback；(7) 用 session journal 重放测试：重启后仅依据 `dag_worker_started/heartbeat/child_completed/node_closed` 事件重建，重复 complete/close 必须幂等。

## 未决问题

无法从 `guards.rs` 确认 `SessionSource::ThreadSpawn` 的完整创建链路、`ThreadId` 是否跨进程稳定、以及上层何时调用 `release_spawned_thread`；也无法由本文件确认昵称是否需要跨 session 唯一。zenpi 映射中的 DAG 状态事件名、heartbeat 周期、每节点配额和 mailbox 授权矩阵需要结合产品协议另行定案；这些不影响上述最小机制和可验证 close 规则。

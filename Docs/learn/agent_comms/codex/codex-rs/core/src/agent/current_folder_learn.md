# Codex `core/src/agent` 目录级学习汇总

## 范围与目录职责

本汇总覆盖 11 份已完成的 1:1 笔记：`agent_names.txt`、`builtins/awaiter.toml`、`builtins/explorer.toml`、`control.rs`、`control_tests.rs`、`guards.rs`、`guards_tests.rs`、`mod.rs`、`role.rs`、`role_tests.rs`、`status.rs`。笔记中的源哈希、字节范围和行范围已作为本次阅读依据；当前环境中用户给出的 opencode 源目录 `/Users/wangweiyang/GitHub/Docs/learn/agent_comms/codex/codex-rs/core/src/agent` 不存在，因此没有把未实际挂载的源码当作新事实。该目录的整体职责是 Codex 多 agent 的控制面：组装模块、解析角色配置、派生/恢复 thread、向 thread 投递输入、投影生命周期状态、限制并发与昵称资源，并通过直接 parent 的完成通知把 child 结果回注入会话。它是“线程树 + 一跳 parent 通知”，不是完整 DAG 编排器：源实现没有 grandparent/sibling 多播、全后代聚合 close、lease heartbeat 或自动派生策略。

## 模块清单

- `agent_names.txt`：101 个非空 ASCII 人名的只读池，从 `Euclid` 到 `Jason`；无 Rust 导出，名称只能做人类可读标签，不能代替 `ThreadId`、`session_id` 或授权身份。
- `builtins/awaiter.toml`：内置 awaiter 配置，关键键为 `background_terminal_max_timeout = 3600000`、`model_reasoning_effort = "low"` 和 `developer_instructions`；没有函数导出，提示词规定只轮询给定任务至成功、失败或明确 stop，不得臆造成功。
- `builtins/explorer.toml`：零字节 TOML 文件；无键、无导出、无默认 prompt，任何 explorer 行为都来自宿主配置或 `role.rs`。
- `control.rs`：导出 `AgentControl`、`SpawnAgentOptions` 及 spawn、fork、resume、`send_input`、`interrupt_agent`、`shutdown_agent`、状态查询和订阅方法；使用 `Weak<ThreadManagerState>` 避免引用环，使用共享 `Arc<Guards>` 限制 slot，并以 `watch::Receiver<AgentStatus>` 驱动 detached completion watcher。
- `control_tests.rs`：不新增生产导出；以 `AgentControlHarness` 覆盖 manager/thread 缺失错误、输入投递、fork 历史、状态 watch、最大线程数、slot 回收、child→parent 通知、nickname/role 恢复。
- `guards.rs`：导出 `next_thread_spawn_depth`、`exceeds_thread_spawn_depth_limit`、`Guards`、`SpawnReservation` 相关能力；原子计数负责活动 agent 上限，`Mutex` 负责 thread/nickname 集合，reservation 以 `Drop` 回滚未 commit 的配额。
- `guards_tests.rs`：不新增生产导出；固定深度递增、上限判断、未知/重复 thread release 幂等、RAII slot 回收，以及昵称池耗尽后 `the 2nd`、`the 3rd` 的历史性 reset 语义。
- `mod.rs`：模块装配入口，`pub(crate)` 重导出 `AgentControl`、`AgentStatus`、`exceeds_thread_spawn_depth_limit`、`next_thread_spawn_depth`、`agent_status_from_event`；本身没有状态存储或执行循环。
- `role.rs`：导出 `DEFAULT_ROLE_NAME`、`apply_role_to_config`、`resolve_role_config`、`spawn_tool_spec` 及 `built_in` 配置查询；用户 role 优先于内置 role，role 作为高优先级 `SessionFlags` 层合并，显式 model/provider/profile 可覆盖当前配置，未声明字段保留。
- `role_tests.rs`：不新增生产导出；验证未知/缺失/非法 role、配置层优先级、profile/provider/sandbox/skills 合并，以及用户 role 的去重、排序和 model/reasoning locked 提示；explorer 正向测试被 `#[ignore]`。
- `status.rs`：导出 `agent_status_from_event` 与 `is_final`；把 `TurnStarted`、`TurnComplete`、`TurnAborted`、`Error`、`ShutdownComplete` 投影为 `AgentStatus`，但 `is_final` 只表示普通 agent 不再等待，不表示业务成功或 DAG 可关闭。

## 运行时数据流与控制流

Codex 源侧的典型路径是：调用方进入 `AgentControl::spawn_agent_with_options` → `Weak` 升级 → `Guards::reserve_spawn_slot` → 从 `ThreadSpawn` 的 parent 继承 shell snapshot、兼容的 exec policy、`depth`、role 和 nickname → 若要求 fork，先 `ensure_rollout_materialized`/`flush_rollout`，读取 parent JSONL 历史并追加带原 `call_id` 的 `FORKED_SPAWN_AGENT_OUTPUT_MESSAGE` → 创建或恢复 child thread → reservation `commit`、`notify_thread_created` → 向 child 投递 `Op::UserInput` → 为有 parent 的 child 启动 completion watcher。child 的 `EventMsg` 经 `agent_status_from_event` 稀疏折叠为 `AgentStatus`；watcher 看到 `is_final` 后只向直接 parent 调 `inject_user_message_without_turn`。`format_environment_context_subagents` 也只枚举直接 child，不递归孙代。

角色路径是：`resolve_role_config` 先查用户 `config.agent_roles`，再查 `built_in::configs()` → 读取用户文件或嵌入的 `explorer.toml`/`awaiter.toml` → 解析相对路径和 TOML → 构造含 `ConfigLayerSource::SessionFlags` 的新层栈 → 全部成功后一次性替换 `Config`。角色缺失给 `unknown agent_type '...'`，读取/解析/重载失败折叠为 `agent type is currently not available`。角色只描述能力和配置，不负责 worker 拓扑。

面向 zenpi 的运行时流应收敛为：`headless.rs` 读取 JSONL `StdioRequest` → `protocol.rs` 校验 mailbox/DAG action、ID、TTL、payload 和 request correlation → `core.rs` 做节点授权、lease/generation、状态 reducer 与 close gate → `session.rs` 先 append durable DAG/邮箱事件并更新 projection → `runtime.rs::BackgroundRunner` 有界 admission、heartbeat poll、tool/provider job 和派生 worker → `AgentEvent`/`RuntimeEvent` 回到 `core.rs` → 只有自身业务结果和全部后代都为 Green 才写 `dag_node_closed` → `headless.rs` 用带 `sequence`、`request_id`、`node_id`、`generation` 的 `StdioEvent` 对外流式呈现。实时 progress 可走 bounded event channel；work、ack、heartbeat、terminal receipt 必须走持久邮箱或 journal，不能只写 stdout。

当前 zenpi 还有一个重要现状：`src/dag.rs` 已公开 `DagStore`、`DagNode`、`DagMessage`、`DagRelation`、`spawn_worker`、`send_to_worker`、`wait_for_message` 和 `status_view`。它使用 file-locked JSON，支持 `parent`、`grandparent`、`sibling`、`child`、`all` 解析，`claim_inbox`/`ack`，`touch_worker`，以及 `DagStore::can_close` 递归检查自身和全部 descendants 的 `green`。但 `src/lib.rs` 只是 `pub mod dag;`，`headless.rs`、`core.rs`、`session.rs`、`runtime.rs` 当前没有 DAG 调用点；因此它是可复用的原型/现状基线，不应被误读为已经接入主执行生命周期。

## 错误、取消、恢复与资源语义

源目录的错误边界很清楚：manager 被 drop 时控制入口返回 `UnsupportedOperation("thread manager dropped")`；thread 缺失时输入/订阅是 `ThreadNotFound`，查询是 `AgentStatus::NotFound`；活动 agent 超上限是 `AgentLimitReached`；未 commit 的 `SpawnReservation` 由 `Drop` 释放 slot，已 commit 的 thread 只能按真实 `ThreadId` 显式 release，未知或重复 release 不会误减计数。nickname 的历史占用与活动 thread 分离，失败 spawn 释放 slot 但仍可能消耗名称；角色缺失/非法配置不应留下半合并 `Config`。`status.rs` 的 `Errored`、`Shutdown`、`NotFound` 虽是 `is_final == true`，都不能直接解释为 Green；`Completed(None)` 也只是普通 agent 完成，不是业务成功证明。

取消是合作式的：`interrupt_agent` 只发 `Op::Interrupt`，保留 thread 和 slot；`shutdown_agent` 发 `Op::Shutdown`，随后无论发送是否成功都尝试 remove/release；awaiter 的反复 tool call 是观察同一任务，不是重新提交任务。fork/resume 的恢复依赖 rollout JSONL 和可选 SQLite metadata；completion watcher 是 detached task，parent 已删除时通知可静默丢失。源实现没有 deadline、mailbox claim、跨进程恢复或副作用回滚协议。

zenpi 必须保留并强化这些语义：`RuntimeEvent::Completed`/`JobOutcome::{Succeeded,Failed,Cancelled,Panicked}` 只描述一个 job；`SubmitError::{QueueFull,Closed}` 只说明 admission 失败，不能标 Green；`CancellationToken` 只做 cooperative cancel，`shutdown grace` 后 detach 不等于副作用回滚。provider transport/partial response 应保持 `UnknownOutcome`，已 claim 的 mailbox work 不能被隐式重复执行。旧 `owner_epoch`、旧 `generation`、过期 lease 的迟到完成必须拒绝；`message_id`、operation id 和 terminal receipt 要幂等。

## zenpi Rust 映射建议

### 通信原语与四向邻接

优先复用 `src/protocol.rs` 的 `MailboxRequest::{Send,Receive,Acknowledge,Claim,Complete}`、`MailboxOutcome`、`StdioEvent` 及现有 ID/文本/TTL 校验；复用 `src/session.rs` 的 `MailboxMessage`、`SessionMailbox::{enqueue,claim_message,claim_next,finish_claim,finish_claim_with_reply}` 和 `LiveSessionRegistry::{register,heartbeat,active}`。进程内高频事件使用 `src/runtime.rs` 的 `RuntimeEvent`/bounded channel，节点状态用 `src/core.rs` 的 `AgentEvent` 适配；`src/headless.rs` 的 `AsyncEventBuffer` 只做有界流式投影，并为 admission、error、Green、derived、closed 事件保留高优先级。

worker 处理 node 时，server-side DAG 索引计算目标而不是信任客户端：`parent` 是 `parent_id`；`grandparent` 沿 parent 链上溯一次；direct `sibling` 是共同 parent 的 children 去掉自身；direct `child` 是当前 node 的 children。消息 envelope 至少包含 `dag_id`、`node_id`、`sender_node_id`、`recipient_node_id`、`relation`、`generation`、`message_id`、`request_id`、`reply_to`、`payload`、`expires_at_ms`。parent/grandparent/sibling/child 的发送都走 mailbox；sibling 可以直接发送但只能发给 registry 计算出的白名单目标，不能用任意 session ID 绕过 workspace/授权检查。

`src/dag.rs` 的 `DagStore::recipients` 和 `claim_inbox` 可作为关系解析与 claim/ack 的参考，但当前 `send` 对任意合法 node ID 也会接受，`DagMessage` 没有 relation、generation、TTL、reply digest，`ack` 也不是 `SessionMailbox` 的 owner epoch/terminal receipt。因此建议选择一个权威事实源：最好将 DAG 事件、边、lease 与 mailbox receipt 纳入 `SessionStore` 的 append-only journal，或明确 `DagStore` 只是 journal 的可重建 projection，避免 `dag.json` 与 session JSONL 分叉。

### 会话父子关系、保活与派生的最小机制

`src/core.rs::Turn::parent_id` 可表示一次对话父链，但不能替代 worker DAG；`src/session_tree.rs` 也主要是 transcript tree。建议在 `session.rs` 的 durable record 中增加 `DagNodeRecord { dag_id, node_id, session_id, parent_id, root_id, children, state, generation, owner_epoch, lease_expires_at_ms, role_digest, policy_digest }` 和 `DagEdgeRecord`。节点创建时校验 parent 存在、depth 单调、无环、ID 幂等；恢复时由 journal 重建 `parent_of`、`grandparent_of`、`children_of`、`siblings_of`、`descendants`。

最小保活协议是：worker 注册 `(session_id,node_id,owner_epoch,workspace)`，获得 `generation` 和 lease；周期 tick 调 `heartbeat` 并写 `dag_heartbeat`，heartbeat 只续租，不等于 Green；本节点工作、tool result、approval 和 terminal receipt 全部 durable 后才能写 `NodeGreen`。`is_final` 需拆为 `is_terminal` 与业务 `is_green`。节点在有界单写者/事务或 compare-and-append 边界内执行：

`can_close(node) = own_state == Green && every_descendant(node) == Green && no_pending_claim && no_pending_approval && no_unsettled_operation && lease_is_current`。

任一 child、grandchild 或更深 descendant 为 `Running`、`Pending`、`Failed`、`Cancelled`、`Unknown`、expired，或收到新 work，节点都保持 `KeepAlive`，继续 heartbeat，并记录 `close_deferred` 原因。只有一次幂等的 `dag_node_closed` 成功写入后，才可调用 `Agent::try_close`/释放 owner。动态派生出的新 descendant 应纳入 close 快照；close 与 child completion 必须由同一 journal 条件写入规则序列化。

收到新 work 或需要恢复时，先以 `(dag_id,node_id,work_id,generation)` 写 `derive_intent`，再用类似 `DagSpawnReservation` 的 RAII admission guard 限制活动 worker/深度，最后调用 `BackgroundRunner::try_submit`。只有收到 `Accepted/Started` 才追加 `derived` 并登记 child edge；`QueueFull`/`Closed` 保留原 lease、邮箱 work 和重试证据，不得伪造 Green。新 worker 先 claim 未完成 mailbox，再处理 work；旧 generation 的 completion 被拒绝。派生成功后父节点仍保持 alive，直至完整后代聚合满足 close gate。

### 四个既有模块的具体落点

- `src/headless.rs`：现有 `handle_command`、`Command::Mailbox`、`run_async_streams`、`AsyncEventBuffer`、`handle_runtime_event` 是 wire I/O 和事件投影边界。增加 `Command::Dag` 或 `DagAction::{Send,Receive,Claim,Complete,Heartbeat,Status,Derive,Close}`，负责 JSONL schema、订阅、sequence/replay、backpressure；不得在此直接判定 subtree Green。EOF、client disconnect、runner detach 都不能当作 close；`QueueFull` 要返回 typed 可重试响应，terminal/Green/Derived 事件不能被普通 progress 挤掉。
- `src/core.rs`：`Agent` 持有 `AgentPhase`、`AgentEvent`、`WorkerExecutionBinding`、live owner/mailbox 方法以及 `process_with_cancel_and_events`；在此增加 `DagCoordinator`/`NodeLedger`、`dispatch_dag_message`、`record_dag_state`、`heartbeat_dag_node`、`evaluate_close_gate`、`derive_worker`、`close_dag_node`。现有 `Agent::try_close` 会直接进入 `AgentPhase::Closed`，必须改为先过递归 close gate；provider/tool 终态只产生 reducer 输入，不直接关闭 DAG。
- `src/session.rs`：`SessionStore::append_event`、`append_runtime_intent`、operation recovery、`MailboxMessage`/`SessionMailbox`、`LiveSessionRegistry` 是 durable 边界。追加 `dag_node_started/status/heartbeat/green/derive_intent/derived/close_deferred/closed` 和 `DagEdgeRecord`；邮箱 payload 记录 relation/generation/digest/reply，claim 绑定 owner epoch，重启从 journal 恢复拓扑与 lease。`append_runtime_intent` 只持久化意图、不启动 scheduler，因此真正派生仍由 `core`/`runtime` 执行；不要把内存 `HashMap` 或当前 `src/dag.rs` 的进程内 `SPAWNED` 当作唯一事实源。
- `src/runtime.rs`：`BackgroundRunner` 提供 bounded command/event channel、`JobId`、`try_submit`、`try_cancel`、`RuntimeEvent` 和 shutdown grace；为 node 保存 `NodeId ↔ JobId`，承载 mailbox poll、heartbeat tick、provider/tool job 和派生。`JobOutcome::Cancelled/Panicked` 是非绿；`Closed` 是 runner 终态，不是 DAG close；`CancellationToken::mark_completed` 后的 late cancel 不能改写已发布成功。用 `RuntimeConfig::max_pending` 防派生风暴，取消前检查 provider chunk、tool batch、mailbox wait 和 retry 边界。

辅助边界保持单一职责：`src/tool_runtime.rs` 只执行有界、可取消、按输入顺序归并的工具批次；`src/approval.rs::ApprovalCoordinator` 继续做 policy/lease 绑定的人类审批闸门，`Accepted` 先持久化再放行 side effect；`src/providers/**` 只做 provider route/model/capability 与 `ProviderEvent`，不保存拓扑、不做邻居路由、不决定 Green/close。role 配置可以形成 immutable model/reasoning/skill/policy snapshot，但不能成为通信授权。

## 未决问题

1. zenpi 的 Green 业务定义尚未完全确定：provider 成功、工具全成功、approval 已落盘、外部副作用可验证、`Completed(None)`、`Interrupted` 和 `UnknownOutcome` 分别如何归类，需写入 `ClosePolicy`。
2. close gate 与动态派生的原子规则仍需明确：新 descendant 是否使已发出的 close 失效，child completion 与 close 谁胜出，Failed child 是否可由新 generation 替代，以及全部后代是否要求无 pending claim/approval/operation。
3. 当前 `src/dag.rs` 使用 `open/green/red` 和单独的 `dag.json`，而核心 `AgentPhase` 是 `Idle/Running/Closed`，邮箱是 `Queued/Claimed/Succeeded/Failed/Expired`；需要决定统一状态模型和唯一持久化事实源。
4. `src/dag.rs::upsert` 可自动创建缺失 parent，存在形成孤儿/错误拓扑的风险；`recipients` 允许任意合法 node ID，需补 server-side relation authorization、workspace 校验、无环校验和 generation/lease 条件。
5. `background_terminal_max_timeout = 3600000` 的单位、硬 deadline、超时错误码和指数轮询上限未由 TOML 定义；zenpi 应显式使用 `deadline_ms`，区分 `TimedOut`、`Cancelled`、`UnknownOutcome`，且轮询不能重新提交副作用任务。
6. DAG node 与 session 是一对一还是一对多、是否允许跨 workspace、grandparent/sibling 是直接 mailbox 还是必须 parent 转发、节点/worker 身份如何认证，尚未由 Codex 源目录规定。
7. 现有 `src/dag.rs` 的 child process registry、heartbeat、message truncation 和 file lock 还缺 crash recovery、TTL 清扫、dead-letter、terminal receipt、重复 close 防护及集成测试；需要覆盖 parent→grandparent、sibling、child 四向通信，孙节点非绿时保活/派生，全部递归 Green 后只 close 一次，以及 QueueFull、取消、过期 lease、旧 generation completion 和重启重放。

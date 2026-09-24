# AC-022 — codex/codex-rs/core/src/agent/mod.rs

- source_id/item_id：codex/codex-rs/core/src/agent/mod.rs / AC-022
- source_path：codex/codex-rs/core/src/agent/mod.rs
- source_hash：7d4ac3345a53771db0d34e90a90c085dd488f1551079d0d4bb5a481b4116e9b1（SHA-256）
- source_bytes：326
- source_lines：10
- coverage：已按文件顺序读完字节 0–325（共 326 字节），覆盖行 L1-L10，包括全部注释、模块声明与 pub(crate) use 导出。

## 完整行为复盘

该文件是 agent 模块的装配入口，没有独立的函数体、数据存储或执行循环；它通过模块可见性和重导出把控制、深度保护、角色配置、状态投影组合为同一 crate 内的 API。以下逐项对应源文件行号。

- pub(crate) mod control;（L1）：声明并编译同目录 control.rs，只在当前 crate 内可见。该子模块承载 AgentControl 控制面，适合异步 spawn、向已有线程发送输入、读取状态及线程间通信；本行本身不创建控制器、不分配线程，也没有默认值或错误路径。子模块的构造依赖 Weak<ThreadManagerState>，管理器已销毁时由其函数返回错误（同目录 control_tests.rs 的 manager-dropped 判据）。
- mod guards;（L2）：声明私有 guards 子模块；其内部符号不会因本声明自动对外暴露。后续 L8-L9 选择性重导出两个函数。模块负责并发 spawn 限额与深度计算，内部以原子计数和互斥集合保护共享状态；入口文件不改变这些同步语义。
- pub(crate) mod role;（L3）：声明 crate 内可见的角色解析/提示构建模块。该模块在本文件没有任何 use 重导出，因此调用者需使用 crate::agent::role::... 路径；本文件不规定角色默认值、解析失败或字符串格式。
- pub(crate) mod status;（L4）：声明 crate 内可见的状态投影模块，供 L10 的函数重导出。模块边界只建立名称解析，不进行事件消费或持久化。
- pub(crate) use codex_protocol::protocol::AgentStatus;（L6）：把外部协议类型 AgentStatus 作为 crate::agent::AgentStatus 暴露。它是类型别名式的重导出，不复制状态、不添加构造器、不改变序列化；具体变体（如运行中、完成、错误、关闭）完全由 codex_protocol 定义。
- pub(crate) use control::AgentControl;（L7）：把 control::AgentControl 提升到 crate::agent::AgentControl。该符号是控制面句柄，复制句柄本身不等于复制线程；实际 spawn、消息投递、管理器升级、配额错误均委托 control.rs。并发语义由其共享 Arc/异步管理器决定，入口不保证阻塞或重试。
- pub(crate) use guards::exceeds_thread_spawn_depth_limit;（L8）：重导出深度判定函数。对应实现是纯比较：候选深度大于 max_depth 才为真；输入边界（负数、零、极大值）由实现的整数比较决定，本文件没有钳制、错误或副作用。它只判定，不预留资源、不派生线程。
- pub(crate) use guards::next_thread_spawn_depth;（L9）：重导出下一层 spawn 深度计算函数。对应实现从 SessionSource::SubAgent::ThreadSpawn 读取当前 depth 并饱和加一；根会话及其他子代理来源按零深度处理后得到一。它是纯函数，无 I/O、锁或取消检查；深度上限须另行调用 L8 的判定。
- pub(crate) use status::agent_status_from_event;（L10）：重导出单事件到 AgentStatus 的投影函数。具体映射在 status.rs：TurnStarted→运行中，TurnComplete→完成并携带最后消息，TurnAborted(Interrupted)→中断，其他 abort→错误，Error→错误，ShutdownComplete→关闭；不影响状态的事件返回 None。函数不修改事件、不写盘、不重试，调用方负责按事件顺序保存最新状态。

因此，mod.rs 的默认行为是“只在编译期组织 API”。不存在本文件内的输入校验、返回错误、取消 token、超时、线程启动或外部副作用；这些行为必须查阅上述被声明/重导出的实现。

## 状态、取消、恢复与副作用

本文件没有字段、静态变量或 Drop，所以没有可取消的临界区、超时计时器、持久化点或 provider/tool 外部副作用。AgentStatus 只是协议状态值，agent_status_from_event 的 None 不能解释为成功或失败，只能解释为“该事件不改变状态”。深度函数也不保留调用历史，故重试不会自动增长以外的状态。

被重导出的 AgentControl 才是潜在副作用边界：其 spawn 可能建立新 agent/thread、向管理器发送输入；manager 不存在时应返回可观察错误。guards 的 spawn reservation 采用“预留后提交”：预留对象在未 commit 时析构释放配额，已 commit 的线程要显式 release；这保证并发超限不会静默泄漏。该机制可作为 DAG 派生配额，但不能替代 DAG 完成判定。

## 源内测试与行为判据

mod.rs 自身未包含测试。相邻模块通过 cfg(test) 引入了 control_tests.rs、guards_tests.rs、role_tests.rs：

- guards_tests.rs:L19-L30 验证 thread-spawn 深度加一及 depth > max_depth 的限制；L40-L70 验证 reservation 析构释放槽位、commit 后保持槽位、release 后才能再次预留。
- control_tests.rs:L146-L170 验证 manager 丢失时发送输入报错、查询状态返回 NotFound；L172-L218 验证事件到 AgentStatus 的关键映射；L220-L230 验证 spawn 在 manager 丢失时失败。
- 可独立验证本入口的判据：运行 cargo test（codex core crate）后，检查 crate::agent::AgentControl、crate::agent::AgentStatus 及三个重导出函数可解析；构造根来源、深度为 1 的 ThreadSpawn，应得到下一深度 2；对 EventMsg::TurnStarted 应得到 Some(AgentStatus::Running)；不相关事件应得到 None。这些检查验证的是重导出指向的真实实现，而不是入口新增逻辑。

## zenpi Rust 映射

zenpi 已有足够的通信积木，但需要补一层 DAG 拓扑与绿色收敛协议。建议按以下可执行落点实现：

1. 通信原语复用。src/session.rs:L3570-L3655 的 SessionMailbox 是私有、按 recipient 定址、带 inode lock、恢复 journal、TTL、claim/complete 的持久邮箱；它适合作为跨进程或跨 runtime 的 parent、grandparent、直接 sibling、直接 child 控制/结果通道。src/protocol.rs:L479-L484 已把 mailbox 命令解析为 Command::Mailbox，L544-L581 对 recipient/message、TTL、分页、claim/complete 做边界校验；src/headless.rs:L2065-L2119 的 mailbox_slash_view 是 TUI/headless 共用落点。内存高频事件继续使用 src/runtime.rs:L171-L190 的有序 RuntimeEvent 与 src/headless.rs:L862-L1035 的有界事件缓冲，避免把持久邮箱当广播总线。
2. 会话父子关系。src/core.rs:L140-L175 的 Turn.parent_id 只能表达一条对话父链，不能独立表达 DAG 的多 child。新增 DagNodeId、DagParent { parent: Option<DagNodeId>, grandparent: Option<DagNodeId> } 和 DagEdge { parent, child, kind }（kind 为 child/sibling 等派生关系），把 node/session ID 写入 SessionStore 的 runtime intent 或事件记录；不要只依赖内存 HashMap。同 workspace 与 addressed-session 校验可直接复用 SessionMailbox::check_access 的约束（src/session.rs:L3598-L3614）。
3. worker 与四类邻居通信。worker 处理节点时，先由拓扑索引得到 parent；grandparent 通过 parent 的 parent 链得到；直接 sibling 通过共同 parent 的 child 集合得到；直接 child 通过本节点的 edge 集合得到。发送采用 SessionMailbox::enqueue（请求/ack/绿色状态），接收采用 claim/finish，并用 message_id/digest 做幂等关联。需要即时 UI 更新时，将同一逻辑事件映射为 AgentEvent（src/core.rs:L281-L325）并交给 BackgroundRunner 的 event channel；不要让 sibling 直接共享可变 Agent。
4. 最小保活机制。在 src/core.rs 或新 src/dag.rs 定义 DagLease { node_id, generation, last_heartbeat_ms, ttl_ms, state }，状态至少包含 Running/Green/Failed/Cancelled/Closing。worker 每个 runtime tick 更新 lease；父节点通过 mailbox 发送 heartbeat/lease-renew，child 回 LeaseAck。可复用 Agent::heartbeat_live_owner 的 owner 心跳接口（src/core.rs:L836-L844）和 SessionMailbox 的 TTL；过期只标记 stale 并触发重派，不直接删除 journal。generation 防止旧 worker 的迟到完成覆盖新 worker。
5. 绿色 close 判定。在 src/core.rs 增加 DagNodeState::can_close()：自身必须为 Green，且递归/索引中的全部 child、grandchild 都为 Green，并且没有未完成 mailbox claim、未确认 lease 或 pending runtime job；否则保持 Closing/保活。父节点收齐 child 的 GreenAck 后再向 grandparent 汇报；任何失败、取消、超时都阻止 close，并保留可重派原因。
6. 派生新 worker。src/runtime.rs:L272-L315 的 BackgroundRunner::spawn、try_submit 和有界 command channel 可承担最小 worker 派生；RuntimeEvent::Started/Completed/Rejected（src/runtime.rs:L171-L190）记录 node/generation。队列满返回 SubmitError::QueueFull，应以 mailbox 中的 RetryScheduled 事件重试；worker 关闭返回 Closed，应转为保活/重派，而非虚报 Green。可把 Codex 的深度函数语义映射为 DagDepth::next() 与 max_depth 检查，把 reservation 映射为 DagSpawnGuard，限制单 session 的活动节点数。
7. headless 落点。src/headless.rs:L4406-L4460 已有 stdin reader、bounded channel 和 owner pool；L4467-L4628 已在 worker 内传递 CancellationToken、按 request 缓冲 provider/agent 事件并在 durable completion 后标记完成；L4951-L5170 按 runtime 事件写入生命周期与终态。应在 AsyncRequest 元数据加入 dag_node_id、generation、parent_session_id，并在 handle_runtime_event 的 Completed 分支先写 GreenAck/失败原因，再依据全子树判定是否 close。
8. tool、approval、provider 对照。src/tool_runtime.rs:L228-L322 的取消 epoch/超时等待可作为节点工具批次的 cooperative cancel；工具产生的外部副作用必须经 src/approval.rs:L131-L190 的 ApprovalCoordinator，DAG 消息只传 approval ID 与结果，不绕过审批。src/providers/** 负责 ProviderEvent 与模型连接，属于叶子工作来源；provider 流事件进入 headless 的有界 per-job buffer（src/headless.rs:L4487-L4503），不得用 provider 回调直接改变 DAG Green。src/protocol.rs 增加 DAG 命令/事件的 schema 与大小校验，src/session.rs 持久化拓扑变更、lease 和 ack，src/runtime.rs 负责取消/重派/关闭时序。

差异清单：Codex 入口提供“控制句柄 + spawn 深度守卫 + 单事件状态投影”，zenpi 目前提供“有界 runtime 队列 + durable mailbox + turn parent_id”，缺少多父/多子 DAG 索引、祖先/兄弟寻址、lease generation、递归 Green barrier 和失败后的自动派生策略。实现完成后应能验证：同一节点可向四类邻居发送并关联 ack；任一 child/grandchild 非 Green 时 close 被拒；旧 generation 的完成被丢弃；队列满、取消、runtime closed 均留下可恢复记录。

## 未决问题

无法从 mod.rs 单独确认 AgentControl 的完整消息协议、角色配置格式、AgentStatus 的全部变体及 provider 事件的持久化策略；这些定义分散在 control.rs、role.rs、status.rs 与 codex_protocol。zenpi DAG 的具体节点 ID 来源、子树规模上限、Green 的业务判据（例如 approval 是否必须完成）也需要项目方在协议层明确；本笔记已给出不依赖这些未确认细节的最小接口与可验证边界。


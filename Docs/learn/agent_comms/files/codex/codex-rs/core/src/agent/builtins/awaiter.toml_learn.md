# AC-016 — codex/codex-rs/core/src/agent/builtins/awaiter.toml

- source_id/item_id：`codex/codex-rs/core/src/agent/builtins/awaiter.toml` / `AC-016`
- source_path：`codex/codex-rs/core/src/agent/builtins/awaiter.toml`
- source_hash：`0e760708892f50ae3c6bcc0080e10276c8ac40e6f98bdd4924c3094ba5693f69`
- source_bytes：`1213`
- source_lines：`35`
- coverage：已按顺序读取完整文件，字节 `1-1213`、行 `L1-L35`，包括全部注释、键和值及多行字符串。

## 完整行为复盘

该文件是 built-in agent 的 TOML 配置，不包含 Rust 函数体；可观察的“导出符号”是两个标量配置和一个 `developer_instructions` 提示协议。

1. `background_terminal_max_timeout = 3600000`（`L1`）。它给后台终端等待设置上限；数值边界是固定正整数 `3600000`，源文件没有声明单位、是否可被调用方覆盖、到期后的错误类型或默认回退。按命名和 3600000 的量级可推测为一小时毫秒数，但这只是映射时的假设，不能当作源内事实。输入是后台终端任务，输出应是任务在该上限内的终态或超时结果；具体超时错误、取消动作和并发数均未在本文件定义。
2. `model_reasoning_effort = "low"`（`L2`）。它把该内置 awaiter 的推理档位固定为 `low`。输入是模型运行配置，输出是低推理预算的模型调用；没有空值、未知档位、动态升级或错误路径说明。该值不改变等待协议本身，只约束解释器/模型资源偏好。
3. `developer_instructions`（起始于 `L3`，结束引号在 `L35`）是一段完整的行为契约。`L3-L4` 将角色限定为 awaiter：等待特定 command/task 完成，只在完成时报告状态。`L6-L10` 要求接收 command 或 task identifier 后，用适当工具执行或等待，并持续到 terminal state；因此输入至少包括任务标识，输出是终态报告，不是中途推测。
4. 禁止项位于 `L12-L16`：不得修改任务、解释或优化任务、做无关动作、未经明确指示停止等待。边界是“只观察/执行被给定任务”，禁止隐式重写、拆分或替代任务；若工具本身失败，该失败应成为任务失败终态，而不是被 awaiter 隐藏。
5. 等待循环在 `L18-L22`：任务运行时重复 tool call 轮询；不得臆造完成；等待应使用长 timeout，多次等待时 timeout/yield time 按指数增加。这里的“重试”语义是继续观察同一任务，不是重新提交任务；源没有声明最大轮询次数、指数基数、上限或退避后的错误。
6. 状态查询规则在 `L24-L26`：若被询问 status，返回当前已知状态，然后立即恢复等待。状态查询是旁路读操作，不应把任务转成完成、取消或重试，也不应丢弃原任务标识。
7. 终止条件在 `L28-L32`：仅当成功完成、失败，或收到明确 stop instruction 才退出等待。超时是否等价于失败/停止未定义，故实现必须把 timeout 映射成显式可核对的状态，不能默认为成功。`L34-L35` 要求 deterministic、conservative：同一输入和同一工具观察序列应给出同一状态转移，未知状态保持未知而不补猜。

并发语义：文件没有声明可同时 await 多个任务；“specific command or task”以及单一当前状态更适合一个 awaiter 实例绑定一个任务，轮询调用按顺序进行。若宿主需要并行 DAG 节点，应为每个节点建立独立 awaiter/worker 状态和相关 mailbox，不能把多个任务混在一条等待循环中。源也没有定义输入格式、状态枚举、终态载荷或错误对象，这些必须由宿主协议补齐。

## 状态、取消、恢复与副作用

源只定义三类终止原因：成功、失败、明确 stop（`L28-L32`）。没有专门的取消 token、deadline 传播、暂停/恢复标志、持久化记录、重启恢复游标或重试次数。`background_terminal_max_timeout`（`L1`）提供一个等待边界候选值，但超时后的状态转换未规定；实现应至少区分 `Running`、`Succeeded`、`Failed`、`Stopped`、`TimedOut/Unknown`，并让超时不能伪装成成功。重复 tool call（`L19-L22`）只用于轮询，不能重新执行原 command，以免产生重复外部副作用。

`L12-L16` 明确禁止修改任务和无关动作，因此 awaiter 不应主动改写 DAG、补发任务、优化参数或清理外部资源。实际 command 可能本身有副作用；awaiter 的职责是等待并报告，不是撤销副作用。源没有持久化要求，zenpi 若需要崩溃恢复必须把状态、claim、terminal receipt 另行写入 `SessionStore`/mailbox journal，并使用幂等 `message_id` 或 operation id。

## 源内测试与行为判据

源内未包含测试，`builtins` 同目录只有 `awaiter.toml` 与 `explorer.toml`，没有测试模块或断言。可独立验证的判据是：给定一个延迟后成功的假任务，awaiter 在任务完成前只轮询、不报告成功，完成后报告成功；给定失败任务，最终报告失败；给定运行中任务并查询 status，返回当前状态后继续轮询；给定明确 stop，立即进入停止终态；检查日志可证明没有修改任务、重复提交或执行无关 command；多次轮询的等待间隔单调增大且不超过宿主设定上限。还应验证超时被标记为非成功终态，并在重新启动后依据持久化 receipt 而非猜测完成。

## zenpi Rust 映射

- 通信原语可直接复用 `src/protocol.rs` 的 `MailboxRequest::{Send,Receive,Acknowledge,Claim,Complete}`（`src/protocol.rs:L81-L110` 附近）和 `MailboxOutcome`，以及 `src/session.rs` 的 `MailboxMessage`、`MailboxStatus`、`MailboxAction`、`LiveSessionRegistry`、`SessionMailbox`（`src/session.rs:L3171-L3270`、`L3361-L3558`）。这些提供有界文本/TTL、digest、sequence、claim token 和完成/失败回执，适合把 awaiter 状态通知 parent、grandparent、直接 sibling、直接 child。建议新增 `DagRoute { Parent, Grandparent, DirectSibling, DirectChild }` 与 `DagEnvelope { message_id, dag_id, node_id, sender, recipient, route, kind, payload, deadline_ms }`，路由只接受会话树中已验证的关系，避免任意 session ID 互发。
- 会话父子关系可复用 `src/core.rs` 的 `Turn::parent_id`（`src/core.rs:L141-L196`）作为对话父链，但 DAG 关系应单独持久化，不能把“grandparent/ sibling/ child”从一条 turn 链猜出来。建议在 `src/session.rs` 增加 `DagNodeRecord { node_id, parent_id, children, state, generation }` 和 `DagState::{Running,Green,Failed,Keepalive,Closed}`，用事件记录边和状态变更；直接 sibling 通过共同 `parent_id` 解析，grandparent 通过两次 parent 边解析。
- `src/runtime.rs` 已有最小等待/派生执行机制：`BackgroundRunner::spawn`、`try_submit`、`recv_timeout`、`try_cancel`、`try_shutdown_with_grace`，以及 `CancellationToken` 的 `is_cancelled`/`mark_completed`（`src/runtime.rs:L40-L100`、`L230-L369`）。可定义 `AwaiterJob { node_id, task_id, deadline, poll_state }`，把每次 poll 作为同一 `JobId` 的观察，不在失败/超时后隐式再 submit。`RuntimeConfig` 的有界 command/event/pending 队列和 `poll_interval`（`L103-L131`）对应源的长 timeout 与保守轮询；`JobOutcome::{Succeeded,Failed,Cancelled,Panicked}`（`L158-L170`）可映射终态。
- 保活与派生的最小机制：worker 启动时在 `LiveSessionRegistry::register` 注册 `(session_id, owner_epoch, workspace)`，周期性 `heartbeat`；任务每次 poll/terminal 都写 `dag_heartbeat`/`dag_terminal` 事件。实现 `DagCloseGate::can_close(node)`：只有节点自身为 `Green`，且递归遍历的全部 child/grandchild 都为 `Green`，才允许写 `Closed`。否则写 `Keepalive`，保留 owner lease，并通过 `BackgroundRunner::try_submit` 派生一个带新 `generation`、同一 DAG 和 parent 链的 worker；派生失败必须返回可重试的 `QueueFull`/`Closed`，不能宣称 close。新 worker 先 claim mailbox 中的未完成工作，再处理新任务，完成后以幂等 terminal receipt 汇报。
- `src/headless.rs` 是 wire I/O 所有者（文件头部注释及其 `Command::Mailbox` 分支），应把 `DagEnvelope` 的 Send/Receive/Claim/Complete 转成 JSONL `StdioRequest`/`StdioEvent`，沿用 request id、重放 journal 和有界事件 mailbox；status 请求只读当前 `Running/Keepalive/Green`，然后让 runtime 继续 poll。不要在 headless 层直接判定 subtree green，判定应留在 core/session。
- `src/core.rs` 适合作为编排落点：已有 `Agent::register_live_owner`、`heartbeat_live_owner`、`claim_live_mailbox`、`finish_live_mailbox`、`finish_live_mailbox_with_reply`（`src/core.rs:L820-L909`），可新增 `dispatch_dag_message`、`record_dag_state`、`evaluate_close_gate`、`spawn_recovery_worker`。`Agent` 负责把 awaiter 状态、审批结果和 provider 终态写入 session，再向关系节点发 envelope。
- `src/session.rs` 负责 durable projection 和并发锁：利用 mailbox 的 claim token/owner epoch 防止旧 worker 完成新一代任务；利用 `MailboxStatus::Expired` 区分 TTL 到期与失败（`src/session.rs:L3226-L3253`）。建议把 DAG 状态事件纳入现有 append-only session journal，并在恢复时重建 parent/child 索引；删除 session 前必须保留或显式 retire mailbox。
- `src/tool_runtime.rs` 只执行已获准的 provider tool batch。其 `execute_tool_batch` 语义是准备、并发执行、按源顺序收集结果，取消时不 detach，且不替 awaiter 重试（文件注释及 `L157-L175` 附近）；因此 awaiter 派生新 worker 前必须读取 operation journal，不能把 `Cancelled` 或 `UnknownOutcome` 当作已成功。涉及外部副作用时继续通过 `src/approval.rs` 的 `ApprovalCoordinator`/`ApprovalPolicy`，不得因 awaiter 的“等待完成”角色自动 Allow。
- `src/providers/**` 保持 provider 适配边界：`ProviderEvent` 流由 `src/core.rs` 的 provider sink 转成有界事件；请求控制应检查 `CancellationToken`/`RequestControl` 后再等待下一块输出或重试。awaiter 只消费 provider/agent terminal receipt，不改变 Anthropic/OpenAI/Google 等 provider 的请求参数和重试策略。

差异清单与可执行验证：
1. 源配置只有无单位整数、字符串和提示文本；zenpi 需新增反序列化类型并明确 `max_timeout_ms` 的单位、零值/溢出策略，以及 `Running -> TimedOut` 的错误码。
2. 源协议没有 DAG 路由、代际或 subtree gate；新增 `DagEnvelope`、`DagNodeRecord`、`DagCloseGate` 后，用两个层级以上的 child fixture 验证：任一后代非 `Green` 时 close 被拒绝并产生 heartbeat/派生 worker，全部 green 才产生唯一 `Closed`。
3. 用四个 session（parent、grandparent、两个 direct sibling、一个 child）验证 mailbox 只能按关系路由；旧 `owner_epoch` 的 completion 必须失败，新 epoch 可 claim。
4. 用 `BackgroundRunner` 的 bounded queue 验证派生遇到 `QueueFull` 不丢任务；用 `CancellationToken` 验证 stop/timeout 不会重写已 `mark_completed` 的成功结果；用 headless JSONL 重连验证 terminal receipt 可重放且不会重复执行。

## 未决问题

1. `background_terminal_max_timeout` 的单位、是否为硬 deadline、超时错误名称和调用方覆盖方式无法从源确认。
2. `developer_instructions` 中“appropriate tool”的具体工具集合、任务标识格式、状态枚举和终态载荷无法从源确认。
3. 指数增加 timeout 的初始值、倍率、最大值及网络/工具错误时是否允许重试无法从源确认。
4. 源没有定义 DAG 节点、parent/grandparent/sibling/child 的身份认证、持久化 schema 或 close gate；上述 zenpi 类型和事件名是可执行映射建议，不是源行为。

# AC-048 — opencode/packages/opencode/src/acp/profile.ts

source_id: `AC-048`
item_id: `AC-048`
source_path: `opencode/packages/opencode/src/acp/profile.ts`
source_hash: `e18bc3b08f5a26506394434436333fddc88ac15dcc9fbfbc0eb09944486e3c3a`
source_bytes: `1281`
source_lines: `42`
coverage: 已按源文件顺序完整读取；字节范围 `0–1280`（共 1281 字节），行范围 `L1-L42`（含空行、类型、注释与导出）。

## 完整行为复盘

这是一个只在 stderr 输出的轻量 ACP 性能剖析模块，不负责业务调度。模块初始化发生在 `L1-L2`：`enabled` 只在模块加载时读取一次，只有 `process.env.OPENCODE_ACP_PROFILE === "1"` 才启用；`started` 只记录一次 `performance.now()`，因此它是进程内的单调计时基准，不是墙上时钟，也不会随环境变量变化而动态开关。

- `mark(name, fields?)`（`L4-L7`）输入为字符串 `name`，可选字段为 `Record<string, string | number | boolean | undefined>`，返回 `undefined`。未启用时在 `L5` 立即返回，不读取计时、不写日志；启用时以模块初始化到当前的差值调用 `write`，日志名为 ``${name}.mark``。它没有校验空名称、字段名、负时间或重复标记。
- `duration(name, startedAt, fields?)`（`L9-L16`）输入名称、调用方提供的 `startedAt` 数值和同样的可选字段，返回 `undefined`。未启用时无副作用；启用时计算 `performance.now() - startedAt` 并交给 `write`。`startedAt` 可以来自模块外，源内没有校准、单调性或单位检查，未来时间会产生负时长，最终仍交给 `Math.round`。
- `measure<T>(name, fn, fields?)`（`L18-L30`）输入名称、返回 `Promise<T>` 的异步函数 `fn`、可选字段，输出为 `Promise<T>` 的结果。未启用时在 `L23` 直接调用并返回 `fn()`，所以不额外 `await`、不测量、也不拦截其同步抛错或 rejected promise。启用时先在 `L24` 取开始时间，然后在 `try/finally` 中 `await fn()`（`L25-L28`）：成功时原样返回解析值，失败时原样传播 rejection，但无论成功、失败或 `fn` 同步抛错，`finally` 都尝试记录从本次调用开始的时长。它不改变并发度、不串行化调用，也不自动重试。
- 私有 `write(name, durationMs, fields?)`（`L32-L40`）是唯一输出点。`fields` 存在时按 `Object.entries` 的原有枚举顺序处理；`L35` 丢弃值为 `undefined` 的项，`L36-L37` 将其余项直接格式化为 `key=value` 并以空格连接。没有转义空格、换行、等号或控制字符，因此字段内容可能造成机器解析歧义。最终在 `L39` 以 `Math.round(durationMs)` 四舍五入为毫秒，并调用 `console.error` 输出 `[acp-profile] name Nms`，有字段时追加一个空格和字段串。源内没有 catch，输出失败异常不会被吞掉。
- `export * as ACPProfile from "./profile"`（`L42`）导出当前模块的命名空间，供调用方以 `ACPProfile.mark`、`ACPProfile.duration`、`ACPProfile.measure` 使用；私有 `write` 不成为该命名空间的公开 API。它是导出别名，不增加状态或调度层。

边界与并发语义：字段容器允许 `undefined`，但输出只保留字符串、数字、布尔值；调用方必须自行保证 `name` 与字段可读。全局状态只有加载时的布尔值和起始时间，多个异步 `measure` 可并行执行，各自拥有局部 `start`；日志写入依赖同一进程的 `console.error`，源文件不提供顺序、去重、采样、聚合或请求关联保证。

## 状态、取消、恢复与副作用

源内没有取消 token、超时、重试、持久化、恢复、网络调用、文件写入或 provider 调用。唯一状态是 `enabled` 与 `started`（`L1-L2`），均为模块级初始化状态；唯一外部副作用是启用时向 stderr 调用 `console.error`（`L39`）。`measure` 的 `finally` 保证被测 Promise 结束（resolve/reject/同步异常）后尝试记时，但不保证日志本身成功，也不把日志失败转换成业务结果。关闭剖析只能在下次模块加载前通过环境变量控制；运行中改变 `OPENCODE_ACP_PROFILE` 不影响已缓存的 `enabled`。因此它不能单独表达 worker 保活、子树完成、断点恢复或派生关系，必须由宿主的运行时、邮箱和持久会话层承载。

## 源内测试与行为判据

源内未包含测试；同目录仅有 `agent.ts`、`config-option.ts`、`content.ts`、`directory.ts`、`error.ts`、`event.ts`、`permission.ts`、`profile.ts`、`service.ts`、`session.ts`、`tool.ts`、`usage.ts`，未发现针对 `profile.ts` 的测试。可独立验证的判据如下：

1. 未设置或设置为非 `"1"` 时调用 `mark`、`duration` 不产生 stderr；设置为 `"1"` 后，`mark("x", {a: 1, b: undefined})` 只输出含 `x.mark`、`a=1` 且不含 `b=undefined` 的一行。
2. 启用时 `duration("x", t)` 的毫秒值等于 `Math.round(performance.now() - t)` 的采样结果；`measure` 的成功值、rejection 和同步异常均保持原样，同时各产生一条对应 `[acp-profile]` 行。
3. `ACPProfile` 命名空间能访问三个公开函数；不能从模块外直接访问 `write`。并发启动两个 `measure` 时，两者各有独立时长，不能要求日志先后与 Promise 完成顺序一致。

## zenpi Rust 映射

### 可直接复用的原语与现有落点

- **消息/邮箱**：`src/protocol.rs:L88-L109` 的 `MailboxRequest` 已覆盖 `Send`、`Receive`、`Acknowledge`、`Claim`、`Complete`，并在 `L544-L585` 限制 ID、文本、TTL 和分页；`src/session.rs:L3171-L3268` 的 `MailboxMessage`/`MailboxAction` 与 `L3574-L3950` 的 `SessionMailbox` 提供有界、带 digest、带序列、持久化 JSONL mailbox。DAG 节点间的 parent、grandparent、直接 sibling、直接 child 通信应复用该邮箱，不应为每种关系新造 transport。
- **事件**：`src/core.rs:L273-L325` 的 `AgentEvent` 可表达 turn、tool、provider、error；`src/runtime.rs:L158-L203` 的 `RuntimeEvent` 提供 `Accepted/Started/Queued/CancelRequested/Completed/Closed` 生命周期；`src/protocol.rs:L882-L918` 的 `StdioEvent` 提供带 `sequence`、`request_id`、`turn_id` 的 JSONL 事件封套。`src/headless.rs:L861-L1041` 的 `AsyncEventBuffer` 已按 provider/agent 分邮箱、按数量和字节限流，并优先保留 admission/tool 终态事件。
- **协议**：`src/protocol.rs` 是版本化 JSONL 边界（`PROTOCOL_VERSION` 为 2，`L20-L36`），`MailboxRequest`、`TreeRequest`（约 `L1161-L1210`）和 `OutputRequest` 可继续承载 DAG 控制，但应新增严格的 `WorkerRequest`/`WorkerEvent` 类型而不是把关系信息塞入自由文本。
- **会话父子关系**：`src/core.rs:L140-L175` 的 `Turn.parent_id` 只保证对话 turn 的父关系；`src/session.rs:L3274-L3359` 的 `LiveSessionRegistry` 管理 `session_id`、`owner_epoch`、workspace 与心跳；`L3361-L3488` 提供 claim/finish，`L3490-L3559` 可向原 sender 回信。这些是实现 worker 通信的基础，但当前没有显式的“grandparent/sibling/child DAG 节点”类型，不能把 `Turn.parent_id` 直接当作完整 worker 图。
- **取消与调度**：`src/runtime.rs:L49-L100` 的 `CancellationToken` 是可复用的协作取消原语；`BackgroundRunner` 在 `L236-L430` 提供有界 command/event channel、`try_submit`、`try_cancel` 和有界 shutdown grace。`src/headless.rs:L4467-L4636` 负责把异步请求接入 runner，`L6000-L6485` 等路径处理取消与 mailbox 命令。`src/core.rs:L3666-L3745` 的 `run_active_turn_cancelable_with_events` 还能在持久化最终答案前阻止晚到取消泄漏。
- **保活与租约**：`src/session.rs:L3274-L3355` 的 `register`/`heartbeat`/`active` 已能以 `owner_epoch + TTL` 判断在线；`src/core.rs:L2870-L2895` 的 blueprint worker lease 可 `renew`、`cancel`、`expire`。但 `LiveSessionRegistry::claim_next` 的注释明确它只 claim、不会启动 scheduler（`L3361-L3369`），所以必须由 headless/runtime 的宿主循环显式拉取并提交 worker。

### 面向 DAG 需求的最小 Rust 机制

建议新增 `src/worker_graph.rs`（或在现有 session-tree 所属模块中落点）并保持职责窄化：

1. 定义可序列化的 `WorkerNode { worker_id, session_id, parent_worker_id: Option<_>, state, generation, last_heartbeat_ms }` 与 `WorkerState::{Running, Waiting, Green, Failed, Closed}`；定义 `WorkerMessage { message_id, sender_worker_id, recipient_worker_id, kind, payload, ttl_ms }`。parent 链可推出 grandparent；共享 `parent_worker_id` 可推出直接 sibling；`parent_worker_id == current` 可推出直接 child，避免复制四套地址关系。
2. 暴露 `send_worker_message`、`claim_worker_message`、`complete_worker_message`，内部调用 `SessionMailbox::enqueue/list/update` 或 `LiveSessionRegistry::{claim_next,finish_claim_with_reply}`。路由策略是：parent/grandparent/sibling/child 都转成明确 `recipient_session_id`；回复使用 `finish_claim_with_reply` 的幂等 request ID。所有消息保留 TTL、digest、owner epoch 和同 workspace 校验。
3. 暴露 `heartbeat_worker` 与 `reconcile_worker`。前者调用 registry heartbeat，并可配合 core 的 lease renew；后者从持久化节点状态计算 `node_green && all_descendants_green`。只有该递归谓词为真才追加 `worker_closed` 并允许 close；任何 child/grandchild 为 running、waiting、failed、过期未结算或缺少终态证据时，当前节点必须保持 `Waiting`/`Running`，不能把本节点自身成功误判为整棵子树成功。
4. 暴露 `derive_worker(parent, work_item)`：以幂等 key 写入 `worker_spawned`/child relation，再通过 `BackgroundRunner::try_submit` 启动新工作；提交失败时保留 durable spawn intent，后续 reconcile 重试，不能丢失派生请求。新 worker 应有独立 `session_id` 或明确 owner epoch，启动消息进入 child mailbox；派生动作与 close 判定必须避免重复（以 worker_id、generation、message_id 去重）。
5. `src/headless.rs` 是宿主落点：在现有 `run_async_stdio`/`BackgroundRunner` 事件循环旁增加周期性 heartbeat、mailbox claim、reconcile 和 spawn admission；将 worker 状态变化编码为 `StdioEvent`，diagnostic timing 仍写 stderr，不能污染 stdout JSONL。`src/core.rs` 负责单 worker 的 turn、工具和 lease 边界；`src/session.rs` 负责 durable graph/邮箱事件；`src/runtime.rs` 负责执行和取消；`src/protocol.rs` 负责有界请求/事件校验。

### 与指定模块的差异清单与可验证落点

- `src/headless.rs` 已有请求级 event mailbox 与邮箱命令路由（约 `L2066-L2112`、`L6392-L6485`），但没有递归 DAG close gate；验证应覆盖父节点成功、child 成功、grandchild 成功后才出现 `worker_closed`，缺任一绿状态时只能出现保活/等待事件。
- `src/core.rs` 的 `AgentPhase::Closed`（`L273-L279`）是单个 agent 生命周期，不等价于“自身及全部 descendants 绿色”；应新增 worker graph gate，不能改写单 agent close 的含义。`Turn.parent_id` 也只能复用为上下文关联，不能替代 worker relation。
- `src/session.rs` 已有 append-only journal、mailbox digest/sequence/TTL、owner epoch 和 reply，但 mailbox claim 是显式动作且过期消息不会隐式重试（`L3570-L3950`）；worker reconcile 必须读取 durable `worker_spawned/worker_state/worker_closed` 事件，处理崩溃恢复和重复投递，而不是只看内存 registry。
- `src/tool_runtime.rs:L157-L176` 的 `execute_tool_batch` 要求调用方提供取消谓词和准备回调，且不会自行 retry/detach；它适合 worker 内部工具批次，但新派生与子树聚合应留在 worker graph/headless，不要塞进工具执行器。
- `src/runtime.rs` 已有有界队列、取消、关闭 grace；但取消不是 rollback，非协作任务可能在 detach 后短暂继续（`L158-L169`、`L328-L345`）。因此 `worker_closed` 必须等待 durable settlement，不得仅凭 `RuntimeEvent::Completed` 或 runner shutdown 认定 descendants 已结束。
- `src/protocol.rs` 的 mailbox/tree/action 校验可复用，但要增加 `WorkerRequest` 的关系方向白名单、最大 payload、generation、correlation 和 close 证据；验证未知字段、错误 recipient、过大 TTL/payload、重复 message ID、错误父关系均 fail closed。
- `src/approval.rs` 已提供带取消 epoch 的审批等待与 durable decision 语义（例如 `L133-L191`、`L409-L446`、`L531-L564`）；worker 派生若触及工具副作用，应复用 approval coordinator 的取消 epoch、持久化前置和不得重复批准原则，而不是由 profiling helper 绕过审批。
- `src/providers/**` 仍是 provider/route/capability 层；`src/backend.rs:L417-L496` 的 `BackendError` 区分 `Cancelled`、`DeadlineExceeded` 与可重试 transport/HTTP，`L1689-L1829` 负责受控请求与 bounded retry。DAG 层只能把 provider 结果映射为 worker 消息/状态，不能把 provider retry 当作 worker 派生；provider event 经 `AgentEvent::Provider`/headless buffer 上送，provider adapter 不应知道 sibling 或 grandparent。
- 对 `profile.ts` 本身的 Rust 对照，建议新增 `src/acp_profile.rs`：`enabled` 对照一次性读取 `OPENCODE_ACP_PROFILE == "1"`，`started` 用 `Instant`，实现 `mark`、`duration`、`measure` 对应函数，使用 stderr `eprintln!` 并过滤 `None` 字段。它只能测量 `headless` 的 claim/spawn/reconcile、`runtime` 的 job、`session` 的 mailbox 操作，不承担状态迁移；可用 `Result<T, E>`/RAII guard 保证失败路径记录，但要保持源行为“业务错误原样传播”。

## 未决问题

无法从 `profile.ts` 确认的点：调用方是否要求字段可被机器无歧义解析、是否允许重复 `name`、是否需要采样/日志轮转、以及 ACP profile 行是否有外部收集器。这些都不影响源内已确认的开关、计时、字段过滤、失败路径和 stderr 副作用。对于 zenpi DAG，现有源码也未定义 worker 节点终态/全 descendants green 的正式 schema；上文将其列为必须新增并通过持久化事件与幂等测试验证的设计，而不是声称源文件已经提供。

# `opencode/packages/opencode/src/acp` 目录级学习汇总

> learn_mode：`understand`。本文汇总 12 个 ACP 源文件及其逐文件笔记。用户给出的 `/Users/wangweiyang/GitHub/Docs/learn/...` 路径在当前工作区不存在；实际笔记位于 `zenpi/Docs/learn/agent_comms/files/opencode/packages/opencode/src/acp/*_learn.md`，源码位于 `/Users/wangweiyang/GitHub/opencode/packages/opencode/src/acp`。

## 目录职责

该目录是 OpenCode 的 Agent Client Protocol 适配层：把 ACP 的请求、内容块、工具状态、权限询问、会话配置和事件流转换为 OpenCode SDK 的 session/provider/command 操作，再把结果转换回 `AgentSideConnection.sessionUpdate`、`RequestError` 与 ACP response。它采用 Effect 的 `Context.Service`、`Layer`、`Effect` 和若干进程内 `Map`/`Ref`，同时由 `service.ts` 组合会话、目录、事件、usage 与权限服务。该目录有两类代码：`content.ts`、`config-option.ts`、`tool.ts`、`usage.ts`、`error.ts`、`profile.ts` 等纯转换/边界模块，以及 `service.ts`、`session.ts`、`directory.ts`、`event.ts`、`permission.ts` 等带内存状态、SDK 调用或连接副作用的服务模块。它本身没有 zenpi 所需的 DAG 拓扑和“全子树绿色才关闭”规则；`forkSession` 的父子关系也主要由 backing SDK 隐含，未在 ACP service 中持久化显式 `parentSessionId`。

## 模块清单

- `agent.ts`：最薄的 ACP façade；导出 `init`、`Agent`，把 `initialize`、`newSession`、`prompt`、`cancel`、`closeSession`、`unstable_forkSession` 等方法逐一委托给 `ACPService.Interface`，并在 `run` 中用 `Effect.runPromise` 将 `ACPService.Error` 映射成 `RequestError`。
- `config-option.ts`：纯函数配置选择器；导出 `DEFAULT_VARIANT_VALUE`、`ConfigOptionModel`/`ConfigOptionProvider`/`ConfigOptionMode`/`ModelSelection`、`buildModelSelectOption`、`buildEffortSelectOption`、`buildModeSelectOption`、`buildConfigOptions`、`parseModelSelection`、`formatCurrentModelId`，负责 model/variant/mode 的展示、解析和确定性回退。
- `content.ts`：ACP `ContentBlock` 与 OpenCode `PromptPart`/`ReplayPart`/`ContentChunk` 的无 I/O 转换；导出 `promptContentToParts`、`contentBlockToParts`、`partsToContentChunks`、`partToContentChunks`、`uriToFilePart` 等，处理 text、image、file、resource、`data:`/`file:`/`zed:` URI 和 audience 标记。
- `directory.ts`：按 `directory` 作用域生成目录快照；导出 `ModelOption`、`ModeOption`、`Snapshot`、`Loader`、`Service`、`build`、`variants`、`loaderLayer`、`loaderNode`、`node`，并用 `SynchronizedRef<Map<...>>` 与 `Effect.cached` 共享同一目录的加载结果。
- `error.ts`：ACP typed error 定义及协议错误映射；导出九类 `Schema.TaggedErrorClass`、闭合 `Error` 联合、`toRequestError` 和 `fromUnknownDefect`，把会话/配置/模型/认证/方法/服务失败映射到 invalid params、auth required、method not found、internal error。
- `event.ts`：全局事件订阅、重连、历史回放和增量 `sessionUpdate`；导出 `start`、`Subscription`，维护 `AbortController`、连接等待器、按 session 的 idle waiter、工具起始标记和 bash 输出快照，处理 message/tool/permission 事件。
- `permission.ts`：权限询问与编辑预览适配；导出 `Handler` 及 `ACPPermission` 命名空间，按 `sessionID` 串行排队 `once`/`always`/`reject` 决策，构造工具调用和 diff content，缺 UI、取消、未知选项或失败时 fail-closed 为 reject。
- `profile.ts`：轻量性能观测；导出 `mark`、`duration`、`measure` 和 `ACPProfile`，仅当模块加载时 `OPENCODE_ACP_PROFILE === "1"` 才用 `performance.now()` 计时并向 stderr 输出，不改变业务结果。
- `service.ts`：目录核心编排服务；导出 `AuthMethodID`、`Interface`、`Service`、`make`，实现 ACP initialize/auth/session create/load/list/resume/close/fork、配置修改、prompt/slash command、cancel、MCP 注册去重、usage update、历史回放和 SDK 错误归一化。
- `session.ts`：进程内 ACP session 状态表；导出 `SelectedModel`、`KnownMessagePartMetadata`、`Info`、`StoreInput`、`Interface`、`Service`，以 Effect `Ref<Map<string, Info>>` 保存 cwd、model、variant、mode、MCP 和 part metadata，提供 `create/load/get/tryGet/remove` 与更新操作。
- `tool.ts`：工具状态到 ACP `ToolCall`/`ToolCallUpdate` 的纯投影；导出工具状态类型、`toToolKind`、`toLocations`、`pendingToolCall`、`runningToolUpdate`、`duplicateRunningToolUpdate`、`completedToolUpdate`、`errorToolUpdate`、`imageContents` 等，不执行工具也不负责重试。
- `usage.ts`：usage 通知服务；导出 `buildUsage`、`contextTokens`、`latestAssistantMessage`、`totalSessionCost`、`findContextLimit`、`MessageLoader`、`ContextLimitLoader`、`Service` 和 `sendUpdate`，从最新 assistant message 计算 token/cost，并缓存 directory/provider/model 的 context limit 后发送 `usage_update`。

所有文件末尾的 `export * as ACP...`、`Directory`、`ACPPermission`、`ACPProfile`、`ACPSession`、`UsageService` 等只是命名空间重导出，不会创建第二份状态。

## 运行时数据流与控制流

入口是 `ACP.init({ sdk })`：`agent.ts` 的 `create(connection)` 调用 `ACPService.make({ sdk, connection })`，服务再装配 `ACPSession`、`Directory`、`UsageService` 与可选 `ACPEvent.start`。请求控制流大致为：

1. `initialize` 返回协议能力和认证方法；`authenticate` 只接受 `AuthMethodID = "opencode-login"`，不在本层验证凭据。
2. `newSession` 先由 `directoryService.get(cwd)` 构造 provider/model/mode/command 快照，再调用 `sdk.session.create`，写入 `ACPSession`，注册 MCP，异步发布 `available_commands_update`，最后用 `buildConfigOptions` 返回模型配置。
3. `loadSession`/`resumeSession`/`forkSession` 读取 backing session 和消息，按 durable model、历史消息、当前目录能力、默认值的顺序恢复 model/variant/mode；`loadSession` 还调用 `replayMessages`。`forkSession` 创建新 backing id，但本目录不落显式 DAG parent edge。
4. `prompt` 先经 `content.ts` 把 ACP content 转成 prompt parts，并检测 slash command；普通命令调用 `sdk.session.prompt`，已知命令调用 `sdk.session.command`，`/compact` 调 `summarize`。有事件订阅时由 `ACPEvent.runUntilIdle` 等待同一 session 的 idle，然后发送 usage update、将 assistant 结果映射为 `end_turn` 或其他 `stopReason`。
5. `event.ts` 订阅 `sdk.global.event({ signal })`；消费循环按事件类型分派，实时记录 part metadata，补齐未知 part，发送 `agent_message_chunk`/`agent_thought_chunk`/`tool_call_update`，并把 `permission.asked` 交给 `permission.ts`。重连会等待 1 秒再订阅，但不是事件游标重放。
6. `tool.ts` 只把 pending/running/completed/error 输入做成 ACP 快照；`usage.ts` 读取消息和 provider context limit 后发送 `usage_update`；`agent.ts` 的 `run` 是最终 Promise/错误边界。

## 错误与取消语义

`error.ts` 的九类错误是稳定的 typed error 边界：`SessionNotFoundError`、配置/模型/effort/mode 错误到 invalid params，`AuthRequiredError` 到 auth-required，`UnsupportedOperationError` 到 method-not-found，`ServiceFailureError` 到 internal error。`fromUnknownDefect` 不读取原始异常，避免 stack、secret、refresh token 泄露。核心创建、恢复、配置验证、prompt 失败会传播；事件消费、消息回放单项错误、usage 读取、配置 update、MCP 单项注册、best-effort `abort` 与部分权限回复错误会被吞掉或降级，因此不能把“没有 ACP 通知”当成工作完成。

取消主要是合作式的：`service.cancel` 和 `closeSession` 调 `sdk.session.abort`；cancel 保留本地 session，close 先移除本地 session，再 best-effort abort。`event.ts` 用 `AbortController` 停止全局流、拒绝连接/idle waiter；`runUntilIdle` 没有 timeout，断连会以 `ACP event stream disconnected` 失败。`permission.ts` 同一 session 的权限请求严格 FIFO，不同 session 可并行，用户取消或 UI 缺失默认 reject。纯函数模块没有取消、锁、重试或持久化。`tool_runtime`/provider 的真实副作用边界不由 ACP 文件回滚，未知结果必须在 zenpi 中保留为非绿色状态。

## zenpi Rust 映射建议

### 可复用原语与现状

- 消息/邮箱优先复用 `src/session.rs::SessionMailbox`、`MailboxMessage`、`MailboxAction` 与 `src/protocol.rs::MailboxRequest::{Send, Receive, Acknowledge, Claim, Complete}`；它们已有 sender/recipient、request id、digest、TTL、sequence、claim token、`Queued -> Claimed -> Succeeded/Failed` 状态和 `finish_claim_with_reply`。`src/headless.rs::execute_mailbox` 是现有 JSONL mailbox 入口，但当前只做显式 mailbox 操作，不自动启动 worker。
- 实时事件复用 `src/core.rs::AgentEvent`（turn、tool、provider、error、handoff）、`src/runtime.rs::RuntimeEvent`（`Accepted`、`Started`、`Queued`、`CancelRequested`、`Completed`、`Closed`）和 `src/protocol.rs::StdioEvent`。可靠命令进入 durable mailbox/journal，普通进度进入 bounded event buffer，不能把 UI 事件流当作可靠队列。
- 当前 zenpi 已有 `src/dag.rs::DagStore`：文件锁 JSON store、`DagNode { parent, children, status, worker_heartbeat_ms }`、`DagRelation::{Parent, Grandparent, Sibling, Child, All}`、`DagMessage`、`send`、`claim_inbox`、`ack`、`wait_for_message`、`touch_worker`、`can_close`、`spawn_worker`；`src/tools.rs` 已注册 `dag_status`、`dag_send`、`dag_recv`、`dag_finish`、`dag_spawn`。`can_close` 会递归检查所有 descendants，不仅是两层 child/grandchild。
- 这个现状与 ACP 的差别是：`DagStore` 自有 JSON mailbox，不是 `SessionMailbox`；节点状态只有 `open/green/red`，没有 durable `Closed/Waiting/UnknownOutcome`、generation、lease 或 operation intent；`spawn_worker` 是显式工具动作，尚未与 `Agent::try_close`、`BackgroundRunner` 的 job terminal 或 SessionStore journal 组成一个原子 DAG supervisor。

### 会话父子关系与四类通信

建议保留执行图和 transcript 树的分层：`src/core.rs::Turn.parent_id` 只表达一次对话 turn 的父项；`src/session_tree.rs` 只表达 transcript 分支；worker 图应以可恢复的 `DagNodeRecord` 表达真实关系，例如 `{ node_id, session_id, parent_id, children, state, generation, lease_id, last_heartbeat_ms }`，并在 `src/session.rs` 或 `src/dag.rs` 中成为唯一写者。每个 worker session 绑定一个 `node_id`，`parent` 是 `parent_id`，`grandparent` 沿父指针上溯一跳，直接 sibling 是同一 parent 的 `children` 去除自身，直接 child 是 `children` 一跳；所有关系均由服务端图快照解析，不信任 worker 自报的 relation 或任意 session id。发送还需验证同一 workspace、节点存在、无环、TTL/大小边界和当前 owner epoch。

最小消息接口可以是 `send(node, relation, payload, request_id)`、`claim(node, digest)`、`complete(node, digest, outcome)`：relation 解析为 recipient 后落到 `SessionMailbox`；若保留 `DagStore`，应至少把 `DagMessage` 与 session id、request id、generation、lease/correlation 关联起来，并为重启恢复提供映射，避免两套邮箱产生不一致。parent、grandparent、直接 sibling、直接 child 均使用同一消息原语；sibling 只能交换工作/状态，不能替另一个节点 close 或 claim。

### 保活、派生、取消和 close 的最小闭环

1. **admit**：父节点先在 `src/session.rs` 的 journal/store 中写入唯一 `derive_intent`，建立 child relation、`generation`、`session_id`、`lease_id`、policy/model digest；只有持久化成功后才提交 worker。
2. **run/heartbeat**：`src/core.rs` 注册 `LiveSessionRegistry` owner；周期性调用 `heartbeat`，必要时调用 `renew_blueprint_worker`。`src/runtime.rs::BackgroundRunner::try_submit` 接受 `WorkerSpec`，`RuntimeEvent::Started/Completed` 更新节点状态。heartbeat 只证明 owner 活着，不等于 green。
3. **report**：worker 在 tool/provider/approval 的所有副作用都有 durable terminal evidence 后，才报告自身 `Green`；`Failed`、取消、过期 claim、缺失/未确认结果、shutdown detach 都是 `Waiting/UnknownOutcome`，不是 green。
4. **close gate**：`DagCoordinator::request_close` 在同一图状态/写入边界计算 `node.state == Green && descendants.iter().all(|n| n.state == Green)`。只有真时追加一次幂等 `dag_closed`，再调用 `Agent::try_close`、settle operation、释放 lease；不能直接把 `AgentPhase::Closed` 当作全子树完成。
5. **keep alive/derive**：判定为假时追加 `close_deferred`/`keep_alive`，保持 runner 和 owner heartbeat；若有新工作，写新的 `derive_intent`，建立新 child 或 generation，再 `try_submit`。队列满、spawn 失败或 journal 失败都保留可恢复 intent，不报告已运行；旧 worker 不被隐式复用为新 attempt，未知副作用不自动重试。
6. **cancel**：接入 `CancellationToken`，先发取消、等待 cooperative worker/join；取消不是 rollback，不能据此 close。`src/approval.rs::ApprovalCoordinator` 继续负责副作用许可，`src/tool_runtime.rs::execute_tool_batch` 继续负责 bounded calls、取消轮询和不 detach 的 join 语义。

### 指定 Rust 文件的落点

- `src/headless.rs`：作为 host/wire owner，扩展现有 `Command`/`StdioRequest`/`StdioEvent` dispatch，接入 `dag_status/send/recv/finish/spawn` 或结构化 `DagRequest`；在 mailbox claim、heartbeat、derive、close gate 后输出可重放事件和唯一 terminal response。当前 DAG 工具主要在 `src/tools.rs`，`headless.rs` 尚未直接拥有拓扑关闭事务，因此应避免 stdin 线程直接创建不可回收 worker。
- `src/core.rs`：新增 `DagCoordinator` 或在 `Agent` 上增加 `request_close/close_if_subtree_green/derive_worker/heartbeat`；复用 `AgentEvent`、`WorkerExecutionBinding`、`register_live_owner`、`renew_blueprint_worker`、`settle_blueprint_worker`。`try_close` 目前只关闭 extension/skill session 并把 `AgentPhase` 设为 `Closed`，必须由 DAG gate 先行阻挡。
- `src/session.rs`：保存 typed `DagNodeRecord`、parent/children 索引、generation/lease/status 事件和 derive intent；继续复用 `SessionMailbox` 的锁、digest、TTL、claim/complete/reply，以及 `LiveSessionRegistry::{register, heartbeat, active, claim_next, finish_claim}`。恢复时未完成 intent 或 claim 只能进入 recovery/keepalive，不能默认为 green。
- `src/runtime.rs`：复用 bounded `BackgroundRunner`、FIFO pending、`RuntimeEvent` 和 `CancellationToken`；将 `WorkerSpec`、`Heartbeat`、`Derive`、`CloseRequested` 作为 supervisor 控制面，处理 `QueueFull`、`Closed`、cancel race 和 shutdown grace。`Completed` 只表示一次 job 结束，不表示整个 descendants 子树完成。

配套边界：`src/protocol.rs` 负责 relation、node/session id、generation、correlation、TTL、payload 与未知字段的严格校验；`src/approval.rs` 负责 side-effect approval；`src/tool_runtime.rs` 负责工具批次；`src/providers/**` 只负责 provider route/capability/stream，不拥有 DAG 拓扑。实现顺序宜为 protocol → session/dag projection → core close/derive coordinator → runtime runner → headless routing，保持 ACP 的“薄 façade + service 错误边界”思想。

## 未决问题

1. zenpi 是继续维护现有 `DagStore`，还是将 DAG 消息统一迁移到 `SessionMailbox`，尚未有唯一存储权威；两套 mailbox 并存会产生 claim、重放和状态聚合分叉。
2. `DagNode` 当前没有 `session_id`、`owner_epoch`、`generation`、`lease_id` 和显式 `Closed/Waiting/UnknownOutcome`；这些字段如何兼容既有 `dag.json` 及工具输出需要协议版本决策。
3. “全绿”应等同 `Green`，还是要求 tool/provider/approval/operation 均有 terminal receipt，以及失败后是否允许人工重试并复用 node id，需要产品层定义；安全默认是未知不绿、新 attempt 用新 generation。
4. `src/headless.rs` 的 DAG 控制面应继续走 `src/tools.rs` 的工具调用，还是增加一等 `DagRequest` JSONL 命令，涉及权限、回放和客户端兼容策略。
5. heartbeat TTL、derive 并发上限、子节点/后代遍历上限、消息顺序与 sibling 广播是否有严格保证，当前 ACP 源文件和 zenpi DAG 基础实现都没有完整规定。
6. ACP 的 `AgentSideConnection.sessionUpdate`、SDK `fork` 和 `Effect.promise` 的顺序/取消保证来自外部 SDK；zenpi 不能把这些 best-effort 行为直接当作 durable DAG completion，仍需以自身 journal、claim/complete 和 close receipt 为准。

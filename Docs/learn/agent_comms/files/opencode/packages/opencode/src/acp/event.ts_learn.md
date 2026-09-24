# AC-046 — opencode/packages/opencode/src/acp/event.ts

## 元信息

- source_id/item_id：opencode/packages/opencode/src/acp/event.ts / AC-046
- source_path：opencode/packages/opencode/src/acp/event.ts
- source_hash：bbbfe1e210ba7ddf7b76a8d7d829e6f7e44d4d714c79dce832681ced9314d9ca
- source_bytes：12434
- source_lines：421
- coverage：已按顺序读取完整文件，字节范围 1-12434，行范围 L1-L421（含导入、注释、类型、私有实现与末尾导出）。

## 完整行为复盘

### 类型、入口与生命周期

- Connection 是能力收窄后的 ACP 连接：必有异步 sessionUpdate，可选 requestPermission、writeTextFile（L24-L25）。GlobalEventEnvelope 的 payload 可缺省，GlobalEventStream.stream 是 AsyncIterable（L26-L31），所以消费循环必须允许空包和流结束。
- start(input) 创建 Subscription、立即调用 start() 并返回实例（L33-L37）。调用者可随后调用 stop()，返回对象同时承担后台订阅与等待器管理。
- Subscription 保存 AbortController（全局取消）、shellSnapshots（每个 callID 的 bash 输出快照）、toolStarts（已发出合成起始事件的调用）、连接等待器集合、按 sessionId 分组的 idle 等待器、ACPPermission.Handler，以及 connected/started 标志（L39-L47）。这些容器只在单个订阅实例内共享，事件处理按消费循环顺序串行执行。
- 构造函数保存 sdk、connection、session，并用同一输入创建权限处理器（L49-L57）。
- start() 幂等：第二次直接返回；第一次置 started=true，启动 run()，后台 Promise 的异常被吞掉；非 abort 错误也不会向调用方抛出（L59-L65）。
- stop() 调用 abort()，标记断连，解析并清空所有连接等待器（L67-L72）。它不会取消已发出的 sessionUpdate，也不清理 toolStarts/shellSnapshots。
- runUntilIdle<A>(sessionId, request) 先等待连接，再创建一次性 signal()，加入该会话 idle 等待集合，执行 request()，最后等待同一 sessionId 的 session.status=idle 通知并返回请求结果；finally 删除等待器并在空集合时删键（L74-L91）。请求异常原样传播；断连时等待器收到 Error("ACP event stream disconnected")；没有超时，可能无限等待。void waiter.promise.catch 防止请求完成前出现未处理拒绝。
- run() 反复调用 consume()；无论消费成功或失败都执行 disconnected()，未 abort 时等待 1000ms 后重连（L144-L150）。这是流级重试，不是单事件重放。

### 事件分派与消息重放

- handle(event) 按 event.type 分派（L93-L106）：session.status 仅在状态类型为 idle 时释放该会话等待器；permission.asked 交给 ACPPermission.Handler.handle；message.part.updated/delta 分别进入对应私有函数；未知类型静默返回。permission.handle 不被 await，若其同步部分失败会落入调用方的 Promise 错误处理。
- replayMessage(message) 只接受 info.role 为 assistant 或 user，其他角色直接返回（L108-L119）。assistant 使用 info.path?.cwd，缺失时回退 process.cwd()；user 不带 cwd。按 message.parts 原顺序先 recordFetchedPart，tool part 走 handleToolPart，其余走 replayContentPart，因此重放中的连接更新严格串行。
- replayContentPart 只处理 text、file、reasoning，其他 part 返回（L122-L124）。reasoning 映射 ACP agent_thought_chunk，user 的 text/file 映射 user_message_chunk，assistant 映射 agent_message_chunk（L125-L131）。partsToContentChunks 产生的每个 chunk 都调用 sessionUpdate；reasoning 的 messageId 使用 part.id，其他使用 message.info.id，并带 session ID 与 chunk 内容（L132-L142）。连接错误会中断当前重放调用。
- consume() 使用 sdk.global.event({signal: abort.signal}) 获取流并视为 GlobalEventStream；成功建立流后置 connected=true、解析并清空连接等待器（L152-L159）。for await 中 abort 立即返回、无 payload 的包跳过，每个有效 payload 调 handle，单事件错误被吞掉，后续事件继续（L160-L165）。
- waitUntilConnected() 在未连接时循环等待连接等待器；若先 abort，抛出 Error("ACP event subscription stopped")（L167-L172）。disconnected() 只有从 connected 状态切换时才动作：置 false，拒绝所有 idle 等待器后清空 map（L174-L182）。idle(sessionId) 只释放该 session 的等待器，先删 map 再逐个 resolve；无等待器是 no-op（L184-L189）。

### 元数据、增量与工具状态

- handlePartUpdated(event) 取 part，以 part.sessionID || event.properties.sessionID 确定会话；通过 session.tryGet 查找，不存在则静默返回（L191-L196）。找到后用 recordPartMetadata 记录 session/message/part 标识、part 类型；reasoning 强制 role=assistant，text 保存 ignored，tool 保存 callID，存在 metadata 字段则保存之（L197-L208）。tool part 随后进入 handleToolPart（L209-L212）。Effect 失败会向上抛出，但实时消费路径会在 consume 中吞掉。
- handlePartDelta(event) 先按 props.sessionID 找会话；未知会话返回（L214-L217）。读取已知 part 元数据；若 role/type 不完整则调用 fetchPartMetadata 回源补齐（L219-L230）。非 assistant 元数据、未知类型或非 field="text" 均不发 ACP 更新。assistant text 且未 ignored 时发 agent_message_chunk，messageId=messageID；reasoning text 发 agent_thought_chunk，messageId=partID；内容均为 {type:"text", text: delta}（L231-L259）。这保证 user 的 live delta 不重复成 ACP user 消息。
- fetchPartMetadata(sessionId,cwd,messageId,partId) 调 sdk.session.message，传 sessionID、messageID、directory 与 throwOnError:true；任何 SDK 错误转为 undefined，消息或目标 part 缺失也返回 undefined；找到时调用 recordFetchedPart（L261-L278）。若回源失败，后续每个 delta 仍可能再次尝试，因为没有失败缓存。
- recordFetchedPart 持久记录 SessionMessageResponse.info.role，text 的 ignored、tool 的 callID、可选 metadata，并返回 Effect.runPromise 的结果（L280-L293）。
- handleToolPart 先 toolStart，再按状态处理（L295-L339）：pending 删除该 call 的 shell 快照后返回；running 调 runningTool；completed/error 先 clearTool，再向连接发一次 tool_call_update，分别使用 completedToolUpdate 或 errorToolUpdate。终态更新包含 toolCallId、toolName、state、cwd，转换细节由 ./tool 决定。
- runningTool 仅接受 running state（L341-L343）。bash 调 shellOutputSnapshot 得到可选输出；若与上次快照相同，发 duplicateRunningToolUpdate（状态更新但不重复内容）并返回；不同则覆盖快照。随后总发 runningToolUpdate，携带 output（L344-L377）。非 bash 没有快照去重。
- toolStart 以 callID 去重：已在 toolStarts 时返回，否则先插入再发 tool_call，内容由 pendingToolCall 根据 cwd/state/tool 生成（L379-L394）。因此发送失败后 marker 仍存在，后续事件不会自动补发起始事件。clearTool 同时删除起始 marker 与 shell 快照（L396-L399）。
- signal() 构造一个 Promise，并返回外部可调用的 resolve/reject 包装器；初始函数为空，实际闭包在 Promise executor 中绑定（L402-L419）。它是一次性 rendezvous 原语，重复 resolve/reject 依 Promise 语义无效。
- export * as ACPEvent from "./event" 使同文件通过命名空间重新导出（L421）。

## 状态、取消、恢复与副作用

- 取消只由 AbortController 驱动：停止全局事件 SDK 流、让 consume 退出、唤醒连接等待器；没有逐请求取消 API，也没有把 abort 写入 session journal（L39-L43、L67-L72、L160-L165）。
- 断连会拒绝全部 idle 等待器，因此正在 runUntilIdle 的调用失败；run() 在 1 秒后重新订阅。已完成的 session/tool 更新不会自动回滚，也没有事件序号或幂等键。
- runUntilIdle 的完成判据是收到同一 sessionId 的 idle 状态事件，且该 idle 由事件消费顺序保证排在本轮更新之后（L74-L90）。连接断开优先于 idle 时走拒绝路径。
- 源码没有 timeout、指数退避、单事件 retry 或持久化重放队列；replayMessage 是调用方主动提供历史消息，实时订阅重连后不会自动重新拉取缺失消息。
- 外部副作用包括：SDK 全局 event/message 请求；ACPSession.recordPartMetadata 的持久化；向 ACP connection.sessionUpdate 发送文本、思考、工具调用/更新；权限处理器可能发起 ACP 权限请求（L93-L105、L197-L212、L261-L293、L309-L336）。所有实时连接更新由消费循环按事件顺序 await，但不同调用方并发调用 handle 时类本身没有锁。

## 源内测试与行为判据

同目录测试 /Users/wangweiyang/GitHub/opencode/packages/opencode/test/acp/event.test.ts 覆盖主要判据：L321-L353 验证按 session 隔离 text/reasoning delta；L355-L387 验证 reasoning 使用 part ID 作为 ACP thought 边界；L389-L432 验证重复 loadSession 不创建额外订阅；L434-L458 验证已知元数据不重复调用 sdk.session.message、未知 part 只回源一次；L460-L537 验证历史 tool 重放按序且单次更新失败后继续后续消息；L539-L553 验证未知 session 与 live user part 不产生重复 user chunk；L555-L621 验证合成 pending tool call、重放 running tool 不重复起始事件；L623-L670 验证 bash 快照去重以及 pending 后清除快照；L672-L749 验证 completed/error 的内容与 rawOutput；L751-L784 验证 image attachment 在 live/replay 都转换为 ACP image content。可独立验证命令为 bun test packages/opencode/test/acp/event.test.ts；若不运行测试，至少应断言上述 sessionUpdate 序列、session/message 调用计数、终态状态与 rawOutput 结构。

## zenpi Rust 映射

### 可复用通信原语与现有落点

- event.ts 的 Event 分派、sessionUpdate 与 per-session idle waiter 可映射到 src/core.rs 的 AgentEvent（TurnAccepted、AssistantMessage、ToolCall、ToolProgress、ToolResult、Provider、Error、Warning，L281-L325）和 src/headless.rs 的 AsyncEventBuffer。后者按请求保存 provider/agent 事件，按数量与字节上限丢弃并保留 admission/tool 生命周期事件（headless.rs:L814-L869,L940-L1061），比无界广播更适合 DAG。
- src/runtime.rs 的 BackgroundRunner 已提供有界 command/event channel、FIFO pending follow-up、JobId、Accepted/Started/Queued/CancelRequested/Completed/Closed（runtime.rs:L38-L40,L49-L99,L156-L199,L272-L365）。可把 ACP 的 runUntilIdle 变成 await_node_idle(node_id)，把 RuntimeEvent::Completed 作为一次 worker turn 结束，而不是 DAG 节点可关闭。
- src/session.rs 的 SessionMailbox/LiveSessionRegistry 是可复用的持久消息/邮箱原语：消息按 digest、sender/recipient、TTL 入队并经过 Queued -> Acknowledged/Claimed -> Succeeded/Failed 转换（session.rs:L3563-L3592,L3617-L3697,L3743-L3795）；注册、heartbeat、claim_next、finish_claim 绑定 owner_epoch（session.rs:L3285-L3438）。它可承载 parent、grandparent、直接 sibling、直接 child 的定址消息，避免进程内裸指针。
- src/session.rs::SessionStore 的 append-only append_event 与 ReconnectJournal 的 checksum/sequence WAL 可分别持久化 DAG 状态和断线事件（session.rs:L1205-L1225,L1908-L2021）。src/protocol.rs 已有 MailboxRequest、StdioEvent、StdioResponse 与严格字段校验（protocol.rs:L81-L110,L882-L923,L994-L1023），可扩展 DagMessage/DagStatus 而不另造无校验 JSON。
- src/approval.rs::ApprovalCoordinator 是等待/唤醒、一次性可见、首个响应获胜、持久化失败回撤、cancel_all 的条件变量 rendezvous（approval.rs:L126-L168,L194-L249,L294-L375,L409-L457）。它适合 DAG worker 的权限消息，但不能替代节点完成判定。
- src/tool_runtime.rs::execute_tool_batch 已有有界批量、准备阶段、取消传播、并行 worker join、未确定副作用阻止后续调用（tool_runtime.rs:L157-L176,L219-L249,L252-L346），可复用为新派生 worker 的工具执行边界；provider 适配器通过 backend::ProviderEvent 的 text/reasoning/tool delta 与取消检查产生流式事件（backend.rs:L238-L286,L384-L413），src/providers/** 只应继续负责协议/流解析，不直接操纵 DAG 图。

### DAG 会话父子关系与最小机制

建议新增 src/dag.rs（或将持久结构放在 src/session_tree.rs 并由 session.rs 作为唯一写者）：

1. 定义 DagNodeId、DagEdge { parent: DagNodeId, child: DagNodeId }、DagNodeState::{Queued,Running,Waiting,Green,Failed,Closed}、DagRelation::{Parent,Grandparent,Sibling,Child,Grandchild} 与 DagMessage { id, from, to, relation, correlation_id, payload, created_at_ms, ttl_ms }。所有 ID 复用 protocol::validate_queue_id；写入时禁止环、限制节点/后代遍历数量并记录版本。
2. 在 SessionStore 增加 typed append_dag_event/dag_snapshot，事件至少包含 node_id、parent_id、state、descendant_digest；不要让通用 append_event 绕过图所有权。每个 worker session 的 Turn.parent_id（core.rs:L140-L204）可保存会话内 turn 链，但跨 worker 的真实 DAG 边必须使用上述显式 edge；runtime::InputBoundary::context_parent（runtime.rs:L765-L830）只能作为当前模型 turn 的上下文引用。
3. 在 protocol.rs 增加 Command::DagMessage/DagStatus/DagClose/DagSpawn 或扩展 MailboxRequest：验证 sender_session_id、recipient_session_id、node_id、correlation ID、payload 上限和 relation。路由实现调用 SessionMailbox::enqueue；recipient owner 用 LiveSessionRegistry::claim_next，处理后 finish_claim_with_reply，从而覆盖 parent、grandparent、直接 sibling/child 的双向请求/回复。
4. 在 core.rs 增加 DagWorkerBinding（可复用 WorkerExecutionBinding 的 goal_id/item_id/lease_id/policy_digest/expires_at_ms，core.rs:L37-L81）和 NodeCoordinator。NodeCoordinator::on_event 把 AgentEvent/provider/tool 终态映射为节点状态；close_request(node) 必须读取自身、所有 child 和 grandchild 的状态，只有全部 Green 才追加 Closed，否则追加 Waiting 并保活。
5. 在 runtime.rs 增加最小派生接口 spawn_child(parent_node, work) -> JobId：先在 SessionStore 写入不可重复的 dispatch intent，再 BackgroundRunner::try_submit；收到 Accepted/Started 写边和 lease，收到 Completed 只结束该 turn。等待 descendants 时不要调用 CancellationToken::cancel；用 LiveSessionRegistry::heartbeat 保活，并周期性 claim_next。需要新工作时从 Waiting 节点重新 try_submit，而不是把已关闭节点复活。
6. 直接 sibling 通信由图中共享 parent 的邻接查询产生目标列表；grandparent/grandchild 通过有界 BFS 解析，不允许任意 session ID 越权。每条消息带 correlation ID、sender/recipient 与 owner epoch，重复 enqueue/complete 必须返回同一 durable result，不能隐式重试已 claimed 的副作用。
7. headless.rs 负责把 DAG 生命周期投影为 JSONL StdioEvent，沿用 per-request AsyncEventBuffer 的界限与 dropped 计数（headless.rs:L833-L869,L957-L1021）；src/approval.rs 在 DagWorker side-effect 前执行 policy/lease 校验。src/providers/** 只产出 provider delta/error，core.rs 才能决定 Green/Failed/Waiting。

### 与当前实现的差异清单（可执行、可验证）

- 已有 Turn.parent_id 是线性 turn 关系，没有 child/grandchild 状态聚合；补 DagEdge 后测试环检测、跨 session sibling 路由和 descendants 全绿判定。
- 已有 mailbox 有 TTL/claim/owner epoch，但没有 DAG relation 与 node correlation；扩展协议后测试 parent->child、child->parent 回复及过期消息拒绝。
- 已有 BackgroundRunner 的 Completed 表示 job 终态，没有“节点等待后代仍 Running”；加入 Waiting/heartbeat 适配器并测试 active job 完成后节点不提前 Closed。
- 已有 cancellation 是合作式且 shutdown 有界，不能用于强制停止外部副作用（runtime.rs:L49-L99,L383-L421）；DAG 保活路径必须保持 token 未取消，并测试 shutdown 与 node close 的竞态。
- 已有工具执行器对不确定副作用停止批次但不重试（tool_runtime.rs:L236-L249）；派生新 worker 前必须持久 dispatch intent，测试 journal 写入失败时不启动 worker。
- 已有 provider retry/streaming 位于 backend/providers，不能把网络重试当作 DAG 消息重试；测试 provider delta 断流只产生可审计 error/event，不重复执行已 claim 工具。
- 建议验证命令：cargo test runtime、cargo test session、cargo test protocol、cargo test approval，并新增 cargo test dag_parent_grandparent_sibling_child_routing、cargo test dag_close_requires_all_descendants_green、cargo test dag_waiting_heartbeat_spawns_replacement。

## 未决问题

- ACPPermission.Handler（src/acp/permission.ts）的具体权限请求副作用不在本文件，无法从 event.ts 确认它是否持久化。
- partsToContentChunks、shellOutputSnapshot、completedToolUpdate 等转换函数的字段细节在依赖文件中，本文只能确认调用时机与分支。
- sdk.global.event 断流后服务端是否保证事件不丢失、是否支持游标重连，源文件未提供协议保证。
- zenpi 是否要把 DAG 图与现有 session_tree 合并，或单独建立 src/dag.rs，需结合产品的跨 session 生命周期约束决定。


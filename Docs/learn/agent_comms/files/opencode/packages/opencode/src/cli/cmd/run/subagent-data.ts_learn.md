# AC-053 — opencode/packages/opencode/src/cli/cmd/run/subagent-data.ts

source_id/item_id: AC-053
source_path: opencode/packages/opencode/src/cli/cmd/run/subagent-data.ts
source_hash: d5f7d3dec7e743679e9b06504cc92381242a4d50842d040806fc2bcbd10dacbb
source_bytes: 22129
source_lines: 876
coverage: 已按源文件顺序读取全部字节，字节范围 1-22129；行范围 L1-L876，含注释、类型、导出符号与私有函数。

## 完整行为复盘

### 数据模型、常量与初始化

- 模块从 SDK 引入 `Event`、`Message`、`Part`、`PermissionRequest`、`QuestionRequest`、`ToolPart`，并复用 `session-data.ts` 的 `createSessionData`、`bootstrapSessionData`、`reduceSessionData`、`formatError`；UI 输出类型来自 `types.ts`（L1-L10）。这说明本文件是“父会话 task 事件 -> 子会话页签/详情快照”的内存投影层，不是任务执行器。
- `SUBAGENT_BOOTSTRAP_LIMIT=200`、`SUBAGENT_CALL_BOOTSTRAP_LIMIT=80` 是导出的上层限制常量；私有保留上限为提交帧 80、单次 call 32、role 32、错误 16、echo 8（L12-L19）。本文件本身只直接使用后五个私有上限；两个导出 bootstrap 常量供调用方配置历史拉取。
- `SessionMessage` 只含 `parts`；`BootstrapChildMessage` 另带完整 `Message`；`Frame` 是稳定 `key` 与 `StreamCommit`；`DetailState` 绑定 `sessionID`、`SessionData` 和帧数组；导出 `SubagentData` 是两个按 session ID 索引的 `Map`：`tabs` 与 `details`；`BootstrapSubagentInput` 接受父消息、已知 child、权限和问题队列（L21-L51）。
- `createDetail(sessionID)` 创建包含 `includeUserText:true` 的 `SessionData` 与空帧；`ensureDetail` 命中则复用同一对象，否则插入并返回新对象（L53-L72）。这是惰性建详情，保证页签存在即可接收后续事件。

### 相等性、队列与字段归一化私有函数

- `sameSubagentTab(a,b)` 对未定义或任一字段差异返回 false；完整比较 session/part/call、label/description/status/background/title、toolCalls、lastUpdatedAt（L74-L91）。它是所有“是否需要写 Map/触发 UI 更新”的幂等门。
- `sameQueue` 同时要求长度相同、同索引 `id` 相同且对象引用相同；`queueSnapshot` 复制权限/问题数组；`queueChanged` 用前后快照判断队列引用或次序变化（L93-L108）。因此内容原地变更但引用不变时，不能仅靠该函数识别。
- `sameCommit` 比较 `StreamCommit` 的 kind/text/phase/source/messageID/partID/tool/interrupted/toolState/toolError 全字段（L110-L123）。`text(value)` 只接受字符串，trim 后空串变为 `undefined`；`num(value)` 只接受有限 number（L125-L140）。
- `inputLabel` 按优先级取已 trim 的 `description`、`command`、`filePath`/`filepath`、`pattern`、`query`、`url`、`path`、`prompt`，没有可用值返回 `undefined`（L142-L184）。这既处理字段别名，也避免对象被隐式转字符串。
- `stateTitle` 只在 `part.state` 有 `title` 字段时读取并归一化；`callKey` 要求 `messageID` 和 `callID` 都存在，返回 `${messageID}:${callID}`，否则无 key（L186-L196）。

### 工具状态压缩与页签生成

- `compactToolState` 保留 pending 的 `input/raw`；running 的 `input/time` 及可选 `metadata/title`；completed 的 `input/output/title/metadata/time`；其余状态统一按 error 保留 `input/error/time` 和可选 metadata（L198-L235）。它缩小快照但不改变状态语义；未知状态会落入 error 分支。
- `recent` 将 iterable 展开并保留末尾 `limit` 个，负向起点用 `Math.max(0,...)`；`copyMap` 只复制 key 在 `keep` 集合内的项（L237-L252）。
- `compactToolPart` 固定复制工具 part 的身份、session/message/call、tool，并将 state 压缩；metadata 存在才保留。`compactCommit` 无 part 时原样返回，有 part 时替换为压缩工具 part（L254-L276）。
- `stateUpdatedAt` 无 `time` 用当前 `Date.now()`；只有 start 用 start 或当前时间；有 end 用 end、否则 start、再否则当前时间（L278-L289）。所以缺时钟数据的 tab 排序会受读取时刻影响。
- `metadata` 优先读 `part.state.metadata[key]`，再回退 `part.metadata[key]`；`taskStatus` 将 completed 映射 completed，error 且 `interrupted===true` 或错误文本为 `Tool execution aborted` 映射 cancelled，其余 error 映射 error，pending/running 映射 running（L291-L309）。
- `taskTab` 将 `subagent_type` titlecase，默认 general；description 优先 input.description，再 state title，再 `inputLabel`，最终空串；填充 status、background、title、三种大小写兼容的 tool call 计数、更新时间（L311-L327）。`taskSessionID` 兼容 metadata 的 `sessionId`/`sessionID`（L329-L331）。
- `syncTaskTab` 只接受 `part.tool === "task"`、存在 child session ID，且当给定 child 集合时必须命中；同 tab 仍确保 detail 但返回 false；变化则写 tab、ensure detail、返回 true（L333-L356）。缺 session ID 的 task 不能成为页签。

### 帧键、合并、限额与阻塞

- `frameKey` 优先使用 part ID，其次 message ID，否则使用 kind/phase/text；键格式分别为 kind:part:phase、kind:message:phase 或 kind:phase:text（L358-L368）。同一 part 的 progress 更新会定位到同帧。
- `limitFrames` 仅在超过 80 帧时从头部 splice，保留末尾 80；`mergeLiveCommit` 对非 progress 或跨 phase 的提交采用“相同则旧对象、不同则新对象”，两者都是 progress 时合并字段并拼接 `current.text + next.text`，合并后仍相同则复用旧对象（L370-L398）。
- `appendCommits` 先对每个 commit 做 compact，再按 key 追加或合并；只有真实变化才标记 changed，变化后裁剪帧，返回是否变化（L400-L432）。输入顺序保持，重复事件可安全重放。
- `ensureBlockerTab` 对已有 tab 先确保 detail；非 running tab 不被 permission/question 重新打开；running tab 更新描述为 `Pending permission` 或 `Pending question`、保留旧 title 优先、刷新 `lastUpdatedAt`；无 tab 时创建 bootstrap ID/call ID、kind titlecase label、running tab（L434-L473）。
- `isAbortedAssistantMessage` 只认 assistant 且 error.name 为 `MessageAbortedError`；`cancelSubagentTab` 只把现存 running tab 改为 cancelled 并刷新时间，非 running/不存在返回 false（L475-L496）。

### 压缩、事件应用与 bootstrap

- `compactCallMap` 保留 `SessionData.call` 的最近 32 个 key，另强制保留权限请求和帧中引用的 tool call；`compactEchoMap` 保留当前 message IDs 与最近 8 个 echo；`compactIDs` 保留最近 96 个 ID（80+16）（L498-L525）。这些保留集合避免 UI 仍引用的 call 被清走。
- `compactDetail` 重建新的 `SessionData`：保留 announced、当前 permissions/questions；按 active part、帧 part、tools 计算 part 集合；按 active part 对应 message 和最近 32 个 role 计算 message 集合；再选择性复制 ids/tools/call/role/msg/part/text/sent/end/echo，最后替换 `detail.data`（L527-L555）。这是有界内存投影，不是持久化。
- `applyChildEvent` 先保存权限/问题队列快照，调用 `reduceSessionData`，追加其 commits，立即压缩 detail，返回“帧变化或队列变化”（L557-L575）。它只负责已存在 child 的实时事件。
- `bootstrapChildEvent` 同样调用 `reduceSessionData` 并追加 commits，但不做 detail 压缩；`bootstrapChildMessages` 按消息顺序先合成 `message.updated`，再按 part 顺序合成 `message.part.updated`，事件 ID 前缀分别为 `bootstrap:message:`/`bootstrap:part:`、part 时间为 0；最后压缩并返回是否变化（L577-L639）。这把 REST/历史消息重放成统一事件流。

### 导出函数逐一行为

- `listSubagentPermissions`、`listSubagentQuestions` 将所有 detail 的对应队列扁平化返回，不排序、不去重（L641-L651）。`createSubagentData` 返回空 `Map` 的新容器（L653-L658）。
- `snapshotDetail` 返回 sessionID 与按帧顺序的 commit；`listSubagentTabs` 先把 running 排在非 running 前，再按 `lastUpdatedAt` 降序；稳定性依赖 sort 对相等项保持原序（L660-L676）。`snapshotQueues` 对 permission/question 按 id 字典序排序；`snapshotState` 组合 tabs、details、排序后的队列（L678-L691）。
- `snapshotSubagentData` 输出全部 detail；`snapshotSelectedSubagentData` 只输出指定且存在的 detail，否则 details 为空对象，但两者都输出全量 tabs 与队列（L693-L707）。快照是新数组/对象投影，commit 内部引用来自帧。
- `bootstrapSubagentData` 先把 `children` 建成 id->child Map 和 Set；父消息的所有 tool part 仅通过 child Set 同步 task tab；权限/问题仅接受 child session；随后遍历已有 tab，确保 detail，按 session 过滤并按 id 排序后调用 `bootstrapSessionData`，压缩 detail，再以队列前后差返回 changed（L709-L760）。它会忽略非 child 的权限/问题和非 tool part；重复调用通过相等性/队列检测尽量幂等。
- `bootstrapSubagentCalls` 要求 session 已知且 messages 非空，否则 false；对该 detail 记录队列与 call 数，调用 `bootstrapSessionData` 写入消息/队列，再由 `bootstrapChildMessages` 重放 commits，返回 commit、call 数或队列任一变化（L762-L790）。未知 child 不会被隐式创建。
- `reduceSubagentData` 是实时入口：`message.part.updated` 若 part 属当前父 session，非 tool 直接 false，tool 仅同步父侧 task tab；其余支持的 sessionID 事件包括 message.updated、part.delta、permission/question asked/replied/rejected、session.error、session.status，并要求该 session 已知，否则 false（L792-L831）。assistant `MessageAbortedError` 先取消 tab；retry 状态追加 system/error/start 帧，message 为 retry attempt；带 error 的 session.error 用 `formatError` 追加去重键；其他事件进入 `applyChildEvent`，并与 cancelled 结果取或（L832-L876）。因此事件处理是同步、单线程式的可重放 reducer；没有锁、线程、网络等待或自动派生。

## 状态、取消、恢复与副作用

状态分两层：`tabs` 是每个 child session 的摘要状态，`details` 是有界的 `SessionData + commits` 检查器状态。`running`、`completed`、`error`、`cancelled` 只影响展示与排序，不代表执行器拥有停止能力（L295-L327、L667-L707）。权限/问题队列被保留并按 id 输出；阻塞事件会把 tab 变成 `running`，但完成/错误 tab 不会因旧 blocker 被重新打开（L434-L473）。

源文件没有 `AbortController`、timer、Promise、thread、retry loop 或 provider/tool 调用；也没有文件、数据库、网络写入。取消仅有两个输入：工具状态 error 且 metadata.interrupted 或错误文本 `Tool execution aborted`，以及 assistant message 的 `MessageAbortedError`；前者在 `taskStatus` 映射为 cancelled，后者在 reducer 中把运行中的页签改为 cancelled（L295-L309、L475-L496、L832-L835）。超时没有独立语义；重试只把 `session.status` 的 retry 事件转成一条 system error commit，不会重新提交任务（L836-L851）。

恢复/重放由调用者提供历史消息与事件：`bootstrapSubagentData` 恢复父侧 task tab 和 blocker，`bootstrapSubagentCalls` 将 child 消息合成为与实时 reducer 相同的事件序列（L709-L790）。恢复不是持久化：`compactDetail` 只在内存中裁剪 map/frames。所有副作用限于改变传入 `SubagentData` 的 Map、detail 和数组；函数没有克隆整个状态，也没有并发安全承诺。调用方必须串行投递事件；若多个线程并发修改同一 `Map` 会产生数据竞争/逻辑破坏（L40-L43、L400-L431）。

## 源内测试与行为判据

源文件自身未包含测试，但同目录对应测试为 `opencode/packages/opencode/test/cli/run/subagent-data.test.ts`，完整覆盖 547 行。可核对的行为判据如下：

- L213-L263：父 task part 只为列出的 child 建 tab；`Explore`、description、title、toolCalls 被正确投影；非 child permission/question 被过滤；details 初始 commits 为空。
- L265-L282：工具状态 `Tool execution aborted` + interrupted metadata 在 bootstrap 后成为 cancelled。
- L284-L418：child 的 user/assistant/message part、reasoning、tool 与 permission 事件可合成为可见 commits；`message.part.delta` 把 `hello` 与 ` world` 合为 `hello world`；permission metadata 保留 tool input。
- L420-L485：`bootstrapSubagentCalls` 按历史消息顺序重放 user、reasoning、text，且能返回 true；L487-L546：assistant `MessageAbortedError` 将运行中 tab 标记为 cancelled。

独立验证判据：构造一个 running 的 task tab 后重放相同事件两次，第二次不应新增 frame；构造 81 个不同 frame，应只保留最后 80；构造 progress 同 key 的两次提交，应得到拼接文本；向非 child session 投递 blocker 应没有 tabs/queues 变化；retry 只增加一条 `error/start/system` frame，不创建新 session。导出快照应满足 running 优先、同状态按 `lastUpdatedAt` 降序、queues 按 id 升序。

## zenpi Rust 映射

### 先划清可复用边界

源文件的可复用核心不是“subagent UI tab”，而是四个数据流习惯：稳定 session ID 关联、事件 reducer、有限队列/帧保留、可重放快照。它没有父/祖父/兄弟/子节点授权、DAG 完成判定、keepalive 或 worker spawn；`children` 只用于过滤父 task 产出的已知 child（L709-L737），不能直接满足 zenpi 的 DAG 编排。

1. **消息/邮箱原语**：优先复用 `src/protocol.rs` 的 `MailboxRequest::{Send,Receive,Acknowledge,Claim,Complete}`，其中 Send 带 `recipient_session_id`、`message_id`、正文和 TTL，Receive 用 sequence cursor 分页，Claim/Complete 提供所有权与结果（`src/protocol.rs:L81-L113`）。验证和 TTL/文本边界已在 `validate_mailbox` 中实现（`src/protocol.rs:L544-L590`）。它适合 parent、grandparent、直接 sibling、直接 child 的点对点控制/结果消息；但必须由 graph owner 证明 recipient 是允许的邻接点，不能把任意合法 `recipient_session_id` 当成拓扑授权。
2. **事件原语**：内部 worker 生命周期复用 `src/runtime.rs` 的 `RuntimeEvent::{Accepted,Started,Queued,CancelRequested,Completed,Rejected,Closed}` 与有界 channel；对外进度复用 `StdioEvent` 的 sequence、request_id、turn_id、event envelope（`src/runtime.rs:L171-L190`，`src/protocol.rs:L882-L918`）。建议将 `NodeEvent` 放进 `event` payload，保留 `node_id`、`parent_id`、`generation`、`kind`、`monotonic_seq`，让断线恢复只回放 durable journal cursor，不把 UI snapshot 当权威状态。
3. **阻塞/协议原语**：权限问题不要另造等待机制；`ApprovalCoordinator` 已是 worker-host rendezvous，worker 等待 bounded condition variable，host 可 drain/answer（`src/approval.rs:L126-L190`），并且持久化失败会 retract，避免未审计批准进入副作用（`src/approval.rs:L370-L390`）。`StdioRequest -> Command` 的 bounded validation、request ID 和 `Mailbox` 路由可作为 headless 控制面（`src/protocol.rs:L265-L267`、`L479-L484`）。

### 建议的 DAG 类型与会话父子关系

建议新增 `src/agent_graph.rs`（或将同等类型并入既有 `session_tree`，但不要把 DAG 语义埋进 UI reducer）：

```rust
type NodeId = String;
struct DagNode {
    node_id: NodeId,
    session_id: String,
    parent_id: Option<NodeId>,
    children: BTreeSet<NodeId>,
    status: NodeStatus,
    generation: u64,
    last_heartbeat_ms: u64,
}
enum NodeStatus { Running, Blocked, Red, Green, Closing, Closed, Alive }
```

`DagNode.parent_id` 表示直接 parent；祖父由 parent 链得到，直接 sibling 是 `parent.children - self`，直接 child 是 `children`，不允许把“任意同 workspace session”当邻居。`src/core.rs` 的 `Turn` 已有可校验的 `parent_id`，并在 `Turn::with_parent` 建立关系（`src/core.rs:L140-L175`），可作为消息/turn 级因果链；但它是 turn parent，不足以表达 node 的全部 descendants，必须另有 DAG 索引和持久记录。`SessionStore` 的 header 提供稳定 session ID（`src/session.rs:L35-L41`），因此建议 `DagNode` 同时记录 `session_id` 与 `node_id`，避免用展示 tab key 代替会话身份。

最小通信 API 建议为：`send_to(node, target, envelope) -> Result<MessageId>`、`receive(node, cursor) -> Page<NodeMessage>`、`claim/complete(message)`、`emit_node_event(event)`；`send_to` 先检查 target 是否是 parent、grandparent、direct sibling 或 direct child，再调用 mailbox。消息 envelope 至少包括 `source_node_id`、`target_node_id`、`relation`、`correlation_id`、`generation`、`kind`、payload；`relation` 必须由 graph owner 计算而不是由调用者自报。这样既复用现有 Mailbox sidecar，又明确覆盖四类关系。

### 保活、派生与 close 的最小机制

1. `NodeStatus::Green` 只表示节点自身工作已达成；`can_close(node)` 必须同时检查自身为 Green、所有直接 child 为 Green，并递归/汇总所有 grandchild descendants 为 Green，且没有未完成 mailbox、approval、tool operation 或新 work generation。只要自身或任一 descendant 为 Running/Blocked/Red，节点保持 `Alive`/`Closing`，不能发 Closed。
2. 在 `DagNode` 保存 `subtree_epoch`/`generation` 与 `last_heartbeat_ms`。每次收到 child 状态、新消息或新工作都增加 generation；close check 读取一次一致性快照，若 check 期间 generation 变化则拒绝 close 并重新调度。避免“检查全绿后又来了 child work”的竞态。
3. keepalive 的最小实现可复用 `Agent::register_live_owner`、`heartbeat_live_owner`、`unregister_live_owner` 和现有 mailbox claim/finish（`src/core.rs:L820-L884`）。每个活跃 node 定期发送 heartbeat event；TTL 过期只能标记 `Alive`/`Red` 或触发恢复候选，不能把未知结果误判为 Green。
4. 派生不应藏在 `subagent-data.ts` 式 reducer 中。复用 `BackgroundRunner` 的 `try_submit`、bounded pending queue、`CancellationToken` 与 `RuntimeEvent`（`src/runtime.rs:L49-L93`、`L272-L368`）；请求体应是 `SpawnNode { parent, work_id, generation, worker_binding }`。submit 成功后先持久化 `node_spawn_requested`，再创建 child session/worker；重复 `work_id+generation` 必须幂等。当前 `BackgroundRunner` 是单 active job、FIFO follow-up 模型（`src/runtime.rs:L544-L577`），若 DAG 需要并行 sibling，必须增加受治理的多 worker pool 或每 node 一个 runner，不能假定当前一个 runner 会并行。
5. “新工作到来而未全绿”路径：graph owner 追加 `work_received`，保持 parent Alive，创建新的 `generation`，通过 `BackgroundRunner` 派生/复用一个 child worker，发 `node_wakeup` mailbox；只有该 generation 及其 descendants 全绿后才允许再次 close。取消只停止协作式工作，不撤销已发生副作用。

### 现有模块落点与差异清单

- **`src/headless.rs`**：已有 mailbox 命令要求 session owner，发送时只接受同目录中唯一匹配的干净 journal，claim/complete 还经过 live owner（`src/headless.rs:L8941-L9093`）；这正是 mailbox 访问层的落点。新增 `graph_send`/`graph_status`/`graph_close` 可先作为 `Command::Mailbox` 的受限 envelope 或新增 `Command::Tree` 分支，在 headless admission 处把 node relation 校验放到 owner，而不是信任 stdin。现有异步行处理已把 runner、job map、request->job map 集中管理（`src/headless.rs:L5322-L5349`），可在同一处接入 `SpawnNode` 与 node event 回放。注意 shutdown 会先 emergency cancel，再等 `RuntimeEvent::Closed` 才确认之前请求终结（`src/headless.rs:L6473-L6485`），DAG close 也应区分“请求 close”与“所有 descendants 已终结”。
- **`src/core.rs`**：`Turn.parent_id` 可承载 turn 因果关系；`Agent` 的 live owner/mailbox API 可承载心跳、claim、reply；`admit_blueprint_worker` 已展示“校验 binding -> 安装 gate -> durable lease/reservation -> 写 admission event，失败回滚”的原子准入模式（`src/core.rs:L1578-L1697`）。建议新增 `Agent::register_dag_node`、`Agent::record_node_heartbeat`、`Agent::evaluate_close`、`Agent::spawn_dag_worker`，将 close/派生和治理绑定放在 core owner；不要让 `SubagentData` 风格的内存快照决定真实 close。
- **`src/session.rs`**：session 是 append-only JSONL，可在崩溃后恢复完整记录且忽略坏尾行（`src/session.rs:L1-L5`）；`append_runtime_intent` 已验证 session identity、source request 去重/冲突并持久化 intent（`src/session.rs:L1171-L1203`）。建议加入 typed `dag_node_registered`、`dag_edge_added`、`node_heartbeat`、`node_green`、`node_close_requested`、`node_closed`、`node_spawn_requested`、`node_work_received` 事件，或新建 `DagJournal`，并为 `close` 保存检查时的 generation/subtree digest。现有 `append_event` 明确禁止直接写 `session_tree`（`src/session.rs:L1205-L1214`）；DAG 也应有单一 owner，避免普通 opaque event 绕过不变量。`fork_to` 会排除 operation/lifecycle/mailbox/reconnect/request 事件（`src/session.rs:L1653-L1711`），所以 fork 只能复制历史上下文，不能复制活跃 worker/未完成消息。
- **`src/tool_runtime.rs`**：`execute_tool_batch` 负责 bounded tool batch，结果按 source order，取消由 owner 轮询，legacy handler 仍会 join，且不会因取消 detach（`src/tool_runtime.rs:L157-L167`）。它适合 node 内部工具执行和“自身是否 green”的 tool outcome 汇总，不适合 DAG worker spawn；不要用 `MAX_BATCH_CALLS=32` 误当 DAG 节点并发上限。节点进入 Green 前应确认 tool batch 的每个结果都已持久化；任何 unknown outcome 维持非 Green。
- **`src/runtime.rs`**：已有 lock-free、幂等 `CancellationToken`，完成标志可赢过晚到的 cancel（`src/runtime.rs:L49-L93`）；`BackgroundRunner` 具 bounded command/event queue、FIFO pending、取消和 panic terminal event（`src/runtime.rs:L100-L190`）。建议扩展 request/event 类型承载 `NodeId`、generation 和 parent correlation，并在 `Completed` 之后由 core 写 durable node result，再做 subtree close check；不要依赖 detach 来证明节点关闭，因为运行线程可能仍有副作用。
- **`src/protocol.rs`**：已有版本化 JSONL、bounded frame/text/id、Mailbox 与 `StdioEvent` sequence。可执行差异是：增加 `NodeRelation`/`NodeMessage` 的 serde 类型，或在现有 `MailboxRequest::Send.text` 中承载版本化 JSON envelope；建议优先新增强类型字段，避免关系、generation、correlation 全塞字符串。必须新增校验：source session 与 authenticated owner、target 邻接关系、generation 单调性、消息 TTL、close reason 长度；现有 mailbox 的 sender identity 本来就刻意不由客户端字段声明（`src/protocol.rs:L87-L89`），应继续保留这一安全边界。
- **`src/approval.rs`**：ApprovalCoordinator 是权限/副作用门，不是 DAG 状态机。可复用其 pending/visible/decisions/accepted 生命周期、`persist_accepted` fail-closed 与 `emergency_cancel`（`src/approval.rs:L136-L151`、`L370-L443`）。节点 close 前把 pending approval 视作非 Green；cancel/worker lease 过期时调用 `cancel_all`，但必须将拒绝/取消写 journal 后才能宣告该 child 不再有待处理副作用。
- **`src/providers/**`**：`providers/mod.rs` 的 `ProviderDefinition`/`RouteRule` 只描述 provider protocol、endpoint、auth header、capabilities 和 option policy，并通过 `get_provider_definition` 选择 provider（`src/providers/mod.rs:L11-L18`、`L136-L161`）；`connection.rs` 的 `ValidatedRoute` 负责路由/凭据复验（`src/providers/connection.rs:L40-L60`、`L113-L135`）；`registry.rs` 是有界模型目录与能力/上下文预算（`src/providers/registry.rs:L38-L86`、`L118-L137`）；各 provider 文件如 `openai.rs` 只是静态 route definition（`src/providers/openai.rs:L1-L27`）。这些模块不应承载 parent/sibling/child 通信或 close 判定。worker 只携带既有 provider/model route，graph admission 另行校验 worker binding、预算、凭据句柄。

### 可执行、可验证的最小改动顺序

1. 先定义 `DagNode`、`NodeMessage`、`NodeEvent`、`NodeStatus` 与关系校验；单元测试覆盖 parent/grandparent/sibling/child 四类允许边，以及非邻接目标拒绝。
2. 在 `session.rs` 增加 durable graph records 与 recovery projection；测试崩溃尾行、重复 `work_id+generation`、generation 变化时 close 被拒绝。
3. 在 `core.rs` 实现 `evaluate_close`：节点自身及全部 descendants 非 Green 时返回保活；全绿且 generation 未变才追加 `node_closed`。测试 child/grandchild 任一 Red/Blocked/Running 的反例。
4. 在 `runtime.rs`/`headless.rs` 接入 bounded `SpawnNode`，持久化 intent 后派生；测试 queue full、cancel、panic、shutdown 和 late result 都不会虚构 Green/Closed。
5. 将 mailbox send/receive/claim/complete 与 node event sequence 接入 headless；测试消息过期、重复 claim、完成回执和恢复 cursor。
6. 最后把 `ApprovalCoordinator` 和 `execute_tool_batch` 的未决 approval/unknown tool outcome 接到 close gate；`providers/**` 只做 route/capability 回归测试，不把其职责扩成 DAG。

与源文件的关键差异是：源文件允许“未知 session 事件直接丢弃”（L827-L831），UI 这样做可避免噪声；zenpi DAG 不能丢弃未知 node 事件，必须 journal 为 orphan/审计错误并保持非 Green。源文件的 80 帧/32 call 等是展示内存上限（L15-L19、L370-L376、L498-L525），zenpi 的消息、状态和关闭证据必须 durable，内存 compact 不能删除 close 判定所需的事实。

## 未决问题

无法从源文件确认：`session-data.ts` 中 `reduceSessionData` 对每类 SDK `Event` 的完整语义及 `limits` 的具体字段；`StreamCommit` 的所有 kind/phase 枚举；child session ID metadata 的权威来源；`SUBAGENT_BOOTSTRAP_LIMIT` 与 `SUBAGENT_CALL_BOOTSTRAP_LIMIT` 在调用方的实际分页策略。源文件也不能确认 DAG 的授权策略、worker 预算、心跳 TTL、Green 定义或 close 后新工作如何入队；这些必须由 zenpi 的 graph owner/治理协议明确，不能从本文件推断。

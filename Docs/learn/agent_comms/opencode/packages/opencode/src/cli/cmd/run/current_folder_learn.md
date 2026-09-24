# `opencode/src/cli/cmd/run` 目录级学习汇总

学习模式：`learn_mode=understand`。本目录当前纳入 1 个源文件，且其 1:1 笔记已完成：`AC-053`，`opencode/packages/opencode/src/cli/cmd/run/subagent-data.ts`，记录的源码版本为 `source_hash=d5f7d3dec7e743679e9b06504cc92381242a4d50842d040806fc2bcbd10dacbb`，覆盖 876 行、22129 字节。给定的 opencode 源码绝对路径在当前工作区不可读取，因此本汇总以该全量笔记为源行为证据，并用 zenpi 当前 Rust 实现作落点核对。

## 1. 目录职责

该目录的职责不是启动或调度 subagent，而是把父 session 产生的 `task` tool part、子 session 的消息/事件、权限与问题队列，投影成可供 CLI/UI 消费的两个内存视图：`tabs` 是每个 child session 的摘要状态，`details` 是有限长度的 `SessionData + StreamCommit` 帧。它通过稳定的 `sessionID` 关联父 task 与子会话，使用同步 reducer 接收实时事件，使用 bootstrap 函数把历史消息重放成与实时路径相同的提交帧，再用有界裁剪控制内存。

因此它更像“子代理运行态读模型/检查器快照”，不是执行器、线程池、IPC 层或 DAG 状态机。`running`、`completed`、`error`、`cancelled` 是展示与排序状态，不等于真正停止了 worker；模块也不负责自动 retry、keepalive、worker spawn 或 close 判定。

## 2. 模块清单

| 文件 | 一句话职责 | 关键导出 |
|---|---|---|
| `subagent-data.ts` | 管理父任务到子 session 的 tab/detail 投影，处理历史 bootstrap、实时 reducer、队列快照、帧合并与有界压缩。 | 常量 `SUBAGENT_BOOTSTRAP_LIMIT`、`SUBAGENT_CALL_BOOTSTRAP_LIMIT`；类型 `SessionMessage`、`BootstrapChildMessage`、`Frame`、`DetailState`、`SubagentData`、`BootstrapSubagentInput`；函数 `createSubagentData`、`listSubagentPermissions`、`listSubagentQuestions`、`snapshotDetail`、`listSubagentTabs`、`snapshotQueues`、`snapshotState`、`snapshotSubagentData`、`snapshotSelectedSubagentData`、`bootstrapSubagentData`、`bootstrapSubagentCalls`、`reduceSubagentData`。 |

文件内部的关键私有原语包括：`sameSubagentTab`/`sameCommit` 的幂等比较，`frameKey` 的稳定帧定位，`mergeLiveCommit` 的 progress 文本合并，`compactDetail` 的引用保留式裁剪，`ensureBlockerTab` 的 permission/question 阻塞投影，以及 `cancelSubagentTab` 的取消状态变换。

## 3. 运行时数据流与控制流

初始化时 `createSubagentData` 创建空的 `tabs: Map<sessionID, tab>` 与 `details: Map<sessionID, detail>`。父 session 的 `task` part 只有在 child 集合中命中、且 metadata 取得 `sessionId/sessionID` 时，才由 `syncTaskTab` 建立 tab；`ensureDetail` 惰性创建 `SessionData`，并保留 `includeUserText:true`。父侧的非 tool part 不会误生成子代理 tab。

历史路径由 `bootstrapSubagentData` 先建立已知 child 的 ID 集合，再从父消息同步 task tab、过滤只属于 child 的 permission/question，随后按 session ID 排序调用 `bootstrapSessionData`。`bootstrapSubagentCalls` 对一个已知 child 的历史消息先合成 `message.updated`，再按 part 顺序合成 `message.part.updated`，然后进入 `bootstrapChildMessages`，因此历史 user、reasoning、text、tool 与队列状态都能重放为统一的 `StreamCommit` 帧。

实时路径由 `reduceSubagentData` 入口分流：父 session 的 `message.part.updated` 只接受 tool part，用来刷新父侧 task tab；child session 的 `message.updated`、`message.part.updated`、`message.part.delta`、permission/question asked/replied/rejected、`session.error`、`session.status` 才进入 child detail。一般事件先由 `reduceSessionData` 更新 `SessionData`，再由 `appendCommits` 以 `frameKey` 追加或定位更新，最后 `compactDetail` 清理 inactive map。相同事件可重放而不重复写帧；同 key 的 progress 会拼接 `current.text + next.text`；超过 80 帧只保留末尾。

permission/question 事件先比较队列快照；`ensureBlockerTab` 会把既有 running tab 的描述改成 `Pending permission` 或 `Pending question`，但不会把已 completed/error 的 tab 重新打开。快照出口再把 running 排在非 running 前、按 `lastUpdatedAt` 倒序，并按队列 ID 排序，形成稳定的消费顺序。

## 4. 错误、取消与恢复语义

工具状态由 `compactToolState` 压缩：pending 保留 input/raw，running 保留 input/time 与必要 metadata，completed 保留 output/title/metadata/time，其他未知状态落入 error 分支。`taskStatus` 把 completed 映射为 `completed`；error 且 `interrupted===true` 或错误文本为 `Tool execution aborted` 映射为 `cancelled`；其余 error 为 `error`；pending/running 为 `running`。

assistant message 的 `MessageAbortedError` 是另一条取消输入：若 tab 仍 running，则 `cancelSubagentTab` 改为 cancelled。retry 不会重新提交任务或创建 child，只在 `session.status` 中追加一条 `error/start/system` 帧。未知 child 的事件会被丢弃；这对 UI 降噪合理，但对 zenpi 的 DAG 审计不够安全。源文件没有 `AbortController`、timer、Promise、thread、retry loop、provider 调用或网络/数据库副作用；“取消”只是投影状态，不能证明底层 worker 已停止。

恢复依赖调用方提供历史消息和事件，不是持久化恢复：`bootstrapSubagentData` 恢复父 task tab/blocker，`bootstrapSubagentCalls` 恢复 child 消息；`compactDetail` 仅做内存裁剪。调用者必须串行投递，同一 `Map` 没有并发安全承诺。权限/问题队列的比较依赖数组长度、ID、对象引用；若对象原地改动而引用不变，不能单靠 `queueChanged` 检出。

## 5. 对 zenpi DAG 编排的 Rust 映射

### 5.1 可复用通信原语与拓扑

可复用的抽象是“稳定身份 + 有界消息/邮箱 + 可重放事件 + claim/ack + 快照”，而不是 `SubagentData` 的 UI tab。zenpi 当前已有两套可利用的通信实现：通用 session mailbox 位于 `src/protocol.rs`/`src/session.rs`，提供 `MailboxRequest::{Send,Receive,Acknowledge,Claim,Complete}`、TTL、sequence cursor 和状态转换；DAG 专用邮箱位于 `src/dag.rs`，由 `DagMessage`、`DagStore::send`、`claim_inbox`、`ack`、`wait_for_message` 组成，并用文件锁保护共享 JSON。

`DagStore::recipients` 已覆盖 worker 需要的 parent、grandparent、直接 sibling、直接 child，以及组合目标 `all`；`DagNode.parent/children` 是当前会话树关系的实际来源。`DagRelation` 提供类型化关系名，`DagMessage` 提供 `from/to/body/ts_ms/claimed_by`，claim 后未 ack 会再次投递。对应的事件原语可映射 `src/runtime.rs` 的 `RuntimeEvent::{Accepted,Started,Queued,CancelRequested,Completed,Rejected,Closed}`，将 `node_id`、`parent_id`、`generation`、`correlation_id` 和单调序列放入事件或 durable journal。`src/core.rs::Turn::with_parent` 只能表达 turn 因果链，不能替代 DAG 的全部 descendants 索引。

需要保留一个安全边界：当前 `DagStore::recipients` 对显式 node ID 也允许发送，role 目标才按拓扑解析；因此“只能联系 parent/grandparent/sibling/child”的要求还没有被完全强制。若目标是严格邻接授权，应让 graph owner 根据 `DagNode` 计算关系，并拒绝非邻接显式 ID，而不是信任 worker 自报 `from` 或任意 `to`。

### 5.2 会话父子关系、保活与派生的最小机制

当前 DAG 会话关系由 `DagNode` 维护：`upsert(id, parent)` 写入 parent 的 `children`，缺失 parent 会先作为 root 创建；`spawn_worker` 用 `ZENPI_DAG_NODE`、`ZENPI_DAG_STORE` 注入 child worker，并为其创建独立的 `--session ...jsonl`。但 `DagNode` 没有显式 `session_id` 字段，`worker` 只保存如 `pid:<pid>` 的绑定，因此 node identity 与 session journal identity 仍需进一步固定映射。

最小保活已存在：`touch_worker` 写入 `worker_heartbeat_ms`，`dag_status`/`dag_recv` 会触碰 heartbeat；`wait_for_message` 最长轮询 30 秒，每 250ms 检查取消并调用 `claim_inbox`。最小派生也已存在：`dag_spawn` 先 `upsert` child，再调用 `spawn_worker`；父进程保留 child stdin，`send_to_worker` 可发送 follow-up prompt，不必重启 worker。换言之，“close 失败则保活并派生新 worker”目前主要由 `dag_finish` 的返回指令与 worker/tool 行为完成，并非后台自动调度器。

close gate 位于 `DagStore::can_close`：自身必须是 `green`，并递归检查 children 的全部 descendants 都是 `green`；任一 child/grandchild 为 `open` 或 `red` 就返回 `unfinished`。`dag_finish` 明确要求 false 时继续存活、`dag_spawn`、`dag_send`、`dag_recv`，直到 subtree 全绿。当前 gate 仍缺少 generation/subtree epoch 的一致性检查，也不检查 pending mailbox、approval、tool operation 或 unknown outcome；heartbeat 只展示 idle 秒数，不会因过期自动判红或阻止 close。这些是 DAG 语义要达到“可靠 close”前必须补齐的最小治理条件。

### 5.3 既有 Rust 文件的具体落点

- `src/headless.rs`：这里集中处理 stdin `StdioRequest`、`Command::Mailbox`、`BackgroundRunner`、job map、request-to-job map 和 shutdown。当前 DAG worker 通过 headless 模式接收 JSON prompt，但 `dag_*` 主要作为 `src/tools.rs` 中的 tool 运行，并未成为独立 headless command。建议在此接入 graph owner 的 admission、node event 回放和 close 请求；`RuntimeEvent::Closed` 才能作为进程级关闭完成信号，不能把“收到 close 请求”当成 descendants 已终结。
- `src/core.rs`：`Agent` 已有 `register_live_owner`、`heartbeat_live_owner`、`unregister_live_owner` 以及 live mailbox claim/finish，可承载 session owner 与 worker lease；`Turn::with_parent` 保留 turn-level parent。建议把 DAG node 的授权、generation、`evaluate_close` 和 worker binding 校验放在 core owner，避免 UI/内存快照决定真实 close。
- `src/session.rs`：`SessionStore` 是 append-only JSONL，已有 `append_runtime_intent`、`SessionMailbox` 的 durable enqueue/claim/complete/failed/expired 状态和 inode lock。若 DAG 要跨进程/崩溃恢复，应在这里或独立 `DagJournal` 持久化 `node_registered`、`edge_added`、`heartbeat`、`work_received`、`spawn_requested`、`green`、`close_requested`、`closed`，并记录 close 时的 generation/subtree digest；当前独立 `.zenpi/dag.json` 仍是轻量共享状态，不等于 session journal。
- `src/runtime.rs`：`BackgroundRunner` 提供 bounded command/event channel、FIFO follow-up、`CancellationToken`、panic terminal event 与 orderly shutdown；适合承载 `SpawnNode { parent, work_id, generation }` 和节点内工作。它默认单 active job，不应被误当成并行 DAG worker pool；多 sibling 并发需受治理的多 runner/pool。取消是 cooperative，且不能撤销已产生的副作用，未知结果必须保持非 green。

辅助落点是 `src/tools.rs` 的 `DagStatusTool`、`DagSendTool`、`DagRecvTool`、`DagFinishTool`、`DagSpawnTool`，它们把上述 store/worker 原语暴露给 agent；`src/approval.rs` 的 pending approval 应成为 close gate 的非 green 条件，`src/tool_runtime.rs::execute_tool_batch` 的 unknown/未持久化结果也应阻止 green。

## 6. 未决问题

1. opencode 源码原目录当前不存在，无法重新核验 `session-data.ts` 的完整 `reduceSessionData`、`StreamCommit` 的全部 kind/phase、child session ID metadata 的权威来源，以及两个 bootstrap limit 在调用方的分页策略；AC-053 笔记已记录这些边界。
2. zenpi 是否把 `DagStore` 继续作为独立的 file-locked JSON coordination store，还是把 DAG 事实迁入 `SessionStore`/`SessionMailbox`，需要统一单一事实源；否则 session journal 与 `.zenpi/dag.json` 可能出现状态分叉。
3. “全绿”的定义仍需明确：是否必须包含无 pending mailbox、无 pending approval、无 running tool、无 detached runtime job，以及 heartbeat 未过期；当前 `can_close` 只检查节点状态递归。
4. close 检查与新工作到达之间需要 generation/subtree epoch 原子性：检查后若收到新 `work_received`，必须拒绝旧 close，增加 generation，并通过 mailbox/`BackgroundRunner` 派生或唤醒 worker。现有实现尚未记录这一竞态的 durable 证据。
5. 需要决定 heartbeat TTL、失联 worker 的状态转移和恢复策略；过期不能直接当作 green，至少应保持 `open`/`red` 或进入可恢复状态。
6. 需要明确 node ID 与 `SessionStore::session_id()` 的一一绑定、worker 重启后的 pid/lease 更新、重复 `work_id+generation` 的幂等规则，以及显式 node ID 发送是否必须收紧为四类邻接关系。

# AC-047 — opencode/packages/opencode/src/acp/permission.ts

## 元信息

- source_id/item_id：`opencode/packages/opencode/src/acp/permission.ts` / `AC-047`
- source_path：`opencode/packages/opencode/src/acp/permission.ts`
- source_hash：`b83f1b30015f92f432e56a444a7304e131b1824b06870e7842c70db6bd969743`
- source_bytes：`7694`
- source_lines：`254`
- coverage：已按源文件顺序读取字节 `0-7693`（完整 7694 字节），行 `L1-L254`（含注释、import、类型、常量、类、辅助函数及最终导出）。

## 完整行为复盘

### 类型、常量与导出

- `PermissionEvent` 是 `Extract<Event, { type: "permission.asked" }>`（`L16`），因此 `Handler.handle` 只接受 ACP 事件联合中权限询问这一种形状。
- `Reply` 为 `"once" | "always" | "reject"`（`L17`）。`Connection` 是 `AgentSideConnection` 的部分投影，只保留可选的 `requestPermission` 与 `writeTextFile`（`L18`）；两个回调缺失时仍可构造处理器。
- `permissionOptions` 是固定三项（`L20-L24`）：`once/allow_once`、`always/allow_always`、`reject/reject_once`。调用方不能通过事件改变选项集合或名称。
- `export * as ACPPermission`（`L254`）把整个模块以命名空间导出，外部实际使用的是 `ACPPermission.Handler`；内部辅助函数并未单独导出。

### `Handler`（`L26-L116`）

- 字段 `queues: Map<string, Promise<void>>`（`L27`）按 `sessionID` 建立串行尾 promise。构造函数（`L29-L35`）只保存 `sdk`、`connection`、`session`，没有初始化外部状态。
- `handle(event)`（`L37-L49`）先取事件属性和该会话已有尾 promise；没有旧任务时以 `Promise.resolve()` 开始。新任务通过 `previous.then(() => this.process(event))` 排队；`catch(() => {})` 吞掉处理异常，避免一个权限失败阻断同会话后续事件；`finally` 仅在 map 中仍指向当前 `next` 时删除键（`L40-L46`），防止旧任务完成时误删已替换的新尾 promise。不同会话使用不同键，可并发处理；同一会话严格先入先出。该方法不返回 promise，调用者不能等待处理完成。
- `process(event)`（`L51-L89`）首先用 `Effect.runPromise(this.input.session.tryGet(permission.sessionID))` 查找会话（`L53`）。结果为空直接返回，不发送回复；`tryGet` 拒绝则由 `handle` 的 catch 吞掉。若无 `connection.requestPermission`，立即调用 `reply(id, "reject", session.cwd)`（`L56-L59`），默认拒绝。
- 有 `requestPermission` 时，先异步构造 `ToolCallUpdate`，再传入 `sessionId`、工具调用、固定 options（`L61-L70`）。工具调用 ID 优先 `permission.tool?.callID`，缺失时回退到权限 `id`（`L64-L68`）。请求 promise 拒绝时 catch 会回复 reject 并返回 `undefined`（`L71-L74`）；`!result` 直接结束（`L76`）。这两条路径都不会再次抛错。
- `selectedReply(result)` 返回 `once` 或 `always` 才继续（`L78-L81`）；用户取消、未知 option、非 `selected` outcome 均转为 reject。权限类型为 `edit` 时，先尝试 `writeProposedEdit(session.id, permission.metadata)`，并在该调用失败时吞掉错误（`L84-L86`），然后才用选定结果回复（`L88`）。因此编辑预览写入失败不会改变 allow 决策。
- `reply(requestID, reply, directory)`（`L91-L97`）把三个值原样提交给 `sdk.permission.reply`；SDK 拒绝会向上传播至 `process`，再由 `handle` 的队列 catch 吞掉。目录来自已找到会话的 `cwd`。
- `writeProposedEdit(sessionId, metadata)`（`L99-L115`）从 `metadata.filepath` 与 `metadata.diff` 读取字符串；缺字段或无 `writeTextFile` 时返回。文件存在则 `readText`，否则以空串为旧内容（`L104`）；`applyPatch` 失败返回 `false` 时不写入（`L105-L108`）。成功时调用 `connection.writeTextFile({sessionId,path,content})`，但使用 `void` 不等待（`L110-L114`），所以写入是 fire-and-forget，异步拒绝不会反馈给本处理流程。

### 工具调用构造与标题

- `permissionToolCall(input)`（`L118-L137`）用 `pendingToolCall` 构造状态为 `pending` 的基础调用，raw input 为原始 `ToolInput`，标题来自 `permissionTitle`（`L123-L130`）。随后先等待 `permissionContent`，再填入 `locations`；content 为空时省略 `content` 字段（`L131-L136`）。
- `permissionTitle(toolName,input)`（`L139-L163`）先 `toLocaleLowerCase()`。`external_directory` 按 `description ?? command ?? parentDir` 取字符串；`webfetch` 取 `url`；`websearch` 取 `query`；`grep/glob` 取 `pattern`；`read/edit/write` 走 `editTitle`；未知工具返回 `undefined`。所有候选必须经过 `stringValue`，非字符串不作为标题。
- `editTitle(input)`（`L165-L170`）先解析文件元数据：恰好一个文件时优先 `relativePath` 再 `filePath`；多个文件返回“`N files`”；没有结构化 files 时回退 `filePath ?? filepath ?? path`。
- `permissionLocations(toolName,input)`（`L172-L181`）有文件元数据时，将每个文件的 `filePath` 和可选 `movePath` 展开，过滤空值后用 `Set` 去重，映射成 `{path}`；没有文件时委托 `toLocations(toolName,input)`。因此移动目标也会成为位置，但重复路径只出现一次。
- `permissionContent(toolName,input)`（`L183-L194`）只有工具名小写为 `edit` 才生成内容。优先处理结构化 files（`L186-L188`）；否则取 `filepath/filePath` 和 `diff`，任一缺失返回空数组。单 patch 通过 `diffContentForPatch`，有结果才包装为单元素数组。
- `diffContentForFiles(files)`（`L196-L205`）对每个文件用 `Promise.all` 并行读取/应用 patch；无 patch 的文件贡献空数组；最终 `flat()`，保留输入文件顺序对应的结果顺序。
- `diffContentForPatch(filepath,diff,displayPath=filepath)`（`L207-L217`）读取旧文件（不存在视为空串），应用 patch；`false` 表示 patch 上下文不匹配，返回 `undefined`。成功返回 ACP diff content：`type:"diff"`、展示路径、`oldText`、`newText`。`displayPath` 可把移动前后的真实路径与 UI 展示路径分开。
- `selectedReply(result)`（`L219-L223`）严格白名单：只有 `outcome.outcome === "selected"` 且 optionId 是 `once`/`always` 才放行，其他一律 `reject`。
- `stringValue(value)`（`L225-L227`）只接受 primitive string，其他类型（数字、数组、null、对象）均为 `undefined`，不做隐式转换。
- `PermissionFileMetadata`（`L229-L234`）要求 `filePath`，`relativePath/movePath/patch` 可选且均为字符串。`fileMetadata(input)`（`L236-L251`）仅在 `input.files` 为数组时解析；逐项过滤 null/非对象、缺 `filePath` 或非字符串项；合法项转为受限记录。它不会验证路径存在、是否在 workspace 或 patch 大小。

## 状态、取消、恢复与副作用

该模块没有显式取消 token、超时、重试计数、持久化队列或恢复日志。唯一内存状态是按会话的 promise 尾指针；会话任务结束后 map 键删除（`L27,L43-L48`）。`requestPermission` 可以无限期阻塞，队列因此会保活该会话但不会自行超时；其他会话不受影响。请求失败、无 UI、无效 outcome 均 fail-closed 为 `reject`，但 `sdk.permission.reply` 自身失败只会被队列吞掉，源中没有补偿重试。

外部副作用有三类：调用 ACP UI 的 `requestPermission`（`L61-L70`）、向 OpenCode SDK 写入权限决定（`L91-L97`）、编辑预览成功后异步调用 `writeTextFile`（`L110-L114`）。`exists/readText` 仅读文件，`applyPatch` 只在内存计算。`writeProposedEdit` 不等待写文件结果，进程退出或连接关闭时可能留下未确认的异步副作用；它也没有原子写入、锁或持久化回滚。`Effect.runPromise` 只负责桥接 `ACPSession` effect 到 promise，不保存处理器状态。

## 源内测试与行为判据

源文件本身未包含测试；对应测试位于 `opencode/packages/opencode/test/acp/permission.test.ts`。可核对的行为判据包括：

- `L159-L184`：有 session 时应发出 `requestPermission`，工具调用状态为 pending，默认 bash 标题取 `command`，options 恰为 once/always/reject，并回复 once。
- `L186-L209`：`webfetch` 标题取 URL，raw input 原样保留。
- `L211-L243`：单文件 edit 应产生一个 diff content，旧/新文本由 patch 计算，位置为文件路径。
- `L245-L294`：结构化 `files` 按文件产生多个 diff 和去重位置，标题为“2 files”。
- `L296-L330`：`external_directory` 优先 description，位置来自工具 locations。
- `L332-L352`：取消 outcome 与 `requestPermission` reject 都必须回复 reject。
- `L354-L374`：session A 的阻塞权限不能阻塞 session B 的消息更新，验证跨会话并发。
- `L376-L400`：同一会话的第二个权限必须等第一个 resolve 后才发出，验证 promise 队列的串行语义。
- 可独立验证：构造缺失 session、缺 `requestPermission`、未知 option、非字符串 metadata、无效 patch、重复 filePath、多个并发 session，分别检查“无回复/拒绝/空标题/无 content/去重/跨会话不互锁”等判据。

## zenpi Rust 映射

### 可复用原语与会话关系

zenpi 已有的 `src/protocol.rs` 提供版本化 JSONL 协议（`StdioRequest`/`Command`）及 `MailboxRequest::{Send,Receive,Acknowledge,Claim,Complete}`（`src/protocol.rs:L87-L113`），适合作为 permission 事件的线协议；`src/runtime.rs` 的 `BackgroundRunner`、有界 `sync_channel`、`RuntimeEvent::{Accepted,Started,Queued,CancelRequested,Completed,Closed}`（`src/runtime.rs:L156-L190,L236-L365`）可承载异步询问、结果和关闭事件。`src/core.rs` 的 `AgentEvent`、`take_events`、live tool mailbox（约 `L300-L325,L846-L889,L1390-L1391`）可把权限询问广播给 headless host，再把决定回送核心。

会话身份应使用 `SessionStore::session_id()` 和现有 `SessionMailbox` 的 sender/recipient 校验（`src/session.rs:L3171-L3219,L3260-L3476,L3574-L3608`）。父子 DAG 关系使用 `src/dag.rs` 的 `DagNode.parent/children`，不要把 transcript 的 `SessionTree.parent_id` 当成进程父子：`SessionTree` 维护记录分支和回放，DAG worker 维护执行节点、邮箱和进程生命周期。若一个 DAG 节点对应一个 session，建议在节点记录 `session_id` 与 `parent` 的双向索引，向上/横向/向下通信均通过节点关系解析。

### 对 permission.ts 的 Rust 类型/函数落点

- 在 `src/approval.rs` 增加 ACP/host 适配层（例如 `PermissionRequest`、`PermissionOption`、`PermissionReply`）。`ApprovalCoordinator::request/respond/respond_with_request/cancel_all`（`src/approval.rs:L131-L216,L293-L345,L409-L447`）已经提供 pending、首个决定获胜、取消唤醒和 fail-closed 基础；把 `once/always/reject` 映射到 `ApprovalDecision`，并保持 worker 决定不可记忆的既有约束。
- 在 `src/core.rs` 增加 `PermissionEvent` 到 `AgentEvent` 的构造函数：复用 `AgentEvent::ToolCall/ToolResult/Provider/Error` 的关联 `turn_id` 设计，把 `session_id`、`request_id`、`tool_call_id`、`metadata` 和候选选项写入事件；权限决定先经 `ApprovalCoordinator`，再由 core 在进入工具副作用前持久化。
- 在 `src/headless.rs` 的异步 dispatch/事件回放位置接入权限请求和决定。使用现有有界事件 mailbox、请求 ID replay 和 JSONL `StdioResponse/Event`，确保 UI 掉线时不把未确认请求当 allow；对应 `permission.ts` 的异常吞并应在 Rust 中变为可观测的 `Rejected`/`Error` 事件，同时默认拒绝。
- 在 `src/protocol.rs` 增加明确的 `Permission` 命令或扩展 `MailboxRequest`：载荷至少包含 `session_id`、`request_id`、`tool_call_id`、`permission`、`metadata`、`options`、`reply`；沿用 `MAX_ID_BYTES`、`MAX_MAILBOX_TEXT_BYTES` 和 `deny_unknown_fields` 风格，拒绝控制字符、超长 diff、未知 option。
- 在 `src/tool_runtime.rs` 复用 `execute_tool_batch` 的 bounded worker、取消轮询和每调用结果关联（`src/tool_runtime.rs:L157-L313`）。`edit` 的 patch 预览应做成纯函数 `preview_patch(path,diff)->DiffContent`，验证路径、大小和 `apply_patch` 失败；不要复制 TS 的 fire-and-forget 写文件，真正写入应由已持久化 allow 的工具调用完成。
- 在 `src/providers/**` 不需要把 provider API 当权限队列；各 provider（`anthropic.rs`、`codex.rs`、`google.rs`、`openai.rs`、`deepseek.rs` 及 `connection.rs/registry.rs`）只提供能力和流式事件。权限门应位于 provider/tool dispatch 之前，provider 错误通过现有 `ProviderEvent` 回到 `src/core.rs`/`src/headless.rs`，避免每个 provider 重复实现一次 allow/reject。

### DAG 编排的具体落点

`src/dag.rs` 已直接覆盖目标需求：`DagRelation::{Parent,Grandparent,Sibling,Child,All}` 及 `recipients`（`src/dag.rs:L79-L112,L332-L400`）解析 worker 到 parent、grandparent、直接 sibling、直接 child；`DagMessage`、`DagStore::send/inbox/wait_for_message`（`src/dag.rs:L114-L122,L402-L437,L600-L618`）是可复用的消息/邮箱原语；文件锁和有界消息/node 上限（`L185-L218,L30-L33`）提供跨进程并发安全。`DagNode.worker_heartbeat_ms`、`touch_worker`、`status_view`（`L57-L76,L320-L329,L620-L657`）是最小 keep-alive 机制，父节点可据 idle 秒数判断失联。

关闭门应直接调用 `DagStore::can_close`（`src/dag.rs:L448-L478`）：自身状态必须 `green`，并用栈遍历全部 child/grandchild 后确认每个后代均为 `green`；缺失节点也列为 unfinished。若 false，worker 保持运行，读取 `unfinished`/新消息并调用 `spawn_worker`；`spawn_worker` 为新节点启动 headless 子进程、设置 `ZENPI_DAG_NODE`/`ZENPI_DAG_STORE`、保持 stdin 打开（`L499-L550`），`send_to_worker` 可向已派生直接 child 追加工作（`L571-L585`）。最小循环是：启动时 `upsert + assign_worker`，周期 `touch_worker`，`wait_for_message`/`inbox`，处理后 `set_status`，`can_close=false` 则派生或继续保活，`can_close=true` 才退出。

会话父子关系建议两层并存：DAG `parent/children` 决定执行拓扑和通信目标；`SessionMailbox` 的 `sender_session_id/recipient_session_id` 决定可审计消息归属；`SessionTree` 仅作为同一 session transcript 的祖先/分支索引。`src/runtime.rs::CancellationToken`（`L49-L99`）接入 `wait_for_message` 的 `cancelled` 闭包和 worker 子进程 shutdown；`src/headless.rs` 负责把 DAG 命令、权限决定、生命周期事件映射到 JSONL 和可重放 journal；`src/session.rs` 负责把 permission resolution、DAG spawn/close、mailbox claim/complete 记录成事件，支持崩溃恢复。

### 与源实现的差异及可执行验证

1. TS 的每 session promise 队列是内存且无容量；Rust 应按 `(session_id, request_id)` 建立有界队列，队列满返回 `QueueFull`，并用 `RuntimeEvent::Queued/Completed` 验证串行与终态。
2. TS 没有取消/超时/持久化；zenpi 应用 `CancellationToken`、shutdown grace、`SessionStore::append_event` 和 operation recovery。验证：取消等待中的 permission 后必须 durable reject，重启不能把未决请求恢复为 allow。
3. TS edit 预览允许异步写建议文件；zenpi 应把预览和真正工具写入分开，验证 patch 失败、路径越界、SDK/host 断连都不会产生未审计文件修改。
4. TS 只在“选中 once/always”时回复 allow；zenpi 的 `ApprovalPolicy` 还要结合工具副作用、worker origin 和 remembered grants，验证 worker 的决定不能升级成全局记忆许可。
5. DAG 关闭验证必须覆盖全部后代而非只有直接 child；使用 `can_close` 的 unfinished 列表写测试：自身 red、孙节点 open、缺失 child、全绿四种情况分别得到 false/false/false/true。
6. 通信验证覆盖四类关系和 `all` 去重：一个节点分别向 parent、grandparent、sibling、child 发送，`inbox(mark_read=true)` 只消费目标邮箱；子 worker stdin 追加 prompt 与共享 `DagMessage` 应最终一致。

## 未决问题

- `ACPSession.tryGet` 的具体失败/不存在语义以及 `Effect.runPromise` 的 runtime 配置未在本源文件定义。
- `pendingToolCall`、`toLocations` 对工具 kind、location 和 metadata 的完整映射在 `tool.ts`，本文件只依赖其接口。
- `writeTextFile` 的 ACP 服务端是否保证顺序、原子性和失败重试无法从本文件确认。
- `RequestPermissionResponse.outcome` 的完整协议枚举由外部 ACP SDK 定义，源文件只显式接受 `selected + once/always`。


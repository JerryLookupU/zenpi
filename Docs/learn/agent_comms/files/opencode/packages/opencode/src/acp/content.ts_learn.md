# AC-043 — opencode/packages/opencode/src/acp/content.ts

## 元信息

- source_id/item_id：`AC-043`
- source_path：`opencode/packages/opencode/src/acp/content.ts`
- source_hash：`8b3c0bc40893eb18a2a4b8f8709a2188ffe5aa11e4d93232b4bd80989ba2bbc3`
- source_bytes：`7092`
- source_lines：`269`
- coverage：已从字节 `0-7091`、行 `L1-L269` 按顺序读完，包含全部注释、类型、导入和导出符号。

## 完整行为复盘

文件是 ACP `ContentBlock` 与 OpenCode 会话输入/回放类型之间的纯转换层。导入 `ContentBlock`、`ContentChunk`、`ResourceLink`、`Role`，Node 的 `path`、URL 转换函数，以及 `SessionV1`（L1-L4）。没有类、全局可变状态或 I/O。

- `PromptPart` 是 `SessionV1.TextPartInput | SessionV1.FilePartInput`（L6）。`ReplayPart` 是三种联合：文本可带 `synthetic`/`ignored`，文件含 `url`/`mime`/可选 `filename`，reasoning 仅含文本（L8-L24）。
- `promptContentToParts(content)` 对只读 `ContentBlock[]` 做 `flatMap(contentBlockToParts)`，保持输入顺序并丢弃每个块转换出的空数组（L26-L28）。
- `contentBlockToParts(block)` 按 `block.type` 分派（L30-L117）。`text` 总返回一个文本 part，文本原样保留，并把单一 assistant audience 映射为 `synthetic: true`、单一 user audience 映射为 `ignored: true`（L32-L39）。`image` 优先使用非空 `data` 组装 `data:<mime>;base64,<data>`，文件名从 URI basename 推导，缺失默认 `image`（L41-L50）；否则接受 `data:` URI（L52-L60）或 `http://`/`https://` URI（L62-L70），同样推导文件名与 MIME；其他 URI 或缺字段返回空（L71-L72）。`resource_link` 始终包装 `resourceLinkToPart` 的单个结果（L74-L75）。`resource` 的 text 资源先尝试解析 URI：`file:` URI 用 `fileURLToPath`（失败时解码 pathname），Windows 反斜杠归一为 `/`，若 hash 匹配 `#L数字` 则把行号附加到 `[path:line]` 前缀；解析异常或非 file URI 使用 `[原始 URI]` 前缀，均以换行拼接资源文本（L77-L99）。无 text 但有 `mimeType` 的资源返回 file part；已有 `data:` URI 原样使用，否则将 blob 包成 `data:<mime>;base64,<blob>`，文件名缺省 `file`（L100-L110）；两者都没有则返回空（L111-L112）。未知类型走 default 空数组（L114-L116）。URI 解析异常被有意吞掉，函数不会抛出。
- `partsToContentChunks(parts)` 对只读 `ReplayPart[]` 顺序 `flatMap(partToContentChunks)`（L119-L121）。
- `partToContentChunks(part)` 分派回 ACP chunk（L123-L151）：空文本/空 reasoning 返回空（L125-L126、L140-L141）；普通文本生成 `{content:{type:"text",text,...partAudience(part)}}`，reasoning 也生成 text 但不恢复 audience（L127-L149）；file 交给 `filePartToContentChunks`（L137-L138）。没有显式 default，但联合类型已穷尽。
- `resourceLinkToPart(link)` 用 `link.mimeType ?? "text/plain"` 和 `link.name` 调用 URI 转换；转换成 file 就返回，否则把 URI 文本包装成 text（L153-L157）。
- `uriToFilePart(uri,mime,filename?)`（L159-L188）接受 `file://` 并保留 URL；文件名优先显式参数，其次 `filenameFromUri`，最后 `file`（L165-L171）。`zed://` 读取 query 的 `path`，存在时用 `pathToFileURL` 变成 file URL，文件名优先显式值，否则 `path.basename`/`file`（L173-L182）。其他协议直接成为 text URI；`URL`、路径转换或任何异常都回退到 text，不传播错误（L184-L187）。
- `filePartToContentChunks(part)`（L190-L239）把 `file://` 转成 `resource_link`，name 默认 `file`（L191-L201）；非 `data:` URL 丢弃（L203-L204）。`decodeDataUrl` 成功后，`image/*` 生成 image content，数据保留 base64，URI 由文件名转 file URL（L205-L217）。文本 MIME 或 `application/json` 解码 base64 为 UTF-8 `resource.text`；其他 MIME 保留 base64 为 `resource.blob`，两者 URI 都由文件名生成（L220-L238）。
- `decodeDataUrl(url)` 只接受正则 `^data:([^;]+);base64,(.*)$`，失败返回 `undefined`，成功返回 MIME 与 base64 字符串，不校验 base64 内容（L241-L245）。
- `audienceFlags(audience)` 仅在 audience 长度恰为 1 且角色为 assistant/user 时分别返回 synthetic/ignored，否则 `{}`，包括 null、undefined、多角色与未知角色（L247-L251）。`partAudience(part)` 做逆映射：synthetic 优先 assistant，否则 ignored 映射 user，否则无 annotations；不会同时输出两者（L253-L257）。
- `filenameFromUri(uri?)` 对空值和 `data:` 返回 undefined；普通合法 URI 取解析后 pathname 的 basename，空 basename 仍 undefined；URI 解析失败时把输入当路径取 basename，失败路径也不会抛出（L259-L269）。

所有转换是同步、确定性的单线程函数；`flatMap` 不改变顺序。没有共享锁、异步任务或并发协调语义。默认值集中在 MIME `text/plain`、文件名 `image`/`file`、空数组与空对象。

## 状态、取消、恢复与副作用

源内没有取消 token、超时、重试、持久化、网络请求、文件读写或进程副作用。`pathToFileURL`、`fileURLToPath`、`new URL`、`Buffer.from(...).toString("utf8")` 只在内存中计算；URI 解析和路径转换错误被捕获并降级为 text/空结果。函数不会修改传入数组或 block；返回的新对象可由调用者继续修改。data URL 的格式错误、非 data/file/http(s) 图像、无 MIME 的资源、空文本均是静默丢弃或文本回退。恢复方向（ReplayPart→ContentChunk）不保留 reasoning 类型，也不做内容去重；文本 audience 只支持单角色的 assistant/user 往返。

## 源内测试与行为判据

存在 `/Users/wangweiyang/GitHub/opencode/packages/opencode/test/acp/content.test.ts`。L7-L29 验证普通文本及 assistant/user audience 标记；L31-L65 验证 base64 图像优先级和 HTTP 图像；L67-L100 验证 `resource_link` 的 file/zed URI 与 MIME/文件名回退；L102-L147 验证 file 文本资源的路径、`#L12-L14` 行号和非 file URI 前缀；L149-L187 验证 blob、data URI 原样保留；L189-L192 验证 audio/unknown 被忽略；L195-L205 验证 synthetic audience 回放；L208-L234 验证 file link 与 data 文本回放。可独立判据是：对这些输入逐项调用导出函数，深比较返回 JSON 结构、顺序、默认字段；额外覆盖空 text、坏 data URL、`application/json`、多 audience、Windows 路径和 `zed://` 缺 path，期望分别为空数组、文本回退或无 annotations。

## zenpi Rust 映射

这里的核心可复用思想不是 ACP 字段本身，而是“有类型的消息/资源转换 + 明确的关系与终态”。建议如下：

- **消息/邮箱/事件/协议原语**：将 `PromptPart`/`ReplayPart` 对应为 `protocol.rs` 的带 `serde(tag)` 消息枚举；现有 `MailboxRequest::{Send,Receive,Acknowledge,Claim,Complete}`（`src/protocol.rs:L79-L113`）已覆盖 DAG 节点间消息投递，`MAX_MAILBOX_TEXT_BYTES`/TTL（L32-L37）提供边界。`SessionMailbox` 的追加式、带 digest/sequence/status 的消息（`src/session.rs:L3180-L3255`）可承载 text/file 元数据；`RuntimeEvent::{Accepted,Started,Queued,CancelRequested,Completed,Closed}`（`src/runtime.rs:L171-L193`）是事件流。可新增 `DagMessage { message_id, sender, recipient, relation, payload: ContentPayload, ttl_ms }` 与 `DagEvent { node_id, kind, sequence }`，在 `protocol.rs` 增加严格验证，沿用 JSONL `StdioEvent`（`src/protocol.rs:L882-L917`）。
- **会话父子关系**：`SessionHeader.session_id` 与 `SessionStore` 的 append-only journal（`src/session.rs:L35-L41、L144-L160`）是稳定身份；`core::Turn.parent_id` 已保存父 turn（`src/core.rs:L140-L175`），`InputBoundaryGate.context_parent` 可携带当前上下文父节点（`src/runtime.rs:L765-L831`）。建议在 `SessionHeader` 或独立 `DagNodeRecord` 增加 `parent_session_id`, `grandparent_session_id`（可由父链计算）、`relation_to_parent`，并用 session_id 建索引：parent/grandparent 是向上路由，直接 sibling 是共享 parent 的子集合，direct child 是 `parent_session_id == self` 的集合；禁止从客户端任意 sender 字段推断权限，复用 mailbox 的 workspace/access 校验（`src/session.rs:L3598-L3614`）。
- **worker 负责节点的最小通信机制**：发送给 parent、grandparent、直接 sibling、direct child 都统一调用 `SessionMailbox::enqueue`/`MailboxRequest::Send`，消息含 `relation` 与 `in_reply_to`；接收端以 `Receive → Claim → Complete` 形成显式状态机。现有 headless 分发已把 mailbox 命令接到 `mailbox_response`（`src/headless.rs:L8630-L8637`），`execute_mailbox` 对 recipient 做唯一会话解析并返回 `execution_started:false`（`src/headless.rs:L8954-L8985`），可在此加 DAG 关系授权与路由，不启动隐式 worker。
- **保活（keepalive）**：直接复用 `LiveSessionRegistry::{register,heartbeat,unregister,active}`（`src/session.rs:L3274-L3355`）。每个 worker 启动时以 `(session_id, owner_epoch, workspace)` 注册；周期性 heartbeat；发送/claim 前 `active` 清理超时 owner。令牌绑定 owner epoch 的 claim/finish（`src/session.rs:L3361-L3488`）保证旧进程不能完成新进程的消息。最小新增是 `DagLease { node_id, owner_epoch, expires_at_ms }`，把 lease_id 纳入 worker 消息和 `ApprovalRequest`，不需要新守护进程。
- **派生（derive）与 close 判据**：`BackgroundRunner::spawn/try_submit` 已提供有界命令队列、事件队列和 `CancellationToken`（`src/runtime.rs:L49-L99、L236-L349`），适合作为单节点 worker；`try_submit` 派生新工作，`try_cancel`/`try_shutdown_with_grace` 处理取消与保活失败。建议在 `core.rs` 增加 `DagNodeState { open, waiting, completed, blocked }`、`derive_worker(parent, work)` 和 `child_status(node)`：只有节点自身 `completed` 且所有 direct child、grandchild 递归为 `completed` 才发出 `CloseAccepted`；否则保持 `waiting`，通过 mailbox 发送新任务并 `try_submit` 派生 worker。关闭前写 session event，事件顺序为 `children_green_checked → close_accepted`；任何 child 非绿、超时、UnknownOutcome 或消息未完成都禁止 close。`AgentPhase::Closed` 的保护和 handoff 写入可参考 `core.rs:L5450-L5473`。
- **持久化与恢复**：用 `SessionStore::append_event`/`SessionRecord.sequence` 保存 node state、lease、派生原因和 close 判定（`src/session.rs:L314-L325` 及 append-only 设计 L1-L5）；恢复时重放最后一个节点状态，发现无 terminal close 或有未完成 claim 则保持活跃并重新 heartbeat/显式 retry，不能把缺失记录当成功。Mailbox 的 `Expired` 是时钟视图，不能伪造执行回执（`src/session.rs:L3245-L3255`）。
- **取消、工具与 provider 边界**：`tool_runtime.rs` 的批处理明确“取消不隐式重试、未完成调用需 owner join”（文件 L157-L167），应把 DAG close 检查放在工具批次和 provider turn 的安全边界之后；`approval.rs` 的 `ApprovalCoordinator`（L126-L201）继续作为副作用批准/取消 rendezvous，DAG 派生不能绕过 approval。`src/providers/**` 只负责 provider wire/stream 适配，输出应转成 `core::AgentEvent`/`RuntimeEvent`，不得直接改 DAG 状态；provider 失败、取消或 unknown outcome 通过 protocol event 回传节点状态。
- **与 `headless.rs` 的落点**：新增异步命令如 `dag_send`、`dag_receive`、`dag_derive`、`dag_close_check` 映射到 `Command`，复用现有 request id、event replay、mailbox response；所有拒绝返回带 correlation id 的 typed error，避免在 stdout 外产生旁路协议。`src/headless.rs:L8626-L8637` 已是 checkpoint/mailbox 分派边界。

差异清单：ACP content.ts 是无状态、面向内容形状的转换；zenpi 还需关系授权、owner heartbeat、持久化状态机、递归全绿判定和派生调度。ACP 的 data URL 不验证 base64，zenpi 若把 payload 持久化应增加大小/MIME/编码校验；ACP 对未知块静默丢弃，而 DAG 消息未知类型应返回协议错误并记录事件；ACP 没有 close 语义，zenpi 必须把 `close_accepted` 作为可恢复的 durable terminal event。

## 未决问题

无法从该源文件确认：DAG 节点 ID 是否必须与 `SessionStore.session_id` 一一对应；grandchild 是否跨多个 session journal；“全绿”是否允许 `Succeeded` 以外的可接受终态；派生 worker 的最大并发、重试预算和 provider payload 大小上限。这些应由 zenpi 的 DAG 协议/产品约束补充，不能从 content.ts 推断。

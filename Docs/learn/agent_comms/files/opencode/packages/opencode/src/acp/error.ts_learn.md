# AC-045 — opencode/packages/opencode/src/acp/error.ts

- source_id/item_id：opencode/packages/opencode/src/acp/error.ts / AC-045
- source_path：opencode/packages/opencode/src/acp/error.ts
- source_hash：60657818576b13275c4d3e665e6c586f686d98cc5ef30ac2ef4c0402c2c8080b
- source_bytes：3525
- source_lines：97
- coverage：已读取字节 1-3525、行 1-97（含全部导入、注释、类型、导出类与函数）。

## 完整行为复盘

该文件只定义 ACP（Agent Client Protocol）错误值及其到 SDK RequestError 的边界转换；没有 I/O、全局可变状态或异步任务。第 1 行导入 RequestError，第 2 行导入 effect 的 Schema，因此每个 Schema.TaggedErrorClass 都同时提供带 _tag 的可判别错误类、字段 schema 与构造器。

- SessionNotFoundError（L4-L6）：标签为 "ACPSessionNotFoundError"，必填 sessionId: Schema.String。表示会话 ID 不存在；构造时缺字段或非字符串属于 schema/运行时输入错误，类本身不执行查找。
- InvalidConfigOptionError（L8-L13）：标签 "ACPInvalidConfigOptionError"，必填 configId 字符串，表示未知配置项。
- InvalidModelError（L15-L18）：标签 "ACPInvalidModelError"，必填 modelId 字符串，providerId 为可选字符串；允许只知道模型、不知道提供商的失败。
- InvalidEffortError（L20-L22）：标签 "ACPInvalidEffortError"，必填 effort 字符串，表示 effort 档位不存在。
- InvalidModeError（L24-L26）：标签 "ACPInvalidModeError"，必填 mode 字符串，表示会话模式不存在。
- AuthRequiredError（L28-L30）：标签 "ACPAuthRequiredError"，可选 providerId 字符串；不携带 provider 也能表达“需要认证”。
- UnknownAuthMethodError（L32-L37）：标签 "ACPUnknownAuthMethodError"，必填 methodId 字符串，表示客户端要求的认证方式未知。
- UnsupportedOperationError（L39-L44）：标签 "ACPUnsupportedOperationError"，必填 method 字符串，表示 RPC 方法未实现。
- ServiceFailureError（L46-L50）：标签 "ACPServiceFailureError"，必填对外安全文本 safeMessage，可选 service 与 errorName 字符串。它刻意不保存原始异常对象，便于跨协议输出时避免泄露密钥、堆栈或内部细节。

Error 类型别名（L52-L61）是上述九类的闭合集合。调用方可把它作为 Effect 失败通道的静态联合类型；标签 _tag 是后续分派的判别字段。

toRequestError(error: Error)（L63-L93）是纯映射函数，按 _tag 分支，输出 ACP SDK RequestError：
- ACPSessionNotFoundError（L65-L66）→ RequestError.invalidParams({sessionId}, "session not found: …")。
- ACPInvalidConfigOptionError（L67-L68）→ invalidParams({configId}, "unknown config option: …")。
- ACPInvalidModelError（L69-L73）→ invalidParams({providerId, modelId}, "model not found: …")；可选 provider 值会原样进入数据对象，序列化是否省略 undefined 由 SDK 决定。
- ACPInvalidEffortError（L74-L75）→ invalidParams({effort}, "effort not found: …")。
- ACPInvalidModeError（L76-L77）→ invalidParams({mode}, "mode not found: …")。
- ACPAuthRequiredError（L78-L79）→ RequestError.authRequired({providerId}, "provider authentication required")；认证错误使用 SDK 专用错误码/消息组合。
- ACPUnknownAuthMethodError（L80-L81）→ invalidParams({methodId}, "unknown auth method: …")。
- ACPUnsupportedOperationError（L82-L83）→ RequestError.methodNotFound(error.method)；方法名同时作为 SDK 方法未找到错误的参数。
- ACPServiceFailureError（L84-L91）→ RequestError.internalError(data, error.safeMessage)。数据对象只在 service/errorName truthy 时通过展开加入对应键（L86-L89），安全文本原样作为内部错误消息。
- switch 在 L92 结束且无运行时 default；静态 Error 联合保证穷举，若外部强行传入伪造未知 _tag，函数运行时可能返回 undefined，这是边界风险。

fromUnknownDefect(_defect: unknown, safeMessage = "Internal service failure")（L95-L97）把任意未知缺陷包装为 new ServiceFailureError({safeMessage})。参数 _defect 有意不读取、不拼接、不序列化；默认消息固定，调用者可提供替代安全文本。该函数不重试、不记录、不抛出原缺陷，随后应交给 toRequestError 形成内部错误。

## 状态、取消、恢复与副作用

错误类是一次性值对象，没有取消 token、超时、重试计数、恢复状态或持久化钩子；toRequestError 与 fromUnknownDefect 均为同步纯函数，不创建线程、不等待锁，也没有外部副作用。并发调用只读参数并各自返回新 RequestError，不会互相影响。恢复语义由上层决定：验证错误通常直接返回客户端修正；认证错误应等待凭据后重新请求；服务失败可由调用方按策略重试，但本文件不声明幂等性。fromUnknownDefect 的安全边界是“丢弃原 defect，仅保留 safeMessage”，因此原始堆栈恢复不可从此值完成。

## 源内测试与行为判据

源文件本身未包含测试；同目录 test/acp/error.test.ts 提供直接判据：
- L6-L16：五类验证失败都映射 JSON-RPC -32602（invalid params）。
- L18-L27：会话/模型的安全字段进入 data，模型缺 provider 时仍可映射。
- L29-L36：认证错误为 RequestError 实例，码 -32000、消息为 "Authentication required: provider authentication required"，并携带 provider。
- L38-L43：不支持方法为 -32601，数据含 method。
- L45-L53：服务失败为 -32603，消息含安全文本，数据只含非空 service。
- L55-L66：未知 Error 的 secret、refresh token、stack 均不出现在序列化结果中；独立验证时应对每个标签断言错误码、消息与数据键，并对伪造未知标签检查上层拒绝/兜底行为。

## zenpi Rust 映射

现有基础可直接复用：src/error.rs:14-L31 的 ZenpiError 是顶层错误枚举；src/core.rs:354-L399 的 AgentError 已有稳定 code() 分类；src/protocol.rs:79-L113 已有 MailboxOutcome/MailboxRequest（Send/Receive/Acknowledge/Claim/Complete），src/protocol.rs:152-L209 的 StdioRequest 负责相关协议字段；src/core.rs:281-L325 的 AgentEvent 可承载错误/警告事件；src/runtime.rs:49-L100 的 CancellationToken 和 src/tool_runtime.rs:157-L176、L252-L347 的协作取消与 join 语义可作为 worker 生命周期基础；src/approval.rs:126-L245 的 ApprovalCoordinator 提供有界等待、取消轮询和宿主响应 rendezvous；src/session.rs 的 append-only journal（错误类型位于 L98-L114）可承载恢复记录；src/headless.rs:540-L715 将解析/调度错误变成带 code 的 StdioResponse。

建议落点与可执行差异：
1. 在 src/protocol.rs 增加 AcpError/AcpErrorKind（或在 src/error.rs 增加同名枚举）九个变体：字段对应上述字符串，ServiceFailure { safe_message, service: Option<String>, error_name: Option<String> }。用 thiserror 保留人类消息，用 serde/显式 to_request_error 保证机器字段；验证空 ID、控制字符和长度时复用现有 validate_identifier（L617-L630）规则。
2. 在 src/protocol.rs 增加 AcpRequestError { code: i32, message: String, data: serde_json::Value } 与 impl From<AcpError>；映射码固定为 -32602/-32000/-32601/-32603，数据只放非空可选字段，等价于 TS L63-L93。未知枚举值返回 AcpError::Internal，禁止 Rust 中出现“静默返回 None”。
3. 在 src/headless.rs 的命令错误响应处调用该转换；在 src/core.rs 的 AgentError 增加 Acp(#[from] AcpError) 或清晰的 InvalidTurn/Session/Backend 到 ACP 错误的适配函数。src/runtime.rs 的 JobOutcome::Failed/RuntimeEvent::Failed（约 L158-L205）承载结构化错误，不要把 safe_message 与原始 defect 混合。
4. 通信原语按 DAG 需求复用并补最小协议：消息用 MailboxRequest::Send/Receive，事件用 AgentEvent::{Handoff, Error, Warning, ToolResult}，外部协议用 StdioRequest 的 mailbox 字段；src/approval.rs 的有界 Condvar 可复用为等待/唤醒模型。建议 DAGNodeId、parent_id: Option<DAGNodeId>、children: Vec<DAGNodeId> 放入 session/tree 状态（现有 Turn.parent_id 在 src/core.rs:140-L175 可作为父子关系样板），并由 src/session.rs journal 持久化节点状态和消息序号。
5. 会话父子关系：每个 worker session 记录 parent_session_id 与 root_session_id；直接 sibling 通过共同 parent 的 children 索引解析，grandparent 通过 parent.parent 解析，直接 child 通过 children 解析。发送接口必须校验目标属于同一 DAG、消息 ID 唯一、TTL 在 validate_mailbox（src/protocol.rs:544-L575）范围内；跨边界/不存在节点映射为 SessionNotFound/InvalidParams。
6. 保活与派生最小机制：在 src/runtime.rs 增加 WorkerLease { node_id, generation, cancellation: CancellationToken, last_heartbeat_ms } 与 WorkerCommand::{Deliver(MailboxMessage), Heartbeat, CloseRequested, SpawnChild(DAGNodeSpec)}；worker 只有在自身状态及递归子孙状态均为 Green 时才发送 CloseRequested 并释放 lease。否则保持 lease/heartbeat，在收到新工作或失败事件时通过 BackgroundRunner::try_submit（L309-L349）派生新 generation；旧 worker 的取消必须先发 token，再 join，沿用 tool_runtime 的“nothing detached”判据。
7. 可验证闭环：src/core.rs 增加 DagHealth::is_green(node)（自身与全部 descendants 递归）、close_if_green 和 spawn_replacement；src/session.rs 追加 dag_node_state、worker_generation、mailbox outcome 事件，崩溃后从 journal 重放；src/headless.rs 暴露 dag_status/send/close 命令；src/approval.rs 仅负责需要人工批准的副作用。src/providers/** 只接收已净化的 safe_message 和取消谓词，不得看到原始 defect。每项可用单元测试验证错误码、父/祖父/兄弟/子节点路由、全绿关闭门槛、取消后 join 与重放幂等性。

## 未决问题

1. @agentclientprotocol/sdk 的 RequestError 对 undefined 数据字段的最终 JSON 序列化细节需以 SDK 版本实测确认。
2. 源文件未规定错误消息是否国际化、是否要求错误码稳定跨版本；zenpi 适配应由协议版本策略明确。
3. DAG 节点的“全绿”定义（失败后是否必须新 generation 成功、祖先是否等待所有后代 mailbox 完成）不在该 TS 文件中，需由 zenpi 产品协议补充。


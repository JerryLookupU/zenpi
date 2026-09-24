# AC-034 — codex/codex-rs/protocol/src/approvals.rs

- source_id/item_id: `AC-034`
- source_path: `codex/codex-rs/protocol/src/approvals.rs`
- source_hash: `b6452ca6956c6f03001b0887a4703146015d2805b82c933598eb6196d0e72cfc`
- source_bytes: `11849`
- source_lines: `319`
- coverage: 已按文件顺序读取完整字节范围（偏移）`0..=11848`、行范围 `L1-L319`（含注释、导入、所有导出符号）。

## 完整行为复盘

- `Permissions`（`L19-L25`）是四项权限值的聚合：`SandboxPolicy`、文件系统/网络 sandbox policy，以及可选 `MacOsSeatbeltProfileExtensions`。无构造函数、校验或内部锁；`Clone/PartialEq/Eq` 使其适合在审批上下文中复制比较。
- `EscalationPermissions`（`L27-L32`）二选一承载完整 `PermissionProfile` 或 `Permissions`。这是类型级的升级载荷，枚举本身不做策略判断。
- `ExecPolicyAmendment`（`L34-L44`）以 `#[serde(transparent)]` 序列化为字符串数组，`command` 是允许规则的 token 前缀。`new`（`L46-L49`）无过滤地包装 `Vec<String>`；`command`（`L51-L54`）返回切片，避免复制；`From<Vec<String>>`（`L56-L60`）等价构造。空数组也不会在本文件被拒绝。
- `NetworkApprovalProtocol`（`L62-L72`）采用 snake_case：`Http`、`Https`、`Socks5Tcp`、`Socks5Udp`。`Https` 额外接受反序列化别名 `https_connect` 和 `http-connect`（`L68-L69`）；websocket 变体由注释说明尚未提供（`L65-L66`）。
- `NetworkApprovalContext`（`L74-L78`）要求 `host: String` 与协议；没有 host 格式校验，校验责任在调用方。
- `NetworkPolicyRuleAction`（`L80-L85`）是 snake_case 的 `Allow/Deny`；`GuardianRiskLevel`（`L87-L93`）使用 lowercase 的 `low/medium/high`；`GuardianAssessmentStatus`（`L95-L102`）使用 snake_case 的 `in_progress/approved/denied/aborted`。三者均为可复制、可比较的无状态枚举。
- `NetworkPolicyAmendment`（`L104-L108`）携带 host 和 allow/deny 动作，无内建冲突检查；`ExecApprovalRequestSkillMetadata`（`L110-L115`）携带 `PathBuf path_to_skills_md`，字段名按 snake_case 导出。
- `GuardianAssessmentEvent`（`L117-L144`）描述 guardian 生命周期：`id` 稳定标识、`turn_id`（`L121-L125`）缺失时 serde 默认空字符串以兼容旧发送方；`status` 必填。`risk_score`（`L126-L129`）为可选 `u8`，注释约束语义为 0–100 但代码不检查上限；`risk_level`、`rationale`、`action: JsonValue`（`L131-L143`）均在进行中可省略，序列化时 `None` 被跳过。`JsonSchema/TS` 派生使 JSON Schema/TypeScript 表示与 serde 约定一致。
- `ExecApprovalRequestEvent`（`L146-L196`）是命令审批事件。必填 `call_id`、`turn_id`（旧消息缺失时默认空字符串，`L157-L160`）、token 化 `command`、`cwd`、`parsed_cmd`；可选字段包括子命令回调 `approval_id`（`L151-L156`）、原因 `reason`、网络上下文、execpolicy amendment、网络策略 amendments、额外 `PermissionProfile`、skill 元数据，以及客户端可展示的有序 `available_decisions`（`L168-L194`）。`approval_id` 缺省表示直接审批整个 call，存在时表示 execve 拦截的子命令（`L151-L153`）。
- `effective_approval_id`（`L198-L203`）克隆并返回 `approval_id`，否则回退到 `call_id`；无错误路径。`effective_available_decisions`（`L205-L217`）优先克隆发送方给出的列表；旧发送方没有该字段时调用静态 `default_available_decisions`。
- `default_available_decisions`（`L219-L252`）有明确优先级和顺序：有 `network_approval_context` 时返回 `Approved, ApprovedForSession`，若 amendments 中找到第一条 `Allow` 则追加 `NetworkPolicyAmendment`，最后追加 `Abort`（`L225-L237`）；否则有 `additional_permissions` 时仅返回 `Approved, Abort`（`L240-L242`）；普通命令先放 `Approved`，存在 execpolicy prefix 时追加 `ApprovedExecpolicyAmendment`，最后 `Abort`（`L244-L251`）。只看是否存在字段，不验证 host、命令或 amendment 数量；网络 amendments 中的 `Deny` 不生成可选策略按钮。
- `ElicitationRequest`（`L255-L274`）是带 `mode` tag 的 `form`/`url` 枚举。两者都有可选 `_meta: JsonValue`；`Form` 需要 `message/requested_schema`，`Url` 还需要 `url/elicitation_id`。`message`（`L276-L281`）用模式匹配借用两变体的消息，无分支错误。
- `ElicitationRequestEvent`（`L284-L294`）携带可选 `turn_id`、`server_name`、数值或字符串兼容的 MCP `RequestId`、以及请求体；`turn_id` 缺失会是 `None`，没有额外校验。
- `ElicitationAction`（`L296-L302`）以 lowercase 编码 `accept/decline/cancel`，是无状态响应动作。
- `ApplyPatchApprovalRequestEvent`（`L304-L319`）携带 patch call 的 `call_id`、旧发送方兼容的默认空 `turn_id`、`HashMap<PathBuf, FileChange>` 变更集、可选原因和 `grant_root`。`grant_root` 表示本 session 后续允许写入的根；本文件只传递意图，不执行写入、授权或路径安全检查。

这些类型只有 serde/TS/Schema 派生和少数纯函数，没有 `Mutex/Condvar/async`；因此并发语义是“可在线程间复制的消息值”，实际排队、去重、取消和持久化由宿主协议实现。枚举字段的未知值、缺失必填字段和类型不匹配会在反序列化层失败；显式 `#[serde(default)]` 字段走空字符串/`None`/空容器等兼容默认。

## 状态、取消、恢复与副作用

源文件定义的是审批/elicitation/patch 请求与事件的数据契约，没有取消 token、超时、重试循环、持久化写入或外部副作用。`GuardianAssessmentStatus::InProgress`、`ElicitationAction::Cancel` 和 `Abort` 只是可传输状态/决定，不会自动中断任务。`effective_*` 与默认决定计算均为纯内存、无副作用。调用方若要恢复，必须利用 `id/call_id/approval_id/turn_id` 做幂等关联，并自行记录已发事件和最终决定；`grant_root`、policy amendment 也必须由调用方显式验证后应用。

## 源内测试与行为判据

源内未包含测试。可独立验证的判据：

1. 对 `ExecPolicyAmendment` 做 JSON round-trip，透明结果应为字符串数组；`new`、`From<Vec<String>>` 与 `command()` 应保持同一 token 顺序（`L39-L60`）。
2. 反序列化 `https_connect`/`http-connect` 应得到 `NetworkApprovalProtocol::Https`，未知协议应失败（`L62-L72`）。
3. 构造四种 `ExecApprovalRequestEvent` 场景，核对决定顺序分别为网络（含首个 Allow amendment）、额外权限、普通带 prefix、普通无 prefix（`L205-L252`）。
4. 缺失 `turn_id` 的 guardian/exec/patch JSON 应得到空字符串，缺失可选字段应为 `None`；`ElicitationRequest::message()` 对 form/url 都返回原消息（`L117-L144`,`L146-L196`,`L255-L319`）。

## zenpi Rust 映射

现有 `src/approval.rs` 已有 `ApprovalRequest/ApprovalResponse/ApprovalCoordinator`（显式 host 决定、条件变量等待、`request_id` 去重和持久化前 accepted 队列），可直接承接 `ExecApprovalRequestEvent` 的 `call_id/approval_id/turn_id`，并把 `effective_approval_id` 作为关联 ID 规则。`src/protocol.rs` 已有版本化 JSONL、`ApprovalDecision` 字段、`MailboxRequest::{Send,Receive,Acknowledge,Claim,Complete}`、会话 ID 校验；因此可新增与本文件同构的 serde DTO（建议 `protocol::ApprovalEvent`, `NetworkApprovalContext`, `ElicitationRequest`），在 `Command`/事件编码处保留 snake_case/lowercase 和旧字段默认，避免把 wire 兼容逻辑塞进 coordinator。

DAG 通信映射必须显式覆盖四类邻接：worker 节点拥有 `parent_session_id`、`grandparent_session_id`、直接 sibling IDs、直接 child IDs；通过现有 `MailboxRequest::Send` 按 `recipient_session_id` 投递消息，`Receive` 拉取、`Claim/Complete` 形成消息状态机。建议在 `src/session.rs` 的 `SessionHeader`/事件投影增加可持久化的 `parent_session_id`, `node_id`, `depth`, `role` 与 `WorkerLease`（lease ID、过期时间、generation、状态），并以 append-only 事件记录 `worker_spawned`, `mailbox_sent`, `mailbox_completed`, `node_green`, `node_close_deferred`。`src/core.rs` 的 `Turn::parent_id`（`with_parent`/校验）可复用为会话内父子因果链；DAG 关系仍应放在 session 元数据/索引而非仅靠 turn 文本。由 session registry 建立父、祖父、兄弟、子节点的受信路由，客户端不可自行伪造 sender 权限。

最小保活与派生机制：在 `src/core.rs` 增加 `NodeStatus::{Running,Green,Closing,Alive,Failed}`、`WorkerLease` 和 `close_gate(node)`；`close_gate` 只有在节点自身为 `Green` 且所有直接/间接 child/grandchild 的聚合状态均为 `Green` 时才允许 `Closing -> Closed`，任一未绿节点都写 `node_close_deferred`、续租并保持 `Alive`。发现新工作时，先原子递增 generation、写入派生意图，再用 `runtime::Runtime::submit` 派生新 job；子节点完成后向 parent 回传 `node_green`，祖父通过 mailbox 聚合。新 worker 的 lease 过期或取消只能阻止结果接纳，不能回滚已经发生的工具副作用。

`src/runtime.rs` 提供 bounded command/event channel、FIFO follow-up、`CancellationToken`、`RuntimeEvent::{Accepted,Started,Queued,CancelRequested,Completed,Closed}`，适合作为每个 worker 的执行与保活心跳层；应增加一个定期 heartbeat/lease-renew 命令和按 parent/child 的 job metadata，沿用队列满/关闭错误。`src/headless.rs` 负责 JSONL 传输、重放与 session WAL，可把 approval/guardian/elicitation 事件编码为有序事件并在 reconnect 时重放；mailbox 的 receive cursor 应绑定 `SessionStore` 的持久序列号。

`src/tool_runtime.rs` 继续作为工具批处理、取消和 side-effect gate 的执行边界：在执行 patch、网络访问或命令前消费 approval DTO，只有 `ApprovalCoordinator` 接受并持久化后才产生副作用；`additional_permissions`/policy amendment 需要映射到现有 policy digest/worker binding，而不是直接放行。`src/providers/**` 是 provider 连接与事件入口，应把 provider 的审批/guardian 事件转换成协议 DTO，不在 provider 层维护 DAG 状态。`src/approval.rs` 负责本地策略和响应 rendezvous，`src/protocol.rs` 负责 wire schema/验证，`src/session.rs` 负责恢复，`src/core.rs` 负责 DAG close gate，`src/runtime.rs` 负责并发 worker 生命周期，四者边界可执行且可单测。

差异清单：

- codex DTO 有 `JsonSchema/TS` 派生和多处旧发送方默认；zenpi 目前协议已有版本验证但需补齐这些字段的兼容反序列化测试。
- codex 的默认决定只计算按钮列表；zenpi 还必须把决定绑定到 `policy_digest/lease_id` 并写入 session journal。
- codex 没有 DAG 拓扑、保活或递归 close 判定；这些应落在 `core/session`，不可由 `protocol` DTO 推断。
- zenpi 已有 mailbox 与 runtime 取消语义，可复用但需加入邻接授权、lease generation、子孙聚合和“保活后派生”事件。

## 未决问题

无法从源确认：`PermissionProfile` 各字段的具体合并规则、`FileChange` 的路径安全策略、`RequestId` 的完整编码范围、guardian 风险分数是否必须运行时限制在 0–100，以及 approval/elicitation 事件由哪个 transport 负责超时和重试。

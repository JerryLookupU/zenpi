# AC-002 — claude-code/examples/settings/README.md

- source_id/item_id：`claude-code/examples/settings/README.md` / `AC-002`
- source_path：`claude-code/examples/settings/README.md`
- source_hash：`ab8ea80cbc9065d195974bcfac84940f058bd7329b010cd162ef468f788ef228`
- source_bytes：`1846`
- source_lines：`31`
- coverage：已按文件顺序读取完整内容，覆盖字节 `0-1845`、行 `1-31`（含空行、警告、表格、链接和全部文字）。

## 完整行为复盘

源文件是说明文档，不含函数、类型定义或可导出的代码符号；因此“逐函数/逐导出符号”在源内的结论是：无可调用函数、无导出符号、无运行时返回值。下面按每个标题、链接和配置条目复盘其可验证行为。

- `# Settings Examples`（L1-L5）：文档把目录定位为 Claude Code settings 示例，主要面向组织级部署；它们只是起点，使用者应按组织需求调整（L3）。示例可放入 settings hierarchy 的任一层（L5），但 `strictKnownMarketplaces`、`allowManagedHooksOnly`、`allowManagedPermissionRulesOnly` 等属性只有写入 enterprise settings 才生效（L5）。输入是 JSON settings 文件及其部署层级，输出是 Claude Code 的配置效果；文档没有规定解析 API、合并顺序或覆盖冲突算法，因此不能从源推断这些细节。
- `## Configuration Examples` 与 WARNING（L8-L11）：示例被标注为社区维护，可能不受支持或不正确；配置正确性由部署者负责（L10-L11）。这构成错误路径和边界：错误配置不会由本 README 自动修复，也没有自动回滚或保证兼容性的承诺。
- 配置表及三个链接（L13-L21）：`settings-lax.json`、`settings-strict.json`、`settings-bash-sandbox.json` 是相对路径链接，表格以 ✅ 表示该文件用于该策略。`settings-lax.json` 与 `settings-strict.json` 都禁用 `--dangerously-skip-permissions`，并阻断 plugin marketplaces（L13-L17）；strict 另外阻断 user/project 定义的 permission `allow`/`ask`/`deny`、hooks、web fetch/search，并要求 Bash approval（L17-L20）；bash-sandbox 还阻断 user/project permission 规则，并要求 Bash 在 sandbox 内执行（L17、L21）。空单元表示该示例未声明该限制，不应解读为明确 allow。表格没有参数默认值、优先级或并发语义；每次加载是配置读取，非任务调度。
- `## Tips`（L23-L27）：建议合并上述 snippets 达到目标（L24），但合并后的 JSON 必须有效（L25）；部署前应通过把配置放入 `managed-settings.json`、`settings.json` 或 `settings.local.json` 在本地测试（L26）。`sandbox` 只作用于 `Bash`，不作用于 `Read`、`Write`、`WebSearch`、`WebFetch`、MCPs、hooks 或内部命令（L27）。因此输入边界是合法 JSON 和具体 settings 文件层；错误 JSON、错误层级或误把 sandbox 当作全局隔离都属于可预期失败/误配置。
- `## Full Documentation`（L29-L31）：完整可用 managed settings 的权威说明在 `https://code.claude.com/docs/en/settings`（L31）。这是外部文档引用，不是本文件内的实现或测试。

源文件没有函数级并发控制：没有 worker、锁、队列、超时、取消 token 或事件流。唯一的操作顺序约束是“合并后保持有效 JSON，再在本地应用测试，然后部署”（L24-L26）。

## 状态、取消、恢复与副作用

文档只描述静态配置样例，不定义运行状态机，因此源内没有取消、超时、重试、恢复、持久化格式或幂等键。部署到 `managed-settings.json`、`settings.json`、`settings.local.json` 是外部文件副作用，且本 README 只要求部署前本地验证（L26）；没有写文件、回滚或并发编辑协议。社区维护警告意味着配置错误、版本不兼容和不受支持行为都必须由调用方发现并处理（L10-L11）。`sandbox` 的作用域明确限制为 `Bash`（L27），不能作为其他工具、hooks、MCP 或内部命令的取消/隔离机制。恢复只能由重新选择正确 settings 文件并再次加载实现，源没有恢复步骤或数据迁移规则。

针对 zenpi DAG，必须把这些缺失的生命周期语义补齐：worker 处理 DAG 节点时，节点自身、全部直接 child 及递归 grandchild 都为 green 才能 `close`；任何非 green、过期、未确认或仍有未完成消息的后代都禁止 close，节点应 heartbeat 保活，并派生新 worker 承接新工作。这个规则不是源 README 的行为，而是映射目标要求，不能误称为 Claude Code 文档保证。

## 源内测试与行为判据

源内未包含测试，也没有同目录测试引用。可独立验证的判据如下：

1. 解析三个相对链接文件，确认均为合法 JSON；对每个配置逐项检查表格 L15-L21 的策略是否出现。
2. 在本地分别应用到 `managed-settings.json`、`settings.json`、`settings.local.json`，确认 enterprise-only 属性仅在 enterprise 层生效（L5、L26）。
3. 运行 Bash、`Read`、`Write`、`WebSearch`、`WebFetch`、MCP 和 hook 场景，确认 `sandbox` 仅改变 Bash 行为而不改变其他类别（L27）。
4. 合并 snippets 后再次做 JSON 解析和策略快照，确保合并没有产生重复/冲突字段；文档本身没有规定冲突时谁胜出，验证结果应记录实际产品行为。

## zenpi Rust 映射

- `src/protocol.rs` 是通信协议落点。可复用 `MailboxRequest::{Send, Receive, Acknowledge, Claim, Complete}`、`MailboxOutcome`、`StdioRequest.mailbox`、`Command::Mailbox`，沿用 `MAX_MAILBOX_TEXT_BYTES`、`MAX_MAILBOX_TTL_MS`、`MAX_ID_BYTES` 及严格 `validate_*`。把 DAG 通信编码成 `MessageEnvelope { message_id, sender_node, recipient_node, relation, payload, ttl_ms, parent_id }`；`relation` 至少取 `Parent`、`Grandparent`、`DirectSibling`、`DirectChild`、`Control`，让 worker 能精确寻址 parent、grandparent、直接 sibling 和直接 child。`TreeAction::{List, Select, Fork, Annotate}` 可承载树观察/派生请求，但不能替代 mailbox 的投递确认。
- `src/session.rs` 已有持久通信原语：`SessionMailbox` 的 `enqueue/list/update`、`MailboxMessage` 的 digest/sequence/status、`LiveSessionRegistry::{register, heartbeat, active, claim_next}`，以及 `MailboxStatus::{Queued, Acknowledged, Claimed, Succeeded, Failed, Expired}`。建议在同一 session journal 增加 `DagNodeRecord`（`node_id,parent_id,status,generation,lease_id,expected_children`）和 `DagTransition`（spawn、heartbeat、green、close、reopen）；用现有 append-only 记录保证崩溃恢复，用 digest + `request_id` 保证重试幂等。会话父子关系必须持久化：`parent_id` 指向直接 parent，向上两次得到 grandparent；同一 `parent_id` 的 active records 是直接 siblings；以该节点为 `parent_id` 的 records 是直接 children，递归查询得到 grandchildren。当前 `Turn.parent_id` 只表达对话 turn 关系，不能直接充当 DAG 节点关系，应避免混用。
- `src/runtime.rs` 提供最小执行机制：`BackgroundRunner::spawn`、有界 `try_submit`、`try_cancel`、`CancellationToken`、`RuntimeEvent::{Accepted,Started,Queued,CancelRequested,Completed,Closed}`、`JobOutcome` 和 bounded shutdown grace。每个派生 worker 应是一个带 `JobId` 的 runtime job；派生动作先落 `spawn` 记录再 `try_submit`，队列满返回 `SubmitError::QueueFull` 并保持父节点 lease；关闭前必须检查 DAG 聚合状态。`CancellationToken` 只取消执行，不把未完成后代伪装成 green。
- `src/headless.rs` 是宿主/编排落点。现有 `run_async_streams`、`AsyncEventBuffer`、`EventMailboxStream`、`handle_runtime_event` 和 bounded event/terminal replay 可复用为节点事件出口；增加 `dag_status`、`dag_send`、`dag_spawn`、`dag_close` 的 dispatch。`dag_close` 在一次持久化读取中验证自身与全部 descendants 均为 green、无 `Queued/Claimed` 消息、无活跃 lease 后才发 `Closed`；否则发 keepalive/拒绝 close，并对新工作调用 runtime 派生 worker。事件需要带 `node_id`、`parent_id`、`generation`、`sequence`，以便重连后不混淆 sibling。
- `src/core.rs` 管理业务状态。`Agent`、`Turn`、`AgentPhase`、`AgentEvent`、`ProcessResult` 可承载 worker 的当前节点状态和状态转移；建议增加纯函数 `descendants_green(node_id, snapshot)`、`close_if_green(node_id)`、`derive_worker(parent, work)`，将“自身 + 全部 child/grandchild 全绿才 close”的判据放在 core 而非 provider。`Turn::with_parent` 可记录消息/turn 的父链，但 DAG `parent_id` 仍应有独立字段。已有 handoff/事件记录可作为派生原因和可审计边界。
- `src/tool_runtime.rs` 的 `ToolBatchDecision`、`ToolBatchOutcome`、`execute_tool_batch` 适合执行一个 worker 的批量工具调用；将消息处理、spawn 和 close 视为可审计 operation，沿用其取消/失败结果，避免在 tool batch 中直接隐式派生无限 worker。
- `src/approval.rs` 的 `ApprovalRequest`、`ApprovalCoordinator`、`ApprovalMode::WorkerAllowAfterPreflight` 和 lease/policy digest 可作为派生或跨会话投递的权限闸门。`dag_send`、`dag_spawn` 若产生副作用，必须携带 node lease、policy digest，并在 lease 过期时拒绝执行；审批取消应让 mailbox operation 进入 `Cancelled`/`Abandoned`，而不是 green。
- `src/providers/**` 只负责 provider 请求和 `ProviderEvent` 流，不应保存 DAG 拓扑、决定 close 或推断 sibling。provider 进度事件经 `src/headless.rs` 转换为带节点身份的事件；provider 超时/失败进入节点非 green 状态，由 `src/core.rs` 决定重试或派生新 worker。

最小可执行机制是：持久 `DagNodeRecord` + `SessionMailbox` 消息/事件 + `LiveSessionRegistry` heartbeat lease + `BackgroundRunner` 派生/取消 + core 的递归 green gate。可验证差异包括：现有 mailbox 有 recipient/session/workspace 校验但没有 DAG relation；现有 runtime 有 job 生命周期但没有 descendant 聚合；现有 protocol 有 `TreeAction` 但没有 close gate；现有 session 有 handoff/父 turn，却没有明确的 node parent/grandparent/sibling 索引。实现后应增加针对四类寻址、过期 lease、队列满、重连重放、非 green 后代禁止 close、全绿后 close 的单元/集成测试。

## 未决问题

- 源 README 未定义 settings 文件内部字段的完整 schema、层级合并冲突优先级，以及 enterprise settings 的具体部署工具；需查外部完整文档或实际 Claude Code 版本确认（L5、L31）。
- 源未说明三个 JSON 文件的精确字段值、版本兼容范围、错误提示和回滚方式（L13-L21、L23-L27）。
- zenpi 映射中 DAG 节点状态的 green 定义、grandchild 深度是否无限、关闭时是否允许迟到消息、以及新 worker 的最大并发数无法由该源确认；这些必须由 zenpi 的 DAG 规范/测试另行确定。

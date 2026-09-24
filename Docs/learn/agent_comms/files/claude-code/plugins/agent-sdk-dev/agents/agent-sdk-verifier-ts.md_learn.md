# AC-005 — claude-code/plugins/agent-sdk-dev/agents/agent-sdk-verifier-ts.md

## 元信息

- source_id/item_id：`AC-005`
- source_path：`claude-code/plugins/agent-sdk-dev/agents/agent-sdk-verifier-ts.md`
- source_hash：`68fe98341656fa242a5de9272165481d62b5670b2fff294961bc45a5c4ee57ea`
- source_bytes：`5419`
- source_lines：`145`
- coverage：已按文件顺序读取完整字节范围 `1-5419` 与行范围 `L1-L145`；未跳过 front matter、注释性说明或报告格式。

## 完整行为复盘

该源文件是给 Claude Code 使用的 agent 规格说明，不含可执行 TypeScript/Rust 函数、类型定义或可导出的代码符号；可观察的“导出符号”是 front matter 的 `name`、`description`、`model`，以及后续的验证流程、报告字段。默认模型为 `sonnet`（L1-L5）。agent 的身份是 TypeScript Agent SDK application verifier，职责是检查 SDK 使用、官方文档一致性以及部署/测试就绪度，并且应在应用新建或修改后调用（L7-L7）。因此输入是待审查的 TypeScript Agent SDK 工程及其配置，输出是结构化审查报告；源文件没有规定返回对象或异常类型。

验证重点逐项如下。

1. SDK 安装与配置：必须检查 `@anthropic-ai/claude-agent-sdk` 是否安装、版本不能古旧、`package.json` 应有 `"type": "module"`，若有 `engines` 则核对 Node.js 要求（L13-L18）。缺失依赖、模块模式不符或 Node 版本不满足，应成为报告中的问题，而不是假设可以运行。
2. TypeScript 配置：检查 `tsconfig.json` 存在、模块解析支持 ES modules、target 足够现代，且编译配置不会破坏 SDK import（L20-L25）。边界是 SDK 导入与编译兼容性，不评价无关的代码风格。
3. SDK 用法：核对从 `@anthropic-ai/claude-agent-sdk` 的 import、agent 初始化、system prompt/model 等配置、SDK 方法参数、streaming 与 single response 处理、权限配置、MCP server 集成（L27-L35）。若使用 subagents 或 session，也要按 SDK 模式验证（L57-L65）。
4. 类型与编译：必须执行 `npx tsc --noEmit`，检查 SDK 类型定义、类型对齐及零编译错误（L37-L42）。命令失败属于可阻止运行的错误，不能只记作风格警告。
5. 脚本与构建：检查 `package.json` 的 build/start/typecheck 脚本，确认 TypeScript/ES module 脚本配置正确，并验证可构建、可运行（L44-L48）。
6. 环境与安全：检查 `.env.example` 是否含 `ANTHROPIC_API_KEY`、`.env` 是否在 `.gitignore`、源码没有硬编码 API key，并核对 API 调用错误处理（L50-L55）。密钥泄露和缺少错误处理都应提升为严重问题。
7. SDK 最佳实践：评估 system prompt 清晰度、model 是否合适、权限范围、MCP custom tools、subagents 配置及 session handling（L57-L65）。这里的“合适”由官方文档和用例比较得出，源未定义具体模型白名单或阈值。
8. 功能验证：检查应用结构、初始化到执行的流程、SDK 特定错误覆盖和官方 pattern 遵循情况（L66-L71）。
9. 文档：检查 README/基本文档、setup instructions 以及自定义配置说明（L73-L76）。

明确排除通用代码风格、`type` 与 `interface` 选择、无用变量命名和与 SDK 无关的 TypeScript 最佳实践（L78-L83）。这构成审查边界：发现这些内容时不应把它们升格为 SDK FAIL。

验证过程规定了审查输入集合和顺序：先读 `package.json`、`tsconfig.json`、主应用文件、`.env.example`、`.gitignore` 与其他配置（L85-L93）；再用 WebFetch 读取官方 TypeScript SDK 文档并对照实现、记录偏差（L95-L99）；随后执行 `npx tsc --noEmit` 并报告编译问题（L101-L104）；最后核对 SDK 方法、配置字段和官方示例模式（L106-L109）。源内没有并发、锁或多 worker 语义；上述步骤是一个有序审查流水线，WebFetch 和 tsc 的失败都应转化为报告中的可核对发现。

报告输出必须包含 Overall Status（`PASS | PASS WITH WARNINGS | FAIL`）和 Summary（L111-L117）。Critical Issues 要覆盖阻止运行的问题、安全问题、会导致运行时失败的 SDK 错误及类型/编译失败（L119-L124）；Warnings 要覆盖次优 SDK 用法、未使用但有益的 SDK feature、文档偏差和缺少文档（L126-L131）；Passed Checks 记录正确配置、正确实现的 SDK feature 和安全措施（L133-L137）；Recommendations 给出具体改进、官方文档引用和下一步（L139-L143）。结尾要求详尽且建设性，目标是可运行、安全、配置正确并遵循官方 pattern 的 Agent SDK 应用（L145-L145）。

## 状态、取消、恢复与副作用

源文件没有声明取消 token、超时、重试、恢复、持久化格式、幂等键或外部副作用控制；也没有函数级错误类型。唯一明确的外部交互是读取官方文档的 WebFetch（L95-L99）和执行 `npx tsc --noEmit`（L101-L104），均用于观察/验证。没有规定网络失败或编译失败的自动重试策略，应在报告中注明无法验证并给出 FAIL 或 WARNING 的理由。审查对象自身的 API 调用错误处理必须被检查（L50-L55、L66-L71），但 verifier 不应替被审查应用发起 agent/tool side effect。源也未定义会话恢复语义；“session handling is correct if applicable”只是被检查的被测行为（L63-L65）。

## 源内测试与行为判据

源内未包含测试（全文仅为 agent 规格说明，L1-L145）。可独立验证的判据是：

- 在目标工程运行 `npm ls @anthropic-ai/claude-agent-sdk` 或等价包管理器检查，确认依赖和版本；读取 `package.json`、`tsconfig.json`、环境模板和忽略文件，逐项对照 L13-L18、L20-L25、L50-L55。
- 执行 `npx tsc --noEmit`，要求退出码为 0；检查所有 SDK import 与配置字段的类型（L37-L42、L101-L104）。
- 用官方文档 URL（L97-L98）逐项核对初始化、system prompt、model、streaming、权限、MCP、subagents、session；把偏差映射到 Critical Issues/Warnings。
- 输出必须能填满 L115-L143 的五类报告字段，且总体状态只能是 `PASS`、`PASS WITH WARNINGS` 或 `FAIL`。若存在硬编码密钥、编译失败或会导致运行时失败的 SDK 调用，判定至少为 `FAIL`（L119-L124）。

## zenpi Rust 映射

该规格可作为 zenpi 的“worker verifier/审查策略”，而不是直接搬运 TypeScript API。现有落点与差异如下。

- **输入/输出与协议：** `src/protocol.rs` 已有 `MailboxRequest` 的 `Send/Receive/Acknowledge/Claim/Complete`，带 `recipient_session_id`、`message_id`、`text`、`ttl_ms` 和 `MailboxOutcome`（L81-L113）；`StdioRequest` 与 `Command` 把 JSONL 输入解析为有界命令（L152-L240），`parse_line`/响应编码在 L795-L803、L857-L995。可复用原语是“消息 + 有界协议 + 事件响应”，建议新增 `DagMessage`/`DagEvent` 只作为已有 `MailboxRequest`/`StdioEvent` 的受限 payload，避免另造无界通道。
- **邮箱实现与四类邻接通信：** `src/session.rs` 的 `MailboxMessage`、`SessionMailbox::enqueue/list/update` 和 `LiveSessionRegistry::claim_next/claim_message/finish_claim`（L3171-L3185、L3286-L3574、L3574-L3743）已经提供持久化消息、TTL、claim、完成/失败。DAG 节点应把每个节点绑定一个 `session_id`，通过 `recipient_session_id` 向 parent、grandparent、直接 sibling、直接 child 发送；grandparent/sibling 不应依赖路径猜测，而应由持久化 DAG 索引解析为 session ID。发送端使用 `Send`，接收端 `Receive`，处理前 `Claim`，处理结束用 `Complete`，并保留 `message_id` 作为幂等相关键。headless 的 `execute_mailbox` 已限制收件人必须是同一 session directory 中唯一的干净 journal，并执行 enqueue/list/claim/complete（L8954-L9093），可直接作为 transport adapter。
- **会话父子关系：** `src/core.rs` 的 `Turn` 有 `parent_id`，`with_parent` 和 `validate` 保证父引用可追踪且 ID/内容有界（L140-L205）；`src/session.rs` 另有 tree snapshot/ancestry/fork 能力（`tree_snapshot`、`tree_ancestry_turns`、`fork_at_tree_leaf`，L895-L1070）。建议新增 `DagNodeRecord { node_id, session_id, parent_id, status, generation, child_ids }`，把 DAG 拓扑与对话 turn 的 parent_id 分开保存；启动或恢复时从 session journal 重建 parent、grandparent、sibling、child 索引，拒绝缺失父节点、重复 edge 和跨 workspace 目标。
- **worker 生命周期、保活与派生的最小机制：** `src/runtime.rs` 的 `BackgroundRunner::spawn`、`try_submit`、`try_cancel` 与有界 `RuntimeConfig`（L101-L133、L272-L325）提供派生 worker、排队、非阻塞提交和取消；`CancellationToken` 是协作式、幂等取消，任务应在边界检查并可用 `mark_completed` 防止晚到取消改写成功结果（L49-L99）。建议在 `runtime.rs` 增加最小 `DagWorkerRequest { node_id, lease_id, work }` 与 `DagWorkerOutcome { status, descendants_green }`，由已有 runner 派生新 worker；队列满/关闭直接成为可重试的提交错误。每个活动节点必须由 `LiveSessionRegistry::heartbeat` 保活（现有 heartbeat/register 在 L3278-L3357），并持久化 lease/heartbeat 事件。
- **关闭门槛（DAG 特有要求）：** 不能把单个 worker 的成功当作 close 条件。新增 `DagStatus::Green|Running|Blocked|Cancelled` 和 `can_close(node_id)`：只有节点自身为 Green 且其全部 child、grandchild 递归均为 Green，才发出 close/注销 owner；任一后代未绿、失败、过期或有未完成 mailbox 时，节点保持 live，继续 heartbeat，并通过 `BackgroundRunner::try_submit` 派生新 worker 处理新工作。该判定应在 session journal 中追加可重放的 `dag_status`/ `dag_close_decision` 事件，避免重启后误关。
- **headless 落点：** `run_headless` 逐帧读取、校验、解析、请求 ID admission、调用 `handle_command` 并在结束时注销 owner（L556-L724）；异步入口 `run_async_streams` 连接有界 runtime（L792-L812）。应在 `handle_command` 的 DAG/mailbox 分支调用统一 DAG service，发送状态事件到 `StdioEvent`，把 close 变成“通过 `can_close` 后才允许”的响应；输入 EOF/显式 shutdown 的取消语义沿用现有 headless 边界，不应提前关闭仍有后代工作的节点。
- **事件、恢复和持久化：** `src/session.rs` 是 append-only JSONL store，暴露 `append_event`、`events`、operation recovery 和 checkpoint API（`append_event` L1207-L1223，恢复相关 L1402-L1590）；`src/headless.rs` 还有有界事件/terminal replay 和 checkpoint/mailbox 响应。建议把 worker accepted/started/heartbeat/derived/child-green/close-deferred/closed 作为事件，使用既有序列号和 replay cursor；恢复时重放未完成节点并重新 heartbeat，禁止凭内存状态直接宣告 close。
- **工具执行与批准：** `src/tool_runtime.rs` 的 `ToolBatchOptions/ToolBatchDecision/ToolBatchOutcome` 与 `execute_tool_batch`（L113-L168）适合承载节点处理中的有界工具批次；`src/approval.rs` 的 `ApprovalCoordinator` 请求/响应、取消及持久化 accepted 机制（L126-L200、L374-L445）可复用为 worker 的权限门。DAG 派生不得绕过 approval；为子 worker 传递 parent 的 `policy_digest`/lease correlation，并在关闭前确认没有 pending approval/tool batch。
- **providers/**：provider adapter 只负责模型请求、streaming 和错误归一化；验证规则中的 model、streaming、SDK-specific error 对应 Rust 侧 provider 请求/事件检查。provider 不应决定 DAG close；它只返回节点结果，close gate 由 core/session DAG service 决定。

建议的可执行、可验证改动（仅映射，不在本次任务修改源码）：在 `src/core.rs` 定义并验证 `DagNodeRecord` 与 `can_close`；在 `src/session.rs` 增加 DAG 事件/索引的 append/replay；在 `src/protocol.rs` 增加受限 dag status/derive/close 命令并复用 mailbox schema；在 `src/runtime.rs` 用 `BackgroundRunner` 派生与取消；在 `src/headless.rs` 接入命令和事件输出；在 `src/tool_runtime.rs`、`src/approval.rs` 复用工具批次及批准；在 `src/providers/**` 验证 provider 结果后回传 core。验证时构造 parent→child→grandchild 与 sibling 图，分别测试四向消息、断线恢复、后代未绿时 close 被拒绝且 heartbeat/derive 生效、全绿后 close 只发生一次。

## 未决问题

- 源文件没有规定官方 SDK 的具体版本下限、Node.js 精确版本、模型白名单、MCP/subagent/session 的字段级 schema 或错误码（相关要求仅见 L15-L18、L29-L35、L57-L65）。
- 源文件没有定义 WebFetch 不可用、`npx tsc --noEmit` 超时或依赖安装失败时的统一状态映射；verifier 需要在报告中明确证据不足。
- 源文件没有规定报告是否持久化、重试次数、并发审查数或取消语义；zenpi 映射中的 DAG lease、close gate 和派生策略属于基于现有 Rust 原语的设计建议。


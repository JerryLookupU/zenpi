# AC-003 — claude-code/plugins/agent-sdk-dev/README.md

- source_id/item_id：`AC-003`
- source_path：`claude-code/plugins/agent-sdk-dev/README.md`
- source_hash：`7f9bc2aac9c7c546e550e2fa2e2d9a66d7ab6bb7dea01a2e07147f78981ba3ee`
- source_bytes：`6397`
- source_lines：`208`
- coverage：已从字节 `1-6397`、行 `L1-L208` 按顺序读取，包含标题、注释性说明、代码块、列表、链接、作者和版本。

## 完整行为复盘

该文件是 Agent SDK Development Plugin 的使用说明，不是实现源码；因此“导出符号”是一个命令和两个 verifier agent，而不是 Rust 函数。插件目标是覆盖 Claude Agent SDK 应用从脚手架到验证的完整生命周期，并支持 Python 与 TypeScript（L1-L7）。README 没有声明可配置的 API、返回类型或并发线程模型，下面把每个公开入口的输入、产出和边界逐项还原。

- **命令 `/new-sdk-app`**（L11-L23）：交互式创建 Claude Agent SDK 应用。输入可以是可选的项目名（`/new-sdk-app my-project-name`，L24-L32）；如果省略项目名，命令在交互阶段补问。必问字段按顺序是语言 `TypeScript` 或 `Python`、项目名、agent type（`coding`、`business`、`custom`）、starting point（`minimal`、`basic` 或 specific example）、tooling preference（TypeScript 为 `npm/yarn/pnpm`，Python 为 `pip/poetry`）（L34-L39）。执行输出是最新 SDK 版本检查/安装、项目文件和配置、`.env.example`、`.gitignore`、按用例定制的可运行示例，以及 TypeScript 的 type checking 或 Python 的 syntax validation（L15-L22）。命令完成后自动调用对应 verifier agent（L21-L22）；README 没有规定取消键、超时、重试次数或失败时的回滚。
- **命令示例的可核对结果**（L41-L48、L121-L143）：`/new-sdk-app customer-support-agent` 或 `code-reviewer-agent` 应产生一个 customer-support/code-review 用例的项目，选择 TypeScript 或 Python 环境，安装 latest SDK，并自动验证。工作流随后把 `ANTHROPIC_API_KEY` 写入 `.env` 并以 `npm start` 启动 TypeScript 示例（L136-L143）。这是文档建议，不表示插件替用户取得或持久化真实密钥。
- **`agent-sdk-verifier-py`**（L50-L81）：输入是一个 Python Agent SDK 应用，触发条件为新项目创建后、修改已有项目后或部署前（L63-L67）。可自动触发，也接受自然语言请求 `Verify my Python Agent SDK application` 或 `Check if my SDK app follows best practices`（L68-L73）。检查集合是 SDK 安装及版本、Python 环境文件（`requirements.txt`/`pyproject.toml`）、SDK 用法与模式、agent 初始化/配置、`.env` 与 API key 安全、错误处理与功能、文档完整性（L54-L61）。输出是综合报告，包含 `PASS`、`PASS WITH WARNINGS` 或 `FAIL`，阻断功能的 critical issues、次优模式 warnings、通过项清单，以及带 SDK 文档引用的具体建议（L75-L81）。没有给出检查顺序、并发方式、阈值或机器可读 schema。
- **`agent-sdk-verifier-ts`**（L83-L115）：输入是 TypeScript Agent SDK 应用，触发条件与 Python verifier 相同（L97-L100），可由创建命令自动调用，或使用 `Verify my TypeScript Agent SDK application`/`Check if my SDK app follows best practices`（L102-L107）。检查 TypeScript 配置（`tsconfig.json`）、SDK 安装及版本、正确用法与 imports、类型安全、agent 初始化/配置、环境与 API key 安全、错误处理/功能和文档完整性（L87-L95）。输出格式同样包含三态总体结果、阻断问题、warnings、通过项和文档引用建议（L109-L115）。
- **安装与使用前提**（L150-L155）：插件随 Claude Code repository 提供；前提是 Claude Code 已安装，命令和 agents 自动可用。没有独立安装参数、版本锁定格式或服务端要求。
- **持续行为判据/规范**（L157-L164）：每次创建使用 latest SDK；部署前运行 verifier；不提交 `.env`、不硬编码 API key；遵循官方 SDK patterns；TypeScript 定期运行 `npx tsc --noEmit`；为 agent 功能建立 test cases。这里“latest”是运行时检查策略，README 未定义网络失败时的 fallback。
- **资源和诊断入口**（L166-L171、L173-L200）：官方 overview、TypeScript/Python reference、examples 链接作为 verifier 建议的依据。TypeScript 类型错误时，先依赖创建命令的自动 type checking，再检查 latest SDK 与 `tsconfig.json`（L175-L183）；Python `claude_agent_sdk` import 错误时运行 `pip install -r requirements.txt`、激活 virtual environment、用 `pip show claude-agent-sdk` 检查安装（L184-L192）；warnings 只表示改进项，不阻止功能，但应阅读报告和引用文档（L193-L200）。
- 文档元信息为作者 Ashwin Bhat、版本 `1.0.0`（L202-L208）。README 没有定义导出函数、数据类型、异常类或锁，因此不能从源文件推断更细的 API 语义；任何 Rust 映射都是基于行为的适配建议。

## 状态、取消、恢复与副作用

状态可从文档行为抽象为 `未创建 → 交互收集 → 脚手架完成 → 本地校验完成 → verifier 报告`；verifier 的终态明确是 `PASS`、`PASS WITH WARNINGS`、`FAIL`（L75-L81、L109-L115）。`PASS WITH WARNINGS` 仍可工作，`FAIL` 表示有阻断功能的问题，但 README 未规定进程退出码。

取消、超时、重试、断点恢复和持久化策略均未在源内定义；“自动验证”只说明触发关系，不说明 verifier 与创建器是同一进程还是两个并发 worker（L21-L22、L68-L73、L102-L107）。可确认的外部副作用是：检查/安装 latest SDK，创建项目文件与配置，写 `.env.example`/`.gitignore`，运行 TypeScript/Python 本地校验，以及用户按示例写 `.env`、启动 `npm start`（L15-L22、L136-L143）。API key 只被要求安全管理，不应进入提交历史（L159-L162）。

## 源内测试与行为判据

源内未包含测试；该 README 只有说明、命令块、资源链接和 troubleshooting（全文件 L1-L208）。可独立验证的判据如下：

1. 在已安装 Claude Code 的环境运行 `/new-sdk-app demo`，记录交互字段是否依次覆盖 language、name、agent type、starting point、tooling（L24-L39），并检查生成 `.env.example`、`.gitignore`、依赖配置和示例入口（L15-L21）。
2. 对 Python 产物检查 `requirements.txt` 或 `pyproject.toml`、`claude_agent_sdk` import、API key 不在源码中，然后触发 `agent-sdk-verifier-py`；对 TypeScript 产物检查 `tsconfig.json`、imports，并运行 `npx tsc --noEmit`（L54-L61、L87-L95、L163-L164）。
3. 记录 verifier 是否输出三态总体状态、critical/warning、passed checks 和带文档引用的 recommendations（L75-L81、L109-L115）。
4. 故意制造类型错误、缺少 Python 依赖和 warning，确认诊断分别指向 latest SDK/`tsconfig.json`、`pip install -r requirements.txt`/virtual environment/`pip show`、报告中的建议且 warning 不阻止功能（L175-L200）。

## zenpi Rust 映射

**可复用原语。** README 的“命令询问→创建→验证→报告”可落到 `src/protocol.rs` 的 `StdioRequest`/`Command`/`StdioResponse`/`StdioEvent`：输入用有界 JSONL 请求，过程事件用 `StdioEvent` 的 `sequence`、`request_id`、`turn_id` 关联，终态用 `StdioResponse` 的 `success`/`error_code`（`StdioEvent::new` 与响应构造在 `src/protocol.rs:L882-L999`）。直接通信优先复用 `src/session.rs` 的 `SessionMailbox`、`MailboxMessage`、`MailboxAction` 和 `MailboxStatus`（`L3170-L3265`、`L3570-L3775`）：digest 寻址、TTL、claim token、幂等 request ID、`Queued → Claimed → Succeeded/Failed` 正好适合 agent 节点消息/邮箱；实时进度用 `src/headless.rs` 的有界 provider/agent event mailbox 与 replay（`MAX_ASYNC_*` 在 `L33-L52`，异步事件排放在约 `L4251-L4395`）。协议层事件是观察通道，邮箱是可重放的命令/结果通道，二者不可混为一个终端结果。

**DAG 会话父子关系和地址。** 在 `src/session.rs` 的 `SessionStore` 记录上新增/复用一个 DAG metadata 结构（建议字段 `node_id`、`parent_session_id`、`grandparent_session_id`、`relation`、`child_ids`、`status`、`generation`），节点身份继续使用 session ID；`src/core.rs` 的 `Turn.parent_id`/`Turn::with_parent`（`L140-L175`）可保存一次 turn 的直接因果父项，但不能单独表达全部 DAG。发送方把 `recipient_session_id` 指向 parent、grandparent、直接 sibling 或直接 child，关系和 `goal_id/item_id` 放进受验证的 JSON payload；`SessionMailbox::check_access` 已要求双方是同一 workspace 且 addressed session（`src/session.rs:L3598-L3614`），可作为跨节点授权边界。`src/protocol.rs::MailboxRequest`（`L89-L113`）提供 Send/Receive/Acknowledge/Claim/Complete；可扩展一个受 `deny_unknown_fields` 保护的 `relation`，而不允许客户端伪造 sender identity（该身份刻意不在 client fields，L87-L89）。

**保活、派生和 close 的最小可执行机制。** `src/session.rs::LiveSessionRegistry` 已有 `register`、`heartbeat`、`active`、`unregister`、owner epoch 和 TTL（`L3274-L3359`），可作为每个 DAG worker 的 lease/保活；`claim_next`/`claim_message`/`finish_claim`/`finish_claim_with_reply`（`L3361-L3559`）可作为邮箱消费和结果回送。建议新增一个纯函数 `dag_closeable(node, graph) -> bool`：仅当自身 `status == Green` 且递归遍历的全部 child/grandchild 都为 `Green` 才返回 true；存在 pending/running/failed/unknown child 时返回 false。`src/headless.rs` 的 DAG/goal 调度落点（`run_blueprint_next`、`run_goal_steps`、`transition_goal_status`，约 `L3175-L3360`、`L7340-L7350`）在 close 请求前调用该判据：false 时写入 keepalive 事件、刷新 `LiveSessionRegistry::heartbeat` 并保留 mailbox owner；发现新工作则创建带 `parent_session_id` 的新 session/worker，发送 `MailboxRequest::Send`，只有所有后代 Green 才发 `close`。最小派生器可封装 `BackgroundRunner` 的 job closure：`BackgroundRunner::spawn`、`try_submit`、`RuntimeEvent::{Accepted,Started,Queued,Completed,Closed}`（`src/runtime.rs:L236-L365`）提供有界提交、FIFO pending 和完成事件；派生请求被拒绝（`QueueFull`/`Closed`）必须保留 DAG 节点为非 Green 并继续保活。

**取消、错误和副作用落点。** `src/runtime.rs::CancellationToken` 是协作取消且幂等，job 必须在 provider chunk、邮箱 claim 前和重试前检查；`try_shutdown_with_grace` 的 grace 有上界，detach 不会回滚副作用（`src/runtime.rs:L49-L99`、`L327-L390`），因此 DAG 节点 close 不能把 detached job 当作 Green。`src/core.rs::AgentPhase`、`AgentEvent`、`TurnSubmission`（`L244-L325`）可映射节点的 idle/running/closed、accepted/rejected/tool/provider/error 事件；`AgentError::Recovery` 与 `SessionStore::interrupted_operations`/`acknowledge_recovery`（`src/core.rs:L653-L671`）用于重启后把未完成节点标为需要显式 retry。`src/approval.rs::ApprovalCoordinator` 的 request/response、pending drain、cancel_all（`L126-L220` 及公开方法约 `L251-L452`）负责 worker 派生涉及工具副作用时的审批；`src/tool_runtime.rs::execute_tool_batch` 已规定有界并发、整批校验、取消轮询和每调用终态（`L112-L230`），可复用于新 worker 的工具执行。

**文件级落点和 provider 边界。** `src/headless.rs` 负责 stdin/stdout JSONL、事件 replay、runtime lifecycle event 和最终 owner cleanup（`run_headless`/`run_async_streams` 约 `L556-L806`，`close_async_owners` 约 `L4898-L4949`），适合实现 DAG 节点进度、keepalive、派生/close 响应。`src/core.rs` 保存 Agent 会话状态、turn parent、worker binding 与资源/预算，适合加入 DAG 状态机而不直接执行 wire I/O。`src/session.rs` 负责 append-only journal、mailbox sidecar、live owner 和恢复；`src/runtime.rs` 负责 worker 生命周期；`src.protocol.rs` 负责稳定序列化和请求校验；`src.approval.rs`/`src.tool_runtime.rs` 负责副作用闸门。`src/providers/**` 仍只处理 provider route/protocol/capabilities：`src/providers/mod.rs` 的 `Protocol`、`ProviderDefinition`、`RouteRule`（`L13-L180`）和 registry 的 `ModelDescriptor::effective_capabilities`（`src/providers/registry.rs:L74-L115`）可用于 verifier 的“SDK/模型能力”检查，但 DAG parent/sibling/child 通信不应塞进 provider codec；provider 事件由 `ProviderEvent` 经 `headless` 关联到 node/turn。

**差异清单（可执行、可验证）。**

- README 有 latest SDK 安装和三态 verifier 报告，zenpi 尚无专门的 `SdkVerifierReport`；建议在 `src/core.rs` 或新 `src/agent_sdk.rs` 定义 `CheckStatus::{Pass,PassWithWarnings,Fail}`、检查项和文档引用，并通过 `StdioResponse.data` 输出。
- README 只描述线性创建工作流，zenpi 需要新 `DagNodeState`、关系字段、递归 Green close 判据和 keepalive/derive 事件；为该判据添加单元测试：自身 Green、一个 child 非 Green、grandchild 非 Green、全部 Green 四种情况。
- `SessionMailbox` 已支持地址、TTL、claim、幂等和 reply，但未表达 DAG relation；扩展 payload schema 后验证 parent→child、grandparent→node、sibling→sibling、child→parent 四条边，拒绝跨 workspace、过期、错误 token 和重复 request。
- `LiveSessionRegistry` 是 liveness admission，不会自动启动 scheduler（`src/session.rs:L3274-L3276`）；因此派生必须由 `headless`/`runtime` 显式 `try_submit`，测试 queue full、worker closed、heartbeat 过期时节点保持非 Green。
- provider verifier 的网络/依赖安装是外部副作用，现有 `src/providers/**` 只应提供路由和能力声明；测试应使用 fake backend/`EchoBackend`，验证报告生成不需要真实 API key。

## 未决问题

1. README 未说明 `/new-sdk-app` 如何处理 SDK 下载失败、依赖冲突、用户中途取消、超时和重试，也未定义任何持久化 checkpoint。
2. README 未确认 verifier 是否并行运行、是否有退出码、报告 JSON schema、检查阈值或版本兼容矩阵。
3. README 未定义 Agent SDK 项目与 Claude Code session 的身份关系，因此 DAG 中 parent/grandparent/sibling/child 的 session ID 绑定需要 zenpi 自行确定。
4. README 没有“节点及全部后代 Green 才 close”的规则；该规则是本项目映射要求，必须由 zenpi DAG 状态机和测试明确实现。

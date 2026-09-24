# AC-004 — claude-code/plugins/agent-sdk-dev/agents/agent-sdk-verifier-py.md

- source_id/item_id：`claude-code/plugins/agent-sdk-dev/agents/agent-sdk-verifier-py.md` / `AC-004`
- source_path：`claude-code/plugins/agent-sdk-dev/agents/agent-sdk-verifier-py.md`
- source_hash：`a72c50412f4f9940932cac0f3ca61d6ced7354b8ac8892e3f1507bcd2957006e`
- source_bytes：5206
- source_lines：140
- coverage：已按原文件顺序读取字节 `0-5205`、行 `L1-L140`，含 front matter、注释、全部清单与报告格式。

## 完整行为复盘

该文件是给 Python Agent SDK 应用使用的“验证工人”说明文件，不含 Python/Rust 函数定义、类定义或可执行导出符号；因此没有可逐函数调用的返回值。可审计的导出配置只有 front matter：`name: agent-sdk-verifier-py`、`description`、`model: sonnet`（L1-L5）。角色边界是“彻底检查 Python Agent SDK 应用是否正确使用 SDK、符合官方文档并可部署”，输出应是验证报告而不是修改应用（L7-L7）。

验证优先级由八组检查组成。第一组安装与配置要求检查 `claude-agent-sdk` 是否在 `requirements.txt`、`pyproject.toml` 或 pip 环境中，版本不能过旧，Python 通常需 3.8+，并确认虚拟环境建议已记录（L9-L18）。第二组检查可复现的 Python 环境：依赖文件、版本约束与完整依赖声明（L20-L25）。第三组检查 SDK 使用模式：从 `claude_agent_sdk` 的正确导入、按文档初始化 agent、system prompt/model 等配置、参数合法性、streaming 或 single response 处理、权限与 MCP server 集成（L27-L35）。这些条目是验证输入的边界：缺文件、错误导入、错误参数、错误响应模式或未经约束的权限均应记录问题；文件存在本身不代表实现正确。

第四组只做与 SDK 相关的基础代码质量核验：语法、导入可用性、错误处理和结构合理性（L37-L42）。第五组检查环境和安全：`.env.example` 应含 `ANTHROPIC_API_KEY`，`.env` 应进 `.gitignore`，源码不得硬编码密钥，API 调用要有错误处理（L44-L49）。第六组按官方最佳实践复核 system prompt 结构、模型选择、权限范围、MCP 自定义工具、subagents 和 session handling（L51-L58）。第七组验证整体初始化和执行流、SDK 特定错误覆盖以及是否遵循文档模式（L60-L65）。第八组验证 README、虚拟环境安装步骤、自定义配置说明和清晰的安装指引（L67-L71）。

明确排除一般 PEP 8、命名风格、snake_case/camelCase、导入排序及与 SDK 无关的 Python 偏好（L73-L78）。这意味着“风格不同”不应单独导致失败，除非它造成 SDK 功能、可复现性或安全问题。

验证过程有四个顺序阶段。先读取 `requirements.txt`/ `pyproject.toml`、主应用文件、`.env.example`、`.gitignore` 与配置文件（L80-L87）；再使用 `WebFetch` 访问官方文档 `https://docs.claude.com/en/api/agent-sdk/python`，对照官方模式并记录偏差（L89-L93）；然后检查全部导入、明显语法错误和 SDK 导入（L95-L99）；最后核对 SDK 方法、配置选项和官方示例模式（L101-L104）。未定义超时、并发度、重试次数或自动修复动作；验证器的“输入”是被检查的项目文件和外部 SDK 文档，“输出”是报告四部分。

报告必须给出 `Overall Status`，取 `PASS | PASS WITH WARNINGS | FAIL`（L106-L112）。`Critical Issues` 收集阻止运行的问题、安全问题、会导致运行时失败的 SDK 错误以及语法/导入问题（L114-L119）；`Warnings` 收集次优模式、遗漏的 SDK 能力、文档偏差和缺少文档（L121-L126）；`Passed Checks` 记录正确配置、正确实现的 SDK 特性和安全措施（L128-L132）；`Recommendations` 必须是具体改进、文档引用和下一步增强（L134-L138）。最后要求彻底但建设性地帮助开发者构建可运行、安全、配置正确且遵循官方模式的应用（L140-L140）。

## 状态、取消、恢复与副作用

源文件没有实现状态机、取消 token、超时、重试、恢复、持久化或执行副作用。它只规定验证器检查目标应用是否正确处理这些相关问题：错误处理应覆盖 API/SDK 错误（L37-L42、L60-L65），session handling 应符合最佳实践（L51-L58），权限、MCP、subagents 要被核验（L27-L35）。`WebFetch` 是验证阶段的外部读取副作用，且来源被限定为官方 Python SDK 文档（L89-L93）；没有写回项目、部署或自动修复授权。并发语义也未定义：streaming 与 single mode 只是被检查的应用行为（L33-L34），不能从源文件推导 worker 并发策略。报告状态的默认值、失败升级规则和重试次数未说明，故应由宿主实现而不能臆造。

## 源内测试与行为判据

源文件或同目录未包含测试。可独立验证的判据是：逐项检查 L13-L18、L22-L25、L29-L35、L39-L42、L46-L49、L53-L58、L62-L65、L68-L71；用 Python 语法检查器验证无语法错误，用可导入环境验证 `claude_agent_sdk` 导入与版本，用最小调用验证初始化/响应路径，并人工对照官方 URL 的当前参数；确认密钥不在源码、`.env` 被忽略。报告必须包含四个固定区块和三态 Overall Status（L106-L138），且排除项不能被误报为 SDK 缺陷（L73-L78）。

## zenpi Rust 映射

现有代码已经提供可复用的通信原语。跨进程/跨会话优先复用 `src/protocol.rs` 的 `MailboxRequest::{Send,Receive,Acknowledge,Claim,Complete}`、`MailboxOutcome`、带 `session_id` 的请求字段以及版本化 JSONL；该协议已有消息大小、TTL、ID 校验（对应 `src/protocol.rs` L1-L110、L520-L560）。持久邮箱复用 `src/session.rs` 的 `MailboxMessage`、`MailboxStatus`、`SessionMailbox::{enqueue,list,update}` 和 `LiveSessionRegistry::{register,heartbeat,claim_next,claim_message,finish_claim,finish_claim_with_reply}`（约 L3171-L3570、L3574-L3798）；它提供 digest、TTL、claim token、幂等完成/失败和同 workspace 约束。进程内事件/响应复用 `src/headless.rs` 的有界 replay/event mailbox 与请求 ID 相关性；不要另造无限队列。工具/审批事件可复用 `src/approval.rs` 的 `ApprovalCoordinator`、`ApprovalRequest` 与取消广播（`cancel_all`），只把它当策略闸门，不当 DAG 拓扑存储。provider 流式增量继续走 `src/providers/**` 的 `ProviderEvent`/streaming 能力，并由宿主转成节点事件。

建议的会话关系模型：在 `src/session.rs` 的 durable record 或现有 tree/handoff 记录中增加受校验的 `DagNodeContext`（`node_id`、`parent_session_id`、`grandparent_session_id`、直接 sibling 会话 ID 列表、直接 child 会话 ID 列表、`generation`、`required_descendant_count`）；在 `src/core.rs` 的 `Agent` 建立/校验上下文，并让 `Turn.parent_id`（约 L146-L193）保留会话内父子链。跨 session 关系必须同时写进 handoff/tree 事件，避免仅凭内存指针推断祖先。

DAG 节点通信的最小可执行机制如下：节点启动时由 `src/core.rs` 注册 `LiveSessionOwner`（已有 `register_live_owner`、`heartbeat_live_owner`、`claim_live_mailbox`、`finish_live_mailbox`，约 L820-L875），为 parent、grandparent、每个直接 sibling、每个直接 child 各发送一个带 `message_id`/TTL/拓扑 epoch 的 mailbox envelope；接收端按 `claim_message` 原子认领，处理后 `finish_claim_with_reply` 回原发送者。消息类型至少包括 `NodeStarted`、`Progress`、`Green`、`Blocked`、`CloseRequest`、`SpawnRequest`、`Cancel`，payload 必须携带 `node_id`、`generation`、依赖摘要和幂等键。这样 parent/grandparent/sibling/child 全都使用同一 mailbox 原语，事件只用于本进程实时唤醒，持久 mailbox 才是恢复依据。

保活与派生的最小机制：`src/runtime.rs` 的 `BackgroundRunner::try_submit/try_cancel/try_shutdown_with_grace`、`RuntimeEvent::{Accepted,Started,Queued,CancelRequested,Completed,Closed}` 与 `CancellationToken`（约 L40-L190、L272-L424）可承载一个节点 worker；心跳由 `LiveSessionRegistry::heartbeat` 定时完成。节点不得在自身 `Green` 时立即 close：只有节点自身状态为 Green，且直接 child 的结果均为 Green，并且递归汇总的全部 child/grandchild closure proof（数量、generation、摘要）完整一致，才写 `NodeClosed` 并允许 `shutdown_and_join`。任一 child 非 Green、过期、缺失、Blocked 或结果未知时，节点保活、继续 heartbeat，并向 `SpawnRequest` 派生新 worker；派生前由 `src/core.rs` 的 worker binding/lease、`src/approval.rs` 的策略校验与 `src/tool_runtime.rs` 的批处理闸门持久化意图，防止重复副作用。新 worker 必须带新的 `generation/lease_id`，旧 worker 的 late completion 按幂等键拒绝覆盖新状态。

各文件落点和差异：`src/headless.rs` 接收/发出 JSONL mailbox、事件和 DAG 状态，并利用已有 replay/有界队列；差异是需增加 DAG 消息路由与终端 close 判据。 `src/core.rs` 是节点状态机、父祖先/兄弟/子节点寻址、心跳、聚合 closure proof 和派生入口；差异是当前已有 live mailbox/worker 取消能力，但未见统一的“全后代绿色才关闭”聚合器。 `src/session.rs` 负责 durable DAG edge、mailbox、claim/complete、恢复扫描；差异是需新增拓扑/绿色证明记录并让 GC 保留未闭合节点及其 mailbox。 `src/tool_runtime.rs` 继续执行工具批次并传播 cancellation；差异是派生请求应先过批次的 side-effect/approval gate。 `src/runtime.rs` 负责有界队列、取消、graceful shutdown 和新 worker 生命周期；差异是需要一个由 core 驱动的 spawn supervisor，而不是让 runtime 自行理解 DAG。 `src/protocol.rs` 增加严格的 DAG message/tree action 变体、拓扑 epoch、generation 和 closure proof 字段；现有 `TreeAction`（约 L1160-L1205）可作为树控制入口。 `src/approval.rs` 复用 worker allow/preflight、取消和审批持久化语义，禁止消息本身提升权限。 `src/providers/**` 仅提供模型路由、streaming/provider event；需要把取消检查放在流式 chunk/retry 边界，并把 provider 失败转换为 `Blocked` 或 `Unknown`，不能直接标 Green。

可验证的实现判据：为一个 parent、grandparent、两个 sibling、两个 child 和一个 grandchild 建立 session；断言每封 mailbox 消息只被一个 owner claim，重放相同 `message_id` 不重复执行；断言 child 未绿时 parent 不产生 `NodeClosed`，heartbeat 仍更新；断言 child 超时/未知会产生带新 generation 的 `SpawnRequest`；全部后代 Green 且 proof 摘要匹配后才允许 close；重启后从 `src/session.rs` journal/mailbox 恢复拓扑、未完成 claim 和保活状态。该验证应覆盖协议边界、TTL、取消竞态、旧 generation late result 和 provider streaming 错误。

## 未决问题

无法从源文件确认：官方 Python SDK 的具体版本下限；验证报告是否需要机器可读 JSON；WebFetch 失败时是否允许缓存文档；streaming 与 single mode 的性能阈值；subagents/session 的具体配置字段；以及 DAG closure proof 的具体哈希算法、心跳 TTL、派生预算和最大后代数量。这些均未在源文件 L1-L140 定义。


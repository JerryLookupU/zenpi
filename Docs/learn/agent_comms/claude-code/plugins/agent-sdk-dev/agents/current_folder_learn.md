# `claude-code/plugins/agent-sdk-dev/agents` 目录级汇总

## 范围与目录职责

本目录包含两个面向 Claude Agent SDK 示例工程的“验证工人”规格文件。它们不是可执行的 Python 或 TypeScript 实现，而是定义审查者的角色、输入边界、检查顺序和报告格式：在应用新建或修改后，检查 SDK 安装、配置、初始化、流式/单响应处理、权限、MCP、subagents、session handling、错误处理、安全和文档，最后输出 `PASS`、`PASS WITH WARNINGS` 或 `FAIL`。笔记源实际位于 `/Users/wangweiyang/GitHub/claude-code/plugins/agent-sdk-dev/agents`；用户给出的 `/Users/wangweiyang/GitHub/Docs/...` 前缀在当前工作区不存在，但两份对应 1:1 笔记已存在于 zenpi 的 `Docs/learn/agent_comms/files/...` 映射目录。

目录的核心职责是“验证规则编排”，不是执行 agent、保存 DAG 或直接修复项目。两份文件共享同一审查骨架：先读取工程配置和入口，再对照官方 SDK 文档，执行语言工具链检查，最后按严重度整理报告；同时明确排除一般代码风格，避免把与 SDK 无关的问题误判为失败。

## 模块清单

- `agent-sdk-verifier-py.md`：定义 Python Agent SDK 验证工人的检查清单与四段报告；关键导出是 front matter 中的 `name: agent-sdk-verifier-py`、`description`、`model: sonnet`，以及 `Overall Status`、`Critical Issues`、`Warnings`、`Passed Checks`、`Recommendations` 五类输出约束。
- `agent-sdk-verifier-ts.md`：定义 TypeScript Agent SDK 验证工人的检查清单与报告；关键导出同样是 `name`、`description`、`model: sonnet` 和五类报告字段，但额外要求核对 `package.json` 的 `type: module`、`tsconfig.json`、Node.js `engines`、构建脚本，并执行 `npx tsc --noEmit`。

Python 文件重点检查 `claude-agent-sdk`、Python 版本/依赖文件、`claude_agent_sdk` 导入、system prompt/model、streaming 或 single response、MCP、权限、`.env.example` 与 `.gitignore`。TypeScript 文件对应检查 `@anthropic-ai/claude-agent-sdk`、ES module 编译配置、类型定义、build/start/typecheck 脚本和 `npx tsc --noEmit`。两者都要求核对官方文档、SDK-specific error、subagents/session handling，并把硬编码密钥、编译/导入失败、会导致运行时失败的问题提升到 `Critical Issues`。

## 运行时数据流与控制流

这两个文件描述的是验证流水线而非真正的 worker 运行时。控制流为：读取依赖与配置文件 → 读取主应用和环境文件 → 使用 `WebFetch` 对照官方 Python/TypeScript SDK 文档 → 检查导入、初始化、参数、权限、MCP、streaming/single response 与 session → Python 做语法/导入核验，TypeScript 执行 `npx tsc --noEmit` → 核对 README、安装和运行说明 → 生成结构化验证报告。数据输入是目标工程文件、SDK 配置和官方文档；数据输出是检查结果和改进建议。源文件没有规定并发 worker、超时、自动修复、持久化或报告机器格式。

映射到 zenpi 时，应把“验证报告”理解为节点工作结果或审查事件，而不是把 verifier 文本直接变成 agent API。一个节点可以产生 `Started`、`Progress`、`Green`、`Blocked` 或 `Unknown` 结果；父节点依据持久化状态和后代证明决定是否关闭。`PASS` 可映射为节点自身验证完成，但不能单独等价于 DAG `close`；只有本节点以及全部 child/grandchild 后代递归为 Green，才可产生 `NodeClosed`。

## 错误、取消与副作用语义

源文件只规定“被审查应用必须正确处理” SDK/API 错误，并没有定义 verifier 自身的异常类型、取消 token、重试次数、恢复格式或幂等键。`WebFetch` 和 TypeScript 的 `npx tsc --noEmit` 是验证步骤，不是被测应用的副作用；文档或编译不可用时，验证器应报告证据不足或失败，而不是臆造通过。风格问题、命名方式、`type`/`interface` 选择等明确不应单独触发 `FAIL`。

在 zenpi 中应沿用协作式取消：`src/runtime.rs` 的 `CancellationToken::cancel`、`is_cancelled`、`mark_completed` 让 worker 在流式 chunk、重试、工具批次和消息处理边界检查取消；取消不是回滚，非协作任务只能在 shutdown grace 后脱离。`JobOutcome::{Succeeded,Failed,Cancelled,Panicked}` 和 `RuntimeEvent::{Accepted,Started,Queued,CancelRequested,Completed,Closed}` 应被转成节点生命周期事件。队列满是 `SubmitError::QueueFull`，运行时关闭是 `SubmitError::Closed`；二者都不能被误报为节点 Green。provider/API 失败应落为 `Blocked` 或 `Unknown`，不能直接发布 Green。

## 与 zenpi Rust 的映射建议

### 通信原语与邻接关系

优先复用 `src/protocol.rs` 的 `MailboxRequest::{Send,Receive,Acknowledge,Claim,Complete}`、`MailboxOutcome`、`message_id`、`recipient_session_id`、`ttl_ms` 和有界 JSONL 协议；复用 `src/session.rs` 的 `MailboxMessage`、`MailboxStatus`、`SessionMailbox::{enqueue,list,update}` 与 `LiveSessionRegistry::{register,heartbeat,claim_next,claim_message,finish_claim,finish_claim_with_reply}`。持久 mailbox 是恢复依据，进程内事件仅用于实时唤醒和 replay，不应另造无限队列。`src/headless.rs` 的 `execute_mailbox`、请求 ID 相关性、bounded event/terminal replay 可作为 JSONL transport adapter。工具审批事件继续复用 `ApprovalCoordinator`、`ApprovalRequest` 与 `cancel_all`；provider 的 `ProviderEvent`/streaming 只负责增量结果，不拥有 DAG 拓扑和 close 权限。

节点启动时必须由持久化 DAG 索引解析并验证四类通信目标：`parent`、`grandparent`、直接 `sibling`、直接 `child`。每个目标都通过同一 mailbox envelope 发送，而不是为 sibling 或祖父节点建立特殊通道。建议的最小消息类型为 `NodeStarted`、`Progress`、`Green`、`Blocked`、`CloseRequest`、`SpawnRequest`、`Cancel`；payload 至少带 `node_id`、`generation`、`topology_epoch`、依赖摘要、`message_id`/幂等键和发送者/接收者 `session_id`。接收端原子 `Claim`，处理后 `Complete` 或 `finish_claim_with_reply`，重放相同 `message_id` 不得重复执行。

### 会话父子关系与持久化

`src/core.rs` 现有 `Turn { parent_id, ... }`、`Turn::with_parent`、`Turn::validate` 适合表达单会话内的对话父链；`src/session.rs` 的 journal、handoff、tree snapshot/ancestry/fork 和 runtime intent 适合保存可恢复记录，但 DAG 节点拓扑不应只藏在内存指针。建议增加受校验的 `DagNodeRecord`/`DagNodeContext`，至少包含 `node_id`、本节点 `session_id`、`parent_session_id`、`grandparent_session_id`、直接 sibling/child ID、`generation`、`lease_id`、节点状态、必需后代数量和 closure proof 摘要。启动、恢复和重连时从 journal 重建关系，拒绝跨 workspace 目标、重复 edge、缺失父节点和过期 lease。

### 保活、派生与 close 门槛的最小机制

`src/core.rs` 已暴露 `register_live_owner`、`heartbeat_live_owner`、`claim_live_mailbox`、`finish_live_mailbox`，可作为节点 owner 和邮箱消费入口；`src/runtime.rs` 的 `BackgroundRunner::spawn`、`try_submit`、`try_cancel`、有界 `RuntimeConfig` 和 graceful shutdown 可承载/派生 worker。最小闭环是：节点注册 owner 并定期 heartbeat；收到新工作或发现后代未完成时，经 `core` 校验 `lease_id`/策略后提交新的 `DagWorkerRequest`；新 worker 使用新的 `generation`，旧 worker 的 late completion 按幂等键拒绝覆盖新状态；节点持续保活直到可关闭。

`can_close(node_id)` 必须是 core/session 层的聚合判定，而不是 `BackgroundRunner` 看到一次 `Succeeded` 就关闭：只有节点自身为 Green，且所有直接 child、grandchild 及更深后代递归均为 Green，且 closure proof 的数量、`generation`、摘要和拓扑 epoch 一致，才追加 `NodeClosed` 并允许注销 owner/`shutdown_and_join`。任一 child/后代为 `Blocked`、`Cancelled`、过期、缺失、未知、仍有 pending mailbox/tool batch/approval，节点都必须保活；当出现新工作时，派生 worker 处理新工作并继续发送 `Progress`/`SpawnRequest`。这满足“节点自身及其全部 child/grandchild 全绿才 close，否则保活并派生新 worker”的 DAG 需求。

### 四个明确落点

- `src/headless.rs`：在现有逐帧 JSONL 读取、请求 ID admission、`handle_command`、有界 runtime/event replay 入口接入 DAG/mailbox 命令和 `StdioEvent`；EOF/shutdown 只取消可取消工作，不得绕过 close gate 关闭仍有后代工作的节点。
- `src/core.rs`：持有 `Agent` 级 DAG 状态机和邻接寻址，负责 `DagNodeRecord` 校验、四向消息路由、worker lease、heartbeat、Green/Blocked 聚合、`can_close`、`SpawnRequest` 和旧 generation 结果过滤；这里应统一协调已有 `LiveSessionRegistry`，而不是让 runtime 自行理解 DAG。
- `src/session.rs`：追加/恢复 DAG edge、状态事件、heartbeat、派生意图、closure proof 和 mailbox claim/complete；复用 append-only journal、`SessionMailbox`、`LiveSessionRegistry` 和 operation recovery，GC 必须保留未闭合节点及相关 mailbox。
- `src/runtime.rs`：继续负责有界命令/事件队列、`CancellationToken`、worker 执行、取消竞态、`JobOutcome` 和 graceful shutdown；可增加受限 `DagWorkerRequest`/`DagWorkerOutcome`，但 close 判定由 `core`/`session` 持有。

配套的 `src/protocol.rs` 应增加严格校验的 DAG action/message payload，复用现有 mailbox/tree action、大小限制、TTL 和版本化 JSONL；`src/tool_runtime.rs` 与 `src/approval.rs` 负责工具副作用和权限闸门，派生意图必须先持久化并通过 policy/approval，不能由消息本身提升权限。

## 未决问题

源规格没有定义 SDK 精确版本下限、Node/Python 版本白名单、MCP/subagent/session 字段级 schema、官方文档不可用时的状态映射、报告是否需要机器可读格式、验证超时和重试策略。zenpi 侧还需要确定 DAG `closure proof` 的哈希/摘要算法、heartbeat TTL、lease 时钟与续租规则、派生预算和最大后代数、sibling 是否允许广播还是必须逐一投递、`Blocked` 与 `Unknown` 的升级策略，以及节点重启后如何区分可重试的新工作和已产生外部副作用的 operation。还需补充覆盖 parent/grandparent/sibling/child 四向通信、claim 幂等、后代未绿时 close 被拒绝且 heartbeat/derive 生效、全绿后只 close 一次、旧 generation late result、取消竞态和 journal 重放恢复的测试。

# `claude-code/examples/settings` 目录级学习汇总

## 目录职责

该目录提供面向组织级部署的 Claude Code settings 起始片段。它不是解析器、调度器或运行时通信模块，而是供部署者选择策略、合并 JSON、在本地验证后放入 settings hierarchy 的静态配置样例。`README.md` 明确说明：示例是 community-maintained snippets，可能 unsupported 或 incorrect，配置正确性由使用者负责；某些字段只有写入 enterprise settings 才生效。`sandbox` 只约束 `Bash`，不自动约束 `Read`、`Write`、`WebSearch`、`WebFetch`、MCPs、hooks 或内部命令。

## 模块清单

- `README.md`：说明适用层级、enterprise-only 字段、三个策略样例、合并/本地测试建议及完整文档链接；无函数、类型或运行时导出。
- `settings-lax.json`：导出 `permissions.disableBypassPermissionsMode = "disable"` 与 `strictKnownMarketplaces = []`，目标是关闭 bypass permissions 并阻断 plugin marketplaces。
- `settings-strict.json`：在 lax 基础上导出 `permissions.ask = ["Bash"]`、`permissions.deny = ["WebSearch", "WebFetch"]`、`allowManagedPermissionRulesOnly`、`allowManagedHooksOnly` 和完整 `sandbox` 网络字段，形成更严格的集中管控策略。
- `settings-bash-sandbox.json`：导出 `allowManagedPermissionRulesOnly = true` 及启用 sandbox、禁止 unsandboxed commands、关闭自动放行 Bash 等字段，专门约束 Bash 的沙箱执行。

三份 JSON 是静态数据，没有 Rust/TypeScript 意义上的 key 导出；其“关键导出”就是 Claude Code settings schema 读取的字段。README 的表格是策略索引，不是字段合并规范：空单元仅表示该样例没有声明相应限制。

## 运行时数据流与控制流

源目录的控制流是部署流程：读取一个或多个 JSON snippet → 合并为合法 JSON → 选择 `managed-settings.json`、`settings.json` 或 `settings.local.json` 等层级 → 本地运行验证 → 部署。README 没有规定字段冲突时的覆盖优先级、默认值、版本迁移、自动回滚或失败重试；因此不能把 JSON 的声明缺失解释为 allow，也不能把 enterprise-only 字段推断成所有层级都生效。

映射到 zenpi 的 DAG worker 后，控制流应是明确的持久状态机：headless 接收 `dag_send`、`dag_spawn`、`dag_close` 等命令；`core` 校验拓扑、权限与状态转移；`session` 将消息、lease 和 transition 写入 journal；`runtime` 以有界队列提交一个 worker job；worker 产生进度/终态事件并 heartbeat。`close` 只能在一次一致性检查中确认“节点自身 + 全部直接 child + 递归 grandchild/descendants 均为 green”，且没有待处理消息、未完成操作或有效活跃 lease 后才提交。任何后代尚未 green 时，节点必须继续保活，且新工作由新派生的 worker 承接，不能把当前 worker 的完成误当成整棵子树完成。

可视为以下最小数据流：

`MessageEnvelope` → `SessionMailbox` append-only journal → `claim/complete` → `core` 更新 `DagNodeRecord` → `RuntimeEvent` → `headless` 重放/终态输出。

## 错误、取消与恢复语义

源配置本身的错误语义很弱：非法 JSON、错误 settings 层级、版本不兼容、合并冲突和社区 snippet 不正确，都由部署者在本地验证或产品文档中发现；README 没有自动修复、回滚、锁、队列、超时、取消 token、幂等键或事件流。Bash sandbox 也不是通用的 worker 取消机制。

zenpi 可复用的边界如下：

- `src/protocol.rs` 的 `MailboxRequest::{Send, Receive, Acknowledge, Claim, Complete}`、`MailboxOutcome`、`StdioRequest.mailbox`、`Command::Mailbox` 是消息/邮箱协议入口；`TreeAction::{List, Enable, Select, Fork, Annotate}` 是会话树控制，不能单独充当 DAG close gate。沿用 `validate_*`、ID/文本/TTL 上限与 `ProtocolError`，不要信任客户端自报的 sender 或拓扑关系。
- `src/session.rs` 的 `SessionMailbox` 通过 inode lock、append-only journal、`MailboxMessage.digest`、`sequence`、TTL 和 `MailboxStatus::{Queued, Acknowledged, Claimed, Succeeded, Failed, Expired}` 保持可恢复通信。`Expired` 是按时钟计算出的视图；过期或已 claim 的消息不能隐式重试，重试应带新的 `request_id` 或明确的恢复决策。
- `LiveSessionRegistry::{register, heartbeat, active, claim_next, claim_message, finish_claim, finish_claim_with_reply}` 是保活、认领、完成和回执的最小 admission boundary；`owner_epoch` 可拒绝旧 worker 在重启后完成新 owner 的工作，但它本身不会启动 scheduler。
- `src/runtime.rs` 的 `BackgroundRunner::spawn`、`try_submit`、`try_cancel`、`RuntimeEvent`、`JobOutcome` 和 `CancellationToken` 提供有界执行骨架。`QueueFull` 与 `Closed` 必须被持久化处理；取消是 cooperative，非 cooperative job 只会在 bounded grace 后 detach，detach 不回滚副作用。`Succeeded`、`Failed`、`Cancelled`、`Panicked` 必须分别进入 DAG 状态机，只有明确成功才可能贡献 green。
- `src/headless.rs` 的 `run_async_streams`、`AsyncEventBuffer`、`EventMailboxStream`、`handle_runtime_event` 负责输入、事件缓冲、请求关联、终态响应和重放。buffer 溢出可丢弃部分 progress，但必须保留 admission、tool lifecycle 和 terminal 边界；单个 runtime job 的 `Completed` 不能直接等价于 DAG 子树 `Closed`。

## 与 zenpi Rust 的映射建议

### 通信原语、寻址与协议

复用 `MailboxRequest` 作为可靠消息通道，复用 `SessionMailbox` 保存消息，复用 `AgentEvent`、`ProviderEvent`、`RuntimeEvent` 传播进度和生命周期事件，并通过协议层的 JSON 校验限制边界。建议增加不可变的 `MessageEnvelope` 字段：`message_id`、`sender_node`、`recipient_node`、`relation`、`payload`、`ttl_ms`、`parent_id`、`generation`、`request_id`。`relation` 至少区分 `Parent`、`Grandparent`、`DirectSibling`、`DirectChild`、`Control`，使 worker 能与 parent、grandparent、直接 sibling、直接 child 通信；服务端根据 session/workspace 和 DAG 索引计算可达关系，不能只接受客户端声明。

### 会话父子关系

在 `src/session.rs` 的 durable journal 中增加独立的 `DagNodeRecord` 与 `DagTransition`，至少保存 `node_id`、`parent_id`、`status`、`generation`、`lease_id`、`expected_children`、`green_at` 和版本/序列信息。直接 parent 由 `parent_id` 获得；向上追溯两次得到 grandparent；相同 `parent_id` 的节点集合是直接 siblings；以当前节点为 `parent_id` 的集合是直接 children，递归索引得到所有 descendants。现有 `core::Turn.parent_id`/`Turn::with_parent` 主要表示 conversational turn 的父链，不能直接承担 DAG 拓扑；可以在 metadata 中关联，但必须保留独立的 node parent 字段。

### 保活与派生的最小机制

最小可运行组合是：持久化 `DagNodeRecord` + `SessionMailbox` 消息/事件 + `LiveSessionRegistry` 的 `owner_epoch`/`heartbeat` lease + `BackgroundRunner` 的 `try_submit`/`try_cancel` + `core` 的纯函数 `descendants_green(snapshot)`、`close_if_green(node_id)`、`derive_worker(parent, work)`。派生先落一条 `spawn` transition，再提交 `try_submit` 并绑定 `JobId`；若返回 `QueueFull`，父节点继续 heartbeat，且新工作必须留在 durable queue，不能丢弃。`close` 前读取一致性 snapshot，检查自身及全部 descendants 为 green、无 `Queued`/`Acknowledged`/`Claimed` mailbox、无未完成 operation、无活跃 lease；失败时写 `keepalive`/`close_rejected` 事件并保持节点可调度，再为新工作派生受权限和并发上限约束的 worker。heartbeat 只证明活跃，不改变 green；cancel 只终止执行，不制造完成事实。

### 四个既有 Rust 落点

- `src/headless.rs`：在请求分派处接入 `dag_send`、`dag_spawn`、`dag_close`，为每个事件附带 `node_id`、`parent_id`、`generation`、`sequence` 和 `request_id`；在 `handle_runtime_event` 中保证同一请求的 progress 先于 terminal，并复用 `AsyncEventBuffer`/replay。这里负责宿主 I/O 和事件出口，不负责递归判定。
- `src/core.rs`：放 DAG 状态转移、关系授权、green 聚合和 close gate。可复用 `Agent`、`AgentPhase`、`AgentEvent`、`TurnSubmission`、`ProcessResult`、`WorkerExecutionBinding`；新增的 `descendants_green`、`close_if_green`、`derive_worker` 应是可测试的纯业务函数，不能让 provider 推断拓扑或 close。
- `src/session.rs`：放 `DagNodeRecord`/`DagTransition`、父子/兄弟索引、mailbox 状态、lease、幂等 transition 和崩溃恢复扫描。将 `digest + request_id` 用作重复投递保护，沿用同一 inode lock；恢复时区分未提交 spawn、已提交 runtime job、过期 lease 和已完成结果。
- `src/runtime.rs`：只负责单个 worker 的 bounded admission、执行、取消、panic/失败分类和 `RuntimeEvent`；`BackgroundRunner` 不负责 descendant 聚合。`QueueFull`、`Closed`、`Cancelled`、`Panicked` 必须回传给 `core/session`，由上层决定保活、重派生或阻止 close。

## 未决问题

- `green` 是否只代表明确成功，还是允许人工确认、warning、可恢复产物；`Failed`、`Cancelled`、`Expired`、`Abandoned` 是否永久阻止 ancestor close，需要写入 DAG 规范。
- “全部 child/grandchild”是否意味着无限递归 descendants，是否允许动态新增 child；并发 child 数、worker 派生上限、`BackgroundRunner` 队列满时的持久重试和 backpressure 尚未确定。
- heartbeat 间隔、lease TTL、stale worker 回收/重新 claim、旧 `owner_epoch` 的 late completion 审计规则尚未确定。
- close 与迟到消息、已发送未 ACK 消息、sibling 并发更新的顺序冲突如何处理：采用 `digest/request_id` 幂等、per-node sequence，还是额外的 compare-and-swap 版本条件，尚需决定。
- DAG 拓扑应扩展 `SessionMailbox` payload，还是新增独立的 DAG journal；是否复用 `TreeAction::Fork`、如何统一 `WorkerExecutionBinding`、policy digest、workspace 校验与权限审批，也需要实现前定稿。
- Claude Code settings 样例没有定义完整 schema、合并优先级、版本兼容和回滚工具；这些不能成为 zenpi DAG 语义的隐含约定，必须由 zenpi 自己的协议与测试明确。

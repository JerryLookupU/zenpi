# `hookify/agents` 目录级汇总

> 范围说明：目录索引表记录本目录只有一个源文件，逐文件中文笔记为 `Docs/learn/agent_comms/files/claude-code/plugins/hookify/agents/conversation-analyzer.md_learn.md`，覆盖源文件 176 行。用户给出的 `/Users/wangweiyang/GitHub/Docs/learn/agent_comms/claude-code/plugins/hookify/agents` 在当前环境不存在，因此本汇总以索引和该 1:1 笔记为源事实，并回看 zenpi 当前 Rust 实现核对映射。

## 目录职责

`hookify/agents` 是 Hookify 的分析 agent 定义目录，不是 Python/Rust 执行模块。它为 `/hookify` 提供一个只读的会话审查角色：从 Claude Code 转录中按“最新用户消息优先”的顺序寻找真实发生、且以后值得由 hook 阻止或警告的行为；把行为关联到 `Bash`、`Edit`、`Write`、`MultiEdit` 或 `Stop` 的实际证据；抽取足够具体的 regex；判断 `High`、`Medium`、`Low` 严重度；最后输出供外层命令展示和让用户选择的规则建议。分析 agent 本身不写 `.local.md`、不保存 `.claude`、不执行命令，也不联网；用户确认后的规则生成和持久化属于 `/hookify` 外层流程。

## 模块清单

- `conversation-analyzer.md`：本目录唯一文件，是提示型、只读会话行为分析 agent；关键配置导出为 frontmatter `name: conversation-analyzer`、`model: inherit`、`color: yellow`、`tools: ["Read", "Grep"]`，没有函数、类或 Rust 语言级导出。

该 agent 的文本输出契约是 `## Hookify Analysis Results`，每个发现包括 `Severity`、`Tool`、`Pattern`、`Occurrences`、`Context`、`User Reaction` 和 `Suggested Rule`（其中有 `Name`、`Event`、`Pattern`、`Message`），末尾是 `## Summary` 以及 High/Medium/Low 计数。示例 regex 覆盖 `rm\\s+-rf`、`sudo\\s+`、`chmod\\s+777`、`console\\.log\\(`、`eval\\(|new Function\\(`、`\\.env$`、`/node_modules/` 和 `dist/|build/`。这些是提示中的分析约束和输出字段，不是可直接链接的程序符号。

## 运行时数据流与控制流

1. 用户无参数调用 `/hookify`，或明确要求回顾会话并为错误创建 hook 时，宿主调用 `conversation-analyzer`，模型默认继承调用方，能力限制为 `Read` 与 `Grep`。
2. agent 从会话快照读取用户消息，逆时间扫描，最新纠正优先。候选信号包括明确禁止/纠正、挫败或判错反应、撤销/修复过程，以及同类错误被重复提醒；单纯偏好、假设提问和教学说明不算真实行为。
3. 对每个候选，先绑定实际工具、命令或代码、任务阶段、危害理由和可核查例子，再将证据压缩成不过宽的动作 regex。`Bash` 应保留真实命令，文件编辑应保留实际加入的代码，`Stop` 应指出停止前缺失的内容。
4. 风险分级为：`High` 用于危险删除、权限放开、硬编码 secret、`eval` 和数据丢失；`Medium` 用于生产 `console.log`、编辑生成文件或缺少实践；`Low` 用于一次性已修复事故或主观风格偏好。分析文本只产出建议，外层展示结果、询问选择，再生成 `.local.md`。
5. 源定义没有并发合并、锁、缓存、持久化、重试或调度流程，因此安全运行假设是单个不可变会话快照、确定性顺序和只读分析；空转录/无证据时应输出零发现而不臆造规则。

## 错误、取消与副作用语义

源文件没有显式错误类型、状态机、取消 token、超时、恢复点或部分结果协议。可推导阶段是“读取转录 → 找候选 → 绑定工具证据 → 抽取 regex → 判定严重度 → 输出结构化文本”。`Read`/`Grep` 失败、转录不可读、regex 无法编译、重复发现如何合并、同一行为跨严重度冲突如何解决，都由宿主决定。取消只能在读取或搜索边界由宿主中止；源 agent 没有要求回滚，因为分析没有写入副作用。

对 zenpi 的安全解释应把 `NoEvidence`、输入非法、读取失败、regex 非法、邮箱过期、claim 冲突、worker 取消和 `Abandoned` 分开。取消不是回滚：已写入 session journal 的成功结果不能被 late cancel 改写；已 claim 的消息不能盲目重试，须依 operation journal 判定已完成、失败或放弃后再恢复。只有用户确认后的规则生成才允许产生文件副作用。

## 与 zenpi Rust 的映射建议

### 可复用的通信原语

- **DAG 消息/邮箱**：当前 `src/dag.rs` 已提供 `DagNode`、`DagRelation`、`DagMessage`、`DagStore`；`DagRelation` 直接支持 `Parent`、`Grandparent`、`Sibling`、`Child`、`All`，`DagStore::recipients` 按节点关系寻址，`send` 写入有界、文件锁保护的 JSON mailbox，`claim_inbox`/`ack` 提供 claim/确认和未确认重投，`wait_for_message` 提供有界轮询与取消检查。Hookify finding 可序列化为受限消息体，不应另造绕过节点关系的 channel。
- **持久邮箱**：需要 session 身份、TTL、digest、幂等 request ID、claim token、完成结果和跨重启语义时，复用 `src/session.rs` 的 `MailboxMessage`、`SessionMailbox`、`LiveSessionRegistry::{register, heartbeat, active, claim_next, claim_message, finish_claim, finish_claim_with_reply}`。该邮箱通过 `sender_session_id`/`recipient_session_id`、workspace、digest 和 `owner_epoch` 校验访问；`MailboxStatus` 区分 `Queued`、`Acknowledged`、`Claimed`、`Succeeded`、`Failed`、`Expired`。
- **事件/审计**：短生命周期进度继续使用 `core::AgentEvent::{Provider, ToolProgress, Warning, Error}`；最终的 `HookifyFinding` 建议包含 `pattern`、`severity`、`tool`、`occurrences`、`evidence`，作为 mailbox payload 或工作结果，并以 `SessionStore::append_event`、`append_handoff` 写入可重放审计。`AgentEvent::Warning` 只表示进度或非终止通知，不能冒充 DAG 完成。
- **外部协议**：`src/protocol.rs` 的 `MailboxRequest` 和 `Command::Mailbox` 已把 Send/Receive/Acknowledge/Claim/Complete 暴露为有界 JSONL 请求；可增加严格校验的 `DagAction::{Status, Send, Recv, Spawn, KeepAlive, Close}`，但必须保留 schema version、correlation ID、reply target、workspace/owner 校验、TTL 和 payload 大小上限。

### 会话父子关系与邻居通信

`core::Turn.parent_id` 与 `Turn::with_parent` 表达同一 session 内输入、assistant、tool turn 的直接父子关系；沿 `parent_id` 上溯两次可得到 grandparent，同一 parent 下除自身外的 children 是直接 sibling，当前 DAG node 的 `children` 是直接 child。它不能单独替代执行 DAG，因此应让持久化 `DagNode` 保存 `id`、`parent`、`children`、`status`、`worker`、`worker_heartbeat_ms`，必要时增加 `session_id`、`generation`、`correlation_id` 和边类型。

worker 负责一个 node 时，发送目标可按 `DagStore::recipients` 解析为 parent、grandparent、直接 sibling、直接 child 或 `all`；真实投递走 `dag_send`/`dag_recv` 或受权限约束的 `SessionMailbox`。每条消息应携带 `correlation_id`、发送节点、接收节点、generation、reply target 和工作类型；sibling 只能共享工作信息，不能凭消息内容冒充 parent 或获得对方 lease。

### 保活、派生与 close gate 的最小机制

1. worker 启动时登记 session owner 和 DAG node worker，持有 `lease_id`/`owner_epoch`；周期调用 `LiveSessionRegistry::heartbeat` 与 `DagStore::touch_worker`。`dag_recv` 使用不超过 30 秒的有界等待，每轮检查取消；`dag_status`/`dag_recv` 的访问也会刷新 DAG heartbeat。
2. worker 收到 parent、grandparent、直接 sibling 或直接 child 的消息后，先做节点关系、workspace、lease、generation 和 payload 校验，再 claim；成功处理后 `ack` 或 `finish_claim`。未 ack 的 `DagMessage` 会再次交付，不能通过重复派生掩盖未确认工作。
3. `DagStore::can_close`/`dag_finish` 是 close gate 的现有落点：同一快照中必须同时确认当前 node 为 `green`，并递归检查全部 child、grandchild 及更深 descendant 都为 `green`。生产语义还应把未完成 tool、approval、claim、未处理邮箱和待恢复 operation 纳入 gate；任一项未完成都不能 close。
4. gate 不通过时，worker 必须保持 `open`/`Running`/`KeepAlive`，继续 heartbeat、接收 mailbox、向相关邻居发送状态，并处理 unfinished 列表；如果新工作需要新执行单元，调用 `dag_spawn` 或 coordinator 的 spawn API 派生新的 child/generation。不能在一次 mailbox claim 内隐式 fork，也不能因 provider 返回就直接退出。
5. `dag_spawn` 当前会 `upsert` child、调用 `spawn_worker` 启动 `--mode headless` worker，设置 `ZENPI_DAG_NODE`、`ZENPI_DAG_STORE`，保留 child stdin 以便 `send_to_worker` 投递 follow-up，并记录 `pid`。正式 coordinator 仍需以幂等 `correlation_id`、唯一 `JobId`、父 lease/approval/policy digest 和持久 journal 防止重复派生；所有 descendant 变绿后才释放 owner 并关闭。

### 指定 Rust 文件的落点

- `src/headless.rs`：`run_headless` 是 worker 的 JSONL stdin/stdout 生命周期和 `AgentEvent` 出口；在异步请求循环中接入 DAG mailbox drain、heartbeat tick、`dag_recv`、`KeepAlive`、取消检查、shutdown grace，以及 `dag_finish` 后“关闭或继续保活/派生”的分支。该文件已有 `Command::Mailbox` 路由、后台 runner、cancel/reissue 和 shutdown 处理，适合做宿主控制流，不应让 provider 自己决定 close。
- `src/core.rs`：`Agent`、`Turn`、`Turn::with_parent`、`AgentEvent`、`WorkerExecutionBinding` 和工具执行状态负责 turn ancestry、finding 结果、lease/policy digest 和工具副作用；可在这里把分析结果转成可审计的工作项，并在 tool/approval 未完成时阻止 `Green`，但 DAG 邻居路由应交给 coordinator/session。
- `src/session.rs`：`SessionStore`、`SessionTree`、`append_event`、`append_handoff` 持久化 ancestry、状态变化、派生和恢复记录；`SessionMailbox` 与 `LiveSessionRegistry` 提供有界寻址、TTL、heartbeat、claim/complete/reply，并用 `owner_epoch` 拒绝旧 worker 完成新 claim。可在此增加 DAG 关系事件和 descendant 状态索引，使 close gate 可重放。
- `src/runtime.rs`：`BackgroundRunner`、`JobId`、有界 command/event queue、FIFO pending follow-up 与 `CancellationToken` 承接实际 worker 执行。`try_submit` 用于受控派生，`try_cancel`/`try_shutdown_with_grace` 用于取消和退出；`mark_completed` 保证 late cancel 不把已经发布的成功结果改写为 `Cancelled`。非协作 worker 在 grace 后可 detach，但这不回滚已经发生的副作用。

辅助落点是 `src/dag.rs` 与 `src/tools.rs`：前者负责关系、file lock、消息、heartbeat、close gate 和 live child stdin，后者暴露 `dag_status`、`dag_send`、`dag_recv`、`dag_finish`、`dag_spawn`。若未来把 DAG 状态从文件 mailbox 迁入 session mailbox，应由 `DagCoordinator` 维护单一事实来源，避免两套状态/journal 分叉。

## 未决问题

1. `conversation-analyzer.md` 未定义空转录、读取失败、regex 编译失败、重复问题合并、跨严重度冲突和取消时的输出协议；zenpi 需确定错误枚举、会话快照 hash、稳定排序和 `Found 0 behaviors` 的精确 schema。
2. 当前 `DagStore` 与 `SessionMailbox` 都能通信：前者天然表达 parent/grandparent/sibling/child 和 subtree close，后者具备身份、TTL、claim、回执和审计。需要决定由 coordinator 桥接还是收敛为单一事实来源，不能让同一工作在两个 journal 中出现不一致状态。
3. `green` 的边界仍需产品化：除节点分析完成外，是否必须没有未读消息、claimed-but-unacked 消息、未完成 tool/approval、有效 child lease、待恢复 operation 和新到达工作；这些条件应在一个可验证的 close 事务中定义。
4. owner heartbeat 过期后应选择 `Abandoned` 后重派生、保持 KeepAlive 等待、回收子进程还是人工介入；还需覆盖崩溃恢复、重复 `dag_spawn`、旧 `owner_epoch`、child stdin 断开和 detached job 的测试。
5. 需要固定 `HookifyFinding` 到 DAG 工作项的 schema、权限与幂等规则，确保假设/教学文本不会派生 worker，直接 sibling 不能伪造 parent，且用户确认前不产生 hook 文件副作用。

# AC-036 — codex/codex-rs/protocol/src/lib.rs

## 元信息

- `source_id`: `codex/codex-rs/protocol/src/lib.rs`
- `item_id`: `AC-036`
- `source_path`: `/Users/wangweiyang/GitHub/codex/codex-rs/protocol/src/lib.rs`
- `source_hash`: `2fddb495f04cc545bb68629845afcfd8d52e339bcc533885c470d430f767f835`
- `source_bytes`: `444`
- `source_lines`: `21`
- `coverage`: 已读取完整字节范围 `0-443`（共 444 字节）与完整行范围 `L1-L21`；没有跳过注释、声明或空行。

## 完整行为复盘

该文件是 `codex-rs/protocol` crate 的根模块清单，不承载消息编解码、状态机或线程逻辑。每个 `pub mod` 在编译时建立一个公开子模块路径；调用方能否使用具体类型取决于对应子模块自己的可见性。按源文件顺序：

- `pub mod account;`（`L1`）：导出 `account` 模块。输入是编译器加载的同名 `account.rs`/目录模块，输出是公开命名空间；本行没有运行时默认值、错误分支、I/O 或副作用。
- `mod thread_id;`（`L2`）：加载私有 `thread_id` 模块。模块本身不对 crate 外公开；其内部类型或函数的并发、取消和错误语义不能由本文件确认。
- `pub use thread_id::ThreadId;`（`L3`）：把私有模块中的 `ThreadId` 重导出为 crate 根符号。它改变可见性和导入路径，不复制值、不创建线程，也不定义 ID 格式；具体构造、比较、序列化和错误边界需查 `thread_id` 实现。
- `pub mod approvals;`（`L4`）：导出审批协议命名空间。
- `pub mod config_types;`（`L5`）：导出配置类型命名空间。
- `pub mod custom_prompts;`（`L6`）：导出自定义提示词命名空间。
- `pub mod dynamic_tools;`（`L7`）：导出动态工具命名空间。
- `pub mod items;`（`L8`）：导出协议项目/条目命名空间。
- `pub mod mcp;`（`L9`）：导出 MCP 相关协议命名空间。
- `pub mod memory_citation;`（`L10`）：导出记忆引用命名空间。
- `pub mod message_history;`（`L11`）：导出消息历史命名空间。
- `pub mod models;`（`L12`）：导出模型类型命名空间。
- `pub mod num_format;`（`L13`）：导出数字格式化命名空间。
- `pub mod openai_models;`（`L14`）：导出 OpenAI 模型命名空间。
- `pub mod parse_command;`（`L15`）：导出命令解析命名空间。
- `pub mod permissions;`（`L16`）：导出权限命名空间。
- `pub mod plan_tool;`（`L17`）：导出计划工具命名空间。
- `pub mod protocol;`（`L18`）：导出具体协议消息命名空间；根模块只负责暴露路径，不规定消息字段或版本。
- `pub mod request_permissions;`（`L19`）：导出权限请求类型/处理命名空间。
- `pub mod request_user_input;`（`L20`）：导出用户输入请求命名空间。
- `pub mod user_input;`（`L21`）：导出用户输入值命名空间。

因此，本文件的输入/输出边界是编译期模块解析：缺失同名子模块会导致编译错误；重复或非法模块声明也在编译期失败。没有函数参数、返回值、默认值、运行时错误、锁、通道、消息队列或并发顺序。`ThreadId` 是唯一根级重导出，但其行为在本文件外定义，不能从 `L1-L21` 推断。

## 状态、取消、恢复与副作用

源内没有可变状态、取消令牌、超时、重试、持久化、网络/文件写入或外部副作用。模块声明只影响链接与名称解析。若子模块实现这些能力，生命周期不会自动由 `lib.rs` 统一协调；调用方必须显式调用子模块 API。特别是 `ThreadId` 重导出不等于会话恢复游标，也不提供取消传播。该文件没有并发语义：并发安全性、`Send`/`Sync`、序列化和错误类型均需从各子模块另行验证。

## 源内测试与行为判据

源文件本身未包含测试，也没有 `#[cfg(test)]` 模块。可独立验证的判据是：

1. 在对应 workspace 执行 `cargo check -p codex-protocol`，确认全部公开模块可解析且根路径存在 `codex_protocol::ThreadId`。
2. 编译一个最小导入片段，分别导入 `codex_protocol::{ThreadId, protocol}` 与每个公开模块，确认 `thread_id` 私有路径不可直接从 crate 外访问而 `ThreadId` 可访问。
3. 对源文件计算 `sha256sum`，应得到元信息中的 hash；`wc -c -l` 应为 `444` 字节、`21` 行。
4. 这些检查只证明模块装配和导出边界，不替代各子模块的协议、取消或持久化测试。

## zenpi Rust 映射

`lib.rs` 的可复用思想是“根模块只装配稳定命名空间，行为下沉到专门模块”。zenpi 已有更丰富的协议实现，应把 DAG 编排能力组合到现有边界，而不是塞入 provider 代码。

- `src/protocol.rs` 是最直接的对照层：已有版本化 JSONL、`MailboxRequest`（`Send/Receive/Acknowledge/Claim/Complete`）、`CheckpointRequest`、`TreeAction`、`Command` 和严格字段校验。复用 `MailboxRequest` 作为 worker 通信原语：发送到 parent、grandparent、直接 sibling、直接 child 都只需解析为目标 `session_id` 的消息；消息必须带稳定 `message_id`、发送者 `session_id`、`item_id`/`goal_id`、类型和 TTL。用 `StdioEvent` 表示 `worker_started/heartbeat/message_received/child_green/close_blocked/worker_closed`，使事件可被 JSONL 客户端观察和重放。
- `src/session.rs` 是会话父子关系和持久化落点。建议增加可序列化的 `DagNodeRecord { session_id, parent_session_id: Option<String>, goal_id, item_id, state, generation }` 与 `DagNodeState::{Pending,Running,Green,Failed,KeepAlive,Closed}`，通过 append-only event 记录 `dag_node_created`, `dag_edge_bound`, `dag_state_changed`, `dag_keepalive`, `dag_worker_spawned`。grandparent 由 parent 的 `parent_session_id` 得到；直接 sibling 定义为共享同一 `parent_session_id` 的节点；直接 child 定义为 `parent_session_id == self.session_id`。不要只依赖内存指针，重启后从事件恢复邻接索引。
- `src/core.rs` 适合放 `DagCoordinator`/`DagNodeHandle`。它可以复用已有 `Agent::register_live_owner`、`heartbeat_live_owner`、`claim_live_mailbox`、`finish_live_mailbox` 的所有权概念。最小 API 建议为 `send_relation_message`, `claim_relation_message`, `complete_relation_message`, `heartbeat_node`, `evaluate_close`, `derive_worker`。`evaluate_close(node)` 必须递归检查“节点自身为 `Green` 且全部 child/grandchild（即全部后代）为 `Green`”；任一后代未绿、失败、超时或仍有未完成 mailbox 时返回 `KeepAlive`，禁止 `Closed`。
- `src/runtime.rs` 提供线程与取消原语：现有 `BackgroundRunner::spawn`、`try_submit`、`try_cancel`、`try_shutdown_with_grace`、`CancellationToken` 和 `RuntimeEvent` 可承载 worker 生命周期。建议增加 `WorkerJob::RunDagNode`/`WorkerJob::DeriveChild` 或等价命令；派生只在 `evaluate_close` 返回 `KeepAlive` 且存在新工作时调用 `try_submit`。将 `Accepted/Started/Queued/CancelRequested/Completed/Closed` 映射到 DAG 事件，并保持有界队列，避免 sibling 洪泛。
- `src/headless.rs` 是宿主/传输适配层：已有 mailbox slash 映射（`mailbox_slash_view`）、事件缓冲、重放序列和运行时生命周期输出。这里负责把 parent/grandparent/sibling/child 的关系消息路由到 `Agent`/`DagCoordinator`，并把 `heartbeat`、`close_blocked`、`worker_spawned` 写入同一有界 replay ledger；不得让 stdout 直接承担未确认的持久化语义。
- `src/tool_runtime.rs` 是节点执行边界。worker 完成工具批次后先写成功/失败事件，再向 coordinator 报告 `Green`；有副作用的工具仍经过现有执行/恢复判定，不得因为 DAG 节点标绿而绕过工具策略。未决工具操作或 `UnknownOutcome` 应阻止 close，触发保活。
- `src/approval.rs` 继续作为 side-effect gate。`ApprovalRequest` 的 `turn_id`、`call_id`、`policy_digest`、`lease_id` 可作为 worker 消息的审计关联；`WorkerAllowAfterPreflight` 只在 live lease 有效时允许执行。审批取消应让节点保持非绿状态，而不是误报成功。
- `src/providers/**` 只产生 provider 内部流事件和错误；它们不是 DAG 拓扑层。provider 事件可通过 `StdioEvent` 关联 `session_id`/`turn_id`，但 parent、grandparent、sibling、child 的寻址、状态聚合和派生必须留在 `protocol`/`session`/`core`/`runtime`。

建议的最小保活/派生流程是：`RunDagNode` 开始时写 `Running` 并启动 heartbeat；执行期间用 `MailboxRequest::Send` 向直接关系节点发送工作/状态；完成自身工作后写 `Green`，调用递归 `evaluate_close`；若所有后代均绿，追加 `Closed` 并释放 live owner；否则追加 `KeepAlive`，保持 heartbeat，计算新工作并通过 `BackgroundRunner::try_submit` 派生一个带新 `session_id`、`parent_session_id=self.session_id`、递增 `generation` 的 child worker。派生和消息发送都以 `message_id`/幂等键去重，`Claim` 后只能有一个 owner，`Complete` 记录 `Succeeded/Failed/Abandoned`；TTL 到期或取消只会产生明确失败/放弃事件，不能隐式变绿。

差异清单：源 `lib.rs` 仅有 20 个公开模块声明加 1 个私有模块重导出，没有 DAG、mailbox 或生命周期模型；zenpi 已有 mailbox、tree、checkpoint、运行时队列和 append-only session，但尚缺统一的 DAG 节点状态聚合、后代全绿判定、关系寻址索引及“保活后派生 child”的原子审计流程。可执行验证包括：新增协议解析用例覆盖四种关系目标和 TTL 边界；session 恢复后重建 parent/child/sibling 索引；runtime 测试验证取消不会产生 `Green`；coordinator 测试验证任一 grandchild 非绿时只能 `KeepAlive`，全部后代绿时才产生一次 `Closed`；headless 重连测试验证 heartbeat/close_blocked/worker_spawned 可按序重放。

## 未决问题

无法从该源文件确认：`ThreadId` 的具体表示、序列化格式和生命周期；各公开子模块是否存在同名文件还是目录模块；Codex 协议实际消息版本、错误枚举、线程安全约束和测试位置。这些都需要读取对应子模块源码后才能确定。

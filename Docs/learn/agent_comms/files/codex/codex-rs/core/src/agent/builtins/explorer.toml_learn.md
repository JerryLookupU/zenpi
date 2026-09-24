# AC-017 — codex/codex-rs/core/src/agent/builtins/explorer.toml

- source_id/item_id：`codex/codex-rs/core/src/agent/builtins/explorer.toml` / `AC-017`
- source_path：`/Users/wangweiyang/GitHub/codex/codex-rs/core/src/agent/builtins/explorer.toml`
- source_hash：`e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`
- source_bytes：`0`
- source_lines：`0`
- coverage：已从文件起点顺序读至 EOF；字节范围 `[0,0)`，行范围为空（无 `L1-Ln` 可引用行）。`wc -c -l` 为 `0 0`，SHA-256 与元数据一致。

## 完整行为复盘

该文件为空，因而没有 TOML 键、注释、函数、类型、常量、导出符号或默认值。逐符号检查结果为“无”；不存在可以标注为 `L12-L40` 的源行，也不存在输入、输出、边界检查、错误分支或并发实现。解析行为可独立判定为：任何 TOML 解析器读取空字节串都得到空文档（具体是否被上层视为缺失配置，取决于调用方）；本源文件本身不创建 worker、不发送消息，也不产生外部副作用。没有隐含的并发语义、取消点、超时或重试策略，不能从此空文件推导出 `explorer` 的业务契约。

## 状态、取消、恢复与副作用

源内没有状态字段或状态机，没有取消 token、超时、重试、持久化记录、网络/文件写入、进程/线程派生。恢复同样没有定义；若系统在运行时表现出恢复、重放或保活行为，应归因于调用该内置定义的宿主代码，而不能归因于本文件。由于文件为零字节，任何“缺少 explorer 配置”的错误、回退默认值或启动策略都必须在调用方验证。

## 源内测试与行为判据

源内未包含测试，也没有同目录测试可引用。可独立验证的判据：`test ! -s /Users/wangweiyang/GitHub/codex/codex-rs/core/src/agent/builtins/explorer.toml` 成功；`wc -l` 返回 `0`；对文件计算 SHA-256 得到给定哈希；读取结果长度为零。任何要求字段、默认 prompt 或导出名的测试都不应因本文件而通过。

## zenpi Rust 映射

映射前提是“空源不提供可复用实现”，因此 DAG worker 协议应落在 zenpi 现有运行时，而不是伪造一个 `explorer.toml` API。`src/runtime.rs` 已有 `CancellationToken`（约 `L56-L100`）、有界 `BackgroundRunner` 与 `try_submit/try_cancel/try_shutdown`（约 `L236-L340`），可复用为每个 DAG 节点的最小邮箱/事件循环：命令通道承载 `WorkerMessage`，事件通道承载 `WorkerEvent`，有界队列满返回显式错误；节点收到取消后停止接收新工作并发出终态。

会话父子关系应复用 `src/core.rs` 的 `Turn { parent_id }` 及 `with_parent/validate`（约 `L141-L197`），并在 `src/session.rs` 的追加式 JSONL 事件投影（`SessionStore` 约 `L144-L160`、`L327-L421`）中持久化 `session_id`、`parent_id`、`grandparent_id`、`node_id`、`edge_kind` 与状态版本。建议新增（或在现有模块中落点）`DagNodeId`、`WorkerRelation::{Parent,Grandparent,DirectSibling,DirectChild}`、`WorkerMessage::{Work,Status,Cancel,CloseRequest,SpawnRequest}`、`WorkerEvent::{Accepted,Progress,Green,Failed,Closed,Spawned}`；消息必须带 `correlation_id`、发送者节点和目标节点，邮箱按目标节点路由。父/祖父通信走显式会话索引，直接 sibling 只允许通过共同 parent 的转发器通信，direct child 由 parent 持有 child mailbox；禁止任意跨树广播。

`src/protocol.rs` 已有 `MailboxRequest`/`MailboxOutcome`（约 `L81-L119`）以及 `Command` 转换和 `validate_mailbox`（约 `L214-L305`、`L544-L585`），应作为外部 JSONL mailbox 协议的入口：扩展字段时保留接收者会话校验、文本上限和关联 ID 校验；`StdioEvent`（约 `L882-L913`）用于把异步 worker 事件投影给 headless 客户端。`src/headless.rs` 的有界请求/事件与重放状态（约 `L39-L47`、`L133-L145`、`L288-L316`）可承载断线后的事件游标，但 DAG 真正状态仍须写入 session journal，不能只依赖 stdout 缓存。

`src/tool_runtime.rs` 的 `ToolBatchOptions` 默认并发与 `execute_tool_batch` 的 worker/channel 及 owner unwind 取消（约 `L113-L133`、`L168-L325`）可复用为节点执行工具时的并发边界；节点关闭前必须等待自己及全部 child/grandchild 的 green 事件。`src/approval.rs` 的 `ApprovalCoordinator`（约 `L126-L170`、`L409-L456`）可作为需要人工批准的消息/副作用闸门，取消应调用 `cancel_all` 或 `emergency_cancel`，并将结果持久化后再释放等待 worker。

最小保活与派生机制：每个节点维护 `child_ids`、`pending_children`、`self_green`、`generation` 和 `CancellationToken`；只有 `self_green == true` 且所有 child（递归汇总 grandchild）都收到持久化 `Green`，才发 `CloseRequest` 并转换 `Closed`。若新工作到达、子节点失败/超时或 green 集合不完整，节点保持活跃，向 parent 发送 `Progress/NeedsWork`，通过 `BackgroundRunner::try_submit` 派生一个带新 `parent_id` 与 generation 的 worker；派生必须先写 `SpawnRequest` 事件，再提交命令，避免崩溃后出现无记录孤儿。超时只触发取消/重派策略，不得把未确认的副作用当作回滚。

`src/core.rs` 的 `AgentEvent`/`AgentSnapshot`（约 `L275-L338`）是聚合 DAG 状态的合适落点；`src/runtime.rs` 负责调度和关闭，`src/session.rs` 负责恢复，`src/protocol.rs` 负责跨进程邮箱，`src/headless.rs` 负责流式投影，`src/tool_runtime.rs` 负责工具批处理，`src/approval.rs` 负责批准闸门，`src/providers/**` 仅提供模型/后端调用，不应自行决定 DAG close。可执行验证包括：构造 parent→child→grandchild 树并断言任一非 green 节点不能关闭；验证 sibling 只能经 parent 转发；重启后从 JSONL 重建 pending 集合；队列满、取消、重复 correlation ID 和 child 崩溃均返回可识别错误且节点保活。

差异清单：空源没有任何行为可兼容；zenpi 需要新增 DAG 类型和事件 schema、持久化恢复投影、关系权限校验、递归 green 聚合、派生幂等键及关闭顺序测试。现有 runtime 的 `JobOutcome`/`RuntimeEvent` 更偏单 job 终态，需增加树级聚合而不能直接把单 job `Completed` 当作 DAG `Closed`。

## 未决问题

无法从空源确认 `explorer` 原本应提供的 prompt、工具白名单、模型参数、配置键名或错误文本；也无法确认 zenpi 期望的 DAG 事件 JSON schema、重试次数和持久化文件命名。这些必须由调用方协议、产品需求或其他非本源文件确定。

# `claude-code/examples/hooks` 目录级学习汇总

> learn_mode：`understand`。本目录实际只有一个源文件。用户给出的
> `/Users/wangweiyang/GitHub/Docs/learn/agent_comms/claude-code/examples/hooks`
> 路径当前不存在；本汇总依据 zenpi 中已完成的 1:1 笔记
> `Docs/learn/agent_comms/files/claude-code/examples/hooks/bash_command_validator_example.py_learn.md`，并回看对应源码
> `/Users/wangweiyang/GitHub/claude-code/examples/hooks/bash_command_validator_example.py` 完成。

## 目录职责

这是一个 Claude Code `PreToolUse`/`Bash` hook 示例目录，职责是在 Bash 工具真正产生副作用前读取一份 JSON 请求、识别命令并执行轻量规则校验。它不执行 `grep`、`find` 或替代命令，不改写输入命令，也不保存跨请求状态；它只通过 `stderr` 给出可见诊断，并用进程退出码表达“放行、输入错误或阻止工具调用”。因此该目录体现的是一个很小但清晰的 preflight/admission 边界：策略必须在工具执行前完成，规则失败不应进入 provider 或 shell。

## 模块清单

- `bash_command_validator_example.py`：单文件、同步、一次性处理一个 stdin JSON 的 Bash 命令验证器；关键符号是私有规则表 `_VALIDATION_RULES`、纯校验函数 `_validate_command(command: str) -> list[str]` 和进程入口 `main()`（由 `if __name__ == "__main__"` 调用）。

`_VALIDATION_RULES` 是有序的 `(regex pattern, message)` 列表：以 `grep` 开头且命令中没有 `|` 时建议 `rg`；以 `find <path> -name` 开头时建议 `rg --files` 组合。`_validate_command` 按顺序遍历所有规则并收集全部命中，不短路，且没有共享可变状态。`main` 只对 `tool_name == "Bash"` 继续，读取 `tool_input.command`，有问题时逐行打印 `• {message}` 并退出 `2`。

## 运行时数据流与控制流

调用方通过 hook 配置把单个 JSON 对象写入脚本 stdin。`main` 先用 `json.load(sys.stdin)` 解码；成功后读取 `tool_name`，非 `Bash` 立即以 `0` 放行。对 Bash 请求，它从 `tool_input` 取 `command`，空值也以 `0` 放行；非空命令进入 `_validate_command`，依次执行两个 `re.search`。若无命中，函数自然返回，进程以 `0` 结束，Claude Code 可继续调用 Bash；若有命中，诊断只写 `stderr`，以 `2` 终止本次工具调用，由上游重新提交采用 `rg` 的命令。

这是一进程一请求的线性流：stdin JSON → 结构读取 → 工具名过滤 → 命令规则匹配 → `stderr`/退出码。没有线程、异步任务、网络、文件、子进程、持久化队列或跨请求缓存，也没有把 hook 规则变成 shell 解析器。源码中的管道例外是精确行为：`grep foo | sort` 会因第一条正则的负向前瞻而放行；前导空格、`sudo grep`、`echo x; grep` 也不满足“字符串开头即 `grep`”的条件。

## 错误、取消与恢复语义

- JSON 解码失败只捕获 `json.JSONDecodeError`，向 `stderr` 输出 `Error: Invalid JSON input: ...`，退出 `1`；stdout 保持空白。
- JSON 是数组、字符串或 `null`，或 `tool_input` 不是映射时，后续 `.get` 可能产生未捕获异常。这说明示例没有完整输入 schema；Rust 迁移不应复制这种隐含崩溃，而应返回结构化 `ProtocolError`。
- 非 Bash、空命令、没有规则命中均为退出 `0`；规则命中为退出 `2`，表示阻止工具调用并让 Claude 看到 `stderr` 诊断。命令不会被自动修复，修复需要上游重试。
- 源文件没有取消令牌、超时、重试、清理钩子或恢复状态。若宿主在 `json.load` 阻塞期间杀掉进程，脚本没有机会记录中间状态；重复运行因无持久化而不会改变结果。非字符串但为真的 `command`、正则语法错误等也不在本地转换为业务错误。

## 与 zenpi Rust 的映射建议

### 通用通信与会话模型

可迁移的核心不是“推荐 `rg`”，而是“在副作用边界前做纯校验，并以结构化结果阻止后续执行”。在 zenpi 中应把 hook 规则放到工具策略/preflight 层，使用统一的消息、邮箱、事件和协议，而不是复制到 `src/providers/**`。provider 只产生请求和流事件，不能绕过统一 admission gate。

对 DAG worker 建议采用最小 `DagEnvelope`：包含 `message_id`、`sender`、`recipient`、`session_id`/`node_id`、父关系、序号、`ttl_ms` 和受限 payload；recipient 可建模为 `DagRecipient::{Parent, Grandparent, Sibling(NodeId), Child(NodeId)}`。每个节点一个有界 mailbox（`VecDeque` 或有界 `mpsc`），一个有界事件流（复用 `StdioEvent`/`RuntimeEvent`），并以 `Send → Claim → Complete/Acknowledge` 形成幂等处理边界。`parent` 直接寻址，`grandparent` 由 `parent.parent` 解析，直接 sibling 由同一 `parent` 的 children 集合排除自身得到，直接 child 由当前节点 children 集合得到；关系必须来自持久化 DAG 索引，不能只写在 provider prompt 中。消息 TTL、最大 ID/文本/邮箱容量和重复 `message_id` 检查应统一走已有协议边界，过期消息产生明确的 abandoned/expired 结果而不是无限重试。

会话父子关系建议同时保留两层：`Turn::with_parent` 的 `parent_id` 表示对话/执行记录的父 turn，新增的 `DagNodeState` 表示编排图中的 `node_id`、`session_id`、`parent_session_id`、children、状态和 lease。每次派生 worker 都要生成新的节点/turn 标识，并把父节点写入关系索引；grandparent 与 sibling 查询不得依赖临时内存 prompt。可复用的状态至少包括 `Running`、`Waiting`、`Green`、`Failed`、`Cancelled`、`Unknown`，并将图版本和状态摘要随事件记录。

### `src/headless.rs`：协议入口、请求关联与事件邮箱

`run_headless` 是同步 JSONL 入口，`run_stdio` 是 stdin/stdout 封装，`run_async_streams`/`run_stdio_owned` 是异步宿主路径。这里负责 `read_frame`、`parse_line`、请求 ID/fingerprint 去重、重放缓存、`HeadlessError` 归类以及只向 stdout 输出协议响应。DAG 消息可作为新的 `Command`/`StdioRequest` 变体进入同一入口；校验失败应通过 `StdioResponse::error_with_code` 返回稳定 code，而不是借用 stderr 退出码。

现有 `AsyncEventBuffer` 已是“按请求隔离且有界”的事件 mailbox：provider/agent 事件分别缓存，按序列化字节数和数量限额，满时记录 dropped 计数。它适合承载 worker 的 `DagMessage`、`KeepAlive`、`Spawned`、`CloseDeferred` 等外部可观察事件，但不能替代持久化图状态。headless 层还应把请求 ID 与 `message_id`、节点 ID和父会话关联，避免 queued worker 完成事件错投到另一个请求；输入 EOF 是正常 drain，显式 shutdown 才是取消请求。

### `src/core.rs`：worker 绑定、图状态机和 close 判据

`WorkerExecutionBinding` 已提供 `blueprint_id`、`goal_id`、`item_id`、`lease_id`、`policy_digest` 与 `expires_at_ms` 的绑定和 `validate`；它应继续作为 worker 权限/租约的前置条件，但不能等同于批准。`Turn` 的 `parent_id`、`Turn::with_parent` 可承载会话父子链；`AgentEvent`、`AgentError` 与现有工具 admission 逻辑可承载 DAG 生命周期和结构化拒绝。

建议在 `core.rs` 增加不可变快照驱动的 `DagCoordinator::can_close(node)` 或等价纯函数：只有节点自身为 `Green`，且该节点所有 child 以及递归的全部 grandchild/descendant 都为 `Green`，才返回 true；任一 descendant 为 `Running`、`Waiting`、`Failed`、`Cancelled` 或 `Unknown` 都必须 fail closed。close 事件应携带图版本、节点 ID和子孙摘要，并在恢复时重新计算，不能信任旧的布尔值。`false` 时节点保持活跃、刷新 lease 并发出 `KeepAlive`；若存在新工作且配额允许，先写 spawn intent，再派生新 worker；没有新工作时只保活，不凭空生成 worker。

### `src/session.rs`：append-only journal、图事件和恢复

`SessionStore`、`append_event`、`append_runtime_intent`、`OperationKind`/`OperationOutcome`、`begin_operation`/`finish_operation` 与 `operation_recovery` 提供了可靠的追加式恢复边界。可把 `dag_node_created`、`dag_message_sent`、`dag_message_claimed`、`dag_message_completed`、`worker_heartbeat`、`worker_spawned`、`dag_node_green`、`dag_close_deferred`、`dag_closed` 作为规范化事件写入；每个事件应带 `session_id`、`node_id`、父关系、序号、图版本和幂等键。新 worker 的不可重放副作用必须先持久化 `RuntimeIntent`/spawn intent，再交给运行时调度。

恢复时，缺少 terminal record 的 operation 必须保持 `Interrupted` 或 `UnknownOutcome`，不能把进程退出当成成功，也不能自动重放有外部副作用的发送或派生。journal 的有界记录、坏尾行告警、请求指纹冲突和追加锁正好可用于 DAG 消息去重与崩溃恢复；恢复后从事件重建 parent/children 索引，并再次执行全 descendant `Green` 判定。

### `src/runtime.rs`：有界 mailbox、保活、派生与合作式取消

`BackgroundRunner::spawn`、`RuntimeConfig`、`RuntimeEvent`、`JobId`、`CancellationToken` 是最小执行层。`command_capacity`、`event_capacity`、`max_pending` 限制 DAG 扩张；`Accepted`、`Started`、`Queued`、`Completed`、`Closed` 提供生命周期事件；`try_submit` 失败时区分 `QueueFull` 与 `Closed`。可将新工作封装为 DAG worker request，把 mailbox 消费、heartbeat 和 provider 执行放在 job 中，把 `worker_spawned` 与 `KeepAlive` 作为事件发布。

保活的最小机制是：节点持有 `lease_deadline`，在 deadline 前由有界循环检查取消、刷新 lease、消费 mailbox 并发 `RuntimeEvent`；deadline 到期或父会话消失则停止接收新工作并写入终态。派生的最小机制是：`can_close == false` 且确有新工作时，`SessionStore` 先落盘 spawn intent，`BackgroundRunner::try_submit`/新的受限 runner 再启动一个带 parent binding 的 worker，成功或失败都写 terminal event。必须受 `max_pending` 和 generation/lease 策略约束，防止“保活失败→无限派生”的爆炸。

取消是合作式的：`CancellationToken::cancel` 幂等，worker 在 provider chunk、邮箱等待、重试和派生前检查 `is_cancelled`；`mark_completed` 防止晚到取消把已发布成功改写成 `Cancelled`。`shutdown_and_join_with_grace` 只等待有限 grace，非合作任务可能被 detach，不能回滚外部副作用。DAG 取消应向 descendants 传播 `CancelRequested`，阻止已关闭节点继续发布 terminal event；已发送但结果未知的消息/工具操作仍需进入 `UnknownOutcome` 恢复流程。

## 最小端到端控制流

1. headless/protocol 接收带 request ID 的新工作或 DAG 消息，验证 schema、recipient、TTL、binding 和幂等键。
2. `src/core.rs` 解析 parent、grandparent、direct sibling、direct child，写入/读取有界 mailbox；同一消息只允许一次有效 claim。
3. worker 执行任务并发出 `Started`、进度、heartbeat、`Completed`；`src/headless.rs` 以请求隔离的 `AsyncEventBuffer` 转发状态。
4. 节点完成自身工作后，`DagCoordinator::can_close` 递归检查自己及全部 child/grandchild。全绿才 append `dag_closed`；否则 append `dag_close_deferred`、保活，并且仅在有新工作时 append spawn intent 后派生 worker。
5. `src/session.rs` 持久化每个关键转移；崩溃恢复时重放 journal、恢复关系和 mailbox 投影、保留未知结果，再由 host 明确决定 retry/abandon。

## 未决问题

- 源示例未定义 Claude Code hook 输入对象的完整 schema，也没有说明非对象 JSON 的宿主行为；zenpi 应确定严格字段、版本兼容和未知字段策略。
- `grep` 规则故意放行含管道及带前缀命令，不能在迁移时擅自扩展成 shell 解析或安全扫描器；是否把它变成正式 `SideEffectPolicy` 仍需产品策略确认。
- zenpi 尚需确定 `Green` 的权威来源、失败 descendant 是否允许人工 reset、取消是否需要级联到全部后代、lease 默认时长、最大 DAG 深度/generation、邮箱丢弃策略和 spawn 并发上限。
- parent/grandparent/sibling/child 的通信是否只限同一 session、是否允许跨 session，以及消息应采用 request-response、事件广播还是两者并存，需由 protocol schema 固化；同时要决定 `CloseDeferred` 与 `KeepAlive` 是否对外暴露为稳定 wire event。
- 对已发送但进程崩溃的 DAG 消息，`UnknownOutcome`、幂等重试和接收端去重的最终契约尚未由源 hook 给出，必须在 `SessionStore` 与 `RuntimeIntent` 的实现测试中明确。

# AC-001 — claude-code/examples/hooks/bash_command_validator_example.py

- source_id/item_id: `AC-001`
- source_path: `claude-code/examples/hooks/bash_command_validator_example.py`
- source_hash: `0d7a9468405bb614ebddfb56037217cd9282a8045ea87a42cf6ff4099a61820d`
- source_bytes: `2078`
- source_lines: `83`
- coverage: 已按源文件顺序读取完整字节范围 `1-2078`、完整行范围 `L1-L83`（含 shebang、文档注释、导入、私有常量、函数、入口保护）。

## 完整行为复盘

1. 文件级声明与配置说明（`L1-L29`）。`L1` 的 shebang 使文件可直接作为 Python 3 脚本执行；模块文档字符串说明它是 Claude Code `PreToolUse`/`Bash` hook，在工具执行前校验命令，并给出 JSON 配置及命令路径示例（`L5-L27`）。文档中的“改用 `rg`”是行为意图，不会被运行时解析。模块导入 `json`、`re`、`sys`（`L31-L33`），没有网络、文件或子进程依赖。

2. `_VALIDATION_RULES`（私有模块符号，`L35-L45`）。它是有序的 `(regex pattern, message)` 元组列表，当前有两条规则。第一条 `r"^grep\\b(?!.*\\|)"` 只匹配从字符串开头出现的 `grep` 单词，并以负向前瞻排除命令中任何位置含 `|` 的情况；命中消息要求使用 `rg`（`L37-L40`）。因此前导空格、`sudo grep`、`echo x; grep` 不命中，而以 `grep` 开头且没有管道的命令命中；以 `grep` 开头但含管道的命令被放行。第二条 `r"^find\\s+\\S+\\s+-name\\b"` 要求开头为 `find`、后接空白和非空路径，再接 `-name` 单词边界；命中消息建议 `rg --files | rg pattern` 或 `rg --files -g pattern`（`L41-L45`）。规则顺序决定诊断顺序，列表本身没有运行时变更保护。

3. `_validate_command(command: str) -> list[str]`（`L48-L53`）。输入是字符串命令；函数按 `_VALIDATION_RULES` 顺序调用 `re.search(pattern, command)`，每次命中就追加对应消息，最后返回所有问题列表（`L49-L53`）。它不短路，所以理论上可同时返回两条消息；当前两个正则的开头条件使同一普通输入通常只命中一条。空字符串会得到空列表；正则语法或类型错误不会在本函数内转换，调用者传入非字符串可能让 `re.search` 抛出 `TypeError`。函数只读常量、无共享可变状态、无副作用，天然可并发调用。

4. `main()`（`L56-L79`）。首先从标准输入执行 `json.load(sys.stdin)`（`L58`）。仅捕获 `json.JSONDecodeError`：捕获时向标准错误输出 `Error: Invalid JSON input: ...`，再以退出码 `1` 结束（`L59-L62`）；标准输出保持空白。输入 JSON 若为非对象（例如数组、字符串或 `null`），后续 `.get` 会产生未捕获的 `AttributeError`/类似异常，属于进程异常路径而非结构化错误。

   - 读取 `tool_name = input_data.get("tool_name", "")`（`L64`）；缺省为空字符串。只有严格等于 `"Bash"` 才继续，否则立即 `sys.exit(0)`（`L65-L66`），所以该 hook 对其他工具是无操作放行。
   - 读取 `tool_input = input_data.get("tool_input", {})`、`command = tool_input.get("command", "")`（`L68-L69`）。两个字段均有空对象/空字符串默认值，但若 `tool_input` 是非映射值，`.get` 会未捕获失败。
   - `if not command` 对空字符串及其他假值直接 `sys.exit(0)`（`L71-L72`）；不会校验，也不会打印诊断。非字符串真值会进入 `_validate_command` 并可能抛出类型异常。
   - 有问题时按返回顺序逐条向标准错误打印 `• {message}`（`L74-L77`），然后 `sys.exit(2)`（`L78-L79`）。注释明确退出码 `2` 表示阻止工具调用并把 stderr 展示给 Claude；没有问题时函数自然返回，进程退出码为 `0`，不产生输出。脚本直接运行时由 `if __name__ == "__main__": main()` 调用（`L82-L83`）；被导入时不执行输入读取。

5. 并发与边界语义。一次进程只解析一份 stdin、同步完成一次决定；没有线程、锁、异步任务、超时或跨请求状态（`L48-L83`）。并发语义由 Claude Code 为每个 hook 进程隔离提供：同一进程不能安全地复用多份 JSONL 输入。stderr 是人类/模型诊断通道，stdout 未被使用，退出码是唯一机器可判定的阻断信号。该实现只“建议/阻止”，不执行 `grep`、`find` 或 `rg` 命令。

## 状态、取消、恢复与副作用

脚本没有显式状态机、取消令牌、超时、重试或恢复逻辑；每次 `main()` 调用均从 stdin 的单个 JSON 建立瞬时状态，并在 `sys.exit` 后销毁（`L56-L83`）。没有持久化、日志文件、环境变量写入或外部 API 调用。可观察副作用只有向 stderr 写入 JSON 错误或规则消息，以及退出码 `0/1/2`；规则命中不会修改命令文本，因此“改用 `rg`”必须由上游重新提交命令完成。若宿主在退出前取消进程，源文件没有清理钩子；若 stdin 阻塞，`json.load` 也没有本地超时。重试由宿主决定，重复执行不会因为本脚本留下状态而改变结果。

## 源内测试与行为判据

源文件及其同目录内容未包含测试（源内未包含测试）。可独立验证的判据如下：

- 输入 `{"tool_name":"Bash","tool_input":{"command":"grep foo file"}}`：退出 `2`，stderr 含 `Use 'rg'`；输入 `grep foo | sort`：退出 `0`，因 `L38` 的管道排除生效。
- 输入 `{"tool_name":"Bash","tool_input":{"command":"find src -name '*.rs'"}}`：退出 `2`，stderr 含 `rg --files`；输入 `{"tool_name":"Read"...}` 或空 command：退出 `0` 且无诊断（`L64-L72`）。
- 输入破损 JSON：退出 `1` 且 stderr 以 `Error: Invalid JSON input:` 开头（`L58-L62`）。
- 规则消息必须保持 `_VALIDATION_RULES` 顺序；同一输入的每条命中各输出一行 `• ` 前缀（`L74-L77`）。

## zenpi Rust 映射

这段 Python 的可迁移核心是“在副作用边界前做纯校验，使用结构化诊断和明确阻断结果”。在 zenpi 中建议把规则实现为 `ToolRegistry`/`SideEffectPolicy` 的纯 preflight，而不是放进 `src/providers/**`；provider（`src/providers/anthropic.rs`、`openai.rs`、`codex.rs`、`google.rs`、`deepseek.rs` 及 `registry.rs`）只负责请求/流事件，不能绕过统一门。

- `src/protocol.rs` 已提供版本化 JSONL、`StdioRequest`/`Command`、`StdioResponse`/`StdioEvent`（协议骨架在 `L1-L37`、请求字段在 `L152-L209`，`parse_line`/编码器在 `L795-L1000`）。可增加 `Command::DagMessage` 或复用现有 `MailboxRequest::{Send,Receive,Acknowledge,Claim,Complete}`（`L81-L113`）传递校验结果、心跳和派生请求。建议 Rust 类型为 `DagRecipient::{Parent,Grandparent,Sibling(NodeId),Child(NodeId)}`、`DagEnvelope { message_id, sender, recipient, ttl_ms, payload }`；解析时复用 `MAX_ID_BYTES`、文本/TTL 边界和 `ProtocolError`，验证失败返回带稳定 code 的 `StdioResponse::error_with_code`，对应 Python 的退出 `1/2`。
- DAG 通信原语的最小实现是：每个节点一个有界 mailbox（`VecDeque` 或 `mpsc::sync_channel`），一个事件流（`StdioEvent`/`RuntimeEvent`），以及一个带 `message_id`、父会话/节点 ID、序号、TTL 的协议 envelope。recipient resolver 维护 `parent` 与 `children` 索引：parent 直接寻址；grandparent 先取 `parent.parent`；direct sibling 通过同一 parent 的 children 集合过滤自身；direct child 直接遍历 children。邮箱操作按 `Send → Claim → Complete/Acknowledge` 建立幂等边界，过期消息产出 `MailboxOutcome::Abandoned`，避免无限重试。
- `src/core.rs` 已有 `WorkerExecutionBinding`（`L41-L80`）将 `goal_id`、`item_id`、`lease_id`、策略摘要和过期时间绑定到 worker；`Turn` 的 `parent_id`（`L140-L175`）可承载会话父子关系。建议新增 `DagNodeState { node_id, session_id, parent_session_id, status, children, lease_deadline }` 与 `DagStatus::{Running,Green,Failed,Cancelled,Waiting}`，并让每个派生 worker 的 `Turn::with_parent` 指向父 turn/session。grandparent 和 sibling 关系必须由持久化图索引解析，不能仅靠 provider prompt 文本。
- `src/session.rs` 是追加式 JSONL 恢复边界：`SessionStore`、`OperationKind/OperationOutcome` 和 `RuntimeIntent` 记录（`L49-L95`、`L117-L160`）适合记录 `dag_node_created`、`dag_message_sent/claimed/completed`、`worker_heartbeat`、`worker_spawned`、`dag_node_green`、`dag_close_deferred`。派生前先持久化不可重放的 spawn intent；恢复时缺少 terminal record 应保持 `Interrupted/UnknownOutcome`，要求确认后再重试，不能把进程退出当作成功。
- `src/runtime.rs` 的 `BackgroundRunner` 提供有界 command/event channel、FIFO pending、`CancellationToken` 和 `RuntimeEvent::{Accepted,Started,Queued,CancelRequested,Completed,Closed}`（`L49-L99`、`L101-L193`）。它是保活与派生的最小执行层：heartbeat 只在 lease 到期前刷新并发出事件；新工作到达时 `BackgroundRunner::spawn`（`L272-L280`）派生一个有界 worker；`RuntimeConfig::max_pending`、event capacity 防止 DAG 爆炸。取消是合作式的；`CancellationToken::is_cancelled` 必须在 provider chunk、邮箱循环和重试前检查，不能承诺回滚外部副作用。
- `src/tool_runtime.rs` 的 `execute_tool_batch`（`L157-L176`）已经规定批量调用的并发上限、取消轮询、结果按源顺序归并；可把 `_validate_command` 的“命中即阻断”做成 `prepare` 回调中的 `ToolBatchDecision::Reject`（`L133-L148`），并把 Bash/`find` 规则放入工具策略层。`classify_master_session_input`（`L75-L110`）的输入长度、控制字符和空值拒绝模式可复用为 mailbox 文本校验。
- `src/approval.rs` 的 `ApprovalCoordinator` 是进程内消息/条件变量 rendezvous，并支持取消、pending/accepted 持久化前置（`L126-L191`）；DAG worker 的副作用操作应携带 `WorkerExecutionBinding` 与 lease，复用 `ApprovalRequest::validate` 的 ID、digest、lease 校验（`L58-L103`），不能因“worker”身份自动放行。`ApprovalMode::WorkerAllowAfterPreflight` 适合作为规则已验证且 gate 匹配后的显式策略。
- close/保活/派生判据必须明确实现：`DagCoordinator::can_close(node)` 只有在“节点自身 `Green` 且其全部 child/grandchild 递归均为 `Green`”时才为真；任一 descendant 为 `Running/Waiting/Failed/Unknown` 都返回 false。false 时至少写入 `dag_close_deferred`、刷新 lease/发 `KeepAlive` 事件，并在存在新工作且容量允许时持久化 spawn intent 后调用 `BackgroundRunner` 派生新 worker；没有新工作时只保活，不凭空派生。节点 close 事件需在 journal 中包含图版本/子孙摘要，恢复后重新计算而不是信任旧布尔值。

可执行差异清单：

1. 为 `protocol.rs` 增加 DAG recipient、heartbeat、spawn、close-deferred 的带版本 schema，并为 `MailboxRequest::Send` 增加 parent/grandparent/sibling/child 解析入口；写解析/边界测试验证 TTL、ID、消息大小。
2. 在 `core.rs` 增加图状态与 `can_close` 递归函数，输入为不可变快照，输出 `Result<bool, DagError>`；验证循环边、未知 child、重复 message ID 均 fail closed。
3. 在 `session.rs` 写入上述 DAG 事件和 spawn intent，恢复后重放为同一图；模拟尾部坏行时保留完整记录并报告 `RecoveryWarning`。
4. 在 `runtime.rs` 用有界 mailbox/event channel 驱动 heartbeat 与派生，验证 `max_pending`、取消和 shutdown grace；禁止 detached worker 继续发布已关闭节点的 terminal event。
5. 在 `tool_runtime.rs` 接入纯 Bash validator，验证规则命中返回 `Reject`、无命中才进入 approval/provider；在 `providers/**` 添加事件转发适配但不复制策略。

## 未决问题

- 源文件未定义 Claude Code hook 输入 JSON 的完整 schema，也未说明宿主是否会传入非对象 JSON；因此 Rust 侧应以 `ProtocolError` 明确拒绝，而不能假定 `.get` 永远存在。
- `grep` 规则有意放行含管道的命令以及带前缀的命令；无法从源确认这是完整安全策略还是示例简化，迁移时不能擅自扩大为 shell 解析器。
- DAG 中“全绿”的持久化枚举、失败节点是否允许人工重置、spawn 的最大代数和 lease 默认时长，源文件无法确认；这些应由 zenpi 的 blueprint/policy 明确定义后再固化到协议。

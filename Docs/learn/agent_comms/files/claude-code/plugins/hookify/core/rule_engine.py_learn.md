# AC-009 — claude-code/plugins/hookify/core/rule_engine.py

- source_id/item_id: `claude-code/plugins/hookify/core/rule_engine.py` / `AC-009`
- source_path: `claude-code/plugins/hookify/core/rule_engine.py`
- source_hash: `3bc18bd7167ab4ea2c06b9693b1994d267e0678c9210db9761c93060607f9fce`
- source_bytes: `10727`
- source_lines: `313`
- coverage: 已按顺序读取字节 `1-10727`、行 `L1-L313`，含注释、类型标注、导入、导出类/函数及 `__main__` 分支。

## 完整行为复盘

- 模块导入与依赖（`L1-L10`）：脚本入口为 Python3；依赖 `re`、`sys`、`lru_cache`，以及 `List/Dict/Any/Optional`。`Rule`、`Condition` 从 `hookify.core.config_loader` 导入。文件不负责加载规则、筛选 `enabled` 或按 `event` 预过滤。
- `compile_regex(pattern: str) -> re.Pattern`（`L13-L24`）：模块级 `@lru_cache(maxsize=128)`；每个 pattern 编译为 `re.IGNORECASE` 正则并返回。缓存是进程内、按参数键控的 LRU，超过 128 个模式淘汰旧项；没有磁盘持久化、取消、超时或重试。无效模式在这里抛 `re.error`，调用方 `_regex_match` 会吸收。多个线程可共享该函数和缓存；函数本身无业务锁，规则引擎实例也不保存可变状态。
- `RuleEngine.__init__`（`L27-L33`）：空初始化，仅 `pass`；注释说明不再使用实例缓存，因此可安全创建多个无状态实例。
- `RuleEngine.evaluate_rules(rules, input_data)`（`L35-L94`）：读取 `hook_event_name`，缺省为空字符串；逐条调用 `_rule_matches`（`L49-L59`）。`action == 'block'` 才进入 blocking，其余 action 一律视为 warning。阻断优先于警告；消息按规则顺序格式化为 `**[name]**\nmessage`，同类消息以两个换行连接（`L60-L64`）。事件为 `Stop` 时返回 `decision: block`、`reason` 和 `systemMessage`（`L65-L71`）；`PreToolUse`/`PostToolUse` 返回 `hookSpecificOutput.hookEventName` 与 `permissionDecision: deny`，同时返回 `systemMessage`（`L72-L79`）；其他事件只返回 `systemMessage`（`L80-L84`）。无阻断但有警告时仅返回合并后的 `systemMessage`，不拒绝操作（`L86-L91`）；无匹配返回 `{}`（`L93-L94`）。输入字典缺少字段不会报错，规则列表为空也返回空字典。方法不修改 rules/input_data，因而同一实例可被串行复用；并发调用只共享正则缓存，不共享评估结果。
- `_rule_matches(rule, input_data) -> bool`（`L96-L125`）：`tool_name` 缺省 `''`，`tool_input` 缺省 `{}`（`L106-L109`）。存在非空 `rule.tool_matcher` 时先调用 `_matches_tool`，失败立即返回 `False`（`L110-L113`）。无条件的规则强制不匹配（`L115-L119`）；有条件时逐个 `_check_condition`，采用全 AND，任一失败短路，全部成功才 `True`（`L121-L125`）。不存在条件不会默认放行。
- `_matches_tool(matcher, tool_name) -> bool`（`L127-L143`）：精确 `'*'` 匹配任意工具；否则按字面 `|` 分割，再以 `tool_name in patterns` 精确比较。它不是正则、没有 trim、没有大小写折叠；例如 `Edit|Write` 只接受完全相同的两个名称，空片段也按普通字符串处理。
- `_check_condition(condition, tool_name, tool_input, input_data=None) -> bool`（`L144-L181`）：先由 `_extract_field` 取值，`None` 直接失败（`L157-L160`）；读取 `operator`、`pattern`（`L162-L165`）。支持 `regex_match`（调用 `_regex_match`）、`contains`、`equals`、`not_contains`、`starts_with`、`ends_with`（`L166-L176`）。未知 operator 静默返回 `False`（`L178-L180`）。直接字段通常已转成字符串，但工具专用分支可能返回非字符串；若调用 `startswith` 等要求字符串的方法而 pattern/返回值类型异常，可能抛 `TypeError`，该函数不做总兜底。
- `_extract_field(field, tool_name, tool_input, input_data=None) -> Optional[str]`（`L182-L254`）：优先读取 `tool_input[field]`；字符串原样返回，其他值 `str(value)`（`L195-L200`）。若 `input_data` 为真值，`reason` 返回 `input_data.get('reason','')`，`user_prompt` 返回对应字段（`L202-L208`、`L226-L228`）。`transcript` 从 `transcript_path` 读取整个文件（`L209-L214`）；文件不存在、无权限、一般 `IOError/OSError`、`UnicodeDecodeError` 均向 stderr 打 warning 并返回空串（`L215-L225`），无路径则继续后续分支。工具特例：`Bash.command`（`L230-L234`）；`Write`/`Edit` 的 `content` 取 `content` 或回退 `new_string`，`new_text/new_string`、`old_text/old_string`、`file_path` 分别映射到对应键（`L235-L245`）；`MultiEdit.file_path` 直接取值，`new_text/content` 将 `edits[*].new_string` 用空格连接（`L246-L252`）。找不到字段返回 `None`（`L254`）。读取 transcript 是唯一显式外部 I/O，未设大小上限、编码参数或超时；畸形 `edits` 元素可能抛异常。
- `_regex_match(pattern, text) -> bool`（`L256-L273`）：调用缓存编译器并执行 `regex.search`，任意搜索命中即 `True`（`L267-L269`）；仅捕获 `re.error`，将无效 pattern 和异常写 stderr 后返回 `False`（`L271-L273`）。缓存会使相同模式避免重复编译，但不缓存匹配结果。
- `__main__` smoke block（`L276-L313`）：构造一个 `Rule`（`L280-L289`），条件为 `command` 的 `regex_match rm\s+-rf`；对 Bash `rm -rf /tmp/test` 打印 `Match result`，对 `ls -la` 打印 `Non-match result`（`L291-L313`）。这是可执行示例，不是测试框架断言；`Rule` 的默认 action 是否为 block 由 `config_loader` 决定。

## 状态、取消、恢复与副作用

本文件没有取消 token、deadline、重试、任务恢复、规则持久化或事务。`RuleEngine` 及其结果均为同步、瞬时内存值；唯一跨调用状态是 `compile_regex` 的进程内 128 项 LRU。评估失败通常以 `False`/`{}` 表示，未知 operator 和缺失字段不会抛业务错误；正则语法错误及 transcript 读取错误会写 stderr。读取 transcript 会产生外部文件访问副作用，但不写文件、不改变会话、不发送网络请求。并发时实例无共享可变字段；同一 `input_data` 若被调用方并发修改，源码没有防护，调用方需提供不可变快照。

## 源内测试与行为判据

源内未包含测试；同目录仅有 `__init__.py`、`config_loader.py`、`rule_engine.py`。`L276-L313` 仅是手工 smoke 示例。可独立验证的判据：运行 `python rule_engine.py` 应分别输出匹配/不匹配两条结果；构造 `Stop`、`PreToolUse`、普通事件的 blocking rule，核对三种返回结构；用 `Edit|Write`、`*` 和未知 operator 验证工具匹配与短路；用无效正则核对 stderr warning 且结果为不匹配；用临时 transcript 文件、缺失路径和无权限路径核对读取内容及空串降级；重复 129 个 pattern 后检查缓存仍只保留最近 128 个（可通过 `compile_regex.cache_info()` 观察）。

## zenpi Rust 映射

现有可复用原语是结构化消息、邮箱、事件和会话日志：`src/protocol.rs:L79-L113` 的 `MailboxRequest::{Send,Receive,Acknowledge,Claim,Complete}` 已提供带 `message_id`、TTL、结果状态的邮箱协议，`L544-L582` 做边界校验；`StdioEvent` 是带单调 `sequence`、`request_id`、`turn_id` 的进度事件（`src/protocol.rs:L882-L918`）。`src/headless.rs:L806-L1061` 的 `AsyncEventBuffer`、有界 provider/agent mailbox 与 `PendingSteer` 可作为背压和跨 worker 事件转发模板。`src/runtime.rs:L38-L99` 的 `JobId`/`CancellationToken`、`L101-L193` 的容量配置和终态事件，以及 `L712-L759` 的 `start_job`，适合作为派生 worker 的最小调度机制。`src/approval.rs:L126-L245` 的 `ApprovalCoordinator` 展示了 `Arc<Mutex<_>>+Condvar`、可见请求、取消和持久化前保留响应的会话式父子通信。

建议的可执行落点与 DAG 约束如下：

1. `src/protocol.rs`：新增 `DagNodeId`、`DagRelation { Parent, Grandparent, Sibling, Child }`、`DagMessage { node_id, sender, recipient, relation, kind, payload, correlation_id, ttl_ms }` 与 `DagControl::{Close, KeepAlive, Spawn}`，复用 `validate_identifier`、`validate_mailbox_text` 和现有 `MailboxRequest` 的有界文本/TTL校验。接收端只能根据会话内已知父链授权：parent/grandparent 是祖先，direct sibling 必须共享直接 parent，direct child 必须是自身登记的 child；拒绝任意越级 recipient。
2. `src/session.rs`：`SessionStore` 已以 append-only JSONL 保存 `Turn`、handoff、runtime intent 和不透明 event（`L1142-L1223`），可增加 `dag_node_registered`、`dag_message_sent/claimed/completed`、`dag_status_changed`、`dag_keepalive`、`dag_spawn_requested` 事件辅助函数。用 `core::Turn.parent_id`（`src/core.rs:L140-L204`）保存会话父子关系，并在恢复时重建 `HashMap<NodeId, {parent, children, status}>`；grandparent、sibling 通过父链计算而非重复持久化。
3. `src/core.rs`：在 `Agent`（字段与事件见 `L403-L442`、`AgentEvent` 见 `L281-L325`）增加当前 DAG 节点上下文和 `DagStatus`；turn 完成时执行 `close_gate(node)`：只有“节点自身为 Green 且所有递归 child/grandchild 均 Green”才发 `Close`。否则保持 `AgentPhase::Running`/保活，写入 `dag_keepalive`，并把新任务封装为待派生请求；可把 parent/child 通信映射为新的 `AgentEvent::DagMessage`，不把 provider 文本误当作状态确认。
4. `src/runtime.rs`：复用有界 `BackgroundRunner`。最小保活/派生流程是：在 session 先原子追加 `dag_keepalive` 与带幂等键的 `dag_spawn_requested`，再 `try_submit` 新 `JobId`；收到 `Started` 才记录 child 活跃，收到 `Completed`/`Cancelled`/`Panicked` 才更新状态。用 `CancellationToken` 让 close、超时或父节点取消向下传播；不能把取消当作回滚，沿用源码注释中的“副作用不可逆”语义。
5. `src/headless.rs`：将 `MailboxRequest`/`DagMessage` 路由到目标 session，使用已有 replay journal、`StdioEvent.sequence` 和有界 `AsyncEventBuffer`；对慢 consumer 丢弃普通进度时必须保留 admission、status、close/spawn 等终态标记。`PendingSteer` 的竞态处理可复用于“父节点尚未拿到 turn_id 时子消息暂存”。
6. `src/tool_runtime.rs`：DAG 消息发送、claim、complete 应视为可审计的非 provider tool 操作；沿用 `execute_tool_batch` 的 `cancelled` 轮询、bounded batch、join 语义（`L157-L176`、`L252-L330`）。涉及真实外部副作用的新 worker 必须先获得现有 approval/policy 决策，并在 unknown outcome 时阻止盲目重试。
7. `src/approval.rs`：对派生 worker 的 spawn、跨节点消息和 close 设定 `ApprovalRequest` 的 `request_id/turn_id/call_id` 关联；复用 `request_response` 后先写 `approval_resolved` 再执行副作用，取消时清理 pending。父节点撤销应调用 `cancel_all`/`emergency_cancel`，子节点只能观察取消而不能伪造 Green。
8. `src/providers/**`：`providers/anthropic.rs`、`google.rs`、`openai.rs`、`deepseek.rs`、`codex.rs` 只负责协议/路由；统一 `Backend` 的 `ProviderEvent`（`src/backend.rs:L236-L276`）和可取消 sink（`L492-L531`）作为 worker 内部事件来源。provider 的 `Completed/Failed` 只能更新该节点执行结果，不能直接满足“全部 descendants Green”的 DAG close 条件；由 `core/session/runtime` 聚合状态。

与源实现的差异：Python 是单次同步规则求值，只有 block/warning 两类输出，没有节点身份、父链、兄弟/子节点授权、可靠邮箱、持久化事件或派生调度；zenpi 需要新增 typed DAG 状态机、关系校验、幂等消息和递归 close gate。最小可验证场景是建立 root→child→grandchild：grandchild 未 Green 时 root 的 close 被拒绝且产生 keepalive；提交新任务后出现唯一 `dag_spawn_requested` 和一个新 `JobId`；child 可向 parent、grandparent、直接 sibling、直接 child 发消息，越级 recipient 被 `ProtocolError` 拒绝；所有 descendant Green 后才产生一次可重放的 close 终态事件。

## 未决问题

无法从源确认：`Rule` 的完整字段默认值（尤其 `action`、`enabled`、`event`）、调用方是否已在外层过滤 disabled/event、以及 transcript 的预期最大大小与编码策略。DAG 映射还需产品侧确定 Green 的精确定义、跨 session 的授权存储位置、spawn 配额和消息 TTL 到期后的最终状态；这些均不是 `rule_engine.py` 可推导出的行为。

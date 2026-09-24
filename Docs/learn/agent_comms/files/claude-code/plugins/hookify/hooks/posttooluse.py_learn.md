# AC-011 — claude-code/plugins/hookify/hooks/posttooluse.py

- source_id/item_id：AC-011
- source_path：claude-code/plugins/hookify/hooks/posttooluse.py
- source_hash：b440f17f3a6c962d87f094ce8974cb6090881adec2e495eb2356aa89b945f741
- source_bytes：1776
- source_lines：66
- coverage：已从第 1 字节至第 1776 字节、L1-L66 按顺序完整读取，包含注释、导入、异常分支及 `__main__` 入口。

## 完整行为复盘

这是一个同步的 Python `PostToolUse` hook 进程。文件头 shebang 与模块 docstring 说明它在 Claude Code 工具执行后被调用，读取 `.claude/hookify.*.local.md` 规则并评估（L1-L6）。`os`、`sys`、`json` 是全部直接依赖（L8-L10）。

模块初始化阶段先读取环境变量 `CLAUDE_PLUGIN_ROOT`（L12-L13）。当变量非空时，取其父目录 `parent_dir`，并将父目录、插件根目录分别以 `sys.path.insert(0, ...)` 放到导入搜索首位；已经存在的路径不会重复插入（L14-L19）。这改变的是当前进程的导入路径，没有写文件或网络副作用。

导入符号 `load_rules`（来自 `hookify.core.config_loader`）与 `RuleEngine`（来自 `hookify.core.rule_engine`）放在 `try` 中（L21-L23）。若任一导入抛出 `ImportError`，构造 `{"systemMessage": "Hookify import error: ..."}`，通过 stdout 输出 `json.dumps` 后以状态码 0 退出（L24-L27）；因此依赖缺失被视为可报告的 hook 消息，而不是进程失败。

唯一函数 `main()` 是入口（L30-L31）：

- 输入：调用 `json.load(sys.stdin)` 读取一个完整 JSON 值（L33-L35）。输入为空、截断、非法 JSON 或底层 stdin 错误都会进入外层 `except`。代码没有显式要求对象形状；后续 `.get` 因非对象而报错时同样走错误输出。
- 事件归类：从 `input_data.get('tool_name', '')` 取工具名，默认空字符串（L37-L38）。`Bash` 映射为事件 `'bash'`；`Edit`、`Write`、`MultiEdit` 映射为 `'file'`；任何其他值（包括缺省）保持 `event = None`（L39-L43）。这意味着未知工具不会被拒绝，而是把 `None` 交给规则加载器。
- 规则加载：调用 `load_rules(event=event)`（L44-L45）。按模块契约，它会查找并解析本地 hookify 规则；本文件不设超时、数量上限或缓存。
- 求值：每次调用都新建 `RuleEngine()`，再以规则列表和原始 `input_data` 调用 `engine.evaluate_rules(rules, input_data)`（L47-L49）。结果不在本文件转换或过滤。
- 输出：无论 `result` 是空字典、列表还是其他可 JSON 序列化值，都执行 `print(json.dumps(result), file=sys.stdout)`；注释明确要求“即使为空也总输出 JSON”（L51-L52）。正常路径没有 stderr 输出，返回值也未被使用。
- 错误：`main` 内任何 `Exception`（包括 stdin、规则加载、规则求值、序列化异常）统一转成 `{"systemMessage": "Hookify error: <str(e)>"}` 并输出 stdout（L54-L58）。异常对象只保留字符串，堆栈不输出。
- 终止：`finally` 无条件执行 `sys.exit(0)`（L60-L62），所以正常和异常路径都以零状态结束；若 `print` 本身触发未被捕获的 `BaseException`，仍会尝试退出。模块仅在直接执行时调用 `main()`（L65-L66），被导入时不会自动处理 stdin。

并发语义是“每个进程、每次调用、单线程、串行一次求值”：源中没有线程、锁、异步任务、共享状态或重入控制；`RuleEngine` 实例不跨调用复用（L30-L52）。stdin 是一次性全量读取，stdout 是单条 JSON 行（L34-L35、L51-L58）。

## 状态、取消、恢复与副作用

源文件没有显式取消令牌、超时、重试、恢复点或持久化状态。`CLAUDE_PLUGIN_ROOT` 只影响导入路径（L12-L19）；规则文件读取与解析由 `load_rules` 隐含完成，执行结果由 `RuleEngine` 计算。唯一外部可观察副作用是 stdout 上的 JSON；错误也保持 stdout 协议并以零退出（L24-L27、L54-L62）。进程结束后内存中的 `event`、规则和引擎全部丢弃。若规则本身触发副作用，其责任属于规则引擎/调用方，本文件没有事务或回滚语义。

## 源内测试与行为判据

源内未包含测试，目标目录也没有发现针对 `posttooluse.py` 的测试文件；同目录只有其他 hook、`config_loader.py` 和 `rule_engine.py`。可独立验证的判据：

1. 设置有效 `CLAUDE_PLUGIN_ROOT`，向 stdin 输入 `{"tool_name":"Bash", ...}`，应调用 `load_rules(event="bash")`；`Edit`/`Write`/`MultiEdit` 应为 `event="file"`；未知或缺省工具应为 `None`（L37-L45）。
2. 用假的 `hookify.core.*` 模块记录调用，确认每次新建 `RuleEngine` 并把原始 JSON 传入 `evaluate_rules`（L21-L23、L47-L49）。
3. 让导入失败、stdin 提供非法 JSON、规则加载抛出普通 `Exception`，均应得到包含 `systemMessage` 的单行 JSON，进程退出码仍为 0（L24-L27、L54-L62）。
4. 正常求值返回空值时仍应有一行 `json.dumps(result)` 输出（L51-L52）。

## zenpi Rust 映射

该 hook 的核心可抽象为“工具完成事件 → 按工具类型选择规则 → 同步评估 → 输出协议消息”。zenpi 已有更强的可复用原语，但需要把 DAG 拓扑与绿色关闭条件显式化：

- **通信原语/协议**：`src/protocol.rs` 已有版本化 JSONL `StdioRequest`/`StdioResponse`/`StdioEvent`（约 L793-L900），以及 `Command::Mailbox`、`MailboxRequest` 的 `Send/Receive/Acknowledge/Claim/Complete`（约 L30-L110、L211-L259）。将 post-tool 结果建模为 `StdioEvent { kind: "tool_completed", turn_id, event }`，规则命中建模为结构化 `data`，不要把诊断混入 stdout。DAG 节点间消息优先复用 `MailboxRequest`，事件流复用 `StdioEvent`；规则选择可做成 `ToolRuleSet::for_event("bash"|"file")`。
- **持久邮箱与幂等**：`src/session.rs` 的 `MailboxMessage`/`MailboxStatus`、摘要校验、TTL、claim token、`SessionMailbox::enqueue/find_message`（L3169-L3265、L3570-L3665）可直接承载 parent、grandparent、直接 sibling、直接 child 的定向消息。发送方把 `recipient_session_id` 与 `request_id` 固定；接收方 `claim` 后只能一次 `Complete/Fail`，避免重复派生。`LiveSessionRegistry::register/heartbeat/active`（L3274-L3355）提供收件人在线租约。
- **会话父子关系**：`src/core.rs` 的 `Turn.parent_id` 与 `Turn::with_parent`（约 L100-L150）保存直接父关系；`src/session_tree.rs` 的 `TreeEntry.parent_id`、`ancestry`、`children` 索引（L48-L84、L245-L268）可推导 grandparent、直接 sibling（同一 `parent_id` 的节点）和直接 child。建议在 `session_tree.rs` 增加只读 `neighbors(node_id) -> {parent, grandparent, siblings, children}`，并在 `src/session.rs` 通过 `SessionStore` 的树快照提供给 mailbox 路由；不得靠文件名猜测关系。
- **worker/规则落点**：工具执行完成后的判定点在 `src/tool_runtime.rs::execute_tool_batch`（L157-L176、L219-L249、L252-L347）及 `src/core.rs` 的工具结果持久化路径；这里应生成 `ToolCompleted` 事件并按 `ToolExecutionOutcome` 选择 DAG 规则。`src/providers/**`（Anthropic/Codex/DeepSeek/Google/OpenAI 适配器）只负责 `Backend`/`ProviderEvent` 流和取消检查，不应直接决定 DAG close；provider 事件可作为节点进度输入。
- **保活与取消**：`src/runtime.rs::CancellationToken` 是协作取消、幂等 `cancel`、`mark_completed` 的最小基础（L49-L98）；`BackgroundRunner` 的有界命令/事件通道、`try_submit`、`try_cancel`、`RuntimeEvent::{Accepted,Started,Queued,CancelRequested,Completed,Closed}`（L101-L190、L272-L365）可承载节点 worker。节点租约应复用 `core.rs::WorkerExecutionBinding` 的 `lease_id`/`expires_at_ms` 校验，并由 `LiveSessionRegistry::heartbeat` 延长保活。超时只标记节点未绿并触发取消/重派，不把取消误判为成功。
- **派生最小机制**：在 `src/runtime.rs` 增加 `DagWorkerSpec { node_id, parent_session_id, lease_id, generation }` 与 `DagWorkerEvent::{Heartbeat, ToolCompleted, Green, Failed, SpawnRequested}`；用 `BackgroundRunner::try_submit` 派生一个新 job，把父/祖父及邻居目标写入 mailbox。派生前在 `src/core.rs` 按 binding 与 approval 规则预检；副作用工具仍经 `src/approval.rs` 的 `ApprovalCoordinator`，其请求取消和持久审计语义可复用（约 L80-L170）。
- **全部 child/grandchild 绿色才 close**：建议在 `src/session_tree.rs` 增加 `NodeHealth`（`Pending/Running/Green/Failed/Expired`）及 `aggregate_close(node)`：读取节点自身、所有后代（child 与 grandchild 以及更深后代）的最终状态，只有全部 `Green` 才返回 `CloseDecision::Close`；任一非绿返回 `KeepAlive`。在 `src/session.rs` 追加不可变事件（如 `dag_node_state`、`dag_close_decision`）保证崩溃恢复；在 `src/headless.rs` 将结果映射成可重放 `StdioEvent`，利用现有 WAL、请求指纹和有界 replay（L33-L52、L334-L381）保证重连幂等。
- **落点差异清单**：现有 zenpi 有邮箱、树、租约、取消、审批和持久日志，但没有“工具后按 `Bash`/文件类选择 hook 规则”的统一 Rust `RuleEngine`；没有后代健康聚合与 close gate；`LiveSessionRegistry::claim_next` 明确“不启动 worker”（L3361-L3410），所以必须由 runtime/core 显式消费 claim 并 `try_submit`；现有 `TreeAction::Fork`（`src/protocol.rs` L1161-L1219）偏会话分叉，不能代替 DAG worker 派生协议；provider 适配器也没有 DAG 拓扑权限。

## 未决问题

无法从本源确认 `load_rules` 规则文件的具体语法、规则冲突优先级、`RuleEngine.evaluate_rules` 返回值 schema，以及 hook 宿主是否会重试同一输入；这些应在 `core/config_loader.py`、`core/rule_engine.py` 和宿主 hooks 配置中另行核对。源文件也未定义 DAG、父子会话或绿色状态语义。

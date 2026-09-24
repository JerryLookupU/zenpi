# AC-012 — claude-code/plugins/hookify/hooks/pretooluse.py

- source_id：AC-012
- item_id：AC-012
- source_path：claude-code/plugins/hookify/hooks/pretooluse.py
- source_hash：cf3f718f26f57979686ca00d9cccbe09bc7920943bdc35cd689ba5163a777aac
- source_bytes：2205
- source_lines：74
- coverage：已从字节 1-2205、行 L1-L74 按文件顺序完整读取，包含 shebang、模块注释、导入、路径设置、异常分支、`main` 及入口判断。

## 完整行为复盘

1. 模块声明与依赖（L1-L10）。L1 的 shebang 使文件可直接由 Python 3 执行。L2-L6 的模块文档说明它是 `PreToolUse` hook：在 Claude Code 执行任意工具前运行，读取 `.claude/hookify.*.local.md` 并评估规则。L8-L10 导入 `os`、`sys`、`json`；模块没有自建线程、锁、队列或持久化。
2. `PLUGIN_ROOT` 路径初始化（L12-L23）。L14 从环境变量读取 `CLAUDE_PLUGIN_ROOT`，缺失或空字符串时不改 `sys.path`，仍继续导入。存在时，L17 取插件目录的父目录；L18-L19 仅在不重复时把父目录插入 `sys.path` 首位，使 `hookify` 包可解析。L22-L23 再把插件根目录插入首位（同样去重），供其它脚本导入。插入顺序意味着后一次插入的 `PLUGIN_ROOT` 位于父目录之前；代码不校验路径存在性、权限或是否为目录，错误延迟到导入阶段。
3. 导入失败降级（L25-L32）。L26-L27 导入 `load_rules` 与 `RuleEngine`。仅 `ImportError` 被捕获；L30 构造 `{"systemMessage": "Hookify import error: ..."}`，L31 以 JSON 写标准输出，L32 `sys.exit(0)`。因此插件包缺失、路径错误等不会阻断原工具；该阶段其它 `BaseException` 不经过此分支。
4. `main()`（L35-L70）。L36 是唯一公开入口函数，返回值没有使用，所有结果通过 stdout 输出。
   - 输入（L37-L40）：L39 对 `sys.stdin` 调用 `json.load`，要求 stdin 是一个完整 JSON 值；空输入、截断 JSON、非对象 JSON 都会在后续或此处触发异常。代码没有显式大小上限、字段白名单或 schema 校验。
   - 事件筛选（L41-L50）：L43 读取 `input_data.get('tool_name', '')`，默认空字符串；因此输入若不是支持 `.get` 的对象（如 JSON 数组）会报错。`Bash` 精确映射为事件名 `bash`（L46-L47）；`Edit`、`Write`、`MultiEdit` 精确映射为 `file`（L48-L49）；其它名称（包括空值、未知工具、大小写不同）保持 `event=None`（L45、L50），交给规则加载器决定是否匹配。
   - 规则与执行（L51-L56）：L52 调用 `load_rules(event=event)`，加载规则的文件发现、解析和过滤责任在外部模块；L55 创建新的 `RuleEngine`，每次 hook 调用都不复用实例；L56 以原始 `rules` 和完整 `input_data` 调用 `evaluate_rules`，结果保存为 `result`。这里没有显式短路、重试或并行，调用是同步的。
   - 输出（L58-L59）：无论 `result` 是否为空都执行 `json.dumps(result)` 并输出一行 stdout JSON；“无规则”应由引擎返回可序列化空结果，而不是省略输出。stdout 同时承载正常结果和诊断，调用者需按 JSON 解析。
   - 错误路径（L61-L67）：捕获所有 `Exception`，生成 `{"systemMessage": "Hookify error: <str(e)>"}` 并输出。错误不会重新抛出，也不会输出 stderr；异常文本可能来自 JSON、规则加载或引擎。`KeyboardInterrupt` 等继承 `BaseException` 的信号不进入此 except，但仍会执行 finally。
   - 终止语义（L68-L70）：`finally` 无条件 `sys.exit(0)`；成功、规则拒绝、输入错误、引擎错误都以进程码 0 结束，宿主无法用退出码区分“允许”与“发生错误”，必须检查 JSON。finally 中的退出还会覆盖普通 `Exception` 的自然返回以及大多数未捕获 `BaseException` 的退出路径。
5. 入口导出（L73-L74）。仅当脚本以主程序运行时调用 `main()`；作为模块导入时只执行路径设置和导入副作用，不自动读取 stdin。源文件没有类、常量导出 API、异步函数或测试函数；可复用边界是 `main` 及其依赖的 `load_rules`/`RuleEngine`。

并发语义：单次进程内是串行的，一次只处理一个 stdin JSON；没有锁和共享状态。多个 Claude Code hook 进程可由宿主并发启动，规则文件和 stdout 的并发安全、跨进程竞态不由本文件保证。每次 `main` 都新建引擎，故本文件不提供跨调用缓存或顺序保证。

## 状态、取消、恢复与副作用

本文件只有瞬时状态：`input_data`、`event`、`rules`、`engine`、`result` 均为一次调用的局部变量。没有取消 token、超时、重试循环、检查点、恢复读取或持久化写入；唯一外部读取是环境变量、stdin 与由 `load_rules` 间接读取的规则文件。没有直接执行工具、写文件、网络请求或修改规则；但导入和规则引擎可能在依赖模块内部产生副作用，脚本对此不隔离。发生异常时“允许操作”是协议约定（JSON 诊断 + exit 0），不是回滚：若下游已产生副作用，本文件不撤销。进程被杀或 stdin 永久阻塞时没有恢复机制；重试只能由 Claude Code 重新启动 hook，且会重新加载规则并重新评估。

## 源内测试与行为判据

源内未包含测试。可独立验证的判据如下：  
(1) 用 `CLAUDE_PLUGIN_ROOT` 指向插件目录运行并传入 `{"tool_name":"Bash"}`，应调用 `load_rules(event="bash")`；`Edit`/`Write`/`MultiEdit` 应为 `file`，未知或缺省工具应为 `None`（L41-L52）。  
(2) 让导入失败，stdout 应是一行含 `systemMessage` 的 JSON，进程码为 0（L25-L32）。  
(3) 传入非法 JSON、数组或让引擎抛出 `Exception`，应得到 `Hookify error: ...` JSON 且进程码仍为 0（L37-L40、L61-L70）。  
(4) 正常返回空结果时仍必须有一行 `json.dumps` 输出（L58-L59）。  
(5) 以模块方式导入而非脚本执行时，不应自动调用 `main`（L73-L74）。

## zenpi Rust 映射

- 通信原语：本文件的“输入 JSON → 分类事件 → 规则评估 → JSON 输出”可映射为 `src/protocol.rs` 的严格 JSONL 请求/事件边界。已有 `MailboxRequest::{Send,Receive,Acknowledge,Claim,Complete}`（`src/protocol.rs` L87-L113）可复用作 worker 间消息/邮箱协议；`MailboxOutcome` 的 `Succeeded/Failed/Abandoned` 对应规则执行结果。`StdioEvent`/`StdioResponse`（`src/protocol.rs` L882-L966）承载逐条诊断与终态。对于 DAG 节点，建议增加或扩展 `WorkerMessage`（sender、recipient、message_id、parent_session_id、kind、payload、ttl）并复用 `MailboxRequest` 的 TTL、分页、claim/complete 语义，而不是在 worker 间共享内存。
- 会话父子关系：`src/core.rs` 的 `Turn { id, parent_id, ... }`（约 L134-L180）已能表达活动转向关系；`src/session_tree.rs` 的 `TreeEntry.parent_id`、`children` 与 `SessionTree::ancestors/page/plan_turn/commit_turn`（L48-L73、L250-L372）适合表达 DAG 节点的会话树索引。建议为 worker 会话增加不可变 `WorkerNodeId`、`parent_session_id` 与有序 `child_session_ids`，并在 `SessionTree` 旁记录直接 sibling/child 查询；grandparent 是沿 `parent_id` 向上两次，直接 sibling 是父节点 children 中除自身者，直接 child 是 children 索引的一层。必须禁止任意跨树 parent，沿用 `plan_turn` 的 ancestry 校验。
- 邮箱落点与身份校验：`src/session.rs` 已有 `MailboxMessage`（L3180-L3194）、状态/TTL/digest 校验（L3208-L3242）、`LiveSessionRegistry` 心跳和 owner epoch（L3274-L3355）、显式 `claim_next`（L3361-L3410）。这正好提供 parent、grandparent、直接 sibling、直接 child 的持久消息通道：发送前校验 workspace 与 DAG 可达关系，接收方用 owner epoch claim，处理完以 Complete 写结果。`src/headless.rs` 顶部的有界 replay/请求去重状态（L1-L18、L334-L380）可作为传输层重放保护；不要把 stdout replay 当成 durable mailbox。
- 保活与派生的最小机制：`src/runtime.rs` 的 `BackgroundRunner`、`CancellationToken`、`RuntimeConfig`（L49-L132、L236-L250）提供有界命令/事件队列、poll interval、取消和 FIFO follow-up；`JobOutcome` 与 `RuntimeEvent`（L156-L193）可承载 `Running/Completed/Cancelled/Panicked`。建议新增最小 `DagNodeState { node_id, parent_id, generation, status, heartbeat_at, required_children, green_children, pending_work }` 与 `DagSupervisor`：节点每个心跳/终态写事件；只有自身 `Succeeded` 且全部直接 child/grandchild 的递归状态为 `Green` 才发布 `CloseEligible`。否则保持 `Alive`，把新工作写入 mailbox 并通过 `BackgroundRunner::submit` 派生一个带新 generation 的 child worker；派生前先持久化 `spawn_intent`，成功注册后写 `spawned`，避免崩溃后重复派生。最小保活是 `LiveSessionRegistry::heartbeat` + TTL；最小派生是 `claim_next`/`MailboxRequest::Claim` 后创建有界 runner job，禁止隐式无限递归。
- `src/core.rs` 落点：在 `Agent` 的 turn admission/steer 路径（`start_or_steer_turn`、`steer_turn`，L3360-L3382）前加 DAG 节点身份与 lease 检查；复用 `append_event`/`append_handoff`/`append_runtime_intent`（约 L5450-L5487）记录 `node_heartbeat`、`node_green`、`spawn_intent`、`close_blocked`。节点关闭函数应在调用 `SessionStore` 前执行递归绿检查，并返回“保活或派生”的明确决定。
- `src/session.rs` 落点：继续使用 append-only `SessionStore`（结构定义约 L130-L160，事件追加 L1207-L1217）作为节点状态事实源；新增事件类型只追加，不重写 transcript。将 DAG node snapshot、generation、close eligibility 和派生幂等键放入事件/恢复投影；借用 `MailboxMessage` 的 digest、TTL、claim token 和 terminal result，恢复时把缺失终态视为 unknown，而不是成功。
- `src/headless.rs` 落点：保留严格 JSONL、request-ID fingerprint、terminal replay 和有界 pending steers（重放状态/`admit` 约 L334-L380）；新增 `dag_message`、`dag_heartbeat`、`dag_close` 命令路由到 owner。输出每条异步 worker 事件为 `StdioEvent`，对慢客户端维持既有有界缓存并显式报告 gap，不能无限积压 child/grandchild 进度。
- `src/tool_runtime.rs` 落点：其 `execute_tool_batch` 的取消轮询、并行上限、join 语义（L157-L175、L283-L321）可复用为一个节点内部工作批次；规则引擎的“任意错误仍返回可解析诊断”对应 `ToolBatchOutcome` 的逐调用结果。节点关闭前不得把未完成 tool 当绿；取消是停止接受结果，不是副作用回滚。
- `src/approval.rs` 落点：worker 派生若涉及工具副作用，沿用 `ApprovalCoordinator` 的 pending/decision、取消与持久化前置约束（L126-L145、L167-L224、L409-L452）。父节点不能通过消息文本伪造 approval；把 `WorkerExecutionBinding`/lease 与 approval request 绑定，审批记录先落 session journal，再允许 child worker 执行。
- `src/providers/**` 落点：provider 层只负责模型/传输能力与取消边界，不应知道 DAG 拓扑；沿用 `ProviderEvent` 流作为节点进度事件，provider 调用前后由 `core` 写 node 状态。若 provider 失败，映射为 node `Failed` 并保活/派生补偿工作，而不是直接 close。
- 差异清单（可执行、可验证）：① 本 Python hook 的事件仅区分 bash/file/None；Rust 需定义带 node/session 关系的强类型消息并做字段、TTL、大小校验。② Python 对所有错误 exit 0；Rust 线协议应返回带 `ProtocolError`/错误码的 JSON，同时只在安全的“允许继续”策略下映射为非阻断诊断。③ Python 无持久化/恢复；Rust 需为 heartbeat、spawn、green、close、mailbox complete 写 append-only 事件，并测试崩溃恢复与幂等。④ Python 无取消/并发；Rust 需用 `CancellationToken`、有界队列和 join 语义测试取消竞态。⑤ Python 可在未知工具上用 `event=None`；Rust 应拒绝未知 DAG 目标/越权 sibling，不能把未知 recipient 当广播。⑥ close 判据必须递归覆盖自身、全部 child 和 grandchild；任一非绿、过期 heartbeat、未完成 mailbox 或 unknown outcome 都只能保活并派生新 worker。⑦ 用集成测试验证四类通信（parent、grandparent、直接 sibling、直接 child）、重复 message_id、TTL 过期、child 崩溃后重新 claim、全绿 close 与非绿保活。

## 未决问题

无法从源文件确认 `hookify.core.config_loader.load_rules` 的规则文件语法、`RuleEngine.evaluate_rules` 的具体返回 schema、规则拒绝是否由某个 JSON 字段表达，以及 Claude Code 对 `systemMessage` 的确切消费方式；这些需要读取同目录 `hookify/core` 或宿主协议后才能确定。

# AC-013 — claude-code/plugins/hookify/hooks/stop.py

- source_id/item_id：`claude-code/plugins/hookify/hooks/stop.py` / `AC-013`
- source_path：`claude-code/plugins/hookify/hooks/stop.py`
- source_hash：`c37678c66db0d9ca0c2d4004ee073de88617c70c6def11bfd110d5610ddfb24c`
- source_bytes：`1557`
- source_lines：`59`
- coverage：已按顺序读取完整字节范围 `[0,1557)`、行范围 `L1-L59`；哈希实测与元数据一致。

## 完整行为复盘

`L1-L6` 是脚本元信息：shebang 指定 `python3`，模块文档说明它是 hookify 的 Stop hook，由 Claude Code 在 agent 想停止时调用，并读取 `.claude/hookify.*.local.md` 后评估 stop 规则。文档没有规定输入 JSON 的 schema，也没有承诺规则文件存在时的错误类型。

`L8-L10` 导入 `os`、`sys`、`json`。`L12-L19` 是模块加载阶段的路径副作用：从环境变量 `CLAUDE_PLUGIN_ROOT` 读取 `PLUGIN_ROOT`；只有值为真（空字符串视为无值）才执行路径调整。先取其父目录 `parent_dir`，再分别检查是否已在 `sys.path`，缺失时用 `sys.path.insert(0, ...)` 插入；因此父目录先插入、插件根目录后插入，且重复导入路径不重复添加。环境变量缺失时不会调整路径，之后的导入直接依赖解释器既有路径。

`L21-L27` 是导入边界。脚本尝试导入 `hookify.core.config_loader.load_rules` 与 `hookify.core.rule_engine.RuleEngine`。任何 `ImportError`（包括依赖链中的导入失败）都被捕获，构造 `{"systemMessage": "Hookify import error: <异常文本>"}`，用 `json.dumps` 写到标准输出，然后 `sys.exit(0)`。该分支不进入 `main`，没有 stderr 输出；退出码 0 明确让宿主继续其 stop 操作。

`main`（`L30-L55`）是唯一显式函数，也是脚本入口。

- `L31` 的 docstring 仅标识 Stop hook 入口。
- `L33-L35` 调用 `json.load(sys.stdin)`，同步读取直到 EOF 并解析一个 JSON 值；没有字段校验、大小限制或默认对象，合法输入可以是对象、数组、字符串、数字、布尔或 null。读阻塞、空输入、非法 JSON、编码/IO 错误都会走统一异常路径。
- `L37` 调用 `load_rules(event='stop')`，事件名固定为字符串 `stop`；规则加载的文件遍历、顺序和持久化细节不在本文件中。
- `L40-L41` 每次调用都新建 `RuleEngine()`，再以 `(rules, input_data)` 调用 `evaluate_rules`，其返回值不在此处改写。
- `L43-L44` 无论结果是否为空都执行一次 `json.dumps(result)` 并写标准输出；因此正常成功路径仍有一个 JSON 输出帧，不以空输出表示“无动作”。输出未显式 flush，也没有 stderr 日志。
- `L46-L51` 捕获所有 `Exception`，包括规则加载、引擎评估、JSON 序列化和普通 stdout 写入异常；输出 `{"systemMessage": "Hookify error: <str(e)>"}`。异常文本直接插入字符串后再由 `json.dumps` 转义。`KeyboardInterrupt`、`SystemExit` 等继承自 `BaseException` 的信号不由此捕获。
- `L53-L55` 的 `finally` 无条件 `sys.exit(0)`；成功、普通异常和导入成功后的任何控制流最终都返回 0。若错误输出本身再次触发未捕获的 `BaseException`，仍会尝试执行该 finally。没有重试、回滚或“拒绝停止”的非零退出协议。

`L58-L59` 仅在直接执行（`__name__ == '__main__'`）时调用 `main()`；被别的 Python 模块导入时只执行模块级路径调整和符号导入，不自动读取 stdin。源文件没有 `__all__`，可观察的自有函数导出为 `main`，外加模块变量 `PLUGIN_ROOT`、`parent_dir` 及导入的 `load_rules`、`RuleEngine`。

并发语义是单进程、同步、一次调用一个 `RuleEngine` 实例；本文件没有线程、锁、async、共享队列或跨调用缓存。若宿主并发启动多个进程，各进程只共享外部规则文件，文件一致性由未展示的 loader 决定。

## 状态、取消、恢复与副作用

本文件没有显式取消令牌、超时、重试、恢复点、状态机或持久化写入。`json.load` 和规则评估在调用线程中持续执行，输入阻塞不会被本文件打断。异常策略是“输出诊断但允许操作”：导入失败或运行时异常均返回 exit 0；这与把异常视为 stop veto 不同。可见副作用只有模块级 `sys.path` 修改和标准输出 JSON；规则加载可能读取 `.claude/hookify.*.local.md`，`RuleEngine.evaluate_rules` 可能产生其自身副作用，但源文件无法确认，不能假定有事务或回滚。没有子进程、网络、环境变量写回或会话日志。

## 源内测试与行为判据

源内未包含测试；同目录仅见 `__init__.py`、`hooks.json`、其他 hook 脚本，没有 `test_*.py` 或测试模块。可独立验证的判据：在设置 `CLAUDE_PLUGIN_ROOT` 且导入依赖可用时，向 stdin 输入合法 JSON，进程应输出一个可解析 JSON 且退出码为 0；输入非法 JSON 或让 `load_rules` 抛出 `Exception` 时，应输出含 `systemMessage` 且前缀为 `Hookify error:` 的 JSON、仍退出 0；令导入路径不可用时，应输出 `Hookify import error:` 并退出 0；作为模块导入时不应自动消费 stdin。

## zenpi Rust 映射

语义对应的最小单元不是一个新的 Stop hook，而是“节点状态检查 + 可寻址通信 + 可取消后台工作”的组合。项目已有 `src/dag.rs` 与要求的四类模块足以承载它：`DagNode` 的 `parent`、`children`、`status`、`worker`（`src/dag.rs:L57-L73`）直接表达会话父子关系；`DagStore::recipients` 已解析 `parent`、`grandparent`、直接 `sibling`、直接 `child` 及 `all`（`src/dag.rs:L279-L347`），`send`/`inbox` 是消息邮箱原语（`L349-L393`），`can_close` 已实现“自身及全部 descendant 为 green 才可关闭，否则返回 unfinished”（`L395-L425`），`spawn_worker` 保持子进程 stdin 打开以便后续派生工作（`L446-L496`），`send_to_worker` 是同一子 worker 的 follow-up 通道（`L518-L532`）。因此 AC-013 的 DAG 要求应复用这些原语，不另造只发给父节点的管道。

- **协议/邮箱/事件落点**：`src/protocol.rs:L81-L116` 的 `MailboxRequest`（Send/Receive/Acknowledge/Claim/Complete）可作为节点间稳定协议；`TreeAction`/`TreeRequest`（`L1161-L1234`）适合传输树视图和导航，但不能单独代替 DAG 状态。`src/session.rs:L3180-L3194` 的 `MailboxMessage` 提供 sender/recipient、digest、TTL、sequence、status、claim token、result；`LiveSessionRegistry` 的 register/heartbeat/claim/finish（`L3274-L3488`）提供最小会话父子存活租约。直接 sibling/child 可由 `DagStore::recipients` 解析成 recipient IDs，再通过 `SessionMailbox` 或 `MailboxRequest` 投递；事件流可用 `src/runtime.rs:L171-L193` 的 `RuntimeEvent::{Accepted,Started,Queued,CancelRequested,Completed,Closed}`，将 `dag_status`、`dag_message`、`dag_spawned`、`dag_close_deferred` 映射为可回放的 headless 事件。
- **headless 编排落点**：`src/headless.rs:L4466-L4625` 已将 `BackgroundRunner` 接到异步请求并周期性 heartbeat；`L4951-L5169` 把 runtime 生命周期事件按 request/turn 关联写出；`L6392-L6400`、`L8943-L9093` 已有 mailbox 路由。建议在同一 dispatch 层加入 `DagAction`（status/send/wait/close/spawn），所有回复经过现有 replay/event writer，避免把 DAG 消息塞进 provider delta。收到 `close` 时先调用 `DagStore::can_close`：可关闭才调用 `Agent::try_close`/runtime shutdown；不可关闭则保持 worker job 活跃，发送 `dag_close_deferred`（含 unfinished IDs），按需对新节点调用 `spawn_worker`。
- **core/session 落点**：`src/core.rs:L819-L914` 的 live owner 与 mailbox facade 可作为节点会话 owner API；`L922-L1001` 的 `tree_request` 展示了 session identity、Idle/Closed 检查和 cancellation 门。将 `DagNode.worker` 绑定到 `session_id`/owner epoch，新增 `Agent::dag_close_if_green`、`Agent::dag_keepalive`、`Agent::dag_spawn_child`，内部复用 `SessionStore`、`DagStore` 和 live owner，而不是在 `Agent` 内维护第二份树。`src/session.rs:L3570-L3805` 的 `SessionMailbox` 已是文件锁、bounded journal、claim token、显式完成且“claimed message never implicitly retried”的耐久通信；父、祖父、兄弟和子节点都应使用同样的 recipient 校验与 workspace 校验。
- **保活与派生的最小机制**：保活记录至少包含 `node_id`、`session_id`、`owner_epoch`、`last_seen_ms`、`lease_expiry`、`unfinished_descendants`；heartbeat 复用 `heartbeat_live_owner`，过期由 `active`/`expire_blueprint_workers` 清除。`close` 只在 `can_close == true` 且当前节点 green 时提交 durable closed 事件；否则保持 BackgroundRunner 的 active job，定时调用 `wait_for_message`（`src/dag.rs:L547-L565`）或 runtime poll，并为新 work 通过 `spawn_worker` 派生直接 child，同时在父节点 mailbox 写入 spawn/result correlation。关闭前必须 re-read locked DAG snapshot，避免并发 worker 在检查与关闭之间改变 descendant 状态。
- **runtime/tool cancellation**：`src/runtime.rs:L49-L99` 的 `CancellationToken` 是合作式取消；`L236-L365` 的 bounded command/event channel、`try_submit`/`try_cancel` 和 `RuntimeEvent` 可承载派生任务与 stop 请求，`L376-L420` 的 bounded grace 明确 detached job 不能回滚副作用。`src/tool_runtime.rs:L157-L168`、`L219-L247`、`L252-L330` 要求工具批次轮询取消并在不确定副作用后停止后续调用；DAG close-deferred 不应粗暴 cancel 掉仍需完成的 descendants。`src/approval.rs:L126-L191` 的 `ApprovalCoordinator` 是同步 worker-host 邮箱，`L409-L446` 的 `cancel_all`/`emergency_cancel` 可在 host stop 时唤醒等待审批；但它只适合审批，不应充当跨节点 DAG mailbox。
- **provider 边界**：`src/providers/**`（例如 `src/providers/mod.rs:L1-L10`、`L136-L159`）负责协议/路由/模型能力，真正的 `Backend`/`ProviderEvent` 在 `src/backend.rs:L238-L278`、`L495-L523`。DAG 节点消息、green 判定和派生属于 orchestration 层，不应写入 provider wire payload；provider streaming 事件只通过 headless 的事件邮箱关联到拥有该节点的 worker。

差异清单与可验证动作：

1. stop.py 是无状态、永远 exit 0 的单次 JSON hook；zenpi 是有 durable journal、claim/ack/result 和严格错误码的长生命周期系统。验证：对同一 `message_id` 重复 Complete，`SessionMailbox` 应拒绝冲突；对 stop.py 则只验证 JSON 输出和 0 退出。
2. stop.py 没有取消/超时；zenpi 必须让 DAG wait、provider/tool 和子 worker都观察 `CancellationToken`，并区分取消与未知副作用。验证 `runtime` 的 `CancelRequested` 后收到唯一 `Completed`，再收到 `Closed`。
3. stop.py 不检查依赖树；zenpi 的 close gate 必须覆盖全 descendant，不只是直接 child。验证构造 red grandchild 时 `can_close(root)` 返回 `false` 且 unfinished 含该节点；全部设 green 后才允许 close。
4. stop.py 不派生 worker；zenpi 派生需绑定 parent、worker/session、stdin 保活和可重试的 durable spawn 事件。验证 spawn 后 `spawned_nodes` 可见、`send_to_worker` 不重启进程，子进程退出前 parent 不报告 closed。

## 未决问题

无法从 `stop.py` 确认 `load_rules` 的文件匹配顺序、规则 schema、`RuleEngine.evaluate_rules` 的返回字段及其是否产生外部副作用；也无法确认 Claude Code 对 `systemMessage` 的具体展示或是否会因 exit 0 而跳过 stop。zenpi 映射中 DAG worker 与 session owner 的一对一约束、heartbeat TTL 的最终数值、spawn 失败后的产品级重试上限仍需由上层协议/产品规则确定；源文件本身没有这些信息。

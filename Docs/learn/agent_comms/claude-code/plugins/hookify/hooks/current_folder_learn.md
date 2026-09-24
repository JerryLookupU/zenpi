# hookify/hooks 目录级学习汇总

> 学习模式：`understand`（目录汇总）。本汇总覆盖 `__init__.py`、
> `hooks.json`、`pretooluse.py`、`posttooluse.py`、`stop.py`、
> `userpromptsubmit.py` 五份已完成的 1:1 笔记，并回看实际源码确认行为。
> 用户给出的 `Docs/learn/...` 源路径在当前工作区不存在；对应源码实际位于
> `/Users/wangweiyang/GitHub/claude-code/plugins/hookify/hooks`，逐文件笔记位于
> `zenpi/Docs/learn/agent_comms/files/claude-code/plugins/hookify/hooks/`。

## 目录职责

该目录是 hookify 的宿主适配层：`hooks.json` 把 Claude Code 的生命周期事件
注册为四个命令进程，三个 Python hook 再把 stdin 中的一次性 JSON 事件交给
`hookify.core.config_loader.load_rules` 和 `hookify.core.rule_engine.RuleEngine`。
它不保存会话、不维护 worker、不解析 DAG，也不直接执行工具；它只负责设置
Python 导入路径、把宿主事件映射成规则事件、输出一条 JSON 结果。规则本身的
格式和匹配细节属于同级 `core` 目录，而不是本目录。

## 模块清单

- `__init__.py`：空包初始化文件，没有函数、类、常量或显式导出。
- `hooks.json`：纯声明式注册表；关键配置键为 `hooks`、`PreToolUse`、
  `PostToolUse`、`Stop`、`UserPromptSubmit`，每个事件各启动一个
  `python3 ${CLAUDE_PLUGIN_ROOT}/hooks/*.py` 命令，`timeout` 均为 `10`；
  没有 matcher、重试、优先级或 session/node 目标。
- `pretooluse.py`：工具执行前的同步入口；关键导出/依赖符号为顶层
  `PLUGIN_ROOT`、`load_rules`、`RuleEngine` 和 `main()`，将 `Bash` 映射为
  `bash`、`Edit`/`Write`/`MultiEdit` 映射为 `file`，其他工具传递 `None`。
- `posttooluse.py`：工具执行后的同步入口；同样提供 `PLUGIN_ROOT`、
  `load_rules`、`RuleEngine`、`main()`，事件分类与 `pretooluse.py` 相同，
  但输入语义是工具已经完成后的事件。
- `stop.py`：agent 请求停止时的同步入口；关键符号为
  `PLUGIN_ROOT`、`load_rules`、`RuleEngine`、`main()`，固定调用
  `load_rules(event='stop')`，不自行检查依赖树或终止 worker。
- `userpromptsubmit.py`：用户提交 prompt 时的同步入口；关键符号为
  `PLUGIN_ROOT`、`load_rules`、`RuleEngine`、`main()`，固定调用
  `load_rules(event='prompt')`。

四个命令脚本均在直接执行时才调用 `main()`，被 import 时不会消费 stdin。
它们都一次性 `json.load(sys.stdin)`，创建新的 `RuleEngine`，原样调用
`evaluate_rules(rules, input_data)`，再用 `json.dumps` 向 stdout 输出一行；
即使结果为空也不会省略该行。

## 运行时数据流与控制流

1. Claude Code 读取 `hooks.json`，在 `PreToolUse`、`PostToolUse`、`Stop` 或
   `UserPromptSubmit` 事件上分别启动相应 Python 进程。命令路径依赖
   `${CLAUDE_PLUGIN_ROOT}`，配置只写出 `timeout: 10`，单位和超时动作由宿主
   解释，源文件没有定义。
2. 脚本读取 `CLAUDE_PLUGIN_ROOT`；非空时把其父目录和自身去重后插到
   `sys.path` 首位，以便导入 `hookify.core`。导入失败立即输出
   `{"systemMessage":"Hookify import error: ..."}` 并返回 0。
3. `pretooluse.py`/`posttooluse.py` 从 `input_data['tool_name']` 做精确分类：
   `Bash -> bash`，`Edit`、`Write`、`MultiEdit -> file`，未知或缺省值为
   `None`；`stop.py` 固定为 `stop`，`userpromptsubmit.py` 固定为 `prompt`。
4. `load_rules(event=...)` 发现并解析 `.claude/hookify.*.local.md`，随后
   `RuleEngine.evaluate_rules` 读取完整原始输入并返回结构化结果。规则命中、
   warning、block 的具体字段由 `core` 定义，本目录不再转换。
5. 正常结果或普通异常都写 stdout JSON；`main()` 的 `except Exception` 将错误
   转为 `systemMessage`，`finally` 无条件 `sys.exit(0)`。因此 hook 的进程码
   不能作为业务成功判据，宿主必须解析 JSON。目录内没有脚本之间的消息流，
   也没有线程、锁、异步任务、共享内存或跨调用缓存。

## 错误、取消与恢复语义

输入为空、非法 JSON、非对象导致的 `.get` 错误、规则加载失败、规则求值失败
和 JSON 序列化失败，都会尽量转成 stdout 的 `systemMessage`；导入阶段只捕获
`ImportError`，`KeyboardInterrupt`、`SystemExit` 等 `BaseException` 不在普通
异常分支内。脚本没有取消 token、deadline 轮询、重试、checkpoint、journal 或
恢复逻辑；stdin 阻塞只能由宿主终止进程，恢复只能重新启动一次全新的 hook。
`hooks.json` 的超时可能让宿主杀掉命令，但是否强杀子进程、已产生的规则副作用
能否回滚均未规定。四个脚本把运行异常也映射为 exit 0，适合 hookify 的“错误不
阻断原操作”策略，却不能直接复制到 zenpi 的 DAG 终态：`Failed`、`Cancelled`、
`Unknown` 和“允许继续”必须区分，取消也不能回滚已经发生的外部工具副作用。

## 与 zenpi Rust 的映射建议

### 可复用的通信原语和协议

本目录的最小抽象是“生命周期事件 -> 结构化消息 -> 结果事件”。zenpi 优先复用
`src/protocol.rs` 的 JSONL `StdioRequest`、`StdioResponse`、`StdioEvent`，以及
`Command::Mailbox` 的 `MailboxRequest::{Send, Receive, Acknowledge, Claim,
Complete}`。普通进度用 `StdioEvent`，需要可靠投递、领取、回执、TTL 和幂等
结果的 DAG 控制消息用 mailbox，不把 provider 文本或 stdout 重放当作 durable
控制面。消息建议定义为强类型 `DagMessage`/`WorkerMessage`，至少包含
`message_id`、`sender_node_id`、`recipient_node_id`、`relation`、`kind`、
`payload`、`correlation_id`、`generation`、`ttl_ms` 和 `idempotency_key`；服务端
根据拓扑计算关系并校验 workspace、owner epoch、大小和 TTL，不能信任客户端
自报的 `parent` 或 `sibling`。

当前 `src/dag.rs` 已有可直接复用的轻量原语：`DagNode` 的 `parent`/`children`、
`DagRelation::{Parent, Grandparent, Sibling, Child, All}`、文件锁保护的
`DagStore`、`DagMessage`、`recipients`、`send`、`claim_inbox`、`ack`、
`touch_worker`、`can_close`、`spawn_worker` 与 `send_to_worker`。它已经能让
一个 worker 与 parent、grandparent、直接 sibling、直接 child 通信，并限制
节点、消息、正文大小。更强的持久邮箱则是 `src/session.rs` 的
`SessionMailbox`/`MailboxMessage`/`MailboxStatus`/`LiveSessionRegistry`，可用
digest、sequence、TTL、claim token、`finish_claim` 和
`finish_claim_with_reply` 形成可恢复的请求-结果协议。`LiveSessionRegistry::claim_next`
只领取消息，明确不会替调用方启动 scheduler，必须由编排层显式提交 worker。

### 会话父子关系和 DAG 邻接

`src/core.rs` 的 `Turn.parent_id`、`Turn::with_parent` 表达 turn lineage；
`src/session_tree.rs` 的 `TreeEntry.parent_id`、`children`、ancestry 可重建
会话树。DAG worker 应以不可变 `node_id` 绑定 `session_id`，以
`DagNode.parent` 和 `children` 持久化直接边：parent 是一跳向上，grandparent
是两跳向上，direct child 是当前 `children`，direct sibling 是同一 parent 的
children 排除自己，grandchild 是 direct child 的下一层。关系应由
`DagStore::recipients` 或新增有界 `neighbors(node_id)` 统一解析，拒绝跨
workspace、跨树或越级目标；`relation` 仅是路由请求，不是授权凭据。若需要跨
重启恢复，应把 `dag_node_created`、`dag_edge_added`、`dag_message_*`、
`dag_status_changed` 等事件追加到 `src/session.rs` 的 journal，而不是只留在
进程内或 prompt 文本中。

### Close gate、保活与派生的最小机制

节点自身成功只产生候选 `Green`，不能直接 `close`。关闭的原子条件应是：当前
节点为 `Green`，全部 direct child 递归到的后代（至少 child 和 grandchild，
实际应覆盖任意深度）均为 `Green`，且没有未完成 mailbox claim、活动 child
lease、未决 approval/tool 操作、未知副作用结果或待处理的新工作。已有
`DagStore::can_close` 会在锁定快照上检查自身及所有 descendant，并返回
`unfinished`；建议把它提升为带原因的 `CloseDecision::{Close,KeepAlive}`，
再通过 session 事件幂等记录 close，避免检查后到写入前的并发 child 更新。

最小可行的保活/派生闭环如下：

1. 节点启动时由 `LiveSessionRegistry::register` 登记
   `session_id`、`owner_epoch`、workspace 和 `last_seen_ms`，worker 周期性
   `heartbeat`/`DagStore::touch_worker`；过期 owner 不得继续 claim、发送或
   close。
2. 每次收到 child 状态、新消息或 poll tick，都在同一受保护快照上重算后代健康。
   未全绿时保持 `Open/Running`，发送 `dag_keepalive` 或继续 `wait_for_message`，
   不把 timeout、cancel、red、缺失记录误报为 green。
3. 新工作必须先追加带 `generation`、`parent=node_id`、
   `correlation_id`/`idempotency_key` 的 `spawn_intent`；成功登记 child 和
   runner job 后追加 `spawned`。同一个幂等键只能产生一个 child，防止重连或
   崩溃恢复重复执行副作用。
4. 通过 `BackgroundRunner::try_submit` 派生有界的新 job（或在已有
   `spawn_worker` 中创建直接 child 并保持 stdin），用 mailbox 投递工作；收到
   `Started` 才认为 child 活跃，收到 `Completed`/`Failed`/`Cancelled`/
   `Panicked` 才更新 durable 状态。节点和所有后代全绿后才写
   `dag_node_closed`；否则继续保活并按配额派生新 worker。

### 四个既有 Rust 落点

- `src/headless.rs`：继续作为 stdin/stdout JSONL、request-id replay 和异步
  `StdioEvent` 的边界；在现有 mailbox dispatch（包括
  `mailbox_slash_view`/`execute_mailbox`）旁加入 DAG 的 `status`、`send`、
  `recv`、`keepalive`、`spawn`、`close` 命令。它应周期性发 heartbeat、排空
  mailbox、输出 close-deferred 原因，并对慢客户端保持有界事件缓存；不能把
  child/grandchild 进度无限堆积，也不能让 provider delta 决定 close。
- `src/core.rs`：`Agent` 在 turn admission、tool 结果落盘和 owner 生命周期
  上执行 DAG 治理；复用 `Turn.parent_id`、`heartbeat_live_owner`、
  `WorkerExecutionBinding`、approval/policy 检查，集中实现
  `close_if_green`、关系授权、`keepalive` 和幂等 `spawn_intent`。只有 core
  的状态聚合器能发布 `Green/Closed`，`RuleEngine` 或 provider 完成不能越权。
- `src/session.rs`：以 append-only `SessionStore` 记录节点、边、heartbeat、
  spawn、green、close、claim/complete 和恢复结果；复用
  `SessionMailbox` 的文件锁、digest、TTL、sequence、claim token 与
  `LiveSessionRegistry` 的 owner epoch。恢复时缺少 terminal 记录应为
  `Unknown`/需恢复，而不是默认为成功；写 journal 失败时不能先在内存中宣布
  close。
- `src/runtime.rs`：`BackgroundRunner`、`JobId`、有界 command/event 队列、
  `RuntimeEvent::{Accepted, Started, Queued, CancelRequested, Completed, Closed}`
  和 `CancellationToken` 承载每个派生 worker。`CancellationToken` 是合作式
  取消，不能回滚外部副作用；队列满、panic、detached job 或未知结果必须反馈
  给 DAG 状态机，而不是变成 Green。runtime 负责执行和生命周期，不负责自行
  计算拓扑。

`src/dag.rs` 是这四处之间现成的轻量协调层：可先复用其邻接解析、锁定快照、
`can_close` 和 `spawn_worker`，再把状态与幂等事件桥接到 `session.rs` 的 durable
mailbox。`src/providers/**` 只产生 provider 请求/流式进度；`src/tool_runtime.rs`
和 `src/approval.rs` 负责可取消工具与审批，均不能直接授权跨节点通信或宣布 DAG
关闭。

## 未决问题

1. 源配置的 `timeout: 10` 单位、stdin schema、matcher 默认行为、非零退出聚合、
   超时后的子进程清理和宿主是否重试均未由 `hooks.json` 定义；
   `systemMessage` 的最终消费语义也要由宿主协议确认。
2. `load_rules` 的文件遍历顺序、冲突优先级、规则格式边界和
   `RuleEngine.evaluate_rules` 的完整返回 schema 仍属于 `core`/宿主问题，不能
   从四个入口脚本推断。
3. zenpi 需要决定 `node_id` 与 `session_id` 是否一对一、DAG 文件邮箱与
   `SessionMailbox` 是否最终合并、消息 TTL 到期标记 `Failed`、`Waiting` 还是
   `Unknown`，以及 `Green` 是否还要求无未读消息、无 approval 和无活动工具。
4. 需要固定最大 descendant 深度、spawn 配额、heartbeat TTL、父取消是否递归、
   child 崩溃后的替代 worker 数量，并规定 close 与新消息并发到达时的线性化顺序。
5. 现有 `DagStore` 的 `open/green/red` 状态和内存 `SPAWNED` worker 表尚未完全
   等同于可重放的 `Open/Running/Green/Failed/Cancelled/Closed` 状态机；需要用
   集成测试覆盖 parent、grandparent、直接 sibling、直接 child 四种路由，任一
   grandchild 非绿时只能保活/派生，全部 Green 时只产生一次 close，并验证
   `message_id`/`correlation_id` 重放不重复外部副作用。

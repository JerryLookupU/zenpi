# hookify/core 目录级学习汇总

> 学习模式：`understand`（目录汇总）
>
> 目标目录：`zenpi/Docs/learn/agent_comms/claude-code/plugins/hookify/core/`
>
> 给定的 `/Users/wangweiyang/GitHub/Docs/learn/agent_comms/claude-code/plugins/hookify/core`
> 在当前工作区不存在；实际对应源码为
> `/Users/wangweiyang/GitHub/claude-code/plugins/hookify/core/`，逐文件笔记为
> `zenpi/Docs/learn/agent_comms/files/claude-code/plugins/hookify/core/*_learn.md`。
> 本汇总按实际源码 `__init__.py`、`config_loader.py`、`rule_engine.py` 与两份逐文件笔记核对。

## 目录职责

`hookify/core` 是 hookify 的声明式规则输入、归一化和同步判定层。它从当前工作目录
`.claude/hookify.*.local.md` 发现规则文件，解析受限 YAML-like frontmatter 与正文消息，
把旧式 `pattern` 或新式 `conditions` 归一化为 `Rule`/`Condition`，再按事件、启用状态、
工具名和字段条件做无状态求值。求值结果按 `warn`/`block` 合并为 hook 响应：命中
`block` 时，`Stop` 使用 `decision: block`，`PreToolUse`/`PostToolUse` 使用
`permissionDecision: deny`，其他事件只返回 `systemMessage`。

该目录不负责注册 hook、不执行实际工具副作用、不启动 worker，也不拥有会话树或 DAG
生命周期；`action == 'block'` 是判定数据，不等于已经取消进程。唯一明显的外部读取是
规则文件和 `transcript_path`，错误通常隔离为告警、空结果或不匹配。

## 模块清单

- `__init__.py`：0 字节的 Python 包标记文件；没有显式代码、`__all__` 或公共导出。
- `config_loader.py`：负责文件发现、frontmatter 解析和规则对象构造；关键导出为
  `Condition`、`Rule`、`extract_frontmatter`、`load_rule_file`、`load_rules`。
  `Condition.from_dict` 默认 `operator='regex_match'`；`Rule.from_dict` 支持显式
  `conditions`，也把旧式 `pattern` 按 `event` 推断为 `command`、`new_text` 或
  `content` 条件。
- `rule_engine.py`：负责对已构造规则做无状态匹配并整形响应；关键导出为
  `compile_regex`、`RuleEngine`。`compile_regex` 是大小写不敏感、容量 128 的进程内
  LRU 缓存；`RuleEngine.evaluate_rules` 负责阻断优先和事件格式分流。

## 运行时数据流与控制流

1. 调用方以可选 `event` 调用 `load_rules(event)`。加载器对当前目录的
   `.claude/hookify.*.local.md` 使用 `glob.glob`，不主动排序、去重、缓存或建立优先级。
2. 每个文件进入 `load_rule_file`，一次性读取文本并调用 `extract_frontmatter`。文本必须
   以精确的 `---` 开始并有足够分隔符，否则返回空 frontmatter 和原文；正文被 `strip`。
   解析器只支持简单顶层键、布尔值、简单列表和列表内字典，不是完整 YAML：数字、`null`、
   转义、多行标量和复杂嵌套没有完整语义。
3. `Rule.from_dict` 优先接受 `conditions` 列表；仅在没有有效条件且 `pattern` 为真时生成
   legacy 条件。默认值是 `name='unnamed'`、`enabled=True`、`event='all'`、
   `action='warn'`，正文消息去首尾空白。`load_rules` 随后筛选 `event == 'all'` 或精确
   事件，并只保留 truthy 的 `enabled` 规则。
4. 加载错误按文件隔离：读取、编码、常见解析错误和意外异常都写入 `stderr` 并跳过该文件，
   不影响其他规则。坏 frontmatter 不会变成有效的空规则。
5. 调用方将规则列表和 hook JSON 传入 `RuleEngine.evaluate_rules`。引擎读取
   `hook_event_name`、`tool_name`、`tool_input`，逐规则检查 `tool_matcher`。`'*'` 匹配任意
   工具，其他 matcher 仅按字面 `|` 分割后精确比较，不 trim、不开启大小写折叠，也不是正则。
6. 没有 conditions 的规则强制不匹配；有条件时采用全 AND 和短路。支持
   `regex_match`、`contains`、`equals`、`not_contains`、`starts_with`、`ends_with`。
   字段提取优先读取 `tool_input[field]`，再处理 `Bash.command`、`Write`/`Edit` 的
   `content`、`new_string`、`old_string`、`file_path`，以及 `MultiEdit.edits` 的拼接文本；
   非工具事件可读取 `reason`、`user_prompt` 和 transcript 文件。
7. 命中规则按输入顺序汇总消息，`action == 'block'` 的集合优先于 warning 集合。没有阻断
   但有 warning 时只返回合并后的 `systemMessage`；没有命中返回 `{}`。规则引擎不修改输入、
   规则或实例状态，实例本身可复用。

## 错误、取消与恢复语义

Python 实现没有取消 token、deadline、重试、检查点、事务、恢复日志或持久化状态。加载阶段
对多数文件错误采取“打印诊断并跳过”；`extract_frontmatter` 对非预期类型并不总兜底。
引擎中缺失字段、未知 operator 和无效正则通常变成 `False`；无效正则由 `_regex_match`
捕获 `re.error` 并写 `stderr`。transcript 找不到、无权限、IO/编码失败时告警并按空字符串
继续。畸形 `MultiEdit.edits` 或不适合字符串操作的值仍可能产生 `TypeError`。

因此在 zenpi 中必须区分三类结果：规则不匹配不是成功状态，Python 的 `block` 不是取消
动作，runtime 的 `Cancelled` 也不能伪装成 DAG `Green`。取消只停止继续接受当前代结果，
不回滚已经发生的工具、文件或外部副作用；若没有 durable close 记录，进程重启后节点应
恢复为未关闭，而不是推断为完成。

## 与 zenpi Rust 的映射建议

### 可复用通信原语：消息、邮箱、事件、协议

短消息、可靠结果和控制请求应优先复用 `src/protocol.rs` 的
`MailboxRequest::{Send, Receive, Acknowledge, Claim, Complete}` 与 `MailboxOutcome`，
沿用 `validate_identifier`、`validate_mailbox_text`、`MAX_MAILBOX_TTL_MS`、分页和文本
上限。持久投递落到 `src/session.rs` 的 `SessionMailbox`/`MailboxMessage`：已有
`sender_session_id`、`recipient_session_id`、`request_id`、`digest`、单调 `sequence`、
`MailboxStatus`、claim token、过期时间和 result，适合承载 DAG 控制与结果。

瞬时进度使用 `src/protocol.rs` 的 `StdioEvent`，在 `src/headless.rs` 的
`AsyncEventBuffer`、provider/agent mailbox 和 `PendingSteer` 上做关联、背压及暂存。
普通进度可以在有界缓冲满时丢弃，但 `admission`、`node_green`、`node_closed`、
`worker_spawned`、`keepalive`、`cancelled`、失败等控制事件必须保留，或必须能由 session
journal 重放。建议新增强类型 `DagMessage`/`DagRequest`，至少包含 `node_id`、
`sender_node_id`、`recipient_node_id`、`relation`、`kind`、`payload`、`correlation_id`、
`generation`、`idempotency_key` 和 TTL；sender 与 relation 不能由客户端自报，必须由
`core` 根据持久化边重新计算和授权。

worker 负责 DAG 节点时，允许的关系应明确为：

- `Parent`：当前节点的直接 `parent_id`。
- `Grandparent`：直接 parent 的 parent，只能是真实祖先。
- `DirectSibling`：与当前节点共享同一直接 parent 的节点，不是任意同 session 节点。
- `DirectChild`：当前节点登记的直接 child。
- `Grandchild`：当前节点 direct child 的直接 child。

越级目标、跨 workspace、过期 owner、伪造 sender、旧 generation、未登记 child 或与原始
sender 不一致的 reply 必须在 `core`/协议入口拒绝。provider 文本、warning 或普通事件
不能直接当成节点状态确认。

### 会话父子关系与 close gate

现有 `src/core.rs` 的 `Turn.parent_id`、`Turn::with_parent`、`Turn::validate` 与
`src/session_tree.rs` 的 `TreeEntry.parent_id`、`depth`、`branch_id`、children 投影，
可作为会话父子关系和祖先遍历的基础。建议增加 `DagNodeId`、`DagRelation` 以及
`DagNodeStatus::{Open, Waiting, Green, Closed, Failed, Cancelled}`；持久化直接边和状态，
祖父、sibling、grandchild 关系按 parent/children 计算，避免重复关系漂移。

close gate 必须集中在 `src/core.rs` 的 `Agent` 或新增 `DagSupervisor`，任何 worker、
parent 或 sibling 都不能自行宣布 close。节点自身执行成功只能让状态成为候选 `Green`；
只有同时满足以下条件，才追加一次幂等 durable `dag_node_closed`：

1. 当前节点自身为 `Green`。
2. 当前节点登记的全部 direct child 与 grandchild 都为 `Green`，且没有未解析的 descendant
   状态；这条规则必须覆盖“节点自身及其全部 child/grandchild 全绿才允许 close”的需求。
3. 没有未完成 descendant mailbox、活动 approval/tool 操作、未决 spawn 或 unknown
   side-effect outcome。
4. 当前 `generation`、owner epoch、workspace 和 session 校验仍有效。

任一 child/grandchild 失败、超时、未领取、取消中、generation 过期，或 gate 前收到新工作，
节点都保持 `Open`/`Waiting`，不能 close。parent、grandparent、sibling 只能发送输入、
进度和结果，不能替目标节点伪造 `Green`。

### 保活与派生的最小机制

最小可运行闭环应是：

1. 节点完成自己的工作后由 `DagSupervisor` 重新计算 close gate。若未满足或有新工作，先
   在 `src/session.rs` 追加带 `node_id`、`generation` 和幂等键的 `dag_keepalive`，再调用
   `LiveSessionRegistry::heartbeat`；保活只延续租约，不改变 Green/Closed 状态。
2. 对待处理新工作追加唯一的 `dag_spawn_requested`，其 `idempotency_key` 与 parent node、
   generation 绑定。相同幂等键只返回已有请求，不能重复工具副作用或重复派生。
3. `LiveSessionRegistry::claim_next`/`SessionMailbox::claim_next` 只领取 mailbox，不启动
   worker。supervisor 从已 claim 的 payload 构造带 node/generation 的任务，使用
   `src/runtime.rs::BackgroundRunner::spawn` 和 `try_submit`；收到 `RuntimeEvent::Started`
   后才把 child 登记为 active。
4. `RuntimeEvent::Completed` 的 `JobOutcome::{Succeeded, Failed, Cancelled, Panicked}` 回写
   child 状态并追加相应 journal 事件；只有 `Succeeded` 且通过聚合 gate 才可能成为 Green。
   `Cancelled`、`Panicked`、Rejected 或未 Started 均不能 close。`CancellationToken` 用于
   close、超时和父取消的协作式向下传播；取消期间仍需记录事实和未决副作用。

### 指定 zenpi 文件的落点

- `src/headless.rs`：扩展现有 stdin/stdout JSONL、`StdioEvent`、`AsyncEventBuffer` 和
  mailbox slash 路由，承载 DAG admission、五类关系消息、reply、keepalive、spawn、cancel
  和 terminal close；用 `request_id`/`turn_id`/`correlation_id` 关联响应，沿现有有界事件
  策略保留终态标记。
- `src/core.rs`：实现 `DagSupervisor`、关系授权、状态聚合、`close_if_green`、owner
  epoch/generation 检查和幂等 spawn；复用 `Agent` 的 `Turn` parent 校验、live owner、
  mailbox claim/finish、approval 与 operation recovery 边界。close 只能在这里由聚合状态
  决定，不能由 provider 或 worker 直接写成功。
- `src/session.rs`：在 append-only journal 中增加 `dag_node_created`、`dag_edge_added`、
  `dag_message_sent/claimed/completed`、`dag_keepalive`、`dag_spawn_requested`、
  `dag_node_green`、`dag_node_closed`、`worker_spawned` 等可重放事件；复用
  `MailboxMessage` 的 digest、sequence、TTL、claim token 与 `finish_claim`/
  `finish_claim_with_reply`。恢复时若没有 durable close，节点保持未关闭。
- `src/runtime.rs`：每个派生 worker 映射为 `BackgroundRunner` 的 `JobId`；复用 bounded
  command/event/pending 容量、`CancellationToken`、`RuntimeEvent::{Accepted, Started,
  Queued, CancelRequested, Completed, Rejected, Closed}` 和 `JobOutcome`。runtime 关闭或
  detach 只能停止接受结果，不能擦除副作用，也不能把任务判为 Green。

配套边界还应落在 `src/protocol.rs`（类型化 DAG 请求、关系和字段上限）、
`src/approval.rs`（把 spawn、工具副作用、policy digest、worker generation 和 approval
绑定）以及 `src/providers/**`（只产生 provider 请求/流，不创建边、保活或 close）。

## 未决问题

源实现不能决定完整 YAML 是否必要、规则文件是否要排序、同名规则是否去重、
`tool_matcher` 是否应大小写折叠、conditions 是否永远为 AND，以及 `action == 'block'`
最终由哪个 hook 层执行。DAG 设计还需固定以下策略：grandparent/grandchild 是否允许
跨 session；sibling 是否必须双向 ACK；消息 TTL 到期是 `Failed` 还是 `Waiting`；最大
descendant 深度、worker 数量和 spawn 配额；父取消是否递归到所有子孙；已提交但未
`Started` 的 worker 是否阻塞“全绿”；以及 close 与新消息并发时的线性化顺序。

这些问题不能从 Python 的同步规则实现推断，应通过 `src/protocol.rs` 的五类关系授权测试、
`src/session.rs` 的重启/恢复与重复 complete 测试、`src/runtime.rs` 的重复 spawn/取消/关闭
测试，以及“root→child→grandchild 未全 Green 时 close 被拒绝、收到新工作保持保活并只派生
一个新 generation、全 Green 后只产生一次 durable close”场景测试固化。

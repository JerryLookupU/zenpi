# AC-010 — claude-code/plugins/hookify/hooks/hooks.json

- source_id/item_id：`claude-code/plugins/hookify/hooks/hooks.json` / `AC-010`
- source_path：`claude-code/plugins/hookify/hooks/hooks.json`
- source_hash：`838dd2ffbe325307cc23a02ef541fd45c25fd42492d64ccbe9ec11233a3a04f0`
- source_bytes：`1020`
- source_lines：`49`
- coverage：已按顺序读完字节 `0–1019`（完整 1020 字节），行范围 `L1-L49`；未跳过注释或字段（本文件实际无注释）。

## 完整行为复盘

这是一个纯声明式 JSON 配置，没有函数、方法、类或可执行导出符号；因此“逐函数/逐导出符号”的结果是：源内没有可调用函数，也没有 `export`/模块导出。唯一的行为由顶层对象和四个事件导出项定义。

- 顶层对象 `L1-L49`：`description`（L2）是说明文本；`hooks`（L3）是事件名到数组的映射。JSON 解析输入必须是对象，输出是供 Hookify/Claude Code 宿主读取的配置；源文件没有版本字段、全局默认值、matcher、优先级或显式并发策略。
- `PreToolUse`（L4-L14）：数组中一个配置块（L5-L13），其 `hooks` 数组只有一个命令 Hook（L6-L12）。`type` 为 `command`（L8），`command` 为 `python3 ${CLAUDE_PLUGIN_ROOT}/hooks/pretooluse.py`（L9），`timeout` 为数值 `10`（L10）。它声明工具调用前触发；没有 matcher 字段，是否匹配所有工具由宿主规范决定，文件本身没有额外筛选。输入是宿主提供的 PreToolUse 事件上下文（通常通过环境或 stdin，源文件未规定）；输出、退出码和拒绝格式由 `pretooluse.py` 与宿主约定决定。
- `PostToolUse`（L15-L25）：结构与前者相同（L16-L23）；命令是 `python3 ${CLAUDE_PLUGIN_ROOT}/hooks/posttooluse.py`（L19-L21），超时仍为 `10`（L22）。触发点是工具调用之后；配置未定义结果如何回传、失败是否阻止后续流程或是否重试。
- `Stop`（L26-L36）：唯一 Hook 位于 L27-L34，`type=command`（L30），命令 `python3 ${CLAUDE_PLUGIN_ROOT}/hooks/stop.py`（L31），`timeout=10`（L32）。它声明停止生命周期触发；没有输入字段、输出字段或状态持久化字段。
- `UserPromptSubmit`（L37-L47）：唯一 Hook 位于 L38-L45，命令 `python3 ${CLAUDE_PLUGIN_ROOT}/hooks/userpromptsubmit.py`（L42），`timeout=10`（L43）。它声明用户提交提示时触发；同样没有 matcher、重试、并发或错误映射配置。

四个命令字符串都依赖宿主展开 `${CLAUDE_PLUGIN_ROOT}`；若变量缺失，最终路径和错误行为不由本 JSON 兜底。`timeout: 10` 是每个命令条目的唯一边界值；本文件未声明单位（应由宿主 Hook 契约解释，不能仅凭源文件断言秒数）。数组形状允许宿主按顺序处理多个 Hook，但当前每个事件都恰好一个配置块、一个命令；是否串行、超时后杀进程、非零退出如何聚合，均不在源内确定。没有显式错误路径：JSON 结构错误、命令不可执行、脚本非零退出和超时都只能由宿主实现处理。

## 状态、取消、恢复与副作用

源文件没有状态机、取消令牌、超时回调、重试次数、恢复点或持久化声明。唯一“取消”边界是宿主可能依据 `timeout=10` 终止或放弃命令，但是否强杀、是否等待子进程、是否允许脚本继续产生副作用，源内未说明。四个 Hook 都启动外部 Python 进程，因而可能读写 `.local.md`、环境变量、会话或工具上下文；这些副作用属于脚本与宿主，不是 JSON 本身的持久化能力。配置不记录成功/失败结果，也不保证幂等；重复触发可能重复执行脚本。恢复只能由宿主重新读取配置并重新触发事件，源内没有 checkpoint 或 replay 语义。

## 源内测试与行为判据

源文件或同目录未包含测试。可独立验证的判据是：

1. JSON 解析成功且顶层仅需包含说明和 `hooks` 映射；共有四个键 `PreToolUse`、`PostToolUse`、`Stop`、`UserPromptSubmit`。
2. 每个事件数组长度为 1，块内 `hooks` 长度为 1，Hook 的 `type` 为 `command`，`timeout` 均为 `10`。
3. 四个命令分别以 `pretooluse.py`、`posttooluse.py`、`stop.py`、`userpromptsubmit.py` 结尾，并保留 `${CLAUDE_PLUGIN_ROOT}` 占位符。
4. 用宿主的配置加载器触发四类事件时，应观察到对应脚本各启动一次；设置脚本阻塞超过宿主约定的 10 单位后，应得到宿主定义的超时结果。非零退出、变量缺失和重复事件的处理应作为宿主集成测试判据，而不能归因于本 JSON。

## zenpi Rust 映射

**可复用的通信原语。** 这份配置把“生命周期事件→命令进程”作为最小 Hook 协议。zenpi 可复用 `src/protocol.rs` 的 JSONL 帧与 `Command::Mailbox`（`MailboxRequest` 的 `Send/Receive/Acknowledge/Claim/Complete` 在 `src/protocol.rs:L87-L113`、校验在 `L544-L598`，命令分派在 `L479-L484`、`L211-L263`），把 Hook 事件建模为带 `request_id/session_id/node_id` 的消息；用 `StdioEvent::new`（`src/protocol.rs:L882-L918`）发布异步事件。持久化消息应落到 `src/session.rs` 的 `SessionMailbox`/`MailboxMessage`（`L3171-L3265`），其 `Queued→Acknowledged→Claimed→Succeeded|Failed` 状态、digest、TTL 和 claim token 正好提供邮箱、去重和结果回传。`LiveSessionRegistry` 的注册、heartbeat、TTL 过期和 `claim_next`（`src/session.rs:L3274-L3409`）可作为活跃 worker 的最小保活门槛。

**会话父子关系与 DAG 邻接。** 现有 `Turn.parent_id`（`src/core.rs:L140-L203`）和 `SessionTree` 的 `TreeEntry.parent_id/depth/branch_id`（`src/session_tree.rs:L48-L84`、规划在 `L316-L362`）能表达父链及分支，但公开 API 主要提供 ancestry/page，尚未直接返回“直接 child ID 列表”或“直接 sibling 列表”。建议新增受限的 `DagNodeId`、`DagRelation { parent, grandparent, direct_siblings, direct_children }` 与 `DagIndex`（可放在 `src/session_tree.rs` 扩展或新 `src/dag.rs`）：以 `TreeEntry.parent_id` 建父子索引，同父节点集合求直接 sibling，沿父链一步得到 grandparent；每次查询执行数量/深度/字节上限。每个 DAG worker 以独立 `session_id` 或节点 ID 作为邮箱收件人，发送到 parent、grandparent、每个直接 sibling、每个直接 child 都走 `MailboxRequest::Send`，结果以 `Complete` 或 `finish_claim_with_reply`（`src/session.rs:L3490-L3560`）关联原 digest，禁止把 provider 输出直接当控制消息。

**保活与派生的最小机制。** (1) 节点启动时在 `LiveSessionRegistry` 注册 `(session_id, owner_epoch, workspace, last_seen_ms)`，周期性 `heartbeat`；邮箱 `ttl_ms` 和 owner epoch 防止旧 worker 继续领取。 (2) 用 `BackgroundRunner`（`src/runtime.rs:L236-L315`）承载节点工作：`try_submit` 受有界队列约束，`RuntimeEvent::Accepted/Started/Completed/Closed`（`L171-L193`）提供可核对生命周期；`CancellationToken`（`L49-L99`）用于合作式取消，不能撤销已经发生的外部副作用。 (3) 在 `src/core.rs` 的 worker 治理入口使用 `set_worker_budget_limits`、`renew_blueprint_worker`、`settle_blueprint_worker`（`L2812-L2843`）和 `cancel_blueprint_worker`（`L2870-L2885`）：只有 host 观察到终态并回收 owned children 后才 settle。 (4) 为每个节点持久化 `DagStatus` 事件/记录；`close_eligible(node)` 必须同时检查节点自身及其全部 child/grandchild 的状态为 `Green`、无未完成 mailbox claim、无活动子 lease。若任一后代未绿，节点保持 `Running/保活`，调用 `renew_blueprint_worker`，并把新工作作为带 `parent=node` 的新 `try_submit` 子 worker 派生；派生动作写入 session event，避免隐式重试。子 worker 失败、过期或取消时向父节点发送失败消息，父节点不得 close。

**既有模块落点。** `src/headless.rs:L2065-L2119` 的 `mailbox_slash_view` 已把 slash mailbox 动作映射为协议并调用共享 mailbox owner；`run_stdio_owned`/`run_async_streams`（`L792-L812`）和其按请求隔离的事件缓冲（`L833-L840`）可承载 Hook/DAG 事件而不混淆 queued replacement。`src/session.rs` 负责 JSONL journal、邮箱 sidecar、树快照/迁移/选择（`L861-L1033`），应新增 DAG 节点关系与状态的 durable event；写入失败时保持内存投影不变。`src/core.rs` 的 `Agent::submit/process/run_active_turn_cancelable`（`submit` 约在 `L3242-L3425`，运行在 `L3662-L3694`）适合作为节点工作入口，在提交前检查父会话与 lease，在终态前汇总后代。`src/tool_runtime.rs:L157-L235` 的 bounded batch、顺序/并发选择和取消轮询可复用于一个节点内部的工具工作；必须把“子节点未绿”视为治理状态，不把普通工具成功误判为 DAG close。`src/approval.rs` 的 `ApprovalCoordinator`（请求等待、`cancel_all`/`emergency_cancel` 约在 `L126-L250`、`L409-L445`）适合为会触发外部副作用的 Hook/工具消息提供显式 approval；持久化失败应 fail-closed。`src/providers/**`（`providers/mod.rs:L136-L160` 的 `ProviderDefinition`/协议路由及 `providers/connection.rs` 的 endpoint/header 校验）只负责模型传输能力，不应承载 parent/sibling/child 控制通信；DAG 控制面应留在 `protocol`、`session`、`runtime` 与 `core`，provider 调用仅是节点工作的一种可取消任务。

**差异清单（可执行、可验证）。**

- 当前 JSON Hook 只有四个全局生命周期事件，没有 session/node 目标；新增协议字段并为未知关系拒绝发送。
- 当前 `SessionTree` 没有直接 child/sibling 查询；增加有界邻接索引及测试：构造一父两子一孙，验证 parent、grandparent、siblings、children 集合准确且越界报错。
- 当前 mailbox 已有 TTL/claim/result，但 `LiveSessionRegistry::claim_next` 明确“不启动 scheduler”；由 headless/core 显式 claim 后 `BackgroundRunner::try_submit`，测试断言重复 digest 不重复执行。
- 当前 worker settle 要求 children 已 reaped；实现 `close_eligible` 聚合检查，测试覆盖一个孙节点失败时父节点续租并派生新 worker，全部 `Green` 后才产生 close/settle 事件。
- 当前 provider 与控制协议分离；验证 provider 超时/取消只产生节点失败或取消消息，不改变邮箱状态机的幂等和审计记录。

## 未决问题

1. `hooks.json` 未定义 `timeout` 单位、stdin/环境输入格式、matcher 默认值、非零退出语义、重试与并发顺序；必须查 Hookify/Claude Code 宿主协议或实际运行器确认。
2. 仅凭本文件无法确认四个 Python 脚本是否幂等、是否写 journal、是否会派生子进程，以及超时后的外部副作用是否可回滚。
3. zenpi 现有 `SessionTree` 的 parent 链不等同于 DAG worker 的 session 拓扑；需要产品层确认节点 ID 与 `session_id` 的一对一关系，以及“全部 child/grandchild 全绿”是否包含已过期、已取消或已归档节点。

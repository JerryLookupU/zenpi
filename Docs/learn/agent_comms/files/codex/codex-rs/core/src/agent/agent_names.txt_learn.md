# AC-015 — codex/codex-rs/core/src/agent/agent_names.txt

- source_id：`codex/codex-rs/core/src/agent/agent_names.txt`
- item_id：`AC-015`
- source_path：`codex/codex-rs/core/src/agent/agent_names.txt`
- source_hash：`f6ae12de892a2543bd109cd4da292e98f41c58ce29f0756f5e1eb8ef477e75f9`（已用 SHA-256 核对）
- source_bytes：`766`
- source_lines：`101`
- coverage：已按顺序读取字节 `0-765`、行 `L1-L101`，包括全部换行、无注释/空白行遗漏。

## 完整行为复盘

该文件是纯文本名称表，不包含函数、宏、`struct`、`enum`、trait、常量定义或 Rust 导出符号；因此没有可逐函数调用的输入输出、返回值或异常分支。每一行是一个独立的、非空的 ASCII 名称记录，按文件顺序消费时应保留行序并以换行作为记录边界。顺序内容如下：

- `L1-L10`：`Euclid`、`Archimedes`、`Ptolemy`、`Hypatia`、`Avicenna`、`Averroes`、`Aquinas`、`Copernicus`、`Kepler`、`Galileo`。
- `L11-L20`：`Bacon`、`Descartes`、`Pascal`、`Fermat`、`Huygens`、`Leibniz`、`Newton`、`Halley`、`Euler`、`Lagrange`。
- `L21-L30`：`Laplace`、`Volta`、`Gauss`、`Ampere`、`Faraday`、`Darwin`、`Lovelace`、`Boole`、`Pasteur`、`Maxwell`。
- `L31-L40`：`Mendel`、`Curie`、`Planck`、`Tesla`、`Poincare`、`Noether`、`Hilbert`、`Einstein`、`Raman`、`Bohr`。
- `L41-L50`：`Turing`、`Hubble`、`Feynman`、`Franklin`、`McClintock`、`Meitner`、`Herschel`、`Linnaeus`、`Wegener`、`Chandrasekhar`。
- `L51-L60`：`Sagan`、`Goodall`、`Carson`、`Carver`、`Socrates`、`Plato`、`Aristotle`、`Epicurus`、`Cicero`、`Confucius`。
- `L61-L70`：`Mencius`、`Zeno`、`Locke`、`Hume`、`Kant`、`Hegel`、`Kierkegaard`、`Mill`、`Nietzsche`、`Peirce`。
- `L71-L80`：`James`、`Dewey`、`Russell`、`Popper`、`Sartre`、`Beauvoir`、`Arendt`、`Rawls`、`Singer`、`Anscombe`。
- `L81-L90`：`Parfit`、`Kuhn`、`Boyle`、`Hooke`、`Harvey`、`Dalton`、`Ohm`、`Helmholtz`、`Gibbs`、`Lorentz`。
- `L91-L101`：`Schrodinger`、`Heisenberg`、`Pauli`、`Dirac`、`Bernoulli`、`Godel`、`Nash`、`Banach`、`Ramanujan`、`Erdos`、`Jason`。

输入边界是读取器传入的文件字节；输出是 101 个字符串（若实现 `read_to_string().lines()`，末尾换行不会产生额外空记录）。源文件没有默认值、参数校验、重复名去重、排序、随机选择或错误编码策略；空文件、空行、非 UTF-8、重复项、超长行如何处理均由调用方决定。文件本身没有并发语义；多个 worker 同时读取只产生独立快照，若把名称当作身份则必须由上层做原子分配，否则会重复占用。

## 状态、取消、恢复与副作用

源文件是静态只读数据，没有状态机、取消、超时、重试、持久化写入或外部副作用。读失败、解析失败、名称耗尽、重复分配不会在此文件中表达错误。将名称分配给 DAG worker 后，取消/恢复语义必须由 `zenpi` 的运行时和会话日志承载；名称只能作为稳定的人类可读标签，不能代替 `JobId`、`session_id`、`request_id` 或消息 digest。恢复时应从会话记录重放“节点 ID→名称”的绑定，避免因列表索引变化导致身份漂移。

## 源内测试与行为判据

源内未包含测试。可独立验证的判据：文件字节数为 `766`，行数为 `101`，SHA-256 等于元信息中的 `source_hash`；按 LF 分割得到 101 个非空、大小写敏感字符串，首项为 `Euclid`（`L1`），末项为 `Jason`（`L101`），且顺序与上述分组完全一致。任何读取器还应验证不存在隐含的函数语义：只做顺序枚举，不自动排序或生成新名称。

## zenpi Rust 映射

**可复用通信原语。** `src/protocol.rs` 已有 `MailboxRequest::{Send,Receive,Acknowledge,Claim,Complete}`（定义见 `L91-L113`）和 `Command::Mailbox`、`Command::Handoff`（`L214-L263`），适合 DAG 节点间可靠消息；`StdioEvent` 用 `sequence/request_id/turn_id` 封装异步事件（`src/protocol.rs:L882-L918`）。`src/session.rs` 的 `SessionMailbox` 是同工作区、加锁、追加式持久邮箱，`MailboxMessage` 带 sender/recipient、digest、TTL、状态和 claim token（`src/session.rs:L3180-L3242`、`L3570-L3615`）；可直接承载 parent、grandparent、直接 sibling、直接 child 的定址通信。进程内低延迟路径复用 `src/runtime.rs` 的有界 `mpsc::sync_channel` 与 `RuntimeEvent`（`L171-L193`），节点事件可转成 `AgentEvent::{Handoff,Provider,ToolResult,Error,Warning}`（`src/core.rs:L281-L325`）。

**会话父子关系。** 现有 `SessionHeader` 只有 `session_id/version/created_at_ms/cwd`（`src/session.rs:L35-L41`），没有 parent 字段。建议在会话记录或独立 DAG 元数据中增加 `NodeRelation { node_id, session_id, parent_id: Option<NodeId>, children: BTreeSet<NodeId>, name: String }`；`grandparent` 通过 `parent.parent_id` 解析，sibling 是 `parent.children - {self}`，直接 child 是 `children`。发送权限沿用 `SessionMailbox::check_access` 的“同 workspace 且 addressed session”约束（`src/session.rs:L3598-L3614`），不要允许任意路径冒充关系。

**保活与派生的最小机制。** 在 `src/runtime.rs` 的 job 输入中携带 `NodeId`、代际关系和 `CancellationToken`；`BackgroundRunner::spawn/try_submit/try_cancel`（`L272-L325`）负责启动、非阻塞提交和取消，`RuntimeEvent::Completed/Closed`（`L184-L193`）驱动状态更新。为满足“节点自身及全部 child/grandchild 全绿才 close”，在 `src/core.rs` 增加 `DagNodeState { self_green: bool, descendants: BTreeMap<NodeId, Health>, close_requested: bool, generation: u64 }` 与 `can_close()`：仅当自身和递归后代均为 Green 且没有未确认 mailbox 才允许 `Agent::close`；否则保持 Alive，发送 heartbeat/status 给 parent，并通过 `try_submit` 派生新 worker。派生必须生成新 `NodeId`/`JobId`、继承 parent 链和取消 token 的父级观察关系，使用幂等 `request_id` 防止重试重复建点；队列满返回 `SubmitError::QueueFull` 时保活并退避，runtime closed 则记录失败而不是假装完成。

**模块落点。**

- `src/headless.rs`：在现有 mailbox/异步事件处理（如 mailbox 响应和 runtime 事件分发）处增加 DAG 命令入口、心跳事件和 close 门闩；把 `StdioEvent` 作为外部可观测进度流。
- `src/core.rs`：新增 `DagNodeId`、`AgentName`（从 101 行白名单加载）、`DagNodeState`、`spawn_child_worker`、`record_child_health`、`can_close`；把 `AgentEvent::Handoff` 用于关系变更，把 `AgentPhase::Closed` 仅在门闩通过后设置（现有阶段定义见 `L275-L279`）。
- `src/session.rs`：持久化关系、名称绑定、heartbeat/health、派生和 close 记录；复用 `SessionMailbox` 的 claim/complete/TTL/过期规则（`L3640-L3697`、`L3743-L3790`），实现崩溃恢复和幂等重放。
- `src/tool_runtime.rs`：若 child 由工具调用产生，在 `MasterSessionCommand`/批处理结果旁携带 `NodeId`，将工具取消映射到 `CancellationToken`；不得把名称文本当作授权。
- `src/runtime.rs`：使用有界命令/事件队列、`CancellationToken`（`L49-L99`）和 `RuntimeEvent` 实现保活轮询、取消传播、派生和有界关闭；保留 `Panicked`、`Cancelled`、`Rejected` 的可观测终态。
- `src/protocol.rs`：扩展 `MailboxRequest` 或新增 `Dag` 命令，字段至少包括 `node_id/parent_id/target_id/generation/health/request_id`，复用现有长度、TTL、ID 校验和版本兼容规则；事件中带 `sequence` 便于断线恢复。
- `src/approval.rs`：child 的工具副作用仍走 `ApprovalCoordinator`，将节点身份写入 approval request 的关联上下文；禁止因“全绿”跳过审批。
- `src/providers/**`：provider 只负责本节点模型/工具执行和取消检查；发送 DAG 消息、判定后代健康、派生 worker 留在 core/runtime/session，避免 provider 产生跨会话副作用。

**差异清单与验证。** 当前名称表没有唯一 ID、父子图、健康状态、心跳或耗尽策略；当前 `SessionHeader` 没有关系字段，当前 `Agent::close` 可直接进入 `Closed`（`src/core.rs:L5737-L5751`）。实现后应添加：加载 101 名称并拒绝越界；同一 `NodeId` 重启后名称不变；parent↔child、grandparent、直接 sibling 的 mailbox 往返测试；一个 descendant 为 Failed/Expired/Running 时 `can_close()==false`；全部递归节点 Green 且邮箱已完成时只关闭一次；取消与派生竞态下不重复提交；队列满、runtime 关闭、TTL 过期和崩溃恢复均产生可重放记录。

## 未决问题

无法从源文件确认：名称是否必须全局唯一、是否允许循环复用、非 UTF-8/空行的读取策略、名称耗尽时的产品行为，以及名称与真实 session 的授权绑定方式。这些必须由 zenpi 的 DAG 元数据协议和生命周期策略明确。

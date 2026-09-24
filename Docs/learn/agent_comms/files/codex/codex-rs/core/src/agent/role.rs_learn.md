# AC-023 — codex/codex-rs/core/src/agent/role.rs

- source_id/item_id：`AC-023` / `AC-023`
- source_path：`codex/codex-rs/core/src/agent/role.rs`
- source_hash：`4fffb32e3f9c39570ba19425568bb0b727f45076610e380e26c096ec377d899b`
- source_bytes：`18139`
- source_lines：`423`
- coverage：已从字节 `1-18139`、行 `L1-L423` 按顺序读取，包含模块注释、注释掉的 `awaiter` 声明、测试模块导出路径。

## 完整行为复盘

模块总览：文件把 agent role 作为配置层叠加到已有 session 配置上；角色在 spawn 时解析，内置角色来自嵌入 TOML，用户角色来自文件。模块只负责解析、合并、工具描述，不负责何时派生子 agent 或选择编排角色（L1-L7）。依赖 `Config`、`AgentRoleConfig`、`ConfigLayerStack`、`ConfigLayerSource::SessionFlags`、TOML 解析和 `LazyLock`（L9-L24）。

- `DEFAULT_ROLE_NAME` 是公开常量，值为 `"default"`；省略 `agent_type` 时使用它。`AGENT_TYPE_UNAVAILABLE_ERROR` 是内部统一错误文本（L26-L28）。
- `apply_role_to_config(config: &mut Config, role_name: Option<&str>) -> Result<(), String>` 是主要导出入口（L30-L54）。`None` 归一化为 `default`；先用 `resolve_role_config` 查用户配置再查内置配置，找不到立即返回 `unknown agent_type '...'`，不改变 config。找到后克隆 `AgentRoleConfig`，调用异步 inner；任意读取、TOML、路径解析或重建错误都会记录 `tracing::warn!` 并折叠为 `agent type is currently not available`。`&mut Config` 表示调用方独占状态；函数本身不创建线程，也没有重试或取消参数。
- `apply_role_to_config_inner` 是异步私有函数（L56-L76）。`is_built_in` 由 `config.agent_roles.contains_key` 的反值判定；没有 `config_file` 的角色（当前 `default`、`worker`）直接成功返回，配置保持不变（L61-L64）。有文件则经 `load_role_layer_toml` 得到 TOML，计算 `(preserve_current_profile, preserve_current_provider)`，再用 `reload::build_next_config` 一次性构造新 `Config`，成功后才通过 `*config = ...` 替换，因而失败不会留下半合并状态（L65-L75）。
- `load_role_layer_toml(config, config_file, is_built_in, role_name) -> anyhow::Result<TomlValue>`（L78-L110）：内置路径先从 `built_in::config_file_contents` 取嵌入字符串，缺失报 `No corresponding config content`，以 `config.codex_home` 为相对基址并直接 `toml::from_str`；用户路径用 `tokio::fs::read_to_string` 异步读取，父目录不存在时报同样文本，再由 `parse_agent_role_file_contents(..., Some(role_name))` 解析并取 `.config`（L84-L103）。两种来源都先用 `deserialize_config_toml_with_base` 做 schema/基址校验，再用 `resolve_relative_paths_in_config_toml` 展开相对路径（L105-L109）。文件 I/O 是唯一显式异步点；没有锁、超时或取消 token。
- `resolve_role_config(config, role_name) -> Option<&AgentRoleConfig>` 为 crate 内可见导出（L112-L120）。查找顺序固定为用户 `config.agent_roles` 优先、内置 `built_in::configs()` 兜底；同名用户角色遮蔽内置角色，返回借用而非复制。
- `preservation_policy(config, role_layer_toml) -> (bool, bool)`（L122-L141）先检查角色顶层是否含 `model_provider` 或 `profile`；再检查当前 active profile 对应的角色 `[profiles.<active>]` 表是否写入 `model_provider`。只有角色未选择 provider/profile 时才保留当前 profile；只有同时未改 active profile provider 时才保留当前 provider。缺 active profile、profiles 不是 table、条目不是 table 时按 `false` 处理，不报错。

`reload` 私有模块（L143-L263）负责保留层栈并把 role 放到 session flags 优先级：

- `build_next_config(config, role_layer_toml, preserve_current_profile, preserve_current_provider) -> anyhow::Result<Config>`（L146-L171）在保留 profile 时提取当前 profile 名；先 `build_config_layer_stack`，再反序列化 effective config。保留 profile 时把合并结果的 `profile` 清空，随后用 `Config::load_config_with_layer_stack` 传入 `reload_overrides`、原 `codex_home` 和新层栈；最后恢复 `active_profile`。所有步骤成功后才返回新配置。
- `build_config_layer_stack`（L174-L191）复制 `existing_layers`，必要时插入解析后的 active profile 层，再插入角色层，并复用原 `requirements` 与 `requirements_toml` 创建 `ConfigLayerStack`。角色层来源标记为 `SessionFlags`。
- `resolved_profile_layer`（L193-L219）没有 active profile 时返回 `None`；有时先临时插入角色层、反序列化 effective config、调用 `get_config_profile` 解析角色影响后的 profile，再把该 profile 序列化成一个 `SessionFlags` 层。这样在随后重建时仍能保持角色覆盖后的 profile 字段。
- `deserialize_effective_config`（L221-L229）以 `config_layer_stack.effective_config()` 和 `config.codex_home` 为基址调用 `deserialize_config_toml_with_base`；错误原样上抛。
- `existing_layers`（L231-L241）按 `LowestPrecedenceFirst` 读取并包含 disabled 层，克隆为 `Vec<ConfigLayerEntry>`，保留原栈所有来源。
- `insert_layer`（L243-L247）按 `layer.name` 的 `partition_point` 插入，保证同名层插在已有同名层之后；结合低到高顺序使后插入的 role/session 层对相同键拥有高优先级。
- `role_layer`（L249-L251）把任意 `TomlValue` 包成 `ConfigLayerEntry::new(ConfigLayerSource::SessionFlags, ...)`。
- `reload_overrides`（L253-L261）始终保留 `cwd`、Linux sandbox 可执行文件、`main_execve_wrapper_exe`、`js_repl_node_path`；仅当策略允许时覆盖 `model_provider`，否则置为 `None` 让层栈决定。其余 `ConfigOverrides` 使用默认值。

- `spawn_tool_spec::build(user_defined_agent_roles) -> String` 为 crate 内公开模块函数（L265-L272）：取得缓存内置 map，与用户 map 交给 `build_from_configs`，返回 spawn-agent 工具的说明文本。
- `build_from_configs`（L274-L296）用 `BTreeSet` 去重；先遍历用户角色，再遍历内置角色，因此用户同名定义优先、用户角色整体排在内置角色前。每项经 `format_role`，最终前缀明确说明省略类型名时使用 `default`。
- `format_role(name, declaration) -> String`（L298-L339）有 description 时尝试读取声明的 `config_file`：内置优先嵌入内容，随后同步 `std::fs::read_to_string` 用户文件；TOML 读取/解析失败被 `ok()` 吞掉。若 TOML 有 `model`、`model_reasoning_effort`，在描述后追加不可更改提示，四种组合分别生成双字段、单字段或空提示；无 description 输出 `name: no description`。这是展示逻辑，不改变配置。
- `built_in::configs() -> &'static BTreeMap<String, AgentRoleConfig>`（L342-L407）用 `LazyLock` 单次初始化缓存，包含 `default`（description `Default agent.`、无 config_file）、`explorer`（描述含并行探索和复用 explorer 规则、`explorer.toml`）、`worker`（执行/生产工作、明确 ownership 且不得回滚他人改动、无 config_file）。`awaiter` 块被注释并标明临时移除（L386-L404），因此运行时不可见。惰性静态读取线程安全且无运行时 I/O。
- `built_in::config_file_contents(path: &Path) -> Option<&'static str>`（L409-L418）把 `explorer.toml`、`awaiter.toml` 映射到 `include_str!` 内容；其他路径、非 UTF-8 路径或不存在映射返回 `None`。即使 awaiter 角色声明被注释，内容映射仍保留。
- `#[cfg(test)] mod tests` 通过 `role_tests.rs` 路径挂载（L421-L423），源文件本身没有测试函数体。

## 状态、取消、恢复与副作用

状态转换是“读角色声明 → 构造临时层栈 → 反序列化 → 成功后替换 `Config`”；无角色文件的内置角色是 no-op。未知角色与配置加载失败分别是稳定的 `unknown agent_type` 与 `agent type is currently not available`。异步用户文件读取使用 Tokio，但没有 `CancellationToken`、超时、指数退避或自动重试；调用方取消 future 只会停止等待，不保证底层文件 I/O 的回滚。TOML 解析、相对路径展开、profile/provider 冲突均在替换前失败，因此不会持久化半成品。

模块不直接写磁盘持久化 session，也不触发 provider、工具、网络或子进程；`tokio::fs::read_to_string` 和 `std::fs::read_to_string` 是只读外部副作用。`reload_overrides` 保留 cwd、sandbox wrapper 与 JS 路径，避免重载丢失宿主运行时绑定。`LazyLock` 的 built-in map 是进程内只读缓存。并发上，`apply_role_to_config` 需要 `&mut Config`，同一配置实例不能并行修改；`spawn_tool_spec` 的 map 只读遍历可并行调用；`built_in::configs` 的惰性初始化由标准库同步保证。

## 源内测试与行为判据

`role_tests.rs` 是同目录测试入口（role.rs L421-L423）。关键判据如下：

- 默认角色 no-op、未知角色报精确错误（`role_tests.rs` L48-L69）；缺失用户文件和非法 TOML 都归一为 unavailable（L86-L123）。
- 用户 role 文件中的 `name`、`description`、`nickname_candidates`、`developer_instructions` 等 agent 元数据不会改变解析边界，但 `model` 会生效（L125-L154）；未指定键、sandbox wrapper 保留（L156-L194）。
- active profile/provider 在 role 未声明接管时保持（L196-L246）；顶层 `profile`/模型设置可覆盖保留 profile（L248-L305）；声明 role profile 或顶层 `model_provider` 时切换 provider（L307-L424）；修改 active profile 内 `model_provider` 时也切换且保留 profile（L426-L489）。
- workspace-write role 不会把默认 sandbox 字段物化到 role 层，但既有 `network_access` 保持（L491-L559）；role session-flags 层对同键 CLI 层有优先级（L561-L590）。
- skills 配置可使 spawned agent 禁用指定 skill（L592-L641）。spawn 工具描述测试验证用户角色先列、同名去重、无描述文本，以及 model/reasoning effort 的 locked 提示（L643-L733）；内置文件解析未知路径返回 `None`（L735-L741）。`explorer` 应用测试当前 `#[ignore]`（L71-L84），因此不能把它视为默认 CI 判据。

可独立验证：运行 core crate 的 `cargo test role` 或只执行 `role_tests`；检查 `ConfigLayerSource::SessionFlags` 数量增加一层、`config.model_provider_id`/`active_profile` 与上述 role 声明一致、错误字符串精确匹配，并对同名用户/内置角色检查描述去重和顺序。

## zenpi Rust 映射

语义对应：`role.rs` 的“高优先级、可复用配置层”可转成 zenpi 的“worker 角色/能力声明层”，但 DAG 通信与生命周期必须落在现有 session/runtime，而不是塞进 role 文本解析。zenpi 已有可复用原语：

1. **消息/邮箱**：`protocol::MailboxRequest` 提供 `Send/Receive/Acknowledge/Claim/Complete`，并限制行、文本、TTL、ID（`src/protocol.rs` L20-L35、L81-L113）；`session::MailboxMessage` 带 sender、recipient、request_id、digest、sequence、状态、claim token、结果（`src/session.rs` L3180-L3255）。`SessionMailbox` 是同 workspace、按 inode 锁和追加 journal 的本地寻址通道，明确“不启动 daemon、不隐式重试”（`src/session.rs` L3570-L3614）。因此 parent、grandparent、直接 sibling、直接 child 都可通过 session ID 定向发送；回复用原 digest 的 `Complete/Fail`，避免 request ID 冲突。
2. **事件**：核心 `AgentEvent` 已有 `TurnAccepted/AssistantMessage/Handoff/ToolCall/Provider/Error/Warning`（`src/core.rs` L281-L325）；headless 对每个异步请求维护独立 `AsyncEventBuffer`，并分别限制 provider/agent 数量与字节数，防止 queued replacement 串线（`src/headless.rs` L814-L869）。`protocol::StdioEvent` 提供 sequence、request_id、turn_id、event 封套，可把 `DagNodeStateChanged`、`WorkerHeartbeat`、`ChildGreen` 作为新事件（`src/protocol.rs` L882-L918）。
3. **会话父子关系**：`core::Turn` 已有 `parent_id`、`with_parent` 和 ID 校验（`src/core.rs` L140-L190）；`SessionStore` 持久化 turns/handoffs/events，并可选 `SessionTree`（`src/session.rs` L144-L160）。建议新增 `DagNodeRecord { node_id, session_id, parent_session_id, root_session_id, depth, sibling_index, status, generation }`，持久化为受 schema 约束的 session event；不要仅依赖 `Turn.parent_id`，因为它表达对话 turn，不足以表达跨 session 的 grandparent/sibling/child。
4. **保活与 liveness**：`LiveSessionRegistry` 已有 owner_epoch、workspace、last_seen_ms、注册上限、heartbeat、TTL 清理（`src/session.rs` L3274-L3355），`Agent` 暴露注册、heartbeat、注销和 claim/finish live mailbox（`src/core.rs` L836-L914）。建议将 DAG node lease 复用 `owner_epoch + expires_at_ms`，增加 `DagNodeHeartbeat { node_id, generation, green_digest, child_summary }`；heartbeat 只更新租约，不能直接宣告 green。
5. **取消、派生和背压**：`runtime::CancellationToken` 是无锁幂等协作取消，后端需在流式 chunk/重试前检查（`src/runtime.rs` L49-L87）；`RuntimeEvent` 已区分 Accepted、Started、Queued、CancelRequested、Completed、Rejected、Closed（L171-L193），`BackgroundRunner` 用有界 command/event channel，`try_submit` 在满时返回 `QueueFull`（L236-L315）。最小派生机制是：父 worker 计算待办后以 `DagSpawnRequested` 事件持久化一次 admission（含 parent/grandparent/root、task digest、role=`worker`、generation）；调用 `BackgroundRunner::try_submit` 创建 child session；收到 `Accepted/Started` 后再发送 child mailbox。队列满只返回可重试的 admission error，不重复创建 child。

建议的可执行落点：

- `src/session.rs`：在 `SessionStore` 旁新增 `DagNodeStore` 或把 `DagNodeRecord`/`DagNodeStatus`（`Open | Running | Green | Blocked | Cancelled | Closed`）作为 typed journal record；提供 `register_node`、`parent_of`、`children_of`、`direct_siblings`、`ancestors`、`descendants`、`mark_green`、`can_close`、`lease_expired`。`can_close(node)` 必须是“自身 `Green` 且全部 child/grandchild 递归 `Green`”；任一非 green 或 lease 过期都返回 false。
- `src/core.rs`：扩展 `WorkerExecutionBinding`（现有 blueprint/goal/item/lease/policy digest 与过期校验见 L37-L80）为可选 `dag_node_id`、`parent_session_id`、`root_session_id`、`generation`；新增 `send_dag_message`、`poll_dag_mailbox`、`heartbeat_dag_node`、`close_dag_node`。`close_dag_node` 先从 `DagNodeStore::can_close` 验证；失败时保持 `AgentPhase::Running/Idle` 与 lease，不得将 node 标记 closed。复用 `AgentEvent::Handoff`（L5450-L5465）记录派生意图或新增 typed `Dag*` 事件。
- `src/runtime.rs`：在 `BackgroundRunner` 外包一层 `DagRuntime`，维护 `HashMap<NodeId, JobId>` 和 parent→children 索引；`spawn_child(parent, spec)` 只在 journal admission 成功后调用 `try_submit`，`cancel_subtree` 逐 JobId 发 `try_cancel`，并将 `Cancelled` 与 `Panicked` 作为非 green 终态。派生任务收到消息后仍走 `CancellationToken`，禁止强杀；shutdown grace 的“可 detach 但副作用不可回滚”语义可直接复用（L327-L341）。
- `src/headless.rs`：将 `Command::Mailbox`/`StdioEvent` 路由扩展为 `dag_message`、`dag_status`、`dag_spawn`、`dag_close`；继续使用按请求 `AsyncEventBuffer`（L833-L869）和 `PendingSteer`（L1064-L1079）解决取消与新 worker admission 之间的竞态。给每个事件附 `node_id`、`parent_id`、`root_id`、`generation`，避免 sibling 输出串线。
- `src/protocol.rs`：在现有有界字段基础上定义 `DagNodeRef`、`DagMessageEnvelope`、`DagSpawnRequest`、`DagCloseRequest`、`DagStatusEvent`；校验 ID、digest、TTL、generation，沿用 `MailboxRequest` 的显式 claim/complete，不让客户端字段伪造 sender/workspace（L87-L113）。`Command::Mailbox` 的协议入口（L211-L263）可作为兼容路径。
- `src/tool_runtime.rs`：worker 处理 DAG 消息或派生前，复用 `execute_tool_batch` 的全批次取消、prepare、结果按源顺序回收语义（L157-L176），把 `DagSpawnRequested` 的持久化放在 `prepare`/admission 之前；工具失败、取消、未知副作用都只能使节点保持 open/blocked，不得触发 close。
- `src/approval.rs`：现有 `ApprovalCoordinator` 是 bounded condition-variable rendezvous，request 等待 host 决策并接受取消谓词（L126-L175）；把派生/跨节点写操作作为 `ToolOrigin::BlueprintWorker` 的 approval request，携带 `policy_digest` 与 `lease_id`，沿用 worker approval 必须关联 policy/lease 的校验（L91-L103），防止 child 伪造父授权。
- `src/providers/**`：provider 层只负责 route、模型能力和认证，不应保存 DAG 拓扑。`providers::Protocol`/`ProviderDefinition` 是静态路由定义（`src/providers/mod.rs` L13-L20、L146-L160），`ValidatedRoute` 只暴露不可变 provider/model/capabilities/digest（`src/providers/connection.rs` L40-L110）。worker 的 role 可选择 provider/model，但 parent-child mailbox、green 判定、保活和派生必须在 core/session/runtime 完成；provider stream 仅通过 `AgentEvent::Provider` 和 headless event buffer 回传。

与 `role.rs` 的差异清单（可验证）：

- role.rs 当前只合并内存 `Config`，zenpi 需要把 DAG 节点、lease、消息状态写入 `SessionStore` journal，并能重启恢复；验证方法是杀掉进程后 `SessionStore::open_existing` 仍能重建节点状态。
- role.rs 的用户角色优先和 `SessionFlags` 高优先级是确定性层叠；zenpi 应为每个 child 记录 immutable role snapshot/digest，防止父配置热改导致运行中 worker 漂移。
- role.rs 没有取消、重试和派生；zenpi 需用 `CancellationToken` + `QueueFull/Closed` 明确 admission 结果，重试只针对未 admission 的请求；已 claim mailbox 不得隐式重试。
- role.rs 没有层级拓扑；zenpi 必须区分 direct parent、grandparent、direct sibling、direct child，并用 ancestry/descendant 查询实现递归 green barrier。只有 `node.green && all_descendants_green` 才允许 `close`；否则续租并派生新 worker 处理新消息。
- role.rs 的错误被压成 unavailable；zenpi 协议应保留 `agent_closed`、`runtime_queue_full`、`owner_required`、`lease_expired`、`not_green` 等机器码，便于 headless 客户端恢复。

## 未决问题

无法从 `role.rs` 确认：role 文件的完整 TOML schema、`parse_agent_role_file_contents` 对未知字段的全部规则、multi-agent tool handler 如何选择 role，以及 provider 实际调用的重试策略。zenpi 侧还需决定 DAG 节点是否一节点一 `SessionStore` 文件、grandchild 消息是否允许跨 workspace（当前 mailbox 明确要求同 workspace），以及“全部 child/grandchild 全绿”是否包含动态派生后新加入的后代；这些点不能由本源文件推出。

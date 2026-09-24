# AC-024 — codex/codex-rs/core/src/agent/role_tests.rs

- source_id/item_id: `AC-024` / `AC-024`
- source_path: `codex/codex-rs/core/src/agent/role_tests.rs`
- source_hash: `17cdabf745d5c0eeab0ca1a40134f23aad4bc3927ddd369337ed8d68e23a1a3b`
- source_bytes: `22803`
- source_lines: `741`
- coverage: 已按源文件顺序读取完整字节范围 `0..22803`、行范围 `L1-L741`，包括全部 `use`、私有 helper、测试属性、断言与文件末尾函数。

## 完整行为复盘

### 文件级导入与测试边界（L1-L14）
文件没有 `pub` 导出符号；`use super::*` 引入被测 agent 模块的 `Config`、`TomlValue`、`AgentRoleConfig`、`apply_role_to_config`、`spawn_tool_spec`、`built_in` 等名称。其余导入固定了配置文件常量、配置层排序、插件/技能管理、推理枚举、文件系统、`Arc` 与 `TempDir`。所有断言都在测试作用域内，生产行为由被测模块实现。

### 私有辅助函数

- `test_config_with_cli_overrides`（`async`，`L16-L29`）：输入 `Vec<(String, TomlValue)>`，创建独立 `TempDir` 作为 `codex_home` 与 fallback cwd，将覆盖项交给 `ConfigBuilder`，异步 `build` 成功则返回 `(TempDir, Config)`。临时目录由返回值持有；构建失败通过 `expect` 直接使测试失败。空向量表示无 CLI 覆盖；没有超时、重试或并发共享状态。
- `write_role_config`（`L31-L37`）：把 `name` 拼到临时 home，异步写入 `contents`，返回 `PathBuf`；父目录必须已存在，写失败 `expect`。它不校验 TOML，故合法性/解析错误留给 `apply_role_to_config`。
- `session_flags_layer_count`（`L39-L46`）：以 `LowestPrecedenceFirst`、包含 session 层的方式读取配置层，按 `ConfigLayerSource::SessionFlags` 计数；只读、无副作用，用于判定应用角色是否追加一层。

### `apply_role_to_config` 行为测试（L48-L641）

- `apply_role_defaults_to_default_and_leaves_config_unchanged`（`#[tokio::test]`，`L48-L58`）：在空 CLI 覆盖的默认配置上用 `role=None`；调用应成功且前后 `Config` 完全相等。判据是默认角色不引入层或字段改变。
- `apply_role_returns_error_for_unknown_role`（`L60-L69`）：传入不存在的 `Some("missing-role")`；必须返回错误字符串 `unknown agent_type 'missing-role'`。错误在异步调用结果中显式比较，配置不要求继续使用。
- `apply_explorer_role_sets_model_and_adds_session_flags_layer`（忽略，`L71-L84`）：传入内建 `explorer`，期望 model=`gpt-5.1-codex-mini`、reasoning effort=`Medium`、session flags 层数增加一。`#[ignore = "No role requiring it for now"]` 表明当前默认角色可能未启用；它仍记录未来行为契约。
- `apply_role_returns_unavailable_for_missing_user_role_file`（`L86-L103`）：向 `agent_roles` 注册 `custom`，`config_file` 指向不存在绝对路径；应用后必须失败并等于 `AGENT_TYPE_UNAVAILABLE_ERROR`。这是文件不存在的统一不可用错误路径。
- `apply_role_returns_unavailable_for_invalid_user_role_toml`（`L105-L123`）：写入 `model = [` 这一截断 TOML，注册为 `custom` 后应用；解析失败也必须折叠为同一 `AGENT_TYPE_UNAVAILABLE_ERROR`，而不是泄漏解析细节。
- `apply_role_ignores_agent_metadata_fields_in_user_role_file`（`L125-L154`）：角色文件同时含 `name`、`description`、`nickname_candidates`、`developer_instructions`、`model`；只断言 model=`role-model`。测试意图是角色文件中的 agent 元数据字段不作为运行时角色元数据覆盖，配置型字段仍可生效。
- `apply_role_preserves_unspecified_keys`（`L156-L194`）：CLI 先设 `model=base-model`，并手工设置 sandbox 与 execve wrapper；角色文件只给 instructions 与 `model_reasoning_effort=high`。应用后 model 仍为 `base-model`，effort 变 `High`，两个路径字段原值保留。边界是合并只覆盖明确出现的键。
- `apply_role_preserves_active_profile_and_model_provider`（`L196-L246`）：临时 `config.toml` 定义 provider 与 profile，harness 选中 `test-profile`；角色只写 instructions。应用后 active profile、provider id/name 全部保持，说明空角色设置不会破坏当前 profile/provider。
- `apply_role_top_level_profile_settings_override_preserved_profile`（`L248-L305`）：当前 profile 提供 model/effort/summary/verbosity；角色文件在顶层提供同四项。profile 名称仍是 `base-profile`，但四个顶层设置分别被角色值覆盖（`role-model`、`High`、`Detailed`、`High`）。优先级是角色 session 层高于已解析 profile。
- `apply_role_uses_role_profile_instead_of_current_profile`（`L307-L366`）：当前 profile 指向 `base-provider`，角色文件顶层 `profile = "role-profile"`；应用后 active profile=`role-profile`、provider 切换为 `role-provider`/`Role Provider`。角色指定 profile 会替换当前 profile 上下文。
- `apply_role_uses_role_model_provider_instead_of_current_profile_provider`（`L368-L424`）：当前 profile 使用 base provider，角色文件直接给 `model_provider = "role-provider"`；应用后 `active_profile=None`，provider 为 role provider。直接 provider 设置会清除 profile 选择并成为解析来源。
- `apply_role_uses_active_profile_model_provider_update`（`L426-L489`）：当前 `base-profile` 有 base provider 与 low effort，角色文件在 `[profiles.base-profile]` 内改 provider 与 effort；应用后仍 active `base-profile`，provider=`role-provider`，effort=`High`。角色对当前 profile 的局部表更新优先于原 profile。
- `apply_role_does_not_materialize_default_sandbox_workspace_write_fields`（非 Windows，`L491-L559`）：CLI 已有 `sandbox_mode=workspace-write`、network access=true；角色文件只给 `writable_roots`。取最后 session flags 层，断言该层不物化 `network_access`、`exclude_tmpdir_env_var`、`exclude_slash_tmp` 默认键；但最终 `SandboxPolicy::WorkspaceWrite` 的 network access 仍为 true。测试区分“层中显式序列化字段”与“解析后的默认策略”。
- `apply_role_takes_precedence_over_existing_session_flags_for_same_key`（`L561-L590`）：CLI session 层先设 `model=cli-model`，角色设 `role-model`；结果使用 role-model，且 session flags 层数增加一。后追加角色层对同键具有更高优先级。
- `apply_role_skills_config_disables_skill_for_spawned_agent`（Windows 忽略，`L592-L641`）：创建 `skills/demo/SKILL.md`，角色文件添加 `[[skills.config]]`，精确路径且 `enabled=false`；应用后通过 `PluginsManager`/`SkillsManager::skills_for_config` 找到 `demo-skill`，`is_skill_enabled` 为 false。验证角色配置能约束派生 agent 的技能启用状态；文件系统写入失败均 `expect`，技能发现结果必须存在。

### `spawn_tool_spec::build` 与 `built_in` 测试（L643-L741）

- `spawn_tool_spec_build_deduplicates_user_defined_built_in_roles`（`L643-L663`）：用户 map 覆盖 `explorer` 并新增 `researcher`；生成 spec 必须含 `researcher: no description`、用户 explorer 描述、内建 default 描述，且不能残留内建 explorer 文案。边界是同名用户角色去重并覆盖内建描述。
- `spawn_tool_spec_lists_user_defined_roles_before_built_ins`（`L665-L683`）：用户角色 `aaa` 的描述在 spec 中的位置必须早于内建 default，规定展示顺序为用户角色优先。
- `spawn_tool_spec_marks_role_locked_model_and_reasoning_effort`（`L685-L708`）：临时 role TOML 含 model=`gpt-5` 与 effort=`high`；spec 必须追加不可变提示，明确模型和 reasoning effort 均不能更改。读取文件失败直接 `expect`。
- `spawn_tool_spec_marks_role_locked_reasoning_effort_only`（`L710-L733`）：只含 effort=`medium` 时，spec 只生成 reasoning effort 锁定提示；验证锁定文案按字段存在性组合，而非无条件宣告 model 锁定。
- `built_in_config_file_contents_resolves_explorer_only`（`L735-L741`）：对 `Path::new("missing.toml")` 调 `built_in::config_file_contents` 必须为 `None`；这是未知/非内建路径的边界判据，未测试 explorer 正向内容。

并发语义：配置构建与角色写入使用 Tokio 异步 API，但每个测试都独占 `TempDir` 与 `Config`；没有共享可变全局、锁、任务间通信或并发断言。`Arc` 只在技能管理器依赖中使用。同步 `#[test]` 仅覆盖纯字符串 spec 构建。

## 状态、取消、恢复与副作用

该源文件是角色配置测试，不实现取消、超时、重试或恢复协议。`async fn` 只等待 `ConfigBuilder::build`、`tokio::fs::write`；没有 `CancellationToken`、deadline、retry loop 或持久化回放。可观察副作用是每个测试在 `TempDir` 下创建配置、role TOML、技能目录/`SKILL.md`，测试结束后由 `TempDir` 清理；`config.toml` 与 role 文件不会写入仓库。`apply_role_to_config` 通过内存中的 config layer stack 改变 `Config`，测试用 clone、层计数和最终字段检查可见；未知角色、缺文件、非法 TOML 走错误返回，测试没有尝试恢复。`#[ignore]` 的 explorer 测试是显式跳过状态，并非运行时取消。因没有真实 provider、shell、网络或工具调用，源内没有外部副作用；技能测试的“spawned agent”只体现在配置过滤，不启动进程。

对 zenpi DAG 的约束应沿用这些可观测边界：取消只撤销尚未安全提交的节点意图，不能把已发生的工具/ provider 副作用伪装成回滚；超时应使节点进入 `Interrupted`/`UnknownOutcome` 并保留可重试证据；恢复必须从 `src/session.rs` 的追加记录重建 parent/child 状态；只有终端结果和所有后代结果已持久化为绿色，才允许 close。

## 源内测试与行为判据

源内已包含 20 个测试函数（`L48-L741`），覆盖默认角色不变、未知角色、缺失/非法用户配置、字段合并与优先级、profile/provider 解析、sandbox 默认字段、技能禁用，以及 spawn tool spec 的去重、排序、锁定提示和未知内建路径。可独立验证的判据是：

1. 在 codex core crate 中运行角色测试筛选（例如 `cargo test role_tests`），默认测试全部通过；被 `#[ignore]` 标记的 explorer 测试只有显式 `--ignored` 才运行。
2. 对每个临时 role TOML，检查 `apply_role_to_config` 的 `Result`：未知角色字符串必须精确匹配，文件不存在和非法 TOML 必须统一为 `AGENT_TYPE_UNAVAILABLE_ERROR`。
3. 比较应用前后 `Config`、`session_flags_layer_count`、active profile/provider、sandbox policy 与技能启用结果，验证“只覆盖显式键、角色层后置优先、profile/provider 选择一致”的判据。
4. 将 `spawn_tool_spec::build` 输出作为纯字符串快照，验证用户角色先于内建角色、同名内建描述被去重、锁定提示只由 TOML 中实际字段触发。


## zenpi Rust 映射

### 可复用原语与 DAG 通信拓扑

`role_tests.rs` 的核心可迁移点是“角色是有边界的配置层，派生 worker 得到显式覆盖、继承未指定值，并在 spec 中看到不可变约束”。zenpi 已有更直接的通信基元：`src/protocol.rs:L91-L113` 的 `MailboxRequest::{Send,Receive,Acknowledge,Claim,Complete}` 是消息协议；`StdioEvent`（`src/protocol.rs:L882-L919`）和 `src/runtime.rs:L171-L193` 的 `RuntimeEvent` 是事件流；`src/session.rs:L3180-L3268` 的 `MailboxMessage`/`MailboxAction` 是带 digest、sequence、TTL、claim token、结果状态的持久邮箱；`LiveSessionRegistry`（`src/session.rs:L3278-L3559`）提供 owner 注册、heartbeat、按序 claim、完成及回执。建议把 DAG 节点通信统一为 `DagMessage` 包在 `MailboxMessage.payload` 中，字段至少为 `{dag_id, node_id, sender_node_id, relation, kind, parent_id, generation, lease_id, payload}`，其中 `relation` 枚举 `Parent|Grandparent|Sibling|Child`；不要另造无审计的内存全局 channel。

节点 worker 负责一个 `node_id` 时，最小可验证路由是：

- parent / grandparent：向对应 session mailbox `Send`，携带 `relation` 和 `in_reply_to`；parent 可代转 grandparent，但必须保留原 sender 与 digest。
- 直接 sibling：由 parent/调度器提供 sibling session id 白名单，worker 直接 `Send`；禁止通过“任意目标 session id”绕过拓扑授权。
- 直接 child：派生时写入 child mailbox 或通过 `LiveSessionRegistry::claim_next` 交付；child 完成后用 `finish_claim_with_reply` 回写原 sender，保留幂等 reply id。
- 事件/状态：worker 本地进度用 `RuntimeEvent`/`StdioEvent`，持久完成、失败、心跳和 close 判定写 `SessionStore::append_event`，避免仅依赖 stdout。

### 会话父子关系与关闭判定

`src/core.rs` 已有 `Turn { parent_id }`（约 `L100-L170`）和 `Turn::with_parent`，可承载“会话父子/上下文父级”；`src/runtime.rs:L765-L831` 的 `InputBoundary`/`InputBoundaryGate::with_context_parent` 可把当前模型 turn 绑定到 parent。会话树控制在 `src/protocol.rs:L1161-L1234` 的 `TreeAction`/`TreeRequest`，`src/session.rs` 的 `tree_snapshot`、`tree_ancestry_turns`、`fork_at_tree_leaf`（约 `L915-L1140`）可作为 DAG 可视化与分支持久化的基础。

建议新增 `src/dag.rs`（或 `src/domain_execution.rs` 的并列模块）及以下可执行类型：

```rust
pub struct DagNodeState {
    pub dag_id: String,
    pub node_id: String,
    pub parent_id: Option<String>,
    pub child_ids: BTreeSet<String>,
    pub status: DagStatus, // Running, Green, Failed, Interrupted, KeepAlive
    pub generation: u64,
    pub lease_id: String,
}
pub enum DagStatus { Running, Green, Failed, Interrupted, KeepAlive }
pub struct DagCloseReport { pub node_green: bool, pub descendants_green: bool, pub open_children: Vec<String> }
```

`DagNodeState` 的 parent/child 关系必须同时写入 session 事件（例如 `dag_node_started`, `dag_edge_declared`, `dag_node_green`, `dag_keepalive`, `dag_worker_derived`, `dag_node_closed`），并用 `node_id` 去重。`close_node(node_id)` 的实现应先读取节点自身及递归全部 child/grandchild 的最新状态；只有自身为 `Green` 且后代集合非空部分全部 `Green`、无活跃 lease、无未完成 mailbox claim 时才追加 `dag_node_closed`。任一后代失败、超时、未知结果、仍运行或未确认，都返回 `KeepAlive`，保留当前 worker lease，并由 `derive_worker(node_id, reason)` 创建新 generation/lease，继续处理新工作；不能把“没有发现 child”误判为全绿，应区分叶节点与尚未展开的 child 集合。

### 各现有模块落点

- `src/headless.rs`：`Command::Tree` 的 dispatch（约 `L5503-L5558`、`L8326-L8333`）和 `Command::Mailbox`（约 `L6392-L6407`、`L8941-L9090`）已经是 JSONL 入口。新增 `Command::Dag` 或将 `tree` 扩展为 `DagAction::{Send,Claim,Heartbeat,Close,Derive,Status}`，在 owner 忙时保持 admission/queue 语义；响应使用 request id 关联，异步进度用 `StdioEvent`。验证：同一 `message_id` 重放必须幂等，非拓扑目标返回 typed error，close 返回 `DagCloseReport`。
- `src/core.rs`：`Agent` 是节点执行所有权和 provider/tool 边界；`WorkerExecutionBinding`（约 `L20-L65`）已有 blueprint/goal/item/lease/policy digest 与过期校验，`renew_blueprint_worker`、`cancel_blueprint_worker`、`settle_blueprint_worker`（约 `L2823-L2905`）可复用为 node lease 生命周期。建议加入 `DagCoordinator` 字段及 `admit_dag_worker`, `record_dag_green`, `try_close_dag_node`, `derive_dag_worker`；close 前调用递归 descendant 汇总，settle 前确认 children 已 reap。现有 `Agent::try_close`（`L5711-L5734`）是整个 session 关闭，不应直接当作 DAG 节点 close。
- `src/session.rs`：`SessionStore` 的 append-only events、operation markers/recovery（约 `L1207-L1245`、`L1410-L1590`）适合持久 DAG 状态；`SessionMailbox`/`LiveSessionRegistry` 负责消息、TTL、claim token 和 owner epoch（`L3278-L3559`），`SessionMailbox::enqueue/list/update`（约 `L3574-L3775`）提供实际存储。建议增加 `DagRecord`/`DagEdgeRecord` 的 JSONL schema 与 `descendant_status(node_id)` 投影；为 `MailboxMessage` payload 加关系校验，确保 sibling/child 只能在同一 workspace 与授权 DAG 内通信。
- `src/tool_runtime.rs`：`execute_tool_batch`（`L157-L347`）已限制 batch 数量/字节、顺序或并行、取消传播，并保证每个 call 一个终端结果；可作为节点内部工具执行器。派生 worker 前在 `prepare` 写入 `dag_worker_derived` 和 operation intent；遇到 side effect 的 `Io/Internal/CommandTimeout/Cancelled` 时保持 unknown/interrupted，不得立即标绿。差异是当前 batch 没有 DAG descendant 聚合，需要由 core coordinator 在 batch 终止后更新节点状态。
- `src/runtime.rs`：`BackgroundRunner`（`L236-L421`）提供有界 command/event channel、FIFO pending、`JobId`、`CancellationToken`、`RuntimeEvent::Completed` 与 shutdown grace；可承载一个节点 worker。最小保活机制是在节点 job 返回 `KeepAlive` 时不发 `Completed`-green，而发一个 `RuntimeEvent`/`DagKeepAlive`，由 host 调 `try_submit` 派生新 generation；`CancellationToken` 只合作式取消，grace 后 detach 不得被视为成功。用 `RuntimeConfig.max_pending` 防止派生风暴，并在 `Closed` 前 flush keepalive/derive 事件。
- `src/protocol.rs`：复用 `MAX_ID_BYTES`、`MAX_MAILBOX_TEXT_BYTES`、TTL 上限、`TreeRequest::validate` 的严格版本/ID 校验（`L91-L113`、`L544-L590`、`L1161-L1234`）。建议定义 `DagRequest`/`DagAction`，字段含 `dag_id,node_id,target_id,relation,lease_id,expected_generation`；拒绝空/控制字符/过长 id、错误 workspace、过期 TTL 和 generation 冲突。`MailboxOutcome::Abandoned` 可映射为节点中断，但不能映射为 green。
- `src/approval.rs`：`ApprovalRequest` 校验 worker 的 `policy_digest` 与 `lease_id`（`L40-L96`），`ApprovalCoordinator` 的取消感知等待和 accepted 持久化顺序（约 `L126-L151`、`L348-L451`）适合节点工具审批。派生 child 时继承 immutable policy digest/lease scope；parent/grandparent 的消息不能自动提升 approval，所有 side effect 仍需 child 自己的 gate。close/derive 前应先排空或取消 pending approvals，并把未决请求写成 interrupted。
- `src/providers/**`：`providers/mod.rs` 的 `Protocol`/`RouteRule`/`ProviderDefinition`（约 `L1-L150`）只负责 provider 路由、wire API 与 capabilities；`registry.rs` 的 `ModelDescriptor`/digest/能力校验用于角色式模型锁定。它们不应保存 DAG 拓扑或决定 close；节点配置可记录 provider/model/reasoning 的 immutable snapshot，调用仍由 core 的 lease/policy gate 约束。验证不同 provider 只改变 backend 请求与能力校验，不改变 mailbox、parent/child 状态机。

### 保活与派生的最小实现步骤

1. `admit_dag_worker` 校验 `WorkerExecutionBinding`、parent edge、generation 与 lease，写 `dag_node_started`，再 `BackgroundRunner::try_submit`。
2. 节点执行期间每个 heartbeat 通过 `LiveSessionRegistry::heartbeat`，并把 `dag_heartbeat` 以有界频率写事件；heartbeat 失效只触发 keepalive/取消，不宣告失败或成功。
3. 节点需要和 parent、grandparent、直接 sibling、直接 child 通信时，统一走 `MailboxRequest::Send/Receive/Claim/Complete`；发送方保存 digest，接收方用 owner epoch claim，回复用 `reply-{digest}` 幂等键。
4. 节点 terminal 时聚合自身结果与递归后代：全绿才 `dag_node_closed` 并 `settle_blueprint_worker`；否则写 `dag_keepalive`，按 reason 创建新 generation/lease、登记 child work，再通过 runner 提交派生 job。
5. 重启由 `SessionStore` 回放事件、`SessionMailbox` 列表和 operation recovery 重建；发现 claim/operation 无终端记录时进入 `Interrupted`/`UnknownOutcome`，必须人工或策略确认后重试。验证项是崩溃重启不会重复执行同一 `message_id`，也不会在未完成后代时 close。

### 与 `role_tests.rs` 的差异清单（可执行、可验证）

- 源测试验证配置层合并与角色展示；zenpi 当前没有等价的 `AgentRoleConfig` 派生 worker spec 锁定测试，应新增 `dag`/`worker` 测试覆盖“未指定字段继承、显式字段覆盖、policy/lease 不可被 child 改写”。
- 源测试没有 parent/grandparent/sibling/child 拓扑；应新增四类 mailbox 路由测试及越权目标拒绝测试。
- 源测试没有 close 聚合；应构造 node green + child green、child failed、grandchild running、unknown operation 四组 fixture，断言只有第一组允许 close，其余返回 KeepAlive 并产生 derive intent。
- 源测试只用 `TempDir` 短生命周期；zenpi 需测试 session 重启、TTL 过期、owner epoch 变化、重复 reply 与 bounded queue 满载，确认恢复和幂等判据。
- 源测试没有 provider/approval 副作用；zenpi 需验证 side-effect 工具在取消/超时后保持 unknown，不被 descendant 聚合错误标绿，并验证 `ApprovalCoordinator` 的 lease/policy digest 绑定。

## 未决问题

无法从源文件确认：`apply_role_to_config` 对所有 agent role 字段的完整合并顺序、内建 `explorer` 当前是否在生产表中启用、`spawn_tool_spec` 的完整格式化器实现，以及 `AGENT_TYPE_UNAVAILABLE_ERROR` 的构造细节。源文件也没有 DAG 拓扑、worker 保活、派生、provider 调度或跨进程通信实现，因此上述 zenpi 类型名和事件名是建议落点，需以新增测试和现有协议版本评审确认。

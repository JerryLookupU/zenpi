# opencode runtime learn — gap digest (zenpi Rust mapping)

## OC-001 packages/opencode/src/agent/agent.ts
- agent 目录/类型落点：建议新增 src/agent.rs，定义可序列化的 AgentInfo（name/description/mode/native/hidden/top_p/temperature/color/permission/model/variant/prompt/options/steps）、AgentMode、AgentCatalog 和 AgentService。AgentCatalog::get/list/default_info/default_name 对应 L312-L344；AgentService 负责从配置构造内置 agent 与合并用户覆盖。
- src/core.rs 对照：现有 Agent（状态机、AgentPhase、AgentEvent、turn/tool 执行）应继续作为运行时执行器；新增 AgentInfo 不要与现有执行器同名，建议字段为 profile/catalog。调用 ToolRuntime 前，将 AgentInfo.permission 编译成 SideEffectPolicy/ApprovalPolicy。
- src/providers/registry.rs 与 src/providers/**：ModelDescriptor、ModelRegistry::resolve 可承载 providerID/modelID 校验和能力交集；src/providers/connection.rs 的 ValidatedRoute 负责端点/auth，不应吸收 agent 权限。新增 AgentGenerator 时通过既有 backend/provider route 发结构化 JSON 请求，明确区分普通请求与 OpenAI OAuth/Codex dialect。
- src/approval.rs：把 TS 的 Permission.merge 结果翻译为已有 ApprovalPolicy；plan 的 deny/allow 路径、explore 的只读工具集合应在进入 execute_tool_batch 前成为不可变策略快照。ApprovalCoordinator 的 pending/accepted/cancel 语义可替代 TS 中未实现的审批，但不能把 remembered approval 当作模型输入权限。
- src/tool_runtime.rs：ToolBatchOptions、ToolBatchDecision 和 execute_tool_batch 已提供批量、顺序/并发、取消协作；按 agent mode 选择 sequential/max_concurrency，并在 prepare 中应用 profile 权限。TS 的 Truncate.GLOB 外部目录 allow 需要落到 ToolContext 的路径规则。
- src/runtime.rs：BackgroundRunner、CancellationToken、SubmitError 可承载 AgentGenerator 的异步任务；为模型请求增加 deadline/abort 绑定，补齐 TS 源没有实现的超时和取消。不要把不可取消的 future 伪装成已取消。
- src/session.rs：TS agent registry 不持久化；zenpi 若要恢复 profile，应在 SessionStore 记录 profile 名称、模型和策略 digest，而不是把完整权限对象隐式塞入 turn。使用现有 operation marker/recovery 机制记录生成请求的 started/completed/failed。
- src/protocol.rs 与 src/headless.rs：当前 Command 有 Prompt/Steer/Cancel/Resume/Approve 等，但没有 agent catalog/generate 命令；建议增加严格大小限制的 AgentList/AgentGet/AgentGenerate 变体及版本化响应。headless.rs 负责 JSONL correlation、replay、错误映射；生成事件应走 AgentEvent::Provider/终端响应，避免直接写 stdout。
- 差异清单与验证：① Effect Layer/Context/InstanceState → Rust 明确所有权与 Arc<RwLock<AgentCatalog>> 或启动期不可变快照；② Schema.Finite/optional 字段 → serde + 自定义校验；③ TS Permission pattern merge → Rust 中确定 deny 优先级和路径规范化；④ TS get 的 undefined 与声明不一致 → Rust 选择 Option<AgentInfo> 并在 protocol 层转错误；⑤ TS plugin/OpenTelemetry hook → Rust trait hook，禁止生成器静默改写策略；⑥ TS 无 timeout/retry → Rust 以 CancellationToken/deadline 明确实现；⑦ TS OAuth streaming 特例 → provider dialect capability 测试。可验证命令：cargo fmt --check、cargo check、cargo test，并为新增 protocol 做重复 request-id、取消、模型解析和权限边界测试。
未决: 1. InstanceState.make 的具体缓存失效/并发保证不在本文件中，无法确认同一 ctx 是否可并行初始化。
2. Effect.promise 在当前 Effect 版本中的中断行为、AI SDK 是否支持底层 abort，源文件未给出。
3. ProviderTransform.providerOptions 对每个 provider 的最终 JSON 形状、Plugin 触发器是否会异步修改 system，需查各自实现。
4. Config 对 maxSteps 等别名的预处理不在本文件；本文件只读取已经规范化的 value.steps。

## OC-002 packages/opencode/src/agent/subagent-permissions.ts
现有 zenpi 已有可复用的“宿主策略/工具门禁”骨架，但没有与 `PermissionV1.Ruleset` 等价的子代理规则集或 `deriveSubagentSessionPermission` 函数。建议按以下方式落点：

- `src/approval.rs` 的 `ApprovalPolicy`、`ApprovalMode` 与 `decide_after_preflight`（约 `L22-L35`、`L466-L525`）面向 side effect 和工具名做审批；它不是规则数组继承器。可新增独立的 `SubagentPermissionRule { permission: String, pattern: String, action: PermissionAction }` 与 `derive_subagent_session_permission(parent: &[SubagentPermissionRule], subagent: &[SubagentPermissionRule]) -> Vec<SubagentPermissionRule>`，保持源实现的过滤顺序、名称存在即跳过默认 deny、以及不去重行为；不要把该逻辑塞进审批决策以免改变全局审批语义。
- `src/core.rs` 的 `Agent` 保存 `ToolRuntime`，`set_approval_policy`（`L1498-L1501`）和 `set_worker_execution_binding`（`L1523-L1556`）分别配置审批与 worker lease。建议在创建子 Agent/worker 的入口先调用上述纯函数，再把结果转换为 `ToolContext` 的 gate/运行时策略；验证点是子会话拥有独立规则快照，父 Agent 的运行时字段不被原地改写。
- `src/tool_runtime.rs` 的 `execute_tool_batch`（`L168-L347`）负责批量、顺序/并行、取消和结果关联；它不应承担“从父会话派生子代理权限”。在 batch 的 `prepare` 之前执行派生并拒绝未允许的 `task`/`todowrite` 调用，可验证默认 deny 在任何 handler 启动前生效。
- `src/session.rs` 的 `append_event`（`L1207-L1215`）和操作恢复机制负责 JSONL 持久化、未知结果和显式 retry/abandon。源函数没有持久化；若 zenpi 要审计派生结果，建议追加不可变事件（记录 parent session、child session、规则摘要/哈希），恢复时重新读取快照而不是自动重试工具副作用。规则派生本身应保持幂等。
- `src/runtime.rs` 的 `BackgroundRunner::try_cancel`（`L319-L327`）和 `CancellationToken` 适合包住子代理生命周期；取消应阻止后续 turn/tool dispatch，但不改变已经生成的规则数组。源行为没有并发状态，故不要为纯派生函数引入额外线程或阻塞。
- `src/protocol.rs` 的 `TurnMode`（`L46-L56`）与 `StdioRequest`（`L156` 起）承载 start/steer/cancel/approval 请求；若暴露子代理创建接口，应增加明确的 child/session correlation 字段并在 admission 阶段派生权限，不能把模型提供的参数直接当作 allow。可验证判据是取消请求只取消目标运行，不产生隐式权限提升。
- `src/headless.rs` 已路由 approval、cancel、steer，并在异步 worker 中排空审批事件（例如 `L4177-L4238`、`L4462-L4695`）。建议子代理规则快照随 worker admission 绑定，在 headless 事件中输出 child session ID 和拒绝原因；不得让 headless 输入绕过父 deny 或把 `remember` 变成子代理 allow。
- `src/providers/**` 目前主要声明 provider 能力和工具调用 wire 格式（如 `src/providers/mod.rs`、`connection.rs`、`registry.rs`），没有权限继承语义。保持 provider 无权决定规则；权限派生应在 `core`/`tool_runtime` 之前完成，provider 仅接收已裁剪的工具集合或调用结果。
- 已有 `src/tools.rs` 的 `ToolOrigin`（`L93-L101`）、`BlueprintPolicySpec`（`L104-L119`）、`ToolContext::with_blueprint_gate`（`L829-L843`）与 `check_call_gate`（`L872-L883`）可作为执行层门禁。映射时将 `external_directory` 规则转成 workspace/path gate，将 `task` 与 `todowrite` deny 转成工具名拒绝；必须保留源语义中“名称出现即视为覆盖”的兼容开关，或明确改成 action-aware 语义并补测试，不能悄然混用。

可执行验证计划：实现纯派生函数的表格单元测试（空父/空子、父 allow 丢弃、deny/external_directory 保留、名称存在但 action=deny 的边界、重复项和顺序），再通过 `execute_tool_batch` 验证默认 deny 在 dispatch 前返回；最后用 session journal 重启/取消测试确认规则快照不触发重试、不产生外部副作用。
未决: - `PermissionV1.Ruleset` 的下游匹配优先级、冲突规则解析及 `pattern: "*"` 的具体 glob 语义不在本文件中，无法仅凭该源确认。
- 调用方在 `src/tool/task.ts` 还会额外追加 `task` 对应的 deny、实验性 primary tool deny 和去重逻辑；这些调用方规则与本函数结果的最终合并顺序需结合会话创建代码确认。
- 当子代理存在同名但相互冲突的 allow/deny 规则时，本函数只看名称，最终能力取决于外部 PermissionV1 评估器，源文件没有说明。

## OC-003 packages/opencode/src/permission/arity.ts
- `src/tool_runtime.rs` 已有 `classify_master_session_input`（约 `L75-L110`），负责空输入、长度、控制字符和 `!`/steer 路由；建议新增纯函数 `command_prefix(tokens: &[&str]) -> Vec<String>` 或零拷贝的 `CommandPrefix<'a>`，并将 arity 表放在 `tool_runtime.rs` 同级的 `command_arity.rs`。该函数只生成审计键，不改变 `MasterSessionCommand` 的审批语义。
- `src/core.rs` 的 `run_user_shell_with_cancel` 是执行落点；在 `user_shell_input`/`tool_execution_started` 事件中可记录 `command_prefix`，但必须保留完整 `command`、`policy_digest` 和 `operation_id`。Rust 实现应继续在审批后、spawn 前持久化，不能用 prefix 结果自动放行。
- `src/headless.rs` 是协议适配和取消响应落点。可在构造 `UserShellRequest` 后分词并附加只读字段；异步审批排空、`emergency_cancel` 和取消错误路径保持现状，prefix 计算不得阻塞事件循环。
- `src/session.rs` 负责 WAL/事件持久化与恢复；若记录 prefix，应作为派生审计字段写入同一 operation 事件，恢复时只重放记录，不重新执行 shell。现有 session 的 owner、序列和权限约束不应被字典影响。
- `src/runtime.rs` 的 `BackgroundRunner` 提供有界命令/事件队列与协作取消；prefix 是短同步计算，宜在提交前或 worker 开始前完成，不需要新增线程。队列满、关闭、取消和 shutdown grace 的语义继续由 runtime 管理。
- `src/protocol.rs` 的 `parse_user_shell_input`（约 `L689-L707`）仅校验单个 `!`、长度和控制字符；它不应承担 shell tokenizer。若 wire 协议需要 prefix，应新增可选响应字段并限制大小，避免把未经解析的字符串误认为 token 数组。
- `src/approval.rs` 的 `ApprovalRequest`/`ApprovalCoordinator` 是授权与取消等待边界；prefix 只能作为 `tool = "user_shell"` 的预览或审计元数据，不能替代 `ApprovalDecision`、policy digest、lease 或 `persist_accepted`。
- `src/providers/**` 当前主要是 API endpoint/prefix 路由和连接校验（如 `providers/mod.rs` 的 `prefix_route`、`connection.rs` 的 URL path prefix），与 shell command arity 不同。不要复用 provider 的 URL prefix 类型；若共享命名，应使用独立 `CommandArityTable`，避免把 URL 段数和 shell token 数混淆。

建议的可执行验证：新增 Rust 单元测试覆盖未知命令、arity 1/2/3、最长嵌套前缀、空输入、短输入和输入不可变；在 headless 集成测试中确认 prefix 仅出现在审计/响应数据，审批拒绝与取消仍阻止 `RunCommandTool`；运行 `cargo test` 及现有 headless/session/approval 测试，并用一个含 flags 的真实 shell token 序列验证 tokenizer 与 arity 表职责分离。
未决: 1. `arity.ts` 的注释要求“flags NEVER count as tokens”，但源文件没有 tokenizer，无法确认所有调用者都已一致过滤 flags。
2. 未从源确认 `ARITY` 是否会由生成流程定期更新，也无法确认字典覆盖范围是否与 zenpi 的 shell 支持矩阵一致。
3. 未从源确认 `export * as BashArity from "./arity"` 在所有构建器中的自引用导出是否有特殊打包限制；现有 Bun 测试已证明其导入路径可用。

## OC-005 packages/opencode/src/permission/index.ts
- **建议落点**：新增 src/permission.rs 并在 src/lib.rs 导出，定义 PermissionAction { Allow, Deny, Ask }、PermissionRule、PermissionRequest、PermissionReply { Once, Always, Reject }、PermissionService。PermissionService 以 workspace/session owner 为作用域，持有 BTreeMap<PermissionId, PendingPermission>、批准规则向量和 Condvar/channel；纯函数 evaluate、from_config、merge、disabled、visible_tools 可直接单测。Wildcard.match 应明确采用与 TypeScript 相同的 glob 语义并以最后匹配为准。
- **src/approval.rs 对照**：现有 ApprovalCoordinator 已有 pending、respond_with_request、cancel_all、emergency_cancel、remember 与持久化前 fail-closed 语义（L163-L242、L289-L340、L366-L429），可复用其等待/取消骨架；但它的请求是单工具 ApprovalRequest，回复只有 Allow/Deny + remember，没有 patterns、always、同 session 级联或 CorrectedError。建议让新 PermissionService 负责规则层，再由 core.rs 的最终工具审批继续经过 ApprovalCoordinator，避免把 provider 名称当作授权。
- **src/core.rs 对照**：ToolRuntime 已保存 approval、approval_policy（L441-L450），set_approval_policy 从 session 事件恢复 remembered policy（L1496-L1518），prepare_tool 在 L4650-L4860 先做工具/技能/不可变 gate，再调用 ApprovalPolicy::decide_after_preflight，必要时创建请求、等待、持久化 approval_resolved 并才允许 dispatch。可在 prepare_tool 的 gate 与 ApprovalCoordinator 之间插入 PermissionService::evaluate；deny 应变成 ToolFailure::PolicyDenied，ask 则暴露 patterns/metadata，always 后把批准规则写入会话恢复数据。
- **src/headless.rs 对照**：drain_approval_events（L4177-L4248）把 coordinator 请求投影为 JSONL approval_request/view 事件；异步循环维护每个 project 的 coordinator（L4660-L4679），EOF 用 cancel_all、显式 shutdown 用 emergency_cancel（L4682-L4696）。新增权限服务时应沿同一 project owner 选择、事件序列和 replay 路径发布 Asked/Replied；同 session 的级联回复必须为每条被取消请求发 terminal event。
- **src/protocol.rs 对照**：现有 Command::Approve { approval_id, decision, remember } 与 approve|approval 解析（L187-L192、L257-L261、L501-L513）可作为传输入口。要映射 once|always|reject，建议新增版本化 PermissionReply 字段（保留旧 Approve 兼容），校验 ID、可选 feedback/message，并把 unknown request 映射成明确协议错误。
- **src/session.rs 对照**：append_event/events 是持久化入口（L1207-L1223）；当前 ApprovalPolicy::with_remembered_events 只恢复工具级 approval_resolved 且拒绝 worker 来源。若要等价 approved: Rule[]，需追加 permission_replied 或 permission_rule_added 事件，在 session reload 时按事件顺序重建规则；dispose/reload 必须像 TypeScript finalizer 一样唤醒并拒绝所有 pending。
- **src/runtime.rs 对照**：CancellationToken 是协作取消，后台 runner 的 queue、cancel、shutdown 和有限 grace period 位于 L49-L99、L308-L360。把 token 的 is_cancelled 注入权限等待；取消只保证等待者失败和不再 dispatch，不能回滚已开始副作用，符合源文件没有重试/回滚的事实。
- **src/tool_runtime.rs 对照**：execute_tool_batch（L157-L249）接收已由 host prepare 的 ToolBatchDecision，按源顺序执行并在取消/不确定副作用时停止；它不应自行推断权限。将 PermissionService 的 allow/deny/ask 结果转换为 ExecuteApproved 或 Reject(ToolFailure)，保留现有 dispatch 前 preview/gate 再检查。
- **src/providers/** 对照：anthropic.rs、openai.rs、
未决: - InstanceState.make 是否保证同一实例内 Effect 并发访问 Map 的串行化，源文件未给出；重复 id 的覆盖行为也未被防护或测试。
- EventV2Bridge.publish 失败时，ask 在进入 Effect.ensuring 前可能留下 pending，reply 在 deferred 完成前可能已删除 pending；事件桥错误下的恢复策略需由上层确认。
- Wildcard.match 的转义、路径分隔符和 glob 边界语义定义在外部模块，本文件无法确认。

## OC-006 packages/opencode/src/provider/auth.ts
- 现有认证底座：src/auth/mod.rs:L1-L170 已有 AuthBinding、AuthError、LoginFlowId、LoginState 与 LoginControl；src/auth/codex.rs 已实现固定 openai-codex 的 browser/device OAuth、active login 互斥、取消和提交；src/auth/callback.rs 已实现 loopback callback 的 state/URI 校验；src/auth/store.rs 负责 API/OAuth 凭据持久化、版本/修订与冲突；src/auth/resolve.rs 负责刷新、锁与 scoped secret。它们可承接 callback 的安全与持久化，不应把 token 放进 session.rs 普通记录。
- 建议新增通用模块：新增 src/auth/provider_methods.rs（或 src/provider_auth.rs）定义 ProviderAuthMethod { Api, Oauth { prompts, authorize, callback } }、TextPrompt、SelectPrompt、When、Authorization、AuthorizeInput、CallbackInput、ProviderAuthError；以 BTreeMap<String, Vec<ProviderAuthMethod>> 替代 TS 的 Record。hook 采用 Rust trait（如 ProviderAuthHook）并由显式 registry 注册，不能假设现有 src/providers/registry.rs 的模型 registry 能承载闭包。
- 对照 src/providers/**：src/providers/mod.rs 的 Protocol/AuthHeaderPolicy 和 src/providers/connection.rs 的 ValidatedRoute/revalidate_route_auth 只处理模型路由与凭据授权；src/providers/codex.rs 提供固定 endpoint 常量（OAuth 交换仍在 src/auth/codex.rs）；src/providers/registry.rs 是静态模型目录。通用 plugin auth 应放在 auth 层，注册时再关联 provider ID，不能绕过 ValidatedRoute 写入任意目的地。
- 协议与宿主落点：在 src/protocol.rs 为 methods/authorize/callback 增加带 provider_id、方法索引、输入/代码的显式 request/response 类型和有限长度校验；在 src/headless.rs 增加 dispatch、结构化错误与事件输出。当前 StdioRequest/Command 没有 ProviderAuth 命令，不能只复用 Prompt 字段。交互事件可参照 src/auth::AuthInteraction，而不是把 OAuth code 当作 approval.rs 的工具批准。
- 取消、恢复、并发差异：用 src/runtime.rs::CancellationToken 与 auth::LoginControl 的 deadline/send budget 包住异步授权；每个授权产生不可猜的 LoginFlowId，状态放在 Arc<Mutex<HashMap<LoginFlowId, PendingAuth>>>，callback 原子地 take/remove，限制同 provider 并发或明确 flow 绑定。TS 当前按 provider 覆盖 pending、允许重复 callback；Rust 实现应把这个差异作为安全修正并写测试。
- 持久化与核心连接：成功 OAuth/API 结果调用现有 credential store 的受控 mutation，之后由 src/core.rs/backend 的 AuthBinding 选择凭据并由 src/providers/connection.rs 重新校验 destination/header scope。不要把 raw key/access token 放进 src/session.rs JSONL；session 只可记录成功/失败/取消/不确定的操作摘要。未知网络结果沿 session.rs 的 operation recovery 规则要求显式新 operation retry。
- 工具与审批边界：src/tool_runtime.rs 只处理 provider tool batch 与取消；OAuth 授权不是 tool approval。若宿主要显示 prompt，可复用 src/approval.rs 的可取消等待模式，但必须保留独立的 auth flow、provider identity 和 credential commit 审计。
- 可执行验证差异清单：实现后应测试（a）未知 provider/越界 method 返回稳定 ProviderAuthError；（b）API 方法不产生 OAuth flow；（c）text validate、code 缺失、失败 callback、key/OAuth 两种 commit；（d）同 provider 两个 flow 互不覆盖，重复 callback 被拒绝；（e）取消/超时停止轮询且不会写入 credential；（f）store 冲突/commit uncertain 不触发隐式重试；（g）Codex 固定 endpoint 与 AuthHeaderPolicy::Codex 仍由 providers/codex.rs/connection.rs 约束。
未决: - AuthOAuthResult、Hooks["auth"] 的完整联合类型不在本文件内，无法仅凭本文件确认 success 结果是否保证 key/refresh 二选一、metadata 的精确 schema，以及 method.authorize/callback 的 Promise 是否可取消。
- Auth.Service.set 的实际存储位置、加密/冲突策略由 src/auth 之外的实现决定；本文件只确认调用形状。
- 对缺失 provider、越界 method、Schema 解码失败和 Promise rejection，Effect 运行时究竟以 typed failure 还是 defect 暴露，需结合 Effect 版本实现或运行时测试确认。

## OC-007 packages/opencode/src/provider/error.ts
- 建议落点为新增 `src/providers/error.rs`（或 `src/backend/error.rs`，由 `src/providers/mod.rs` 导出），定义 `HeaderTimeoutError`、`ResponseStreamError`、`ParsedStreamError`、`ParsedApiCallError` 及 `parse_stream_error`/`parse_api_call_error`。Rust 联合可用 `enum`，字段用 `String`、`Option<u16>`、`BTreeMap<String, String>`，并派生 `Debug`；若要跨协议输出再派生 `Serialize`。
- `src/providers/mod.rs`、`src/providers/connection.rs` 和各 `src/providers/**` 定义了 provider ID、协议、路由和能力；它们可提供 `provider_id` 与最终 URL。OpenCode 的 `providerID.startsWith("openai")` 应落为显式 `provider.starts_with("openai")` 分支；路由的 `route_digest`/`identity_scope` 可作为 `metadata` 的安全内部替代，避免把凭证写入错误文本。
- 现有 `src/backend.rs` 的 `BackendError` 已有 `Configuration`、`Transport`、`HttpStatus`、`InvalidResponse`、`Cancelled`、`DeadlineExceeded`、`Authentication` 与 `is_retryable()`。建议增加 `ContextOverflow { message, response_body }`，并让 HTTP 原始错误在映射前保留可选 headers/body/url；当前 `HttpStatus` 只保留 status 与 retry-after，无法完整承载本文件的 `ParsedAPICallError`。现有 `ProviderEvent::Failed`/`Warning` 可承载清洗后的展示消息，但不应替代结构化错误。
- `src/protocols/**` 的流式响应解码是 `parseStreamError` 的最佳调用点：在 SSE/JSON 错误帧完成后先构造结构化错误，再产生 `ProviderEvent::Failed`。`src/backend.rs` 的请求控制和重试循环已有“取消前检查、已发出事件后不自动重试、退避和熔断”语义，接入时应保留这些条件；只有在尚未发出业务事件且错误的 `is_retryable()` 为真时才允许重试。
- `src/core.rs` 的 `AgentError::Backend` 与稳定 `code()` 应保留 `backend_context_overflow`/`backend_api_error` 等机器码，不要把上下文溢出降成普通字符串；一个失败 turn 的 session 变更仍遵循 Agent 现有 admission 规则。
- `src/headless.rs` 将 `AgentEvent`/`ProviderEvent` 投影为 JSONL；当前 `provider_event_block` 对 `Failed` 固定 `retryable: true`。应改为读取结构化 `is_retryable` 和 `BackendError::code()`，并通过 `StdioResponse.error_code` 输出稳定码；错误响应要沿用其重放/请求 ID 账本，避免客户端重试造成重复 turn。
- `src/runtime.rs` 的 `CancellationToken` 是取消映射：在每个流块、头部等待和重试退避前调用 `is_cancelled()`；`Cancelled`、`DeadlineExceeded` 不应进入普通 API 重试。其 `JobOutcome::Cancelled` 说明取消不是回滚，因此 provider 已产生的外部副作用不能由错误解析器假设已撤销。
- `src/session.rs` 负责 JSONL 会话恢复和未知结果审计。provider 错误可作为受限事件/turn 错误持久化，但不要无界写入原始 `responseBody` 或认证 HTML；若请求在取消/断线时结果未知，应沿用 session 的显式 retry/abandon 规则，而不是依据 `isRetryable` 自动重放。
- `src/tool_runtime.rs` 的错误与取消边界针对工具副作用，不应把 provider `api_error` 当作工具可重试；provider 失败发生在工具调用之前可映射为 agent/backend error，工具已执行后的不确定结果必须遵循现有 operation journal 和显式 retry 约束。
- `src/protocol.rs` 已有 `ProtocolError`、`StdioResponse.error`/`error_code` 和 JSONL 编码。建议新增稳定错误码映射（至少 `context_overflow`、`provider_api_error`、`provider_header_timeout`、`provider_stream_error`），保持 `error` 人类可读、`error_code` 机器可分支；不要把 `responseBody` 直接放进公共协议。
- `src/approval.rs` 没有 provider 解析职责。其等待取消、首次响应获胜和持久化确认语义应保持不变：provider 超时/流错误不能伪造 approval decision；若取消与审批响应竞态，仍由 coordinator 的 cancellation epoch 决定，错误
未决: 1. `APICallError` 的 `responseBody`、`statusCode`、`responseHeaders` 和 `url` 在当前安装的 `ai` 版本中是否始终具有本文假设的类型与可选性，源文件自身未声明。
2. `isContextOverflow` 的具体匹配规则不在本文件中，无法仅凭本源确认哪些自然语言消息会被判定为上下文溢出。
3. `ProviderV2.ID` 的实际字符串规范及 `startsWith("openai")` 是否覆盖所有 OpenAI 兼容 provider，需要查看 provider 注册表才能完全确认；本文件只给出调用方式。

## OC-008 packages/opencode/src/provider/model-status.ts
最接近的落点是 `src/providers/registry.rs`：现有 `ModelDescriptor`（`L58-L68`）承载 `(provider, id)`、能力、上下文预算、价格和来源；`ModelRegistry::resolve`（`L366-L373`）按精确身份返回描述符，未知模型走 `unknown` 保守描述（`L384-L429`）。建议在该模块新增：

- `#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)] #[serde(rename_all = "lowercase")] pub enum ModelStatus { Alpha, Beta, Deprecated, Active }`，并在 `ModelDescriptor` 增加 `status: ModelStatus`。不要用 `Default` 静默填充，除非产品明确规定新模型状态；这样才能保留 TS schema“无默认值”的边界。
- 另建三值 `CatalogModelStatus { Alpha, Beta, Deprecated }`，用于外部 catalog 解码，再显式映射到规范化 `ModelStatus`；不要把 `CatalogModelStatus` 与 `ModelStatus` 合并，否则会重新允许 catalog 直接声明 `active`。
- 在 `ModelRegistry::with_overrides`（`L291-L363`）为 override 的 status 做严格 serde 解码与字段校验，并把 status 纳入 `ModelDescriptor::digest`（`L112-L118`），保证已选模型的元数据变化能被发现。

现有对照和差异清单：

- `src/core.rs` 的 `Agent::model_status`（`L1284-L1291`）目前返回 `active` 描述符、能力、预算、catalog、`registry_bound` 和 reasoning effort；建议把 `ModelDescriptor.status` 序列化进该 JSON，而不要把生命周期状态混同 `AgentPhase` 或运行请求状态。`restore_model_selection` 的 digest 校验（`L1262-L1274`）可验证状态变更是否需要显式重新选择。
- `src/headless.rs` 的异步请求/事件缓冲（`L3912-L3935`）以及 provider 事件投影（`L4339-L4365`）处理的是流式工作；模型生命周期状态应作为 status 快照字段，不应伪装成 `ProviderEvent`。headless 的 `status` 命令解析在 `src/protocol.rs:L415-L424`，可复用来返回快照。
- `src/protocol.rs` 的 `Command::Status` 是传输命令，不是模型状态枚举；若对外发送 status，给响应增加可序列化的 `model_status` 字段，并对未知字符串返回协议错误。协议已有有界输入，不能用它替代 schema 级严格枚举。
- `src/session.rs` 的 `OperationKind`/`OperationOutcome`（`L49-L65`）描述 provider、tool、compaction 操作的成功/失败/取消/中断；这些不是 `alpha/beta/deprecated/active`，不能复用。若要恢复模型元数据，应将 status 纳入选择事件或描述符 digest，而非记录成 operation outcome。
- `src/runtime.rs` 的 `JobOutcome` 包含 `Succeeded/Failed/Cancelled/Panicked`，并由 `CancellationToken` 协作取消；它是作业调度语义，与本文件无取消语义。模型 status 读取应保持纯函数，不因 job cancel 改写。
- `src/tool_runtime.rs` 的 tool batch 有并行/串行、批量上限和取消轮询；模型状态校验不应放进 tool 执行或 approval 流程，避免把生命周期标签误当副作用授权。
- `src/approval.rs` 的 `ApprovalDecision` 是 `Allow/Deny`，与模型状态集合完全不同；不得把 `active` 映射为 `Allow`，也不得让 `deprecated` 自动触发审批。
- `src/providers/mod.rs` 的 `ProviderDefinition`/`RouteRule`（约 `L146-L180`）以及 `openai.rs`、`anthropic.rs`、`deepseek.rs`、`codex.rs`、`google.rs` 中的静态 provider 路由主要声明协议、鉴权和能力；建议保持 provider 路由无状态，把模型 status 放在 `registry::ModelDescriptor`。`connection.rs` 只负责连接与能力交集，不能替代生命周期字段；`registry.rs` 才是 catalog/override 的集中验证点。

建议的可验证实现步骤是：先给两个 Rust enum 加 serde 单元测试，覆盖四值/三值边界和未知值；再让 `model_status()` 返回 status；最后测试修改 status 后 descriptor digest 改变、旧 session 恢复被拒绝并要求显式选择。这样分别覆盖运行时解码、公共 JSON 投影和持久化恢复三层行为。
未决: - `model-status.ts` 只转导出 `CatalogModelStatus`，无法单独确认 catalog 状态的业务转移规则，以及何时允许 `deprecated` 模型继续发送请求。
- 无法从本文件确认 `active` 是否代表“可选”、 “已验证”或仅表示 provider 侧规范化标签；zenpi 映射应把该语义留给上层策略配置。
- 本文件没有持久化协议，无法确认状态是否应写入 session；只能依据 zenpi 现有 descriptor digest 机制提出可验证落点。

## OC-009 packages/opencode/src/provider/provider.ts
- **provider catalog/模型元数据**：`src/providers/registry.rs` 的 `ModelDescriptor`、`effective_capabilities`、`validate_reasoning`、`context_budget`、`digest` 已对应 `Model` 的能力、限制、推理与价格核心语义（L58-L119）。建议新增/扩展 `ProviderCatalog` 保存 `Info` 等价字段（来源、env、options、variants），并实现 `from_models_dev_model`、`cost`、`model_suggestions`；当前 registry 是静态、有界 catalog，源是动态 models.dev+config/plugin，差异是必须显式处理刷新、status alpha/deprecated 和未知 provider。
- **路由与 SDK 选择**：`src/providers/mod.rs` 的 `Protocol`、`RouteRule`、`ProviderDefinition`、`get_provider_definition` 与各 provider 文件把 openai/anthropic/google/deepseek/codex 的端点、header、能力、option policy 固化（L13-L20、L103-L182、L152-L224）；`src/providers/connection.rs::resolve_connection/resolve_model_route` 验证 model/provider、协议、URL、auth 和 streaming（L142-L240）。这对应源 `BUNDLED_PROVIDERS`、`custom` 与 `resolveSDK` 的“provider+model→实际 API”。建议落点为 `providers::registry::ProviderCatalog::resolve_model`、`providers::connection::prepare_route` 与 `backend::Backend::complete/stream`，保留 `ValidatedRoute` 不可被 encoder 替换。差异：源允许动态安装任意 npm `create*`，Rust 当前仅静态定义；源支持大量第三方 loader（Bedrock、Vertex、Cloudflare、Snowflake、GitLab），Rust 需明确新增 `ProviderDefinition` 或拒绝未知协议，不能用名称启发式代替。
- **请求/流式超时与取消**：`src/runtime.rs::CancellationToken` 是协作取消，要求 job 在 chunk/retry 边界检查，shutdown 有 grace（L45-L115、L300-L430）；`src/headless.rs` 有有界 provider/agent mailbox，慢 stdout 时丢弃并报告（L39-L51、L941-L1041）。建议在 `backend` fetch adapter 中加入 header/chunk deadline，映射 `ProviderError.HeaderTimeoutError`/`ResponseStreamError` 到 `BackendError::Transport`，每个 SSE chunk 调 `CancellationToken::is_cancelled`。差异：源把多种 `AbortSignal` 合并且允许 `false` 禁用，Rust 需定义 `Option<Duration>`/`None` 的等价配置并保证关闭时 join，不可 detach 未知 provider 工作。
- **会话、默认模型与恢复**：`src/session.rs` 是 append-only JSONL、interrupted operation 需显式 retry/abandon（L1422-L1585），`src/core.rs::Agent::new` 会读取恢复状态并发出“requires explicit retry”，模型选择通过 `set_model` 验证并写 `model_selected` 事件（L598-L640、L1167-L1188）；这比源仅读取 `model.json` recent 更强。建议实现 `ProviderSelectionStore` 写入 session event，`default_model` 先配置、再 recent、再排序；恢复时校验 provider/model digest，避免源的无版本 recent 被静默接受。`src/headless.rs` 的 reconnect journal、request replay 和 bounded terminal cache（L155-L256、L405-L497）可承接 provider 初始化/流式事件的重放，但源本身没有同等级幂等协议。
- **工具与审批副作用**：源 provider 只把 SDK/tool call 暴露给上层；zenpi `src/tool_runtime.rs` 要求 prepare 后再执行、取消前停止新调用、worker join 且未知副作用进入恢复（L130-L341），`src/approval.rs` 以 `ApprovalCoordinator` 等待 host 决策、支持取消和持久化接受记录（L122-L241、L367-L405）。建议 `ProviderRequestScope` 在 `src/core.rs` 中绑定 route digest、identity scope、policy/lease（L521-L555），任何 provider 请求都复用；差异是 zenpi 的审批/worker gate 比源更严格，应保持而不是把 provider `aut
未决: 1. `ModelsDev.Service.get()` 的 catalog 快照是否会在同一进程热刷新，源文件只展示一次 layer 初始化，无法确认刷新策略（L1403-L1406）。
2. `getLanguage` 首次并发 miss 是否由 Effect runtime 在实例级串行化，代码本身没有显式锁，无法确认是否可能重复 SDK 构造（L1896-L1918）。
3. `BUNDLED_PROVIDERS` 动态 import 工厂返回值及第三方 `create*` 导出唯一性由 npm 包保证，源未验证多导出或工厂返回 Promise 的边界（L113-L140、L1842-L1862）。
4. `defaultModelIDs` 在空模型 provider 上的异常是否由调用方先过滤，函数本身没有空数组防护（L1137-L1139）。

## OC-010 packages/opencode/src/provider/transform.ts
- 模型与 provider 入口：zenpi 的 `src/providers/registry.rs` 已有 `ModelDescriptor`、`ReasoningLevel`、capabilities/limits；`src/providers/mod.rs` 有 `Protocol`、`OptionPolicy`；`src/providers/connection.rs` 负责 route 校验。因此可新增 `src/providers/transform.rs`，定义 `ModelTransform`、`ProviderOptions`、`ReasoningVariant`，由 `ValidatedRoute::model()` 和 protocol 选择。
- 消息映射：在 `src/core.rs` 的 provider turn 准备处调用 `transform_messages(&mut [ModelMessage], &ModelDescriptor, &TransformOptions)`；消息类型可放 `src/protocol.rs` 或新模块，复刻 surrogate 清洗、空内容过滤、toolCallId 规范化、interleaved reasoning、unsupported modality 替换。需要明确 Rust 借用规则，避免 JS 原地别名；建议输入 owned `Vec`，输出新 Vec，并保留 `TransformWarning`。
- 请求选项与变体：将 `temperature/top_p/top_k/max_output_tokens` 落在 `src/providers/mod.rs::OptionPolicy` 扩展字段；将 `options`、`smallOptions`、`providerOptions`、`reasoningEffort/reasoningBudget` 落在新 `transform.rs`，以 `serde_json::Map` 生成 wire JSON。OpenAI/Azure/Mantle 的 `forceReasoning`、Responses encrypted include、Gateway slug 路由必须在 provider encoder（`src/providers/openai.rs`、`codex.rs`）最终确认。
- Schema：新模块提供 `sanitize_openai_schema`、`sanitize_gemini_schema`、`sanitize_moonshot_schema`，输入 `serde_json::Value`，在 `src/tool_runtime.rs` 调用工具注册/发送前执行；与 `src/providers/registry.rs` 的 `structured_output` capability 联动，未支持时返回可诊断错误或降级文本。
- 生命周期与取消：transform 本身保持纯同步；网络请求/超时由 `src/runtime.rs` 的 `CancellationToken` 和 provider worker 检查，工具副作用由 `src/tool_runtime.rs`、`src/approval.rs` 继续控制。重试不可在 transform 内自动做，重试请求必须沿用 `src/session.rs` 的 operation journal/新 operation ID 规则。
- 持久化与协议：sessionID/cache key 只作为请求字段，不直接写盘；会话记录、恢复和未知 provider/tool 状态仍由 `src/session.rs`、`src/headless.rs`、`src/protocol.rs` 管理。建议在 protocol 事件中记录 transform warnings（如 unsupported modality、schema lowering），供 headless replay 核对。
- 差异清单（可执行）：(1) zenpi 当前 provider registry 是静态能力集合，缺少 npm SDK namespace 与模型发布日期，需在 `ModelDescriptor` 增加 `api_id`、`release_date`、`sdk_key`；(2) 当前 provider 文件以 OpenAI/Anthropic/Google/DeepSeek/Codex 方言编码，需逐项补齐 Bedrock/Gateway/Copilot/Mistral/SAP/Alibaba 等分支；(3) Rust 需要显式区分 message-level/content-level cache；(4) Rust schema lowering 必须测试 bool schema、type array、$ref sibling、tuple items；(5) 将变体支持与 registry 的 `ReasoningLevel` 做交集，禁止 JS 中“返回空对象但 capability=true”的隐式情况；(6) 使用 property-based/快照测试验证 32,000、31,999、output-1 等边界。
未决: 无。源文件可确定上述转换和边界；具体 provider SDK 最终如何消费字段、以及 zenpi 当前请求编码器是否已覆盖所有 npm 分支，需要在后续读取各 provider encoder 时再核对，但不影响本文件行为结论。

## OC-012 packages/opencode/src/session/instruction.ts
对照现有 zenpi：

- src/core.rs:L4050-L4137 已把技能、persona、resource 和 selected skills 合成 instructions，按 instructions.len().div_ceil(4) 扣减 context budget，再通过 CompletionRequest::with_instructions 注入 provider。建议新增 src/instruction.rs 的 InstructionService，在这里合并 system() 文本并计入同一个 token 预算；保留 core.rs:L3981-L4000、L4063-L4065 的取消轮询。
- src/backend.rs:L176-L213 已有 CompletionRequest.instructions: Option<&str>，不需要改变请求模型。provider wire 层已经正确承载：Chat Completions 把文本插入首个 system message（src/protocols/chat.rs:L759-L762），Responses 写入 instructions（src/protocols/responses.rs:L88-L90），Anthropic 和 Google 分别生成 system text（src/protocols/anthropic.rs:L323-L328、src/protocols/google.rs:L280-L285）。src/providers/** 的 connection.rs:L166-L220 只负责 provider/model 路由，应保持与 instruction 搜索解耦。
- src/headless.rs:L1148-L1210 的 owner_workspace 与 resolve_workspace_path_at 提供 workspace 所有权、canonicalize、拒绝绝对路径/父遍历/符号链接。建议 InstructionService 接受已确认的 workspace root；对来自配置的 ~/、绝对路径是否保留兼容性要单独开关。若复用该安全策略，应把 TypeScript 的绝对 instruction glob 行为记录为有意差异，并用组件级 Path::starts_with 避免字符串前缀误判。
- src/session.rs:L1205-L1219 的 append-only journal 与 L1410-L1521 的 operation recovery 明确“中断操作不自动重试”。instruction claims 不应写入 operation marker；建议作为 Agent 或 workspace session 生命周期内的 HashMap<MessageId, HashSet<PathBuf>>，对应 TS 的 InstanceState。若要跨重启去重，才另行设计事件类型，当前源没有此要求。
- src/tool_runtime.rs:L1-L7 将调用者责任限定为 approval、journal、持久化和重试；指令读取是只读准备步骤，不应进入 ToolRegistry、ApprovalCoordinator 或 side-effect gate。其 L313-L328、L357-L425 的取消传播可作为读取/附加前的统一取消回调。
- src/runtime.rs:L49-L99 的 CancellationToken 是合作式取消，L318-L350 提供非阻塞 cancel/shutdown。建议远程 fetch 使用同一 token；5 秒 timeout 触发空结果，不能把取消误报为成功附加。若并行读取，使用 bounded worker 或已有 runtime 队列实现 8/4 上限，结果按输入顺序重排。
- src/protocol.rs:L1-L24 与 StdioRequest:L152-L209 是版本化 JSONL 控制协议，已有 text/path/attachments，没有 instruction 专用字段。建议先从 workspace/config 装载，不把 AGENTS.md 内容直接扩展为客户端可注入的协议字段；如必须支持远程配置，再新增经校验的配置动作而非复用 prompt 文本。
- src/approval.rs:L122-L187 把 approval 保持为工具副作用的同步 rendezvous，且取消可中断等待。instruction 文件和 HTTP GET 属于上下文读取，不应请求 approval；若产品政策把网络读取视为外部副作用，则需另加明确 policy，而不能从 provider 输出推断。
- 建议的可执行接口：InstructionConfig { global_config, home, instructions, disable_claude_code_prompt, disable_project_config }；InstructionService::system_paths(&WorkspaceContext) -> Result<Vec<PathBuf>, InstructionError>；system() -> Result<Vec<String>, _>；find(&Path) -> Option<PathBuf>；resolve(messages, filepath, message_id) -> Vec<ResolvedInstruction>；以及纯函数 extract_loaded。测试应覆盖层级优先级、Set 去重、claims/clear、已读 metadata、HTTP timeout/retry、并发上限和路径边界。
- 关键差异清单：TypeScript 的 FS/HTTP 错误多数变为空内容，Rust 需决定是否完全兼容；TypeScript 支持绝对配置 glob 与 ~/，headless Rust 默认拒绝绝对/符号链
未决: - withTransientReadRetry 的具体重试次数、退避和哪些状态码属于 transient 不在本文件中定义。
- FSUtil.globUp/findUp 遇到符号链接、权限错误和 worktree 边界时的精确语义无法仅由本文件确认。
- InstanceState.make 对并发 resolve 调用的原子性未在本文件展示；源码只显示共享 Map 的读写，没有锁或去重竞态测试。
- 配置 instructions 的来源、刷新时机和 cfg.get() 的失败类型由 Config.Service 决定，本文件未展开。

## OC-013 packages/opencode/src/session/llm.ts
现有 zenpi 已有可复用边界，但语义粒度不同：

- provider 抽象落在 `src/backend.rs`：`CompletionRequest`（约 L182-L234）承载 turns/model/tools/instructions/token limit，`ProviderEvent`（L236-L294）提供 TextDelta、ReasoningDelta、ToolCallDelta/Done、Usage、Completed、Failed；`Backend::complete_with_control`（L492-L531）提供同步完成与取消检查。这对应本文件的统一 `LLMEvent` 流，但当前是 callback sink + 最终 `Completion`，没有 native/AI SDK 双 runtime。
- 会话编排落在 `src/core.rs`：`run_active_turn_cancelable_with_events`（L3674-L3787）建立 provider operation marker、检查取消、调用 `complete_with_tools`；`complete_with_tools`（L3929 起）循环 provider 与工具调用，最多 8 次迭代并持久化操作结果。建议新增 `llm_runtime.rs) 或在 core 前增加 `LlmRequest`/ `LlmEventStream`，把准备、runtime 选择、事件规范化从 core 抽出。
- 工具执行映射到 `src/tool_runtime.rs` 的 `execute_tool_batch`（L168-L347）和 `dispatch`（L349-L436），已有批量上限、并行度、取消、panic 转错误、审批后 preview 校验；对应 workflow `toolExecutor` 时应复用 registry，而不是直接执行任意 JSON handler。建议类型为 `ToolExecutor) trait：输入 `ToolCall{id,name,args}`、`CancellationToken)，输出 `ToolResult)，并把未知工具映射为稳定错误。
- 取消/并发映射到 `src/runtime.rs`：`CancellationToken`（L49-L99）是可克隆、幂等、合作式 token；`BackgroundRunner`（L236-L450）有有界队列、FIFO follow-up、取消事件、shutdown grace、非合作任务 detach。建议 `StreamRequest.abort` 对应 token clone，provider 每个 chunk/重试前检查 `is_cancelled`；native 与 fallback 选择应在一个 job 内完成，并发 provider metadata 获取可用独立线程或顺序读取，不能引入无界任务。
- 权限与 workflow approval 映射到 `src/approval.rs`：`ApprovalRequest/Response`（L41-L125）、`ApprovalCoordinator::request_response`（L181-L245）、`persist_accepted/mark_persisted`（L370-L403）已支持等待、取消、先持久化后放行和 first decision wins。建议把 `workflow_tool_approval` 作为 ToolOrigin/permission kind，按 `name:title` 生成 patterns，并用 session-local `BTreeSet<String>) 实现已批准工具缓存；不要照搬 TypeScript 中忽略 reply 的 listener。
- 持久化/恢复映射到 `src/session.rs`：`begin_operation_with_key`（L1424-L1451）在 dispatch 前写 marker，`finish_operation`（L1453-L1474）写 terminal，`operation_recovery`（L1492-L1522）保留 unknown/interrupted，明确禁止自动重试。provider 请求建议 kind=Provider、tool kind=Tool、idempotency key 由 turn/session 生成；取消、transport/partial response 分别映射 Cancelled/UnknownOutcome。
- wire/UI 事件映射到 `src/protocol.rs` 与 `src/headless.rs`：`ProviderEvent) 可封装为 `StdioEvent)，`Command::Cancel)/`Shutdown`（protocol L214-L265、L415-L425）驱动 runtime token；headless 的异步 provider/agent mailbox（headless L806-L1065）已有有界事件缓冲和丢弃报告，可承接流式 LLMEvent。
- provider 路由映射到 `src/providers/**`：`providers::Protocol/Dialect/AuthHeaderPolicy` 及 registry 负责 OpenAI Responses/Chat、Anthropic、Google、Codex、DeepSeek 路由；新增 `NativeRuntime) 时应把能力判断（provider、协议、auth、API key）落在 registry/connection，避免在 core 按字符串硬编码。现有 backend 已有 `BackendError::is_retryable`（backend.rs L450-L472）和 provider retry/backoff（backend.rs 
未决: - `LLMRequestPrep.prepare`、`LLMNativeRuntime.stream`、`LLMAISDK.toLLMEvents` 的完整 provider 特殊字段和事件映射不在本文件内，需结合各自源码确认最终 wire schema。
- workflow `Permission.Event.Replied` listener 中 `void data.reply` 与 `perm.ask` 的确切等待契约无法仅由本文件确认。
- native runtime unsupported 的所有具体原因集合由 `native-runtime.ts` 决定，本文件只记录并 fallback。
- 上层如何把 `LLMEvent` 持久化成 MessageV2、如何判定网络错误可重试，由 `session/processor.ts` 等调用方决定，本文件没有定义。

## OC-014 packages/opencode/src/session/llm/ai-sdk.ts
- `src/backend.rs` 的 `ProviderEvent`（`ProviderEvent::TextDelta/ReasoningDelta/TextDone/ToolCallDelta/ToolCallDone/Usage/Completed/Failed`）是最接近的共享事件层；`src/core.rs` 的 `AgentEvent::Provider` 将 provider 事件按 `turn_id` 关联并送入 session。建议新增 `src/llm_adapter.rs`（或 `src/ai_sdk_adapter.rs`）中的 `AdapterState`、`ProviderChunk` 和 `to_provider_events(state, chunk) -> Result<Vec<ProviderEvent>, AdapterError>`，明确实现文本/推理 ID、工具名映射、usage 归一化和 `network_error` 错误。
- `src/core.rs`（`run_active_turn_cancelable_with_events`）应成为事件消费边界：provider sink 顺序调用映射器，完成后再持久化 assistant turn；映射器只产生事件，不绕过 core 的 admission、工具循环或错误状态。现有 `AgentEvent` 没有 step/block start/end 全套语义，若需要 1:1 保真，应扩展 `ProviderEvent` 或在 `AgentEvent::Provider` 的 `event` 中增加 `StepStart/StepFinish/TextStart/TextEnd/ReasoningStart/ReasoningEnd/ToolInputStart/ToolInputDelta/ToolInputEnd/ToolResult/ToolError`。
- `src/session.rs` 负责 append-only journal、`OperationKind::Provider`、`OperationOutcome` 和中断恢复；它可记录适配器失败、完成和未知结果，但不应把 `adapterState` 当作持久化状态。`network_error` 的重试必须新建 operation ID，并遵守已有 interrupted/unknown outcome 的显式确认规则。
- `src/runtime.rs` 的 `CancellationToken` 是取消落点；provider/backend 已在流块之间轮询取消。Rust 映射器本身只需在每个 chunk 边界检查 token（或让 backend 返回 `BackendError::Cancelled`），不要尝试线程强杀；取消不是副作用回滚。
- `src/tool_runtime.rs` 执行真实工具批次、审批后 dispatch、结果持久化和取消协调。适配器中的 `tool-input-*`/`tool-call` 只能描述 provider 输出，不能直接执行；应把完整 `ToolCall` 交给 core/tool runtime，沿用批次顺序、审批与 unknown-outcome 规则。
- `src/protocol.rs` 定义 headless JSONL 的版本、相关 ID、取消、checkpoint/mailbox 等线协议；若要向客户端暴露细粒度 LLM 事件，应为 `ProviderEvent` 增加可序列化字段并由 protocol 投影，不能让适配器自行写 stdout。
- `src/headless.rs` 管理 bounded mailbox、重放、请求指纹、终端响应和 reconnect journal。它适合承载适配器产生的有序事件与 terminal error，但不应保存临时 block ID state；重连应依靠 session/replay 序列，而非重新猜测 `text-0`。
- `src/approval.rs` 是主机审批协调器；AI SDK 的 `tool-approval-request` 在本源适配器中被忽略，zenpi 则应继续让 host 显式决定 `ApprovalDecision`，适配器不能从 provider 请求推断 Allow。
- `src/providers/**`（`connection.rs`、`registry.rs` 及 `openai.rs/anthropic.rs/codex.rs/google.rs/deepseek.rs`）负责路由、能力和原生 streaming 解码；建议把各协议 decoder 输出统一到 `ProviderChunk`，再经上述 adapter 归一化。`Usage` 当前只有 `u64 input/output/total`，与源的 reasoning/cache read/cache write、`ProviderMetadata`、Copilot `totalNanoAiu` 不等价，应扩展可选 usage/metadata，或在 `Completion.annotations`/事件 metadata 中保存，并对负数/非有限外部数值采用拒绝或丢弃策略。

差异清单：zenpi 当前没有 AI SDK 的 `fullStream` 事件联合与 `Effect` 错误语义；没有细粒度 text/reasoning block ID 生命周期；工具事件是 `ToolCall`/`ToolCallDone` 而非输入增量、结果、错误三套事件；provider usage 缺少 cache/reasoning/Copilot 字段；`ProviderEvent::Failed` 是消息型事件而源的 `error` 是 effect 失败；zenpi 已有更严格的取消、审批、operation journal 和 headless replay，需要在映射层保持这些约束。
未决: 无法从本文件确认 `ProviderMetadata`、`FinishReason`、`ToolResultValue.make` 的完整 schema，以及上层收到 `ProviderError.ResponseStreamError` 后具体重试次数、退避和 session 持久化时机；这些行为需查 `@opencode-ai/llm` 定义与 `src/session/llm.ts`/processor。除此之外，适配器分支、默认值、状态清理和忽略事件均可由本文件及上述测试直接确认。

## OC-015 packages/opencode/src/session/llm/native-request.ts
建议新增纯函数模块 `src/native_request.rs`（或放入 `backend.rs` 的独立 `native` 子模块），定义 `NativeRequestInput`、`NativeMessage`/`NativeContentPart`、`NativeToolDefinition` 和 `lower_request(&NativeRequestInput) -> Result<NativeRequest, BackendError>`；测试应使用固定 JSON fixture 验证确定性。这一层只做 lowering，不能取得 `ApprovalCoordinator` 或执行工具。

- `RequestInput.model` 对应 `providers::registry::ModelDescriptor` 加 `providers::connection::ProviderConnection`；`model()` 的 npm 路由应落到 `providers::Protocol/EndpointOperation/resolve_connection`。现有 `src/providers/**` 已有 OpenAI Responses/Chat、Anthropic、Google、custom、Codex、DeepSeek 路由和认证策略，但没有源文件中的 Azure、Amazon Bedrock、OpenRouter 专用分支，需明确映射为 unsupported 或新增 definition，并为缺失 `base_url` 保持可验证错误。
- `messages()` 的 system 分离和结构化 parts 应落到 `src/backend.rs` 的 `CompletionRequest`/协议编码边界；当前 `src/core.rs` 在 `L4000-L4225` 以 `Turn { role, content: String }` 构造 `CompletionRequest`，缺少 reasoning/media/provider metadata。建议扩展 backend 内部 `MessagePart`（text/media/reasoning/tool-call/tool-result）并由各 `src/protocols/**` codec 编码，system 文本可暂时合并为 `instructions`。
- `tools()` 应从 `src/tools.rs::ToolDefinition`（含 `side_effect` 和 JSON `input_schema`）生成 provider-facing definitions；执行仍交给 `src/tool_runtime.rs`，该模块已经区分顺序/并行、审批前置、结果相关性和取消后的不确定副作用。不能把 native adapter 的工具定义当作授权。
- `src/core.rs` 的 `RequestControl`、`src/runtime.rs::CancellationToken` 和 backend 的 deadline/重试循环提供本文件没有的取消、超时、退避与 circuit breaker；lowering 只传递字段，不重复实现这些语义。`src/protocol.rs` 负责 JSONL `cancel` 等命令，`src/headless.rs` 负责有界事件/终端重放，均不是 request conversion 层。
- `src/session.rs` 是 append-only journal，`src/approval.rs` 持久化审批和撤销 epoch；它们应继续拥有恢复、审批、凭据/副作用审计。若要恢复 provider continuation metadata，应把受限 JSON metadata 放入 `Turn.metadata`/session record，再由 `native_request` 映射，禁止在 adapter 内写盘。

可执行差异清单：补齐结构化消息与 `providerMetadata` 的 Rust 类型；实现 file data 仅接受 `String`/`Vec<u8>` 且默认 MIME 的边界；实现工具 schema 的空 object 默认；为每种 Protocol 建立 provider package/URL 矩阵测试；验证 `headers` 合并覆盖规则和 generation 全空时为 `None`；增加未知 provider、缺 URL、非法 part 的 typed `BackendError` 测试。
未决: - `@opencode-ai/llm` 各 `configure/model` 实现如何进一步解释 `limits`、`providerMetadata` 以及 `toolChoice`，仅凭本文件无法确认，需查该包实现。
- `isRecord` 的精确定义来自 `@opencode-ai/tui/util/record`，本文件未展开；数组是否被排除需以该实现为准。

## OC-016 packages/opencode/src/session/llm/native-runtime.ts
- **请求与 provider 路由：** 建议在 `src/backend.rs` 的 `CompletionRequest`、`RequestControl`、`ProviderEvent`（当前定义见 `CompletionRequest` L174-L188、`ProviderEvent` L238-L276、取消/截止检查 L390-L415）上增加一个可选 native-stream 适配层；`Backend::complete_with_request_control` L495-L507 已有逐事件 sink，最接近 `Stream.Stream<LLMEvent>`。差异是 zenpi 当前 `Backend` 是同步 trait，返回 `Completion` 并通过 callback 发 `ProviderEvent`，没有 TypeScript 的可组合异步 Stream；可执行落点是用 callback 转有界 channel，再由 `runtime::BackgroundRunner` 暴露事件。
- **status 门槛：** 不要在 Rust 复制 npm 字符串白名单作为唯一能力判断。`src/providers/connection.rs` 的 `ValidatedRoute`（字段和 accessor L42-L110，能力与 streaming 校验 L288-L365）已经绑定 provider、URL、auth、`ProviderCapabilities` 和 route digest；`src/providers/registry.rs` 的 `ModelDescriptor::effective_capabilities` L70-L82 以及 `ModelRegistry::resolve` L366-L373 可实现“模型能力 ∩ wire 能力”的支持判定。建议新增 `NativeRuntimeStatus { api_key: SecretHandle/受控引用, base_url: Option<String> }` 和 `status_native(route, auth, fetch_override)`，明确 OAuth fetch override 的特例。与 TS 的差异：zenpi 还验证 destination、credential revision、streaming capability，并有 `BackendError::Configuration/Authentication`，不能只返回字符串 reason。
- **消息、headers、options：** TS 的 `ProviderTransform.message/providerOptions` 可落在现有 provider wire 编码器（`src/providers/openai.rs`、`anthropic.rs`、`google.rs`、`codex.rs`、`deepseek.rs`）和 `OpenAiWireApi`；`ValidatedRoute::protocol/dialect/options` 应作为单一翻译来源。`providerHeaders` 的字符串过滤可放在 route/header policy 层，避免把任意 JSON 头传入 transport。TS 允许调用方 headers 覆盖 provider headers；Rust 应在已验证的 `AuthHeaderPolicy` 之后应用非敏感 request headers，并保持审计字段不可覆盖。
- **工具桥：** `nativeTools` 对应 `src/tool_runtime.rs` 的 `execute_tool_batch`（入口 L168-L176，取消与 join 语义 L219-L331）和 `src/core.rs` 的 `ToolRuntime`/`AgentEvent::ToolCall`、`ToolProgress`、`ToolResult`（L283-L315）。建议新增 `NativeToolAdapter`：把 `ToolDefinition`/`ToolCall` 转为 native definition，handler 错误映射为 `ToolFailure`，将 `turn_id`、messages snapshot 和 cancellation predicate 放入 `ToolContext`。差异是 zenpi 的批处理限制最多 32 call、256 KiB，并区分 sequential/parallel；TS 使用无界 FiberSet/Queue，且单事件 dispatch 后允许重叠。
- **并发、取消与 shutdown：** `src/runtime.rs` 的 `CancellationToken` L56-L100、`BackgroundRunner::{try_cancel,try_shutdown,recv_timeout}` L309-L390 可承载 abort、关闭和事件读取；`InputBoundaryGate` L797-L910 可防止工具未结算时进入下一模型边界。实现时应把 provider callback 和工具结果都送入 bounded channel，保持“provider tool-call/finish 在先、tool-result 在后”，并在取消时让 `CancellationToken` 先置位，再 join cooperative workers。与 TS 的差异：zenpi 明确有 bounded queue、shutdown grace 和 `JobOutcome::Cancelled`，取消不回滚副作用；TS 本文件的 Queue 是 unbounded 且没有独立 shutdown grace。
- **approval 与外部副作用：** `src/approval.rs` 的 `ApprovalCoordinator`/`ApprovalPolicy`（
未决: 1. `LLMNative.request` 和 `LLMClient.stream` 对 `input.abort` 的具体 signal 传播不在本文件内，无法确认 provider HTTP 是否会因该 signal 中止。
2. `ToolRuntime.dispatch` 返回的 `dispatched.events` 完整事件集合及其对失败 cause 的内部处理在本文件未定义，只能确认会被批量放入 `results` queue。
3. `ProviderTransform.message/providerOptions` 的具体字段转换、`toDefinitions` 的 schema 序列化和 `LLMRequest.update` 是否去重工具定义均需查看相邻模块才能确认。
4. `Stream.scoped` 被下游取消时，正在运行的 provider request 和 tool fiber 的最终中断/清理顺序由 Effect 实现决定，源码未给出本文件级保证。

## OC-018 packages/opencode/src/session/message-error.ts
- `src/core.rs` 当前 `AgentError`（`L354-L400`）已有 `Backend`、`Recovery`、`Approval` 等分类，但没有消息级 provider 认证/输出长度变体。建议新增可序列化的 `MessageError` enum：`ProviderAuth { provider_id: String, message: String }`、`Unknown { message: String, reference: Option<String> }`、`OutputLength`，并在 `AgentError::code()` 中给出稳定代码；保持 admission 失败不写 turn 的既有约束（`L1-L6`）。
- `src/providers/**` 的 `AuthHeaderPolicy` 和 provider definition（`src/providers/mod.rs:L72-L100,L146-L160`）已经知道 provider ID 与认证头。各适配器（`anthropic.rs`、`openai.rs`、`codex.rs`、`deepseek.rs`、`google.rs`）应在认证失败边界统一转换为 `MessageError::ProviderAuth { provider_id, message }`，不要把 API key 写入 `message`；可验证判据是相同 HTTP 认证失败在不同 provider 下仍保留正确 `provider_id`。
- `src/protocol.rs`（`L20-L37` 的大小常量及 `L43-L55` 的 turn 模式）负责 JSONL 输入约束。建议在 `StdioResponse` 的错误 payload 中使用显式 `kind`/`data` 标签序列化上述 enum，并复用 `MAX_TEXT_BYTES` 约束 `message`；未知标签返回 `ProtocolError`，不要静默降级为成功。
- `src/headless.rs` 将 `AgentError` 转成相关响应（`HeadlessError` 在 `L539-L551`，主循环从 `L553` 起）。映射时应保留 `ProviderAuth` 的 provider ID、`OutputLength` 的稳定代码，并让 replay 使用同一终态，避免重放改变错误类别。
- `src/session.rs` 已有 append-only JSONL 记录与 `OperationOutcome::{Succeeded,Failed,Cancelled,Interrupted,UnknownOutcome}`（`L49-L95`）。消息错误可作为 turn/assistant metadata 的结构化 `error` 字段持久化；认证错误或输出超长不应被误标成 `Cancelled`，重启恢复时仍应能区分 `Failed` 与 `UnknownOutcome`。
- `src/runtime.rs` 的 `CancellationToken`（`L49-L99`）和 `JobOutcome`（`L156-L169`）只描述工作生命周期。provider 返回 `MessageError` 时应走 `JobOutcome::Failed(E)`；仅 token 被观察到才走 `Cancelled`。不要在取消竞态中把已发布的认证失败或输出超长改写为取消。
- `src/tool_runtime.rs` 的执行器声明“不重试中断调用”（文件头 `L1-L7`），因此不应把 provider 消息错误塞进工具重试逻辑。若工具输出受到消息长度限制，应在 owner 的输出压缩/持久化边界生成 `OutputLength`，并留下可审计的失败结果。
- `src/approval.rs` 的 `ApprovalError`（`L573-L589`）处理审批无效、未知请求和取消；它与 `ProviderAuth` 语义不同。认证错误不应触发自动 `ApprovalDecision`，审批等待取消仍保持 `ApprovalError::Cancelled`。

可执行差异清单：一是在 `core.rs` 定义并测试三类 Rust 错误及稳定 code；二是在所有 `providers/**` 适配器加入统一认证错误转换；三是在 `protocol.rs`/`headless.rs` 加入带 `kind/data` 的 JSONL 编解码和 replay 判据；四是在 `session.rs` 增加错误 metadata 的 round-trip 测试；五是在 `runtime.rs` 验证失败、取消、未知结果三者在竞态下不互相改写。
未决: 无法从该源文件确认：`NamedError` schema 对未知字段的精确解码策略、调用方何时把认证失败标为可重试、输出长度阈值的数值、错误 metadata 的最终持久化版本，以及 `NamedError.Unknown` 的完整字段语义；这些均应查调用方或 `NamedError` 实现后再定 Rust 兼容细节。

## OC-019 packages/opencode/src/session/message-v2.ts
建议把该文件拆成可执行、可测试的 Rust 层，而不是把所有逻辑塞进一个 `Agent` 方法：

- `src/session.rs` 已有 append-only `SessionStore`、`SessionRecord`、`Turn`、`OperationRecovery` 和 `SessionStore::records/turns`。可新增 `MessageInfo`、`MessagePart`、`WithParts` 以及 `MessageStore::page/get/parts/stream`；用 `(created_at_ms, id)` 游标替代 TS `Cursor`，serde + URL-safe base64 实现 `cursor.encode/decode`。验证点是复现 page 的 `limit+1`、反转和跨 session 过滤；不要把读取操作改成重写 JSONL。
- `src/core.rs` 的 `Turn`/agent 状态适合承载 `latest` 所需 role、parent、finish、summary 元数据；新增纯函数 `filter_compacted`、`latest`、`is_after`，以 `TurnRole` 和 typed part enum 表达 `compaction`/`subtask`。必须保留 `tail_start_id` 重排规则，并为导入 ID 非单调增加相同时间的 ID tie-breaker 测试。
- `src/tool_runtime.rs` 已有 `ToolBatchDecision`、取消和失败结果；可在 `src/message_runtime.rs`（建议新模块）实现 `to_model_messages`，将 `ToolResult` 映射为 `ToolOutput::{Text,Content,Json}`，对 pending/running 统一产生 interrupted error，对 compacted 采用固定清除文本。该模块只返回模型请求值，不直接执行工具。
- `src/runtime.rs` 的 `CancellationToken`、`BackgroundRunner` 可包住模型转换/请求；`message_runtime` 不自行创建线程或重试，只检查取消并让上层决定中断。`src/session.rs` 的 `OperationOutcome::{Cancelled,Interrupted,UnknownOutcome}` 可承载恢复判据，与 TS 的 `fromError` aborted 语义对齐。
- `src/protocol.rs` 适合定义 wire 层的 `ModelMessage`、分页 cursor 和错误 code；沿用现有 `MAX_ID_BYTES`、`MAX_TEXT_BYTES` 做输入上限。HTTP/JSONL 只负责序列化 `NotFound`、cursor 无效和 `more`，不要把 provider media 能力判断散落在协议解析中。
- `src/approval.rs` 与 `src/tool_runtime.rs` 的 side-effect/approval gate 对应 TS 工具结果的 providerExecuted 与中断安全边界：工具执行前记录 approval，工具结果转换只读取已审计结果；Rust 不应因 provider 返回的工具名自动放行副作用。
- `src/providers/mod.rs`、`src/providers/connection.rs`、`src/providers/registry.rs` 已有 provider/model identity、wire protocol 与 `ProviderCapabilities`。在 `ProviderCapabilities` 中显式增加 `tool_result_images`、`tool_result_files` 或等价能力，并实现 Anthropic/OpenAI/Bedrock/xAI/Google 的矩阵；`to_model_messages` 依据能力把不支持的 media 提取到合成 user message。`src/providers/anthropic.rs`、`google.rs`、`codex.rs` 的协议定义应提供默认能力，未知 provider 默认 false 或按 registry 明确覆盖。
- `src/headless.rs` 负责 host/命令边界，可调用 session page/stream 和 message runtime；不要在 headless 层复制 `fromError` 分支。建议在 `src/error.rs` 或新 `src/message_error.rs` 定义 `AssistantError`（`Aborted`、`Auth`、`Api{retryable}`、`ContextOverflow`、`Unknown`），把 `ECONNRESET`、解压失败、header timeout、stream error、APICallError 的解析集中到一个 `from_error`。

差异清单：TS 使用 Effect/Drizzle/AI SDK，Rust 需用 `Result`、数据库或 JSONL 投影和自有 `ModelMessage` wire 类型；TS 的 `MessageTable/PartTable` 是规范化表，zenpi 当前主要是 append-only turns，需决定是否建立内存索引或新增 durable part record；TS 允许 provider metadata 任意 JSON，Rust 应用 `serde_json::Value` 并过滤 `providerExecuted`；TS `MessageID.ascending()` 是外部 ID 生成器，Rust 要提供单调、session-scoped 的合成 ID；TS 对
未决: - `MessageTable.data`、`PartTable.data` 的完整 schema 不在本文件中，无法仅凭本源确认所有 metadata 字段及 `time_created` 的物理精度。
- `convertToModelMessages` 对动态 `tools` 的最终 provider-specific 序列化由 AI SDK 决定，本文件只明确了输入适配形状。
- `ProviderError.parseAPICallError` 与 `parseStreamError` 的具体识别规则在外部模块，无法从本文件确认所有 HTTP 状态和响应 body 的映射。
- 分页期间并发写入是否需要事务快照由调用方/数据库层决定，本文件未作承诺。

## OC-020 packages/opencode/src/session/message.ts
建议新增 `src/message.rs`（由 `lib.rs` 或现有模块树公开），把该文件的协议形状与持久化/运行时分开；若必须沿用现有布局，则 `src/session.rs` 放持久化消息类型，`src/core.rs` 只消费其投影。建议落点如下：

- `ToolCall`/`ToolPartialCall`/`ToolResult`/`ToolInvocation` 对应 Rust `enum ToolInvocation` 的三个 `#[serde(tag = "state")]` 变体，`step: Option<u64>` 表达 `NonNegativeInt`，`args: serde_json::Value` 表达 `Schema.Unknown`，结果保留 `String`。现有 `src/tools.rs` 的 `ToolCall { id, name, arguments }` 与 `ToolResult::{Success,Error}` 可作为执行层对象，但字段名和“result 必须字符串”的 wire 契约不同，建议单独转换，不直接复用。
- `TextPart`、`ReasoningPart`、`ToolInvocationPart`、`SourceUrlPart`、`FilePart`、`StepStartPart` 对应 `#[serde(tag = "type")] enum MessagePart`。`ReasoningPart.provider_metadata` 与扩展字段用 `Option<BTreeMap<String, Value>>`；`SourceUrlPart`/`FilePart` 的 URL 仍是字符串。现有 `src/backend.rs::ProviderEvent::ReasoningDelta`、`ToolCallDelta`、`ToolCallDone` 可作为流式输入来源，`src/core.rs::AgentEvent::{ToolCall,ToolResult,Provider}` 可作为事件投影，但不能替代可恢复的 parts 数组。
- `Info` 建议为 `MessageInfo { id, role: MessageRole, parts, metadata }`；`MessageRole` 用 `serde(rename_all = ...)` 或显式 `user/assistant`，不要复用 `TurnRole` 的 `system/tool` 变体，除非做明确转换。`metadata.sessionID` 可关联 `src/session.rs::SessionHeader.session_id`；`metadata.time`、`snapshot`、`tool` 与 assistant token/cost 应写入 `SessionRecord.value` 的事件 JSON，以便恢复和审计。
- `assistant.modelID/providerID` 可映射 `src/backend.rs::CompletionRequest.model` 与 providers registry 的 provider/model 路由；`path.cwd/root` 应取 core/workspace 上下文。`cost` 与 token 计数可从 provider `Usage` 事件累计后落盘。`MessageError.SharedSchema` 应映射 `src/core.rs::AgentError`、`src/session.rs::OperationOutcome` 或一个专用可序列化 message error，而不是把错误只变成自由文本。
- 取消、重试和并发仍由现有 `src/runtime.rs::CancellationToken`/`JobOutcome`、`src/tool_runtime.rs::execute_tool_batch`、`src/approval.rs::ApprovalCoordinator`、`src/headless.rs` 的 admission/replay 机制实现。schema 层只在提交或恢复边界做校验；工具副作用前仍须走 approval 与 operation journal。

可执行差异清单：

1. TypeScript `Schema.Array`、`Schema.Record` 未设容量；Rust 端应在 `protocol.rs` 或 message validator 增加消息/parts/metadata 大小上限，避免 `serde_json::Value` 无界增长，并记录拒绝原因。
2. `NonNegativeInt` 在 Rust 用 `u64` 最接近，但若 JSON 数字先落 `Value`，需拒绝负数、浮点与溢出；`cost`/tokens 用 `f64` 后必须显式 `is_finite()`，不能仅依赖 serde。
3. TypeScript 的 `Schema.Unknown` 比现有 `tools::ToolCall.arguments` 的默认对象约束更宽；wire 解码应保留任意 JSON，执行层再按工具 schema 验证。
4. `StructWithRest` 允许工具元数据扩展键；Rust 不应对该内部对象使用 `deny_unknown_fields`，应采用扁平扩展 map；但顶层协议请求仍可沿用 `src/protocol.rs` 的严格字段策略。
5. 源 schema 不检查 ID 唯一性、URL 可达性、snapshot 内容、tool 时间单调性或 token 非负；若 zenpi 需要更强不变量，应在 `src/session.rs` 的恢复校验或 `src/core.rs` 的业务验证中新增并测试，不能声称它们来自本源文件。
6. 源 `ToolResult` 用字符串结果，而 zenpi 执行层结果是 `Value`/`ToolFailure`；转换时应保留结构化值的 
未决: 1. `NonNegativeInt`、`SessionID`、`ModelV2.ID`、`ProviderV2.ID` 和 `MessageError.SharedSchema` 的精确约束定义在其他文件，本源只能确认其被引用，不能确认长度、格式或错误变体。
2. `export * as Message from "./message"` 在构建产物中的循环再导出形态、tree-shaking 结果及调用方实际使用方式需由 TypeScript 构建配置确认。
3. `Schema.StructWithRest` 对重复键、未知键的编码排序和解码细节由 Effect 版本实现决定，源文件未声明稳定顺序。

## OC-021 packages/opencode/src/session/overflow.ts
- `src/context.rs` 已有 `ContextBudget { max_tokens, reserved_output_tokens }`、`estimate_tokens` 和上下文准备逻辑；建议新增纯函数 `usable_context_tokens(model: &ModelDescriptor, budget: ContextBudget, output_token_max: Option<u64>) -> u64` 与 `is_context_overflow(usage: &TokenUsage, ...) -> bool`，把本文件的 `max(0, …)` 改写为 `saturating_sub`。可执行判据是分别测试 context=0、input cap、无 input cap、显式 reserved 和阈值相等。
- `src/providers/registry.rs` 的 `ModelDescriptor` 已有 `context_window` 与 `max_output_tokens`（对应 `Provider.Model.limit.context`/输出上限）；其 `context_budget`（约 `L101-L110`）会把用户预算裁剪到模型能力，适合作为 `ProviderTransform.maxOutputTokens` 的 Rust 落点。`src/providers/**` 目前没有与 TypeScript `limit.input` 完全等价的统一字段，需在 descriptor/override 中明确该差异，不能静默假设 context 等于 input。
- `src/backend.rs` 的 `Usage`（约 `L298-L304`）提供 `input_tokens`、`output_tokens`、`total_tokens`，但未见 `cache.read`/`cache.write` 对应字段。若要复刻回退公式，应扩展 usage 或新增 `cache_read_tokens`/`cache_write_tokens`；并明确 `total_tokens == 0` 时才回退，保持 JavaScript `||` 的语义。
- `src/core.rs` 已有 `Agent::set_context_budget`、`context_budget`、`compact_context`/`compact_context_with_control` 和 `CompactionReport`（约 `L1963-L1967`、`L2412-L2472`）。建议在 provider 请求 admission 前调用 overflow 判定，`true` 时进入既有 `compact_context`，而不是让判定函数直接写 session；将 `CompactionReport` 作为可验证的执行结果。
- `src/session.rs` 提供 append-only event、semantic checkpoint 和恢复接口（`L1231-L1389` 一带），承担压缩后的持久化/恢复；映射应保持纯判定与持久化分离，overflow 函数不直接操作 `SessionStore`。
- `src/runtime.rs` 的 `CancellationToken`（`L49-L96`）及 `BackgroundRunner` 负责协作取消、排队和 bounded shutdown；本文件无取消点，若压缩流程在后台运行，应由调用方在 compaction 前后检查 token，不能把 overflow `bool` 当作取消结果。
- `src/headless.rs` 的 JSONL runner/replay 负责事件传输和恢复，`src/protocol.rs` 定义版本化 wire 类型；若需要对外可观测性，新增 `context_overflow`/`compaction_required` 事件应放在 protocol/core 适配层，保留当前纯函数作为内部判据。
- `src/tool_runtime.rs` 的 tool batch 有输入/字节上限和取消传播，`src/approval.rs` 处理 side-effect approval；二者都不是 token overflow 的执行点。压缩前若涉及工具结果裁剪，应由 `Agent`/`SessionStore` 完成并遵守现有 cancellation 与持久化顺序。

建议的可验证 Rust 差异清单：补齐 model input-cap 字段；补齐 cache token 统计；固定 `total == 0` 回退规则；用 `saturating_sub` 防下溢；为 context=0、auto-disabled、显式 reserved、等号阈值添加单元测试；在 `Agent` 的请求路径中验证 overflow 后确实产生 `CompactionReport`/checkpoint，而纯判定单元测试不应产生文件或网络副作用。
未决: 1. `ProviderTransform.maxOutputTokens` 在 `outputTokenMax` 未提供、模型没有输出上限或配置非法时的精确返回/抛错规则不在本文件内。
2. `tokens.total` 是否始终已包含 `cache.read`、`cache.write`，以及何时会为 0，只能由 `SessionV1.Assistant["tokens"]` 的生产方确认。
3. `ConfigV1.Info.compaction.reserved` 的单位、允许负值与校验规则未在本文件定义；本函数只在结果处做 `Math.max(0, …)`，没有对显式 reserved 单独校验。

## OC-022 packages/opencode/src/session/processor.ts
- `src/core.rs` 的 `Agent::process_with_cancel_and_events`、`run_active_turn_cancelable_with_events` 是 `process` 的主要落点：前者负责 admission，后者负责 provider operation marker、取消、工具循环和 `ProviderEvent` sink（`src/core.rs:L5310-L5373`、`L3674-L3801`）。建议新增 `ProcessorState`（assistant turn、当前 text/reasoning、tool map、needs_compaction）及 `process_provider_events`，保持 `Agent` 只负责生命周期提交。
- `src/backend.rs` 的 `ProviderEvent` 只有 `TextDelta`、`ReasoningDelta`、`ToolCallDelta/Done`、`Usage`、`Completed/Failed` 等（`src/backend.rs:L236-L276`），对应 `LLMEvent` 时需补一个可验证的归一化层：`reasoning-start/end`、`text-start/end`、`step-start/finish`、provider tool error、metadata 和附件必须明确映射，不能把 delta 当完整 part。`Backend::complete_with_control` 已提供取消检查和事件 sink（`src/backend.rs:L492-L531`）。
- `src/session.rs` 是 append-only JSONL durable owner，已有 `OperationKind::{Provider,Tool,Compaction}`、`OperationOutcome`、`InterruptedOperation`/`OperationRecovery`；可将 `SessionV1` part 事件编码为 `SessionRecord`/`append_event`，并在 operation marker 缺终态时沿用 unknown outcome，不自动重放副作用（`src/session.rs:L49-L95`、`src/session.rs:L314-L325`）。这比当前 TS 的内存 `Deferred` 更耐崩溃。
- `src/tool_runtime.rs` 已定义有界 tool batch、source-order prepare/persist 和不自动重试中断调用；映射 `ensureToolCall/completeToolCall/failToolCall` 时应让 `ToolBatchDecision` 产生 pending/running/completed/error 事件，并把 `Tool execution aborted` 转成 `OperationOutcome::Cancelled/Interrupted`。批量并行的边界必须与 TS 的单调用事件顺序分开测试。
- `src/approval.rs` 的 `ApprovalCoordinator::request/request_response` 与取消 epoch 可承载 `doom_loop`、权限拒绝和 `Question.RejectedError` 的阻断语义；新增 `permission="doom_loop"` 的 typed request，并在三次同名同 input 判定上复用 `src/core.rs` 的工具调用历史（`src/approval.rs:L122-L241`）。
- `src/runtime.rs` 的 `BackgroundRunner`、`CancellationToken`、`JobOutcome` 适合承载 Effect fiber 外层：队列有界、取消合作式、shutdown grace 后可 detach，但必须像 TS cleanup 一样在 job 终态前写中断工具结果；runtime detach 不能声称回滚副作用（`src/runtime.rs:L49-L99`、`L156-L193`、`L272-L374`）。
- `src/headless.rs` 与 `src/protocol.rs` 只负责 JSONL admission、replay、`TurnMode`、事件相关性和 stdout 背压，不应直接执行 `processor` 逻辑；将 `compact/stop/continue` 投影为终端 `StdioResponse`，并以 request/turn ID 关联 provider/tool 事件。headless 的 replay journal 与 `src/session.rs` durable journal 要保持两套 cursor 语义。
- `src/providers/**` 负责 protocol/dialect/endpoint/auth 路由；provider-specific metadata、Anthropic thinking transformation、usage/cost 计算应放在 provider 归一化或 `core` 的纯函数中，而不是散落到 headless。当前 Rust `Usage` 是 token 三元组，TS 还计算 cost 与模型 limit overflow，需补 `Cost/ContextBudget` 适配并为 overflow→compaction 写集成测试。

可执行差异清单：①建立统一 `ProviderEvent -> PartEvent` 枚举并为每个事件写序列断言；②为 `ProcessorState` 提供 snapshot/patch、文本 hook、图片附件过滤接口；③将 retry policy 的可重试分类和 status attempt 
未决: 无。

## OC-024 packages/opencode/src/session/reminders.ts
建议新增 `src/session_reminders.rs`（或放入 `core.rs` 的独立私有模块）实现纯决策函数加副作用适配层。现有 `src/core.rs` 的 `TurnRole::{System,User,Assistant,Tool}`、不可变 `Turn { id,parent_id,role,content,metadata }`位于 L131-L152，构造器在 L154-L175；可把 TS 的 synthetic part映射为带 `metadata: {"synthetic":true,"reminder_kind":...}` 的 `TurnRole::System` 或 `User` 子记录，但需明确 zenpi 当前一条 `Turn`只有单个 `content`，不能直接等价于 `parts` 数组。`Turn::validate`限制 ID、父 ID及内容长度（L177-L204），Rust 映射应在生成 reminder 前验证长度。

`src/core.rs` 的 `TurnInputRequest`及模式字段在 L207-L240，提交边界与拒绝原因在 L3220-L3337；`start_new`会构造并持久化 user turn（L3364-L3406），`process_with_cancel_and_events`在 L5327-L5349串起提交和 provider turn。建议在 `start_new`完成 user turn入 journal 后、provider 请求准备前调用 `SessionReminders::apply`，根据一个显式 `AgentMode::{Plan,Build}`和 `experimental_plan_mode`配置返回待注入的 system reminder；build/steer 重发路径也应以“最后 assistant agent”元数据判断，避免仅看 role。

持久化落点是 `src/session.rs`：文件头说明 append-only JSONL、崩溃后恢复完整记录（L1-L5）；`SessionStore::append_turn`先验证、写盘，再更新内存 projection，失败不改变内存（L861-L890）。若 synthetic reminder需可恢复，建议追加独立 `Turn`或 `event`记录，并保存 `synthetic`、`reminder_kind`、`plan_path`；若仅是每次 provider 请求的瞬时 system prompt，则不要写 journal，并以可重复纯函数生成。`SessionStore`已有 event projection（L144-L160、L1205-L1208附近），可用于审计“提醒已注入”，但必须增加幂等键（session、user turn、kind）以避免恢复重复。

取消与并发对照 `src/runtime.rs`：`CancellationToken`是可观察、幂等取消，且有完成标记防止晚到 cancel 改写成功结果（L49-L99）；运行时事件包含 `CancelRequested`、`Completed`、`Closed`（L171-L193），提交/取消是有界非阻塞队列（L308-L350）。reminder 生成应在取消检查点前后保持原子：若目录检查或 update 期间取消，返回 cancelled 且不要发布半完成的 synthetic 记录；不要把取消当成文件回滚。运行时本身没有本 TS 的“重试”，重试应沿用 `src/session.rs` 的 interrupted operation 语义而不是重复追加提醒。

协议映射在 `src/protocol.rs`：`StdioRequest`携带 `text/message/mode/expected_turn_id`等字段（L152-L209），`prompt`解析为 bounded text、默认 `TurnMode`并校验 attachments（L377-L395），`cancel`要求有效 target ID（L415-L423）。无需新增 provider wire 字段；若要让客户端选择 plan/build，建议新增严格枚举字段并在协议层拒绝未知值。`src/headless.rs`只负责 JSONL 传输和请求关联（L1-L5、L553-L625），应调用 core 的 reminder 逻辑而不是自行拼提示；其异步事件按请求隔离缓冲（L823-L869），可承载 reminder 注入事件但不应把它伪装成 provider delta。

副作用与工具层对照：TS 提醒不需要 approval。zenpi 的 `src/approval.rs`把审批请求持久化前后的协调、取消拒绝与 cancellation epoch定义在 L122-L187、L405-L445，reminder 的目录创建不应绕过现有 host policy；若计划路径被视为写操作，应在调用 `ensure_dir`前走对应权限边界。`src/tool_runtime.rs`规定批量工具取消、串并行和结果相关性（L157-L176、L189-L249），但本功能不是 tool batch，不应复用并行执行器。

`src/providers/**`（入口 `src/providers/mod.rs` L1-L10，协议枚举与 provider definition 在 L13-L50、L146-L159）只描述 wire protocol、认证和能力路由；不要把 plan/build reminder硬编码到 `anthropic`、`openai`等 provider，统一在 core 的请求准备层注入，确保所有 provider 行为一致。可执行验证顺序：为 `SessionReminders`写纯函数单测覆盖五个判据；用临时 `SessionStore`验证 append/重放幂等；用 `CancellationToken`在 exists/en
未决: - `Session.plan(input.session, ctx)`的确切路径规则、是否始终位于 workspace 内，无法从本文件确认。
- `FSUtil.existsSafe`对权限错误、符号链接和竞态的具体返回策略未在本文件定义。
- `Session.Service.updatePart`是否立即持久化、是否幂等、失败后是否自动重试，需查看其实现。
- `PartID.ascending()`的线程安全、进程重启后的单调性和跨 session 唯一性未由本文件保证。
- `prompt.ts`何时、每轮调用多少次 `SessionReminders.apply`未在本文件确认，这直接影响重复提醒风险。

## OC-025 packages/opencode/src/session/retry.ts
现有 zenpi 的 `src/backend.rs` 已有最接近的落点：`BackendError` 用结构化 `Transport`、`HttpStatus { status, retry_after_ms }`、`CircuitOpen`、`Cancelled`、`DeadlineExceeded` 等类型表达错误，并用 `is_retryable()` 判定 Transport、408/409/425/429/5xx（backend.rs L417-L472）。provider 请求内部已有最多 `max_retries` 的指数等待、`retry_after_ms` 取最大值、每 10ms 轮询取消（backend.rs L1774-L1825），HTTP `Retry-After` 解析支持秒和 HTTP date，且上限为 60 秒（backend.rs L1873-L1890）。这与 TS 的 2 秒起步、25% jitter、无头 30 秒上限、带头 32 位上限存在明确差异。

建议的可执行 Rust 映射与差异清单：

1. 新增 `src/retry.rs`（或将策略拆到 `backend.rs` 的独立纯模块），定义 `RetryReason`、`RetryAction`、`Retryable { message, action }`、`RetryPolicyConfig`，实现 `delay(attempt, &BackendError, random)`、`classify_retryable(&AgentError, provider)`；用 `Result<Option<Retryable>, RetryError>` 或纯 `Option` 对应 TS 的 `Retryable | undefined`。先写表格驱动测试复刻 retry.test.ts 的 L35-L95、L151-L336、L338-L427 判据。
2. `src/backend.rs` 的 `BackendError::is_retryable` 目前是状态码白名单，需决定是否扩展到 response body/transport 文本正则和 `ContextOverflow` 明确拒绝。建议保留结构化类型优先，增加 `RetryHint { after: Option<Duration> }`，并让 `retry-after-ms` 优先于秒/date；将现有 60 秒上限改为与需求一致的“无头 30 秒、带头 32 位毫秒上限”，或显式记录这是 zenpi 的产品差异。
3. 在 `src/core.rs` provider turn 边界接入 policy：每次失败先分类，再在可重试且未超过 5 次时写 retry 状态，实际 sleep 用 `CancellationToken` 每 10ms 检查。现有 `AgentError::Backend` 映射和 provider event 流（core.rs 中 `ProviderEvent` 处理及操作结果归类）可作为输入输出边界；不要在已有 unknown outcome 操作上自动重放外部副作用。
4. `src/runtime.rs` 已提供 `CancellationToken`，且文档明确 backend 在每次重试前检查（runtime.rs L49-L99）；将它传入新的 retry sleep。`BackgroundRunner` 的有界提交、取消和关闭语义（L308-L350）负责并发，不应把 attempt 计数放进全局 runner。
5. `src/session.rs` 已定义 `OperationOutcome::{Succeeded,Failed,Cancelled,Interrupted,UnknownOutcome}` 与 `retry_requires_confirmation`（session.rs L49-L95），并有 operation marker/recovery API（session.rs L1443-L1514）。TS retry 只更新 `SessionStatus`，没有 durable journal；若 zenpi 要恢复 UI，应新增明确的 `retry_scheduled`/`retry_finished` 事件，但 provider/tool 的 `UnknownOutcome` 必须继续要求显式确认，不能套用自动 retry。
6. `src/headless.rs` 已将队列压力标成可重试并释放 request ID reservation（L3870-L3901），并将 provider `Failed` 映射为 `retryable: true`、拒绝映射为 false（L4339-L4353）。可把 `Retryable.action` 投影到 headless 错误响应/状态，但需要保留 request ID 幂等与缓存规则；不要因 transient admission 失败写 terminal cache。
7. `src/protocol.rs` 的 `Command::Cancel { target_id }`（L211-L235）是显式取消入口；可增加可选 `retry_after`/`attempt` 的只读状态字段，但 retry 逻辑不应绕过协议校验。`src/approval.rs` 的取消检查和 50ms 条件变量等待（L167-L245）说明审批等待也必须在重试前后尊重取消，且 `emergency_cancel` 是不可清除的 epoch（L438-L446）。
8. `src/providers/**`（`anthropic.rs`、`google.rs`、`openai.rs`、`codex.rs`、`deepseek.rs`、`connection.rs`、`registry.rs`）当前主要负责协议、路由、认证、能力和模型元数据；`providers/mod.rs` 将 provider 映射为 `Protocol`/`Auth
未决: 1. `SessionStatus.set` 的具体存储介质和进程重启后的保留策略不在 `retry.ts` 内，需查看其 service 实现才能断言 retry 状态是否 durable。
2. `Schedule.InputMetadata.attempt` 的初始值和 Effect 版本的精确计数契约由 `effect` 库提供；本文件只通过测试确认首次可写 attempt 为 `1`、第六次停止。
3. `responseHeaders` 的实际 schema 是否始终是字符串值、以及 provider 是否会同时提供 `retry-after-ms` 与 `retry-after`，源文件未定义；代码仅按 truthiness 和 `parseFloat` 处理。
4. `GoUsageLimitError` 缺少 `workspace` 时生成的双斜杠链接是否被 UI 接受，源文件没有校验或回退策略。

## OC-026 packages/opencode/src/session/revert.ts
对照结果显示 zenpi 当前没有 SessionRevert、Snapshot.Service、session_diff 或 provider 级工作区回退 API；rg 在 src/headless.rs、src/core.rs、src/session.rs、src/tool_runtime.rs、src/runtime.rs、src/protocol.rs、src/approval.rs、src/providers/** 未发现同名实现。

建议落点与可执行验证：

1. **会话与回退状态：src/session.rs**。新增可序列化 RevertState { message_id: String, part_id: Option<String>, snapshot_id/digest: String, summary: DiffSummary }，通过 append-only event（如 session_revert_set/session_revert_clear）持久化；实现 SessionStore::set_revert、clear_revert、revert_state、cleanup_revert。用现有 SessionRecord 回放验证崩溃后状态可恢复。现有 SessionStore::append_turn 只追加 turn（L863-L889），没有 removeMessage/removePart，所以 cleanup 应追加“隐藏/截断”事件并在投影层过滤，或明确实现新的原子重写协议。
2. **工作区快照：新增 src/workspace_snapshot.rs（或并入 src/core.rs）**。实现 track、restore、revert_patches、diff，优先以受控 git tree/index 或带 SHA-256 的文件清单保存基线；验证连续回退的 restore-then-apply 顺序与路径越界拒绝。该能力不应放入 src/providers/**，因为 provider 只负责模型传输/重试。
3. **核心门禁：src/core.rs**。在 Agent 增加 revert_session、unrevert_session、cleanup_revert，要求 phase == AgentPhase::Idle，否则返回既有 AgentError::NotIdle（Agent::snapshot 暴露 phase，L1341-L1350）。将 session journal、workspace snapshot、diff summary 按明确顺序提交；对 Storage.write 被忽略这一点，Rust 版本应改为显式 Result 并记录可恢复错误。
4. **传输与 headless：src/protocol.rs、src/headless.rs**。在 Command 增加 Revert { session_id, message_id, part_id }、Unrevert、Cleanup 或一个带 action 的结构；在 StdioRequest::into_command 做 ID/长度校验，然后由 headless 的 session owner 分发。现有 Cancel 只请求运行时取消（Command::Cancel，L220-L251），不能替代回退；session slash dispatch 当前只有 list/search/open/fork 等生命周期动作（L7697-L7772）。
5. **取消、并发与审批：src/runtime.rs、src/tool_runtime.rs、src/approval.rs**。复用 CancellationToken 的协作式语义（L49-L99）和 Runtime::try_cancel 非阻塞请求（L318-L325），但在回退步骤间检查取消并把未完成状态写成 interrupted；不要把取消误当成文件 rollback。工具批处理明确规定取消不是回滚证据（tool_runtime.rs L157-L167、L236-L239），应作为新 API 的错误判据。ApprovalCoordinator::emergency_cancel 只取消等待中的审批（L438-L443），不应承担回退。
6. **providers/**：无需修改。src/backend.rs/src/providers/connection.rs 的取消、流式传输、retry-after 只影响模型请求；回退的文件副作用应由 workspace/session owner 管理。可验证差异是 provider 测试无需知道 RevertState，而 core 集成测试要覆盖 provider turn 完成后回退。

关键差异清单：TypeScript Effect 依赖层与 orDie 缺陷语义对应 Rust 的显式 Result；OpenCode 的可变消息/part 存储对应 zenpi 的 append-only JSONL；OpenCode 按消息数组顺序扫描并可物理删除，zenpi 现有 session tree 更适合追加分支/投影；OpenCode 忽略 diff storage 写失败，zenpi 应保留错误并可重放；OpenCode 没有本文件级取消/重试，zenpi runtime 已有协作式取消且 provider 有 retry，因此不能自动推断“取消即恢复”。
未决: - Snapshot.Patch 的具体 patch 方向、哈希校验和 Snapshot.Service 对未跟踪/删除文件的精确处理不在本文件中，需继续核对 src/snapshot 实现与测试。
- sessions.setRevert、Session.Info.revert、SessionSummary.computeDiff 的持久化格式和并发锁细节由依赖模块定义，本文件只能确认调用顺序与错误处理。

## OC-027 packages/opencode/src/session/run-state.ts
- `src/runtime.rs` 的 `CancellationToken`（`L56-L91`）、`BackgroundRunner`（`L237-L319`）和 `RuntimeEvent`（`L158-L184`）最接近 `Runner`：建议新增 `SessionRunState`，内部用 `HashMap<SessionId, SessionRunner>`，`SessionRunner` 持有 `JobId`、取消 token、busy 标记和 `on_interrupt` 回调；`ensure_running` 映射为提交或复用活动 job，`start_shell` 映射为带 ready barrier 的提交。`try_cancel` 与 `shutdown_and_join_with_grace`（`L319-L390`）可实现显式取消和有限关闭等待；差异是 zenpi 已明确“协作取消、不可回滚”，而源文件没有 grace timeout。
- `src/core.rs` 的 `AgentPhase`/`AgentEvent`（约 `L270-L350`）和 `submit_with_cancel`（`L3222-L3265`）是状态与工作入口。建议把 `SessionRunState` 的 idle/busy 写入转换为 `AgentEvent` 或新的 `SessionStatus` 事件；忙错误可复用 `NotSubmittedReason`，但需保留 `session_id` 以对应 `Session.BusyError`。`cancel_blueprint_worker`（`L2853-L2868`）只撤销 durable lease，不能替代本文件的进程内 runner 取消，应在 host 层串接。
- `src/session.rs` 的 `SessionStore`（`L146-L160`）、`OperationOutcome`（`L59-L65`）和 `OperationRecovery`（`L88-L96`）适合记录 start/terminal/interrupted marker。建议为每个 runner job 写 operation id 与 session id，取消落 `Cancelled`/`Interrupted`；源文件没有持久化，所以这是 zenpi 的增强，并须避免把“取消请求”误写成“已完成”。
- `src/tool_runtime.rs` 的 `execute_tool_batch`（`L168-L238`）已经要求调用方提供 `cancelled` 谓词，并在取消时阻止未开始调用、等待已开始调用；可把 `SessionRunState.cancel` 的 token 谓词传入。其批次是有界且会 join，源文件的后台取消是无界并发且只处理 `running` job，需明确这一差异。
- `src/protocol.rs` 的 `TurnMode`（`L46-L58`）和 `Command::Cancel`（`L214-L260`）可承载 `StartIfIdle`、`StartOrSteer`、`Steer` 以及 session/job cancel。建议新增 `SessionRunState` 控制器函数，由协议层把 cancel target 映射到 `SessionId`，返回 busy/cancelled/unknown 的稳定响应。
- `src/approval.rs` 的 `ApprovalCoordinator`（`L127-L187`、`L405-L440`）有 `cancel_all`、`emergency_cancel` 和 cancellation epoch；工具等待 approval 时应监听同一会话 token，否则 `run-state.cancel` 只停 runner 而 approval 阻塞仍存活。源文件不涉及审批，需保持审批取消与 job 取消的顺序可观测。
- `src/headless.rs` 已在多处处理 `Command::Cancel` 与 `CancellationToken`（例如 `L6291-L6346`），建议把每个 headless session 的 runner 注册/查找集中到新模块，再由 headless 只发控制命令；避免在 headless、TUI 各自维护 busy map。其有限 mailbox/replay 语义也应与 runner 的 terminal outcome 对齐。
- `src/providers/**`（`mod.rs`、`registry.rs` 及各 provider adapter）负责 provider 能力与请求执行，不应直接拥有 session runner。建议 provider 调用接收 `&CancellationToken`/取消闭包，在流式 chunk、重试前检查；provider 的网络取消不保证撤销已提交的远端副作用，这与源的 `onInterrupt` 回调边界一致。
- 建议落点：`src/session_run_state.rs` 定义 `SessionRunState`, `SessionRunner`, `SessionBusyError`、`ensure_running`、`start_shell`、`cancel`、`assert_not_busy`；`src/runtime.rs` 暴露可复用 runner 句柄；`src/headless.rs`/TUI host 仅负责协议与事件投影；`src/session.rs` 负责可恢复 marker。可执行差异清单是：补充按 session 的 map、idle/busy 事件、后台 job 的 session/parent 关联遍历、approval 取消联动、可选 shutdown grace，并为每项写集成测试。
未决: 1. `Runner.make` 的 `ensureRunning`、`startShell`、`busy` 字段以及 `onInterrupt` 的确切触发时机不在本文件中，无法确认工作是否排队、替换还是拒绝。
2. `BackgroundJob.list()` 返回快照的并发一致性与 `background.cancel` 在 job 已结束时的行为未由本文件确认。
3. `SessionStatus.Service.set` 是否持久化、是否幂等、失败时是否影响 runner 生命周期，源码未给出。

## OC-030 packages/opencode/src/session/status.ts
- `src/core.rs`：`AgentPhase::{Idle,Running,Closed}`、`AgentSnapshot` 和 `AgentEvent`（尤其 `TurnAccepted`、`Provider`、`Error`）是现有状态投影。建议新增 `SessionStatusInfo`（`Idle|Busy|Retry{attempt,message,action,next}`）及 `SessionStatusStore`，把 `get/list/set` 做成 `&self/&mut self` 方法；在 `Agent` 的 turn admission、provider 错误和完成路径调用它。
- `src/session.rs`：`SessionStore::append_event` 是持久事件入口，但 TS 服务本身不持久化。若要保留跨进程状态，应增加可选的 `session_status` 事件或由 `AgentSnapshot` 重建；必须明确 idle 删除是内存视图还是 journal 记录，避免把 TS 的实例重置误当成 durable 恢复。
- `src/runtime.rs`：`BackgroundRunner` 的 `RuntimeEvent::{Started,CancelRequested,Completed}` 与 `CancellationToken` 可驱动 `Busy`/`Idle` 转换。完成标记应在发布成功结果后再置 idle，以匹配 TS 的“先事件、后状态变更”顺序；取消仍是合作式，不应声称回滚。
- `src/headless.rs`：已有异步 stdout/replay 和事件邮箱，可把状态事件包装为带 `session_id`/`turn_id` 的 `StdioEvent`。需限制事件重放预算，并保证 `session.status` 与 `session.idle` 的顺序，不能把状态查询结果当成第二个 terminal response。
- `src/protocol.rs`：`Command::Status` 和 `StdioEvent` 是协议落点。建议为 `Status` 响应定义与 `SessionStatusInfo` 对应的 serde 结构，并把未登记 id 的响应固定为 idle；显式版本兼容应沿用现有 `schema_version` 规则。
- `src/tool_runtime.rs`：没有直接等价物；它负责工具批处理、批准前置和取消传播。只需在工具批次开始/结束时调用状态 store，不能让工具执行器自行发布重复状态或自行持久化。
- `src/approval.rs`：无直接状态映射。审批等待可使会话保持 busy；若审批被取消，应由 core/runtime 统一发布 retry、failed 或 idle，不能由 `ApprovalCoordinator` 猜测 session status。
- `src/providers/**`：各 provider 的 `ProviderEvent`、网络重试与错误是 retry 信息来源；provider 模块只产出错误/流事件，不应持有跨 session 的 Map。建议由 `core.rs` 将 provider 错误转换成 `Retry{attempt,next,message,action}`，并由统一状态 owner 发布。

差异清单：TS 是每个实例一份可变 Map，Rust 当前以 `Agent`/`SessionStore` 和持久 JSONL 为主；TS 的 `get` 默认 idle 但不登记，Rust 查询 API 需显式遵守；TS idle 兼发 deprecated `session.idle`，zenpi 目前主要是 `AgentEvent`/`StdioEvent`；TS 不提供锁或 CAS，Rust 若跨线程共享应使用 owner thread 或 `Mutex/RwLock` 明确串行化；TS 的事件先行规则与 zenpi 当前部分“先 journal 再输出”路径可能不同，映射时必须写集成测试验证顺序。
未决: 1. `InstanceState.make` 的具体生命周期和并发保证不在本文件中，无法仅凭 `status.ts` 判断 Map 是否会跨 workspace 共享。
2. `EventV2Bridge.publish` 的 durable 选项未在本文件传入，无法确认 `session.status` 是否会被底层事件总线持久化。
3. `Info` 的 schema 细节来自外部 `@opencode-ai/schema/session-status-event`；本文件自身不定义 retry 的数值上限或 `action.link` 约束。

## OC-031 packages/opencode/src/session/summary.ts
zenpi 的 `src/session.rs` 已有会话持久化与 `SessionSummary`（`src/session.rs:L280-L297`），但它是路径、会话 ID、turn/handoff/event 计数和恢复警告的会话级目录摘要；`summary.ts` 的 `additions/deletions/files/diffs` 属于单个 user message，不能直接复用同名结构。建议新增 `src/session_summary.rs`（或将下列类型放入 `src/session.rs` 的独立区域）：`FileDiff { file: Option<String>, additions: u64, deletions: u64, ... }`、`MessageSummary { additions, deletions, files, diffs: Vec<FileDiff> }`、`SessionSummaryService`，并以 `SessionStore` 的单写者 API 持久化 message metadata/event。

- 对照 `src/session.rs`：将 `summarize` 的清零和最终写回实现为一个可验证的 `update_message_summary(session_id, message_id, summary)`；写入应沿 `append_json`/事件日志路径，保证“先持久化后更新内存投影”的习惯（现有 `append_turn` 的行为可作判据，`src/session.rs:L861-L891`）。可将快照 ID 与 `Turn.metadata` 或专门 `snapshot_step` 事件关联；当前 `SessionStore` 没有 `messages()`、`updateMessage()` 或 `Snapshot.Service` 等价物，需要新增索引或由 `Agent` 提供投影。
- 对照 `src/core.rs`：`Agent::snapshot()` 只产出会话/运行状态（`src/core.rs:L1337-L1346`），可在 Agent 的 turn 完成路径调用新 service；已有语义压缩/`append_semantic_checkpoint_for_operation`（`src/session.rs:L1231-L1290`、`src/context.rs`）是上下文恢复，不等价于文件 diff，应保持两套数据模型。
- 对照 `src/headless.rs`：已有 cancellable worker、请求重放和资源/文件差异命令（例如 `src/headless.rs` 中 `/diff` 的 `diff_value_at` 路由），可把 `SessionSummaryService::diff` 暴露为一个明确的 headless command；必须复用 headless 的 `CancellationToken` 检查和有界响应策略，不要把文件内容或差异缓存无限放入 replay WAL。
- 对照 `src/tool_runtime.rs` 与 `src/approval.rs`：这些模块负责工具执行、审批、取消和副作用审计。快照采集若调用工具或工作区写操作，应先走 `ApprovalCoordinator`/`SideEffectPolicy`；纯读取 diff 可以作为只读工具，不应从 provider 名称或响应推断批准。工具取消只代表“不再启动/停止等待”，不能假设外部文件变更已回滚。
- 对照 `src/runtime.rs`：用 `CancellationToken::is_cancelled()` 在遍历消息、快照读取和每次 diff 计算间隙轮询；成功结果发布前调用 `mark_completed()`，避免晚到取消把已完成摘要改成 cancelled。不要创建脱离现有 `BackgroundRunner` 的线程。
- 对照 `src/protocol.rs`：新增命令输入可采用 `session_id`、可选 `message_id` 的 serde 结构，沿现有 `Command` 校验 ID 长度、控制字符和未知字段；`message_id` 缺失时返回空 diff，保持源行为，而不是协议错误。
- 对照 `src/providers/**`：provider registry/connection 只描述模型、协议、路由和认证（`src/providers/mod.rs:L13-L50`、`src/providers/connection.rs:L18-L60`），没有工作区快照能力。`Snapshot` 应由本地 workspace/session 层实现，provider 仅产生带 `step-start`/`step-finish` 元数据的 turn 事件；禁止把文件 diff 计算下沉到 Anthropic、Google、OpenAI 等 provider 适配器。

可执行验证：新增 Rust 单元测试覆盖“首个 start + 最后 finish”“缺端返回空”“user/assistant parent 过滤”“Git quoted path 八进制解码”“取消发生在 diff 前后”“写入失败后事件顺序”；再以 headless JSONL 命令读取同一 `message_id`，比较持久化摘要与读取接口结果。差异清单是：zenpi 当前无 `Snapshot.Service`/`Snapshot.FileDiff`、无按 message 的 summary diffs、无 `Session.Event.Diff` 同名事件、无 `Session.messages/updateMessage` 投影 API，也没有本文件的 Git path 解码器。
未决: 1. `Snapshot.diffFull` 的具体快照存储、差异排序、`FileDiff` 字段和错误类型不在本文件中，无法确认空文件、删除文件或重命名的精确表示。
2. `sessions.setSummary`、`sessions.updateMessage` 是否原子写入、是否有并发版本检查，源文件未说明；因此并发 `summarize` 的最后写入胜出只是基于调用顺序的风险判断。
3. `Effect.orDie` 产生的缺陷如何被上层记录、是否自动重试，需查看 `Session.Service` 与运行时宿主实现。

## OC-032 packages/opencode/src/session/system.ts
- **模型与 provider prompt 路由**：源的字符串启发式应落在新模块 `src/system_prompt.rs` 的 `provider_prompt(&ModelDescriptor) -> &'static str`（Muse 另返回 `String`）。zenpi 的 `src/providers/registry.rs` 用精确 `(provider,id)` 的 `ModelDescriptor`、`resolve`（约 `L366-L373`）和能力交集（`L70-L81`），与源按 ID 子串猜测不同；可执行差异是保留显式 provider/id 规则，避免把 `gpt-6-codex` 的优先级藏在多个 `contains` 中，并为 Muse 模型增加明确字段或测试表。
- **环境段**：映射到 `SystemPromptBuilder::environment`，输入 `ProjectContext`/`SessionHeader`、当前 model 和引用索引，输出 `Vec<String>`。`src/session.rs` 是 append-only JSONL 持久化（文件头注释 `L1-L5`），不应把环境段写成 session 事件；`src/headless.rs` 的 workspace/read-only 观测接口可提供目录边界，但本源只需要工作目录、worktree、VCS、平台、日期和排序后的 references。建议用稳定排序（Rust `sort_by`/`BTreeMap`）并显式 XML 转义，测试无描述项和空引用项。
- **skills**：zenpi 已有 `src/skills.rs` 的 `SkillSet::metadata_index`（`L265-L277`）和 `instructions/effective_instructions`（`L323-L357`），`src/core.rs` 的 `Agent` 持有 `skills`（`L402-L439`），并可通过 `set_skills` 替换（`L2776-L2779`）。建议新增 `SystemPromptBuilder::skills(&Agent, PermissionView) -> Option<String>`，复用 `SkillSet::instructions` 或 `effective_instructions`，另提供等价于 `Skill.fmt(..., verbose: true)` 的确定性格式化；权限拒绝时短路，且不加载 skill body。Rust 已有 `SkillSet::load_body` 的取消和 hash 校验（`src/skills.rs:L282-L317`），这比源文件的无显式取消更严格，属于可保留差异。
- **MCP**：指定的 `src/headless.rs`、`src/core.rs`、`src/session.rs`、`src/tool_runtime.rs`、`src/runtime.rs`、`src/protocol.rs`、`src/approval.rs` 与 `src/providers/**` 没有等价的 `MCP.Service.instructions()` 抽象；`src/resources.rs` 仅提供资源类别统计。建议新增 `src/mcp.rs`：定义 `McpInstruction { name, instructions, tools }`、`McpRegistry::instructions()`，以及 `filter_by_permission`，输出与 `<mcp_instructions>` 对应的字符串。工具许可可接 `src/approval.rs` 的 `ApprovalPolicy`/`ApprovalDecision`（`L466-L520`），但必须区分“approval 允许一次调用”和“system prompt 中完全隐藏 server”，以复现源的“全部工具禁用才过滤”规则。
- **依赖注入与生命周期**：`Service`/`LayerNode` 可映射为 `Arc<SystemPromptServices>` 或在 `Agent` 构造时注入；`src/core.rs` 的 `Agent::new` 会恢复中断操作并初始化 skills（`L598-L640`），适合在此注入 immutable prompt services。初始化失败应在 `prepare_project` 发布替换前返回；该方法已有“先完整准备再发布”的约束（`src/core.rs:L725-L750`）。
- **协议、并发与取消**：源 API 是内部 Effect 函数，不直接暴露 wire 命令；zenpi 的 `src/protocol.rs` 将 `prompt/steer/cancel` 解码为 `Command`（`L211-L263`、`L377-L421`），由 `src/runtime.rs` 的 `CancellationToken`（`L49-L99`）和 `JobOutcome::{Succeeded,Failed,Cancelled,Panicked}`（`L156-L168`）承载取消/并发。建议系统提示构建作为 job 内的纯准备阶段，取消时返回 `BackendError::Cancelled`，不得写 session；若 provider prompt 已提交，取消不回滚外部副作用，符合 runtime 语义。
- **验证清单**：新增 Rust 单元测试覆盖 provider 路由优先级、环境输出排序/日期字段、skill 权限短路、MCP 全禁用过滤、XML 转义和取消不持久化；再用 `cargo test system_prompt` 与现有 `cargo test` 验证。检查 `src/session.rs` 事件计数保持不变，`src/headless.rs` JSONL 响应只在真正提交 turn 后出现，
未决: 无法从源确认：`Skill.Service`、`MCP.Service`、`Permission.disabled/merge` 的具体实现和并发保证；`Provider.Model` 字段的完整约束；`LocationServiceMap` 提供的 layer 是否会缓存引用；`Skill.fmt` 的排序/空列表格式；以及上游 prompt 文本文件本身的内容。源文件也没有规定 XML 转义、长度预算、超时或重试策略。

## OC-041 packages/opencode/src/tool/invalid.ts
- **工具定义落点：** zenpi 的 `src/tools.rs` 已有 `ToolDefinition`、`Tool` trait、`ToolRegistry` 和 `ToolResult`。建议新增 `InvalidTool`（可与其它 builtin 放在同文件），`definition()` 返回 `name: "invalid"`、`description: "Do not use"`、对象型 `input_schema`（必需字符串 `tool`/`error`），`side_effect: ToolSideEffect::ReadOnly`；`invoke()` 仅校验对象字段并返回 `json!({"title":"Invalid Tool","output":...,"metadata":{}})`。若希望直接调用 `ToolRegistry::with_read_only_builtins()` 得到等价注册表，应在那里注册它，并用 `ToolDefinition::validate` 保证定义合法（现有定义/结果类型见 `src/tools.rs`，与 `src/tool_runtime.rs` L16-L19 的使用一致）。
- **调用与结果差异：** zenpi `ToolResult` 以 `call_id`、`tool`、`output: Value` 或 `ToolFailure` 表示（`src/tool_runtime.rs` L16-L19、L446-L455），没有 opencode `ExecuteResult` 的一等 `title`/`metadata` 字段。因此把三字段对象放入 `output` 是兼容性最高的映射；若 headless 客户端只期待文本，则应在协议投影层读取对象的 `output`，不能静默丢掉 `title`/`metadata`。
- **修复入口：** opencode 的关键语义是“坏工具调用 → 合成 `invalid` 调用”，而 zenpi 当前 registry 未知工具通常走 `ToolError::UnknownTool`。建议在 `src/core.rs` 的工具调用准备/`invoke_tool` 前增加 `repair_invalid_tool_call(call, error)`：保留原 `call.id`，生成 `ToolCall { name: "invalid", arguments: json!({"tool": original_name, "error": redacted_error}) }`，再走普通 registry；仅对参数解析/工具调用格式错误启用，不能把权限拒绝、取消或未知副作用错误伪装成参数错误。`src/core.rs` 已有 `PreparedTool`/`ToolInvocationOutcome`（L120-L129）和单调用批执行入口（L4928-L4935、L5086-L5099），适合放置该转换并保留审计证据。
- **批执行、并发与取消：** `src/tool_runtime.rs` 的批执行器规定结果按源顺序返回、批量取消时不执行待处理调用，并对非协作处理器执行 join（L151-L176、L438-L455）。`invalid` 是只读、无状态的诊断工具，可标记为 `Parallel` 或沿用默认 `Sequential`；为保持简单和与修复调用的调用序一致，建议先沿用默认顺序模式。它不应引入额外线程、超时或重试。宿主取消仍由 `cancelled` 谓词传播；取消发生在调用前应得到 `ToolErrorCode::Cancelled`，不能产出伪造的“invalid 参数”成功结果。
- **运行时取消边界：** `src/runtime.rs` 的 `CancellationToken` 是协作式、幂等取消，并要求工作在重试/流式边界检查（L49-L90）。这与源工具不主动读取 `abort` 的差异应明确记录：Rust 适配器可以在进入 `invoke` 前检查 token，但 `InvalidTool` 本身没有长任务，不需要新增 polling；完成标记与迟到取消的竞争由 runtime 统一处理。
- **审批与副作用：** `src/approval.rs` 将审批请求与取消等待分开（L109-L135、L167-L245）。将 `invalid` 声明为 `ReadOnly` 后，它应绕过写入审批；不要为它创建 `ApprovalRequest`，也不要把 `tool` 字段当作真实待执行工具名。若修复前的原调用是写入工具，原调用的审批/预览仍应在修复前失败路径中记录，合成诊断调用不能绕过安全门。
- **持久化与恢复：** `src/session.rs` 将工具操作区分为 `Succeeded/Failed/Cancelled/UnknownOutcome`，并规定未知结果必须显式恢复决定、重试需新操作（L49-L95、L1525-L1588）。源 `invalid.ts` 没有持久化；Rust 侧只需把一次诊断结果作为普通已完成工具事件写入，不能把它标成原始副作用已成功。若原始调用在重写前已留下 operation marker，仍应保留 `UnknownOutcome` 证据，避免用 `invalid` 结果覆盖原调用的不确定性。
- **协议与 headless 投影：** `src/protocol.rs` 以有界 JSONL 传输（L1-L37），因此合成的 `error` 必须经过既有文本/字节上限与脱敏规则，不能无限复制 provider 错误。`src/headless.rs` 将 `ToolCall`/`ToolResult` 作为高优先级事件（L907-L920），并把 provider 的 `ToolCallDone` 映射成 queued 工具状态（L4339-L4364）；建议让修复后的 `invalid` 调用仍沿用原 `call_id`，在事件中标明原工具名和修复原因，避免 UI 只看到无关联的 `in
未决: 1. `Schema.Struct` 对未知字段的精确策略（保留、剥离或拒绝）及其版本差异，无法仅由 `invalid.ts` 确认。
2. 工具错误文本是否已在 LLM/provider 层脱敏、截断或带有调用上下文，需结合完整 session/provider 实现确认；本文件只保证模板字符串原样插入 `params.error`。
3. zenpi 是否要把 `title`/`metadata` 提升为 `ToolResult` 的独立字段，或统一保留在 `output` 对象中，取决于现有 headless 客户端契约；本源文件不能决定该协议选择。

## OC-043 packages/opencode/src/tool/lsp.ts
- `src/tools.rs`：建议新增只读 `LspTool` 实现，或拆出 `src/lsp.rs` 后由 `ToolRegistry` 注册。定义 `LspOperation`（九个枚举变体）、`LspRequest { operation, file_path, line, character, query }` 与 `LspResult { title, result: serde_json::Value, output }`；在 `ToolDefinition.input_schema` 保留 `line/character` 最小值 1，并将 `side_effect` 设为 `ToolSideEffect::ReadOnly`。`ToolContext` 的路径解析可承接工作区边界，但需额外允许“用于选服务器的存在文件”语义。
- `src/core.rs`：在工具预检/执行路径接入 LSP，复用现有 `ToolCall`、`ToolRegistry`、`ApprovalRequest` 和 `ToolExecutionEvidence`。差异是 TypeScript 每次都调用 `ctx.ask(permission='lsp')`，而 zenpi 的 `ApprovalPolicy` 可能自动放行只读工具；若要求严格一致，应新增 `lsp` 专用权限类别或在核心中强制产生可审计的只读审批记录。
- `src/tool_runtime.rs`：LSP 查询是单个有序调用，建议登记为 `ToolExecutionMode::Sequential`，通过 `execute_tool_batch` 的 `prepare` 阶段完成路径/权限/服务器可用性检查；取消时返回 `ToolErrorCode::Cancelled`，不要把“已发出请求”误判为可回滚。
- `src/runtime.rs`：若 LSP 客户端为长生命周期后台进程，可由 `BackgroundRunner` 承载启动与请求队列，使用 `CancellationToken` 在请求前、响应后和重试前检查；当前源文件没有重试，Rust 初版应保持一次提交一次终态。
- `src/protocol.rs` 与 `src/headless.rs`：若要暴露 JSONL 接口，增加受限的 `lsp` 工具调用或 `Command` 分支，复用现有 ID、路径长度和错误编码；`headless.rs` 的重放/终端缓存可记录最终结果，但必须明确这是 zenpi 扩展，因为源实现没有持久化。路径解析和 `file not found`/`no server` 应在命令进入 provider 前完成。
- `src/session.rs`：源实现无会话记录。若 zenpi 需要审计，使用 `OperationKind::Tool` 的 `begin_operation_with_key`/`finish_operation` 包围一次 LSP 请求；崩溃后按现有 `UnknownOutcome` 规则要求显式恢复决策，不能自动重放可能已抵达语言服务器的请求。
- `src/approval.rs`：映射 `ctx.ask` 的权限请求、`patterns=['*']` 和 `always=['*']` 到 `ApprovalRequest` 的工具名/参数/预览字段；保持只读副作用分类，且取消时清除 pending 请求。若采用默认只读自动批准，要在行为判据中记录与 opencode 的差异。
- `src/providers/**`：`anthropic.rs`、`openai.rs`、`google.rs`、`deepseek.rs`、`codex.rs` 及 `connection.rs` 只负责模型 provider 协议，不能把 LSP 操作编码成 provider 请求。应新增独立本地 LSP client/transport（例如 `src/lsp.rs`），由工具层调用；provider registry 仅继续提供模型能力，不参与 `workspaceSymbol` 等请求。

建议的可执行验证顺序是：先实现九项枚举和 Schema 约束，再加入工作区外路径、文件存在、客户端可用性三道门；用假的 `LspService` 断言九个方法的参数和顺序，测试 `line/character` 的 1→0 转换、`query.unwrap_or_default()`、标题/空结果文本，最后用 `CancellationToken` 和会话故障注入验证取消及未知结果不自动重试。
未决: 1. `LSP.Service` 各方法的具体返回类型、服务器选择算法、`touchFile` 是否启动进程，无法仅由本文件确认。
2. `Effect.orDie` 在当前宿主中如何被上层序列化为工具错误，需结合 Effect 运行入口验证。
3. `assertExternalDirectoryEffect` 是否解析符号链接、是否允许工作树外的只读路径，源文件只展示调用点，具体规则需查其实现。

## OC-044 packages/opencode/src/tool/mcp-websearch.ts
- 建议新增 src/mcp_websearch.rs（或放入现有工具模块）承载 ExaUrl/ParallelUrl 常量、McpResult、SearchArgs、ParallelSearchArgs、McpRequest<F> 的 serde 类型，以及 parse_payload、parse_response、call。用 serde/serde_json 做结构校验，用现有 HTTP 客户端抽象（若无统一抽象则在 providers 层补充）发送 POST。
- 对照 src/headless.rs：headless 已有工具生命周期事件、取消传播和外部证据/资源控制入口；MCP 调用应在既有工具工作 lane 中发出并把开始、完成、失败、取消映射到现有事件，而不是在 headless 中直接散落 HTTP。
- 对照 src/core.rs：Agent/ToolRuntime 与 AgentEvent::ToolCall/ToolResult 是注册远端 MCP 工具和回传结果的合适边界。可增加封装工具或 provider-backed handler，把 ToolResult::Success/Error 与 string | undefined 明确转换；保留 schema 校验失败和 HTTP 错误的可诊断错误码。
- 对照 src/session.rs：源 TS 不持久化；zenpi 若将调用纳入 agent 操作，应沿现有 operation/tool journal 记录 dispatch、finish、cancelled 和“副作用未知”状态，以支持恢复检查，不能把取消误当成远端回滚。
- 对照 src/tool_runtime.rs：该文件已有批处理、顺序/并行模式、ToolErrorCode::Cancelled 和可取消执行。将 call 作为单个可取消 handler；批量并行由 tool_runtime 决定，保持 TS 的“本函数无内部并发”语义；超时错误应落到明确的工具错误，而非 Rust panic/abort。
- 对照 src/runtime.rs：使用 CancellationToken 和 runtime job deadline 包裹 HTTP future/阻塞调用；区分“请求取消”“请求超时”“响应解析失败”。runtime 的 cooperative cancellation 不能假设中断远端请求已撤销。
- 对照 src/protocol.rs：若由 headless 客户端配置/调用 MCP，增加受限命令或工具参数载荷的 serde/长度校验；固定 JSON-RPC 字段可在内部构造，避免让 wire 输入任意修改 method 或 id。
- 对照 src/approval.rs：远端 MCP 属于外部副作用候选，应在 dispatch 前生成包含工具名、目标 origin、参数摘要的 ApprovalRequest，遵守现有 ApprovalPolicy；批准结果需和一次调用/operation 关联，不能因 URL 或工具名自动推断信任。
- 对照 src/providers/**：现有 providers/connection.rs 已集中 URL、HTTPS、凭据和允许目的地校验，MCP endpoint 应复用它或新增等价 allowlist；providers/* 目前主要描述模型端点，没有 Exa/Parallel MCP 适配器。建议新增 provider kind/adapter，仅允许配置的 EXA_URL/PARALLEL_URL，将 API key 放入受控 secret/header 或 URL 生成器并避免日志泄露。
- 可执行差异清单：TypeScript Effect schema → Rust serde + 显式错误枚举；Effect.die 超时 → ToolErrorCode::CommandTimeout/专用 McpTimeout；SSE 解析必须保留精确 data: 前缀和首个 truthy 文本规则；固定 id=1 的请求格式需测试；Rust 需补充 URL 安全、响应大小上限、取消竞态和 journal 记录，这些均不是源文件提供的能力。
未决: 1. HttpClient.filterStatusOk 对非 2xx 的具体错误类型和是否保留响应正文，源文件未说明。
2. Duration.Input 的允许单位/零值行为由 effect 版本实现决定，源文件只表明由调用方传入。
3. MCP 服务端是否把 type 字段用于其他语义、是否可能返回多条可用文本，当前实现只取首个非空 text，无法从本文件确认。

## OC-045 packages/opencode/src/tool/plan.ts
- 会话承载可落在 src/session.rs 的 SessionStore（L146）及 append_turn（L863）、append_event（L1207）、events（L1223）。zenpi 的持久化是追加式 JSONL，append_turn 先持久化再更新内存投影；建议新增一个经过校验的 PlanApprovalTurn/BuildHandoff 记录，原子地表达“批准”和合成提示，避免 TS 中 message/part 两次写入造成的半完成状态。
- agent 状态可映射到 src/core.rs 的 Agent（L403）、AgentSnapshot（L328）和 AgentError（L355）。增加 Agent::request_plan_exit 或 Agent::approve_build_handoff：输入 session id、计划路径和确认结果；确认成功后设置 build 区域/agent 状态并追加会话记录。现有 set_model（L1167）及 restore_model_selection（L1277）可支持“沿用最近模型，否则 backend 默认模型”，但应明确把默认模型读取做成纯查询。
- 工具注册与结果应放进 src/tools.rs 的 ToolDefinition/ToolRegistry（定义约在 L505、L1057），调用批处理由 src/tool_runtime.rs:execute_tool_batch（L168）承载。建议定义无参数 plan_exit，返回 ToolResult::Success 的 title/output/metadata；用户拒绝应映射为可识别的拒绝错误，不能把正常拒绝伪装成 panic。与 TS 的 Effect.orDie 相比，Rust 应保留 Result 的结构化错误。
- 交互确认可复用 src/approval.rs:ApprovalCoordinator::request_response（L185）与 cancel_all/emergency_cancel（L409-L445），或定义专用 PlanExitApproval。差异是 zenpi approval 主要围绕工具副作用策略（ApprovalPolicy），而本源是面向用户的二选一问题；应在 ApprovalRequest.origin 中区分 plan handoff，禁止 remembered tool grant 误替代一次性 build 确认。
- 协议入口可扩展 src/protocol.rs 的 Command（L214）和 StdioRequest 的 kind/text/session_id 字段；建议加入 plan_exit/build_handoff 命令，校验 session、计划相对路径和确认值。现有 prompt/steer/cancel 解析及 target_id 校验可作为边界模板。若以既有 handoff 命令表达，应把“用户确认”和“实际切换”拆成两个可审计阶段。
- 取消与并发应接入 src/runtime.rs:CancellationToken（L56-L86）以及工具批次的取消检查（src/tool_runtime.rs L168-L313）。在等待用户回答、读取历史、写入两条记录前后检查 token；取消不回滚已提交 journal，返回 cancelled，并保证不会启动 build turn。runtime 的 worker 完成/取消竞态应保持单一终态。
- src/headless.rs 已提供会话 owner、重连和事件投递边界；实现 handoff 时应在 owner 持有的 Agent 上串行更新，向 headless event 输出 admission/approval/terminal 事件。不要直接从工具线程修改共享 session。
- src/providers/** 仅负责 backend/model；不要把 plan_exit 的确认逻辑放入 provider。provider 只需提供当前模型/默认模型查询，和源文件中的 Provider.defaultModel() 对应。
- 可执行差异清单：① zenpi 当前没有 TS 的 Question.Service 和 SessionV1.User/TextPart 二层消息 API，需要设计一个 Rust 会话事件/turn schema；②源实现将拒绝抛为致命 Effect，zenpi 应结构化返回并可被 headless 协议观察；③源实现接受任何非精确 No 的回答，Rust 应明确采用兼容还是严格枚举并写测试；④源实现两次写入可能留下中间态，Rust 建议单次 append handoff 或可恢复的 operation intent；⑤路径应限制为相对 workspace 的安全表示，避免直接复刻可含 .. 的 path.relative。
未决: 1. Session.plan(info, instance) 的具体计划文件命名、是否保证存在以及是否总在 instance.worktree 下，无法由本文件确认。
2. Question.Service.ask 返回数组的完整类型、空回答的上游约束、以及 RejectedError 被运行时如何渲染，源文件未定义。
3. session.updateMessage/updatePart 的幂等性、写入格式和跨进程并发策略未在本文件确认。
4. agent: "build" 是否会自动触发后续 provider turn，还是仅作为下一轮提示，需要查看调用方/调度器才能确定。
5. MessageV2 的未使用导入是否由编译器配置豁免，无法从本文件确认。

## OC-046 packages/opencode/src/tool/question.ts
建议新增 src/question.rs（或 src/tool_runtime.rs 的独立子模块）：

- 类型落点：QuestionPrompt { question, header, options, multiple }、QuestionAnswer(Vec<String>)、QuestionRequest { id, session_id, questions, tool }、QuestionError::{Rejected, NotFound, Cancelled}、QuestionService。用 serde 加显式 validate 复刻 Question.Prompt 与 Answer 的数组语义；答案长度不要假定与问题数相等，格式化时使用安全索引。
- 函数落点：QuestionService::ask 创建递增 ID、登记 pending、发出 question_asked；reply/reject 以 request ID 唤醒等待者并清理 pending；list 返回当前 pending。QuestionTool::execute 负责调用 ask、按索引生成 Unanswered、构造 Asked N question(s) 和 output。Rust 应返回 Result<ToolResult, QuestionError>，明确记录 TS Effect.orDie 把拒绝转 defect 的差异。
- src/core.rs：Agent 已持有 session 与 input_port（L402-L430、L598-L625），可注入 QuestionService；将 question 事件作为会话级交互，而不是 TurnInputRequest（L207-L240）这种新的用户 turn。工具调用完成后仍由 core 负责把结果回写当前 turn。
- src/headless.rs：现有异步请求/响应和 input queue 有 pending ticket、回执及关闭时未知结果处理（L5253-L5320、L5609-L5629）。可增加 question_request/question_reply 事件流和 session 关联，复用 ticket 的异步回执模式；关闭时应拒绝未答问题并输出明确终态。
- src/protocol.rs：现有 InputQueueAction 仅承载文本 enqueue/edit/cancel/list/configure（L1035-L1107），InputQueueRequest 严格校验版本、ID 和大小（L1119-L1158）。建议新增严格的 QuestionReply { request_id, answers } 与 QuestionReject 命令，限制 ID、答案数组和总字节数；不要把结构化问题偷偷降级成普通 Prompt。
- src/session.rs：append_event 是可扩展的不透明事件入口（L1205-L1220），可持久化 question_asked、question_replied、question_rejected，并在恢复时重建未完成 pending。append_turn 的“先持久化、成功后更新内存”原则（L861-L884）可作为答案提交的耐久边界。
- src/runtime.rs：CancellationToken 是协作式、幂等取消，后台 runner 对 queued/active job 有明确终态（L49-L99、L156-L190）。ask 应绑定 token；取消需从 pending 移除并返回 Cancelled，不能把取消当作用户拒绝，也不能假设线程会被强杀。
- src/tool_runtime.rs：现有批处理按调用顺序产生结果、限制批量大小，并在取消后阻止后续调用（L150-L176、L219-L249）；question 属于只读、可等待的交互工具，建议作为单次 ToolCall 结果接入，避免把多个问题误当成可并行副作用。输出仍应保留结构化 answers metadata，不能只保存拼接后的文本。
- src/approval.rs：ApprovalCoordinator 的 pending/visible/accepted 是“是否允许副作用”的决策流（L127-L151、L167-L245），与用户回答问题不同；不要复用 approval decision 表示答案。若问答工具被纳入权限策略，只在调用前做工具许可，答案通道仍由 QuestionService 管理。
- src/providers/**：providers/mod.rs 仅定义 provider 协议、认证头和能力路由（L13-L49、L136-L160），各 provider 文件只提供 endpoint definition；question 不应下沉到 OpenAI/Anthropic/Google wire codec。可在 provider 无关的 tool schema 注册处暴露 question，并用现有 provider 端到端测试验证模型收到的是工具定义而非 provider 特有消息。

主要差异清单：TS 使用 Effect 环境注入和内存 Deferred，Rust 需显式 Arc<Mutex<HashMap>> 加 Condvar 或 oneshot；TS 不持久化问答，zenpi 的 session 是 append-only durable journal；TS 不检查 abort/超时，zenpi 必须用 CancellationToken；TS 的 Effect.orDie 隐藏了 RejectedError 的业务返回，Rust 应保留机器可判定错误码；TS 只按 answers[i] 格式化且不转义引号，Rust 实现应固定相同外显格式，同时对 JSON/文本边界做严格编码。
未决: - Question.Prompt 的最终约束由 @opencode-ai/schema/question-v1 提供；本文件只能确认其被引用，无法单独确认运行时是否会拒绝空 options、超长 header 或缺失可选字段。
- Effect.orDie 在当前应用运行时是否会被统一捕获并转成工具错误事件，需结合工具执行器和部署层验证；源文件本身没有定义该映射。

## OC-050 packages/opencode/src/tool/shell.ts
- src/tools.rs::RunCommandTool（现有 L2219 起）是最接近落点：它已有 invoke_with_cancel/invoke_user_shell_with_cancel、命令超时、进程组清理、stdout/stderr 限制和 artifact capture。应新增独立 ShellTool 或扩展该类型，加入 tree-sitter 等价扫描、PowerShell/CMD 分支、合并输出和工作目录审批。
- src/tool_runtime.rs 负责批量准备、顺序/并行执行和取消传播；把本文件的“审批后才 spawn”对应到 ToolBatchDecision::ExecuteApproved，将超时映射为 ToolErrorCode::CommandTimeout，将 kill 后不确定结果记录为取消/未知，而非假设回滚。
- src/approval.rs 的 ApprovalCoordinator/ApprovalRequest 对应 ctx.ask；建议把命令模式、外部目录 glob、原始 command 放入 arguments/preview，并维持“目录授权”和“命令授权”两个可审计请求。
- src/runtime.rs 的 CancellationToken、BackgroundRunner 对应 Effect scope、raceAll 和子进程监督；需要在 worker 中轮询 token，并在超时/取消时先终止 process group 再 join 输出读取线程。
- src/core.rs 负责 turn/tool admission、ToolContext 和副作用策略；在这里绑定 session/call ID、workspace root、policy digest，拒绝路径越界和未审批 command。
- src/headless.rs 是 JSONL wire owner；将 shell 的中间 output metadata 映射为有界 StdioEvent，将最终 title/output/exit/truncated/outputPath 映射为终端响应，避免把诊断写入 stdout。
- src/protocol.rs 已有 UserShellRequest、MAX_USER_SHELL_BYTES 和版本化事件；若要暴露 workdir/timeout，应新增显式字段及校验，拒绝未知字段、负数和超出策略上限的值。
- src/session.rs 适合持久化命令 operation marker、取消/超时结论和 output artifact 引用，支持 crash 后 UnknownOutcome；本 TypeScript 文件本身没有恢复语义，Rust 不应把取消误报为成功。
- src/providers/**（openai、anthropic、google、deepseek、codex、registry）只负责 provider 请求/流，不应直接执行 shell；provider 返回的 tool call 应经 core -> approval -> tool_runtime -> RunCommandTool 链路。
- 可执行验证差异清单：为 Bash/PowerShell/CMD 各建路径扫描测试；验证 ~、环境变量、filesystem::、动态表达式和 glob 前缀；验证外部目录越界审批；验证 UTF-8 尾部裁剪；验证超时/取消下 process group 无遗留；验证 artifact 写入和 session 恢复；验证插件环境覆盖；验证 Windows cygpath 分支（Rust 当前 RunCommandTool 主要是 Unix /bin/sh -c，存在平台差异）。
未决: 无。

## OC-051 packages/opencode/src/tool/shell/id.ts
- `src/core.rs:L2941-L2941` 的 `Agent::run_user_shell_with_cancel` 是当前用户 shell 所有者：要求单个 `!` 前缀、检查 agent 空闲态、构造 `ToolOrigin::UserShell`，执行审批/策略门控，并在 `src/core.rs:L3125-L3125` 调用 `RunCommandTool::invoke_user_shell_with_cancel`。建议在 `src/tools.rs` 或新的 `src/shell_id.rs` 放置 `#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum ShellKind { Bash, Pwsh, Powershell, Cmd }`、`pub const TOOL_ID: &str = "bash"` 与 `pub fn to_kind(value: &str) -> ShellKind`；未知值返回 `ShellKind::Bash`，保留本文件的大小写敏感和不报错语义。
- `src/tool_runtime.rs:L37-L37` 的 `MasterSessionCommand::{Bash, Steer}` 与 `src/tool_runtime.rs:L83-L83` 的 `classify_master_session_input` 是输入分类层。它把 `!` 后文本交给现有用户 shell，并限制长度/控制字符；不要把该分类器误当成 shell 名称归一化。可让其调用 `shell_id::TOOL_ID` 生成稳定权限键。
- `src/protocol.rs:L689-L689` 的 `parse_user_shell_input` 负责线协议验证：要求 `!`、拒绝 `!!` 和非法控制字符。建议保持协议错误与 `to_kind` 的未知值回退分离：命令输入错误仍返回 `ProtocolError`，shell 名称未知才回退 `Bash`。
- `src/runtime.rs:L56-L91` 的 `CancellationToken` 及运行队列只负责作业取消/完成竞争；本文件映射函数应保持纯函数，不持有 token。上层在启动命令前后继续检查 `is_cancelled`，避免把 shell 类型解析和取消语义耦合。
- `src/approval.rs:L41-L41` 的 `ApprovalRequest` 与 `src/approval.rs:L185-L185` 的 `request_response` 为副作用审批边界；`core.rs:L3059-L3059` 的 `persist_accepted` 在执行前持久化批准。建议权限键继续使用字面量 `"bash"`/ `TOOL_ID`，不能用 `ShellKind::Cmd` 替换，否则会破坏已有权限和插件兼容性。
- `src/session.rs:L51-L76` 定义 `OperationKind`、`OperationOutcome`、`InterruptedOperation`；`src/session.rs:L1414-L1414`、`src/session.rs:L1453-L1453` 的 `begin_operation`/ `finish_operation` 支持命令前写入和结果收尾。该文件本身无需新增持久化记录；若 Rust 端增加 shell 类型字段，应只记录解析后的值和稳定 `TOOL_ID`，并保持未知值回退可重放。
- `src/headless.rs:L1569-L1579` 将 master console 的 `Bash` 转换成 `Command::UserShell`，`src/headless.rs:L4606-L4606` 等路径传入取消谓词。这里应调用统一 `to_kind`（若需要展示/分析实际 shell），但执行和审批仍走现有 user-shell owner，不能在 headless 层绕过它。
- `src/providers/**`（尤其 `src/providers/registry.rs:L143-L143` 的 `ModelRegistry` 与 `src/providers/registry.rs:L366-L366` 的 `resolve`）只管理 provider/model 身份、能力和价格，不应承载 `ShellKind` 或 `ToolID`。建议差异清单明确：opencode 的 shell ID 模块是工具层常量；zenpi 当前 provider registry 没有同等抽象，需要新增小型、无 I/O 的 shell identity 模块并由 `core/tools/headless` 复用。
- 可执行验证：新增 Rust 单元测试覆盖四个精确匹配、未知/大小写/空白回退、`TOOL_ID == "bash"`、并发只读调用；再运行现有 core/protocol/headless 测试，确认审批事件、`OperationOutcome` 和取消结果不变。

差异清单：TypeScript 使用字符串字面量联合与运行时 `Set`；Rust 需要显式 enum/匹配。TypeScript 的默认回退是静默 `"bash"`；Rust 不能用 `Result` 错误替代该默认。TypeScript 的 `ToolID` 是值和同名字面量类型；Rust 应用 `const TOOL_ID` 加类型化 `ShellKind`，避免把 enum 当权限键。TypeScript 通过命名空间重导出；Rust 用模块路径 `crate::shell_id::{...}`。两者都应把 shell 识别保持为纯、无持久化副作用的边界。
未决: 无。源文件没有说明 `Shell.name(shell)` 的所有可能返回值，也没有定义 `sh`、`zsh` 等别名是否未来加入；当前可确认的契约是只有四个精确值，其余统一回退 `"bash"`，重命名计划在 opencode 2.0 前保持兼容。

## OC-053 packages/opencode/src/tool/skill.ts
- 技能模型/加载：zenpi 的 src/skills.rs 已有 SkillSet、SkillMetadata、SkillBody、SkillResource、ModelSkillTools。建议把 Parameters 映射为 SkillToolArgs { name: String }，把 Skill.require 映射为 SkillSet::load_body(name, SkillInvocation::Model, "", cancelled)，保留名称校验、正文读取和 SKILL.md 排除。
- 工具注册/执行：在 src/tools.rs 或 src/skills.rs 的 ModelSkillTool 旁增加只读 skill 定义，或将现有 load_skill 作为兼容别名；执行落点可放在 src/tool_runtime.rs 的 dispatch/execute_cancellable 路径，保证先审批后扫描。tool_runtime.rs 已提供批量决策和取消回调，可把 ctx.abort 映射为 cancelled()。
- 核心策略：src/core.rs 的 prepare_tool 已检查 skill allowlist、ToolContext::check_call_gate 和取消；应保持未知技能为拒绝/失败，并在 persist_tool_invocation 记录结果。TS 的 Effect.orDie 是 defect 语义，Rust 可映射为 ToolError::InvalidCall 或 ToolError::Cancelled，但协议层应区分取消、拒绝和内部错误。
- 审批与副作用：src/approval.rs 的 ApprovalCoordinator/ApprovalPolicy 对应 ctx.ask；ApprovalRequest 的 tool 应为 skill，arguments 仅含 name，再把拒绝作为终止结果。工具是只读扫描，仍须经过 zenpi 的审批/Blueprint gate。
- 会话/恢复：src/session.rs 是追加式 JSONL 持久化；源工具没有持久化，因此不应新增恢复记录。若统一工具审计，可沿用 tool_execution_finished，标注只读成功或取消，不伪造可恢复操作。
- 运行时/协议/无头：src/runtime.rs 的 BackgroundRunner 可提供有界队列和终止事件；src/protocol.rs 的 Command/工具结果负责传输结构化输出；src/headless.rs 负责 JSONL、重放和关闭，不应把技能正文无界写入缓存。建议结果保持 title、output、metadata.name、metadata.dir，并限制正文及列表大小。
- providers/**：src/providers/** 仅负责模型连接和路由，不应实现技能扫描。模型产生工具调用后仍由 core.rs → tool_runtime.rs 执行；验证点是 provider 只看到工具 schema 和结果，不直接获得文件系统权限。
- 可执行差异清单：1) 增加 skill 工具 schema 与 name 描述；2) 复用 SkillSet 读取正文并计算 dir；3) 接入 ApprovalCoordinator 后再调用受取消控制的 ripgrep/搜索实现；4) 排除 SKILL.md、允许隐藏文件、禁止跟随链接、限制十项并输出绝对路径；5) 为未知技能、审批拒绝、取消、扫描错误补充单元/集成判据；6) 明确 Rust 错误映射，避免把失败静默成成功。
未决: - Skill.Service.require 返回的 info.location 是否始终为文件路径、是否已做路径安全校验，源文件无法确认。
- Ripgrep.Service.find 的排序、glob 语义、错误类型以及 signal 取消时的具体错误标签未在本文件定义。
- ctx.ask 是否阻塞、如何表示拒绝/超时、always 决定是否持久化未在本文件定义。
- DESCRIPTION（./skill.txt）的实际文本和 Tool.define 的生命周期/错误包装细节需查看对应实现才能确认。

## OC-054 packages/opencode/src/tool/task.ts
- `TaskPromptOps` 建议落在 `src/core.rs` 的 `SubagentTaskOps` trait（对照 `Agent::process_with_cancel` L5327-L5351、`run_active_turn_cancelable` L3642-L3678），实现 `resolve_prompt_parts`、`prompt`、`cancel`，由 `Agent`/会话宿主注入；不要把 provider HTTP 细节塞进 task 调度器。
- `Parameters` 对应 `src/protocol.rs` 新增带 `description/prompt/subagent_type/task_id/command/background` 的 `TaskRequest`，复用 `MAX_TEXT_BYTES`、ID 校验和 `Command` 解析（L214-L265、L601-L653）；background 功能开关应在解析/能力声明处体现，而不是只在执行后拒绝。
- `BackgroundJob` 对应 `src/runtime.rs` 的 `BackgroundRunner`、`CancellationToken`、`JobOutcome`（L49-L93、L158-L185、L237-L321）。`start/extend/wait/waitForPromotion` 需要一个按 session/job ID 的表层 adapter；队列满映射 `SubmitError::QueueFull`，关闭映射 `Closed`。前台 race 可用事件接收与 promotion 状态实现。
- 父子 session 可复用 `src/session.rs` 的 `SessionStore`、`SessionRecord` 和已有 `Turn.parent_id`（L146-L166、L327-L335），但当前没有与 TypeScript `parentID/agent/permission` 等价的 session 树字段，建议新增结构化 session metadata/event，并为 task_id 恢复加入“存在且属于当前父会话”的校验，避免源实现静默降级造成误复用。
- `ctx.ask` 映射 `src/approval.rs` 的 `ApprovalCoordinator::request/request_response`（L159-L219），`subagent_type` 作为受控 permission pattern；拒绝/取消必须转为结构化 `AgentError`，并在执行前持久化 approval 结果。
- 子 agent 模型选择对照 `src/providers/mod.rs` 的 `Protocol`/route（L22-L104）与 `src/providers/registry.rs` 的 `ModelDescriptor`/`ModelRegistry`（L59-L147）。zenpi 当前 provider registry 只描述 provider/model 能力，没有 agent type catalogue、agent 专属 model、variant 继承策略；建议新增 `AgentSpec {name, model, permissions}`，模型缺省时沿父 turn 的 provider+model。
- 工具权限执行对照 `src/tool_runtime.rs::execute_tool_batch`（L168-L347）和 `src/core.rs` 的工具准备/取消路径：task 子 session 的 deny 规则应在 `ToolContext`/registry 构造时落地，不能只在 prompt 文本中提示。`src/tool_runtime.rs` 的取消结果是“未执行”，可作为子任务 cancelled 的判据。
- 后台事件/协议返回对照 `src/headless.rs` 的 `run_async_streams`（L806-L837）及 bounded event mailbox（L862-L1065），把 running/completed/error 和 job_id 编成 `StdioEvent`/`StdioResponse`，并沿用重放与容量限制；`renderOutput` 可实现为 `TaskResult` 结构后再序列化，避免手写未转义 XML。
- 取消与关闭必须接入 `src/runtime.rs` 的 grace/shutdown 语义（L379-L424）：协作式取消不是回滚，超出 grace 的任务可 detach；需要明确 task session 的 terminal event，防止 headless 重连将未知结果误判为成功。
- `src/session.rs` 的 `OperationOutcome::{Succeeded,Failed,Cancelled,Interrupted,UnknownOutcome}`（L51-L96）可承载任务终态，但源文件没有重试/幂等策略；若 Rust 增加 retry，必须增加显式用户确认或 idempotency key。`src/headless.rs` 的 session/replay 持久化可承载通知，但源实现的 synthetic parent prompt 需要单独 event 类型。
- `src/providers/**`（`anthropic.rs`、`google.rs`、`connection.rs`、`registry.rs` 等）只负责协议路由、能力和模型目录，不应直接实现子 agent 生命周期；验证点是 task 调度器能在 provider 不变和 provider 切换两种模型选择下生成相同的 session/permission 记录。
未决: - `Tool.define`、`BackgroundJob.Service`、`Session.Service`、`Agent.Service` 的具体实现、锁粒度、队列容量和重连语义不在本源文件中。
- `DESCRIPTION`（task.txt）的完整文本及 `Tool.Context`、`ctx.ask/metadata/abort` 的错误/持久化保证未在此确认。
- `deriveSubagentSessionPermission` 的冲突优先级、`next.permission` 的默认规则，以及 `task_id` session 是否必须属于当前父 session 未由本文件约束。
- `background.extend` 的“更新”是追加 prompt、复用未完成 job 还是恢复持久化 job，及 `waitForPromotion` 的竞态优先级需查看 BackgroundJob 实现。
- `MessageV2.get`、`ops.prompt` 返回 parts 的顺序保证、tool error 的数据结构和 provider 重试策略需由相应模块确认。

## OC-055 packages/opencode/src/tool/todo.ts
- src/tools.rs（核心落点）：新增 TodoItem { content: String, status: String, priority: String } 与 TodoWriteArgs { todos: Vec<TodoItem> }，为 ToolRegistry 注册 todowrite。输入 schema 要求对象和 todos 数组，保持源实现“不在工具层限制长度/状态枚举”的差异。工具执行先走现有 ToolContext/approval gate，再调用会话服务；成功返回 ToolResult::Success，output 用 serde JSON 两空格格式，metadata 携带列表，title 复现未完成计数。副作用分类应明确选择现有 ToolSideEffect::WorkspaceWrite，或新增专用 SessionWrite；不能误标为 ReadOnly。
- src/session.rs（持久化与事件）：增加按 session_id 替换 todo 列表的接口（建议 replace_todos），在同一持久化边界完成删除旧列表、写入带 position 的新列表，并追加可重放的 todo.updated 事件。现有 SessionStore::append_event（L228-L239）可作为审计通道，但要补充读取当前 todos 的接口，保证恢复和查询可验证。
- src/core.rs（注册和会话归属）：在 set_tools/工具运行时装配链（L1384-L1425）注册该工具，执行时从 ToolContext 取得并校验当前 session_id，再把工具调用/结果纳入既有 AgentEvent::ToolCall/ToolResult。若 agent 非 Idle 或会话已切换，应由现有 admission gate 拒绝，避免写入旧会话。
- src/tool_runtime.rs（批执行）：将 todowrite 标记为 whole-batch sequential/mutating，确保同一批及相邻只读调用不会并发重排列表。沿用 execute_tool_batch 的 cancellation predicate；源行为没有重试，因此不要为失败的 todo update 自动重放，且在取消时返回明确的取消错误。
- src/runtime.rs（取消边界）：把 CancellationToken 映射到工具执行前、权限等待中、update 前的检查；mark_completed 后的晚到取消不能把已提交结果改成 Cancelled。源文件没有 timeout，Rust 侧若增加超时必须记录为架构差异并验证事务回滚。
- src/protocol.rs（线协议）：优先复用现有 typed tool-call JSON；若要提供独立 todo 命令，加入带 session_id 的请求/结果类型并使用已有 identifier 校验。响应字段应能表达 title、pretty JSON output、todos metadata 和错误，不要把 provider 原始参数直接当作已授权写入。
- src/approval.rs（权限）：把工具名映射到 todowrite 权限决策，通配模式对应源里的 patterns:["*"] 与 always:["*"]。拒绝、取消、持久化失败应与现有 ApprovalError/事件审计衔接；不可因工具名自动推断允许。
- src/headless.rs（宿主/展示）：在 headless JSONL 和 TUI 事件投影中识别 todowrite 的 ToolCall/ToolResult，展示 title 和列表摘要；session owner 校验沿用现有 active session 逻辑。不要在 headless 层复制 update 事务，避免绕过 core 的 approval/cancellation。
- src/providers/**（provider 适配）：现有 anthropic、google、openai、codex、deepseek 等模块只负责协议路由、鉴权和模型响应，不应直接写 todo。它们只需接收注册后的工具 schema 并产生标准 tool call；参数解码、权限、会话副作用均留在 tools.rs/core。可验证差异是：同一 todowrite JSON 在各 provider 方言转换后仍保持数组顺序和字段值不变。

可执行验证集：Rust 单测覆盖 schema 缺字段、空数组、完成计数、权限拒绝零写入、update 失败无成功结果、同会话并发串行化、取消前后边界、session 重启后列表恢复；集成测覆盖每个 provider 产生相同 ToolCall。这些是建议的落点，源文件本身没有 Rust 类型或协议约束。
未决: 1. 仅凭本文件无法确定 ctx.ask 是否监听 ctx.abort，以及权限请求的超时策略。
2. 无法从本文件确认 Todo.Service.update 的数据库隔离级别、事件发布失败后的处理；这些属于服务实现。
3. 无法确认 Tool.define 上层是否会截断 output、附加 tracing 或把 Effect defect 转成模型可见错误。
4. 无法确认同一 sessionID 的并发 todowrite 调用是否在调用方排队；文件内没有锁或顺序令牌。
5. 无法确认 provider 是否对工具参数做额外字段过滤；本文件只定义 Effect schema，不控制各 provider 的 wire 编码。

## OC-056 packages/opencode/src/tool/tool.ts
- `src/tools.rs` 的 `ToolDefinition`、`ToolContext`、`ToolResult`、`ToolRegistry`（约 L505、L747、L1057 及 L1196-L1324）是最接近的运行时落点：将 `Def` 映射为含 `name/description/schema/execute` 的 trait 对象，将 `ExecuteResult` 映射为 `ToolResult { title, output, metadata, attachments }`，把 `Context.sessionID/messageID/callID` 映射到现有 `ToolContext` 的关联字段。
- `src/tool_runtime.rs` 的 `execute_tool_batch`（文件前部）已负责批量校验、顺序/并行模式、取消传播和结果按源顺序收集。建议新增 `ToolSpec`/`ToolInit` 或在 `ToolDefinition` 中加入一次性 JSON schema 编译结果；在 dispatch 前执行与 `Schema.decodeUnknownEffect` 等价的 `validate_call`，失败返回结构化 `InvalidArguments`，禁止进入 registry execute。
- `src/runtime.rs` 的 `CancellationToken`、`InputBoundaryGate`（L49-L100、L769-L910）可承接 `Context.abort`：建立 `AbortSignal` 等价的只读取消视图，在每个 provider chunk、重试前和工具边界检查；取消不应回滚已开始的副作用，符合现有 runtime 注释与 `JobOutcome::Cancelled` 语义。
- `src/approval.rs` 的 `ApprovalCoordinator::request/request_response`（L126-L220）对应 `Context.ask`。Rust 适配器应从 `ToolContext` 构造 `ApprovalRequest`，保留 `session_id/turn_id/call_id/tool` 关联，并在副作用前持久化决定；不要让 schema 解码错误绕过审批审计。
- `src/core.rs` 的工具调用准备、`ToolInvocationOutcome`、operation evidence 和 turn metadata（如工具调用处理区）对应 `Effect.orDie` 外层编排及 `ExecuteResult.metadata`。建议把 `truncated`、`output_path` 和附件写入 `Turn.metadata`/`SessionStore`，并在恢复时尊重已存在的 `truncated` 标记以保证幂等。
- `src/session.rs` 的 `Turn.metadata`、附件/操作 journal 与 `OperationRecovery` 是持久化落点；`tool.ts` 本身没有恢复算法，Rust 应由 session owner 记录 started/finished/unknown outcome，并在取消后保留未知副作用状态。
- `src/headless.rs` 与 `src/protocol.rs` 负责 JSONL 相关请求和错误投影。可将 `InvalidArguments` 序列化为带 `id/call_id/tool/detail` 的 correlated tool result，保持协议版本与请求 ID；不要把异常文本直接写 stdout 诊断通道之外。
- `src/providers/**`（尤其 `registry.rs`、`connection.rs` 及各 provider adapter）只应提供模型/流式事件和调用参数，不复制工具 schema 逻辑。provider 返回的 tool call 交给 `ToolRegistry` 解码，`Context.abort` 接到 provider stream 的取消；provider metadata 可并入 `ExecuteResult.metadata`，但截断与审批仍由 core/tool runtime 统一处理。

可执行差异清单：1）TypeScript 的 `Effect` 环境注入需在 Rust 中改为显式 `&ToolRegistry`、`&AgentCatalog`、`&Truncator` 参数或 trait；2）`Schema.Decoder`/`JSONSchema7` 双表示需统一验证入口并保留可选 schema；3）`orDie` 的 defect 语义需映射为可观测 `ToolError::Internal`，同时保留 `InvalidArguments` 的可恢复、模型可读错误；4）`Metadata` 的任意值需用 `serde_json::Map<String, Value>` 并限制大小；5）`Effect` 无内建并行，批量并行由 `tool_runtime.rs` 明确控制，且单个副作用失败后的停止规则要与现有 batch policy 对齐。
未决: - `Truncate.output` 的具体字节/行阈值、`outputPath` 的持久化位置不在本文件中。
- `Agent.Service.get` 对未知 `ctx.agent` 的错误类型与可恢复性需查 `agent/agent` 实现确认。
- `Effect.orDie` 在项目的统一运行时/日志层如何呈现 defect，单凭本文件无法确定。
- `PermissionV1.Request` 的剩余字段及 `Context.ask` 的审批超时由外部实现决定。

## OC-057 packages/opencode/src/tool/truncate.ts
zenpi 已有最接近的落点是 `src/tool_output.rs`（由 `src/lib.rs` 导出）：`OutputLimits`、`SessionOutputStore`、`CommandOutputCapture`、`OutputProgress` 和 `OutputError` 已提供受限文件存储、TTL、取消读取与恢复清理；`src/core.rs:L993-L1148` 的 `tool_output_control`、`read_tool_output`、`cleanup_tool_output` 负责协议入口和带 journal receipt 的清理。建议把本文件的“预览算法”单独落在 `src/tool_output.rs` 或新建 `src/tool_truncate.rs`：定义 `TruncateOptions { max_lines: Option<usize>, max_bytes: Option<usize>, direction: HeadTail }`、`TruncateResult { content, truncated, output_path }`、`resolve_limits(config)`、`truncate_preview(text, options)` 与 `write_full_output(...)`，并用 Rust `text.as_bytes().len()`、`char_indices`/安全 UTF-8 边界实现可验证的字节裁剪。

- `src/core.rs`：在 `run_command` 捕获流程（现有 `begin_output_capture`/`persist_output_capture`）之后生成 bounded preview；保留现有 `OutputError`、磁盘治理和 `tool_output_captured` 事件，不直接复刻 TypeScript 的裸路径字符串。
- `src/tool_runtime.rs`：执行器已有批量边界、策略和“不中途重试”语义；可让截断作为结果格式化阶段，不能让 preview 截断改变工具执行或 side effect 判定。
- `src/runtime.rs`：`CancellationToken` 是取消映射点。预览和写入前后检查 `is_cancelled()`；取消不得伪装成成功的 `TruncateResult`，并遵循现有 worker 的 cooperative cancellation 与 detach 语义。
- `src/headless.rs` 与 `src/protocol.rs`：已有 `ToolOutput(OutputRequest)`、`read_tool_output` 控制命令、JSONL 关联响应和事件字节上限（`src/headless.rs` 的异步 mailbox 上限）。建议新增显式 `preview` 字段或复用 `OutputSnapshot`，不要把绝对内部路径直接暴露到 provider/协议层；若暴露 `outputPath`，应改为现有 artifact ID/受信引用。
- `src/session.rs`：现有 journal 大小和恢复边界可承载 `tool_output_captured`、cleanup intent/receipt；应把截断文件生命周期与 session 绑定，并复用已存在的 pending cleanup 恢复，而不是依赖每小时 fiber 才能找回状态。
- `src/approval.rs`：该文件处理 side-effect approval；截断写盘属于宿主内部输出保存，不应绕过工具审批把它当作新的 workspace write。若策略要求审批，复用既有 `ToolSideEffect` 判定，不能从 provider 名称推断权限。
- `src/providers/**`：provider 模块（Anthropic/Codex/DeepSeek/Google/OpenAI、connection、registry）负责模型身份、协议和能力，不应实现截断算法。只需确保 provider 请求/响应的 bounded text 使用 `TruncateResult` 的预览，并保留 provider 原始调用 ID 以便 `ArtifactRef` 关联；provider 配置中没有 TypeScript `tool_output.max_lines/max_bytes` 的直接对应项，需在 zenpi config 中新增并做范围校验。

关键差异：TypeScript 默认是 2000 行/50 KiB、7 天全局目录清理、成功截断后返回可读提示；zenpi 当前 artifact 默认是每 session 私有目录、24 小时 TTL、64 KiB view/read 预算，并有私有 inode、哈希、完成标记、持久化 receipt、取消和恢复机制。Rust 映射应保留这些更强的安全与恢复约束，不能简单复制 `path.join(TRUNCATION_DIR, ToolID.ascending())` 的任意路径模型；同时补齐 `head`/`tail` 的精确字节与行判据。
未决: 1. `TRUNCATION_DIR`、`ToolID.ascending()` 和 `Config.Service` 的具体实现及路径权限不在本文件中，无法确认目录是否跨进程共享、ID 是否必然以 `tool_` 开头。
2. Effect `Schedule.spaced` 在该版本中的首次 tick 时序、以及 `catchCause` 对 fiber interruption 的具体传播，需要结合运行时版本验证。
3. 配置允许零或负 `max_lines`/`max_bytes` 是否是有意行为，源文件没有约束或错误分支。

## OC-059 packages/opencode/src/tool/webfetch.ts
- 工具落点：在 src/tools.rs 的 Tool/ToolRegistry 或新建 src/tools/webfetch.rs。现有 ToolDefinition 需要对象 schema，ToolResult 用 Success/Error 关联 call_id（src/tools.rs:L502-L535、L648-L663），可把结果编码为 {title,output,metadata,attachments}。当前 builtin_effect 只有 ReadOnly、WorkspaceWrite、CommandExecution（src/tools.rs:L395-L403），网络读取应新增 NetworkRead 或明确 host policy，不能隐式获得任意联网能力。
- 执行并发：src/tools.rs:L1011-L1045 提供 ToolExecutionMode 与 invoke_cancellable；src/tool_runtime.rs:L157-L249 负责批量预算、来源顺序、取消和停止，L252-L346 负责并行只读调用并 join。webfetch 可声明只读并参与并发，但单调用内部必须保持首次请求后才 fallback 的顺序。
- 审批：ApprovalRequest 的工具、side_effect、arguments、preview、origin、policy/lease 字段见 src/approval.rs:L40-L56；只读自动决策见 L495-L528。要映射 ctx.ask 的 webfetch、URL pattern、always，应增加网络权限/host pattern 字段并校验 scheme/host，不能只按工具名记忆。headless ask/always/never 路由在 src/headless.rs:L7417-L7439。
- 网络与输出：实现 5 MiB Content-Length 预检和最多 5 MiB+1 的实际读取；复用 ToolError 的 LimitExceeded/Cancelled 等（src/tools.rs:L671-L717），必要时补 Network/Timeout。ToolContext::read_attachment 的先解析再限长读取模式见 src/tools.rs:L908-L937，但它是本地文件能力，不能代替网络客户端。图片 attachment 应限制 MIME、Base64 大小和 data URL 形状。
- 协议/providers：src/protocol.rs:L222-L227 和 L717-L732 的 attachments 是输入 prompt 附件，不是工具输出附件；应在 ToolResult 到模型消息适配层转发图片。按 src/providers/registry.rs:L175-L183、L396-L404 的 images/files/tools 能力，不支持时返回 Unsupported，不能丢弃图片。
- 取消与运行时：src/runtime.rs:L49-L99 的 CancellationToken 是协作式取消，HTTP handler 要在读取块、超时和 fallback 前检查；L357-L365 提供事件超时接收，L794-L912 的 InputBoundaryGate 保证工具批次未完成时不能消费新输入。
- 会话恢复：src/session.rs:L1422-L1556 将缺终止操作标为 UnknownOutcome，L1558-L1588 要求恢复决定后创建新 operation；网络 timeout 应记录可能已发生的远端副作用，不自动重放。src/core.rs:L6679-L6708 注册 builtin/context/approval，L6726-L6749 裁剪并结构化工具错误，应接入新网络错误码和审计字段。
- headless/providers：headless 通过共享工具、approval、runtime 路由本地调用（src/headless.rs:L7385-L7439）。src/providers/** 负责模型协议和能力，不应在 anthropic/openai/google provider 重复抓取；只在消息适配处消费 webfetch 文本/图片，并依据 registry 能力拒绝不支持附件。
- 可执行差异清单：新增等价 serde schema；增加 URL scheme/host policy；实现双重 5 MiB 限制、30/120 秒总 timeout、一次 Cloudflare UA fallback、HTML text/markdown 转换；接入 CancellationToken；把 timeout/未知网络结果写 operation journal，重试必须新 operation；补 mock HTTP、HTML、图片、超时、大小、审批测试。
未决: 1. ctx.ask 的 always=["*"] 精确匹配和持久化语义不在源文件内。
2. HttpClient.filterStatusOk 对非 2xx、重定向和 body 读取错误的具体错误结构未定义。
3. Effect.timeoutOrElse 是否立即中断 socket，以及 Effect.die 如何序列化，需查看 Effect runtime/调用方。
4. isImageAttachment 的完整 MIME 集合、TextDecoder 编码策略和 Turndown 对畸形 HTML 的细节无法从本源确认。

## OC-060 packages/opencode/src/tool/websearch.ts
- `src/tools.rs` 的 `Tool`、`ToolDefinition`、`ToolContext`、`ToolRegistry`、`ToolResult` 是最直接落点：新增 `WebSearchTool` 实现 `definition()` 与 `invoke_cancellable()`，输入 JSON 对应 `Parameters`，输出对应 `ToolResult::Success { output }`。建议新建 `src/tools/websearch.rs`（或当前扁平工具模块中的同名类型）并在注册表注册 `"websearch"`；使用现有 `ToolError::{InvalidArguments, Io, Json, CommandTimeout, Cancelled, Unsupported}` 映射错误，禁止把网络失败变成 panic/defect。
- `src/tool_runtime.rs:L157-L176,L219-L249` 已提供批量顺序执行、取消轮询和结果按 source order 返回。将 websearch 标为顺序工具；`invoke_cancellable` 在请求前、读取响应期间和解析 SSE 每个帧之间检查 `cancelled`。没有源内重试，应保持一次调用一次请求；若未来增加重试，必须在 operation journal 中显式记录。
- `src/approval.rs:L40-L56,L167-L245` 的 `ApprovalRequest`/`ApprovalCoordinator` 对应 `ctx.ask`。为 websearch 生成稳定 call/request ID，把 query 和 provider 放进 arguments/metadata；在 Rust 的 `SideEffectPolicy` 中决定它属于网络只读还是需要显式批准。若沿用现有 ReadOnly 枚举，仍应保留 `websearch` 的 per-tool 审批，以复刻源行为。
- `src/core.rs` 负责把工具调用绑定到 turn、policy 和 `ToolContext`；应从 Agent 当前 session ID、当前模型 provider/id 构造 Parallel 的 `session_id`/model_name，并把 provider 选择结果写入 `AgentEvent::ToolResult` 或 metadata。不要把 websearch 当作 `src/providers/**` 的 LLM wire route。
- `src/runtime.rs:L49-L99` 的 `CancellationToken` 是外层取消边界；网络客户端应使用可中断/短超时请求，并在 token 取消后返回 `ToolError::Cancelled`。运行时 shutdown 只能停止接收结果，不能假设已发出的远程 POST 被回滚。
- `src/session.rs:L45-L96` 的 `OperationKind::Tool`、`OperationOutcome::{Succeeded,Failed,Cancelled,UnknownOutcome}` 可记录一次搜索。远程请求发出而进程中断时不得自动判定成功或隐式重试；按现有 recovery 规则标为 unknown，显式新 operation 才能重试。源工具本身无持久化，因此这是 Rust 为可恢复性增加的宿主层约束。
- `src/headless.rs` 的 JSONL owner/replay/cancel/approval 通道应只传递工具事件和终态；将 query、provider、错误码放入现有事件 payload，遵守请求/响应关联与重放预算。取消命令应触发 token，不能由 headless 层猜测远程请求是否已完成。
- `src/protocol.rs` 当前有版本化 `StdioRequest` 和工具/审批相关载荷，但没有 websearch 专用 schema；新增字段时保持 deny-unknown-fields、大小上限和 v1/v2 兼容，不把 API key 放进协议。至少为参数对象增加 query 非空、字符串枚举和数字范围校验。
- `src/providers/**` 目前是 OpenAI/Anthropic/Google 等模型路由、认证头和能力目录；它们不应承载 Exa/Parallel MCP。可复用其 ureq/HTTP 错误处理和 provider 配置风格，但建议在独立 `websearch` 模块保存 `EXA_URL`、`PARALLEL_URL`、`OPENCODE_WEBSEARCH_PROVIDER`、`PARALLEL_API_KEY`，并对允许的网络 host 做显式白名单。Cargo 已有 `ureq`，可实现 25 秒总超时和 JSON/SSE 解析。
- 可执行验证顺序：注册工具后运行 registry definition/schema 测试；用本地 HTTP mock 断言 Exa/Parallel 的 URL、JSON-RPC method、参数默认值、Authorization/User-Agent；用 `CancellationToken` 在响应读取中取消；模拟 approval deny、HTTP 500、超时、坏 JSON 和进程中断，分别断言 `ToolResult::Error`、session unknown outcome 和不可自动重试。

主要差异：TypeScript 的 Schema 注解默认值不等同于解析默认值；`||` 会吞掉 0/空字符串；`Effect.orDie` 偏向 defect，而 zenpi 应返回结构化错误；源工具每次显式 ask 且网络只调用一次；zenpi 还要求批量边界、取消 token、journal/replay、字节上限和 redaction；源选择器依赖 `checksum` 的 base36 算法，Rust 必须先
未决: 1. 本文件未给出 `checksum` 的具体算法；无法仅凭本源确认其对任意 session ID 的 base36 输出，Rust 映射必须读取 `@opencode-ai/core/util/encode` 后做跨语言 fixture。
2. `Tool.Context["extra"]` 的完整运行时形状、`RuntimeFlags.enableExa/enableParallel` 的来源及 `ctx.ask` 的拒绝错误类型不在本文件定义。
3. `McpWebSearch.call` 的具体 HTTP/SSE 解析、`EXA_URL` 的 API key 拼接和 Effect 中断细节属于被导入文件，不能由本文件单独证明；已在测试与映射中标明依赖边界。

## OC-061 packages/opencode/src/tool/write.ts
- 主落点是现有 `src/tools.rs::WriteFileTool`：`definition`/`invoke` 对应 `Parameters` 与 `execute`，`ToolPreview::Diff` 对应 `createTwoFilesPatch`/`trimDiff` 的审批预览；`ToolContext::resolve_for_write`、`atomic_write_text` 已提供工作区约束、临时文件写入、`sync_all`、rename 和目录落盘。若要完整复刻 BOM，建议在 `src/tools.rs` 或独立 `src/file_write.rs` 增加 `BomState`、`split_bom`、`join_bom`，并在 `WriteFileTool::invoke` 与 `approval_preview` 共用。
- `src/approval.rs::ApprovalCoordinator` 和 `ToolBatchDecision::ExecuteApproved`（`src/tool_runtime.rs`）承接 `ctx.ask`；Rust 预览带 `source_sha256`，执行前会重新生成预览并拒绝 `StalePreview`，比源实现多出明确的快照竞态保护。写工具应保持 `ToolSideEffect::WorkspaceWrite`，不能因 provider 名称自动放行。
- `src/tool_runtime.rs::execute_tool_batch` 提供取消轮询、顺序执行和 join 语义；`src/runtime.rs::CancellationToken` 是宿主取消入口。建议在 `WriteFileTool::invoke_cancellable` 的文件写入前后检查 token，但要记录“取消不是回滚”，与 Rust 现有语义一致。
- `src/core.rs` 负责工具注册、`AgentEvent::ToolCall/ToolResult`、审批策略及错误归类；若需要 `output` 文本，可由 `ToolResult::Success` 生成“Wrote file successfully.”并附诊断摘要。`src/headless.rs` 将这些结果编码到 JSONL/事件流，适合承接源中的事件通知。
- `src/session.rs` 是追加式 JSONL 持久化，当前写工具本身不写会话；若产品要求恢复审计，应在 core 的操作开始/结束处记录 operation，而不是把文件内容写入 session。`src/protocol.rs` 的 `tool_output`、`approve`、`cancel`、`resume` 命令可暴露审批/结果/取消边界。
- `src/providers/**`（`anthropic`、`openai`、`codex`、`deepseek`、`google` 等）只负责模型协议、路由与认证，不应实现文件写入；provider 返回的 tool call 仍由 `src/core.rs` → `src/tool_runtime.rs` → `src/tools.rs` 本地校验和执行。

关键差异清单：源接受绝对路径并仅由 `assertExternalDirectoryEffect` 判定外部目录，zenpi 默认要求 workspace-relative 且拒绝凭据目录、符号链接和私有输出 inode；源未见 `MAX_WRITE_BYTES`/diff 上限，zenpi 已有 512 KiB 写入与 64 KiB diff 限制；源使用 `fs.writeWithDirs`，zenpi 的 `atomic_write_text` 具有临时文件和目录 fsync 的更强原子/耐久语义；源显式保留/同步 BOM、格式化并触发 LSP/Watcher，zenpi 现有 `WriteFileTool` 主要返回 bounded diff，需要补 BOM、formatter、LSP 诊断和等价事件适配；源的 `Effect.orDie` 把未处理错误变成 defect，zenpi 应映射为 `ToolErrorCode`；源只限制其他文件诊断五个，Rust 侧若无 LSP 服务应明确返回空诊断而非伪造结果。
未决: 1. `assertExternalDirectoryEffect` 对 workspace 外路径的精确允许规则，以及 `ctx.ask` 的 `always: ["*"]` 是否会持久化授权，无法从本文件确认。
2. `FSUtil.existsSafe`、`writeWithDirs`、`Bom.syncFile` 是否保证原子替换、权限继承和目录 fsync 未在本文件定义。
3. `format.file` 的真值含义、格式化失败后的文件状态，以及 `lsp.diagnostics()` 是否等待索引完成无法从本文件确认。
4. 两类事件的 publish 是否可靠投递、是否可重复，以及 `Effect` 取消恰好发生在各副作用中间时的上层恢复协议无法从本文件确认。

# OC-002 — packages/opencode/src/agent/subagent-permissions.ts

- source_id: `OC-002`
- item_id: `OC-002`
- source_path: `packages/opencode/src/agent/subagent-permissions.ts`
- source_hash: `c3bbbd4ea8f076c9da07f5dd242bb07581bd8744b602e3e23deb02ea87601e6c`
- source_bytes: `1225`
- source_lines: `27`
- coverage: 已按源文件顺序读完字节范围 `[0, 1225)`（0–1224，共 1225 字节），行范围 `L1-L27`；包括导入、注释、类型签名、实现和导出符号。

## 完整行为复盘

源文件先从 `@opencode-ai/core/v1/permission` 导入 `PermissionV1`，并以类型导入 `Agent`（`L1-L2`）。文件注释明确函数用途：通过 `task` 工具创建子代理会话时构造 `permission` 规则集；父会话的拒绝规则与 `external_directory` 规则需要继承，而父代理自身限制不应替代子代理自己的能力配置；若子代理没有显式覆盖，则默认拒绝 `todowrite` 和 `task`（`L4-L13`）。

唯一导出符号是 `deriveSubagentSessionPermission`（`L14-L27`）。输入是对象 `{ parentSessionPermission: PermissionV1.Ruleset, subagent: Agent.Info }`，返回新的 `PermissionV1.Ruleset`，没有 `Promise`、异常声明或可选返回值（`L14-L17`）。`Agent.Info.permission` 是规则数组；同一函数只读取它，不修改 `input` 或数组内容。

1. `canTask` 在 `L18` 通过 `.some()` 判断子代理规则中是否存在 `rule.permission === "task"`。只检查权限名称，不检查 `action`、`pattern` 或规则顺序。因此只要出现一条 `task` 规则（即使该规则的 `action` 是 `deny`，或只匹配有限 pattern），就视为“已许可/已覆盖”，后续不会追加默认全局拒绝。
2. `canTodo` 在 `L19` 以同样方式判断 `todowrite`。同样存在“名称出现即算覆盖”的边界；不会解析 allow/deny 的优先级，也不会验证 pattern 是否为 `*`。
3. 返回数组的第一段是对 `parentSessionPermission` 的 `filter` 结果（`L20-L23`）。保留条件为 `rule.permission === "external_directory" || rule.action === "deny"`：所有 `external_directory` 规则都保留（无论 action），所有 `deny` 规则都保留（无论 permission）；allow 规则如果不是 `external_directory` 则被丢弃。`filter` 保持父数组原有顺序和对象引用/字段值，空输入得到空段。
4. `L24` 根据 `canTodo` 条件追加默认项。若为 `false`，追加 `{ permission: "todowrite", pattern: "*", action: "deny" }`；若为 `true`，追加空数组。`as const` 只是在 TypeScript 中把三个字面量收窄到规则所需类型，不改变运行时值。
5. `L25` 对 `canTask` 做同样处理，追加 `{ permission: "task", pattern: "*", action: "deny" }` 或空数组。顺序固定为“父规则段 → todowrite 默认拒绝（若有）→ task 默认拒绝（若有）”，并且不会去重；父数组中已有相同拒绝项时仍可能重复出现。
6. `L26-L27` 结束数组和函数。函数没有显式错误路径：若输入满足静态类型，即使规则数组为空、含未知权限、含重复项或包含冲突 allow/deny，也会同步返回结果；运行时若传入 `null`/缺失字段等违反类型的值，访问 `.filter` 或 `.some` 才可能抛出普通 JavaScript `TypeError`，源码没有防御式校验。

该实现的核心判据是“继承父的安全边界 + 对子代理未声明的两个递归/待办能力设置全局 deny”。它不把子代理完整规则直接拼接进结果，所以子代理的 allow 规则（除 `external_directory` 外）不会由本函数传递；调用方若需要保留它们，必须在函数外另行合并。调用是纯同步计算：没有共享可变状态、锁、线程、异步等待或 I/O；并发调用在调用者不同时修改输入数组的前提下相互独立，输出顺序确定。

## 状态、取消、恢复与副作用

函数本身没有内部状态，也没有取消令牌、超时、重试、恢复或持久化逻辑（`L14-L27`）。它不启动任务、不访问文件/网络、不发送审批、不执行工具；直接副作用为零。唯一外部影响是返回的规则集会被 `task` 调用方用于新会话权限：父会话的所有 deny 与 `external_directory` 约束继续存在，缺少子代理 `todowrite`/`task` 名称时增加 `pattern: "*"` 的 deny。父规则对象不被改写，故取消或重试调用只会重新计算一个数组，不会回滚或重复外部动作。

注意该函数没有“deny 优先级”算法：如果子代理规则含 `task: deny`，`canTask` 仍为真，函数不会再添加默认 deny；最终是否真的拒绝取决于下游 PermissionV1 评估器和规则顺序。父规则的 deny 则无条件保留，但函数也不保证去重或规范化。源码没有超时/取消错误路径；输入类型错误属于调用方契约破坏，不是领域错误返回。

## 源内测试与行为判据

源文件及其 `src/agent` 同目录未包含测试，也未发现针对 `deriveSubagentSessionPermission` 的测试引用；因此源内未包含测试。可独立验证的判据如下：

- 父规则 `[{permission: "read", pattern: "*", action: "allow"}, {permission: "x", pattern: "a", action: "deny"}, {permission: "external_directory", pattern: "/tmp", action: "allow"}]`、子代理无 `task`/`todowrite` 时，结果应依次保留后两条，再追加 `todowrite/* deny`、`task/* deny`。
- 子代理只含 `{permission: "task", pattern: "narrow", action: "deny"}` 时，不应追加默认 `task/* deny`，因为 `L18` 只按名称判断；这项边界应作为回归测试。
- 子代理含 `todowrite` 但不含 `task` 时，只追加 `task/* deny`；两者都含时不追加默认项。
- 父规则 allow（且 permission 不是 `external_directory`）必须被丢弃；父规则 deny 和任意 action 的 `external_directory` 必须保留，顺序不得改变；重复规则不得被隐式去重。

## zenpi Rust 映射

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

## 未决问题

- `PermissionV1.Ruleset` 的下游匹配优先级、冲突规则解析及 `pattern: "*"` 的具体 glob 语义不在本文件中，无法仅凭该源确认。
- 调用方在 `src/tool/task.ts` 还会额外追加 `task` 对应的 deny、实验性 primary tool deny 和去重逻辑；这些调用方规则与本函数结果的最终合并顺序需结合会话创建代码确认。
- 当子代理存在同名但相互冲突的 allow/deny 规则时，本函数只看名称，最终能力取决于外部 PermissionV1 评估器，源文件没有说明。

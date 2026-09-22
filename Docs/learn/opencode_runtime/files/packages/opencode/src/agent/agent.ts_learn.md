# OC-001 — packages/opencode/src/agent/agent.ts

- source_id/item_id：OC-001
- source_path：packages/opencode/src/agent/agent.ts
- source_hash：e781c571d584fa996200e320f14c2735e32968256b8db194391e7c06aed7433e
- source_bytes：16746
- source_lines：453
- coverage：已从头到尾读取字节范围 [0, 16746)、行范围 L1-L453（含导入、注释、类型、内部实现和导出符号）。

## 完整行为复盘

1. 模块依赖与数据模型（L1-L34）  
   模块把 Config、Auth、Plugin、Skill、Provider、LocationServiceMap 组合成 Effect 服务；用 Permission 计算工具权限，用 InstanceState 按实例/工作目录缓存状态，用 generateObject/streamObject 生成结构化 agent 配置。PROMPT_GENERATE、PROMPT_COMPACTION、PROMPT_EXPLORE、PROMPT_SUMMARY、PROMPT_TITLE 是外部文本提示；ProviderTransform 负责 provider-specific 参数。Global.Path.tmp/data、Truncate.GLOB 和 reference/skill 路径参与权限边界。

2. Info 与 Info 类型（L35-L56）  
   Info 是 Schema.Struct，字段包括必填 name、mode（仅 "subagent"|"primary"|"all"）、permission、options，以及可选 description、native、hidden、topP、temperature、color、model（providerID/modelID）、variant、prompt、steps。数值使用 Schema.Finite，options 接受任意 JSON 值。导出的 Info 类型是 DeepMutable<Schema.Schema.Type<typeof Info>>；运行时对象仍可能由 get 返回 undefined（L312-L314），这是类型签名与实际查找结果的边界。

3. 内部 GeneratedAgent（L58-L62）  
   生成器的结构化输出必须含字符串 identifier、whenToUse、systemPrompt 三个字段。没有额外字段限制逻辑写在这里；实际标准 schema 由 L412-L415 组合。

4. Interface、State、Service、use（L64-L86）  
   Interface.get(agent) 返回 Effect<Info>，list() 返回 Effect<Info[]>，defaultInfo() 返回 Effect<Info>，defaultAgent() 返回名称字符串；generate 输入为 description 和可选 {providerID, modelID}，输出为 GeneratedAgent，错误类型声明为 Provider.DefaultModelError。State 去掉 generate，表示可缓存的目录状态。Service 的上下文键是 "@opencode/Agent"；use = serviceUse(Service) 提供调用入口。

5. 服务层依赖与状态构造（L88-L117）  
   layer 通过 Layer.effect 注入六个服务（L90-L96），随后以 InstanceState.make<State> 创建状态（L98-L352）。状态初始化读取 config.get()（L100）、skill.dirs()（L101）；当 cfg.references 或兼容字段 cfg.reference 非空时，先等待 PluginV2.ID("core/config-reference")，再通过按当前 ctx.directory 提供的 LocationServiceMap 查询 Reference.Service.list() 并收集路径（L102-L107）。whitelistedDirs 包含截断 glob、临时目录、skill 目录和 reference 目录（L108-L113）。readonlyExternalDirectory 将其他外部路径设为 "ask"，白名单设为 "allow"（L114-L117）。

6. 默认权限与内置 agent（L119-L265）  
   defaults 默认允许所有工具，但 doom_loop 询问，外部目录默认询问，question/plan_enter/plan_exit 默认拒绝；读取规则允许普通文件、询问 *.env/*.env.*、允许 *.env.example（L119-L136）。全局用户权限来自 cfg.permission（L138）。
   - build：primary/native，默认描述；在 defaults 上允许 question、plan_enter，再合并用户规则（L140-L155）。
   - plan：primary/native；允许 question、plan_exit，拒绝 task.general，只放行计划目录及 .opencode/plans/*.md 等编辑路径（L156-L181）。
   - general：subagent/native；在默认权限上拒绝 todowrite（L182-L195）。
   - explore：subagent/native；先全拒绝，再只放行 grep/glob/list/bash/webfetch/websearch/read 和只读外部目录白名单，并使用 PROMPT_EXPLORE（L196-L218）。
   - compaction、title、summary：primary、native、hidden，均在 defaults 上覆盖为全拒绝；分别使用对应提示。title 还固定 temperature: 0.5（L219-L264）。

7. 配置覆盖、禁用与权限补丁（L267-L310）  
   遍历 cfg.agent ?? {}。value.disable 会删除同名 agent 并继续（L267-L271）。未知名称创建 mode:"all"、native:false、空 options，权限为 defaults+user（L272-L280）。随后按非空值覆盖模型（Provider.parseModel）、variant、prompt、description、temperature、top_p -> topP、mode、color、hidden、展示名、steps；options 用 mergeDeep 合并，agent 级权限再次 Permission.merge（L281-L294）。最后每个 agent 若没有显式拒绝 external_directory 的 Truncate.GLOB，就追加 allow 规则；显式 deny 保持优先（L296-L310）。

8. 状态查询函数  
   - get（L312-L314）：按键直接取 agents[agent]，不存在时返回 undefined，无额外错误转换。  
   - list（L316-L326）：重新读取当前 config，仅用于决定排序；先把配置的 default_agent（若有）或 build 排在前面，再按 name 升序。返回同一状态中的 agent 对象数组。  
   - defaultInfo（L328-L340）：若配置了 default_agent，不存在、mode==="subagent"、hidden===true 分别抛出明确 Error；否则返回该对象。未配置时返回 Object.values(agents) 中第一个非 subagent 且非 hidden 的对象；没有则抛 "no primary visible agent found"。  
   - defaultAgent（L342-L344）：委托 defaultInfo()，只取其 name。

9. 服务包装与 generate（L355-L437）  
   get/list/defaultInfo/defaultAgent 包装为带名字的 Effect.fn，通过 InstanceState.useEffect 访问状态（L355-L367），因此状态初始化/实例隔离由 Effect 层处理。
   generate（L368-L436）先读配置，使用输入模型或 provider.defaultModel()（L372-L374），再解析模型、取得 language model（L374-L375）。开启 OpenTelemetry 时尝试取得 OtelTracer，否则 tracer 为 undefined（L376-L378）。系统提示数组初始为 PROMPT_GENERATE，先允许插件事件 experimental.chat.system.transform 原地改写（L380-L381）；从状态取已有 agent 名称，放入用户提示以避免重名，并要求只返回 JSON（L382、L397-L415）。通过 auth.get 检测 openai + oauth，失败 Effect.orDie 会转为 defect（L384-L386）。
   参数固定 temperature:0.3，附带 telemetry/userId（用户名缺省 "unknown"）、结构化 GeneratedAgent schema。OAuth 分支不发送普通 system message，而把合并后的 instructions 与 store:false 放进 ProviderTransform.providerOptions，使用 streamObject 消费完整流；任何 part.type==="error" 都抛出，onError 回调本身忽略错误（L418-L432）。普通分支调用 generateObject(params).then(r => r.object)（L433-L435）。两条路径都返回三字段对象，不在本文件持久化结果或自动重试。

10. locationServiceMapNode、node、命名空间导出（L441-L453）  
    locationServiceMapNode 把 LocationServiceMap.Service 和 locationServiceMapLayer 组成无依赖节点（L441-L445）。导出的 node 声明 Agent layer 依赖 Config.node/Auth.node/Plugin.node/Skill.node/Provider.node/locationServiceMapNode（L447-L451）。最后 export * as Agent from "./agent" 让调用方通过 Agent.Service、Agent.node 等命名空间访问（L453）。

## 状态、取消、恢复与副作用

- 状态：agents 是状态初始化期间创建的普通可变 Record；InstanceState 负责按实例上下文保存它。config.get() 在初始化、list、defaultInfo 时分别读取，因此 agent 字典本身不会因后续配置自动重建，但默认排序/选择会看最新 config。
- 并发语义：本文件没有 mutex、队列或显式锁；同一状态内查询共享 agent 对象，配置覆盖发生在初始化阶段。Effect 的调度/隔离决定调用交错，源内没有“更新时复制”或快照保证。
- 取消/超时：没有 timeout、取消 token、deadline、重试计数或 abort controller。generate 的 provider 调用包在 Effect.promise 中；可传播的 Effect 中断取决于外层运行时和 AI SDK，源内未提供 promise 清理。OAuth 流只在完整流中遇到 error part 时失败。
- 恢复/持久化：agent 定义、配置覆盖和生成结果都不写 SessionStore 或文件；进程/实例重建会重新计算。恢复仅能由上层重新加载 config。
- 外部副作用：初始化可能等待插件并读取 reference；查询 skill/reference 目录；plugin.trigger 可改写 system prompt；auth.get 读取认证状态；generateObject/streamObject 发起模型请求并产生 telemetry。没有工具执行、审批或 session journal 写入。

## 源内测试与行为判据

源内未包含测试（源文件本身没有 inline test）。仓库 test/agent/agent.test.ts 覆盖了：无配置时七个 native agent（L45-L57）、build 默认权限（L60-L71）、plan 的编辑例外及 task.general 拒绝（L74-L103）、explore 外部目录白名单（L112-L135）、general/compaction 权限（L157-L182）、自定义 agent 字段和权限合并（约 L184-L220）、禁用 agent（约 L230-L260）、mode/name 覆盖（约 L320-L350）、defaultAgent/defaultInfo 的默认 build、显式 plan/custom、subagent/hidden/不存在错误和 build 禁用后的回退（L649-L740）。test/agent/plan-mode-subagent-bypass.test.ts 还验证 explore 在 plan 场景的权限合并。

可独立验证的判据：在 opencode 仓库运行 bun test test/agent/agent.test.ts；断言 list() 名称与排序、get("missing")===undefined、权限匹配结果、defaultAgent() 的四类错误文本；为 generate 注入 fake provider，核对非 OAuth 发送 system prompt、OAuth 使用 providerOptions.instructions/store:false，以及重复名称出现在 user message 中。

## zenpi Rust 映射

- agent 目录/类型落点：建议新增 src/agent.rs，定义可序列化的 AgentInfo（name/description/mode/native/hidden/top_p/temperature/color/permission/model/variant/prompt/options/steps）、AgentMode、AgentCatalog 和 AgentService。AgentCatalog::get/list/default_info/default_name 对应 L312-L344；AgentService 负责从配置构造内置 agent 与合并用户覆盖。
- src/core.rs 对照：现有 Agent（状态机、AgentPhase、AgentEvent、turn/tool 执行）应继续作为运行时执行器；新增 AgentInfo 不要与现有执行器同名，建议字段为 profile/catalog。调用 ToolRuntime 前，将 AgentInfo.permission 编译成 SideEffectPolicy/ApprovalPolicy。
- src/providers/registry.rs 与 src/providers/**：ModelDescriptor、ModelRegistry::resolve 可承载 providerID/modelID 校验和能力交集；src/providers/connection.rs 的 ValidatedRoute 负责端点/auth，不应吸收 agent 权限。新增 AgentGenerator 时通过既有 backend/provider route 发结构化 JSON 请求，明确区分普通请求与 OpenAI OAuth/Codex dialect。
- src/approval.rs：把 TS 的 Permission.merge 结果翻译为已有 ApprovalPolicy；plan 的 deny/allow 路径、explore 的只读工具集合应在进入 execute_tool_batch 前成为不可变策略快照。ApprovalCoordinator 的 pending/accepted/cancel 语义可替代 TS 中未实现的审批，但不能把 remembered approval 当作模型输入权限。
- src/tool_runtime.rs：ToolBatchOptions、ToolBatchDecision 和 execute_tool_batch 已提供批量、顺序/并发、取消协作；按 agent mode 选择 sequential/max_concurrency，并在 prepare 中应用 profile 权限。TS 的 Truncate.GLOB 外部目录 allow 需要落到 ToolContext 的路径规则。
- src/runtime.rs：BackgroundRunner、CancellationToken、SubmitError 可承载 AgentGenerator 的异步任务；为模型请求增加 deadline/abort 绑定，补齐 TS 源没有实现的超时和取消。不要把不可取消的 future 伪装成已取消。
- src/session.rs：TS agent registry 不持久化；zenpi 若要恢复 profile，应在 SessionStore 记录 profile 名称、模型和策略 digest，而不是把完整权限对象隐式塞入 turn。使用现有 operation marker/recovery 机制记录生成请求的 started/completed/failed。
- src/protocol.rs 与 src/headless.rs：当前 Command 有 Prompt/Steer/Cancel/Resume/Approve 等，但没有 agent catalog/generate 命令；建议增加严格大小限制的 AgentList/AgentGet/AgentGenerate 变体及版本化响应。headless.rs 负责 JSONL correlation、replay、错误映射；生成事件应走 AgentEvent::Provider/终端响应，避免直接写 stdout。
- 差异清单与验证：① Effect Layer/Context/InstanceState → Rust 明确所有权与 Arc<RwLock<AgentCatalog>> 或启动期不可变快照；② Schema.Finite/optional 字段 → serde + 自定义校验；③ TS Permission pattern merge → Rust 中确定 deny 优先级和路径规范化；④ TS get 的 undefined 与声明不一致 → Rust 选择 Option<AgentInfo> 并在 protocol 层转错误；⑤ TS plugin/OpenTelemetry hook → Rust trait hook，禁止生成器静默改写策略；⑥ TS 无 timeout/retry → Rust 以 CancellationToken/deadline 明确实现；⑦ TS OAuth streaming 特例 → provider dialect capability 测试。可验证命令：cargo fmt --check、cargo check、cargo test，并为新增 protocol 做重复 request-id、取消、模型解析和权限边界测试。

## 未决问题

1. InstanceState.make 的具体缓存失效/并发保证不在本文件中，无法确认同一 ctx 是否可并行初始化。
2. Effect.promise 在当前 Effect 版本中的中断行为、AI SDK 是否支持底层 abort，源文件未给出。
3. ProviderTransform.providerOptions 对每个 provider 的最终 JSON 形状、Plugin 触发器是否会异步修改 system，需查各自实现。
4. Config 对 maxSteps 等别名的预处理不在本文件；本文件只读取已经规范化的 value.steps。


# OC-017 — packages/opencode/src/session/llm/request.ts

## 元信息

- source_id/item_id：OC-017
- source_path：packages/opencode/src/session/llm/request.ts
- source_hash：a92010ff1981f9bdf62c7d2f6dcbe28baea041e2cd54bf0b54fd2ea667b92cf2
- source_bytes：7591
- source_lines：226
- coverage：已按源文件顺序读取完整内容，字节范围 0–7590（7591 字节），行范围 L1-L226；提供的 SHA-256 与实际读取文件一致。

## 完整行为复盘

### 常量、输入输出类型与合并辅助

- USER_AGENT（L18-L18）由安装版本插入为 opencode/<InstallationVersion>，最终会进入请求 headers。
- PrepareInput（L20-L36）是 prepare 的完整输入契约：用户 SessionV1.User、当前/父会话 ID、Provider.Model、Agent.Info、可选 PermissionV1.Ruleset、系统提示片段、ModelMessage[]、small 标志、工具表、provider/auth/plugin/runtime flags，以及 workflow 标志。small、parentSessionID、permission、auth 都可缺省；其余字段必需。
- 导出的 Prepared（L38-L51）包含最终 system、messages、按名称排序的 tools、模型参数 params（temperature/topP/topK/maxOutputTokens/options）、消息变换选项和 headers。
- mergeOptions（L53-L54）对 target 与 source 做 mergeDeep；undefined source 被当作空对象。后续层覆盖前层，返回值强制为 Record<string, any>。它是深合并，不是浅层替换。

### prepare（L56-L206，导出）

这是一个 Effect.fn 生成器；插件 effect 或 InstanceState.context 失败时整个 effect 失败，成功时返回 Prepared。

1. 系统提示构造与插件变换（L57-L78）：当 provider ID 为 openai 且 auth 类型为 oauth 时设置 isOpenaiOauth。系统数组初始总是一个元素：优先使用非空 agent.prompt，否则展开 SystemPrompt.provider(input.model)，然后追加 input.system 和非空 input.user.system，过滤 falsy 项并以换行拼接（L58-L66）；因此即使所有来源为空，仍有一个可能为空字符串的元素。保存首元素 header，触发 experimental.chat.system.transform，携带 session/model 上下文和可变的 { system }（L68-L73）。若插件使数组长度超过 2 且首元素仍等于原 header，保留首项，把其余项再次换行合并为第二项（L74-L78）；若首项已被插件改写，则不做该压缩。
2. 模型变体、基础选项与 provider 特例（L80-L100）：非 small 且存在 model.variants 与 user.model.variant 时索引选择变体，否则使用空对象（未知 key 等价于 undefined，随后按空层合并）。small 请求取 ProviderTransform.smallOptions(model)；普通请求取 ProviderTransform.options({model, sessionID, providerOptions})（L84-L90）。最终 options 按 base → model.options → agent.options → variant 深合并（L91）。若 npm API 是 @ai-sdk/azure 且任一 provider/model/合并后的 options 请求 useCompletionUrls，删除 reasoningSummary 与 include（L92-L98）。OpenAI OAuth 把完整 system 以换行放入 options.instructions（L99），这会与后面的 system message 分支形成不同协议形态。
3. 消息形态（L101-L112）：OpenAI OAuth 或 isWorkflow 为真时直接复用 input.messages；否则先把每个 system 字符串映射成 {role:"system", content:x}，再追加原消息。该分支不复制消息对象，调用方/插件可观察到引用语义。
4. 参数插件钩子（L114-L132）：触发 chat.params，上下文含 session、agent 名称、model、provider、当前 user。默认参数为：只有模型 capability temperature 为真时才设置 temperature，值为 agent.temperature ?? ProviderTransform.temperature(model)；topP 为 agent.topP ?? ProviderTransform.topP(model)；topK 为 ProviderTransform.topK(model)；maxOutputTokens 为 ProviderTransform.maxOutputTokens(model, flags.outputTokenMax)；options 为上一步合并对象。插件返回值成为最终 params，所以可改写或引入 effect 错误。
5. headers 插件钩子（L134-L146）：同样以 session/agent/model/provider/user 作为上下文，默认 {headers:{}}；解构出的 headers 作为最高优先级的附加层。
6. 工具解析、严格模式与 Copilot 兼容（L148-L175）：先由 resolveTools(input) 按权限及用户开关过滤。对 @ai-sdk/openai、@ai-sdk/azure、@ai-sdk/amazon-bedrock/mantle，遍历所有工具并以对象展开强制 strict:false（L149-L158），保证动态/MCP schema 可注册。若 providerID 含 github-copilot、过滤后工具为空、且历史消息含 tool-call/tool-result，则加入 _noop；其 aiTool 的 schema 只有可选 string reason，execute 为异步空输出，意图是满足 API 需要而不应被调用（L159-L175）。
7. 项目上下文与结果 headers（L177-L205）：providerID 以 opencode 开头时读取 InstanceState.context 的 project.id；否则不读取。工具通过 Object.entries(...).toSorted(([a],[b]) => a.localeCompare(b)) 稳定按名称排序后重建。opencode provider 的默认 headers 是可选 x-opencode-project、x-opencode-session、x-opencode-request（user.id）、x-opencode-client 和 User-Agent；其他 provider 使用 x-session-affinity、X-Session-Id、User-Agent（L187-L200）。有 parentSessionID 时追加 x-parent-session-id；随后 model.headers 覆盖默认值，插件 headers 再覆盖 model.headers（L201-L204）。返回的 messageTransformOptions 与 params.options 指向同一 options 结果，工具排序只影响返回副本，不改变此前过滤表的逻辑。

### resolveTools（L208-L214，私有）

取工具名集合，与 Permission.merge(input.agent.permission, input.permission ?? []) 合并规则交给 Permission.disabled 计算禁用集合；仅保留用户没有显式设为 false（input.user.tools?.[k] !== false）且不在 disabled 集合的工具。未提供用户工具覆盖时默认不因该项禁用；权限结果优先于工具可用性。函数无网络和持久化副作用，但返回表随后可能被 prepare 的 provider 特例修改。

### hasToolCalls（L216-L223，导出）

顺序遍历 ModelMessage[]；忽略 content 非数组的消息，数组内任一 part 的 type 为 tool-call 或 tool-result 即返回 true，否则遍历结束返回 false。空消息、文本 content、未知 part 类型都返回 false；短路发生在第一个匹配处。

### 命名空间导出（L226-L226）

export * as LLMRequestPrep from "./request" 把本模块（包括导出的 prepare、Prepared、hasToolCalls）以 LLMRequestPrep 命名空间重新导出，形成调用方的稳定入口。

## 状态、取消、恢复与副作用

- 本文件没有显式超时、重试、持久化或恢复逻辑；prepare 只是请求准备 effect，不发送 HTTP，也不写 session journal。Effect 运行器可以中断生成器，插件 effect/InstanceState.context 的错误直接向上传播。
- 副作用边界是三个 plugin trigger（system、params、headers）以及插件传入对象可能发生的原地修改；options 的字段删除、工具 strict:false 写入、Copilot _noop 插入都发生在本次准备的内存对象上。结果 headers 只是值构造，不代表已发送。
- 取消/超时只能由外部 Effect 运行器或上层请求循环实现；源内没有 cancellation token、deadline 或 catch/finally。因而准备被取消时没有本文件负责的补偿动作，也没有“已发送但未知”的恢复标记。
- 重试不会在这里发生。若上层重新调用 prepare，插件会再次触发，插件的非幂等副作用可能重复；调用方应自行保证幂等。
- _noop.execute 虽为 async，但正常契约明确禁止调用；若 provider 仍调用，它只返回 {output:"", title:"", metadata:{}}，不会在本文件中持久化。
- input.messages 在 OAuth/workflow 分支直接复用，在普通分支只创建新的 system 前缀数组；调用方必须把 provider 发送、assistant/tool 结果落盘和崩溃恢复交给上层。

## 源内测试与行为判据

源文件中未包含测试（只有实现与注释，L1-L226）。可独立验证的判据如下：

1. 构造普通 provider：system 返回一个合并字符串，并在 messages 首部为每个 system 字符串生成 system role；将 small、variant、agent/model options 组合后断言深合并优先级为 variant > agent > model > base。
2. 设置 @ai-sdk/azure 与任一 useCompletionUrls=true，断言 reasoningSummary、include 不存在；设置 temperature capability=false，断言 params.temperature 为 undefined。
3. 使用 OpenAI OAuth 或 isWorkflow=true，断言 messages 与输入数组语义一致且 OAuth options 含 instructions；普通 provider 断言 headers 含 session affinity，opencode provider 断言 x-opencode headers 和 User-Agent。
4. 权限合并后禁用某工具、用户工具开关为 false、未设置开关三种情况分别断言过滤结果；OpenAI family 工具均有 strict:false；GitHub Copilot 历史含 tool-call 且无工具时存在 _noop。
5. 对 hasToolCalls 覆盖空数组、非数组 content、tool-call、tool-result 和其他 type，并检查短路返回。
6. 令 plugin trigger 返回错误或修改 system/params/headers，确认 effect 失败传播和覆盖顺序；opencode provider 缺少 project ID 时不得生成 x-opencode-project。

## zenpi Rust 映射

建议新增 src/llm_request.rs（或 src/providers/request_prep.rs）作为纯准备层，定义 PrepareInput、Prepared、RequestOptions、PreparedHeaders，由 Agent::complete_with_tools 在当前准备边界调用。现有对应关系与差异如下：

- src/core.rs 的 complete_with_tools 在 L4100-L4161 构造 instructions、工具 definitions、CompletionRequest，并执行 context budget、资源/skill 合并；L3659-L3835 负责可取消 provider turn、操作标记和结果持久化。可把 request.ts 的 system/messages/options/headers 逻辑落到 prepare_request(...) -> Result<Prepared, AgentError>，保留 core 的 admission、begin_operation_with_key 和 CompletionRequest 创建。当前 Rust 没有 SystemPrompt.provider 同名调用，应将 persona/resource/skill instructions 明确定义成 Vec<String> 后再拼接。
- src/providers/registry.rs 的 ModelDescriptor、effective_capabilities、context_budget（L75-L130）承载模型能力和输出上限；src/providers/connection.rs 的 ValidatedRoute、resolve_connection、revalidate_route_auth（L39-L119、L160-L365）负责 provider/model、URL、auth/header policy 和 route digest。它们可提供 temperature/top_p/top_k/max_output_tokens 的 Rust 输入，但目前没有 TS 的任意 model.options/agent.options 深合并；需引入受限 serde_json::Map，禁止未经 route policy 的任意 header/option 注入。
- src/providers/mod.rs 的 OptionPolicy、ProviderDefinition、Protocol/Dialect（文件开头及 L130-L190）是 ProviderTransform.options 的落点。建议为每个 Protocol 实现 base_options(model, session_id, provider_options) 与 small_options，并在同一处实现 Azure completion URL 与 Responses/Anthropic/Google 的字段兼容删减。Rust 现有 provider 子模块（openai.rs、codex.rs、anthropic.rs、google.rs、deepseek.rs）提供静态路由定义，没有 TS 的运行时 plugin 变体表。
- src/tool_runtime.rs 的 ToolBatchOptions、ToolBatchDecision、execute_tool_batch（L113-L176、L345-L440）执行已获准工具批次，并明确由 caller 负责 policy/approval/journal。resolveTools 应映射到 core 在 L4031-L4048 的 definitions 过滤，再叠加 ApprovalPolicy/skill allow-list；不要把 strict:false 当作 approval。若要兼容 Copilot，可在 provider encoder 生成 _noop definition，但执行器仍应拒绝未知/禁用调用。
- src/approval.rs 的 ApprovalPolicy、ApprovalCoordinator（L15-L70、L125-L190、L295-L370）是 Permission 规则的安全落点。TS 的 Permission.merge/disabled 是静态工具名过滤，Rust 还要求 side-effect、worker binding、持久化 ack；映射时应先生成可见的 disabled set，再经过 approval/immutable gate，不能由 provider 返回的工具名授予权限。
- src/session.rs 提供 Turn、OperationKind::Provider、OperationOutcome 和 append-only append_turn/append_event（L40-L90、L863-L885、L1200-L1225）。request.ts 不写 journal；因此只在 core 的 provider dispatch 前后写 operation marker、assistant/tool turn 和 event，不能把 Prepared 本身当作已完成请求。Rust 的恢复区分 Cancelled、Interrupted、UnknownOutcome，比本文件无状态恢复更严格。
- src/runtime.rs 的 CancellationToken（L45-L103）及 BackgroundRunner/job outcome（L150-L190、L300-L375）负责取消与队列；应把 token/deadline 传入上层 provider loop，而不是塞进纯 prepare_request。prepare 被取消时只返回错误，core 决定是否 finish operation。
- src/protocol.rs 的 TurnMode、StdioRequest（L43-L75、L150-L209）负责 Start/Steer admission，StdioEvent/StdioResponse（L854-L980）负责相关事件和错误码。它们对应 sessionID/parentSessionID/request ID 的外层关联；headers 中的 x-opencode-* 不应直接暴露为客户端可任意设置的协议字段，应由 core/project context 生成。
- src/headless.rs 只拥有 JSONL I/O（L1-L31），并将 provider/agent/runtime lifecycle 投影为有 sequence 的 StdioEvent（约 L4367-L4395）。映射应在 headless 层发送 provider progress，在 core 层产生 Prepared/operation 结果；不要让 request-prep 模块直接写 stdout。
- 可执行验证差异清单：①实现 prepare_request 单元测试覆盖上述五类判据；②用 ModelRegistry/ValidatedRoute capability 计算输出 token，拒绝未知 header；③把 plugin trigger 映射为受类型约束的 extension hooks，并测试 hook 错误不产生 assistant journal；④在 run_active_turn_cancelable 中验证 cancel、transport unknown outcome 与 retry confirmation；⑤验证工具排序使用 BTreeMap/显式排序、权限过滤先于 provider encoder；⑥为 opencode-like gateway 定义 project/session/request/client headers 的固定生成器，插件只能追加经 allow-list 的 header。

## 未决问题

- Plugin.Interface.trigger 是否允许原地修改传入数组、其返回值是否总是完整替换 payload，源文件本身无法确认；需查看 plugin runtime 的契约与错误类型。
- ProviderTransform.*、SystemPrompt.provider、Permission.disabled/merge 的具体默认值和 capability 判定不在本文件内，不能仅凭本文件确定各 provider 的数值。
- InstanceState.context.project.id 缺失或 context effect 失败时的具体错误类型由外部 effect 定义；本文件只表明该失败会阻止 prepare 返回。
- _noop 是否会被下游 provider 实际调用、以及 aiTool 的 schema 序列化细节取决于 ai SDK 与 GitHub Copilot adapter，源文件没有进一步保证。


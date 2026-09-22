# OC-006 — packages/opencode/src/provider/auth.ts

- source_id/item_id：packages/opencode/src/provider/auth.ts / OC-006
- source_path：packages/opencode/src/provider/auth.ts
- source_hash：7331fda8558fe517aa5a69a8aa78dcbb80af719f43caf16c8960f9c5ad7a0d2e
- source_bytes：7877
- source_lines：229
- coverage：已按文件顺序读取全部字节，字节范围 [0,7877)，行范围 L1-L229（含注释、类型、导出符号）。

## 完整行为复盘

- Schema 与输入形状（L1-L39）：When 是条件对象，字段为 key: string、op: "eq"|"neq"、value: string。TextPrompt 固定 type: "text"，含 key、message，可选 placeholder 与 when；SelectOption 含 label、value、可选 hint；SelectPrompt 固定 type: "select"，含 key、message、options: SelectOption[]、可选 when；Prompt 是两者联合。Schema 只描述/解码形状，源码没有执行 when 条件。
- 导出模型（L41-L66）：Method（类名 ProviderAuthMethod）的 type 为 "oauth"|"api"，有 label，可选 prompts: Prompt[]；Methods 是 Record<string, Method[]> 及同名 TypeScript 类型。Authorization（类名 ProviderAuthAuthorization）返回 url、method: "auto"|"code"、instructions。外部输入 AuthorizeInput 为有限数值 method（注释称认证方法索引）及可选字符串记录 inputs；CallbackInput 为有限数值 method 及可选 code。
- 错误、接口与状态（L68-L107）：OauthMissing(providerID) 表示没有待处理 OAuth；OauthCodeMissing(providerID) 表示 code 流缺 code；OauthCallbackFailed 表示回调结果不是成功；ValidationFailed(field,message) 表示 prompt 校验失败。Error 联合了 Auth.AuthError 与这四类错误。Interface.methods 返回 Effect<Methods>；authorize 接收 providerID + AuthorizeInput，返回 Effect<Authorization | undefined, Error>；callback 接收 providerID + CallbackInput，返回 Effect<void, Error>。私有 State 保存 hooks: Record<ProviderV2.ID, Hook> 和 pending: Map<ProviderV2.ID, AuthOAuthResult>。Service 是 Context service，use 是其 serviceUse 包装。
- 状态初始化与服务层（L109-L128）：layer 依赖 Auth.Service、Plugin.Service。首次创建 InstanceState<State> 时调用 plugin.list()，仅收集存在 x.auth?.provider 的插件，转换为 ProviderV2.ID 后建立 hooks；pending 为空的 Map。插件清单因此在实例状态初始化时快照，后续插件变更不会自动刷新。
- methods（L130-L161）：从 InstanceState.get(state) 取 hooks，对每个 provider 的 item.methods 按原顺序映射，只暴露 type、label 和存在时的 prompts。select prompt 保留 options/when；text prompt 保留 placeholder/when。可选字段通过条件展开省略，不把 undefined 写入结果；最后用 Schema.decodeUnknownSync(Methods) 校验/解码。插件方法形状不合 schema 时会在同步解码处抛出异常（不在显式 Error 联合内）。
- authorize（L163-L186）：读取 hooks/pending，并用 hooks[input.providerID].methods[input.method] 取方法；非 oauth 直接返回 undefined，不会调用 API 方法。若同时有 method.prompts 和 input.inputs，逐个检查 text prompt：只有存在 prompt.validate 且对应输入值非 undefined 时调用校验；返回错误字符串则以 ValidationFailed(field=prompt.key,message=error) 失败。它不要求所有输入齐全，也不根据 when 过滤。随后以 Effect.promise(() => method.authorize(input.inputs)) 等待插件 Promise；Promise 失败沿 Effect 失败/缺陷路径传播。完成后把完整 AuthOAuthResult 写入 pending.set(providerID,result)，并返回只含 url/method/instructions 的 Authorization。同一 provider 的并发 authorize 会按 Promise 完成顺序覆盖 pending，未建立 flow ID。
- callback（L188-L221）：取 provider 的 pending；不存在即 OauthMissing。若保存结果的 method === "code" 且 input.code 为假值（空字符串也算缺失）则 OauthCodeMissing。调用保存的 callback：code 模式传 input.code!，auto 模式无参数。结果为空或 result.type !== "success" 即 OauthCallbackFailed。成功结果含 key 时调用 auth.set(providerID,{type:"api",key,...metadata?})；含 refresh 时去掉 type/provider/refresh/access/expires 后，把 access/refresh/expires 与其余字段写入 auth.set 的 OAuth 记录。两个 in 分支可在异常联合值同时执行。成功返回 void；pending 不删除，所以可重复 callback，且保存的 callback 闭包会一直留在实例 Map 中。
- 装配与导出（L223-L229）：返回 Service.of({ methods, authorize, callback })；node 用 LayerNode.make 声明依赖 Auth.node、Plugin.node；末行以 export * as ProviderAuth from "./auth" 导出命名空间。

## 状态、取消、恢复与副作用

该文件没有显式取消 token、超时、重试、恢复或锁。Effect 取消可能使调用方停止等待，但 Effect.promise 包装的原生 Promise 本身没有在此处传入 abort controller；插件网络请求/浏览器等待是否停止由插件实现决定。没有 retry loop，也没有把 pending 写入磁盘；进程/实例重建后 pending 丢失。Map 以 provider 为粒度，未提供一次性消费、TTL、并发互斥或回调状态机。

外部副作用有三类：初始化调用 plugin.list()；authorize/保存的 callback 可执行插件网络、设备码轮询或浏览器流程；成功 callback 调用 Auth.Service.set 持久化 API key/OAuth token 与 metadata。源码不记录审计事件，也不清理失败/成功的 pending。provider/method 越界、缺失 hook 等访问会在 hooks[input.providerID] 或 method.type 处产生未归类的运行时异常，不能保证落入声明的 Error。

## 源内测试与行为判据

源文件及其同目录没有测试（源内未包含测试）。可独立验证的判据如下：

1. 注册一个含 API 与 OAuth 方法的 hook，methods() 的 provider key、方法顺序、label、prompt 的可选字段应与输入一致；select 的 options 应完整保留，when 仅被透传。
2. 选择 API 方法调用 authorize 应得到 undefined 且不调用 hook；选择 OAuth 方法应先执行已提供 text 值的 validate，错误返回 ValidationFailed，成功返回 Authorization 并登记 pending。
3. code 模式无 code（含空字符串）应为 OauthCodeMissing；无 pending 应为 OauthMissing；callback 返回非 success 应为 OauthCallbackFailed；success 的 key/OAuth 形状应分别触发一次对应的 Auth.set。
4. 仓库外部的补充测试 packages/opencode/test/plugin/auth-override.test.ts:L40-L82 验证插件对 github-copilot 的 auth methods 可覆盖内置方法、并保持方法列表可见；它不覆盖本文件的 authorize/callback 错误路径。

## zenpi Rust 映射

- 现有认证底座：src/auth/mod.rs:L1-L170 已有 AuthBinding、AuthError、LoginFlowId、LoginState 与 LoginControl；src/auth/codex.rs 已实现固定 openai-codex 的 browser/device OAuth、active login 互斥、取消和提交；src/auth/callback.rs 已实现 loopback callback 的 state/URI 校验；src/auth/store.rs 负责 API/OAuth 凭据持久化、版本/修订与冲突；src/auth/resolve.rs 负责刷新、锁与 scoped secret。它们可承接 callback 的安全与持久化，不应把 token 放进 session.rs 普通记录。
- 建议新增通用模块：新增 src/auth/provider_methods.rs（或 src/provider_auth.rs）定义 ProviderAuthMethod { Api, Oauth { prompts, authorize, callback } }、TextPrompt、SelectPrompt、When、Authorization、AuthorizeInput、CallbackInput、ProviderAuthError；以 BTreeMap<String, Vec<ProviderAuthMethod>> 替代 TS 的 Record。hook 采用 Rust trait（如 ProviderAuthHook）并由显式 registry 注册，不能假设现有 src/providers/registry.rs 的模型 registry 能承载闭包。
- 对照 src/providers/**：src/providers/mod.rs 的 Protocol/AuthHeaderPolicy 和 src/providers/connection.rs 的 ValidatedRoute/revalidate_route_auth 只处理模型路由与凭据授权；src/providers/codex.rs 提供固定 endpoint 常量（OAuth 交换仍在 src/auth/codex.rs）；src/providers/registry.rs 是静态模型目录。通用 plugin auth 应放在 auth 层，注册时再关联 provider ID，不能绕过 ValidatedRoute 写入任意目的地。
- 协议与宿主落点：在 src/protocol.rs 为 methods/authorize/callback 增加带 provider_id、方法索引、输入/代码的显式 request/response 类型和有限长度校验；在 src/headless.rs 增加 dispatch、结构化错误与事件输出。当前 StdioRequest/Command 没有 ProviderAuth 命令，不能只复用 Prompt 字段。交互事件可参照 src/auth::AuthInteraction，而不是把 OAuth code 当作 approval.rs 的工具批准。
- 取消、恢复、并发差异：用 src/runtime.rs::CancellationToken 与 auth::LoginControl 的 deadline/send budget 包住异步授权；每个授权产生不可猜的 LoginFlowId，状态放在 Arc<Mutex<HashMap<LoginFlowId, PendingAuth>>>，callback 原子地 take/remove，限制同 provider 并发或明确 flow 绑定。TS 当前按 provider 覆盖 pending、允许重复 callback；Rust 实现应把这个差异作为安全修正并写测试。
- 持久化与核心连接：成功 OAuth/API 结果调用现有 credential store 的受控 mutation，之后由 src/core.rs/backend 的 AuthBinding 选择凭据并由 src/providers/connection.rs 重新校验 destination/header scope。不要把 raw key/access token 放进 src/session.rs JSONL；session 只可记录成功/失败/取消/不确定的操作摘要。未知网络结果沿 session.rs 的 operation recovery 规则要求显式新 operation retry。
- 工具与审批边界：src/tool_runtime.rs 只处理 provider tool batch 与取消；OAuth 授权不是 tool approval。若宿主要显示 prompt，可复用 src/approval.rs 的可取消等待模式，但必须保留独立的 auth flow、provider identity 和 credential commit 审计。
- 可执行验证差异清单：实现后应测试（a）未知 provider/越界 method 返回稳定 ProviderAuthError；（b）API 方法不产生 OAuth flow；（c）text validate、code 缺失、失败 callback、key/OAuth 两种 commit；（d）同 provider 两个 flow 互不覆盖，重复 callback 被拒绝；（e）取消/超时停止轮询且不会写入 credential；（f）store 冲突/commit uncertain 不触发隐式重试；（g）Codex 固定 endpoint 与 AuthHeaderPolicy::Codex 仍由 providers/codex.rs/connection.rs 约束。

## 未决问题

- AuthOAuthResult、Hooks["auth"] 的完整联合类型不在本文件内，无法仅凭本文件确认 success 结果是否保证 key/refresh 二选一、metadata 的精确 schema，以及 method.authorize/callback 的 Promise 是否可取消。
- Auth.Service.set 的实际存储位置、加密/冲突策略由 src/auth 之外的实现决定；本文件只确认调用形状。
- 对缺失 provider、越界 method、Schema 解码失败和 Promise rejection，Effect 运行时究竟以 typed failure 还是 defect 暴露，需结合 Effect 版本实现或运行时测试确认。


# AC-049 — opencode/packages/opencode/src/acp/service.ts

| 字段 | 值 |
|---|---|
| source_id/item_id | `opencode/packages/opencode/src/acp/service.ts` / `AC-049` |
| source_path | `opencode/packages/opencode/src/acp/service.ts` |
| source_hash | `238750d52e220080db7b7f4992432fbfc7fb7ec5a15a6175460f8f0832a2c8de` |
| source_bytes | `42641` |
| source_lines | `1226` |
| coverage | 已按顺序读完整文件：字节 `1-42641`、行 `L1-L1226`；含注释、类型、导入与导出符号 |

## 说明与源码骨架

本文件把 ACP（Agent Client Protocol）请求适配成 OpenCode SDK 的 session 操作。外层使用 `Effect.Effect<结果, ACPError.Error>`，内层维护进程内 `ACPSession`、目录快照、MCP 去重表，并由 `ACPEvent.start` 把 SDK 事件转成 `sessionUpdate`。源文件没有直接实现 DAG；它提供的是“有状态会话 + 父子 fork + 取消 + 事件邮箱/回放 + best-effort 外部调用”的通信边界，后文将把这些原语映射到 zenpi 的 DAG worker。

## 完整行为复盘

### 导入、公开类型与服务接口

- `L1-L47` 导入 ACP SDK 的连接、认证、session 生命周期、prompt、配置与取消类型；导入 OpenCode `OpencodeClient`、`Session`、`Message`、`SessionMessageResponse`，Effect 的 `Context/Effect/Layer/ManagedRuntime`，以及本目录的 `ACPError/Directory/ACPEvent/ACPSession/UsageService/ACPProfile`。`ProviderV2/ModelV2/Provider` 负责模型身份和值域校验，`Command` 用于合并命令与 skill。
- `AuthMethodID`（`L49`）固定为 `opencode-login`。`Error`（`L51`）是 `ACPError.Error` 别名。`ServiceConnection`（`L52-L53`）只要求 `sessionUpdate`，`requestPermission` 与 `writeTextFile` 可选，因此没有强制审批/文件写通道。
- `Interface`（`L55-L71`）完整暴露 `initialize`、`authenticate`、`newSession`、`loadSession`、`listSessions`、`resumeSession`、`closeSession`、`forkSession`、三个配置修改操作、`prompt` 与 `cancel`；每个调用都返回可失败的 Effect。`Service`（`L73`）把该接口注册为 Effect Context service，标签为 `@opencode/ACP/Service`。

### `make` 与生命周期操作

- `make`（`L75-L93`）接收必选 `sdk`，其余 `connection/directory/session/usage/eventSubscription` 可注入。缺省时分别用 `makeSessionService`、`makeDirectoryService`；`registeredMcp` 按 `sessionId` 保存已注册 MCP key，`sessionSnapshots` 按 session 缓存目录快照。只有有 `connection` 才启动 `ACPEvent.start`；事件订阅回调若存在立即收到 subscription。`runUntilIdle` 在有事件订阅时等待指定 session 进入 idle，否则直接执行函数；它是并发收敛点，不是新线程调度器。
- `initialize`（`L94-L139`）创建认证方法；当 `clientCapabilities._meta["terminal-auth"] === true` 时补充 `opencode auth login` 元数据。固定返回协议版本 `1`、支持 load/MCP HTTP+SSE/embeddedContext/image、close/fork/list/resume，以及 `InstallationVersion`。无外部请求；通过 `ACPProfile.duration` 记录耗时。
- `authenticate`（`L141-L146`）只接受 `AuthMethodID`，其它 method 产生 `UnknownAuthMethodError`；接受时返回空对象，不验证凭据，实际登录由终端命令负责。
- `directorySnapshot`（`L148-L153`）从 `directoryService.get(cwd)` 取得目录配置并统计耗时。`configSnapshot`（`L155-L161`）优先使用 session 级缓存，否则按 `state.cwd` 加载并写入 `sessionSnapshots`；缓存粒度是 session 而非全局目录。
- `newSession`（`L163-L209`）先取目录快照，选默认 model/variant，并在存在 mode 时使用默认 mode；通过 profile 包装的 `sdk.session.create` 创建 backing session，传入目录、可选 agent、provider/model/variant。随后在本地 `ACPSession` 创建状态、保存快照、注册 MCP、异步发送 available commands，返回 `sessionId` 与完整 `configOptions`。SDK create、session create、MCP 注册或命令发送的 Effect 错误会中断；MCP 单项错误在 `registerMcpServers` 内被忽略。默认 model 可能是未知占位值，见 `selectDefaultModel`。
- `loadSession`（`L211-L247`）加载目录快照，再并行语义上顺序请求 backing session 与完整 messages；`restoreSession` 按持久 model、历史消息、当前可用目录恢复 model/variant/mode，调用 `session.load`，缓存快照，注册 MCP、发送 commands，并通过 `replayMessages` 回放完整消息 parts。返回配置但不返回新 sessionId。任何 backing/messages 读取失败会转 ACP service error。
- `listSessions`（`L249-L293`）把 `cursor` 用 `Number` 解析，固定 `limit=100`；SDK 列 `roots:true`，映射为 `SessionInfo`，再拼入本地 live sessions（按 id 去重），按 `updatedAt` 降序排序。无效/缺失/非有限 cursor 视为无 cursor；有效 cursor 只保留时间戳小于 cursor 的项。返回首 100 条，只有 `filtered.length > 100` 且 page 有尾项才给 `nextCursor`，因此时间戳相同会有分页边界风险但没有额外 tie-breaker。
- `resumeSession`（`L295-L334`）与 load 类似，但 messages 只取 `limit:20`，MCP 缺省 `[]`，不回放 messages；恢复后可继续 prompt。`abortBackingSession`（`L336-L345`）调用 backing `session.abort`，失败只 `logError` 后转成功，明确是 best-effort。
- `closeSession`（`L347-L355`）先从本地 session 删除，再删除 MCP 与快照；找不到时幂等返回 `{}`。找到时才 abort backing，abort 失败仍成功返回。`cancel`（`L357-L360`）只取得当前 session 并 abort backing，不删除本地 session，故取消后可再次 prompt。
- `forkSession`（`L362-L407`）取 cwd 快照，调用 `sdk.session.fork` 生成新 backing id，再读取 fork 后最多 20 条消息，以 fork backing 与历史恢复配置，`session.load` 新本地状态，注册 MCP、发送 commands、回放 messages，返回新 `sessionId` 与配置。父子关系由 SDK fork 隐含，不在此文件显式保存 `parentSessionId`。

### 配置、prompt 与返回值

- `setSessionConfigOption`（`L409-L466`）先取 session 与快照，`value` 非字符串立即 `InvalidConfigOptionError`。`configId=model` 用 `parseSelectedModel` 校验 provider/model/variant，保留同 model 的当前 variant 或按默认选择，先设 variant 后设 model，发送 `config_option_update` 并返回选项；`effort` 要求当前 model 有该 variant，非法为 `InvalidEffortError`；`mode` 要求存在于 `availableModes`，非法为 `InvalidModeError`；其它 id 为 invalid config。每个分支返回当前模型、variant、mode 的配置视图。
- `setSessionMode`（`L468-L476`）在快照中验证 mode，再持久化/更新本地 session，成功返回空对象，不主动发送 config update。
- `setSessionModel`（`L478-L495`）解析合法 model，依照当前 session 选择 variant，先写 variant 再写 model，向连接发送 `config_option_update`，成功返回空对象。
- 返回对象（`L497-L507`）只导出上面的生命周期与配置函数；`prompt` 与 `cancel` 作为 inline Effect 函数加入。
- `prompt`（`L509-L590`）先取当前 session 与目录快照；缺 model 时写入默认 model，缺 variant/mode 时临时采用默认值。`promptContentToParts` 转换 ACP 文本、图片、资源，`detectSlashCommand` 判断是否是 slash command。
  - 普通 prompt（`L521-L543`）用 `runUntilIdle` 包裹 `sdk.session.prompt`，发送 session/model/variant/parts/agent/directory；完成后发 usage update，调用 `promptResponse(response.info, messageId)`。因此返回 ACP response 前会等待事件流 idle。
  - 已知命令（`L546-L568`）改调 `sdk.session.command`，model 编成 `provider/model` 字符串，参数为 command args；同样等待 idle、发 usage、映射响应，且不调用 provider prompt。
  - `/compact`（`L570-L586`）调用 `sdk.session.summarize`，不返回 assistant 结果；未知 slash 也不会报错，只发 usage 并以空 info 得到 `end_turn`（`L588-L589`）。
- `makeSessionService`（`L595-L599`）构建 `ACPSession.node` 的 ManagedRuntime 并同步取得 service；`makeDirectoryService`（`L601-L615`）构建 `Directory.node`，注入 loader，loader 通过 `request(loadDirectorySnapshot)`，均为进程内依赖组装。
- `makeUsageService`（`L617-L690`）以 `${directory}\0${providerID}\0${modelID}` 为 key 缓存 context-limit Promise；并发请求复用同一 Promise，失败返回 `undefined` 且不会抛出。`sendUpdate` 读全量 messages，取最新 assistant；无 assistant/provider/model/limit 时静默返回，否则通过 connection 发 `usage_update`，used 为 context tokens，size 为 limit，cost 为 USD 总成本；读 messages 失败只记录日志并返回。
- `replayMessages`（`L692-L699`）无 subscription 时为空 Effect；有时按输入顺序逐条 `replayMessage`，单条失败被吞掉，不阻断后续回放。

### 类型、请求包装、目录与模型恢复

- `ConfigState`（`L701-L705`）是 model 加可选 variant/mode；`SdkResponse<T>`（`L707-L710`）是可选 data/error 包装；`MessageInfo`（`L712-L720`）只抽取恢复配置所需的 user/assistant 字段；`AssistantError`/`AssistantInfo`（`L722-L723`）抽象 assistant 错误与 token cost。
- `request`（`L725-L737`）执行 Promise，若结果形如 SDK `{data,error}`，优先把 error 抛出、data 非 undefined 时取 data，否则把原值当 T；所有异常经 `fromUnknownError` 映射。它没有 retry、timeout 或取消信号注入。
- `profiledRequest`（`L739-L741`）只是在 `request` 外包 `ACPProfile.measure`。
- `loadDirectorySnapshot`（`L743-L798`）用 `Promise.all` 并发读取 providers、agents、commands、skills、config；config.get 失败被当成 undefined。过滤 `subagent` 和 hidden agent 形成 modes，命令与同名 skill 去重后合并并排序，默认 mode 是第一个可见 primary，否则字符串 `build`；最终 `Directory.build` 生成快照。除 config 外任一请求 reject 会让整体失败。
- `defaultModelFromConfig`（`L800-L817`）优先接受配置中且存在的 `provider/model`；否则优先 `opencode` provider 的 `Provider.sort` 首项，再从全部 provider model 的排序首项取值，最后返回配置值（即使其不在当前 provider 表）；注释明确新 session 不扫描历史。
- `selectDefaultModel`（`L819-L824`）依次用 snapshot default、第一 model option，最终用类型伪装的 `unknown/unknown` 占位。
- `detectSlashCommand`（`L826-L837`）只拼接 text parts、trim、要求以 `/` 开始；首个空白分段是 name，余下 trim 为 args；`/` 本身返回 undefined。
- `promptResponse`（`L839-L888`）无 error 返回 `end_turn`、可选 usage/userMessageId 与 `_meta:{}`；`MessageAbortedError`→`cancelled`，`MessageOutputLengthError`→`max_tokens`，`ContentFilterError`→`refusal`；`ProviderAuthError` 抛 `AuthRequiredError(providerID)`；其它错误抛 `ServiceFailureError(service=session,safeMessage,errorName)`。有 error 时 usage 仍会被带回。
- `promptErrorMessage`（`L890-L893`）优先取 `error.data.message` 字符串，否则固定 `OpenCode prompt failed`。
- `sendUsageUpdate`（`L895-L908`）无 connection 时为空 Effect，否则使用注入的 usage 或临时 `makeUsageService` 发更新。
- `selectVariant`（`L910-L915`）无 variants 返回 undefined，有 `default` 固定选 `default`，否则取第一个 key；`selectModelVariant`（`L917-L928`）优先显式 selected variant，其次同 model 且当前 variant 仍合法，最后默认 variant。
- `hasVariant`（`L930-L933`）把 `DEFAULT_VARIANT_VALUE`（注释所说持久化 no-override sentinel）视为合法，或检查 variants 自身；`configOptions`（`L935-L943`）把 providers、当前 model/variant、modes/current mode 交给 `buildConfigOptions`。
- `sendConfigOptionUpdate`（`L945-L962`）无 connection 为空；有则异步调用 `sessionUpdate(config_option_update)`，任何发送失败被转 undefined 并 ignore。
- `parseSelectedModel`（`L964-L986`）解析 provider/model 选择，检查 provider/model 存在，失败为 `InvalidModelError`；显式 variant 不存在为 `InvalidEffortError`；成功返回规范化 ID 与 variant。

### 命令、MCP、持久状态与错误识别

- `sendAvailableCommands`（`L988-L1008`）无 connection 为空；否则用 `setTimeout(...,0)` 异步发送 `available_commands_update`，只暴露每个 command 的 name/description，发送 Promise 不被等待也不捕获。
- `registerMcpServers`（`L1010-L1059`）按 session 取/建 `Set`，用 `mcpConfig` 后以 `mcpRegistrationKey` 去重；current 与本轮 pending 均跳过重复项；通过 `Effect.all(...,{concurrency:"unbounded"})` 并发 `sdk.mcp.add`，成功才加入 current，单项失败 `ignore`，最后记录数量与耗时。这是本文件最明确的无界并发区，输入 servers 过大时没有本地数量限制。
- `mcpRegistrationKey`（`L1061-L1063`）是 name 加 stable JSON；`mcpConfig`（`L1065-L1078`）将有 `type` 的 server 转 remote/url/headers，否则转 local/command/environment；重复 header/env key 时 `Object.fromEntries` 的后者覆盖前者。
- `stableStringify`（`L1080-L1087`）数组保持顺序，primitive 用 JSON，object 按 key localeCompare 排序递归序列化；未显式处理 `undefined` 造成的 JSON 语义由 `JSON.stringify` 决定。
- `restoreSession`（`L1089-L1102`）先 `restoreFromMessages`，再 `restoreDurableModel(backing.model)`，model 由 `restoreModel` 选，variant/mode 分别恢复。`restoreDurableModel`（`L1104-L1113`）把 backing 的 providerID/id/variant 转成强类型；无 durable model 返回空。
- `restoreModel`（`L1115-L1123`）优先 durable 且仍存在于 snapshot，其次 history 且存在，最后 `selectDefaultModel`；`restoreVariant`（`L1125-L1138`）只在所选 model 有 variants 时工作，优先同 model 的 durable variant，再同 model 的 history variant，最后默认；`restoreMode`（`L1140-L1144`）优先合法 durable，再合法 history，再 snapshot default mode，无 modes 返回 undefined。
- `hasModel`（`L1146-L1148`）检查 provider/model 索引；`hasMode`（`L1150-L1152`）检查 mode id；`sameModel`（`L1154-L1155`）比较 providerID 与 modelID。
- `restoreFromMessages`（`L1158-L1180`）先找最后一条带模型的 user message，恢复 user model/variant/agent；若无，再找最后一条带 provider/model 的 assistant，恢复 assistant variant 与 `mode ?? agent`；两者都无返回空。它只看传入 messages，不做分页或时间排序。
- `isSdkResponse`（`L1182-L1184`）把非 null object 且拥有 `data` 或 `error` 字段者判为 SDK 包装；`fromUnknownError`（`L1186-L1192`）保留已有 ACP error，把递归识别到的认证错误映射为 `AuthRequiredError`，其余统一为安全消息 `OpenCode service failure`。
- `isACPError`（`L1194-L1202`）要求 object、`_tag` 为 string 且以 `ACP` 开头；`isAuthRequired`（`L1204-L1217`）递归检查 Error name/message、普通对象 `name/_tag`，以及 `error/data` 子对象中的 `ProviderAuthError/LoadAPIKeyError`；`findProviderID`（`L1220-L1226`）从 `providerID` 或 `providerId` 取字符串，否则递归 `data/error`。文件在 `L1226` 结束，无隐藏导出或后续逻辑。

## 状态、取消、恢复与副作用

- 进程状态：`make` 级 `registeredMcp`、`sessionSnapshots`、事件 subscription；session 状态在 `ACPSession`；真正 transcript/model/session 由 OpenCode SDK backing session 持久化。目录快照命中后同一 ACP session 不重新读 provider/agent/command/skill，失败的首次加载不会写入缓存（`L155-L161`、`L743-L798`）。
- 取消：ACP `cancel` 与 `closeSession` 都调用 `sdk.session.abort`（`L336-L360`），但前者保留本地 session，后者先移除本地 state；abort 失败只记日志，不回滚删除或向调用者报错。`promptResponse` 将 backing 返回的 `MessageAbortedError` 转成 `stopReason=cancelled`（`L858-L863`）。源文件没有 `AbortSignal`、显式 timeout、指数退避或自动 retry；SDK Promise 的取消依赖 backing API。
- 并发：目录加载 `Promise.all` 并发；MCP 注册 unbounded 并发；同一 session 的 prompt/command 通过 `ACPEvent.runUntilIdle` 串到事件 idle，但此文件没有显式 per-session mutex。`makeUsageService` 复用同 key 的 in-flight Promise；connection update/replay/available-command update 多为异步副作用，部分故意不等待或吞错。
- 恢复：`loadSession` 完整读 messages 并回放，`resumeSession/forkSession` 只读最多 20 条；恢复优先 backing durable model/agent，其次历史消息，再回退当前目录默认值（`L1089-L1180`）。fork 会创建新 backing session 并复制 SDK 语义历史；本文件不记录显式 parent/grandparent/sibling/child 图，也不判断后代完成度。
- 外部副作用：`sdk.session.create/get/messages/list/prompt/command/summarize/abort/fork`、`sdk.mcp.add`、`connection.sessionUpdate`、usage 读取 provider 配置。MCP 注册成功会修改 backing 配置；session close 会清理内存映射并 best-effort abort；profile 只产生观测数据，不改变协议结果。
- 错误与安全边界：SDK error 经 `request` 统一映射；认证错误保留 provider id，普通错误隐藏原始内容。发送 update、usage 读取、消息 replay 失败大多被吞掉；核心 session 创建、恢复、配置验证失败会传播。没有源内持久化的“已派生 worker”意图，也没有 close 的全子孙绿检查，因此 zenpi 必须补充 durable DAG gate。

## 源内测试与行为判据

源文件本身没有测试；同目录的 `packages/opencode/test/acp/service-session.test.ts` 提供可独立核对的行为判据：

- `L350-L372` 验证 `newSession` 返回 session/config、重复 MCP server 只 add 一次并异步发送 `available_commands_update`；`L374-L434`、`L436-L486`、`L488-L536` 验证 durable 状态优先、历史恢复、默认 variant sentinel 与 stale 状态回退。
- `L538-L617` 验证 `loadSession` 回放 user/agent chunks 与 reasoning thought chunks；`L619-L650` 验证按 updated 时间排序、cursor 与 100 条分页；`L652-L691` 验证 `resumeSession` 恢复配置但不回放 transcript。
- `L693-L731` 是取消/关闭判据：close 删除 ACP state 并 abort backing，cancel abort 但 session 仍可用，abort 失败不使 cancel/close 失败；`L733-L793` 验证 fork 取新 id、恢复 fork durable model/variant/mode。
- `L822-L887` 验证 provider auth error 映射与失败快照不缓存；`L889-L936` 验证不同 session/config 下同名 MCP 不误去重；`L938-L1005` 验证配置 model 优先且新 session 不扫描历史；`L1007-L1171` 验证 model/effort/mode 更新、非法配置为 invalid params、session snapshot 不重复取远端。
- `L1301-L1351` 验证普通 prompt 携带 model/variant/mode/转换后的 parts、usage 与 message id；`L1353-L1420` 明确事件 update 必须完成后才返回 `end_turn`；`L1422-L1466` 验证普通 provider error→request error、abort→cancelled；`L1468-L1530` 验证 audience/image/resource 转换；`L1532-L1573` 验证已知 slash command 与 `/compact` 分流；`L1575-L1616` 验证认证错误→auth-required。

可独立复验的最小判据是：对 mock `OpencodeClient` 记录每个 SDK 方法及参数，依次调用 `newSession → setSessionConfigOption → prompt → cancel/close`，断言调用顺序、配置值、`sessionUpdate` 类型、`stopReason` 与 session 是否仍可取；再用 102 个 session 复验分页。对 DAG 映射则应额外断言：任意节点存在未绿的 child/grandchild 时 close 被拒或转保活，全部后代绿且本节点绿时才持久化 close；派生 worker 的 parent/grandparent/sibling/child mailbox 消息必须带 sender、recipient、request_id、TTL、claim/complete 结果且可重放。

## zenpi Rust 映射

### 可直接复用的通信原语

1. **消息/邮箱**：直接复用 `src/protocol.rs:L81-L113` 的 `MailboxOutcome` 与 `MailboxRequest::{Send,Receive,Acknowledge,Claim,Complete}`，以及 `src/protocol.rs:L479-L484` 的解析入口；不要另造一套 worker RPC。现有 `src/session.rs:L3171-L3272` 的 `MailboxStatus`、`MailboxMessage`、`MailboxAction` 已有 queued→acknowledged→claimed→succeeded/failed 的状态机、TTL 与 result。
2. **持久邮箱**：`src/session.rs:L3570-L3697` 的 `SessionMailbox::open/enqueue` 已做同 workspace、session identity、request id 幂等、digest、TTL 与 archived 检查；`L3700-L3795` 的 `list/update` 提供有界分页与不可隐式重试的状态转移。worker 之间的“请求新工作/询问父状态/回报绿或失败”应均落成 mailbox message，不用共享可变内存直接互调。
3. **活性/保活**：`src/session.rs:L3274-L3359` 的 `LiveSessionRegistry::{register,heartbeat,unregister,active}` 是最小 liveness lease。它目前只描述 session owner，不描述 DAG 节点；建议新增 `LiveDagWorker`（`node_id,parent_node_id,session_id,owner_epoch,last_seen_ms,expires_at_ms,state`）或将 node identity 作为 session owner 的受约束扩展。`heartbeat` 必须是幂等的；过期只阻止 claim/派生，不自动把工作标成成功。
4. **事件**：复用 `src/core.rs:L281-L325` 的 `AgentEvent` 与 `src/protocol.rs:L882-L917` 的异步事件 envelope，把 `DagWorkerStarted/Heartbeat/MessageReceived/Green/Failed/KeepAlive/Derived/CloseDeferred` 作为事件 payload。事件是可观察流，结论仍要先写 `SessionStore`；不能把“收到 Green 事件”当成 durable commit。
5. **协议**：复用 `src/protocol.rs:L152-L209` 的 `StdioRequest` 公共 envelope、`L211-L263` 的 `Command`、`L265-L517` 的版本/字段校验。建议在现有 `Command::Mailbox` 旁新增 `Command::Dag(DagRequest)`，或者先把 DAG 控制面编码为受限 mailbox payload；必须用 `MAX_ID_BYTES`、`MAX_MAILBOX_TEXT_BYTES`、TTL 上限（`L32-L37`）并拒绝未知字段。`headless.rs:L5323-L5503` 是异步请求解析/dispatch，`L6392-L6558` 已有 mailbox/cancel/resume 路由，适合挂 DAG 查询、keepalive 与 derive。

### 会话父子关系与 DAG 邻接

源 `forkSession`（`L362-L407`）虽然创建了 fork，却没有在 `service.ts` 保存 `parentSessionId`；zenpi 不能照搬这个隐式关系。建议在 `src/session_tree.rs` 或新 `src/dag.rs` 建立持久的 `DagNode`：`node_id`、`session_id`、`parent_id: Option<String>`、`created_by`、`state`（`Running|Green|Failed|KeptAlive|Closed`）、`generation`、`lease_epoch`、`close_reason`。每次派生前先在父 `SessionStore` 追加 `dag_derive_intent`，再创建 child `SessionStore`，最后追加 `dag_child_attached`；恢复时按 intent/attached 对账，未完成 attach 的 intent 只能是 `UnknownOutcome`，不能假定 child 已启动。

现有 `session_tree.rs:L48-L84` 的 `TreeEntry/TreeNode/TreePage`、`L156-L217` 的 recover、`L245-L301` 的 ancestry/page、`L316-L372` 的 active-leaf parent/children 计数可复用 ID、深度、预算、恢复校验，但它是“选中 transcript leaf”的树，不是 worker DAG：`children` 只有计数，不能回答 sibling/全后代状态。因此建议新增索引 `BTreeMap<NodeId, DagNode>`、`BTreeMap<NodeId, BTreeSet<NodeId>>`，并在 `SessionTree` 旁独立维护 DAG；不要把多个 worker journal 强行写进一个 active-leaf 链。

通信授权的最小邻接函数应为 `DagGraph::related(node, recipient) -> bool`：允许自身、直接 parent、grandparent（`parent.parent`）、直接 child，以及直接 sibling（`children[parent]` 中除自身者）；其它 node id 一律 `InvalidField/Unauthorized`。消息 envelope 至少含 `message_id, sender_node_id, recipient_node_id, relation, parent_session_id, generation, payload, ttl_ms`，接收端再用图当前快照核对 relation，防止 client 自报 sibling。祖先关系是图数据，不应从 filename 或 session title 推断。

### 保活与派生的最小机制（必须实现）

1. `DagCoordinator::request_close(node_id)` 先在同一 journal/锁内读取 node 自身和 descendants；只有 `node.state == Green` 且每个 child/grandchild 的最新 terminal state 都是 `Green` 才追加 `dag_closed`。这对应用户要求的“节点自身及其全部 child/grandchild 全绿才允许 close”。空 child 集合不阻塞，但 node 自身必须先绿。
2. 若存在任一 `Running|Failed|KeptAlive|UnknownOutcome` 后代，必须追加一次幂等 `dag_keepalive`（key=`node_id:current_generation:close_attempt`）、刷新 `LiveDagWorker` heartbeat/lease，保持 session/runner 不关闭，并返回 `CloseDeferred { pending_nodes }`；绝不能把关闭失败当作成功 close。
3. 若待处理工作不属于当前 worker，追加 `dag_derive_intent` 后通过 `SessionMailbox::enqueue` 向 child 发 `derive`；请求带 `source_request_id`/fingerprint，重复请求返回既有 intent。child owner `claim` 后才调用 `BackgroundRunner::spawn`，启动成功追加 `dag_child_started`，结束时先持久化所有 tool/provider 结果，再追加 `dag_green` 或 `dag_failed`，最后回复 parent mailbox。
4. 派生至少需要 `derive_intent → child_attached → heartbeat/claim → terminal(green|failed) → close_or_keepalive` 六种可恢复记录/事件；重启扫描未闭合 intent，结合 `SessionMailbox` 的 claim/result 与 `LiveSessionRegistry::active` 决定恢复、保活或显式 `UnknownOutcome`。任何 side effect 未有 terminal journal 不能自动重试。
5. 父 worker 保持 `Running` 直到所有直接 child 的 terminal 结果被消费；grandparent 不需要同步等待每条消息，但可通过自身 mailbox 收到聚合 `subtree_green`。sibling 只允许旁路咨询/结果消息，不能替另一个 sibling close 或 claim；child 不能修改 parent 状态，只能发结果/请求。

### 既有模块落点

- `src/headless.rs`：沿用 `L23-L31` 的 `Command/StdioRequest/SessionStore` 组装，扩展 `AsyncRequestParts` 与 `L5323` 的异步入口；在 `L5503` 的 Tree dispatch、`L6292` 的 Cancel、`L6392` 的 Mailbox、`L6531-L6558` 的 Resume 分支加入 `Dag` 查询、heartbeat、derive、close 请求。输出使用 `StdioEvent`，每个 request id 只产生一个 terminal response，进度/keepalive 作为 event；不要在 stdin 线程直接 spawn 无法回收的 worker。
- `src/core.rs`：复用 `WorkerExecutionBinding`（`L37-L82`）绑定 blueprint/goal/item/lease/policy/expiry，把 `DagWorkerContext` 放在 `Agent` 的 session 与 `live_owners`（`L402-L442`）旁。`AgentEvent`（`L283-L325`）增加 DAG 生命周期事件；在 `process_with_cancel_and_events`（`L5385-L5409`）完成一个 node turn 的 admission/run，在 `process_steer_reissue_with_events`（`L5411-L5430`）处理保活后的新工作。新增 `close_if_subtree_green` 应只调用 SessionStore 的原子 gate，不让 UI/headless 自己计算后关闭。
- `src/session.rs`：`SessionStore` 已是 append-only journal（`L144-L160`），并且把 tree、handoff、runtime intent、opaque event 分层；复用 `append_runtime_intent` 的校验/幂等形状（`L1171-L1202`）记录 `DagDeriveIntent`，复用 `append_event` 的 opaque event 记录 keepalive/terminal，但 tree 专用事件仍应走 typed owner（`L1205-L1214`）。在 `SessionMailbox`/`LiveSessionRegistry` 之上加 `DagStore` 或 `SessionStore` 的 typed methods，确保“写 intent、写结果、判定 close”在同一 append lock 与恢复投影中可验证。
- `src/runtime.rs`：直接复用 `CancellationToken`（`L49-L99`）的 cooperative cancel/completion race 语义，`BackgroundRunner` 的 bounded command/event channel（`L101-L193`、`L236-L320`）、`RuntimeEvent` terminal ordering、`try_cancel` 与 bounded shutdown（`L319-L419`）。派生 worker 必须通过 host coordinator admission 后 `spawn`，不能在 mailbox receive 线程无限创建线程；用 `RuntimeEvent::Started/Completed/Closed` 映射 node 状态，`shutdown_and_join_with_grace` 的 detach 语义只能表示 unknown/keepalive，不能伪造 green。`InputBoundaryGate`（`L794-L912`）可作为“工具批次完成后才接收新 DAG 工作”的安全边界。
- `src/protocol.rs`：保留 JSONL 单行、有界文本/ID/TTL、版本兼容与 `MailboxRequest`；增加 `DagRequest` 的 `Inspect/Send/Heartbeat/Derive/Close`，在 `StdioRequest::into_command` 的版本分支中校验 node/session/relation，错误必须落到既有 `ProtocolError`，而非让 headless 进程退出。把 `target_id`、`session_id` 与 node id 分开，避免把 turn cancel 错当 DAG close。
- `src/approval.rs`：worker 的工具副作用必须继续经过 `ApprovalCoordinator`（`L126-L200`），取消时使用 `cancel_all/emergency_cancel`（`L409-L447`）；保留 accepted decision 在 journal 写入前不可消费、写入失败 retract 的 fail-closed 语义。`WorkerExecutionBinding` 可作为 approval request 的 correlation，但 worker “全绿”只能在 approval 记录、tool result、provider result 都持久化之后判定，不能把 mailbox `Complete` 先行当绿。
- `src/tool_runtime.rs`：复用 `execute_tool_batch` 的有界 call/byte budget、全 batch 取消与 join 语义（`L157-L176`、`L177-L217`），node terminal 判定接在所有 call 的 terminal result 之后。该模块明确 cancellation 不是 rollback、不会 detach（`L157-L167`）；所以 worker 被取消时写 `Cancelled/UnknownOutcome` 并保活/等待人工决策，不可直接 `Green` 或自动重试。
- `src/providers/**`：`providers/registry.rs:L1-L16` 的 local model catalogue、`ModelDescriptor` 能力/上下文/价格（`L74-L136`），`providers/connection.rs:L40-L135` 的 `ValidatedRoute` 与 auth revalidation 是 provider 选择边界；`anthropic.rs`、`codex.rs`、`deepseek.rs`、`google.rs`、`openai.rs` 继续只负责各 provider wire/capability。DAG worker 不应在 mailbox 中携带可替换的 URL、credential 或“模型能力声明”，只传经 `Agent`/registry 校验的 model id、route digest 与任务 payload。

### 差异清单与验收步骤

1. **已有、直接复用**：`MailboxRequest/SessionMailbox`、`LiveSessionRegistry`、`AgentEvent`、`BackgroundRunner/CancellationToken`、append-only `SessionStore`、`ApprovalCoordinator`、`execute_tool_batch`。验收：不改这些旧命令的 JSONL；跑既有 mailbox、runtime、approval、session tests，并用一条 `Send→Claim→Complete` 记录检查重启后结果仍可读。
2. **缺失、需要新增**：持久 `parent_id/children` DAG 索引、grandparent/sibling relation 校验、`DagWorkerContext`、derive intent/child attached/green/keepalive/close-deferred 记录、全后代绿 gate。验收：构造 `P→C→G` 与 `P→C1,C2`，让 G 未绿时 `close(P)` 只产生 keepalive；令 P/C1/C2/G 全绿后再次 close 才有 `dag_closed`，重复 close/derive 不增加记录。
3. **语义差异**：ACP `forkSession` 的 parent 是 SDK 隐式关系且没有全子孙关闭规则（`L362-L407`）；zenpi 必须把关系、generation、lease、state 写入可恢复记录。验收：杀掉 child owner 后重启，不能从孤立 filename 判 green；未完成 intent 进入 `UnknownOutcome/KeepAlive`。
4. **并发差异**：ACP 目录/MCP 可并发且部分 update 丢错；zenpi 的 DAG control plane 必须 bounded、可回放、幂等。验收：向同一 node 连续提交同一 request id、超出 mailbox/runtime 容量、并发 sibling claim，分别得到一次性结果、显式 QueueFull/容量错误、至多一个 claim owner。
5. **关闭/取消差异**：ACP close 会先删本地状态，zenpi close 只能在 durable gate 成功后删/归档；ACP abort 是 best-effort，zenpi cancel 必须保留 `Cancelled/UnknownOutcome` 证据。验收：cancel 与 provider side effect 竞态时，journal 不能出现 `Green`；shutdown grace 后只可 keepalive/reconcile。
6. **落点顺序**：先在 `src/protocol.rs` 加类型与校验，再在 `src/session.rs` 加 typed DAG/mailbox projection，然后在 `src/core.rs` 实现 close gate/derive coordinator，接入 `src/runtime.rs` 的 runner，最后由 `src/headless.rs` 路由。`src/approval.rs`、`src/tool_runtime.rs` 与 `src/providers/**` 只接 correlation/policy，不直接拥有 DAG 拓扑。每一步都可用纯 Rust 单测验证，无需 provider 网络。

## 未决问题

1. 从 `service.ts` 无法确认 `ACPSession.Interface`、`ACPEvent.start/runUntilIdle`、`Directory.build` 的内部锁、队列容量与事件去重策略；这里只能依据本文件的调用契约映射，需读取对应文件才能决定 Rust channel 的精确容量。
2. `sdk.session.fork` 的 backing session 是否在服务端保留完整父历史、是否能查询 parent id，源文件未暴露；zenpi 应以自己的 durable `parent_id` 为准，不依赖外部 fork 元数据。
3. 源文件未定义“节点绿”的领域语义，也未定义 child/grandchild 的完成聚合，因此全后代绿 gate、失败后是否人工重试、派生上限只能由 zenpi 产品策略确定；本笔记给出的最小策略是未知结果不算绿、失败不自动重试、保活优先。
4. `connection.sessionUpdate` 是否严格有序由 `ACPEvent`/连接实现决定；源只在 prompt 入口等待 `runUntilIdle`，其它 update 可能异步丢失，不能据此推断 zenpi 的事件投递可靠性。

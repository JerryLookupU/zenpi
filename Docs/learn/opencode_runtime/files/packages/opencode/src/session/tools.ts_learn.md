# OC-034 — packages/opencode/src/session/tools.ts

- source_id/item_id: `OC-034`
- source_path: `packages/opencode/src/session/tools.ts`
- source_hash: `5ad3f44682fc8e2bacb2222157943de8ab7dd663a1e0ddf5e4a213539b1f67d7`
- source_bytes: `23478`
- source_lines: `590`
- coverage: 已按源文件顺序读取完整内容，字节范围 `1-23478`，行范围 `L1-L590`（含注释、类型、导入、导出与尾部辅助函数）。

本文件是会话层把内建 `Tool`、MCP 资源工具和 MCP 普通工具转换为 AI SDK `AITool` 的适配器。所有执行都通过 `EffectBridge` 进入当前运行时，并在前后触发插件钩子；工具结果会被截断、规范化并补齐会话附件身份。

## 完整行为复盘

### 模块常量与依赖边界（L1-L39）

`resolve`依赖 `Agent`、`Provider.Model`、`Session.Info`、`SessionProcessor`、`ToolRegistry`、`Permission`、`MCP`、`Plugin`、`Truncate`、`RuntimeFlags`，并把 `ProviderTransform.schema` 的模型差异适配到 AI SDK 的 `jsonSchema`。`MCP_RESOURCE_TOOLS` 固定三个公开工具名：`list_mcp_resources`、`list_mcp_resource_templates`、`read_mcp_resource`（L27-L31）。二进制 MCP 资源只有 `application/pdf`、`image/gif`、`image/jpeg`、`image/png`、`image/webp` 可作为附件；单个 base64 解码后大小上限为 `10 * 1024 * 1024` 字节（L32-L39），超限或 MIME 不支持时保留可读的省略文本。

### `resolve`（导出，L41-L493）

输入对象含 `agent`、`model`、`session`、带 `message`/`updateToolCall`/`completeToolCall` 的 `processor`、`bypassAgentCheck`、消息数组 `messages` 和 `promptOps`（L41-L49）；输出是 `Effect`，成功值为 `Record<string, AITool>`。函数先创建空工具表，再从 `EffectBridge` 和各服务层取得运行器、插件、权限、注册表、MCP、截断器及运行时标志（L50-L58）。服务调用是生成式 `Effect`，在同一运行时中按顺序完成；注册表、MCP 客户端读取失败会使整个解析失败。

内部 `context` 工厂（L59-L90）把一次调用绑定到当前 `session.id`、处理器消息 ID、AI SDK `toolCallId`、`agent.name`、原始消息和 `extra={model,bypassAgentCheck,promptOps}`。`abort`直接取 `options.abortSignal!`，因此调用方按契约必须提供信号。`metadata(val)`只在当前 tool part 状态为 `running` 或 `pending` 时更新；写入标题、元数据、完整输入，并在首次从 `pending` 进入运行时使用 `Date.now()`，已有 `running` 状态则保留原 `time.start`（L67-L79）。`ask(req)`把请求与会话 ID、消息/调用 ID 和 `Permission.merge(agent.permission, session.permission ?? [])` 合并后调用权限服务，任何权限效果失败都由 `Effect.orDie`转为致命失败（L81-L89）。

第一段注册内建工具（L92-L134）：向 `registry.tools` 传入模型 ID、provider ID、agent 和会话权限，逐项用 `ToolJsonSchema.fromTool` 取得 schema，再经 `ProviderTransform.schema` 适配。每个条目的 `description`、`inputSchema` 原样暴露，`execute(args, options)`通过 `run.promise`运行效果：先触发 `tool.execute.before`（工具 ID、session/call ID、`{args}`），调用 `item.execute(args, ctx)`，然后给每个结果附件补 `PartID.ascending()`、session ID 和 message ID（L103-L120），再触发 `tool.execute.after`并返回输出（L121-L125）。若信号在返回时已 aborted，仍以已生成的 output 调用 `processor.completeToolCall`，没有回滚或重试（L126-L129）。

#### MCP 资源工具的条件与 `list_mcp_resources`（L136-L220）

只有至少一个 `mcp.clients()` 客户端声明 `serverCapabilities.resources` 时才注册三个资源工具（L136-L140）。`list_mcp_resources` 输入 schema 仅允许可选字符串 `server`，默认省略表示所有连接服务器（L140-L155）。执行时用 `parseListMcpResourcesArgs`解析；收集支持 resources 的客户端并按名称排序。指定但不支持的服务器立即抛错；错误会列出可用服务器，若为空则说明该服务器不支持 resources（L157-L171）。权限 pattern 为指定服务器的 `mcp:<server>:*`，或所有资源服务器的同类 pattern；先触发 before，再以 `read` 权限、服务器 metadata、相同 patterns 作为 `always` 请求审批（L172-L185）。从 `mcp.resources(parsed.server)`读取，按 server 过滤（冗余保护），以 `client\0name\0uri`稳定排序，转成不含 `client` 而含 `server` 的 JSON，交给 `truncate.output`（L187-L196、L523-L526）。输出标题按是否指定服务器变化，metadata 至少含 `count`、支持服务器列表、可选 `server`、`truncated`，截断时附 `outputPath`；`output`为截断内容（L197-L207）。随后触发 after；若信号 aborted 则完成 tool call，最后返回 output（L208-L217）。

#### `list_mcp_resource_templates`（L222-L303）

输入与服务器选择规则和列表工具相同，但描述明确表示 URI template 需要填参（L222-L238）。它调用 `mcp.resourceTemplates`，按 `client\0name\0uriTemplate`排序，包装键名为 `resourceTemplates`，其格式化函数同样删除 `client`并添加 `server`（L270-L279、L528-L531）。标题为 `MCP resource templates` 或带服务器后缀；metadata、插件钩子、截断与 aborted 后 `completeToolCall` 语义逐项等同列表工具（L280-L302）。

#### `read_mcp_resource`（L305-L385）

输入 schema 要求 `server`、`uri` 两个字符串且禁止额外字段；描述要求使用列表返回的精确值（L305-L325）。解析后先检查服务器已连接、且声明 resources，否则分别抛出“not connected”或“does not support resources”（L327-L337）。before 钩子之后，请求 `read` 权限；精确 pattern 为 `mcp:<server>:<uri>`，`always` 放宽为服务器通配符（L338-L348）。`mcp.readResource`返回空值会抛 `Failed to read MCP resource`。非空结果交给 `formatMcpResourceContent`，文本再截断；输出含标题、server/uri、内容计数、附件计数、截断信息、文本 output，以及为每个附件补 ID/session/message（L350-L371）。after 钩子在返回前执行；aborted 时完成 tool call（L373-L381）。

#### code mode 分支与普通 MCP 工具（L388-L493）

`flags.experimentalCodeMode`为真时在资源工具注册后立即返回，因此不会把 `mcp.tools()`普通工具加入表（L388-L388）。否则遍历 MCP 工具条目，将定义经 `McpCatalog.convertTool(def, client, timeout)`转换；没有 `execute` 的条目跳过。异步取得 `asSchema(item.inputSchema).jsonSchema`，保证 properties 至少为空对象，再经 provider schema 转换并回写 `item.inputSchema`（L390-L399）。

普通 MCP `execute`先构造上下文并触发 before；随后在 `Tool.execute` span 中以 `ctx.ask({permission:key, patterns:["*"], always:["*"]})`审批，再执行原函数。span 记录 tool name、call ID、session/message ID；审批或执行失败直接传播（L399-L419）。after 钩子接收原始 `result`（L420-L424）。结果内容逐项转换：`text`加入文本数组；`image`变成 `data:<mime>;base64,<data>` 文件附件；`resource`的 `text`加入文本，`blob`按默认 MIME `application/octet-stream`计算大小，未支持或超过 10 MiB 时写入明确省略原因，否则生成带原 URI filename 的文件附件（L426-L461）。文本以空行拼接并截断，metadata 合并原 metadata 与截断字段，输出标题为空、同时保留原始 `content`；附件补 Part ID、session/message（L464-L482）。aborted 后完成调用并返回；最终把包装好的 item 放入 `tools[key]`（L483-L492）。

### 辅助函数与最终导出（L495-L590）

`toRecord(value)`（L495-L498）使用 `isRecord`，非记录值统一变为空对象，因此列表/读取参数不会因 `null` 或数组本身崩溃。`parseListMcpResourcesArgs`（L500-L503）仅返回可选 `server`；`parseReadMcpResourceArgs`（L505-L508）返回必需 `server` 与 `uri`。`optionalString`（L510-L515）把 `undefined`、`null`、空字符串视为未提供；非字符串抛 `${key} must be a string`。`requiredString`在此基础上对缺失/空值抛 `${key} is required`，真值字符串（包括仅空白）通过（L517-L521）。

`formatMcpResource`（L523-L526）和 `formatMcpResourceTemplate`（L528-L531）浅复制对象、删除内部 `client` 字段并写入公开的 `server`。`formatMcpResourceContent(server, uri, content)`（L533-L576）把非数组 contents 包成单元素数组，再过滤非 record 项；每项 URI/MIME分别回退到请求 URI和 `application/octet-stream`。文本项输出 `Resource/MIME/文本`三段；blob 项先以 `base64Size`计算大小，按支持 MIME与 10 MiB 规则省略或附加，并记录“attached”标记；既无 text 也无 blob 时记录明确提示。返回有效 item 数、附件列表和以双换行连接的文本；全空时返回 `MCP resource <uri> from <server> returned no contents.`（L571-L575）。

`base64Size`去除所有空白，按 base64 长度计算字节并扣除 0/1/2 个尾部 `=` padding，结果以 `Math.max(0, …)`封底（L578-L582）；它不验证字符集或 padding 合法性。`formatBytes`以 `<1024`显示 B、`<1 MiB`向上取整显示 KB，否则向上取整显示 MB（L584-L588）。最后 `export * as SessionTools from "./tools"`把本模块作为命名空间再次导出（L590）。

## 状态、取消、恢复与副作用

- **调用状态与并发**：`resolve`本身是一次性构建快照；内建注册表工具和 MCP 工具都闭包捕获同一 `input`，实际 `execute`由 `EffectBridge.make().promise`提交给当前 Effect 运行时。注册循环是顺序的，工具调用之间没有本文件级的锁或去重；并发上限、队列和线程归属由外层运行时决定。MCP 普通工具的 `Effect.withSpan("Tool.execute")`只提供观测上下文，不改变并发策略（L395-L419）。
- **取消/超时**：所有工具从 `ToolExecutionOptions.abortSignal`读取信号并放入 `Tool.Context.abort`。本文件没有主动监听、超时计时器或重试；底层 `item.execute`/MCP `execute`是否检查信号由其实现决定。信号已 aborted 时，只有在结果已经构造后才调用 `completeToolCall`，这表示“完成已到达但请求已取消”的收尾，不撤销外部动作（L126-L129、L213-L215、L296-L297、L378-L380、L483-L485）。MCP catalog 的 `entry.timeout`仅传入 `McpCatalog.convertTool`，超时实现不在本文件（L390-L392）。
- **权限状态**：内建与资源工具通过 `Permission.merge`后的会话/agent ruleset 请求；资源读取使用细粒度 `mcp:<server>:<uri>`并以 server 通配符作为 always，普通 MCP 工具则用权限名 `key`和全局 pattern。`ask`使用 `Effect.orDie`，权限层失败会终止整个效果，而不是转换为普通工具错误（L81-L89、L343-L348、L407-L409）。
- **插件副作用**：每次执行先 `tool.execute.before`，成功生成结果后 `tool.execute.after`；before 失败会阻止实际执行，after 失败会阻止正常返回。钩子携带 session/call/tool 标识，内建工具的 after 看到已补齐附件 ID，MCP 普通工具的 after 看到原始 MCP `result`（L106-L125、L175-L212、L338-L376、L402-L424）。
- **持久化/恢复**：本文件不写 session journal、不保存审批决定、不实现 resume/retry。它只通过 `processor.updateToolCall`写运行中的工具元数据，并在 aborted 完成路径调用 `processor.completeToolCall`；附件 ID和截断 `outputPath`是本次返回结构，持久化责任在 `SessionProcessor`/会话层。资源读取、MCP tool 执行、插件钩子和可能的底层 `item.execute`都可能产生外部副作用，但本文件不提供事务回滚。
- **数据副作用与边界**：MCP blob 会被嵌入 data URL，内存占用受 10 MiB 单项检查约束但 base64 原字符串仍先由 MCP 返回；文本和列表在 `Truncate.output`中可能写出截断文件，返回 `outputPath`。`formatMcpResourceContent`仅过滤 record，不验证 MCP schema；`base64Size`也不验证 base64 合法性。

## 源内测试与行为判据

源文件本身没有测试代码（L1-L590仅实现与导出）。同目录测试引用为 `/Users/wangweiyang/GitHub/opencode/packages/opencode/test/session/tools.test.ts`：测试在 `L97-L166`构造 fake service 和 `timing` 工具，调用 `SessionTools.resolve`并执行包装后的工具，验证两次 `ctx.metadata`更新都保留原始 running `time.start=100`（断言在 `L161-L165`）。这直接判定了源文件 `context.metadata`在 `running`状态不能重置开始时间；可独立验证的最小判据是：给已有 running tool part 连续发送两次 metadata，`processor.updateToolCall`收到的 start 值应恒等于首次值，最终状态仍为 running。

还可独立验证以下行为判据：

1. 无资源能力的 MCP clients 时，返回表不含三个 `MCP_RESOURCE_TOOLS`；有能力时三者均出现，并且 list 输出按 `client\0name\0uri`稳定排序。
2. `parseReadMcpResourceArgs({server:"",uri:"x"})`必须抛 `server is required`；非字符串字段必须抛 `${key} must be a string`。
3. 资源 blob 使用不支持 MIME、或 `base64Size`大于 `10*1024*1024`时，输出包含 `[Binary MCP resource omitted: ...]`而不产生附件；支持且未超限时产生 `data:` URL和 filename。
4. `experimentalCodeMode=true`时，资源工具已经注册仍直接返回，`mcp.tools()`普通工具不应被枚举。
5. abort signal 在底层执行返回后为 aborted 时，`completeToolCall`必须收到带 session/message 补齐附件的最终 output；代码没有要求底层执行被中断。

## zenpi Rust 映射

当前 zenpi 已把“工具执行”拆在核心状态机和运行时，建议将本文件的职责映射为一个会话工具适配层，而不是把 TypeScript 的 `resolve`整体搬进单一函数。

| opencode 行为 | zenpi 建议落点 | 可执行映射与差异 |
|---|---|---|
| `resolve`输入快照、工具表构建、模型 schema 适配（L41-L134、L390-L493） | `src/core.rs` 的 `Agent`/`ToolRuntime`，配合 `src/tool_runtime.rs` | 在 `ToolRuntime`增加 `resolve_for_turn(model, agent, session, message, flags)`，返回按名称索引的可调用定义；沿用现有 `ToolRegistry`、`ToolContext`和 `ToolBatchOptions`。Rust 侧是同步/线程批处理，不是 Effect 生成器；需显式返回 `Result`并把 provider schema 转换失败归入 `ToolError`。 |
| `context`的 session/message/call 绑定与 metadata 更新（L59-L90） | `src/core.rs` 的 `ToolContext`及 live tool event sink | 将 `session_id`、`message_id`、`call_id`、`agent`和 model 放入不可变上下文；增加 `metadata(title, metadata, input)`回调，只有 pending/running 可更新并保留 `time.start`。现有 `src/core.rs`已有 `ToolProgress`/`ToolResult`事件，可用事件而非闭包修改 JS part。 |
| Agent/session permission merge 与 `ctx.ask`（L81-L89、L180-L185） | `src/approval.rs` 的 `ApprovalPolicy`/`ApprovalCoordinator`，`src/core.rs` 调度 | 内建工具调用前应由 `ApprovalPolicy::decide_after_preflight`决定，交互请求用 `ApprovalCoordinator::request_response`；把 `mcp:<server>:<uri>` pattern 和 `always`通配符序列化到 `ApprovalRequest`。差异是 Rust 审批可持久化、可取消，不能复刻 `Effect.orDie`的进程级致命语义，应返回 `ApprovalError`。 |
| `processor.completeToolCall`的迟到取消收尾（L126-L129 等） | `src/tool_runtime.rs` 的 `execute_tool_batch`与 `src/runtime.rs` 的 `CancellationToken` | `execute_tool_batch`已经在取消时产生 `ToolErrorCode::Cancelled`并等待 worker join；应额外判定“执行已返回但 token 已取消”并发出终态 `ToolResult`，不做回滚。以 `CancellationToken::is_cancelled`和 `ToolBatchOutcome`验证，而不是在工具包装器内直接改 session。 |
| MCP 资源 list/template/read 三个工具（L136-L385） | 建议新增 `src/mcp_runtime.rs`；权限接 `src/approval.rs`，输出接 `src/tool_output.rs`/`src/protocol.rs` | 当前列出的核心文件没有与 TS `MCP.Service`等价的资源目录适配层。新增 `McpResourceRegistry`：按服务器能力过滤、按 `client\0name\0uri`排序、format server 字段、限制 10 MiB、生成 `FilePart`/文本。为协议增加 `list_mcp_resources`、`list_mcp_resource_templates`、`read_mcp_resource`的 tool call 名和结构化输出；用 `SessionStore::append_event`记录工具结果。 |
| MCP 普通工具 schema/timeout/内容展开（L390-L489） | `src/tool_runtime.rs` + `src/providers/**` | `McpCatalog.convertTool`的 Rust 对应应把 provider 的 JSON schema 与 `src/providers/anthropic.rs`、`openai.rs`、`google.rs`、`deepseek.rs`、`codex.rs`的工具调用格式统一在 provider adapter；`entry.timeout`映射为执行选项或 `CancellationToken`截止时间。保留 text/image/resource 三路内容归一化，附件交给 `tool_output`并执行同一 MIME/10 MiB策略。现有 Rust 工具批次可并行，但有 sequential/unknown/mutating 模式，必须声明 MCP 工具副作用类型。 |
| 截断和 outputPath（L195-L207、L278-L289、L353-L371、L464-L482） | `src/tool_output.rs`（被 `src/core.rs`持有） | 复用 `SessionOutputStore`的捕获/读取协议；列表、模板、资源文本和 MCP 普通工具都应先统一截断再发布 metadata。差异是 TS 返回字符串 `outputPath`，Rust 协议已经有 `OutputRequest`/分页读取，应把路径转为受控 output handle，避免泄露绝对路径。 |
| 插件 before/after（L106-L125、L175-L212、L338-L376、L402-L424） | 当前 `src/core.rs` 的扩展/事件路径；必要时新增 `src/extension_runtime.rs` hook | 在 dispatch 前后发同一 tool/session/call correlation；before 拒绝不得执行，after 错误要成为工具失败。需保证并行批次每个调用各自触发一次，不能像 TS 一样依赖闭包隐式顺序。 |
| session/recovery 不由本文件实现（L495-L590及状态章节） | `src/session.rs`、`src/headless.rs`、`src/protocol.rs` | `SessionStore`已有 append-only JSONL、`begin_operation`/`finish_operation`、interrupted operation recovery；工具适配层只提交开始/结果/取消事件。`headless.rs`负责 resume/steer，`protocol.rs`已有 cancel/resume/tool_output 命令，适合承载宿主控制，不应在 `mcp_runtime`重复实现恢复。 |
| `SessionTools`命名空间导出（L590） | `src/lib.rs`或模块公开 API | Rust 以 `pub mod mcp_runtime`/`pub mod tool_runtime`暴露类型；用集成测试固定公共函数，而不是 JS namespace re-export。 |

与 `src/runtime.rs` 的并发差异尤其需要验证：TS 使用 Effect runtime 的 `run.promise`，源文件没有自己的队列、join 或批次取消；zenpi runtime 会拒绝满队列、传播取消并 join worker。迁移时应让 MCP/内建包装器只实现“准备上下文、权限、格式化结果”，把排队、并行度、join 和取消交给 `execute_tool_batch`。

与 `src/providers/**` 的差异是 provider schema/消息协议：TS 统一调用 `ProviderTransform.schema`后交给 AI SDK；zenpi 各 provider adapter 可能对 tool schema、图片/文件 content block 有不同限制。建议为 `ToolDefinition`增加经过 provider 能力裁剪后的 schema，并写跨 provider 快照测试：同一个 MCP schema 在 Anthropic/OpenAI/Google/Codex/DeepSeek 路径均保持必填字段与 `additionalProperties`语义。

建议的可验证落地顺序：

1. 先在 `src/tool_runtime.rs`补一个纯函数 `format_mcp_resource_content`及 `base64_size/format_bytes`，覆盖空内容、非法 record、padding、10 MiB 边界和五种允许 MIME。
2. 再增加 `McpResourceRegistry`的排序/服务器能力过滤测试，确认指定未知服务器产生可枚举错误且不会调用 provider。
3. 把审批 pattern 映射为 `ApprovalRequest`，测试 cancel 在等待审批、执行中、执行返回后三个时点的终态；断言没有 detached worker和错误回滚假象。
4. 在 `src/core.rs`接入 metadata 起始时间保持逻辑与 before/after 事件；用 `src/protocol.rs`的 tool output 读取接口验证截断内容和附件引用。
5. 最后为 `src/headless.rs` resume/recovery 场景追加一次 interrupted MCP operation，确认 `SessionStore::operation_recovery`给出待决状态，并由宿主决定重试或拒绝，而非自动重放副作用。

## 未决问题

1. `MCP.Service`、`McpCatalog.convertTool`以及 `ToolRegistry`各自对工具超时、连接断开和错误值的精确类型未在本文件定义；只能确认本文件会传播失败，无法确认是否自动重连或重试。
2. `processor.updateToolCall`与`completeToolCall`是否立即持久化、以及 aborted 后完成记录的最终状态编码，需要结合 `SessionProcessor`实现确认。
3. `Truncate.output`何时写 `outputPath`、路径生命周期和清理策略不在本文件；这里只能确认截断字段被透传。
4. `ProviderTransform.schema`对不同 provider 的具体 schema 改写规则未在本文件展开；资源工具的 `additionalProperties:false`是否被所有 provider 保留需 provider 级测试确认。
5. `base64Size`对非法 base64、超长空白、内部 padding 的行为仅由公式决定，源内没有验证测试；是否需要严格解码应由安全/兼容性需求另定。
6. `EffectBridge.make().promise`的 abort 传播和 worker 生命周期由运行时实现决定；本文件只在返回点检查 `abortSignal.aborted`，不能据此断言底层 IO 会中断。

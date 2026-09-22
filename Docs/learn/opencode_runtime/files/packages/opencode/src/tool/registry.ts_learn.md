# OC-048 — packages/opencode/src/tool/registry.ts

- source_id/item_id：`OC-048`
- source_path：`packages/opencode/src/tool/registry.ts`
- source_hash：`a8b24a6d58a80c42307e251905dbaa4f25ca0724569b1e531e412da934ab00fe`
- source_bytes：`17374`
- source_lines：`455`
- coverage：已从首字节到末字节完整读取，字节范围 `0–17373`（共 17374 字节），行范围 `L1-L455`（含全部导入、注释、类型、导出与结尾换行）。

## 完整行为复盘

文件把工具发现、规范化、按模型过滤和 Effect 服务注入集中到 `ToolRegistry`。导入区（`L1-L56`）表明它依赖 `Config`、`Plugin`、`Agent`、`MCP`、`RuntimeFlags`、`Truncate`、`Permission`、`InstanceState` 以及全部内建工具；`ProviderV2.ID`/`ModelV2.ID`只用于提供方和模型条件判断。

- `webSearchEnabled(providerID, flags = { exa: false, parallel: false })`（`L58-L65`）返回布尔值：提供方是 `ProviderV2.ID.opencode` 或新建的 `opencode-go`，或者 `flags.exa`/`flags.parallel` 任一为真时启用。未传 flags 时两开关均为假；没有异常、I/O 或副作用。
- `TaskDef`、`ReadDef`（`L67-L68`）分别是 `TaskTool`、`ReadTool` 经过 `Tool.InferDef` 推导的定义类型。`State`（`L70-L75`）保存 `custom`、`builtin`、命名的 `task` 与 `read`，是实例级缓存的最小状态。
- `Interface`（`L77-L87`）导出四个 Effect API：`ids()` 返回字符串 ID；`all()` 返回内建加自定义的 `Tool.Def[]`；`named()` 只返回 task/read；`tools({providerID, modelID, agent, permission?})` 返回供模型使用的定义。`permission` 可选，输入的 `agent` 是 `Agent.Info`。
- `Service`（`L89-L89`）是 Context service，键为 `@opencode/ToolRegistry`；`layer`（`L91-L349`）以 `Layer.effect` 构造服务。初始化先取得配置、插件、代理、截断器、运行时 flag、MCP 服务（`L93-L100`），再取得 invalid/task/read/question/todo/LSP/plan/webfetch/websearch/shell/glob/write/edit/grep/patch/skill 等内建工具（`L101-L116`）。`experimentalCodeMode` 为真时动态 `import("./code-mode")`，否则 `codeMode` 与 `codeModeTool` 为 `undefined`（`L118-L119`）；动态导入失败会使层初始化失败。
- 嵌套 `fromPlugin(id, def)`（`L125-L180`）把插件定义转成 `Tool.Def`。`def.args` 缺省为 `{}`，用 `Object.entries` 检测每个值是否 `isZodType`（`L131-L133`）；全为 Zod 时构造 `z.object(args)`、通过 `zodJsonSchema` 生成 wire schema，并以 `Schema.declare` 调用 `safeParse` 做运行时验证；否则使用 `legacyJsonSchema`，参数验证退化为 `Schema.Unknown`（`L134-L137`）。返回对象保留 `id`、description 与 schema。
  `execute(args, toolCtx)` 使用 `EffectBridge.make()` 将宿主 Effect 型 `ask` 包装成 Promise（`L143-L151`），并覆盖插件上下文的 `directory/worktree` 为当前 `InstanceState` 上下文；`def.execute` 在 `Effect.promise` 中运行，支持字符串结果或 `{output,title,metadata,attachments}` 结构（`L154-L158`）。通过 `agent.get(toolCtx.agent)` 找到代理，再调用 `truncate.output(output, {}, info)`；截断时返回截断内容并在 metadata 写入 `truncated: true` 与 `outputPath`，否则保留原文（`L159-L169`）。执行被 `Tool.execute` span 包裹，span 属性含工具、session/message/call ID（`L170-L179`）。Promise rejection、代理不存在、截断失败都会沿 Effect 错误通道传播；本函数不自行重试。
- `InstanceState.make<State>(Effect.fn("ToolRegistry.state", ...))`（`L121-L254`）只在实例状态构建时扫描自定义工具。先用 `config.directories()` 取得目录，对每个目录用 `Glob.scanSync("{tool,tools}/*.{js,ts}", {absolute:true,dot:true,symlink:true})`（`L183-L186`）；有匹配才 `config.waitForDependencies()`（`L187-L187`）。每个绝对路径以 `pathToFileURL(match).href` 动态导入，文件名去扩展名得到 namespace；只有 `isPluginTool` 的导出才注册，default 导出 ID 为 namespace，具名导出 ID 为 `${namespace}_${id}`（`L189-L196`）。随后遍历 `plugin.list()` 的 `p.tool`（空值按空对象），逐个经过 `fromPlugin`（`L199-L203`）。`config.get()` 在内建工具装配前被调用（`L206-L206`）。
  `questionEnabled` 仅在 client 为 `app/cli/desktop` 或 `enableQuestionTool` 为真时成立（`L207-L207`）。`Effect.all` 初始化内建工具映射，并在有 code mode 时加入 `execute`（`L209-L227`）；最终 `builtin` 顺序固定为 invalid、可选 question、shell/read/glob/grep/edit/write/task/fetch/todo/search/skill/patch，可选 execute、实验 LSP、以及仅 CLI 且开启实验 plan mode 的 plan（`L229-L249`）。`task/read`同时写入状态（`L250-L252`）。
- `all()`（`L256-L259`）读取 `InstanceState`，返回新数组 `[...builtin, ...custom]`，调用者不能通过数组改写缓存；`ids()`（`L261-L263`）把 `all()` 映射成 ID，顺序与该数组一致。
- `describeTask(agent)`（`L265-L278`）列出非 primary agent；用 `Permission.evaluate("task", item.name, agent.permission)` 排除 action 为 `deny` 的项；按名字 `localeCompare` 排序，description 缺省为固定手工调用提示，拼接成“可用 agent 类型及其工具”文本。代理列表失败会传播。
- `describeCodeMode({agent, permission?})`（`L280-L289`）无 code mode 时返回 `undefined`。否则合并 agent permission 与调用者 permission，再以 `Permission.visibleTools(yield* mcp.tools(), ruleset)` 筛选 MCP 工具；空集返回 `undefined`，非空时调用 `codeMode.describeCatalog`，并把 `mcp.clients()` 的键经 `McpCatalog.sanitize`传入。MCP 查询或 catalog 生成失败会传播。
- `tools(input)`（`L291-L339`）先从 `all()`过滤：web search 依 `webSearchEnabled`；`ApplyPatchTool` 仅在 model ID 含 `gpt-` 且不含 `oss`、不含 `gpt-4` 时保留；`EditTool`/`WriteTool`在该条件下反向隐藏（`L292-L303`）。存在 `execute` 时调用 `describeCodeMode`；只有有可见 catalog 才保留 execute（`L305-L309`）。对每个可见工具并发执行 `Effect.forEach(...,{concurrency:"unbounded"})`（`L310-L339`）：复制 description/parameters/jsonSchema 为可变输出，调用 `plugin.trigger("tool.definition", {toolID}, output)` 允许插件修改定义；若 parameters 未变且 jsonSchema 未变则返回 `jsonSchema: undefined`，否则返回新 schema（`L313-L323`）。description 追加 task agent 说明和 execute code-mode 说明；保留原 `execute` 与 `formatValidationError`。无界并发意味着插件 hook 的完成顺序不保证，但结果数组仍由 Effect 按输入位置组装；单个 hook 失败会使整体 Effect 失败。
- `named()`（`L342-L345`）再次从实例状态读取并返回缓存中的 task/read 定义引用。`Service.of({ids, all, named, tools})`（`L347-L348`）完成服务暴露。
- `isZodType(value)`（`L351-L353`）要求非空 object 且含 `_zod` 属性；这是结构检测，不验证具体 Zod 版本。`isPluginTool(value)`（`L355-L357`）要求非空 object 且同时含 `args`、`description`、`execute` 键；键存在但类型错误会进入后续并在运行时报错。`isJsonSchemaDefinition(value)`（`L359-L361`）接受 boolean 或非数组对象。
- `legacyJsonSchema(entries)`（`L363-L372`）过滤出合法 JSON Schema definition，构造 `type:"object"`、`properties`，并把所有保留属性名都列入 `required`；非法 entry 被静默丢弃。`zodJsonSchema(schema)`（`L374-L381`）用 `z.toJSONSchema(...,{io:"input",metadata:zodMetadataRegistry(schema)})` 转换并递归规范化；结果必须是对象，否则抛出明确 Error。它把顶层 `$defs` 改名为 JSON Schema 传统的 `definitions`。
- `zodMetadataRegistry(schema)`（`L383-L407`）创建 Zod registry，用 `WeakSet` 去重递归遍历 schema 和 `_zod.def`。每个 Zod 节点合并 `meta()` 对象与 `description` 字符串，非空时注册；普通对象则遍历 values。循环引用不会无限递归。
- `normalizeZodJsonSchema(value)`（`L409-L421`）递归处理数组和对象；删除值为 boolean 的 `exclusiveMaximum`/`exclusiveMinimum`，保留数值边界及所有其他键。`isJsonSchemaObject`（`L423-L425`）接受非空非数组 object。
- `node`（`L427-L453`）是导出的 `LayerNode`，声明 `Service`/`layer`，依赖 Config、Plugin、Question、Todo、Agent、Skill、Session、BackgroundJob、Provider、LSP、Instruction、FSUtil、EventV2Bridge、`httpClient`、CrossSpawnSpawner、Format、Truncate、RuntimeFlags、MCP、Database、Ripgrep；缺任一依赖不能编译层。`export * as ToolRegistry from "./registry"`（`L455-L455`）提供命名空间式公共导出。

## 状态、取消、恢复与副作用

状态只存在 `InstanceState` 的 `State` 缓存中（`L121-L254`），没有本文件写入的磁盘持久化、检查点或恢复协议；重建实例会重新扫描目录、动态 import 并重新列出插件。初始化和 `tools()` 使用 Effect 错误传播，但没有显式超时或重试。插件执行接收 `toolCtx.abort`（由调用者提供）且 `ask` 通过 `EffectBridge` 跨 Promise 保持上下文；registry 本身没有轮询 AbortSignal、取消动态 import 或强制终止插件 Promise。`truncate.output` 可能落盘输出文件并把路径放进 metadata；插件 `execute`、MCP catalog、配置读取和工具自身可产生外部副作用。`tools()` 的 `unbounded` 并发只涉及定义 hook，不代表工具执行并行；工具真正执行由返回的 `execute` 和上层 runtime 决定。没有 retry/rollback 语义，取消或 Promise 中断不等于已经发生的插件外部副作用被撤销。

## 源内测试与行为判据

源文件本身没有 `describe`/`test`（“源内未包含测试”）。同目录测试可独立验证：`test/tool/websearch.test.ts:L40-L45`覆盖 provider/flag 判定；`test/tool/registry.test.ts:L102-L150`验证 `task_status` 不暴露、code mode 开关及空 MCP 时隐藏 `execute`；`L170-L300`验证 singular/plural `.opencode/tool(s)` 扫描、忽略非工具导出及 `args: undefined` 空 schema 回退；`L302-L350`验证 Zod JSON Schema、必填校验和 prompt schema；`L420-L463`验证结构化结果的 attachments；`L465-L495`验证 legacy JSON Schema；`L497-L570`验证外部依赖动态加载。可执行判据是运行对应 Bun 测试，并检查 `ids/all/tools/named` 的顺序、过滤结果、schema 与 execute 输出。

## zenpi Rust 映射

- 注册核心落点建议放在 `src/tools.rs` 的 `ToolDefinition`/`ToolRegistry`（现有 `ToolRegistry::with_all_builtins`、`definitions`、`definition`），而 `src/tool_runtime.rs` 保持执行批次、并发和取消边界；`src/core.rs` 继续持有 `Arc<ToolRegistry>`、`ToolContext` 与 provider 调用。为对应 TypeScript 的动态插件，新增 `CustomToolLoader`（目录扫描、namespace/id 规则、JSON Schema/参数校验）并让 `ToolRegistry::definitions()` 合并 builtin/custom。
- `src/providers/registry.rs` 的 provider/model descriptor 与 `src/providers/mod.rs` 的 capabilities 可实现 `webSearchEnabled` 和 `ApplyPatchTool` 的模型条件；在 `src/core.rs` 组装发送给 backend 的 `ToolDefinition` 时执行同样过滤。`src/backend.rs` 已接收工具列表，故无需让 provider 文件直接执行工具。
- `src/approval.rs` 的 `ApprovalCoordinator` 对应上层决定是否允许副作用；插件 `ask` 应映射为 `ApprovalRequest`/`ApprovalResponse`，不能把 schema 可见性当授权。`src/session.rs` 的 append-only journal 可记录 dispatch/result/unknown outcome，但 registry 本身不应偷偷持久化。
- `src/runtime.rs` 的 `CancellationToken` 对应 Effect 中断/`AbortSignal`；`src/tool_runtime.rs::execute_tool_batch` 已规定取消时不 detached、结果按源顺序持久化，适合承载 custom tool execute。超时、取消后的未知副作用由 `src/core.rs`/`src/session.rs` 的 `OperationRecovery` 处理，而不是 loader 重试。
- `src/headless.rs` 与 `src/protocol.rs` 负责 JSONL 请求和回放；若要暴露 `ids/tools`，增加只读命令或在 prompt 事件中输出经过 policy 的定义，遵守现有大小界限。`src/session.rs` 可恢复 journal，但应明确 registry 的目录重扫不是结果恢复。
- 差异清单：TypeScript 使用 Effect `Layer/Context/InstanceState`，Rust 当前是显式构造和 `Arc`；TypeScript 允许 `.js/.ts` 动态 import 与插件 Promise，Rust 需选择受限 ABI/WASM/进程扩展；TypeScript 的 Zod schema 与 legacy schema 需要映射到 `serde_json::Value` + JSON Schema 校验；TS `Permission.visibleTools` 对 MCP 的可见性需对应 `approval.rs` policy；TS `Effect.forEach` 无界并发与 Rust `tool_runtime` 的最大 32 调用/字节预算不同，迁移时必须保留 Rust 的上限和 source-order 结果；TS 没有 registry 层重试，Rust 也应禁止 loader 自动重试副作用。

## 未决问题

无法从本文件确认：`Tool.Def` 的完整字段约束、`truncate.output` 的具体落盘位置和上限；插件 `def.execute` 在 Effect 中断时是否真正停止；`Plugin.trigger("tool.definition")` 是否允许异步修改后仍保证 schema 与 parameters 一致；`McpCatalog.sanitize` 的冲突处理；以及 `InstanceState` 在宿主重载时是否复用同一缓存。上述点需分别查阅对应模块或运行测试确认。

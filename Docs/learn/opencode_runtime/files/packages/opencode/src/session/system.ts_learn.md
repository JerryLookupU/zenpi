# OC-032 — packages/opencode/src/session/system.ts

## 元信息

- `source_id/item_id`: `OC-032`
- `source_path`: `packages/opencode/src/session/system.ts`
- `source_hash`: `7257f0660202d3e5d7ef86ee48cb4e51d5d0aa67703c274b8eb840b8ae307bbc`
- `source_bytes`: `6462`
- `source_lines`: `154`
- `coverage`: 已按源文件顺序读取完整字节范围 `0-6461`（共 `6462` 字节）及行范围 `L1-L154`。

## 完整行为复盘

文件先导入 `LayerNode`、Effect 的 `Context/Effect/Layer`，`InstanceState`，十种内置 prompt 文本，以及 `Provider`、`Agent`、`Permission`、`Skill`、`AbsolutePath`、`Location`、`Reference`、`MCP`、`PermissionV1` 等类型/服务（`L1-L26`）。这些导入表明本模块只负责把模型、工作区、skill 和 MCP 信息拼成系统提示，不负责执行模型请求。

- `provider(model: Provider.Model)`（`L28-L51`）按固定优先级返回只含一个字符串的数组。先检查 `model.api.id` 是否含 `muse`：`muse-glimmer` 使用 `PROMPT_META` 将所有 `{{MODEL_NAME}}` 替换为 `Muse Glimmer`，其他 Muse 使用 `Muse Spark`（`L29-L32`）。随后 `gpt-4`、`o1`、`o3` 命中 `PROMPT_BEAST`（`L33-L34`），所以这些模型不会继续落入普通 `gpt` 分支。普通 `gpt` 中 `gpt-6` 优先使用 `PROMPT_ASTRA`，含 `codex` 使用 `PROMPT_CODEX`，其他使用 `PROMPT_GPT`（`L35-L41`）。接着依次识别 `gemini-`、`claude`、大小写不敏感的 `trinity`，以及模型 ID 含 `kimi` 或 provider ID 属于 `kimi-for-coding`、`moonshotai`、`moonshotai-cn` 的情况，分别返回 `PROMPT_GEMINI`、`PROMPT_ANTHROPIC`、`PROMPT_TRINITY`、`PROMPT_KIMI`（`L42-L49`）。任何未命中值返回 `PROMPT_DEFAULT`（`L50-L51`）。输入没有显式空值保护，假设 `model.api.id` 和 `providerID` 已由 `Provider.Model` 保证；匹配是大小写敏感（只有 Trinity 分支主动 `toLowerCase`），也没有网络、重试或副作用。
- `Interface`（`L53-L57`）定义三项 Effect API：`environment(model)` 产出 `Effect.Effect<string[]>`；`skills(agent)` 和 `mcp(agent, permission?)` 产出可缺省的 `string | undefined`。`Service`（`L59-L59`）是名为 `@opencode/SystemPrompt` 的 `Context.Service`，作为依赖注入接口，不包含自身状态。
- `layer`（`L61-L140`）通过 `Layer.effect` 构造服务。它顺序取得 `Skill.Service`、`MCP.Service`、`LocationServiceMap.Service`（`L63-L67`），并返回 `Service.of` 的三个 `Effect.fn` 实现。
  - `environment(model)`（`L69-L105`）先读取 `InstanceState.context` 得到 `ctx`（`L70`）。然后从 `Reference.Service.list()` 读取引用，只保留 `description !== undefined` 的项，并通过当前目录 `AbsolutePath.make(ctx.directory)` 获取对应 location layer 后执行（`L71-L73`）。输出第一段固定环境文本：精确模型为 `providerID/api.id`，随后是 `ctx.directory`、`ctx.worktree`、Git 判断（`ctx.project.vcs === "git"`）、`process.platform` 和执行时的 `new Date().toDateString()`（`L74-L85`）。若没有引用，第二段为 `undefined`；有引用时按 `name.localeCompare` 排序，生成 `<available_references>`，每项包含 name、path、可选 description（`L86-L103`）。末尾过滤掉 `undefined`，所以结果数组只有一或两段字符串（`L104-L105`）。边界是“无描述引用永不展示”“无引用不生成空 XML 段”；目录、引用服务或上下文读取失败时 Effect 错误向调用者传播，没有本地 fallback、超时或重试。日期和平台是每次调用现场值；引用排序提供确定性，但引用内容没有转义。
  - `skills(agent)`（`L107-L119`）先用 `Permission.disabled(["skill"], agent.permission).has("skill")` 判断 skill 权限；禁用时立即返回 `undefined`，不会调用 `skill.available`（`L108-L109`）。允许时读取 `skill.available(agent)`（`L110`），再拼接三行说明和 `Skill.fmt(list, { verbose: true })`（`L112-L118`）。注释明确要求系统提示使用更详细版本，而工具描述可使用较简版本（`L115-L117`）。可用列表失败时错误传播；列表为空仍会返回包含说明和格式化空列表的字符串；源内没有长度截断、排序、持久化或重试。
  - `mcp(agent, permission?)`（`L121-L137`）将 `agent.permission` 与可选规则集合合并，缺省值为 `[]`（`L122`）。读取 `mcp.instructions()` 后保留“无工具限制”或“未被禁用工具数仍小于工具总数”的条目（`L123-L125`），因此只有全部工具都被禁用时才过滤 server；没有任何条目返回 `undefined`（`L126`）。有条目时输出 `<mcp_instructions>`，每个 server 用 `name` 属性包裹，并把多行 instruction 按行前置四个空格（`L128-L136`）。未对 XML 特殊字符做转义，`instructions` 的换行会被保留为缩进文本；MCP 读取失败或权限合并失败向上抛出，无缓存、超时、重试或写盘。
- `locationServiceMapNode`（`L142-L146`）把 `LocationServiceMap.Service` 与 `locationServiceMapLayer` 封装成无依赖 `LayerNode`。`node`（`L148-L152`）导出系统提示节点，依赖 `Skill.node`、`MCP.node` 和该 location 节点，表明服务初始化顺序由依赖图控制。文件末尾 `export * as SystemPrompt from "./system"`（`L154`）提供命名空间再导出；它不是额外运行逻辑。

## 状态、取消、恢复与副作用

本文件的 `Service` 与 `layer` 没有显式可变字段；每次 `environment`、`skills`、`mcp` 调用重新读取 context、references、skills 或 MCP instructions。Effect 的错误/中断语义由运行时承接，源中没有 `timeout`、`retry`、取消 token 检查或恢复分支，因此不能推断存在自动重试。读取 `process.platform`、当前日期和工作区引用是外部观测副作用；技能/MCP/location 服务可能执行各自读取，但本文件不写 session、不提交 provider turn、不产生 approval、不修改文件。Layer 构造是依赖注入注册，`node` 的依赖失败会使服务无法建立。并发调用之间没有本地共享计数器或锁；共享服务的线程安全、取消和生命周期由 `Skill.Service`、`MCP.Service`、`LocationServiceMap.Service` 实现决定。

## 源内测试与行为判据

源文件内未包含测试，也未在同目录 `src/session` 中看到针对该文件的测试引用。可独立验证的判据：

1. 以模型 ID 逐个调用 `provider`，断言 `gpt-4` 命中 Beast、`gpt-6-codex` 命中 Astra（由于 `gpt-6` 先于嵌套的 `codex`）、`gpt-codex` 命中 Codex、未知 ID 命中 Default；Muse 文本必须替换全部 `{{MODEL_NAME}}`。
2. 用含/不含 description 的 references 验证 `environment` 的过滤、按名称排序、Git 文本和一段/两段返回形状。
3. 用禁用 `skill` 的 `agent.permission` 验证 `skills` 返回 `undefined` 且 `available` 未被调用；允许时断言结果包含 `Skill.fmt(..., { verbose: true })`。
4. 为 MCP 条目构造空工具、部分禁用工具、全部禁用工具三种情况，验证保留规则和逐行四空格缩进。

## zenpi Rust 映射

- **模型与 provider prompt 路由**：源的字符串启发式应落在新模块 `src/system_prompt.rs` 的 `provider_prompt(&ModelDescriptor) -> &'static str`（Muse 另返回 `String`）。zenpi 的 `src/providers/registry.rs` 用精确 `(provider,id)` 的 `ModelDescriptor`、`resolve`（约 `L366-L373`）和能力交集（`L70-L81`），与源按 ID 子串猜测不同；可执行差异是保留显式 provider/id 规则，避免把 `gpt-6-codex` 的优先级藏在多个 `contains` 中，并为 Muse 模型增加明确字段或测试表。
- **环境段**：映射到 `SystemPromptBuilder::environment`，输入 `ProjectContext`/`SessionHeader`、当前 model 和引用索引，输出 `Vec<String>`。`src/session.rs` 是 append-only JSONL 持久化（文件头注释 `L1-L5`），不应把环境段写成 session 事件；`src/headless.rs` 的 workspace/read-only 观测接口可提供目录边界，但本源只需要工作目录、worktree、VCS、平台、日期和排序后的 references。建议用稳定排序（Rust `sort_by`/`BTreeMap`）并显式 XML 转义，测试无描述项和空引用项。
- **skills**：zenpi 已有 `src/skills.rs` 的 `SkillSet::metadata_index`（`L265-L277`）和 `instructions/effective_instructions`（`L323-L357`），`src/core.rs` 的 `Agent` 持有 `skills`（`L402-L439`），并可通过 `set_skills` 替换（`L2776-L2779`）。建议新增 `SystemPromptBuilder::skills(&Agent, PermissionView) -> Option<String>`，复用 `SkillSet::instructions` 或 `effective_instructions`，另提供等价于 `Skill.fmt(..., verbose: true)` 的确定性格式化；权限拒绝时短路，且不加载 skill body。Rust 已有 `SkillSet::load_body` 的取消和 hash 校验（`src/skills.rs:L282-L317`），这比源文件的无显式取消更严格，属于可保留差异。
- **MCP**：指定的 `src/headless.rs`、`src/core.rs`、`src/session.rs`、`src/tool_runtime.rs`、`src/runtime.rs`、`src/protocol.rs`、`src/approval.rs` 与 `src/providers/**` 没有等价的 `MCP.Service.instructions()` 抽象；`src/resources.rs` 仅提供资源类别统计。建议新增 `src/mcp.rs`：定义 `McpInstruction { name, instructions, tools }`、`McpRegistry::instructions()`，以及 `filter_by_permission`，输出与 `<mcp_instructions>` 对应的字符串。工具许可可接 `src/approval.rs` 的 `ApprovalPolicy`/`ApprovalDecision`（`L466-L520`），但必须区分“approval 允许一次调用”和“system prompt 中完全隐藏 server”，以复现源的“全部工具禁用才过滤”规则。
- **依赖注入与生命周期**：`Service`/`LayerNode` 可映射为 `Arc<SystemPromptServices>` 或在 `Agent` 构造时注入；`src/core.rs` 的 `Agent::new` 会恢复中断操作并初始化 skills（`L598-L640`），适合在此注入 immutable prompt services。初始化失败应在 `prepare_project` 发布替换前返回；该方法已有“先完整准备再发布”的约束（`src/core.rs:L725-L750`）。
- **协议、并发与取消**：源 API 是内部 Effect 函数，不直接暴露 wire 命令；zenpi 的 `src/protocol.rs` 将 `prompt/steer/cancel` 解码为 `Command`（`L211-L263`、`L377-L421`），由 `src/runtime.rs` 的 `CancellationToken`（`L49-L99`）和 `JobOutcome::{Succeeded,Failed,Cancelled,Panicked}`（`L156-L168`）承载取消/并发。建议系统提示构建作为 job 内的纯准备阶段，取消时返回 `BackendError::Cancelled`，不得写 session；若 provider prompt 已提交，取消不回滚外部副作用，符合 runtime 语义。
- **验证清单**：新增 Rust 单元测试覆盖 provider 路由优先级、环境输出排序/日期字段、skill 权限短路、MCP 全禁用过滤、XML 转义和取消不持久化；再用 `cargo test system_prompt` 与现有 `cargo test` 验证。检查 `src/session.rs` 事件计数保持不变，`src/headless.rs` JSONL 响应只在真正提交 turn 后出现，防止把提示构建误当作持久化事件。

## 未决问题

无法从源确认：`Skill.Service`、`MCP.Service`、`Permission.disabled/merge` 的具体实现和并发保证；`Provider.Model` 字段的完整约束；`LocationServiceMap` 提供的 layer 是否会缓存引用；`Skill.fmt` 的排序/空列表格式；以及上游 prompt 文本文件本身的内容。源文件也没有规定 XML 转义、长度预算、超时或重试策略。

# Docs/learn/opencode_runtime/packages/opencode/src/tool — 目录汇总（master 聚合）

- folder_path: `Docs/learn/opencode_runtime/packages/opencode/src/tool`
- files: 25
- total_source_bytes: 171721
- 说明：本汇总由主控从各文件 1:1 笔记确定性聚合（不新增未在笔记中出现的语义）。

## 模块清单与要点

- `packages/opencode/src/tool/apply_patch.ts` (11051 B) -> `Docs/learn/opencode_runtime/files/packages/opencode/src/tool/apply_patch.ts_learn.md`
  - 要点：1. **导出与依赖（L1-L33）**
- `packages/opencode/src/tool/code-mode.ts` (11808 B) -> `Docs/learn/opencode_runtime/files/packages/opencode/src/tool/code-mode.ts_learn.md`
  - 要点：`CODE_MODE_TOOL`（L12）是导出常量 `"execute"`；`DESCRIPTION`（L14）是静态描述。`Parameters`（L16-L20）是 Effect `Schema.Struct`，只接受必填字符串 `code`，并为该字段写入脚本用途说明；缺失或非字符串由 schema 解码失败。
- `packages/opencode/src/tool/edit.ts` (24530 B) -> `Docs/learn/opencode_runtime/files/packages/opencode/src/tool/edit.ts_learn.md`
  - 要点：文件开头的注释说明算法来源于 Cline/Gemini 的 diff edit 方案（`L1-L4`）。导入 `path`、Effect 的 `Schema`/`Semaphore`、`Tool`、`LSP`、`diff`、`FileSystem`/`Watcher`、`EventV2Bridge`、`Format`、`InstanceState`、`Snapshot`、外部目录检查、`FSUtil` 与 `Bom`，分别支撑路径、Ef
- `packages/opencode/src/tool/external-directory.ts` (1380 B) -> `Docs/learn/opencode_runtime/files/packages/opencode/src/tool/external-directory.ts_learn.md`
  - 要点：1. `L1-L6` 导入 `path`、`Effect`、`InstanceState`、`Tool.Context` 类型、`containsPath` 和 `FSUtil`。本文件不访问文件系统，也不直接持久化；路径拼接和 Windows 规范化由 `path`/`FSUtil` 完成。
- `packages/opencode/src/tool/glob.ts` (2895 B) -> `Docs/learn/opencode_runtime/files/packages/opencode/src/tool/glob.ts_learn.md`
  - 要点：L1-L8 导入 Node 的 path，Effect 的 Effect 与 Schema，实例上下文 InstanceState，文件系统服务 FSUtil，ripgrep 服务 Ripgrep，外部目录检查 assertExternalDirectoryEffect，工具描述文本 DESCRIPTION 和工具定义 API Tool。该文件本身不实现遍历算法，而是组合这些服务。
- `packages/opencode/src/tool/grep.ts` (4072 B) -> `Docs/learn/opencode_runtime/files/packages/opencode/src/tool/grep.ts_learn.md`
  - 要点：1. **模块依赖与导出参数 Schema（L1-L18）**
- `packages/opencode/src/tool/invalid.ts` (531 B) -> `Docs/learn/opencode_runtime/files/packages/opencode/src/tool/invalid.ts_learn.md`
  - 要点：`import { Effect, Schema } from "effect"`（L1）引入 Effect 运行时构造器和 Effect Schema 解码器；`import * as Tool from "./tool"`（L2）引入本地工具定义 API。文件没有其它隐式依赖或全局状态。
- `packages/opencode/src/tool/json-schema.ts` (5875 B) -> `Docs/learn/opencode_runtime/files/packages/opencode/src/tool/json-schema.ts_learn.md`
  - 要点：`fromSchema(schema: Schema.Top): JSONSchema7`（L8-L21）：先查 `cache`，命中即返回（L9-L10）。未命中时调用 `Schema.toJsonSchemaDocument(schema, { additionalProperties: true })`（L12），把文档的 `$schema` 固定为 `JsonSchema.META_SCHEMA_URI_DRAFT_2020_1
- `packages/opencode/src/tool/lsp.ts` (4329 B) -> `Docs/learn/opencode_runtime/files/packages/opencode/src/tool/lsp.ts_learn.md`
  - 要点：`operations`（`L11-L21`）是 `as const` 的九项白名单：`goToDefinition`、`findReferences`、`hover`、`documentSymbol`、`workspaceSymbol`、`goToImplementation`、`prepareCallHierarchy`、`incomingCalls`、`outgoingCalls`。它同时驱动 Schema 字面量校验和后续 `s
- `packages/opencode/src/tool/mcp-websearch.ts` (2998 B) -> `Docs/learn/opencode_runtime/files/packages/opencode/src/tool/mcp-websearch.ts_learn.md`
  - 要点：模块导入（L1-L2）：依赖 effect 的 Duration、Effect、Schema，以及 effect/unstable/http 的 HttpClient、HttpClientRequest。HTTP 请求、解码和超时都在 Effect 计算中完成。
- `packages/opencode/src/tool/plan.ts` (3131 B) -> `Docs/learn/opencode_runtime/files/packages/opencode/src/tool/plan.ts_learn.md`
  - 要点：1. 文件依赖与导出（L1-L11）。导入 Node 的 path，Effect 的 Effect/Schema，工具定义框架 Tool，以及 SessionV1、Question、Session、MessageV2、Provider、InstanceState、MessageID/PartID。MessageV2 在本文件没有直接使用，属于保留依赖或类型侧副作用；plan-exit.txt 的默认导出作为用户可见描述。文件没有定义新的
- `packages/opencode/src/tool/question.ts` (1528 B) -> `Docs/learn/opencode_runtime/files/packages/opencode/src/tool/question.ts_learn.md`
  - 要点：import 依赖（L1-L4）：引入 Effect、Schema，把本目录的 Tool 定义、../question 服务命名空间和 ./question.txt 描述文本接入。该文件没有自行实现 UI 或网络传输。
- `packages/opencode/src/tool/read.ts` (13126 B) -> `Docs/learn/opencode_runtime/files/packages/opencode/src/tool/read.ts_learn.md`
  - 要点：依赖与常量（L1-L19）：使用 `effect` 的 `Effect/Option/Schema/Scope/Stream`，`FSUtil`、`LSP`、`Instruction`、`InstanceState`、外部目录检查和媒体嗅探。`DEFAULT_READ_LIMIT=2000` 行，单行最多 `MAX_LINE_LENGTH=2000` 字符，输出总字节上限 `MAX_BYTES=50*1024`（标签为 `50 KB`）
- `packages/opencode/src/tool/registry.ts` (17374 B) -> `Docs/learn/opencode_runtime/files/packages/opencode/src/tool/registry.ts_learn.md`
  - 要点：`webSearchEnabled(providerID, flags = { exa: false, parallel: false })`（`L58-L65`）返回布尔值：提供方是 `ProviderV2.ID.opencode` 或新建的 `opencode-go`，或者 `flags.exa`/`flags.parallel` 任一为真时启用。未传 flags 时两开关均为假；没有异常、I/O 或副作用。
- `packages/opencode/src/tool/schema.ts` (444 B) -> `Docs/learn/opencode_runtime/files/packages/opencode/src/tool/schema.ts_learn.md`
  - 要点：1. **依赖导入（L1-L4）**
- `packages/opencode/src/tool/shell.ts` (20439 B) -> `Docs/learn/opencode_runtime/files/packages/opencode/src/tool/shell.ts_learn.md`
  - 要点：导入与导出（L1-L25）：依赖 Effect/Stream、ChildProcess、web-tree-sitter、文件流、路径、配置、插件、权限和截断服务；唯一显式导出是从 ./shell/prompt 转出的 Parameters（L25）。
- `packages/opencode/src/tool/skill.ts` (2215 B) -> `Docs/learn/opencode_runtime/files/packages/opencode/src/tool/skill.ts_learn.md`
  - 要点：1. 模块导入（L1-L6）：使用 Node path 做目录和绝对路径处理；从 effect 引入 Effect、Schema；注入 @opencode-ai/core/ripgrep 的 Ripgrep 服务；引入 ../skill 的 Skill 服务、./tool 的工具定义器，以及 ./skill.txt 文本资源作为 DESCRIPTION。文件本身不实现技能发现或文件读取，只编排这些服务。
- `packages/opencode/src/tool/task.ts` (14200 B) -> `Docs/learn/opencode_runtime/files/packages/opencode/src/tool/task.ts_learn.md`
  - 要点：1. **依赖、接口与静态提示（L1-L52）**
- `packages/opencode/src/tool/todo.ts` (1338 B) -> `Docs/learn/opencode_runtime/files/packages/opencode/src/tool/todo.ts_learn.md`
  - 要点：依赖导入（L1-L4）：引入 Effect、Schema，工具定义命名空间 Tool，静态说明文本 DESCRIPTION_WRITE（来自 ./todowrite.txt），以及 Todo.Info/Todo.Service。本文件不实现数据库、事件或权限策略，只把这些能力接到工具生命周期。
- `packages/opencode/src/tool/tool.ts` (6130 B) -> `Docs/learn/opencode_runtime/files/packages/opencode/src/tool/tool.ts_learn.md`
  - 要点：导入与 `Metadata`（L1-L13）：依赖 `PermissionV1`、`SessionV1`、`Effect/Schema`、`JSONSchema7`、会话类型、`Truncate`、`Agent`。`Metadata` 是允许任意键值的索引接口，因而工具结果元数据可扩展但缺少静态字段约束。
- `packages/opencode/src/tool/truncate.ts` (6152 B) -> `Docs/learn/opencode_runtime/files/packages/opencode/src/tool/truncate.ts_learn.md`
  - 要点：1. **依赖与全局常量**：`L1-L10` 导入 Effect 运行时、`FSUtil`、权限 `evaluate`、`Config`、`ToolID` 与 `TRUNCATION_DIR`。`NodePath` 在本文件中没有被后续表达式使用。`RETENTION` 是 7 天；`MAX_LINES=2000`、`MAX_BYTES=50*1024`；`DIR` 是截断目录，`GLOB` 是该目录下的 `*` 路径（`L12-L1
- `packages/opencode/src/tool/truncation-dir.ts` (148 B) -> `Docs/learn/opencode_runtime/files/packages/opencode/src/tool/truncation-dir.ts_learn.md`
  - 要点：`import path from "path"`（L1）：加载 Node.js 的 `path` 默认导出，后续使用其 `join` 进行平台相关的路径拼接。这里没有读写文件，也没有创建目录；导入模块本身可能触发 Node 模块加载，但本文件没有额外初始化逻辑。
- `packages/opencode/src/tool/webfetch.ts` (6881 B) -> `Docs/learn/opencode_runtime/files/packages/opencode/src/tool/webfetch.ts_learn.md`
  - 要点：1. L35-L37：url 必须以 http:// 或 https:// 开头，否则抛出 URL must start with http:// or https://；不是完整 URL 解析。
- `packages/opencode/src/tool/websearch.ts` (5189 B) -> `Docs/learn/opencode_runtime/files/packages/opencode/src/tool/websearch.ts_learn.md`
  - 要点：`Parameters`（L10-L25）是导出的输入 Schema。必填 `query: string`。`numResults`、`livecrawl`、`type`、`contextMaxCharacters` 可省略；其中 `livecrawl` 只接受 `fallback|preferred`，`type` 只接受 `auto|fast|deep`，其余两个是数字。注解给出了文档默认值：结果数 8、livecrawl 为 fa
- `packages/opencode/src/tool/write.ts` (3957 B) -> `Docs/learn/opencode_runtime/files/packages/opencode/src/tool/write.ts_learn.md`
  - 要点：`L1-L16` 是依赖边界：`Schema`/`Effect` 提供参数校验和 Effect 组合，`path` 做路径运算，`Tool` 定义工具契约；`LSP`、`FileSystem`、`Watcher`、`EventV2Bridge`、`Format`、`FSUtil` 是注入服务；`Bom` 处理字节顺序标记；`createTwoFilesPatch` 与 `trimDiff` 生成审批 diff；`DESCRIPTION`

## zenpi Rust 映射（聚合）

- `packages/opencode/src/tool/apply_patch.ts`: **工具落点**：建议新增 `src/tools/apply_patch.rs` 并由 `src/tools.rs` 注册。现有 `ToolDefinition`、`ToolCall`、`ToolResult`、错误码、`ToolPreview::Diff` 在 `src/tools.rs:L502-L572,L642-L718`；`with_all_builtins` 当前注册 write/edit/run command（`L107
- `packages/opencode/src/tool/code-mode.ts`: **工具执行落点**：`src/tool_runtime.rs` L157-L176 的 `execute_tool_batch` 已有批量、顺序/并行、调用前 `prepare`、结果按源顺序归并、取消轮询和 panic 转内部错误；可新增 `code_mode.rs`，将脚本调用编译为 `ToolCall`，把 `toolTree` 的 namespace/path 映射为 `ToolRegistry` 名称。其 L177-L217
- `packages/opencode/src/tool/edit.ts`: `zenpi/src/tools.rs` 已有最接近的落点：`EditFileTool` 定义为 `WorkspaceWrite`，schema 是 `path/old/new` 且只允许一次精确 occurrence（`src/tools.rs:L2108-L2170`）；`approval_preview` 读取有界文本、计算 source SHA-256 和 unified diff（`L2111-L2149`），`invoke`
- `packages/opencode/src/tool/external-directory.ts`: `src/tools.rs` 的 `ToolContext::new`（`L747-L773`）负责 canonical workspace root，`resolve_existing`（`L979-L1003`）负责 workspace 内路径。建议在 `src/tools.rs` 增加纯函数 `external_directory_request(target: Option<&str>, kind: ExternalDirect
- `packages/opencode/src/tool/glob.ts`: 核心落点：zenpi 的最接近实现是 src/tools.rs 的 FindFilesTool，其定义声明工具名 find、只读副作用、Parallel 执行模式，并通过 ToolContext::resolve_existing 限制在固定 workspace_root。建议新增独立 GlobTool（或给 FindFilesTool 增加兼容别名 glob），输入字段保持 pattern 与可选 path，输出增加与源一致的 pat
- `packages/opencode/src/tool/grep.ts`: **类型/Schema**：新增 `GrepTool`、`GrepArguments { pattern: String, path: Option<String>, include: Option<String> }` 或直接沿用 JSON `ToolCall`。Schema 名称可取 `"grep"`，`pattern` 必填，`path` 默认 `"."`，`include` 为 glob；保留 `ToolSideEffect::
- `packages/opencode/src/tool/invalid.ts`: **工具定义落点：** zenpi 的 `src/tools.rs` 已有 `ToolDefinition`、`Tool` trait、`ToolRegistry` 和 `ToolResult`。建议新增 `InvalidTool`（可与其它 builtin 放在同文件），`definition()` 返回 `name: "invalid"`、`description: "Do not use"`、对象型 `input_schema`（
- `packages/opencode/src/tool/json-schema.ts`: 类型落点：`pub type JsonSchema = serde_json::Value`；`SchemaError` 用 `thiserror` 表达非法结构/大小/循环；`from_schema(value: &Value) -> Result<Value, SchemaError>`、`from_tool(def: &ToolDefinition) -> Result<Value, SchemaError>`；内部实现 `nor
- `packages/opencode/src/tool/lsp.ts`: `src/tools.rs`：建议新增只读 `LspTool` 实现，或拆出 `src/lsp.rs` 后由 `ToolRegistry` 注册。定义 `LspOperation`（九个枚举变体）、`LspRequest { operation, file_path, line, character, query }` 与 `LspResult { title, result: serde_json::Value, output }`；
- `packages/opencode/src/tool/mcp-websearch.ts`: 建议新增 src/mcp_websearch.rs（或放入现有工具模块）承载 ExaUrl/ParallelUrl 常量、McpResult、SearchArgs、ParallelSearchArgs、McpRequest<F> 的 serde 类型，以及 parse_payload、parse_response、call。用 serde/serde_json 做结构校验，用现有 HTTP 客户端抽象（若无统一抽象则在 provider
- `packages/opencode/src/tool/plan.ts`: 会话承载可落在 src/session.rs 的 SessionStore（L146）及 append_turn（L863）、append_event（L1207）、events（L1223）。zenpi 的持久化是追加式 JSONL，append_turn 先持久化再更新内存投影；建议新增一个经过校验的 PlanApprovalTurn/BuildHandoff 记录，原子地表达“批准”和合成提示，避免 TS 中 message/pa
- `packages/opencode/src/tool/question.ts`: 类型落点：QuestionPrompt { question, header, options, multiple }、QuestionAnswer(Vec<String>)、QuestionRequest { id, session_id, questions, tool }、QuestionError::{Rejected, NotFound, Cancelled}、QuestionService。用 serde 加显式 valid
- `packages/opencode/src/tool/read.ts`: `src/tools.rs` 的 `ReadFileTool`（L1515-L1589）是最直接落点：对应 `ToolDefinition{name:"read_file", side_effect:ReadOnly}`、`ToolContext::resolve_existing`、UTF-8 校验和 bounded read。建议新增 `ReadTool` 或扩展 `ReadFileTool` 参数为 `path/offset/li
- `packages/opencode/src/tool/registry.ts`: 注册核心落点建议放在 `src/tools.rs` 的 `ToolDefinition`/`ToolRegistry`（现有 `ToolRegistry::with_all_builtins`、`definitions`、`definition`），而 `src/tool_runtime.rs` 保持执行批次、并发和取消边界；`src/core.rs` 继续持有 `Arc<ToolRegistry>`、`ToolContext` 与 p
- `packages/opencode/src/tool/schema.ts`: **建议落点**：新增 src/tool_id.rs（并在 lib.rs 导出，或把小型类型放入现有 src/tools.rs）；定义 pub struct ToolId(String)，实现 TryFrom<&str>/FromStr、Serialize/Deserialize、Display，校验 starts_with("tool")。提供 pub fn ascending(given: Option<&str>) -> Resu
- `packages/opencode/src/tool/shell.ts`: src/tools.rs::RunCommandTool（现有 L2219 起）是最接近落点：它已有 invoke_with_cancel/invoke_user_shell_with_cancel、命令超时、进程组清理、stdout/stderr 限制和 artifact capture。应新增独立 ShellTool 或扩展该类型，加入 tree-sitter 等价扫描、PowerShell/CMD 分支、合并输出和工作目录审批。
- `packages/opencode/src/tool/skill.ts`: 技能模型/加载：zenpi 的 src/skills.rs 已有 SkillSet、SkillMetadata、SkillBody、SkillResource、ModelSkillTools。建议把 Parameters 映射为 SkillToolArgs { name: String }，把 Skill.require 映射为 SkillSet::load_body(name, SkillInvocation::Model, "", 
- `packages/opencode/src/tool/task.ts`: `TaskPromptOps` 建议落在 `src/core.rs` 的 `SubagentTaskOps` trait（对照 `Agent::process_with_cancel` L5327-L5351、`run_active_turn_cancelable` L3642-L3678），实现 `resolve_prompt_parts`、`prompt`、`cancel`，由 `Agent`/会话宿主注入；不要把 provider
- `packages/opencode/src/tool/todo.ts`: src/tools.rs（核心落点）：新增 TodoItem { content: String, status: String, priority: String } 与 TodoWriteArgs { todos: Vec<TodoItem> }，为 ToolRegistry 注册 todowrite。输入 schema 要求对象和 todos 数组，保持源实现“不在工具层限制长度/状态枚举”的差异。工具执行先走现有 ToolCon
- `packages/opencode/src/tool/tool.ts`: `src/tools.rs` 的 `ToolDefinition`、`ToolContext`、`ToolResult`、`ToolRegistry`（约 L505、L747、L1057 及 L1196-L1324）是最接近的运行时落点：将 `Def` 映射为含 `name/description/schema/execute` 的 trait 对象，将 `ExecuteResult` 映射为 `ToolResult { title, 
- `packages/opencode/src/tool/truncate.ts`: `src/core.rs`：在 `run_command` 捕获流程（现有 `begin_output_capture`/`persist_output_capture`）之后生成 bounded preview；保留现有 `OutputError`、磁盘治理和 `tool_output_captured` 事件，不直接复刻 TypeScript 的裸路径字符串。
- `packages/opencode/src/tool/truncation-dir.ts`: 1. 若要表达“数据根目录下的工具输出目录”这一纯配置概念，可在 `src/tool_output.rs` 增加私有常量或纯函数 `fn tool_output_dir(data_root: &Path) -> PathBuf { data_root.join("tool-output") }`；函数只拼接路径，不创建目录，直接对应 L4。若 zenpi 需要跨平台稳定性，应使用 `PathBuf::join`，不要手工拼接 `/`。
- `packages/opencode/src/tool/webfetch.ts`: 工具落点：在 src/tools.rs 的 Tool/ToolRegistry 或新建 src/tools/webfetch.rs。现有 ToolDefinition 需要对象 schema，ToolResult 用 Success/Error 关联 call_id（src/tools.rs:L502-L535、L648-L663），可把结果编码为 {title,output,metadata,attachments}。当前 built
- `packages/opencode/src/tool/websearch.ts`: `src/tools.rs` 的 `Tool`、`ToolDefinition`、`ToolContext`、`ToolRegistry`、`ToolResult` 是最直接落点：新增 `WebSearchTool` 实现 `definition()` 与 `invoke_cancellable()`，输入 JSON 对应 `Parameters`，输出对应 `ToolResult::Success { output }`。建议新建 `
- `packages/opencode/src/tool/write.ts`: 主落点是现有 `src/tools.rs::WriteFileTool`：`definition`/`invoke` 对应 `Parameters` 与 `execute`，`ToolPreview::Diff` 对应 `createTwoFilesPatch`/`trimDiff` 的审批预览；`ToolContext::resolve_for_write`、`atomic_write_text` 已提供工作区约束、临时文件写入、`s

## 未决问题（聚合）

- `packages/opencode/src/tool/apply_patch.ts`: 无法从本文件确认 `Format.Service` 是否原子、`LSP.diagnostics()` 是否阻塞、`EventV2Bridge.publish` 的失败策略；实现位于导入模块。
- `packages/opencode/src/tool/code-mode.ts`: 1. `@opencode-ai/codemode` 解释器的资源限制（CPU、内存、脚本最大时长、`$codemode.search` 的完整实现）不在本源文件内，无法从本文件确定。
- `packages/opencode/src/tool/edit.ts`: 1. `assertExternalDirectoryEffect` 的外部目录允许/拒绝规则不在本文件，无法仅凭 `edit.ts` 确认是否允许绝对路径越出 worktree（调用点见 `L79-L83`）。
- `packages/opencode/src/tool/external-directory.ts`: 1. 本文件无法确认 `ctx.ask` 对 `always` 的具体持久化、重复请求合并、UI 展示和拒绝错误类型；这些属于 `Tool.Context`/permission 实现。
- `packages/opencode/src/tool/glob.ts`: Tool.define 对 Effect.orDie defect 的最终捕获、展示和工具消息编码未在本文件中给出。
- `packages/opencode/src/tool/grep.ts`: `Ripgrep.Service.grep` 的具体 JSON 字段、排序、正则错误类型和是否自身截断，无法仅由本文件确认（调用点只使用 `item.entry.path/line/text`，L63-L78）。
- `packages/opencode/src/tool/invalid.ts`: 1. `Schema.Struct` 对未知字段的精确策略（保留、剥离或拒绝）及其版本差异，无法仅由 `invalid.ts` 确认。
- `packages/opencode/src/tool/json-schema.ts`: 
- `packages/opencode/src/tool/lsp.ts`: 1. `LSP.Service` 各方法的具体返回类型、服务器选择算法、`touchFile` 是否启动进程，无法仅由本文件确认。
- `packages/opencode/src/tool/mcp-websearch.ts`: 1. HttpClient.filterStatusOk 对非 2xx 的具体错误类型和是否保留响应正文，源文件未说明。
- `packages/opencode/src/tool/plan.ts`: 1. Session.plan(info, instance) 的具体计划文件命名、是否保证存在以及是否总在 instance.worktree 下，无法由本文件确认。
- `packages/opencode/src/tool/question.ts`: Question.Prompt 的最终约束由 @opencode-ai/schema/question-v1 提供；本文件只能确认其被引用，无法单独确认运行时是否会拒绝空 options、超长 header 或缺失可选字段。
- `packages/opencode/src/tool/read.ts`: 1. `read.txt` 的完整 description 文本未在本文件内展开，无法仅凭 `read.ts` 确认最终展示文案。
- `packages/opencode/src/tool/registry.ts`: 
- `packages/opencode/src/tool/schema.ts`: 无法仅由该源文件确认 Effect Schema 当前版本对 schema.make 失败时抛出的具体错误类/消息格式；需按锁定依赖版本运行判据 5。
- `packages/opencode/src/tool/shell.ts`: 
- `packages/opencode/src/tool/skill.ts`: Skill.Service.require 返回的 info.location 是否始终为文件路径、是否已做路径安全校验，源文件无法确认。
- `packages/opencode/src/tool/task.ts`: `Tool.define`、`BackgroundJob.Service`、`Session.Service`、`Agent.Service` 的具体实现、锁粒度、队列容量和重连语义不在本源文件中。
- `packages/opencode/src/tool/todo.ts`: 1. 仅凭本文件无法确定 ctx.ask 是否监听 ctx.abort，以及权限请求的超时策略。
- `packages/opencode/src/tool/tool.ts`: `Truncate.output` 的具体字节/行阈值、`outputPath` 的持久化位置不在本文件中。
- `packages/opencode/src/tool/truncate.ts`: 1. `TRUNCATION_DIR`、`ToolID.ascending()` 和 `Config.Service` 的具体实现及路径权限不在本文件中，无法确认目录是否跨进程共享、ID 是否必然以 `tool_` 开头。
- `packages/opencode/src/tool/truncation-dir.ts`: 1. `Global.Path.data` 的实际来源、默认值、是否绝对路径以及是否保证字符串类型，无法从本文件确认，需要查看 `@opencode-ai/core/global`。
- `packages/opencode/src/tool/webfetch.ts`: 1. ctx.ask 的 always=["*"] 精确匹配和持久化语义不在源文件内。
- `packages/opencode/src/tool/websearch.ts`: 1. 本文件未给出 `checksum` 的具体算法；无法仅凭本源确认其对任意 session ID 的 base36 输出，Rust 映射必须读取 `@opencode-ai/core/util/encode` 后做跨语言 fixture。
- `packages/opencode/src/tool/write.ts`: 1. `assertExternalDirectoryEffect` 对 workspace 外路径的精确允许规则，以及 `ctx.ask` 的 `always: ["*"]` 是否会持久化授权，无法从本文件确认。

## 覆盖

- 本目录 25 个源文件均有 1:1 笔记；`file_learn_index.tsv` 与本清单一一对应。

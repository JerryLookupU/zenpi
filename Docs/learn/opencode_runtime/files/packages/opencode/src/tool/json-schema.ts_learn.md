# OC-042 — packages/opencode/src/tool/json-schema.ts

- source_id/item_id：`OC-042`
- source_path：`packages/opencode/src/tool/json-schema.ts`
- source_hash：`123ae7e75b54161e54645d14f614700674acb6eb293d8a1249a9de57d84f4dba`
- source_bytes：`5875`
- source_lines：`164`
- coverage：已按顺序读取字节 `0-5874`、行 `L1-L164`，包含全部导入、类型、常量、注释、函数和导出。

## 完整行为复盘

文件把 Effect `Schema.Top` 或工具定义转换成发给模型/Provider 的 JSON Schema。`JsonObject` 是 `Record<string, unknown>`，`cache` 是以 `Schema.Top` 对象身份为键的 `WeakMap<Schema.Top, JSONSchema7>`（L5-L6）；因此缓存不阻止 schema 被垃圾回收，且同一 schema 对象重复转换可复用同一个结果对象。

- `fromSchema(schema: Schema.Top): JSONSchema7`（L8-L21）：先查 `cache`，命中即返回（L9-L10）。未命中时调用 `Schema.toJsonSchemaDocument(schema, { additionalProperties: true })`（L12），把文档的 `$schema` 固定为 `JsonSchema.META_SCHEMA_URI_DRAFT_2020_12`，合并 `document.schema`，仅在 definitions 非空时将其放进 `$defs`（L13-L17）。随后先 `normalize`，再 `inlineLocalReferences`，最后 `dropDefinitionsIfResolved`（L18）。如果最终值既不是布尔 schema 也不是对象，`isJsonSchema` 失败并抛出 `Error("tool JSON Schema helper produced a non-schema value")`（L19）；否则写入弱缓存并返回（L20-L21）。底层转换异常不会被捕获，会直接向上传播。该函数同步执行，没有异步等待、取消点或重试；转换过程本身没有文件、网络或持久化副作用。
- `fromTool(tool: Tool.Def): JSONSchema7`（L24-L25）：优先返回工具显式提供的 `tool.jsonSchema`；只有其为 `null`/`undefined` 时才把 `tool.parameters` 视为 `Schema.Top` 调 `fromSchema`。显式 schema 不经过本文件的规范化、内联或缓存，这是一个重要边界。
- `normalize(value, options = {})`（L28-L88）：递归规范化未知 JSON 值。数组逐项递归（L29）；非 record 原样返回（L30）。先把字符串类型的 `required` 转为集合（L32-L34）；对象每个键递归，`properties` 下的属性额外传递 `stripNull: !required?.has(name)`，所以非必填属性会尝试去掉 null 分支，而必填 nullable 属性保留（L35-L47）。顶层或任意对象中的 `additionalProperties: true` 被删除，`false` 或其他值保留（L49）。当 `stripNull` 且 `anyOf` 存在时，移除 record 且 `type === "null"` 的项；有移除就重新递归规范化并立即返回（L51-L54）。
  - 对 `anyOf` 的兼容性折叠（L56-L76）：若存在 `type: "number"` 分支，且其余所有分支都是 enum 且每个枚举值均为 `"NaN"`、`"Infinity"` 或 `"-Infinity"`，就删除 `anyOf`，把 number 与其余父字段合并（L57-L65）；恰好由“无 properties 的 object”和“无 items 的 array”组成的空结构联合被视为 `{ type: "object", properties: {}, ...rest }`（L67-L70）；只剩一个 record 分支时将它展开到父对象（L72-L76）。空 `anyOf`、非 record 单分支或循环引用不会在这些分支中被强行修复。
  - `allOf` 只有在所有项为 record 且 `canFlattenAllOf` 为真时才展开（L78-L82）。展开时先 `Object.assign` 合并各子对象，再把父对象其余字段覆盖到合并结果；重复键会阻止展开，因此不会丢弃重复约束。
  - 对 `type === "integer"` 且没有 `maximum` 的 schema 自动补 `minimum: Number.MIN_SAFE_INTEGER` 和 `maximum: Number.MAX_SAFE_INTEGER`（L83-L85）。已有 `minimum` 保留；已有 `maximum` 则整个补界逻辑不触发。其他值原样返回 record（L87）。函数没有异常返回类型，递归只处理内存中的 JSON-like 值。
- `isRecord(value)`（L90-L92）：仅接受非 null、`typeof === "object"` 且不是数组的值；数组、null、原始值均拒绝。
- `isJsonSchema(value)`（L94-L96）：接受布尔值或 `isRecord` 对象，符合 JSON Schema 可用布尔 schema 的形状；不会验证 `$schema`、`type`、`properties` 等关键字的完整规范。
- `isNonFiniteNumber(value)`（L98-L100）：只把三个字符串 `"NaN"`、`"Infinity"`、`"-Infinity"` 认定为非有限数标记；真正的 JavaScript `NaN` 或 `Infinity` 数值不会命中。
- `isEmptyStructUnion(items)`（L102-L107）：严格要求两个分支，且分别存在无 `properties` 的 object 与无 `items` 的 array；多余字段、分支数量变化或字段已定义都不会折叠。
- `canFlattenAllOf(allOf, parent)`（L110-L119)：建立父对象除 `allOf` 外的键集合，逐项检查每个子对象键；任何与父键或前一个子对象键重复都会返回 false，同时把新键加入集合。它只检查键冲突，不判断语义上的约束是否可合并。
- `inlineLocalReferences(value, definitions?, seen = new Set())`（L121-L144)：数组递归，非 record 原样返回（L122-L124）。definitions 默认取当前对象的 `$defs`，因此根对象可建立本地定义表并传给后代（L125）。遇到 `$ref` 字符串时，支持 `#/\$defs/name` 与 `#/definitions/name` 两种前缀（L126-L129）；名称未在 `seen` 中且目标存在时，去掉当前 `$ref`，将目标 record 字段与当前引用对象其余字段合并，且引用处字段覆盖目标字段，再以复制后的 `seen + name` 递归（L130-L136）。缺失目标或检测到循环时保留原引用，防止无限递归。普通对象的全部字段（包括 `$defs` 本身）继续递归（L141-L143）。
- `dropDefinitionsIfResolved(value)`（L146-L150)：只有根值为 record 且 `hasLocalReference(value)` 为假时才删除根层 `$defs` 与 `definitions`；若仍存在任一本地引用、值是数组或原始值，则原样返回。它不主动删除嵌套对象中的 definitions。
- `hasLocalReference(value)`（L152-L162)：数组使用 `some` 递归；record 发现以 `#/$defs/` 或 `#/definitions/` 开头的 `$ref` 即返回 true，否则扫描所有值。非 record 返回 false。它只识别本地引用，外部 URI 引用不阻止 definitions 清理。
- `export * as ToolJsonSchema from "./json-schema"`（L164）：以命名空间方式重新导出本模块，调用方可使用 `ToolJsonSchema.fromSchema` 和 `ToolJsonSchema.fromTool`。

并发语义上，所有函数都是同步纯计算，唯一共享状态是模块级 `WeakMap`。JavaScript 单线程执行模型下不会在一次调用中被异步打断；没有锁、原子操作、跨线程一致性或 cache eviction API。不同 `Schema.Top` 对象即使结构相同也不会共享缓存。

## 状态、取消、恢复与副作用

状态只有进程内 `WeakMap` 缓存；没有会话状态、重试计数、超时、取消 token、AbortSignal、恢复日志或持久化。`fromSchema` 的异常不会被转换为可恢复结果；调用方若重试，需自行再次调用。规范化只创建/返回 JSON 值，不写文件、不发网络请求、不调用工具，也不触发 approval。`fromTool` 对显式 `jsonSchema` 的旁路意味着调用方必须自行保证其 provider 兼容性。若未来放入可取消的 zenpi runtime，取消只能包住调用边界，不能中断当前同步递归；取消不应被解释为回滚，因为本文件没有外部副作用。

## 源内测试与行为判据

源文件及 `src/tool` 同目录未包含测试，写明：**源内未包含测试**。仓库独立测试 `test/tool/parameters.test.ts` 将 `fromSchema` 结果做快照，并验证 named child schema 被内联且没有 `$defs`（L35-L64）、必填 nullable 字段保留 null（L66-L70）、重复 `allOf` 约束不被错误扁平化（L72-L78）、裸 integer 获得 safe-integer 上下界（L80-L84）、带默认值的可选字段不暴露为 nullable（L86-L91）。可独立验证的判据是：同一 `Schema.Top` 两次返回同一缓存结果；缺失目标或循环 `$ref` 保留引用；`additionalProperties: true` 被删除而 `false` 保留；`fromTool` 的显式 `jsonSchema` 不发生 normalize；非法最终标量抛出指定 Error。

## zenpi Rust 映射

对照现有代码，zenpi 的 `src/tools.rs` 已有 `ToolDefinition { input_schema: serde_json::Value }`，并要求 schema 顶层 `type` 为 `object`、序列化大小受限；`ToolCall.arguments` 是 JSON 对象，执行前由工具注册表做受限 schema 校验。因此建议新增 `src/tool_schema.rs`（或在 `src/tools.rs` 内拆分 `schema` 子模块）：

- 类型落点：`pub type JsonSchema = serde_json::Value`；`SchemaError` 用 `thiserror` 表达非法结构/大小/循环；`from_schema(value: &Value) -> Result<Value, SchemaError>`、`from_tool(def: &ToolDefinition) -> Result<Value, SchemaError>`；内部实现 `normalize`、`inline_local_references`、`drop_definitions_if_resolved`、`has_local_reference`。
- `src/tools.rs` 的 `ToolDefinition::input_schema` 是最接近 `Tool.Def.jsonSchema` 的现有字段；`from_tool` 应先返回它，只有 zenpi 未来引入 typed schema 描述时才走转换。`validate_call`/`validate_hook_call` 仍是权威执行校验，不应把展示 schema 当成安全授权。
- `src/protocols/chat.rs`、`src/protocols/responses.rs`、`src/protocols/anthropic.rs`、`src/protocols/google.rs` 当前分别把 `input_schema` 放入 `parameters`、`input_schema` 或 `parametersJsonSchema`；规范化应在这些编码器之前完成，避免各 Provider 产生不同的 `$ref`/nullable 形状。`src/providers/**` 负责路由、能力和认证，不应承载通用 schema 递归逻辑。
- `src/protocol.rs` 只负责 JSONL 边界、反序列化和大小约束；可在工具定义进入协议前调用 `from_tool`，并复用现有 object/size 检查。`src/headless.rs` 只拥有 wire I/O，适合记录 schema 错误为协议错误，但不应修改转换算法。
- `src/core.rs` 管理 Agent、tool call 与结果；建议在构造 provider `CompletionRequest` 的位置调用一次转换并传递不可变 `Value`。`src/tool_runtime.rs` 负责批量执行、顺序/并行和取消，schema 转换应位于执行之前，不应在线程 worker 内重复生成。
- `src/runtime.rs` 的 `CancellationToken` 是合作式取消；本转换同步且无安全中断点，最多在调用前后检查 token。`src/session.rs` 的 append-only journal 可记录 schema 版本/摘要用于审计，但当前 TypeScript 实现没有持久化，不能凭空恢复 cache。`src/approval.rs` 的 side-effect approval 与 schema 无关，schema 变化不能自动放行或拒绝工具。

差异清单：TypeScript 由 Effect `Schema.Top` 生成 JSON Schema，zenpi 目前只有预构造 `serde_json::Value`；TypeScript `WeakMap` 按对象身份且由 GC 回收，Rust 需要明确缓存键（建议规范化字节的 SHA-256）和有界 LRU，避免无界内存；TypeScript 先允许 `additionalProperties: true` 再删除，zenpi 现有定义常直接指定约束；TypeScript 支持本地 `$defs`/`definitions` 内联、nullable 清理、非有限数枚举折叠、空结构联合和 integer safe bounds，Rust 当前没有等价层；TypeScript 最终只做“布尔或对象”形状检查，zenpi 还要求顶层 object 与大小上限，迁移时应保留更严格的 wire 边界；Provider-specific dialect 变换仍应留在 `src/protocols/**`。

## 未决问题

无法从本文件确认 `Schema.toJsonSchemaDocument` 对所有 Effect schema 的具体输出、`JSONSchema7` 类型为何承载 Draft 2020-12 `$schema` 字符串，以及显式 `tool.jsonSchema` 是否已经经过同样规范化；这些行为需要查阅 `effect`、`@ai-sdk/provider` 和工具注册流程的版本实现。除此之外，本文件内的递归规则、错误文本和无副作用边界均已由 `L1-L164` 确认。

# OC-049 — packages/opencode/src/tool/schema.ts

## 元信息块

- source_id/item_id：opencode_runtime / OC-049
- source_path：packages/opencode/src/tool/schema.ts
- source_hash：8bce49e6fbabc75b96c54d8a394287df2f3ea2513dd957f4bd34140449b01905
- source_bytes：444
- source_lines：14
- coverage：已从第 1 字节连续读到第 443 字节（共 444 字节），覆盖源文件行 L1-L14；下文引用均对应本次完整读取。

## 完整行为复盘

1. **依赖导入（L1-L4）**  
   Schema 来自 effect，负责运行时字符串 schema、检查器与 brand；Identifier 来自 @/id/id，提供带前缀的 ID 生成；statics 来自 @opencode-ai/core/schema，把静态方法拷贝到 schema 对象。该文件没有 IO、异步函数或状态容器。

2. **toolIdSchema（L6-L6）**  
   Schema.String.check(Schema.isStartsWith("tool")) 先限定输入为字符串，再增加“以 tool 开头”的谓词；pipe(Schema.brand("ToolID")) 增加名义品牌。因此运行时值仍是普通字符串，品牌主要约束 TypeScript 类型。边界是“前缀”而非完整格式：tool、tool_...、toolbox 都满足 starts-with 检查；空串和其他类型不满足。此常量未导出，只通过后面的导出 schema 暴露。未设置默认值，也没有截断、长度或字符集检查（L6-L6）。

3. **类型导出 ToolID（L8-L8）**  
   export type ToolID = typeof toolIdSchema.Type 导出 schema 的静态类型，即带 ToolID brand 的字符串。它只在编译期存在，不产生运行时代码，不能单独执行校验；运行时校验仍由 ToolID schema 完成。输入输出均为类型层面的约束，没有异常或副作用（L8-L8）。

4. **值导出 ToolID 及其 ascending 静态函数（L10-L14）**  
   export const ToolID = toolIdSchema.pipe(statics(...)) 返回原 schema，并通过 statics 的 Object.assign 追加 ascending 方法（L10-L14）。这是值与类型同名的 TypeScript 配对导出：调用方可把 ToolID 当 schema，也可调用 ToolID.ascending。
   - 签名 ascending: (id?: string)（L12-L12）：无参数或 undefined 时委托 Identifier.ascending("tool", undefined) 生成新 ID；返回值再经 schema.make 构造成 ToolID，输出是符合 tool 前缀谓词的品牌字符串。
   - 给定非空 id 时，Identifier.ascending 先要求 id.startsWith("tool")；不满足会同步抛出 Error("ID ... does not start with tool")（依赖实现 id.ts L22-L39）。满足时原样返回，不会规范化、补下划线或重新生成。特别地，空字符串是 falsy，会走“生成新 ID”分支，而不是作为无效给定值报错。
   - 生成分支使用 Identifier 的 ascending 算法（依赖实现 id.ts L51-L70）：tool_ 加 6 字节时间/计数编码（12 个十六进制字符）和 14 个 Base62 随机字符，总后缀 26 字符；同一模块实例内时间相同的连续调用通过计数器区分，随机尾缀降低碰撞。schema.make 若遇到不满足 schema 的结果会走 Effect Schema 的构造失败路径；正常委托生成结果已满足 tool 前缀。
   - 调用是同步、无 Promise、无重试和无取消点。statics 本身只在模块初始化时把方法挂到 schema（依赖实现 schema.ts L20-L23），不会复制或持久化数据。

5. **模块整体边界（L1-L14）**  
   文件只定义一个 ID schema、一个对应类型和一个生成入口；没有工具注册、工具执行、参数 schema、网络请求或错误类型封装。错误主要来自前缀校验（Identifier.ascending）或 Effect Schema 构造/解码，而不是本文件自定义错误。

## 状态、取消、恢复与副作用

- 本文件没有取消、超时、AbortSignal、重试或恢复逻辑；ascending 一次调用即同步返回或抛错。
- 唯一可观察状态在依赖 Identifier 模块的进程内 lastTimestamp/counter（依赖实现 id.ts L18-L23、L54-L60）。它只影响后续生成 ID，不写磁盘、数据库、session 或网络；随机字节来自 crypto.randomBytes。
- 同一 Node.js isolate 的同步调用按事件循环顺序执行，计数器不会在单次调用中并发交错；不同 worker/isolate 各自拥有模块状态，不能把计数器顺序当作跨进程全局序列。没有锁、事务或持久化去保证跨进程唯一性/排序。
- 传入已有 ID 时没有外部副作用，只做前缀判断并原样返回；toolbox 这类仅有 tool 前缀的值也会被接受，格式完整性由调用方承担。

## 源内测试与行为判据

源文件及其同目录 tool 源码中未包含针对该 schema 的测试，故为“源内未包含测试”。可独立验证的判据：
1. ToolID schema 对字符串 tool_x、tool、toolbox 通过，对 x_tool、空串及非字符串拒绝。
2. ToolID.ascending() 返回字符串，且每次结果以 tool 开头；依赖当前生成器时通常形如 tool_<12 hex><14 Base62>。
3. ToolID.ascending("tool_custom") 原样返回；ToolID.ascending("bad") 同步抛出前缀错误；ToolID.ascending("") 生成新 ID。
4. 在同一毫秒连续生成多个值，时间编码相同而计数/随机部分不同；不能要求跨进程保持顺序。
5. 可用 Effect Schema 的 decode/make API 实测无效值的具体错误对象，以确认版本相关的异常类型。

## zenpi Rust 映射

- **建议落点**：新增 src/tool_id.rs（并在 lib.rs 导出，或把小型类型放入现有 src/tools.rs）；定义 pub struct ToolId(String)，实现 TryFrom<&str>/FromStr、Serialize/Deserialize、Display，校验 starts_with("tool")。提供 pub fn ascending(given: Option<&str>) -> Result<Self, ToolIdError>；对 None 或空串生成新值，对非空错误前缀返回结构化错误。
- **src/tool_runtime.rs 对照**：现有 ToolCall { id: String, ... }（src/tools.rs）和 execute_tool_batch 已做调用 ID 重复检查、取消传播与副作用串行化。可把 ToolCall.id 保持 wire 字符串，在入口用 ToolId::try_from 做格式校验；不要在执行器内重新生成 ID，避免与批次关联/持久化错位。
- **src/core.rs 对照**：核心已有工具调用准备、UnknownToolOutcome 和操作证据。生成 ToolId 应在创建一次工具调用/操作时完成，并沿准备、审批、结果记录链路传递；失败应成为 admission/validation 错误，不能静默改写调用方 ID。
- **src/session.rs 对照**：session 的 generate_id("session") 是 prefix-ms-pid-sequence 格式（L2236-L2247），且 append-only journal 持久化操作结果。它与 tool_ + 时间编码格式不同；若需要恢复工具调用，应把 ToolId 作为记录字段并保留原值，不能复用 session ID 生成器或把重放当作重新生成。
- **src/runtime.rs 对照**：JobId(u64) 是运行时队列相关的单调数字 ID，取消是 cooperative token。它不应替代 ToolId；二者应建立显式映射（JobId 关联执行任务，ToolId 关联模型工具调用）。
- **src/protocol.rs 对照**：协议已有 MAX_ID_BYTES 与 JSONL 相关 ID 字段。增加工具 ID 校验时应复用长度/控制字符边界，再叠加 tool 前缀；注意源 TS 不限制长度，因此若 Rust 增加上限属于 zenpi 的安全策略差异，需在协议文档和判据中明确。
- **src/approval.rs 对照**：ApprovalRequest 的 request_id/call_id 是通用字符串并有独立校验、取消等待和持久化前审计。可让 call_id 携带 ToolId 的序列化值，但 approval ID 不应强制 tool 前缀，因为审批请求也覆盖非工具流程；执行副作用前仍按现有 coordinator 记录 decision。
- **src/providers/** 对照：provider 路由、模型 ID 和协议能力（src/providers/connection.rs、registry.rs 及各 provider 定义）不负责工具调用 ID 生成。将 ToolId 保留在 core/tool 边界，wire encoder 仅按各 provider 协议转换；禁止把 provider/model ID 校验混入 ToolID。
- **关键差异与可验证实现**：TS 的 brand 是编译期名义类型，Rust newtype 可在反序列化时强制验证；TS 的 Identifier.ascending 使用毫秒时间+计数+随机 Base62，zenpi 当前 session ID 使用进程号/序列且带连字符。实现后应添加单元测试覆盖前缀边界、空串语义、原样透传、重复生成、serde round-trip，并在 tool_runtime 集成测试中验证无效 ID 在任何工具副作用前被拒绝。

## 未决问题

- 无法仅由该源文件确认 Effect Schema 当前版本对 schema.make 失败时抛出的具体错误类/消息格式；需按锁定依赖版本运行判据 5。
- 源文件没有规定生成 ID 的全局唯一性、跨进程排序或持久化要求；这些只能由调用方和 Identifier 实现推断，不能视为 ToolID schema 的保证。


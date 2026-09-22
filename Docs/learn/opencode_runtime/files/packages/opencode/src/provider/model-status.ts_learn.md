# OC-008 — packages/opencode/src/provider/model-status.ts

- source_id/item_id: `OC-008`
- source_path: `packages/opencode/src/provider/model-status.ts`
- source_hash: `7e99e64d54d69505e3d8f46051adf56a991a63e32fd2ec4acac53fc197218f66`
- source_bytes: `291`
- source_lines: `8`
- coverage: 已从首字节至末字节完整读取，字节范围 `0-290`（共 291 字节）；行范围 `L1-L8`（含空行、注释、类型和全部导出符号）。

## 完整行为复盘

该文件没有函数、可变对象或控制流，职责是给 provider 层建立一个运行时可解码、编译期可推导、并可被命名空间重新导出的模型生命周期状态定义。

1. `import { Schema } from "effect"`（`L1-L1`）。仅引入 `effect` 的 `Schema` 构造器，后续用它建立运行时 schema；没有本地配置、IO 或初始化副作用。
2. `export { CatalogModelStatus } from "@opencode-ai/core/models-dev"`（`L3-L3`）。这是原样转导出，状态集合的实现和边界由外部模块决定，当前文件不包装、不扩展、不提供默认值。仓库对应实现把 catalog 集合定义为 `"alpha" | "beta" | "deprecated"`；因此它与本文件随后定义的规范化 provider 集合有意不同。
3. `export const ModelStatus = Schema.Literals(["alpha", "beta", "deprecated", "active"])`（`L5-L5`）。构造一个 Effect `Schema` 值，运行时输入只有四个精确字符串可通过：`alpha`、`beta`、`deprecated`、`active`。成功解码的结果仍是同一个字符串字面量；数组之外的字符串、数字、空值、对象等都应被 schema 拒绝。源码没有 `Schema.optional`、转换器或 fallback，所以不存在缺省状态或隐式归一化；值的顺序也不表现为优先级。
4. `export type ModelStatus = typeof ModelStatus.Type`（`L6-L6`）。从上面的 schema 提取静态 TypeScript 类型，得到四个字面量的联合类型。它只影响编译期，不产生新的运行时值，也不会改变 `Schema.decodeUnknown` 等校验行为；同名的值导出与类型导出在 TypeScript 的不同命名空间中并存。
5. `export * as ProviderModelStatus from "./model-status"`（`L8-L8`）。建立模块命名空间导出，消费者可通过 `ProviderModelStatus.ModelStatus`、`ProviderModelStatus.CatalogModelStatus`（以及对应类型）访问本模块公开的符号。它不是第二份 schema，不复制状态集合，也没有额外执行路径；模块装载仍只发生一次。

边界与错误路径：唯一可观察的拒绝路径是 `ModelStatus` 或转导出的 `CatalogModelStatus` 在 Effect 解码时抛出/返回错误。文件本身没有 `try/catch`、错误码、日志或重试。并发语义是只读共享：schema 常量和类型别名在定义后不变，多个调用方可并发解码；源码没有锁、队列、取消令牌或顺序保证。

## 状态、取消、恢复与副作用

这里的“状态”只是四个生命周期标签，不是请求运行状态机。`active` 与 `deprecated` 等值不会在本文件内自动迁移，任何转移规则必须由 provider/catalog 消费者实现。没有超时、取消、重试、恢复、持久化、网络访问、文件写入、环境变量读取或外部副作用。schema 解码失败也不会改变全局状态。`ProviderModelStatus` 仅提供导出组织，不引入缓存或注册表。

## 源内测试与行为判据

对应测试文件 `packages/opencode/test/provider/model-status.test.ts`：

- `L8-L13` 的测试验证 catalog 与规范化集合分离：`CatalogModelStatus` 接受 `"deprecated"`（`L10-L10`），拒绝 `"active"`（`L11-L11`），而 `ModelStatus` 接受 `"active"`（`L12-L12`）。这直接证明四值集合不是对 catalog 三值集合的简单别名。
- `L15-L60` 验证公共 provider schema 能承载 `status: "active"`：`ConfigProviderV1.Model` 在 `L16-L17` 保留该值；`Provider.Model` 在 `L30-L59` 保留该值；`ModelsDev.Model` 示例在 `L18-L28` 未提供 status，因此结果为 `undefined`，说明 catalog 模型字段可选并不改变本文件 schema 的四值约束。

可独立验证的判据：使用 Effect 的 `Schema.decodeUnknownSync(ModelStatus)`，四个字符串均应成功并原样返回；`""`、`"unknown"`、`null`、`1` 应抛出解码错误。对 `CatalogModelStatus`，`"alpha"`、`"beta"`、`"deprecated"` 应成功，`"active"` 应失败。再检查 `ProviderModelStatus.ModelStatus === ModelStatus`，确认命名空间导出没有复制实例。

## zenpi Rust 映射

最接近的落点是 `src/providers/registry.rs`：现有 `ModelDescriptor`（`L58-L68`）承载 `(provider, id)`、能力、上下文预算、价格和来源；`ModelRegistry::resolve`（`L366-L373`）按精确身份返回描述符，未知模型走 `unknown` 保守描述（`L384-L429`）。建议在该模块新增：

- `#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)] #[serde(rename_all = "lowercase")] pub enum ModelStatus { Alpha, Beta, Deprecated, Active }`，并在 `ModelDescriptor` 增加 `status: ModelStatus`。不要用 `Default` 静默填充，除非产品明确规定新模型状态；这样才能保留 TS schema“无默认值”的边界。
- 另建三值 `CatalogModelStatus { Alpha, Beta, Deprecated }`，用于外部 catalog 解码，再显式映射到规范化 `ModelStatus`；不要把 `CatalogModelStatus` 与 `ModelStatus` 合并，否则会重新允许 catalog 直接声明 `active`。
- 在 `ModelRegistry::with_overrides`（`L291-L363`）为 override 的 status 做严格 serde 解码与字段校验，并把 status 纳入 `ModelDescriptor::digest`（`L112-L118`），保证已选模型的元数据变化能被发现。

现有对照和差异清单：

- `src/core.rs` 的 `Agent::model_status`（`L1284-L1291`）目前返回 `active` 描述符、能力、预算、catalog、`registry_bound` 和 reasoning effort；建议把 `ModelDescriptor.status` 序列化进该 JSON，而不要把生命周期状态混同 `AgentPhase` 或运行请求状态。`restore_model_selection` 的 digest 校验（`L1262-L1274`）可验证状态变更是否需要显式重新选择。
- `src/headless.rs` 的异步请求/事件缓冲（`L3912-L3935`）以及 provider 事件投影（`L4339-L4365`）处理的是流式工作；模型生命周期状态应作为 status 快照字段，不应伪装成 `ProviderEvent`。headless 的 `status` 命令解析在 `src/protocol.rs:L415-L424`，可复用来返回快照。
- `src/protocol.rs` 的 `Command::Status` 是传输命令，不是模型状态枚举；若对外发送 status，给响应增加可序列化的 `model_status` 字段，并对未知字符串返回协议错误。协议已有有界输入，不能用它替代 schema 级严格枚举。
- `src/session.rs` 的 `OperationKind`/`OperationOutcome`（`L49-L65`）描述 provider、tool、compaction 操作的成功/失败/取消/中断；这些不是 `alpha/beta/deprecated/active`，不能复用。若要恢复模型元数据，应将 status 纳入选择事件或描述符 digest，而非记录成 operation outcome。
- `src/runtime.rs` 的 `JobOutcome` 包含 `Succeeded/Failed/Cancelled/Panicked`，并由 `CancellationToken` 协作取消；它是作业调度语义，与本文件无取消语义。模型 status 读取应保持纯函数，不因 job cancel 改写。
- `src/tool_runtime.rs` 的 tool batch 有并行/串行、批量上限和取消轮询；模型状态校验不应放进 tool 执行或 approval 流程，避免把生命周期标签误当副作用授权。
- `src/approval.rs` 的 `ApprovalDecision` 是 `Allow/Deny`，与模型状态集合完全不同；不得把 `active` 映射为 `Allow`，也不得让 `deprecated` 自动触发审批。
- `src/providers/mod.rs` 的 `ProviderDefinition`/`RouteRule`（约 `L146-L180`）以及 `openai.rs`、`anthropic.rs`、`deepseek.rs`、`codex.rs`、`google.rs` 中的静态 provider 路由主要声明协议、鉴权和能力；建议保持 provider 路由无状态，把模型 status 放在 `registry::ModelDescriptor`。`connection.rs` 只负责连接与能力交集，不能替代生命周期字段；`registry.rs` 才是 catalog/override 的集中验证点。

建议的可验证实现步骤是：先给两个 Rust enum 加 serde 单元测试，覆盖四值/三值边界和未知值；再让 `model_status()` 返回 status；最后测试修改 status 后 descriptor digest 改变、旧 session 恢复被拒绝并要求显式选择。这样分别覆盖运行时解码、公共 JSON 投影和持久化恢复三层行为。

## 未决问题

- `model-status.ts` 只转导出 `CatalogModelStatus`，无法单独确认 catalog 状态的业务转移规则，以及何时允许 `deprecated` 模型继续发送请求。
- 无法从本文件确认 `active` 是否代表“可选”、 “已验证”或仅表示 provider 侧规范化标签；zenpi 映射应把该语义留给上层策略配置。
- 本文件没有持久化协议，无法确认状态是否应写入 session；只能依据 zenpi 现有 descriptor digest 机制提出可验证落点。

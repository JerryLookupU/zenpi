# OC-004 — packages/opencode/src/permission/evaluate.ts

- source_id/item_id：`OC-004`
- source_path：`packages/opencode/src/permission/evaluate.ts`
- source_hash：`0ee7b7fea6766c57ce0583bd6f23a303c5af314c0e5287018fa6bdfdc38e5a6e`
- source_bytes：`29`
- source_lines：`1`
- coverage：已按顺序读取完整文件，字节偏移 `0-28`（共 29 字节），行范围 `L1-L1`；包含该行全部导出语法，无注释、类型声明或其他隐藏行。

## 完整行为复盘

本文件只有一个导出声明：`export { evaluate } from "."`（`L1-L1`）。它没有重新实现函数，而是把当前 `permission` 目录的入口模块（目录解析到 `index.ts`）中的 `evaluate` 原样转出。因此从本文件能够确定的直接行为是“提供稳定的 `permission/evaluate` 导入路径，并保持符号身份与目录入口一致”；输入、返回值、匹配算法和错误行为均由被转发的 `evaluate` 决定，本文件没有包装、转换或默认参数。

为完成可核对的 1:1 复盘，核对了同目录入口的实现：`index.ts` 在 `L28-L38` 声明 `evaluate(permission: string, pattern: string, ...rulesets: PermissionV1.Ruleset[]): PermissionV1.Rule`。它先对可变参数 `rulesets` 做一层 `flat()`，再按数组原顺序用 `findLast` 找到最后一个同时满足 `Wildcard.match(permission, rule.permission)` 与 `Wildcard.match(pattern, rule.pattern)` 的规则；找到时返回原规则对象（包括其 `action`、`permission`、`pattern`），没有命中时返回 `{ action: "ask", permission, pattern: "*" }`。因此规则优先级是“最后匹配者胜出”，不是按通配符具体程度自动排序；空规则集、未知 permission、无匹配 pattern 都会走 `ask` 默认值。多个 ruleset 的边界是扁平化后共享同一顺序，后一个 ruleset 的规则可覆盖前一个（`index.ts L28-L38`）。

该符号的输入在 TypeScript 类型层面是两个字符串和零个或多个 `PermissionV1.Ruleset`；没有显式运行时参数校验、空字符串特判或异常映射。正常路径是同步返回 `PermissionV1.Rule`，没有 `Promise`、`Effect` 或迭代器。若调用方在运行时传入违反类型约束的数据，源文件没有定义其结果；任何异常只能来自目录入口实现或 `Wildcard.match`，不能归因于此 re-export 文件。调用方可直接读取 `.action`，上层 permission service 以 `allow`、`deny`、`ask` 分支处理（入口 `index.ts L67-L84`）。

并发语义上，`evaluate` 是同步、无内部共享状态的计算；每次调用只读取传入 rulesets，并返回命中规则引用或新建默认对象。该文件不加锁、不排队、不启动任务，也不保证调用方并发修改数组时的结果；在调用期间保持输入不变时，可由多个线程/任务独立调用。`evaluate.ts` 本身没有导出的类型、类、常量或其他函数，唯一导出符号就是上述 `evaluate`（`L1-L1`）。

## 状态、取消、恢复与副作用

本文件没有局部状态、缓存、持久化、网络、文件、日志或外部进程副作用。它没有取消令牌、超时参数、重试循环或恢复分支；调用一旦开始就完成一次同步匹配。模块加载时，JavaScript 的目录导入机制可能执行 `index.ts` 的顶层导入，但 `evaluate.ts` 没有额外初始化逻辑，也不会创建 permission service 状态。上层 service 的 pending 请求、`Deferred`、事件发布和拒绝清理属于 `index.ts L42-L175`，不应误记为本文件的副作用。

由于规则计算不写入状态，取消只能由调用方在调用前后自行处理；无法中断一次已经开始的 `findLast`。重试也只能由上层重新调用，并且本函数没有幂等键或重试计数。若上层把命中规则用于审批或工具执行，审批、会话日志和执行副作用必须由上层负责，不能从 `L1-L1` 推导出本文件会自动允许、拒绝或记录任何动作。

## 源内测试与行为判据

源文件自身仅有 `L1-L1` 的 re-export，`permission` 源目录未包含测试，**源内未包含测试**。可独立验证的判据如下：

1. 构建后从 `permission/evaluate` 导入的 `evaluate` 应与从 `permission` 入口导入的同名符号可调用且行为一致；不能出现包装层改变对象身份或参数顺序。
2. 使用 `[{ permission: "bash", pattern: "*", action: "allow" }, { permission: "bash", pattern: "rm", action: "deny" }]` 评估 `("bash", "rm")` 应返回 `deny`；反转两条规则应返回 `allow`，验证最后匹配优先。包级测试已明确覆盖该判据（`test/permission/next.test.ts L290-L304`）。
3. 空 ruleset 或无匹配规则应返回 `action === "ask"`（`test/permission/next.test.ts L327-L342`）；`permission: "*"`、`pattern: "*"` 的规则应参与匹配（`L383-L399`）。
4. 两个 ruleset 合并时，后传入的规则可以覆盖前者（`test/permission/next.test.ts L443-L448`）。

## zenpi Rust 映射

`zenpi` 当前的审批核心在 `src/approval.rs`：`ApprovalMode`、`ApprovalRequest`、`ApprovalDecision` 和 `ApprovalCoordinator` 位于 `L22-L175`，强调显式 host 决策、请求校验、条件变量等待与取消；它比该 TypeScript 纯规则查找多了会话关联、side effect 和持久化门槛。建议在 `src/approval.rs` 增加纯数据类型 `PermissionRule { permission: String, pattern: String, action: PermissionAction }` 与 `evaluate_permission(permission, pattern, rulesets)`，让函数只做扁平化、通配匹配、最后命中和 `Ask` 默认，返回 `&PermissionRule` 或拥有值的默认规则；为跨线程安全，调用期间借用不可变 slice，不共享可变全局状态。

`src/core.rs` 的 `Agent`/`ToolRuntime` 已持有 `ApprovalCoordinator` 与 `ApprovalPolicy`（`L397-L457`），适合作为集成点：在工具准备阶段先调用纯 evaluator，再把 `allow/deny/ask` 映射到现有审批流程；不要把 `evaluate` 直接放进 provider 请求发送路径。`src/tool_runtime.rs` 的 `execute_tool_batch` 在 `L157-L175`、`L349-L438` 处理 prepare、审批复核、并发和取消，建议在 `prepare` 回调中执行 evaluator，并保留“每个调用一个结果、策略失败不扩大副作用”的现有语义。

`src/runtime.rs` 的 `CancellationToken` 在 `L49-L100` 是合作式取消并带完成竞态保护；它与纯 evaluator 正交，不应为匹配函数增加异步取消参数。`src/session.rs` 的操作恢复模型在 `L51-L98`，把失败、取消、未知结果和“重试需确认”持久化；若 zenpi 要支持 `remember/always` 规则，应在 session 事件中持久化规则或审批决定，而不是让 evaluator 自己写 WAL。`src/headless.rs` 的重连日志和有界重试位于 `L155-L175`，可负责协议层重放，但不要把连接重试混入规则匹配。

`src/protocol.rs` 已在 `L156-L205` 定义 `StdioRequest`，包含 `approval_id`、`decision`、`remember`，并在 `L214-L259` 建模命令；可执行的映射是增加可选 permission/pattern/rules 字段及严格反序列化，再在命令校验后调用 `evaluate_permission`。`src/providers/**` 目前由 `src/providers/mod.rs L142-L157` 的 `ProviderDefinition`、路由和能力描述组成，各 provider 文件只返回定义；它们不应承载通用 permission 规则，最多提供工具能力或 side-effect 元数据供 `ApprovalRequest` 使用。

差异清单：TypeScript 的 `rulesets` 是可变参数并一层 `flat()`，Rust API 需明确 `&[&[PermissionRule]]` 或先构造有序 `Vec`；TypeScript 使用 `Wildcard.match`，Rust 必须选定并测试等价 glob 语义；TypeScript 默认返回 `ask` 规则对象，Rust 需定义拥有型默认值避免悬垂引用；TypeScript 的最后匹配规则覆盖先前规则，现有 Rust `ApprovalDecision` 只有 `Allow/Deny`（`approval.rs L109-L120`），需要额外的 `Ask` 中间态；取消、重试、持久化和 provider 路由均属于 zenpi 上层，不应伪装成 evaluator 的行为。

## 未决问题

1. 仅凭 `evaluate.ts` 无法确认目录导入 `"."` 在所有构建器/路径别名下都解析到哪个入口；本笔记按当前同目录 `index.ts` 核对。
2. `Wildcard.match` 的转义、路径分隔符和异常处理细节不在本文件中；若 Rust 要字节级兼容，需补充跨语言 glob 对照测试。
3. `PermissionV1.Rule` 允许的 `action` 具体联合类型及规则对象是否可被调用方后续变更，需以其定义文件为准。

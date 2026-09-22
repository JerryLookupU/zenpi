# OC-003 — packages/opencode/src/permission/arity.ts

- source_id: OC-003
- item_id: OC-003
- source_path: packages/opencode/src/permission/arity.ts
- source_hash: 4200d3074fa8b6b6a63100fa93deb201be16f92c655721401d47c2ca0a109832
- source_bytes: 6376
- source_lines: 163
- coverage: 已按源文件顺序读取完整字节范围 `1-6376`、行范围 `L1-L163`（含全部注释、类型、常量和导出符号）。

## 完整行为复盘

`prefix(tokens: string[])` 是文件唯一的函数导出，定义在 `L1-L9`。它接收已经分词的命令 token 数组，不负责 shell 解析、引号处理、变量展开或去除 flags。函数从 `tokens.length` 向下递减 `len`，每轮将 `tokens.slice(0, len).join(" ")` 作为精确键查询 `ARITY`（`L2-L5`）。因此匹配优先级是最长 token 前缀优先；它不依赖对象枚举顺序，也不会比较模糊前缀。命中后返回 `tokens.slice(0, arity)`（`L4-L5`），原数组不被修改，返回的是新数组。若 `tokens` 为空，循环不进入并在 `L7` 返回 `[]`；若没有任何键命中且非空，则在 `L8` 返回首 token。未知命令因此采用 arity 1 的保守默认值。

`ARITY: Record<string, number>` 是 `L24-L161` 的模块私有字典。其普通命令（如 `cat`、`git`、`npm`、`docker`）和嵌套子命令（如 `"docker compose"`、`"npm run"`、`"aws"`）分别给出 1、2 或 3 等 token 数。具体而言，文件注释规定 flags 不计入命令 token、最长匹配获胜，并要求只有当更长前缀改变 arity 时才登记（`L11-L22`）；这些是字典生成约束，运行时实现本身只看传入数组。字典值目前为正整数；若未来出现 `0`，`slice(0, 0)` 会返回空数组；若值大于输入长度，JavaScript `slice` 会自然截断为现有 token 数，文件没有额外错误校验。键大小写、空白和 token 内容均按字符串精确匹配，故调用者需负责 shell 类型和规范化。

`export * as BashArity from "./arity"` 位于 `L163`，提供命名空间导出；同目录测试通过 `import { BashArity }` 使用 `BashArity.prefix`。没有其他公开字典或类型导出。模块加载时仅创建静态对象，没有 I/O、进程启动、日志或异常分支；正常输入不会抛错。时间复杂度约为 `O(n^2)`（最多 n 次 `slice/join`，其中 n 为 token 数），空间复杂度为 `O(n)` 的临时前缀数组和返回数组；通常命令 token 数很小。

调用关系上，`src/tool/shell.ts` 在 `L392-L410` 将语法树命令转换为 `tokens`，再把 `BashArity.prefix(tokens).join(" ") + " *"` 放入 `scan.always`，用于权限/路径扫描规则。因此返回值代表“人可理解的命令前缀”，不是要直接执行的完整命令，也不是安全授权本身。

## 状态、取消、恢复与副作用

该文件的 `prefix` 是同步、纯函数：无共享可变状态、锁、Promise、线程、并发队列或重入协议；并发调用只读取模块字典，各调用之间没有状态传递。没有取消、超时、重试、恢复、持久化、网络访问、文件写入或子进程副作用。命令失败、审批拒绝、取消和超时都不在此层表达，未知命令只回退到首 token。

在 zenpi 中，等价能力应放在命令“分类/审计”层，而不是把 arity 结果当作执行许可。`src/core.rs` 的 `run_user_shell_with_cancel`（约 `L2941-L3170`）负责 `!` 路由、审批、取消轮询、操作日志和执行前持久化；`src/headless.rs` 负责异步请求、审批事件排空和取消传播。`arity` 若接入这些路径，只能生成稳定的审计/匹配键，不能绕过 `ToolSideEffect::CommandExecution`、`ApprovalCoordinator` 或 session journal。

## 源内测试与行为判据

同目录测试 `/Users/wangweiyang/GitHub/opencode/packages/opencode/test/permission/arity.test.ts` 覆盖：未知命令和 arity 1（`L4-L7`）；`git checkout`、`docker run` 等 arity 2（`L9-L12`）；`aws s3 ls`、`npm run dev` 等 arity 3（`L14-L17`）；嵌套前缀最长匹配（`L19-L22`）；刚好达到字典长度（`L24-L27`）；空数组、单 token 和仅 `git`（`L29-L33`）。可独立验证的判据是：对每个测试输入调用 `BashArity.prefix`，输出必须逐元素等于断言数组，且输入数组在调用后保持不变。

测试未覆盖 flags 是否在上游被过滤、引号/空白分词、大小写、字典值异常、超长 token 数以及 `BashArity` 自引用导出的打包行为。可补充的独立判据是：`prefix([]) === []`；未知非空输入只保留首 token；`["docker","compose","up","x"]` 必须优先得到前三项；命中 arity 3 但输入只有两项时不得产生越界元素；对同一数组并发/重复调用结果相同。

## zenpi Rust 映射

- `src/tool_runtime.rs` 已有 `classify_master_session_input`（约 `L75-L110`），负责空输入、长度、控制字符和 `!`/steer 路由；建议新增纯函数 `command_prefix(tokens: &[&str]) -> Vec<String>` 或零拷贝的 `CommandPrefix<'a>`，并将 arity 表放在 `tool_runtime.rs` 同级的 `command_arity.rs`。该函数只生成审计键，不改变 `MasterSessionCommand` 的审批语义。
- `src/core.rs` 的 `run_user_shell_with_cancel` 是执行落点；在 `user_shell_input`/`tool_execution_started` 事件中可记录 `command_prefix`，但必须保留完整 `command`、`policy_digest` 和 `operation_id`。Rust 实现应继续在审批后、spawn 前持久化，不能用 prefix 结果自动放行。
- `src/headless.rs` 是协议适配和取消响应落点。可在构造 `UserShellRequest` 后分词并附加只读字段；异步审批排空、`emergency_cancel` 和取消错误路径保持现状，prefix 计算不得阻塞事件循环。
- `src/session.rs` 负责 WAL/事件持久化与恢复；若记录 prefix，应作为派生审计字段写入同一 operation 事件，恢复时只重放记录，不重新执行 shell。现有 session 的 owner、序列和权限约束不应被字典影响。
- `src/runtime.rs` 的 `BackgroundRunner` 提供有界命令/事件队列与协作取消；prefix 是短同步计算，宜在提交前或 worker 开始前完成，不需要新增线程。队列满、关闭、取消和 shutdown grace 的语义继续由 runtime 管理。
- `src/protocol.rs` 的 `parse_user_shell_input`（约 `L689-L707`）仅校验单个 `!`、长度和控制字符；它不应承担 shell tokenizer。若 wire 协议需要 prefix，应新增可选响应字段并限制大小，避免把未经解析的字符串误认为 token 数组。
- `src/approval.rs` 的 `ApprovalRequest`/`ApprovalCoordinator` 是授权与取消等待边界；prefix 只能作为 `tool = "user_shell"` 的预览或审计元数据，不能替代 `ApprovalDecision`、policy digest、lease 或 `persist_accepted`。
- `src/providers/**` 当前主要是 API endpoint/prefix 路由和连接校验（如 `providers/mod.rs` 的 `prefix_route`、`connection.rs` 的 URL path prefix），与 shell command arity 不同。不要复用 provider 的 URL prefix 类型；若共享命名，应使用独立 `CommandArityTable`，避免把 URL 段数和 shell token 数混淆。

建议的可执行验证：新增 Rust 单元测试覆盖未知命令、arity 1/2/3、最长嵌套前缀、空输入、短输入和输入不可变；在 headless 集成测试中确认 prefix 仅出现在审计/响应数据，审批拒绝与取消仍阻止 `RunCommandTool`；运行 `cargo test` 及现有 headless/session/approval 测试，并用一个含 flags 的真实 shell token 序列验证 tokenizer 与 arity 表职责分离。

## 未决问题

1. `arity.ts` 的注释要求“flags NEVER count as tokens”，但源文件没有 tokenizer，无法确认所有调用者都已一致过滤 flags。
2. 未从源确认 `ARITY` 是否会由生成流程定期更新，也无法确认字典覆盖范围是否与 zenpi 的 shell 支持矩阵一致。
3. 未从源确认 `export * as BashArity from "./arity"` 在所有构建器中的自引用导出是否有特殊打包限制；现有 Bun 测试已证明其导入路径可用。

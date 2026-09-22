# OC-051 — packages/opencode/src/tool/shell/id.ts

## 元信息

- source_id/item_id：`OC-051`
- source_path：`packages/opencode/src/tool/shell/id.ts`
- source_hash：`5e6dc20d9147234224af97a222784e1e2c55e17873fcc8425e9be39b7f0a2b91`
- source_bytes：`572`
- source_lines：`19`
- coverage：已读取完整字节范围 `0-571`（572 字节）与完整行范围 `L1-L19`，包括注释、类型声明、私有符号和导出符号。

## 完整行为复盘

该文件是 shell 工具的身份与 shell 类型归一化模块；没有 I/O、异步调用或命令执行。所有行为都在模块初始化和纯函数调用中完成。

- `const kinds = ["bash", "pwsh", "powershell", "cmd"] as const`（L1-L1）。`as const` 将数组元素收窄为四个字符串字面量，并形成只读元组语义。允许值的顺序固定为 `bash`、`pwsh`、`powershell`、`cmd`；没有空字符串、大小写变体或其他 shell 别名。该常量未导出，外部只能通过类型和函数间接使用。
- `export type Kind = (typeof kinds)[number]`（L2-L2）。导出的静态类型是联合类型 `"bash" | "pwsh" | "powershell" | "cmd"`。它只在编译期约束，不会在运行时验证字符串，也不改变输入值。
- `const shellKinds = new Set<string>(kinds)`（L4-L4）。模块加载时把四个字面量复制进集合；集合元素类型显式为 `string`。集合私有且没有后续写操作，因此实际用途是只读查找表。初始化不会抛出业务错误；模块装载失败只可能来自宿主运行时异常。
- `function isKind(value: string): value is Kind`（L6-L8）。函数以 `shellKinds.has(value)` 做精确、区分大小写的 O(1) 平均查找；不做 `trim`、路径解析、大小写转换或空值替换。返回值为布尔值，同时通过 TypeScript 类型谓词把成功分支中的 `value` 收窄为 `Kind`。输入类型已声明为 `string`，因此调用方若绕过类型系统传入其他值，运行时行为不在源文件契约内。
- `export function toKind(value: string): Kind`（L10-L12）。对已知四种字符串原样返回；对未知字符串统一返回默认值 `"bash"`。因此 `""`、`"BASH"`、`"sh"`、带空格的字符串和未来未加入集合的别名都不会报错，而会回退到 `bash`。函数总是返回 `Kind`，没有异常、`undefined`、重试或异步分支；其边界行为完全由 `isKind` 的精确匹配决定。
- 注释（L14-L15）说明兼容性约束：对外暴露的工具 ID 与权限键必须继续使用 `"bash"`，以兼容既有插件、用户配置和已保存权限；计划在 opencode 2.0 重命名。这是协议/持久化兼容要求，而非运行时分支。
- `export const ToolID = "bash"`（L16-L16）。运行时导出的常量值恒为字符串 `"bash"`，是工具注册和权限查找使用的稳定键；它与 `Kind` 中的 `"bash"` 相同，但不代表当前实际 shell 一定是 bash（`toKind` 可识别其他 shell）。
- `export type ToolID = typeof ToolID`（L17-L17）。导出类型是字面量类型 `"bash"`，所以 TypeScript 调用方不能把任意 `Kind` 当成工具 ID；shell 类型和工具身份被有意分离。
- `export * as ShellID from "./id"`（L19-L19）。导出自引用命名空间 `ShellID`，让其他模块可以用 `ShellID.ToolID`、`ShellID.toKind` 等形式访问同一模块的导出。它不创建第二份状态，也不递归执行代码；模块缓存保证初始化一次。

同目录调用可核对：`src/tool/shell.ts` 在 `src/tool/shell.ts:L16-L16` 导入 `ShellID`，`src/tool/shell.ts:L284-L284`、`src/tool/shell.ts:L339-L339` 用 `ShellID.ToolID` 作为权限键和 `ShellTool` 名称，`src/tool/shell.ts:L390-L390` 用 `ShellID.toKind(Shell.name(shell))` 识别解析器 shell，随后在 `src/tool/shell.ts:L395-L399` 对 `cmd` 和文件路径规则作分支。由此可见，未知宿主 shell 名称会安全地按本文件规则当作 `bash`，而不会改变稳定的工具 ID。

并发语义：没有显式共享可变状态、锁、Promise 或 worker。模块初始化后的 `shellKinds` 只读使用，因此并发调用 `isKind`/ `toKind` 是无副作用的；即使多个调用同时发生，也只能得到相同结果。集合本身理论上可变，但没有导出引用或写入路径，源内不存在竞态。

## 状态、取消、恢复与副作用

本文件不维护会话状态，也不产生外部副作用。没有取消令牌、超时、重试、持久化、子进程、文件、网络或权限写入。未知输入的“恢复”仅指 `toKind` 返回 `"bash"`，不是执行失败后的重试；调用方若随后执行 shell，审批和取消责任在上层。唯一可观察的初始化副作用是创建私有 `Set`（L4-L4），不会写入系统资源。

## 源内测试与行为判据

源文件及其同目录未包含针对 `id.ts` 的专门测试（检索到的引用在 `shell.ts`，未发现 `toKind`/ `ToolID` 的断言测试），因此源内未包含测试。可独立验证的判据如下：

1. `toKind("bash")`、`toKind("pwsh")`、`toKind("powershell")`、`toKind("cmd")` 必须分别原样返回。
2. `toKind("")`、`toKind("sh")`、`toKind("BASH")`、`toKind(" bash ")` 必须返回 `"bash"`。
3. `ToolID === "bash"` 且其静态类型只能赋值 `"bash"`。
4. 调用 `ShellID.toKind(Shell.name(shell))` 后，未知名称不得抛错；结果必须属于四项联合类型。
5. 并发重复调用不得改变结果或集合内容。

## zenpi Rust 映射

- `src/core.rs:L2941-L2941` 的 `Agent::run_user_shell_with_cancel` 是当前用户 shell 所有者：要求单个 `!` 前缀、检查 agent 空闲态、构造 `ToolOrigin::UserShell`，执行审批/策略门控，并在 `src/core.rs:L3125-L3125` 调用 `RunCommandTool::invoke_user_shell_with_cancel`。建议在 `src/tools.rs` 或新的 `src/shell_id.rs` 放置 `#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum ShellKind { Bash, Pwsh, Powershell, Cmd }`、`pub const TOOL_ID: &str = "bash"` 与 `pub fn to_kind(value: &str) -> ShellKind`；未知值返回 `ShellKind::Bash`，保留本文件的大小写敏感和不报错语义。
- `src/tool_runtime.rs:L37-L37` 的 `MasterSessionCommand::{Bash, Steer}` 与 `src/tool_runtime.rs:L83-L83` 的 `classify_master_session_input` 是输入分类层。它把 `!` 后文本交给现有用户 shell，并限制长度/控制字符；不要把该分类器误当成 shell 名称归一化。可让其调用 `shell_id::TOOL_ID` 生成稳定权限键。
- `src/protocol.rs:L689-L689` 的 `parse_user_shell_input` 负责线协议验证：要求 `!`、拒绝 `!!` 和非法控制字符。建议保持协议错误与 `to_kind` 的未知值回退分离：命令输入错误仍返回 `ProtocolError`，shell 名称未知才回退 `Bash`。
- `src/runtime.rs:L56-L91` 的 `CancellationToken` 及运行队列只负责作业取消/完成竞争；本文件映射函数应保持纯函数，不持有 token。上层在启动命令前后继续检查 `is_cancelled`，避免把 shell 类型解析和取消语义耦合。
- `src/approval.rs:L41-L41` 的 `ApprovalRequest` 与 `src/approval.rs:L185-L185` 的 `request_response` 为副作用审批边界；`core.rs:L3059-L3059` 的 `persist_accepted` 在执行前持久化批准。建议权限键继续使用字面量 `"bash"`/ `TOOL_ID`，不能用 `ShellKind::Cmd` 替换，否则会破坏已有权限和插件兼容性。
- `src/session.rs:L51-L76` 定义 `OperationKind`、`OperationOutcome`、`InterruptedOperation`；`src/session.rs:L1414-L1414`、`src/session.rs:L1453-L1453` 的 `begin_operation`/ `finish_operation` 支持命令前写入和结果收尾。该文件本身无需新增持久化记录；若 Rust 端增加 shell 类型字段，应只记录解析后的值和稳定 `TOOL_ID`，并保持未知值回退可重放。
- `src/headless.rs:L1569-L1579` 将 master console 的 `Bash` 转换成 `Command::UserShell`，`src/headless.rs:L4606-L4606` 等路径传入取消谓词。这里应调用统一 `to_kind`（若需要展示/分析实际 shell），但执行和审批仍走现有 user-shell owner，不能在 headless 层绕过它。
- `src/providers/**`（尤其 `src/providers/registry.rs:L143-L143` 的 `ModelRegistry` 与 `src/providers/registry.rs:L366-L366` 的 `resolve`）只管理 provider/model 身份、能力和价格，不应承载 `ShellKind` 或 `ToolID`。建议差异清单明确：opencode 的 shell ID 模块是工具层常量；zenpi 当前 provider registry 没有同等抽象，需要新增小型、无 I/O 的 shell identity 模块并由 `core/tools/headless` 复用。
- 可执行验证：新增 Rust 单元测试覆盖四个精确匹配、未知/大小写/空白回退、`TOOL_ID == "bash"`、并发只读调用；再运行现有 core/protocol/headless 测试，确认审批事件、`OperationOutcome` 和取消结果不变。

差异清单：TypeScript 使用字符串字面量联合与运行时 `Set`；Rust 需要显式 enum/匹配。TypeScript 的默认回退是静默 `"bash"`；Rust 不能用 `Result` 错误替代该默认。TypeScript 的 `ToolID` 是值和同名字面量类型；Rust 应用 `const TOOL_ID` 加类型化 `ShellKind`，避免把 enum 当权限键。TypeScript 通过命名空间重导出；Rust 用模块路径 `crate::shell_id::{...}`。两者都应把 shell 识别保持为纯、无持久化副作用的边界。

## 未决问题

无。源文件没有说明 `Shell.name(shell)` 的所有可能返回值，也没有定义 `sh`、`zsh` 等别名是否未来加入；当前可确认的契约是只有四个精确值，其余统一回退 `"bash"`，重命名计划在 opencode 2.0 前保持兼容。


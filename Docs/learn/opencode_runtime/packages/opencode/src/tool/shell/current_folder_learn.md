# Docs/learn/opencode_runtime/packages/opencode/src/tool/shell — 目录汇总（master 聚合）

- folder_path: `Docs/learn/opencode_runtime/packages/opencode/src/tool/shell`
- files: 2
- total_source_bytes: 17351
- 说明：本汇总由主控从各文件 1:1 笔记确定性聚合（不新增未在笔记中出现的语义）。

## 模块清单与要点

- `packages/opencode/src/tool/shell/id.ts` (572 B) -> `Docs/learn/opencode_runtime/files/packages/opencode/src/tool/shell/id.ts_learn.md`
  - 要点：`const kinds = ["bash", "pwsh", "powershell", "cmd"] as const`（L1-L1）。`as const` 将数组元素收窄为四个字符串字面量，并形成只读元组语义。允许值的顺序固定为 `bash`、`pwsh`、`powershell`、`cmd`；没有空字符串、大小写变体或其他 shell 别名。该常量未导出，外部只能通过类型和函数间接使用。
- `packages/opencode/src/tool/shell/prompt.ts` (16779 B) -> `Docs/learn/opencode_runtime/files/packages/opencode/src/tool/shell/prompt.ts_learn.md`
  - 要点：`command`：必需 `Schema.String`，描述为 “The command to execute”（L16-L18）；空字符串、shell 语法和路径安全性不由此 schema 判定。

## zenpi Rust 映射（聚合）

- `packages/opencode/src/tool/shell/id.ts`: `src/core.rs:L2941-L2941` 的 `Agent::run_user_shell_with_cancel` 是当前用户 shell 所有者：要求单个 `!` 前缀、检查 agent 空闲态、构造 `ToolOrigin::UserShell`，执行审批/策略门控，并在 `src/core.rs:L3125-L3125` 调用 `RunCommandTool::invoke_user_shell_with_cancel
- `packages/opencode/src/tool/shell/prompt.ts`: `src/core.rs` 的 `Agent::run_user_shell_with_cancel`（约 L2961 起）是最直接的执行落点：它校验单个 `!`、要求 `ToolRuntime` workspace/policy、创建 approval、在 spawn 前 `begin_operation` 并调用 `RunCommandTool::invoke_user_shell_with_cancel`。对应关系是：`comma

## 未决问题（聚合）

- `packages/opencode/src/tool/shell/id.ts`: 
- `packages/opencode/src/tool/shell/prompt.ts`: 1. `DESCRIPTION`（`./shell.txt`）的完整占位符集合不在本源文件内；仅能确认 `render` 显式提供的 11 个键（L275-L287），无法仅凭本文件确认模板是否还有其他占位符。

## 覆盖

- 本目录 2 个源文件均有 1:1 笔记；`file_learn_index.tsv` 与本清单一一对应。

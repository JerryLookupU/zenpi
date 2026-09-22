# Docs/learn/opencode_runtime/packages/opencode/src/permission — 目录汇总（master 聚合）

- folder_path: `Docs/learn/opencode_runtime/packages/opencode/src/permission`
- files: 3
- total_source_bytes: 14266
- 说明：本汇总由主控从各文件 1:1 笔记确定性聚合（不新增未在笔记中出现的语义）。

## 模块清单与要点

- `packages/opencode/src/permission/arity.ts` (6376 B) -> `Docs/learn/opencode_runtime/files/packages/opencode/src/permission/arity.ts_learn.md`
- `packages/opencode/src/permission/evaluate.ts` (29 B) -> `Docs/learn/opencode_runtime/files/packages/opencode/src/permission/evaluate.ts_learn.md`
- `packages/opencode/src/permission/index.ts` (7861 B) -> `Docs/learn/opencode_runtime/files/packages/opencode/src/permission/index.ts_learn.md`
  - 要点：1. **依赖与公开符号（L1-L26）**

## zenpi Rust 映射（聚合）

- `packages/opencode/src/permission/arity.ts`: `src/tool_runtime.rs` 已有 `classify_master_session_input`（约 `L75-L110`），负责空输入、长度、控制字符和 `!`/steer 路由；建议新增纯函数 `command_prefix(tokens: &[&str]) -> Vec<String>` 或零拷贝的 `CommandPrefix<'a>`，并将 arity 表放在 `tool_runtime.rs` 同级的 `co
- `packages/opencode/src/permission/index.ts`: **建议落点**：新增 src/permission.rs 并在 src/lib.rs 导出，定义 PermissionAction { Allow, Deny, Ask }、PermissionRule、PermissionRequest、PermissionReply { Once, Always, Reject }、PermissionService。PermissionService 以 workspace/session ow

## 未决问题（聚合）

- `packages/opencode/src/permission/arity.ts`: 1. `arity.ts` 的注释要求“flags NEVER count as tokens”，但源文件没有 tokenizer，无法确认所有调用者都已一致过滤 flags。
- `packages/opencode/src/permission/evaluate.ts`: 1. 仅凭 `evaluate.ts` 无法确认目录导入 `"."` 在所有构建器/路径别名下都解析到哪个入口；本笔记按当前同目录 `index.ts` 核对。
- `packages/opencode/src/permission/index.ts`: InstanceState.make 是否保证同一实例内 Effect 并发访问 Map 的串行化，源文件未给出；重复 id 的覆盖行为也未被防护或测试。

## 覆盖

- 本目录 3 个源文件均有 1:1 笔记；`file_learn_index.tsv` 与本清单一一对应。

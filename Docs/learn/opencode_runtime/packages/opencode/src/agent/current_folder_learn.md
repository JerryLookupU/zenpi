# Docs/learn/opencode_runtime/packages/opencode/src/agent — 目录汇总（master 聚合）

- folder_path: `Docs/learn/opencode_runtime/packages/opencode/src/agent`
- files: 2
- total_source_bytes: 17971
- 说明：本汇总由主控从各文件 1:1 笔记确定性聚合（不新增未在笔记中出现的语义）。

## 模块清单与要点

- `packages/opencode/src/agent/agent.ts` (16746 B) -> `Docs/learn/opencode_runtime/files/packages/opencode/src/agent/agent.ts_learn.md`
  - 要点：1. 模块依赖与数据模型（L1-L34）
- `packages/opencode/src/agent/subagent-permissions.ts` (1225 B) -> `Docs/learn/opencode_runtime/files/packages/opencode/src/agent/subagent-permissions.ts_learn.md`
  - 要点：1. `canTask` 在 `L18` 通过 `.some()` 判断子代理规则中是否存在 `rule.permission === "task"`。只检查权限名称，不检查 `action`、`pattern` 或规则顺序。因此只要出现一条 `task` 规则（即使该规则的 `action` 是 `deny`，或只匹配有限 pattern），就视为“已许可/已覆盖”，后续不会追加默认全局拒绝。

## zenpi Rust 映射（聚合）

- `packages/opencode/src/agent/agent.ts`: agent 目录/类型落点：建议新增 src/agent.rs，定义可序列化的 AgentInfo（name/description/mode/native/hidden/top_p/temperature/color/permission/model/variant/prompt/options/steps）、AgentMode、AgentCatalog 和 AgentService。AgentCatalog::get/list/de
- `packages/opencode/src/agent/subagent-permissions.ts`: `src/approval.rs` 的 `ApprovalPolicy`、`ApprovalMode` 与 `decide_after_preflight`（约 `L22-L35`、`L466-L525`）面向 side effect 和工具名做审批；它不是规则数组继承器。可新增独立的 `SubagentPermissionRule { permission: String, pattern: String, action: Permi

## 未决问题（聚合）

- `packages/opencode/src/agent/agent.ts`: 1. InstanceState.make 的具体缓存失效/并发保证不在本文件中，无法确认同一 ctx 是否可并行初始化。
- `packages/opencode/src/agent/subagent-permissions.ts`: `PermissionV1.Ruleset` 的下游匹配优先级、冲突规则解析及 `pattern: "*"` 的具体 glob 语义不在本文件中，无法仅凭该源确认。

## 覆盖

- 本目录 2 个源文件均有 1:1 笔记；`file_learn_index.tsv` 与本清单一一对应。

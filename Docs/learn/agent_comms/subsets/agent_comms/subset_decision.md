# Subset Decision — agent_comms

目标：学习三家 CLI（opencode / codex CLI / Claude Code）的 agent 通信与 session 通信实现，为 zenpi 的 DAG 编排（节点与 parent/grandparent/sibling/child 通信；节点及其全部后代全绿才允许 close，否则保活并派生新 worker）提取最小可用机制。

| 源 | 文件 | 角色 |
|---|---|---|
| codex-rs | 26 | agent 定义/角色/状态、tasks 生命周期、protocol items/approvals/request_user_input/plan |
| opencode | 17 | acp（Agent Client Protocol）、sync、share、subagent 运行态投影 |
| claude-code (plugins/examples) | 14 | hookify 规则引擎与 hooks、agent-sdk 子代理定义、示例 hook |

Excluded: 各 CLI 的 UI、模型调用、工具实现细节（已在 opencode runtime 学习覆盖）。
mapping_mode=understand，目标产物为中文 1:1 笔记 + zenpi DAG 映射。

# Subset Decision — opencode_agent_runtime

subset_id: `opencode_agent_runtime`
source_repo: `https://github.com/anomalyco/opencode`
source_revision: `fe3f3a41f79ad292cc3c7c629567385a20ec5130` (2026-09-21)
learn_mode: `understand`
target_repo: `weiyangzen/zenpi` (Rust agent runtime + TUI + headless)
language: 中文正文；代码标识符、路径、命令保留原文；每文件附 zenpi Rust 映射。

## Scope

Included (exact manifest: `subsets/opencode_agent_runtime/source_manifest.tsv`, 61 files, 688046 bytes):

| Directory | Files | Bytes | Role |
|---|---|---|---|
| `packages/opencode/src/session/**` | 24 | 301766 | agent 回合循环、LLM 流、消息、压缩、重试、摘要 |
| `packages/opencode/src/tool/**` | 27 | 189072 | 工具注册与内置工具执行（bash/read/write/edit/grep/...） |
| `packages/opencode/src/permission/**` | 3 | 14266 | 权限/审批判定 |
| `packages/opencode/src/provider/**` | 5 | 164971 | provider 与模型运行时 |
| `packages/opencode/src/agent/**` | 2 | 17971 | agent 定义与子代理权限 |

Excluded: TUI/desktop/web/server/SDK/plugin/skill/mcp/lsp/ide/storage 等非 runtime 面。Exclusion keeps the subset on the headless agent runtime the user wants to strengthen in zenpi.

## Rationale

`session` + `tool` + `permission` + `provider` + `agent` form the complete headless runtime: turn admission, prompt assembly, provider streaming, tool dispatch, approval gating, compaction and recovery. zenpi's corresponding surfaces are `src/core.rs`, `src/headless.rs`, `src/session.rs`, `src/tool_runtime.rs`, `src/runtime.rs`, `src/protocol.rs`, `src/approval.rs`, `src/providers/**`.

## Coverage Obligations

- One final note per source file: `files/<source_path>_learn.md`.
- One folder artifact per represented folder (8): `<source_dir>/current_folder_learn.md`.
- Every manifest row ends `[x]`; `[ ]` and `[_]` are unfinished.
- Each note carries source_id/hash/bytes/line coverage and a `## zenpi Rust 映射` section.

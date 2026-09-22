# Docs/learn/opencode_runtime/packages/opencode/src/session/llm — 目录汇总（master 聚合）

- folder_path: `Docs/learn/opencode_runtime/packages/opencode/src/session/llm`
- files: 4
- total_source_bytes: 33104
- 说明：本汇总由主控从各文件 1:1 笔记确定性聚合（不新增未在笔记中出现的语义）。

## 模块清单与要点

- `packages/opencode/src/session/llm/ai-sdk.ts` (9524 B) -> `Docs/learn/opencode_runtime/files/packages/opencode/src/session/llm/ai-sdk.ts_learn.md`
  - 要点：`adapterState()`（`L10-L20`）创建每条流独享的可变状态：`step/text/reasoning` 计数器均从 0 起；当前文本、推理块 ID 初始为 `undefined`；`toolNames` 是调用 ID 到工具名的映射；`copilotTotalNanoAiu` 初始无值。状态不是线程安全对象，调用方须按流事件顺序消费。
- `packages/opencode/src/session/llm/native-request.ts` (7953 B) -> `Docs/learn/opencode_runtime/files/packages/opencode/src/session/llm/native-request.ts_learn.md`
  - 要点：`ToolInput`（`L16-L19`）是工具配置的最小形状，`description`、`inputSchema` 均可缺省且只读。导出的 `RequestInput`（`L21-L35`）要求 `model`、`messages`，其余均可选：API key/base URL、额外 system 文本、工具表、`toolChoice`（`auto|required|none`）、采样参数、最大输出 token、provider 
- `packages/opencode/src/session/llm/native-runtime.ts` (8036 B) -> `Docs/learn/opencode_runtime/files/packages/opencode/src/session/llm/native-runtime.ts_learn.md`
  - 要点：1. L54-L56 检查 `model.providerID`。仅接受精确的 `"openai"`、`"anthropic"`，或以 `"opencode"` 开头的 provider；否则返回 `unsupported`，原因是 provider 不是 openai、opencode 或 anthropic。
- `packages/opencode/src/session/llm/request.ts` (7591 B) -> `Docs/learn/opencode_runtime/files/packages/opencode/src/session/llm/request.ts_learn.md`
  - 要点：USER_AGENT（L18-L18）由安装版本插入为 opencode/<InstallationVersion>，最终会进入请求 headers。

## zenpi Rust 映射（聚合）

- `packages/opencode/src/session/llm/ai-sdk.ts`: `src/backend.rs` 的 `ProviderEvent`（`ProviderEvent::TextDelta/ReasoningDelta/TextDone/ToolCallDelta/ToolCallDone/Usage/Completed/Failed`）是最接近的共享事件层；`src/core.rs` 的 `AgentEvent::Provider` 将 provider 事件按 `turn_id` 关联并送入 ses
- `packages/opencode/src/session/llm/native-request.ts`: `RequestInput.model` 对应 `providers::registry::ModelDescriptor` 加 `providers::connection::ProviderConnection`；`model()` 的 npm 路由应落到 `providers::Protocol/EndpointOperation/resolve_connection`。现有 `src/providers/**` 已有 OpenA
- `packages/opencode/src/session/llm/native-runtime.ts`: **请求与 provider 路由：** 建议在 `src/backend.rs` 的 `CompletionRequest`、`RequestControl`、`ProviderEvent`（当前定义见 `CompletionRequest` L174-L188、`ProviderEvent` L238-L276、取消/截止检查 L390-L415）上增加一个可选 native-stream 适配层；`Backend::complet
- `packages/opencode/src/session/llm/request.ts`: src/core.rs 的 complete_with_tools 在 L4100-L4161 构造 instructions、工具 definitions、CompletionRequest，并执行 context budget、资源/skill 合并；L3659-L3835 负责可取消 provider turn、操作标记和结果持久化。可把 request.ts 的 system/messages/options/headers 逻

## 未决问题（聚合）

- `packages/opencode/src/session/llm/ai-sdk.ts`: 
- `packages/opencode/src/session/llm/native-request.ts`: `@opencode-ai/llm` 各 `configure/model` 实现如何进一步解释 `limits`、`providerMetadata` 以及 `toolChoice`，仅凭本文件无法确认，需查该包实现。
- `packages/opencode/src/session/llm/native-runtime.ts`: 1. `LLMNative.request` 和 `LLMClient.stream` 对 `input.abort` 的具体 signal 传播不在本文件内，无法确认 provider HTTP 是否会因该 signal 中止。
- `packages/opencode/src/session/llm/request.ts`: Plugin.Interface.trigger 是否允许原地修改传入数组、其返回值是否总是完整替换 payload，源文件本身无法确认；需查看 plugin runtime 的契约与错误类型。

## 覆盖

- 本目录 4 个源文件均有 1:1 笔记；`file_learn_index.tsv` 与本清单一一对应。

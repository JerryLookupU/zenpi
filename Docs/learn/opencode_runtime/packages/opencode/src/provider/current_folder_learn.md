# Docs/learn/opencode_runtime/packages/opencode/src/provider — 目录汇总（master 聚合）

- folder_path: `Docs/learn/opencode_runtime/packages/opencode/src/provider`
- files: 5
- total_source_bytes: 164971
- 说明：本汇总由主控从各文件 1:1 笔记确定性聚合（不新增未在笔记中出现的语义）。

## 模块清单与要点

- `packages/opencode/src/provider/auth.ts` (7877 B) -> `Docs/learn/opencode_runtime/files/packages/opencode/src/provider/auth.ts_learn.md`
  - 要点：Schema 与输入形状（L1-L39）：When 是条件对象，字段为 key: string、op: "eq"|"neq"、value: string。TextPrompt 固定 type: "text"，含 key、message，可选 placeholder 与 when；SelectOption 含 label、value、可选 hint；SelectPrompt 固定 type: "select"，含 key、message、
- `packages/opencode/src/provider/error.ts` (5912 B) -> `Docs/learn/opencode_runtime/files/packages/opencode/src/provider/error.ts_learn.md`
  - 要点：`HeaderTimeoutError`（L7-L13）继承 `Error`，固定只读 `name = "ProviderHeaderTimeoutError"`，构造输入是毫秒数 `ms: number`，同时保留只读字段 `ms`，消息精确为 `Provider response headers timed out after ${ms}ms`。它只是“响应头超时”分类标记；构造不启动定时器、不取消请求，也不改变任何共享状态。
- `packages/opencode/src/provider/model-status.ts` (291 B) -> `Docs/learn/opencode_runtime/files/packages/opencode/src/provider/model-status.ts_learn.md`
  - 要点：1. `import { Schema } from "effect"`（`L1-L1`）。仅引入 `effect` 的 `Schema` 构造器，后续用它建立运行时 schema；没有本地配置、IO 或初始化副作用。
- `packages/opencode/src/provider/provider.ts` (80947 B) -> `Docs/learn/opencode_runtime/files/packages/opencode/src/provider/provider.ts_learn.md`
  - 要点：`anthropic` 关闭 autoload，仅注入 interleaved/fine-grained tool streaming beta header（L176-L184）。
- `packages/opencode/src/provider/transform.ts` (69944 B) -> `Docs/learn/opencode_runtime/files/packages/opencode/src/provider/transform.ts_learn.md`
  - 要点：基础类型、常量与识别函数：`Modality` 为模型输入模态联合类型（L8）；`mimeToModality` 把 `image/*`、`audio/*`、`video/*`、`application/pdf` 转成 `image|audio|video|pdf`，未知返回 `undefined`（L10-L16）。`OUTPUT_TOKEN_MAX=32000`，`INCLUDE_ENCRYPTED_REASONING=["reas

## zenpi Rust 映射（聚合）

- `packages/opencode/src/provider/auth.ts`: 现有认证底座：src/auth/mod.rs:L1-L170 已有 AuthBinding、AuthError、LoginFlowId、LoginState 与 LoginControl；src/auth/codex.rs 已实现固定 openai-codex 的 browser/device OAuth、active login 互斥、取消和提交；src/auth/callback.rs 已实现 loopback callback 的
- `packages/opencode/src/provider/error.ts`: 建议落点为新增 `src/providers/error.rs`（或 `src/backend/error.rs`，由 `src/providers/mod.rs` 导出），定义 `HeaderTimeoutError`、`ResponseStreamError`、`ParsedStreamError`、`ParsedApiCallError` 及 `parse_stream_error`/`parse_api_call_error`。
- `packages/opencode/src/provider/model-status.ts`: `#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)] #[serde(rename_all = "lowercase")] pub enum ModelStatus { Alpha, Beta, Deprecated, Active }`，并在 `ModelDescriptor` 增加 `status: ModelStatus`。不要用 `Defaul
- `packages/opencode/src/provider/provider.ts`: **provider catalog/模型元数据**：`src/providers/registry.rs` 的 `ModelDescriptor`、`effective_capabilities`、`validate_reasoning`、`context_budget`、`digest` 已对应 `Model` 的能力、限制、推理与价格核心语义（L58-L119）。建议新增/扩展 `ProviderCatalog` 保存 `Info
- `packages/opencode/src/provider/transform.ts`: 模型与 provider 入口：zenpi 的 `src/providers/registry.rs` 已有 `ModelDescriptor`、`ReasoningLevel`、capabilities/limits；`src/providers/mod.rs` 有 `Protocol`、`OptionPolicy`；`src/providers/connection.rs` 负责 route 校验。因此可新增 `src/provid

## 未决问题（聚合）

- `packages/opencode/src/provider/auth.ts`: AuthOAuthResult、Hooks["auth"] 的完整联合类型不在本文件内，无法仅凭本文件确认 success 结果是否保证 key/refresh 二选一、metadata 的精确 schema，以及 method.authorize/callback 的 Promise 是否可取消。
- `packages/opencode/src/provider/error.ts`: 1. `APICallError` 的 `responseBody`、`statusCode`、`responseHeaders` 和 `url` 在当前安装的 `ai` 版本中是否始终具有本文假设的类型与可选性，源文件自身未声明。
- `packages/opencode/src/provider/model-status.ts`: `model-status.ts` 只转导出 `CatalogModelStatus`，无法单独确认 catalog 状态的业务转移规则，以及何时允许 `deprecated` 模型继续发送请求。
- `packages/opencode/src/provider/provider.ts`: 1. `ModelsDev.Service.get()` 的 catalog 快照是否会在同一进程热刷新，源文件只展示一次 layer 初始化，无法确认刷新策略（L1403-L1406）。
- `packages/opencode/src/provider/transform.ts`: 

## 覆盖

- 本目录 5 个源文件均有 1:1 笔记；`file_learn_index.tsv` 与本清单一一对应。

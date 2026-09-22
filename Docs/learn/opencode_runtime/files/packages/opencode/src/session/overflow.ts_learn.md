# OC-021 — packages/opencode/src/session/overflow.ts

- source_id/item_id: `OC-021`
- source_path: `packages/opencode/src/session/overflow.ts`
- source_hash: `fa0d52e8a59b737e8129d6473f9f6416567cd765c6068f854e4ea0dd3d351d10`
- source_bytes: `1313`
- source_lines: `34`
- coverage: 已按顺序读取完整文件，字节范围 `0-1312`（共 1313 字节），行范围 `L1-L34`（共 34 行），包含全部注释、类型导入、常量和导出符号。

## 完整行为复盘

文件只有一个模块常量和两个导出函数；没有类、可变模块状态或默认导出。`L1-L6` 的导入分别提供 `Config` 类型、`ConfigV1.Info`、`SessionV1.Assistant["tokens"]`、`Provider.Model`、`ProviderTransform` 和 `MessageV2` 类型/符号。就本文件而言，`Config` 与 `MessageV2` 没有被后续函数使用，属于保留或类型边界导入；运行时关键依赖是 `ConfigV1` 和 `ProviderTransform`。

`COMPACTION_BUFFER` 在 `L8` 固定为 `20_000`，它是没有显式配置 `cfg.compaction.reserved` 时的最大保留缓冲上限，单位是 token。

`usable(input)`（导出，`L10-L20`）计算触发压缩前可接受的 token 阈值，返回非负整数：

- 输入对象是 `{ cfg: ConfigV1.Info, model: Provider.Model, outputTokenMax?: number }`；读取 `model.limit.context`（`L11`）。
- 若 context 为 `0`，立即返回 `0`（`L12`）。这表示未知/禁用的 context 不给出可用预算；`isOverflow` 会在调用它前短路，因此不会把所有正常用量判为溢出。
- `reserved` 取 `cfg.compaction?.reserved`，若未配置则取 `Math.min(COMPACTION_BUFFER, ProviderTransform.maxOutputTokens(model, outputTokenMax))`（`L14-L16`）。因此显式配置优先；否则最多保留 20,000 token，但不会超过模型/调用层计算出的最大输出 token。`outputTokenMax` 可选，具体缺省解释委托给 `ProviderTransform.maxOutputTokens`。
- 当 `model.limit.input` 为真值（通常是正数）时，返回 `Math.max(0, model.limit.input - reserved)`（`L17-L18`）；所以有 input cap 时仍扣除输出/压缩保留空间，结果不会为负。
- 当 `model.limit.input` 未设置或为假值时，返回 `Math.max(0, context - ProviderTransform.maxOutputTokens(model, outputTokenMax))`（`L18-L19`）。此分支直接从 context 扣除最大输出预算，而不是使用前面算出的 `reserved`。若模型输出上限大于 context，返回 0。
- 函数没有显式异常分支、I/O、写入或异步等待；依赖函数若抛错，错误会原样向调用方传播。它只读取输入，重复调用结果相同；无共享可变状态，天然可并发调用。

`isOverflow(input)`（导出，`L22-L34`）判断一次 assistant usage 是否达到阈值，返回同步 `boolean`：

- 输入对象是 `{ cfg: ConfigV1.Info, tokens: SessionV1.Assistant["tokens"], model: Provider.Model, outputTokenMax?: number }`（`L22-L27`）。
- `cfg.compaction?.auto === false` 时立即返回 `false`（`L28`），显式关闭自动压缩优先于 token 数量。
- `model.limit.context === 0` 时也立即返回 `false`（`L29`），避免“阈值为 0”导致任何用量都溢出。
- token 总量在 `L31-L32` 计算：优先使用 `tokens.total`；只有当它为 JavaScript 假值（典型是 `0`）时，才回退到 `tokens.input + tokens.output + tokens.cache.read + tokens.cache.write`。回退式包含 cache 读写，未单独累加 reasoning 字段；若上游的 `total` 已包含这些字段则直接信任上游值。
- `L33` 使用 `count >= usable(input)`，等于阈值也算 overflow，不要求严格大于。最终结果不修改 token、config 或 model，也不发起压缩；调用者负责根据 `true` 执行后续 compaction。

调用关系上，同目录 `compaction.ts` 在 `L17-L18` 导入这两个函数；其 `preserveRecentBudget` 在 `L115-L120` 复用 `usable`，而服务层的 `isOverflow` 在 `L203-L213` 注入 config 与 `RuntimeFlags.outputTokenMax` 后调用本文件函数。processor 在 `L491-L496` 只有在 assistant 尚无 summary 且本判断为真时才设置 `needsCompaction`，说明本文件是判据而非压缩执行器。

## 状态、取消、恢复与副作用

该文件是纯计算边界：没有会话状态、缓存、锁、Promise、线程、定时器或并发协调；不存在内部取消、超时或重试语义。`usable`/`isOverflow` 不创建或更新持久化记录，也不触碰 provider、网络、文件系统、工具或 approval。发生 overflow 只返回布尔值，真正的 compaction、重试策略和恢复由上层 session/processor 处理；同样，context 为 0 或 `auto === false` 是“跳过自动压缩”的稳定结果，不是错误。

恢复语义是间接的：上层压缩完成后可能把摘要/检查点写回 session，下一次调用重新传入新的 `tokens`；本文件不会记住上一次判断。由于无可变状态，多个 turn 并行读取不同输入时不会互相影响；若上游在计算期间修改对象，结果只反映调用时读取到的字段，文件没有快照或原子性承诺。

## 源内测试与行为判据

源文件本身没有测试，写为“源内未包含测试”。同目录行为测试位于 `packages/opencode/test/session/compaction.test.ts`：

- `L382-L405` 验证超阈值为 `true`、预算内为 `false`；`L407-L417` 验证 `cache.read` 会计入回退总量。
- `L419-L451` 覆盖存在 `model.limit.input` 时的上下界，以及输出接近上限仍应为 `false` 的边界。
- `L467-L511` 比较有无 `limit.input` 的模型；测试注释声称 input cap 不应丢失输出余量，但当前实现实际在 `L17-L18` 扣减 `reserved`，判据应以可执行公式为准：有 input cap 时 `usable=max(0,input-reserved)`，无 input cap 时 `usable=max(0,context-maxOutput)`。
- `L535-L555` 验证 `context === 0` 和 `compaction.auto === false` 均返回 `false`。独立验证可构造最小 `ConfigV1.Info`、`Provider.Model` 和 token 对象，逐项断言 `usable` 数值及 `isOverflow` 的等号边界、cache 回退和两个短路条件。

## zenpi Rust 映射

- `src/context.rs` 已有 `ContextBudget { max_tokens, reserved_output_tokens }`、`estimate_tokens` 和上下文准备逻辑；建议新增纯函数 `usable_context_tokens(model: &ModelDescriptor, budget: ContextBudget, output_token_max: Option<u64>) -> u64` 与 `is_context_overflow(usage: &TokenUsage, ...) -> bool`，把本文件的 `max(0, …)` 改写为 `saturating_sub`。可执行判据是分别测试 context=0、input cap、无 input cap、显式 reserved 和阈值相等。
- `src/providers/registry.rs` 的 `ModelDescriptor` 已有 `context_window` 与 `max_output_tokens`（对应 `Provider.Model.limit.context`/输出上限）；其 `context_budget`（约 `L101-L110`）会把用户预算裁剪到模型能力，适合作为 `ProviderTransform.maxOutputTokens` 的 Rust 落点。`src/providers/**` 目前没有与 TypeScript `limit.input` 完全等价的统一字段，需在 descriptor/override 中明确该差异，不能静默假设 context 等于 input。
- `src/backend.rs` 的 `Usage`（约 `L298-L304`）提供 `input_tokens`、`output_tokens`、`total_tokens`，但未见 `cache.read`/`cache.write` 对应字段。若要复刻回退公式，应扩展 usage 或新增 `cache_read_tokens`/`cache_write_tokens`；并明确 `total_tokens == 0` 时才回退，保持 JavaScript `||` 的语义。
- `src/core.rs` 已有 `Agent::set_context_budget`、`context_budget`、`compact_context`/`compact_context_with_control` 和 `CompactionReport`（约 `L1963-L1967`、`L2412-L2472`）。建议在 provider 请求 admission 前调用 overflow 判定，`true` 时进入既有 `compact_context`，而不是让判定函数直接写 session；将 `CompactionReport` 作为可验证的执行结果。
- `src/session.rs` 提供 append-only event、semantic checkpoint 和恢复接口（`L1231-L1389` 一带），承担压缩后的持久化/恢复；映射应保持纯判定与持久化分离，overflow 函数不直接操作 `SessionStore`。
- `src/runtime.rs` 的 `CancellationToken`（`L49-L96`）及 `BackgroundRunner` 负责协作取消、排队和 bounded shutdown；本文件无取消点，若压缩流程在后台运行，应由调用方在 compaction 前后检查 token，不能把 overflow `bool` 当作取消结果。
- `src/headless.rs` 的 JSONL runner/replay 负责事件传输和恢复，`src/protocol.rs` 定义版本化 wire 类型；若需要对外可观测性，新增 `context_overflow`/`compaction_required` 事件应放在 protocol/core 适配层，保留当前纯函数作为内部判据。
- `src/tool_runtime.rs` 的 tool batch 有输入/字节上限和取消传播，`src/approval.rs` 处理 side-effect approval；二者都不是 token overflow 的执行点。压缩前若涉及工具结果裁剪，应由 `Agent`/`SessionStore` 完成并遵守现有 cancellation 与持久化顺序。

建议的可验证 Rust 差异清单：补齐 model input-cap 字段；补齐 cache token 统计；固定 `total == 0` 回退规则；用 `saturating_sub` 防下溢；为 context=0、auto-disabled、显式 reserved、等号阈值添加单元测试；在 `Agent` 的请求路径中验证 overflow 后确实产生 `CompactionReport`/checkpoint，而纯判定单元测试不应产生文件或网络副作用。

## 未决问题

1. `ProviderTransform.maxOutputTokens` 在 `outputTokenMax` 未提供、模型没有输出上限或配置非法时的精确返回/抛错规则不在本文件内。
2. `tokens.total` 是否始终已包含 `cache.read`、`cache.write`，以及何时会为 0，只能由 `SessionV1.Assistant["tokens"]` 的生产方确认。
3. `ConfigV1.Info.compaction.reserved` 的单位、允许负值与校验规则未在本文件定义；本函数只在结果处做 `Math.max(0, …)`，没有对显式 reserved 单独校验。

# OC-010 — packages/opencode/src/provider/transform.ts

## 元信息

- source_id/item_id：`OC-010`
- source_path：`packages/opencode/src/provider/transform.ts`
- source_hash：`c07d49e48dd2478ad2813a10805781a72551db2fd847b7df994cc854bf654c16`
- source_bytes：`69944`
- source_lines：`1909`
- coverage：已从字节 1 至 69944、行 `L1-L1909` 按顺序读取；文件哈希与给定值一致。

## 完整行为复盘

该文件是 AI SDK 请求进入具体 provider 前的同步适配层。输入主要是 `Provider.Model`、`ModelMessage[]`、模型能力/限制和 provider options，输出仍是普通对象或消息数组；没有网络 I/O、Promise、线程或隐式重试。

- 基础类型、常量与识别函数：`Modality` 为模型输入模态联合类型（L8）；`mimeToModality` 把 `image/*`、`audio/*`、`video/*`、`application/pdf` 转成 `image|audio|video|pdf`，未知返回 `undefined`（L10-L16）。`OUTPUT_TOKEN_MAX=32000`，`INCLUDE_ENCRYPTED_REASONING=["reasoning.encrypted_content"]`（L18-L23）。`sanitizeSurrogates` 用正则把孤立 UTF-16 高/低代理替换为 U+FFFD，成对代理（正常 emoji）保持（L25-L27）。`isKimiFamily` 检查 provider/model id 是否含 `kimi`/`moonshot`，或 URL 是否含四个 Moonshot 域名（L29-L39）。`sdkKey` 将 npm 包名映射成 AI SDK providerOptions 命名空间，如 OpenAI、Anthropic、Bedrock、Vertex、Gateway、OpenRouter 等；未知返回 undefined，`ai-gateway-provider` 特意返回 camelCase `openaiCompatible`（L41-L98）。

- `normalizeMessages`（L100-L356）：先逐消息就地清洗文本。tool 消息只处理 `tool-result` 输出的 `text/error-text/content`；system 处理字符串；user/assistant 同时处理字符串及数组中的 text，assistant 还处理 reasoning 和 tool-result（L106-L166）。Anthropic 删除空字符串消息、空 text；空 reasoning 只有在有 Anthropic `signature` 或 `redactedData` 时保留（L168-L195）。Bedrock 采用类似过滤，但 reasoning 只有 provider 对应命名空间或 `bedrock` 中存在 `signature/redactedContent/redactedData` 才保留（L197-L222）。Claude 家族把 assistant/tool 的 toolCallId 替换为只含字母数字、`_`、`-` 的值（L224-L251）。Mistral/Devstral/Codestral/Pixtral/Mixtral 的 ID 去除非字母数字、截断 9 字符并用 `0` 补齐；tool 后紧邻 user 时插入 assistant 文本 `Done.`，然后提前返回（L253-L301）。DeepSeek 的每条 assistant 消息若无 reasoning 就补空 reasoning（字符串会先变成 text part）（L303-L319）。当 capabilities.interleaved.field 存在且不是 OpenRouter 时，从 assistant 数组移除 reasoning，将拼接文本写入 `providerOptions.openaiCompatible[field]`，即使空串也写入，保证下一轮回放（L321-L353）。注意该函数及其子函数会修改原消息/part 对象（sanitize、providerOptions 写入），并非深拷贝；正常路径返回同一数组元素。

- `applyCaching`（L358-L407）：取前两个 system 与后两个非-system 消息，去重后注入 Anthropic/OpenRouter/OpenAI-compatible/Copilot/Alibaba 的 `cacheControl` 或 Bedrock `cachePoint`。非 Anthropic/Bedrock 且有内容数组时优先把选项合并到最后一个内容 part；tool approval request/response 不写 part 级缓存，改写 message 级。使用 `mergeDeep`，已有字段保留并递归合并。

- `unsupportedParts`（L409-L445）：只转换 user 数组中的 file/image。空 data URL base64 变为错误 text；根据 MIME/文件名和模型 capabilities.input 判断，不支持时替换为明确的 `ERROR: Cannot read ...` 文本；支持或未知 MIME 原样保留。它不抛异常，错误通过消息交给模型/用户。

- `mapProviderOptions`（L447-L463）对 message 级和 content part 级 providerOptions 应用转换函数，但跳过 tool-approval request/response。`message`（导出，L465-L518）依次执行 unsupported、normalize；Anthropic/Claude/Alibaba 等在非自动缓存时执行 applyCaching（L468-L484）；按 `sdkKey` 把存储的 providerID 命名空间重映射到 SDK 期望键（L486-L499）；当 `store!==true` 且使用 OpenAI Responses 系列包时，删除各级 `itemId`，保留加密 reasoning 等元数据（L501-L515）。输出为变换后的消息数组。

- 采样默认值：`temperature`（L527-L544）按模型 ID 返回固定值或 undefined：North Mini Code、指定 Gemini、GLM-4.6/4.7、MiniMax-M2 为 1；Kimi-K2 thinking/2.5 变体为 1，普通 K2 为 0.6；Claude 明确 undefined。 `topP`（L546-L560）为采样默认 0.95 的 Gemini、MiniMax/Kimi 2.5、特定 DeepSeek V4；否则 undefined。`topK`（L562-L571）给 MiniMax M2/M2.5/M2.1 分别 20/40，采样 Gemini 为 64，否则 undefined。

- 推理 effort 识别：常量集合定义广泛、OpenAI GPT-5 版本/Codex/Pro/Chat 的 effort 层级及日期门槛（L573-L597）。`gpt5Version` 解析版本数字（L599-L601）；`versionedGpt5ReasoningEfforts`、`gpt5CodexReasoningEfforts`、`gpt5ChatReasoningEfforts` 按正则及版本返回数组或 undefined（L603-L622）。`openaiReasoningEfforts` 处理 deep-research、GPT-5 chat/pro/Codex、版本及发布日期，按 none/xhigh 发布日期扩展（L624-L644）；`openaiCompatibleReasoningEfforts` 使用兼容 API 的固定/版本集合（L646-L652）。

- Anthropic/Gemini 辅助：`anthropicUsesModernAdaptiveThinking` 识别 Claude 4.7+（L654-L663）；`anthropicOpus45` 识别 Opus 4.5（L665-L667）；`anthropicAdaptiveEfforts` 返回 4.6/4.7+ 的 adaptive effort 集（L669-L681）；`anthropicOmitsThinking` 与现代 adaptive 同步（L683-L685）。`anthropicBindsThinking` 识别 Claude 5.1+，但 Mythos 5.1 排除（L687-L698）。`anthropicBlockBinding` 消费 OpenCode-only `blockBinding:false`，或为 Anthropic/Vertex/Bedrock 的 enabled/adaptive thinking 注入 `prefixMismatchBehavior:"drop_block"`（L700-L736）。`googleThinkingLevelEfforts`、`googleThinkingBudgetMax` 为 Gemini 3 各变体列出 level 和预算（L738-L751）。`wrapInSapModelParams` 将每个 SAP variant 包在 `modelParams`（L753-L757），`googleThinkingVariants` 为 Gemini 2.5 返回 high/max budget，否则按 thinkingLevel 生成（L759-L775）。

- `variants`（导出，L777-L1205）：无 reasoning capability 返回空对象。其余按模型/provider 分派：MiniMax-M3 的 NVIDIA/Lilac 使用 `chat_template_kwargs`，普通 Anthropic/OpenAI-compatible 使用 disabled/adaptive thinking（L780-L819）；Kimi Anthropic 生成 low..max adaptive；DeepSeek、MiniMax、GLM（除 5.2）、Kimi、Qwen、Big Pickle 多数返回空（L820-L842）。Grok-3-mini 使用 OpenRouter `reasoning.effort` 或通用 `reasoningEffort`（L843-L855）。OpenRouter、Cloudflare `ai-gateway-provider`、AI SDK gateway、Copilot、Cerebras/Together/XAI/DeepInfra/Venice/OpenAI-compatible、Azure、OpenAI/Mantle、Anthropic/Vertex、Bedrock、Google、Mistral、Cohere、Groq、Perplexity、SAP 均有对应字段；返回空代表 SDK/模型不支持（L857-L1205）。预算受 `model.limit.output`、Anthropic 16,000/31,999、Google 32,768/24,576 等边界约束。

- `options`（导出，L1207-L1375）生成请求级默认 options：Vertex Anthropic/非 Claude Anthropic 关闭 toolStreaming；OpenAI/Azure/Copilot/Mantle/XAI 默认 `store:false`（L1214-L1235）；OpenRouter/LLMGateway 请求 usage，Gemini 3 默认 high；Baseten/Kimi/GLM 设置 thinking；ZAI/ZhipuAI 设置 enabled/clear_thinking；Meta OpenAI 加 reasoningSummary 与 encrypted include；Google reasoning 模型启用 thinkingConfig（L1236-L1275）。MiniMax-M3 Anthropic、Kimi、Alibaba-cn 分别设置 adaptive/high 或 enable_thinking（L1278-L1308）。按 `setCacheKey` 默认规则写 `prompt_cache_key` 或 `promptCacheKey=sessionID`，Gateway 开启 auto caching（L1310-L1327）。Azure completion URL 对 GPT<5.5 设置 medium 后立即返回；GPT-5 非 chat/pro 默认 medium、部分 SDK 加 summary/encrypted include；支持的 GPT-5.4+ OpenAI/Mantle 默认 `textVerbosity:"low"`，opencode provider 可写缓存键及 reasoning include（L1329-L1374）。

- `smallOptions`（导出，L1377-L1400）取首个 model variant；OpenAI/Copilot/XAI 合并 `store:false`；OpenRouter/LLMGateway 的 Google 空 variant 禁用 reasoning；Venice 空 variant 返回 `disableThinking:true`，否则原样返回。

- `providerOptions`（导出，L1408-L1466）先对 OpenAI/Azure/Mantle reasoning 强制 `forceReasoning:true`，否则应用 Anthropic block binding。Gateway 把 `gateway` 与模型上游 slug（首段路径，amazon 映射 bedrock）分开；无 slug 时合并到 gateway。其他 SDK 根据 `sdkKey` 或 providerID（某些 npm 按点号截断）生成单命名空间；Azure 同时输出 `openai` 与 `azure`，兼容两条 SDK 读取路径。

- `maxOutputTokens`（导出，L1468-L1470）返回 `Math.min(model.limit.output, outputTokenMax)`，若结果为 0/假值退回上限，默认上限 32,000。

- Schema 清洗：`JsonRecord`、`isPlainObject`（L1472-L1476）提供对象判断。`sanitizeOpenAISchema`（L1478-L1560）递归转换 OpenAI tool schema：布尔 schema 变 string；保留 `$ref/description/enum/const/properties/required/items/additionalProperties/anyOf|oneOf|allOf/$defs/definitions`；过滤未知 type；根据 keywords 推断 object/array/string/number，缺失 object properties 补空对象，缺失 array items 补 string。遇到只有 ref/combiner 的节点直接返回。
  `schema`（导出，L1562-L1702）对 OpenAI/Azure 应用上述兼容子集（L1581-L1584）；Moonshot/Kimi 递归去除 $ref 兄弟键、把 tuple items 取首项（L1586-L1602）；Google/Gemini 把 enum 全转字符串、整数/数字 enum 改 string，拆分 type 数组为 anyOf+nullable，过滤不存在属性的 required，补 array items 类型并删除非 object 的 properties/required（L1604-L1699）。没有匹配 provider 时原 schema 返回。

- 推理变体生成：`reasoningVariants`（导出，L1704-L1720）读取 ModelsDev `reasoning_options`：无配置返回 undefined，空数组返回空对象；优先 effort 选项，值 null 映射 `none`，再用 `reasoningEffort`；否则组合 toggle 与 budget_tokens，预算生成 high/max。 `effortVariants`（L1722-L1734）过滤非 string/non-null、过滤 provider 不支持项；`budgetVariants`（L1736-L1749）把 max 限制为模型 output-1 与 31,999，high 为至少一半，调用 `reasoningBudget`；`nonEmptyVariants` 空对象转 undefined（L1751-L1753）。`reasoningToggle`（L1755-L1767）仅 Alibaba/Cohere 提供 none/high 开关。 `reasoningEffort`（L1769-L1837）按 npm provider 将 effort 编码成 reasoning/thinking/reasoningEffort/modelParams 等，未支持返回 undefined；`anthropicEffort`（L1839-L1851）处理 Opus 4.5、Kimi、adaptive；`anthropicOpus45Effort` 固定 enabled 且预算不超过 16,000 或 output/2-1（L1853-L1861）；`reasoningBudget`（L1863-L1906）把预算映射到各 provider 的 token 字段，未支持 provider 返回 undefined。

- 文件最后通过 `export * as ProviderTransform from "./transform"` 暴露命名空间（L1909），调用方可统一使用上述导出符号。

## 状态、取消、恢复与副作用

该模块没有取消令牌、超时参数、重试循环、持久化写入或外部网络副作用；所有函数同步返回。副作用集中在消息对象：`normalizeMessages` 会直接改写 msg.content、tool output、reasoning/providerOptions；`applyCaching` 会向原 message/content part 合并 providerOptions；其余 map/变体/schema 多数通过浅拷贝返回，但嵌套值仍可能共享。没有原子回滚：异常（例如调用方传入不符合类型的对象导致访问字段失败）会直接抛出。恢复语义只体现在请求可重放所需元数据：非 store Responses 请求删除 `itemId` 但保留 encrypted reasoning；interleaved reasoning 被写回 providerOptions；Anthropic block binding 请求 provider 丢弃前缀不匹配 block。缓存键使用 sessionID，但本文件不落盘；实际取消/超时/重试由上层 `session/llm`、runtime 负责。

## 源内测试与行为判据

源文件本身没有测试代码；同目录测试位于 `packages/opencode/test/provider/transform.test.ts`。可核对判据包括：`options.setCacheKey` 的 true/false/undefined 分支和 OpenAI 默认键（L48-L117）；Mistral tool-call ID 变为 9 字符（测试 L2148-L2195）；DeepSeek reasoning 写入 `openaiCompatible.reasoning_content` 且非 DeepSeek 不处理（L2197-L2319）；孤立 surrogate 在 system/user/assistant/tool-result 全部替换而合法 emoji 保留（L2321-L2431）；空图片转固定错误文本（L2433-L2534）；Anthropic/Bedrock 删除空消息和空 text/reasoning、保留 tool-call（L2536-L2714）；OpenAI `store:false` 删除 itemId 但保留 encrypted content，并按 SDK namespace 而非 providerID（L2893-L2993）。独立验证还应覆盖每个 provider 分支的快照：给定最小 `Provider.Model`，断言 `options`、`variants`、`providerOptions`、`schema`、`reasoningVariants` 的字段和边界预算。

## zenpi Rust 映射

- 模型与 provider 入口：zenpi 的 `src/providers/registry.rs` 已有 `ModelDescriptor`、`ReasoningLevel`、capabilities/limits；`src/providers/mod.rs` 有 `Protocol`、`OptionPolicy`；`src/providers/connection.rs` 负责 route 校验。因此可新增 `src/providers/transform.rs`，定义 `ModelTransform`、`ProviderOptions`、`ReasoningVariant`，由 `ValidatedRoute::model()` 和 protocol 选择。
- 消息映射：在 `src/core.rs` 的 provider turn 准备处调用 `transform_messages(&mut [ModelMessage], &ModelDescriptor, &TransformOptions)`；消息类型可放 `src/protocol.rs` 或新模块，复刻 surrogate 清洗、空内容过滤、toolCallId 规范化、interleaved reasoning、unsupported modality 替换。需要明确 Rust 借用规则，避免 JS 原地别名；建议输入 owned `Vec`，输出新 Vec，并保留 `TransformWarning`。
- 请求选项与变体：将 `temperature/top_p/top_k/max_output_tokens` 落在 `src/providers/mod.rs::OptionPolicy` 扩展字段；将 `options`、`smallOptions`、`providerOptions`、`reasoningEffort/reasoningBudget` 落在新 `transform.rs`，以 `serde_json::Map` 生成 wire JSON。OpenAI/Azure/Mantle 的 `forceReasoning`、Responses encrypted include、Gateway slug 路由必须在 provider encoder（`src/providers/openai.rs`、`codex.rs`）最终确认。
- Schema：新模块提供 `sanitize_openai_schema`、`sanitize_gemini_schema`、`sanitize_moonshot_schema`，输入 `serde_json::Value`，在 `src/tool_runtime.rs` 调用工具注册/发送前执行；与 `src/providers/registry.rs` 的 `structured_output` capability 联动，未支持时返回可诊断错误或降级文本。
- 生命周期与取消：transform 本身保持纯同步；网络请求/超时由 `src/runtime.rs` 的 `CancellationToken` 和 provider worker 检查，工具副作用由 `src/tool_runtime.rs`、`src/approval.rs` 继续控制。重试不可在 transform 内自动做，重试请求必须沿用 `src/session.rs` 的 operation journal/新 operation ID 规则。
- 持久化与协议：sessionID/cache key 只作为请求字段，不直接写盘；会话记录、恢复和未知 provider/tool 状态仍由 `src/session.rs`、`src/headless.rs`、`src/protocol.rs` 管理。建议在 protocol 事件中记录 transform warnings（如 unsupported modality、schema lowering），供 headless replay 核对。
- 差异清单（可执行）：(1) zenpi 当前 provider registry 是静态能力集合，缺少 npm SDK namespace 与模型发布日期，需在 `ModelDescriptor` 增加 `api_id`、`release_date`、`sdk_key`；(2) 当前 provider 文件以 OpenAI/Anthropic/Google/DeepSeek/Codex 方言编码，需逐项补齐 Bedrock/Gateway/Copilot/Mistral/SAP/Alibaba 等分支；(3) Rust 需要显式区分 message-level/content-level cache；(4) Rust schema lowering 必须测试 bool schema、type array、$ref sibling、tuple items；(5) 将变体支持与 registry 的 `ReasoningLevel` 做交集，禁止 JS 中“返回空对象但 capability=true”的隐式情况；(6) 使用 property-based/快照测试验证 32,000、31,999、output-1 等边界。

## 未决问题

无。源文件可确定上述转换和边界；具体 provider SDK 最终如何消费字段、以及 zenpi 当前请求编码器是否已覆盖所有 npm 分支，需要在后续读取各 provider encoder 时再核对，但不影响本文件行为结论。


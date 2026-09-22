# OC-023 — packages/opencode/src/session/prompt.ts

- source_id/item_id: `OC-023`
- source_path: `packages/opencode/src/session/prompt.ts`
- source_hash: `f0c5bc64c0f0e966693d4a57f7ede1e9d6e188b396152f04b55303dc75b9b768`
- source_bytes: `64944`
- source_lines: `1631`
- coverage: 已从字节 `0-64943`、行 `L1-L1631` 按顺序读取，包含导入、注释、类型、实现与导出。

## 完整行为复盘

- 模块级常量与判定函数：`decodeMessageInfo`/`decodeMessagePart` 在 `L63-L64` 以 `Schema.decodeUnknownExit` 做保存前诊断；MCP 附件上限 `MAX_MCP_RESOURCE_BLOB_BYTES=10 MiB`、允许 MIME 集合在 `L65-L72`；结构化输出说明和系统提示在 `L74-L82`。`mcpResourceBase64Size(value)`（`L84-L88`）去空白后按 Base64 长度和尾部填充估算字节数，非负返回；`formatMcpResourceBytes(value)`（`L90-L94`）按 B/KB/MB 向上取整；`isOrphanedInterruptedTool(part)`（`L96-L100`）只把 `state.status=error` 且 `metadata.interrupted=true` 视为清理后的孤儿 tool。
- `Interface`（`L102-L109`）定义 `cancel`、`prompt`、`loop`、`shell`、`command`、`resolvePromptParts` 六个服务操作及错误类型；`Service`（`L111`）是 Effect Context 服务。`layer`（`L113-L1492`）注入会话、Agent、Provider、processor、compaction、Plugin、Permission、FS、MCP、LSP、工具注册、Image、子进程、运行状态、数据库等依赖，并在 `node`（`L1598-L1629`）声明依赖图；文件末 `export * as SessionPrompt`（`L1631`）重新导出模块。
- `ops`（`L144-L150`）给 `TaskTool` 暴露受限的 `cancel`、`resolvePromptParts`、`prompt`；prompt 异常被 `Effect.die`，因此子任务调用方看不到可恢复业务错误。`cancel(sessionID)`（`L152-L155`）记录日志并调用 `SessionRunState.cancel`，不直接删除或回滚会话。
- `resolvePromptParts(template)`（`L157-L191`）读取 `ConfigMarkdown.files` 匹配项；以 `seen` 去重，`~/` 解析到 home，其余相对 `worktree`；`fsys.stat` 失败时尝试同名 Agent，成功则生成 `agent` part，否则忽略；存在文件生成 `file:` URL，目录 MIME 为 `application/x-directory`，其他为 `text/plain`。`Effect.forEach` 使用 `concurrency:"unbounded"`，返回顺序由输入匹配顺序展平，文件 stat 可并发。
- `title(input)`（`L193-L253`）只为无 `parentID` 且仍是默认标题的根会话工作；历史中必须恰好一个非 synthetic 的 user 消息，并以其为上下文起点。若该消息全为 `subtask`，直接拼接子任务 prompt，否则经 `MessageV2.toModelMessagesEffect` 转模型消息。优先 title Agent 的显式模型，否则小模型再否则当前模型；LLM 流式文本最多 `retries:2`，移除 `<think>`、取首个非空行、截断至 100 字符（超出保留 97 加 `...`），写标题失败只记录错误。
- `handleSubtask(input)`（`L255-L449`）创建 assistant 消息和 running `TaskTool` part，保存 prompt/description/agent/command，先触发 `tool.execute.before`。找不到 Agent 时发布 `Session.Event.Error` 并抛 `NamedError.Unknown`。调用 `TaskTool.execute` 时传递可取消 `AbortSignal`、父消息、promptOps、合并权限、metadata 回写；异常转为日志并保存 `error`，中断时 abort、完成 assistant、把 running part 置为 `Cancelled` 错误状态。成功结果写 title/metadata/output/attachments 并触发 after；无结果写失败状态。若带 `command`，追加 synthetic user 消息要求模型总结并继续。
- `shellImpl(input, ready?)`（`L451-L592`）外层 `uninterruptibleMask` 保证消息初始化和收尾；清理 revert，解析 Agent/model，写 user、synthetic“用户执行工具”、assistant、running shell tool part，并用 `ready` latch 标记初始化完成。按配置选择 shell，`Shell.args` 组命令；插件可修改 `shell.env`，子进程继承环境但 `TERM=dumb`、忽略 stdin、3 秒强杀。流式追加输出并实时更新 part metadata。中断被识别为用户 abort，输出追加 `<metadata> User aborted the command </metadata>`；`finish` 不可中断地补 completed 时间、把 running part 变 completed 并保存 output。非中断失败重新抛 cause，中断则返回已持久化结果。
- `getModel(providerID, modelID, sessionID)`（`L594-L612`）调用 Provider；成功返回模型，`ModelNotFoundError` 会发布带 suggestions 的会话错误后 defect，其它错误也 defect。`currentModel(sessionID)`（`L614-L633`）先查 `SessionTable.model`，保留非 default variant；没有则取最近带模型的 user 消息，仍无则 Provider 默认模型。
- `createUserMessage(input)`（`L635-L1050`）解析显式或默认 Agent，解析输入/Agent/当前模型及 variant（只有模型匹配且 provider 中存在 variant 才采用 Agent variant），构造 `SessionV1.User`；若 Agent/model/variant 改变则 `setAgentModel`，并注册 finalizer 清理 instruction。内部 `assign`（`L693-L697`）为缺失 part ID 生成升序 ID。
  - 内部 `resolvePart`（`L699-L993`）逐类型展开。MCP resource（`L703-L784`）先插入“Reading” synthetic 文本，读取 text 内容；blob 先估大小，再拒绝不支持 MIME 或超过 10 MiB，合格时插入说明和 `data:` file part，读取失败记录日志并插入失败文本。`data:` 且 `text/plain`（`L785-L807`）插入 Read 调用说明、解码文本和原 part。`file:`（`L808-L970`）为 Read tool 创建可中断执行器；文本文件支持 `start/end`，若相等则用 LSP symbol 行范围扩展，成功结果带输出及附件，失败发布错误事件并留下失败文本；目录调用 Read 并返回输出；其它 MIME 将文件读入 Base64 data URL。`agent` part（`L974-L990`）原样附上并追加提示模型调用 Task tool；其他 part 只补 message/session ID。`execRead` 的 abort 在中断时传播到 Read tool（`L813-L828`）。
  - 所有输入 part 以 `concurrency:"unbounded"` 并发解析再按输入顺序 flatten（`L995-L997`）；触发 `chat.message` 插件（`L999-L1009`），图片 part 经 `image.normalize`，仅 `ResizerUnavailableError` 回退原图（`L1011-L1020`）。保存前对 message/part 做 Schema 诊断并记录非法项（不阻止保存），随后 `sessions.updateMessage` 和逐 part `updatePart` 持久化（`L1022-L1050`）。
- `prompt(input)`（`L1052-L1071`）取 session、执行 revert cleanup、创建 user message、touch session；`input.tools` 转为全局 `allow/deny` 规则并保存。`noReply=true` 只返回消息，不进入模型循环；否则调用 `loop`。
- `lastAssistant(sessionID)`（`L1073-L1079`）找最近非 user 消息，找不到则取一条消息，再没有抛 `Impossible`，为运行状态提供基线。
- `runLoop(sessionID)`（`L1081-L1341`）是主循环：每轮设 busy、读取 compacted 消息，要求存在 user；依据最后 assistant finish 和未执行 tool calls 决定退出，清理标记的中断孤儿不再触发 prefill（`L1100-L1129`）。首轮 fork 标题任务；优先消费 `subtask`（`handleSubtask`）和 `compaction` 任务，溢出时自动创建 compaction；取得 Agent、计算 `steps`/`MAX_STEPS_PROMPT`，应用 reminders。每轮新建 assistant 并持久化，processor 中断时写 `AbortError` 和 completed 时间。解析 SessionTools、按 JSON schema 动态加入 `StructuredOutput` tool；首轮 fork summary；并发准备 skills/environment/instruction/MCP/model messages，结构化格式追加强制系统提示。`handle.process` 携带权限、父会话、工具和最后一步提示。StructuredOutput 成功即保存 `structured` 并 break；content filter 写 `ContentFilterError` 并发布错误；要求 JSON schema 但模型未产出则写 `StructuredOutputError`；结果 `compact` 自动创建 compaction，否则 continue。finally 清理 instruction；循环结束异步 prune 并返回最后 assistant。
- `loop(input)`（`L1343-L1347`）用 `state.ensureRunning` 防止同 session 并发运行，并以 `lastAssistant` 为前置状态。`shell(input)`（`L1349-L1354`）建立 `Latch`，交给 `state.startShell`，忙时返回 `Session.BusyError`。
- `command(input)`（`L1356-L1481`）查命令，不存在则列出可用命令并发布错误。用 `argsRegex`（`L1594`）解析引号、`[Image N]` 或非空白 token；`$1...$n` 中最后一个占位符吸收剩余参数，`$ARGUMENTS` 替换原始字符串；无占位符但有参数时追加双换行参数。`!\`...`` shell 片段由配置 shell 并发执行，按出现顺序替回，最后 trim。模型优先命令模型、命令 Agent 模型、输入模型、当前模型，并用 `getModel` 校验。解析模板文件/Agent parts，去掉与输入重复的 file URL；subagent 条件下生成单个 `subtask` part，否则合并模板和输入 parts。触发 `command.execute.before` 后调用 `prompt`，发布 `Command.Event.Executed`。
- 输入 schema：`ModelRef`（`L1494-L1497`）要求 `providerID/modelID`；`PromptInput`（`L1499-L1521`）包含 session/message ID、可选 model/agent/noReply/tools/format/system/variant 及 text/file/agent/subtask 联合 parts；`tools` 标记 deprecated。`LoopInput`（`L1523-L1525`）只有 sessionID；`ShellInput`（`L1527-L1534`）要求 agent/command；`CommandInput`（`L1536-L1562`）要求 command/arguments/session，可选 agent/model/variant/messageID/file parts，并内联 FilePart schema 以保持 SDK 输出。
- `createStructuredOutputTool(input)`（`L1564-L1591`）删除 schema 的 `$schema`，以 AI SDK `jsonSchema` 校验；execute 成功回调 `onSuccess(args)` 并返回固定成功结果，`toModelOutput` 转为文本。正则常量 `bashRegex`（`L1592`）、`argsRegex`、`placeholderRegex`、`quoteTrimRegex`（`L1594-L1596`）定义命令插值边界。

## 状态、取消、恢复与副作用

运行状态由 `SessionRunState` 管理：`cancel` 只发取消请求，`ensureRunning`/`startShell` 约束单 session 活跃工作。模型 processor、TaskTool、Read tool 和 shell 都把 Effect interrupt 映射到 AbortController 或 assistant/tool 的终态；shell 的收尾在不可中断区完成，模型中断写 `AbortError`。没有通用重试循环：标题 LLM 明确 `retries:2`，模型/工具重试由下层 `LLM`、processor 或工具实现负责。恢复依赖已持久化的 message/part、revert cleanup、compaction/prune；检测到 `interrupted` orphan tool 时只记录并退出，避免重复执行。外部副作用包括数据库 session model/permission、会话消息和 parts、插件 hooks、事件总线、MCP/LSP/FS 读取、Image 归一化、shell 子进程和工具执行；失败多数转 synthetic 文本或错误事件，只有关键 model/Agent 缺失 defect/throw。并发点是文件/MCP part 解析、命令 shell 替换、系统提示准备和摘要/标题 fork；同一 session 的主 loop 仍由状态门串行化。

## 源内测试与行为判据

源文件内未包含测试。可独立验证的判据：给定默认标题且仅一条真实 user 消息时，title 只写一次并截断到 100 字符；重复 `ConfigMarkdown.files` 只产生一个 part；MCP blob 超过 10 MiB 或 MIME 不在六项集合时不生成 data attachment；`noReply` 不创建 assistant loop；同 session 并行 `loop` 必须被 busy/ensureRunning 拒绝；最后一步模型请求必须含 `MAX_STEPS_PROMPT`；JSON schema 未调用 StructuredOutput 必须得到 `StructuredOutputError`；shell 中断后 part 必须是 completed 且输出含用户中止 metadata；`$2` 应吸收其后的全部参数而 `$ARGUMENTS` 保留原始参数串。

## zenpi Rust 映射

- `PromptInput`/`CommandInput`/`LoopInput`/`ShellInput` 应落到 `src/protocol.rs` 的 `Command`、`TurnMode`、`UserShellRequest`（`L46-L119`、`L214-L510`），继续使用 `MAX_TEXT_BYTES`、控制字符和 ID 校验；命令模板插值可落在 `core.rs` 的 admission 辅助函数。当前 Rust `Command::Prompt/Steer/UserShell/Cancel` 是文本协议，尚无 TS 的 file/agent/subtask parts，需要新增受限 attachment/agent 字段并做兼容解析。
- `prompt`、`loop`、`runLoop` 对应 `src/core.rs` 的 `Agent`（`L403-L430`）、`process_sync`/`process_with_cancel`（`L1377-L1377`、`L5334-L5350`）及 `run_active_turn_cancelable_with_events`（`L3686-L3686`）；`Turn` 的 parent 链（`L143-L207`）可承载 user/assistant/tool 关系。可执行差异：实现 per-session busy gate、subtask/compaction 队列和 `MAX_STEPS_PROMPT` 等价的末步提示，并让结构化结果写入 turn metadata。
- 持久化映射到 `src/session.rs` 的 append-only JSONL `SessionStore`：`append_turn`（`L863-L863`）、`append_event`（`L1207-L1207`）以及 recovery marker。把 TS 的 `updateMessage/updatePart/setPermission/setAgentModel/touch` 映射为不可变 turn/event；工具启动前先写 intent，未有 terminal record 时按 unknown/interrupted 恢复，禁止隐式重放。
- 取消与并发映射到 `src/runtime.rs` 的 `CancellationToken`（`L49-L104`）、`Runner::try_cancel`（`L319-L319`）和 bounded worker loop；`headless.rs` 的异步路由已把 prompt/steer/cancel/user-shell 接到 `Agent`（相关 dispatch 在 `L4509-L4607`）。需验证 late cancel 不覆盖已完成结果，并将 TS 的 AbortSignal 传播到 provider/tool polling。
- `handleSubtask`、SessionTools 和 shell tool 映射到 `src/tool_runtime.rs` 的 `execute_tool_batch`（`L168-L180`）与现有 ToolRegistry；该模块已约束 32-call/256 KiB、来源顺序、整批取消和未知结果。新增 `TaskTool`/subagent adapter、输出附件与 synthetic summary，并保持工具参数不可改写。
- Permission.ask 对应 `src/approval.rs` 的 `ApprovalCoordinator::request`（`L170-L180`）和 `ApprovalPolicy`；`src/core.rs` 已有 tool admission、unknown outcome、recovery decision。需把 TS 的 session permission 合并、插件 before/after 事件和 `providerExecuted`/orphan 判定持久化为明确事件，拒绝必须返回 correlated tool result。
- Provider/model 选择对应 `src/providers/registry.rs` 与各 provider definitions（`src/providers/mod.rs` 的 `Protocol`/`ProviderDefinition`），模型不存在要返回可操作 suggestions；`structured output` 可利用 provider capability 的 `response_format/tool_choice`，但需补 AI SDK 式 schema 校验和一次性 `StructuredOutput` tool 语义。MCP、LSP、Image、ConfigMarkdown 文件引用在 Rust 目标模块中没有同名现成层，应分别落到受限 resource/file adapter，并为大小、MIME、路径和取消写测试。

## 未决问题

1. `Session.updateMessage/updatePart` 的事务/幂等保证不在本文件中，无法确认并发写入失败时是否原子回滚。
2. `SessionProcessor.process` 对 provider 重试、流式 finish 映射及 `providerExecuted` metadata 的精确契约由下层实现决定。
3. MCP `readResource` 返回的 `contents` 结构、LSP 行号是 0-based 还是 1-based、以及 Image resizer 的具体压缩策略需查各自模块才能完全确认。
4. `Permission.merge` 的规则冲突优先级和 `SessionRunState` 对 shell/loop 的忙状态细节不由本文件定义。

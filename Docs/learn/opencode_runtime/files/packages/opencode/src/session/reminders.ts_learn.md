# OC-024 — packages/opencode/src/session/reminders.ts

- source_id: `OC-024`
- item_id: `OC-024`
- source_path: `packages/opencode/src/session/reminders.ts`
- source_hash: `cb684f1b13330966c6d8b07b2279bb5e0f060211597fd8da172c591dfc7f8cd7`
- source_bytes: `3373`
- source_lines: `92`
- coverage: 已连续读取完整文件，字节范围 `1-3373`、行范围 `L1-L92`，包含全部 import、类型标注、导出符号、注释与 `export *`。

## 完整行为复盘

文件先导入 `path`、`SessionV1`、`Effect`、`Agent`、`FSUtil`、`InstanceState`、`RuntimeFlags`、`PartID`、`MessageV2`、`Session` 以及三个静态提示文本：`PROMPT_PLAN`、`BUILD_SWITCH`、`PLAN_MODE`（L1-L13）。其中 `MessageV2` 在本文件没有直接使用；其余服务用于识别会话状态、检查计划文件、生成合成消息部件。

唯一主函数是 `export const apply = Effect.fn("SessionReminders.apply")`（L15-L90）。它接收对象 `{ messages: SessionV1.WithParts[]; agent: Agent.Info; session: Session.Info }`（L15-L19），返回一个 `Effect`：成功值始终是原来的 `input.messages` 数组（通常是原地追加 part 后返回同一引用），失败则由所调用的 Effect 服务向上传播。函数通过 generator 依赖注入取得 `RuntimeFlags.Service`、`FSUtil.Service`、`Session.Service`（L20-L22），然后用 `findLast` 找最后一条 `role === "user"` 的消息（L23）。找不到用户消息时立即返回原数组，不创建提示、不访问文件系统或 Session 服务（L24），这是空历史和只有 assistant/tool 历史的边界行为。

1. 旧计划模式分支：当 `flags.experimentalPlanMode` 为假时进入 L26-L49。若当前 `input.agent.name === "plan"`，就直接向最后用户消息的 `parts` 推入一个合成文本 part（L27-L35）：`id` 由 `PartID.ascending()` 生成，`messageID`/`sessionID` 取用户消息元数据，`type` 为 `"text"`，文本为完整 `PROMPT_PLAN`，`synthetic: true`。这一步是内存数组原地变更；没有调用 `sessions.updatePart`，因此该分支是否持久化取决于上层消息管线。随后以 `some` 扫描所有消息，判断是否曾出现 `role === "assistant" && info.agent === "plan"`（L37）。若曾经计划代理回复且当前代理是 `"build"`，再向同一用户消息追加一个合成 `BUILD_SWITCH` 文本 part（L38-L46）。条件互斥于当前代理名的实际取值，但扫描保留了完整历史语义。分支最终原样返回 messages（L48），没有目录检查、超时、重试或并发等待。

2. 实验计划模式的“计划切换到构建”：L51 先取最后一条 assistant 消息。若当前代理不是 `plan` 且最后 assistant 的 `info.agent` 是 `plan`（L52），函数读取 `InstanceState.context`（L53），用 `Session.plan(input.session, ctx)`计算计划文件路径，再用 `fsys.existsSafe(plan)`安全检查存在性（L54-L55）。随后调用 `sessions.updatePart` 创建合成文本 part（L56-L65），文本总是以 `BUILD_SWITCH` 开头；若计划文件存在，追加两个换行及 `A plan file exists at ${plan}. You should execute on the plan defined within it`，否则仅为 `BUILD_SWITCH`（L60-L64）。`updatePart` 返回的 part 才被推入 `userMessage.parts`（L66），成功后返回原数组（L67）。路径存在检查是只读副作用；更新 part 是会话层写入副作用。任一 context、路径计算、存在性检查或更新失败都会使 Effect 失败并阻止本次返回。

3. 实验计划模式的“进入计划代理”：若 L52 条件不成立，L70 的守卫 `if (input.agent.name !== "plan" || assistantMessage?.info.agent === "plan") return input.messages` 过滤两类情况：当前不是 `plan`，或当前是 `plan` 但上一条 assistant 已经来自 `plan`。因此只有“当前是 plan 且上一条 assistant 不来自 plan（也可以没有 assistant）”继续执行（L70）。函数再次读取 context、计算 plan 路径并检查存在性（L72-L74）。若计划文件不存在，先对 `path.dirname(plan)`执行 `fsys.ensureDir`，并用 `Effect.catch(Effect.die)`把失败转为不可恢复的 defect（L75）；它只确保父目录，不创建计划文件内容。随后调用 `sessions.updatePart`（L76-L87），把 `PLAN_MODE` 中的 `${planInfo}`替换为动态说明：存在时要求读取并用 edit tool 增量修改；不存在时要求用 write tool 创建，具体字符串由 L81-L85 的条件表达式决定。返回的 part 推入用户消息（L88），最后返回原数组（L89）。

`export * as SessionReminders from "./reminders"`（L92）提供命名空间再导出，使调用方可通过 `SessionReminders.apply`访问；没有其他导出函数或类型。

## 状态、取消、恢复与副作用

此文件没有显式取消 token、超时、重试循环或并发控制；`Effect.fn`只包装 Effect 组合，取消语义由运行时在等待 service Effect 时决定。`PartID.ascending()`承担合成 part 的递增 ID 生成，但本文件不声明锁或去重策略；若同一消息被重复调用，旧分支和实验分支都可能再次追加提醒。旧分支先直接 `push`，实验分支先 `sessions.updatePart`后 `push`，两者均不是事务：更新成功而后续数组操作失败（理论上的内存异常）不会回滚，服务失败则不会执行后续 push。`fsys.existsSafe`只读文件系统；实验进入计划代理时的 `ensureDir`会创建计划文件父目录，是唯一明确的文件系统写副作用。代码从不写计划文件本体、删除文件、调用 provider 或网络。

恢复依赖上层重新从消息历史调用 `apply`；函数本身没有“已提醒”标记、幂等键或持久化检查，因此恢复/重放可能重复合成 part。实验切换构建时仅依据最后一条 assistant；历史中更早的 plan 回复不会触发该分支。实验进入计划时若 `plan` 文件已存在，提示内容改变但不会读取文件内容。`Effect.catch(Effect.die)`使目录创建错误成为 defect，而 `existsSafe`、`updatePart` 的普通失败保持 Effect 错误；不存在专门错误码。没有共享可变状态写入（除传入的 `messages`/`parts` 原地变更），所以并发安全取决于调用方是否串行拥有消息对象。

## 源内测试与行为判据

源内未包含测试；同目录检索只发现 `prompt.ts` 对 `SessionReminders` 的调用以及 `effect/runtime-flags.test.ts` 对 `experimentalPlanMode` 配置的测试，没有针对 `reminders.ts` 分支的断言。可独立验证的判据如下：

- 无 user 消息：`apply` 成功返回同一 `messages` 引用，所有 service 均不应被调用。
- `experimentalPlanMode=false` 且 agent 为 `plan`：最后 user 的 parts 增加一个 `synthetic` 文本，文本等于 `PROMPT_PLAN`，ID/消息会话 ID正确。
- 同配置下存在任意 plan assistant 且 agent 为 `build`：增加 `BUILD_SWITCH`；无 plan assistant 则不增加。
- `experimentalPlanMode=true`、上一 assistant 为 plan、当前非 plan：调用一次 `existsSafe` 与 `updatePart`；存在文件时文本包含绝对/计算出的 `plan` 路径和执行说明。
- `experimentalPlanMode=true`、当前 plan 且上一 assistant 非 plan：不存在计划文件时先 `ensureDir(dirname(plan))`，文本必须把 `${planInfo}`替换为“创建计划”的说明；存在时替换为“读取并增量编辑”的说明。
- `updatePart` 或目录创建失败时 Effect 不成功；重复调用应能观察到没有内置去重而产生重复 part。

## zenpi Rust 映射

建议新增 `src/session_reminders.rs`（或放入 `core.rs` 的独立私有模块）实现纯决策函数加副作用适配层。现有 `src/core.rs` 的 `TurnRole::{System,User,Assistant,Tool}`、不可变 `Turn { id,parent_id,role,content,metadata }`位于 L131-L152，构造器在 L154-L175；可把 TS 的 synthetic part映射为带 `metadata: {"synthetic":true,"reminder_kind":...}` 的 `TurnRole::System` 或 `User` 子记录，但需明确 zenpi 当前一条 `Turn`只有单个 `content`，不能直接等价于 `parts` 数组。`Turn::validate`限制 ID、父 ID及内容长度（L177-L204），Rust 映射应在生成 reminder 前验证长度。

`src/core.rs` 的 `TurnInputRequest`及模式字段在 L207-L240，提交边界与拒绝原因在 L3220-L3337；`start_new`会构造并持久化 user turn（L3364-L3406），`process_with_cancel_and_events`在 L5327-L5349串起提交和 provider turn。建议在 `start_new`完成 user turn入 journal 后、provider 请求准备前调用 `SessionReminders::apply`，根据一个显式 `AgentMode::{Plan,Build}`和 `experimental_plan_mode`配置返回待注入的 system reminder；build/steer 重发路径也应以“最后 assistant agent”元数据判断，避免仅看 role。

持久化落点是 `src/session.rs`：文件头说明 append-only JSONL、崩溃后恢复完整记录（L1-L5）；`SessionStore::append_turn`先验证、写盘，再更新内存 projection，失败不改变内存（L861-L890）。若 synthetic reminder需可恢复，建议追加独立 `Turn`或 `event`记录，并保存 `synthetic`、`reminder_kind`、`plan_path`；若仅是每次 provider 请求的瞬时 system prompt，则不要写 journal，并以可重复纯函数生成。`SessionStore`已有 event projection（L144-L160、L1205-L1208附近），可用于审计“提醒已注入”，但必须增加幂等键（session、user turn、kind）以避免恢复重复。

取消与并发对照 `src/runtime.rs`：`CancellationToken`是可观察、幂等取消，且有完成标记防止晚到 cancel 改写成功结果（L49-L99）；运行时事件包含 `CancelRequested`、`Completed`、`Closed`（L171-L193），提交/取消是有界非阻塞队列（L308-L350）。reminder 生成应在取消检查点前后保持原子：若目录检查或 update 期间取消，返回 cancelled 且不要发布半完成的 synthetic 记录；不要把取消当成文件回滚。运行时本身没有本 TS 的“重试”，重试应沿用 `src/session.rs` 的 interrupted operation 语义而不是重复追加提醒。

协议映射在 `src/protocol.rs`：`StdioRequest`携带 `text/message/mode/expected_turn_id`等字段（L152-L209），`prompt`解析为 bounded text、默认 `TurnMode`并校验 attachments（L377-L395），`cancel`要求有效 target ID（L415-L423）。无需新增 provider wire 字段；若要让客户端选择 plan/build，建议新增严格枚举字段并在协议层拒绝未知值。`src/headless.rs`只负责 JSONL 传输和请求关联（L1-L5、L553-L625），应调用 core 的 reminder 逻辑而不是自行拼提示；其异步事件按请求隔离缓冲（L823-L869），可承载 reminder 注入事件但不应把它伪装成 provider delta。

副作用与工具层对照：TS 提醒不需要 approval。zenpi 的 `src/approval.rs`把审批请求持久化前后的协调、取消拒绝与 cancellation epoch定义在 L122-L187、L405-L445，reminder 的目录创建不应绕过现有 host policy；若计划路径被视为写操作，应在调用 `ensure_dir`前走对应权限边界。`src/tool_runtime.rs`规定批量工具取消、串并行和结果相关性（L157-L176、L189-L249），但本功能不是 tool batch，不应复用并行执行器。

`src/providers/**`（入口 `src/providers/mod.rs` L1-L10，协议枚举与 provider definition 在 L13-L50、L146-L159）只描述 wire protocol、认证和能力路由；不要把 plan/build reminder硬编码到 `anthropic`、`openai`等 provider，统一在 core 的请求准备层注入，确保所有 provider 行为一致。可执行验证顺序：为 `SessionReminders`写纯函数单测覆盖五个判据；用临时 `SessionStore`验证 append/重放幂等；用 `CancellationToken`在 exists/ensure/update 前后注入取消；最后通过 `run_headless`发送 prompt、steer、cancel JSONL检查事件和 journal。

## 未决问题

- `Session.plan(input.session, ctx)`的确切路径规则、是否始终位于 workspace 内，无法从本文件确认。
- `FSUtil.existsSafe`对权限错误、符号链接和竞态的具体返回策略未在本文件定义。
- `Session.Service.updatePart`是否立即持久化、是否幂等、失败后是否自动重试，需查看其实现。
- `PartID.ascending()`的线程安全、进程重启后的单调性和跨 session 唯一性未由本文件保证。
- `prompt.ts`何时、每轮调用多少次 `SessionReminders.apply`未在本文件确认，这直接影响重复提醒风险。

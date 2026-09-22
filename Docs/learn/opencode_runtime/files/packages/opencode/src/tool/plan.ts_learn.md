# OC-045 — packages/opencode/src/tool/plan.ts

- source_id/item_id：OC-045
- source_path：packages/opencode/src/tool/plan.ts
- source_hash：cbe103563700b770bf77ef9fd6ea3b1e97eed6aa996a8ce28342ff0ce6ee3719
- source_bytes：3131
- source_lines：79
- coverage：已从首字节到末字节完整读取（bytes 1-3131），覆盖 lines 1-79，包含全部 import、注释、类型引用、导出符号与闭包实现。

## 完整行为复盘

1. 文件依赖与导出（L1-L11）。导入 Node 的 path，Effect 的 Effect/Schema，工具定义框架 Tool，以及 SessionV1、Question、Session、MessageV2、Provider、InstanceState、MessageID/PartID。MessageV2 在本文件没有直接使用，属于保留依赖或类型侧副作用；plan-exit.txt 的默认导出作为用户可见描述。文件没有定义新的 class/type/interface。
2. Parameters（L13）。导出一个 Schema.Struct({})，因此 plan_exit 的参数必须是空结构；任何必填业务参数都不存在，边界是调用方仍需提供可被 schema 接受的对象。
3. PlanExitTool 注册（L15-L21）。导出 Tool.define("plan_exit", Effect.gen(...))。构造阶段从 Effect 环境依赖注入 Session.Service、Question.Service、Provider.Service；缺失任一服务会使工具构造/执行失败。工具 ID 固定为 plan_exit，供 registry 暴露给模型（L15-L20）。
4. 工具元数据与执行入口（L22-L27）。返回对象的 description 是 EXIT_DESCRIPTION，parameters 是上面的空 schema。execute(_params: {}, ctx: Tool.Context) 忽略参数，仅使用 ctx；每次调用先取得 InstanceState.context（L25-L28），所以工作树/实例状态是运行时必需依赖。
5. 计划路径计算（L28-L30）。通过 session.get(ctx.sessionID) 读取当前会话信息，再调用 Session.plan(info, instance) 取得计划路径，最后用 path.relative(instance.worktree, ...) 变为相对工作树路径 plan。会话不存在、实例状态不可用、路径解析失败都会沿 Effect 错误通道传播；相对路径可能包含 ..，源码没有额外规范化或存在性检查。
6. 用户确认询问（L30-L45）。question.ask 发送一个只有一题的选择题，题干插入 plan：计划完成后是否切换到 build agent 并开始实现；header 为 Build Agent，custom: false，选项恰为 Yes/No。请求作用域是 ctx.sessionID；只有 ctx.callID 有值时才附加 {messageID: ctx.messageID, callID: ctx.callID} 工具关联元数据，否则传 undefined（L30-L44）。等待是顺序的异步 Effect；源码没有并发询问、轮询或超时参数。
7. 拒绝分支（L46）。仅当 answers[0]?.[0] === "No" 时抛出 new Question.RejectedError()。因此空回答、缺少第一题、或任何不是精确字符串 "No" 的值不会拒绝，会继续切换流程；这是一个重要的宽松边界。此错误发生在持久化新消息前。
8. 读取历史并选择模型（L48-L51）。调用 session.messages({sessionID: ctx.sessionID})，并以 Effect.orDie 将可恢复业务错误提升为致命缺陷。使用 findLast 找到最后一个同时满足 item.info.role === "user" 且 item.info.model 真值的消息；若找到则沿用其 model，否则调用 provider.defaultModel()。因此模型选择优先最近的带模型用户消息，历史为空或模型缺失时依赖 provider 默认值；没有显式模型校验、重试或多模型并行。
9. 构造并写入 build 用户消息（L53-L61）。创建 SessionV1.User：id: MessageID.ascending() 单调生成，sessionID 使用当前上下文，role: "user"，time.created: Date.now()，agent: "build"，model 为上一步结果。先 session.updateMessage(msg)，写入失败即由外层 Effect.orDie 终止。该操作是持久化/会话副作用，源码没有先检查重复 ID 的本地逻辑。
10. 构造并写入合成文本 part（L62-L69）。PartID.ascending() 生成 part ID，关联新消息和会话，type: "text"，文本固定为 “The plan at <plan> has been approved, you can now edit files. Execute the plan”，并标记 synthetic: true；satisfies SessionV1.TextPart 在编译期约束形状。随后 session.updatePart 持久化，失败同样被 Effect.orDie 视为致命错误。没有实际文件编辑，文本只是下一个 agent 的合成输入。
11. 成功返回值（L71-L76）。返回 title: "Switching to build agent"、固定 output、空 metadata: {}。整个执行 Effect 最外层 .pipe(Effect.orDie)（L76），所以问答拒绝、会话读取/写入、默认模型失败都不会转换成结构化工具错误，而会进入运行时的致命失败语义。

## 状态、取消、恢复与副作用

该文件没有显式取消 token、超时、重试、锁或后台线程。Effect 的取消只能在 InstanceState.context、session.get、question.ask、历史读取或两个持久化写入的可取消边界生效；取消后不会回滚已完成的 updateMessage 或 updatePart，也没有补偿事务。流程严格串行：先询问，再读历史/选模型，再写 message，最后写 part；没有并发语义，调用之间也没有去重或幂等检查。拒绝发生在任何新消息写入之前。成功会产生两个会话外部副作用（user message、synthetic text part），但不会直接修改工作树文件、启动进程或调用 provider 推理。恢复依赖会话存储本身：若进程在两次写入之间退出，源码没有定义修复标记；下次运行可能看到只有 message 没有 part 的中间状态。模型回退到 provider.defaultModel()，无持久化的模型选择更新。

## 源内测试与行为判据

源文件和 src/tool 同目录未包含针对 plan_exit 的测试；检索只看到 registry.ts 在 L107 导入并注册 PlanExitTool。可独立验证的判据如下：用空对象参数调用工具应进入一次 Question.Service.ask；选择精确 No 必须返回 Question.RejectedError 且不新增消息；选择 Yes（以及源码允许的非 No 异常回答）后，应读取最后一个带模型的 user message 或调用 defaultModel，依次写入 agent=build 的 user message 和 synthetic text part，文本包含相对计划路径；最终结果的 title/output/metadata 必须与 L71-L75 完全一致。还应验证无 ctx.callID 时 question 请求的 tool 字段为 undefined。

## zenpi Rust 映射

- 会话承载可落在 src/session.rs 的 SessionStore（L146）及 append_turn（L863）、append_event（L1207）、events（L1223）。zenpi 的持久化是追加式 JSONL，append_turn 先持久化再更新内存投影；建议新增一个经过校验的 PlanApprovalTurn/BuildHandoff 记录，原子地表达“批准”和合成提示，避免 TS 中 message/part 两次写入造成的半完成状态。
- agent 状态可映射到 src/core.rs 的 Agent（L403）、AgentSnapshot（L328）和 AgentError（L355）。增加 Agent::request_plan_exit 或 Agent::approve_build_handoff：输入 session id、计划路径和确认结果；确认成功后设置 build 区域/agent 状态并追加会话记录。现有 set_model（L1167）及 restore_model_selection（L1277）可支持“沿用最近模型，否则 backend 默认模型”，但应明确把默认模型读取做成纯查询。
- 工具注册与结果应放进 src/tools.rs 的 ToolDefinition/ToolRegistry（定义约在 L505、L1057），调用批处理由 src/tool_runtime.rs:execute_tool_batch（L168）承载。建议定义无参数 plan_exit，返回 ToolResult::Success 的 title/output/metadata；用户拒绝应映射为可识别的拒绝错误，不能把正常拒绝伪装成 panic。与 TS 的 Effect.orDie 相比，Rust 应保留 Result 的结构化错误。
- 交互确认可复用 src/approval.rs:ApprovalCoordinator::request_response（L185）与 cancel_all/emergency_cancel（L409-L445），或定义专用 PlanExitApproval。差异是 zenpi approval 主要围绕工具副作用策略（ApprovalPolicy），而本源是面向用户的二选一问题；应在 ApprovalRequest.origin 中区分 plan handoff，禁止 remembered tool grant 误替代一次性 build 确认。
- 协议入口可扩展 src/protocol.rs 的 Command（L214）和 StdioRequest 的 kind/text/session_id 字段；建议加入 plan_exit/build_handoff 命令，校验 session、计划相对路径和确认值。现有 prompt/steer/cancel 解析及 target_id 校验可作为边界模板。若以既有 handoff 命令表达，应把“用户确认”和“实际切换”拆成两个可审计阶段。
- 取消与并发应接入 src/runtime.rs:CancellationToken（L56-L86）以及工具批次的取消检查（src/tool_runtime.rs L168-L313）。在等待用户回答、读取历史、写入两条记录前后检查 token；取消不回滚已提交 journal，返回 cancelled，并保证不会启动 build turn。runtime 的 worker 完成/取消竞态应保持单一终态。
- src/headless.rs 已提供会话 owner、重连和事件投递边界；实现 handoff 时应在 owner 持有的 Agent 上串行更新，向 headless event 输出 admission/approval/terminal 事件。不要直接从工具线程修改共享 session。
- src/providers/** 仅负责 backend/model；不要把 plan_exit 的确认逻辑放入 provider。provider 只需提供当前模型/默认模型查询，和源文件中的 Provider.defaultModel() 对应。
- 可执行差异清单：① zenpi 当前没有 TS 的 Question.Service 和 SessionV1.User/TextPart 二层消息 API，需要设计一个 Rust 会话事件/turn schema；②源实现将拒绝抛为致命 Effect，zenpi 应结构化返回并可被 headless 协议观察；③源实现接受任何非精确 No 的回答，Rust 应明确采用兼容还是严格枚举并写测试；④源实现两次写入可能留下中间态，Rust 建议单次 append handoff 或可恢复的 operation intent；⑤路径应限制为相对 workspace 的安全表示，避免直接复刻可含 .. 的 path.relative。

## 未决问题

1. Session.plan(info, instance) 的具体计划文件命名、是否保证存在以及是否总在 instance.worktree 下，无法由本文件确认。
2. Question.Service.ask 返回数组的完整类型、空回答的上游约束、以及 RejectedError 被运行时如何渲染，源文件未定义。
3. session.updateMessage/updatePart 的幂等性、写入格式和跨进程并发策略未在本文件确认。
4. agent: "build" 是否会自动触发后续 provider turn，还是仅作为下一轮提示，需要查看调用方/调度器才能确定。
5. MessageV2 的未使用导入是否由编译器配置豁免，无法从本文件确认。


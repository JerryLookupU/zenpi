# OC-046 — packages/opencode/src/tool/question.ts

元信息块：

- source_id: OC-046
- item_id: OC-046
- source_path: packages/opencode/src/tool/question.ts
- source_hash: 15a90d87b9b928d91c1558ba12b229a919c575e79f6af6b6a5809e439f36121e
- source_bytes: 1528
- source_lines: 44
- coverage: 已连续读取字节 0–1527、行 1–44（包含全部 import、注释、类型、导出和结尾括号）。

## 完整行为复盘

- import 依赖（L1-L4）：引入 Effect、Schema，把本目录的 Tool 定义、../question 服务命名空间和 ./question.txt 描述文本接入。该文件没有自行实现 UI 或网络传输。
- Parameters（导出，L6-L8）：Schema.Struct 只接受一个 questions 字段；其类型为可变的 Schema.Array(Question.Prompt)，并标注描述 “Questions to ask”。数组可以为空（本文件未加非空约束），元素结构和字段约束由 Question.Prompt schema 决定。Schema.mutable 影响解码后的可变性，不会自动复制或排序数组。
- Metadata（局部类型，L10-L12）：工具结果元数据固定为 { answers: ReadonlyArray<Question.Answer> }；答案是按问题顺序排列的字符串数组集合，类型层面只读，运行时数据仍由服务提供。
- QuestionTool（导出，L14-L18）：调用 Tool.define 注册 id question，参数 schema 是 Parameters，结果元数据是 Metadata，所需环境是 Question.Service。初始化阶段通过 yield* Question.Service 获取服务实例（L16-L18）；若环境缺失，初始化无法成功，而不是返回一个降级工具。
- 工具定义对象（L19-L23）：description 使用导入的 DESCRIPTION 原文，parameters 暴露 Parameters。execute 接收已解码的 params 和 Tool.Context<Metadata>；参数类型由 Schema.Schema.Type<typeof Parameters> 推导，避免重复声明。
- execute 的询问调用（L22-L28）：先调用 question.ask，传入 ctx.sessionID 和 params.questions。当 ctx.callID 为真值时才传 tool: { messageID: ctx.messageID, callID: ctx.callID }，否则传 undefined；因此空字符串 call ID 不会建立工具关联。调用是一次整体请求，函数会等待用户回答后才继续。
- 答案格式化（L30-L32）：按 params.questions 原顺序 map，以同索引 answers[i] 配对。若该答案数组存在且 length 大于 0，则用 join(", ") 拼接标签；缺失、空数组或其他假值均输出精确字符串 Unanswered。每项格式为 “问题文本”=“值”，项间用逗号和空格连接；问题文本和答案未做引号转义，因此包含引号时输出仍可能产生歧义。
- 成功结果（L34-L40）：title 为 Asked N question；只有 params.questions.length > 1 才追加 s，所以 0/1 分别是单数形式。output 固定为 User has answered your questions: 加上 formatted，再接 You can now continue with the user's answers in mind.；metadata.answers 原样返回服务答案，不返回格式化字符串。
- 错误边界（L41）：整个 Effect.gen 通过 Effect.orDie，所以 Question.Service 的可恢复失败（例如用户拒绝）在此工具边界被转为 defect，而不是在该返回类型中暴露为业务错误。Tool.define 外层还会先按 Parameters 解码；解码失败由通用工具层产生参数错误，question.ts 本身不捕获、不重试。
- 并发语义：单次 execute 内部顺序是 ask → 等待答案 → 格式化 → 返回。多个工具调用可同时进入 Question.Service；服务为每次 ask 分配递增 QuestionID，在 pending map 中分别等待，通过 request ID 回复，不按完成时间重排答案。服务的 ensuring 会在成功、拒绝或中断后删除 pending 项（src/question/index.ts L87-L112）。

## 状态、取消、恢复与副作用

本文件没有静态可变状态、文件写入、数据库、网络请求或 provider 直接调用；唯一外部副作用是触发 Question.Service.ask，由服务发布询问事件并使调用挂起（源文件 L24-L28）。Question.Service 将 sessionID、questions、tool 放入内存 pending map 并发布 Asked 事件，回复时发布 Replied，拒绝时发布 Rejected（src/question/index.ts L97-L148）。用户拒绝会使 ask 失败，随后被 L41 的 Effect.orDie 变成终止性 defect。

源文件没有显式超时、重试、恢复或持久化逻辑，也没有读取 ctx.abort；取消是否能打断等待取决于 Effect 运行时对 Deferred.await 的中断传播。服务终结器会把残留 pending 请求统一失败为 RejectedError 并清空 map（src/question/index.ts L74-L80），这属于进程或实例关闭时的清理，不是答案恢复。Tool.define 的通用层可能对 output 做截断，但本文件返回的 metadata.answers 不由格式化输出替代。

## 源内测试与行为判据

源文件自身未包含测试；同目录 test/tool/question.test.ts 提供了可核对判据。测试 L44-L69 用合法单问题启动 tool.execute，等待 Question.Service.list() 出现 pending，再以 question.reply 返回 [["Red"]]，断言标题为 Asked 1 question。L71-L90 验证 header 超过 12 字符仍可执行，并断言 output 包含问题文本与 Dog 的配对。L93-L132 的超长 header/label 校验测试已注释移除，不能把它当作当前保证。

可独立验证的判据：给一个问题并回复一个非空标签，结果标题应为单数且 output 按问题文本和标签配对；回复空数组或缺失答案时对应值必须是 Unanswered；两个问题的标题必须是 Asked 2 questions；没有 callID 时请求的 tool 关联必须为 undefined；重复或未知 request ID 的回复应由 Question.Service 报 NotFoundError，而不是生成成功结果。

## zenpi Rust 映射

建议新增 src/question.rs（或 src/tool_runtime.rs 的独立子模块）：

- 类型落点：QuestionPrompt { question, header, options, multiple }、QuestionAnswer(Vec<String>)、QuestionRequest { id, session_id, questions, tool }、QuestionError::{Rejected, NotFound, Cancelled}、QuestionService。用 serde 加显式 validate 复刻 Question.Prompt 与 Answer 的数组语义；答案长度不要假定与问题数相等，格式化时使用安全索引。
- 函数落点：QuestionService::ask 创建递增 ID、登记 pending、发出 question_asked；reply/reject 以 request ID 唤醒等待者并清理 pending；list 返回当前 pending。QuestionTool::execute 负责调用 ask、按索引生成 Unanswered、构造 Asked N question(s) 和 output。Rust 应返回 Result<ToolResult, QuestionError>，明确记录 TS Effect.orDie 把拒绝转 defect 的差异。
- src/core.rs：Agent 已持有 session 与 input_port（L402-L430、L598-L625），可注入 QuestionService；将 question 事件作为会话级交互，而不是 TurnInputRequest（L207-L240）这种新的用户 turn。工具调用完成后仍由 core 负责把结果回写当前 turn。
- src/headless.rs：现有异步请求/响应和 input queue 有 pending ticket、回执及关闭时未知结果处理（L5253-L5320、L5609-L5629）。可增加 question_request/question_reply 事件流和 session 关联，复用 ticket 的异步回执模式；关闭时应拒绝未答问题并输出明确终态。
- src/protocol.rs：现有 InputQueueAction 仅承载文本 enqueue/edit/cancel/list/configure（L1035-L1107），InputQueueRequest 严格校验版本、ID 和大小（L1119-L1158）。建议新增严格的 QuestionReply { request_id, answers } 与 QuestionReject 命令，限制 ID、答案数组和总字节数；不要把结构化问题偷偷降级成普通 Prompt。
- src/session.rs：append_event 是可扩展的不透明事件入口（L1205-L1220），可持久化 question_asked、question_replied、question_rejected，并在恢复时重建未完成 pending。append_turn 的“先持久化、成功后更新内存”原则（L861-L884）可作为答案提交的耐久边界。
- src/runtime.rs：CancellationToken 是协作式、幂等取消，后台 runner 对 queued/active job 有明确终态（L49-L99、L156-L190）。ask 应绑定 token；取消需从 pending 移除并返回 Cancelled，不能把取消当作用户拒绝，也不能假设线程会被强杀。
- src/tool_runtime.rs：现有批处理按调用顺序产生结果、限制批量大小，并在取消后阻止后续调用（L150-L176、L219-L249）；question 属于只读、可等待的交互工具，建议作为单次 ToolCall 结果接入，避免把多个问题误当成可并行副作用。输出仍应保留结构化 answers metadata，不能只保存拼接后的文本。
- src/approval.rs：ApprovalCoordinator 的 pending/visible/accepted 是“是否允许副作用”的决策流（L127-L151、L167-L245），与用户回答问题不同；不要复用 approval decision 表示答案。若问答工具被纳入权限策略，只在调用前做工具许可，答案通道仍由 QuestionService 管理。
- src/providers/**：providers/mod.rs 仅定义 provider 协议、认证头和能力路由（L13-L49、L136-L160），各 provider 文件只提供 endpoint definition；question 不应下沉到 OpenAI/Anthropic/Google wire codec。可在 provider 无关的 tool schema 注册处暴露 question，并用现有 provider 端到端测试验证模型收到的是工具定义而非 provider 特有消息。

主要差异清单：TS 使用 Effect 环境注入和内存 Deferred，Rust 需显式 Arc<Mutex<HashMap>> 加 Condvar 或 oneshot；TS 不持久化问答，zenpi 的 session 是 append-only durable journal；TS 不检查 abort/超时，zenpi 必须用 CancellationToken；TS 的 Effect.orDie 隐藏了 RejectedError 的业务返回，Rust 应保留机器可判定错误码；TS 只按 answers[i] 格式化且不转义引号，Rust 实现应固定相同外显格式，同时对 JSON/文本边界做严格编码。

## 未决问题

- Question.Prompt 的最终约束由 @opencode-ai/schema/question-v1 提供；本文件只能确认其被引用，无法单独确认运行时是否会拒绝空 options、超长 header 或缺失可选字段。
- Effect.orDie 在当前应用运行时是否会被统一捕获并转成工具错误事件，需结合工具执行器和部署层验证；源文件本身没有定义该映射。

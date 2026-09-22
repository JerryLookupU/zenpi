# OC-054 — packages/opencode/src/tool/task.ts

## 元信息

- source_id/item_id：OC-054
- source_path：packages/opencode/src/tool/task.ts
- source_hash：db09fa5868ad3ecfdd83aa2bb7243f85e9e2cd13d0b19fd6f307f7a337b2b36e
- source_bytes：14200
- source_lines：371
- coverage：已按源文件顺序读取完整内容；字节范围 1-14200，行范围 L1-L371（含导入、注释、类型、常量、导出和实现）。

## 完整行为复盘

1. **依赖、接口与静态提示（L1-L52）**  
   文件导入 `Tool`、`DESCRIPTION`、`ToolJsonSchema`、`SessionV1`、`BackgroundJob`、`Session`、`SessionID/MessageID`、`MessageV2`、`Agent`、`deriveSubagentSessionPermission`、`SessionPrompt`、`Config`、Effect 原语、`EffectBridge`、`RuntimeFlags`、`Database`。`TaskPromptOps`（L18-L22）是 `TaskTool` 依赖的运行时桥：`cancel(sessionID)` 取消子会话，`resolvePromptParts(template)` 将模板解析为 prompt parts，`prompt(input)` 执行一次子会话提示并返回 `SessionV1.WithParts`。三者均返回 `Effect`，因此调用链可组合、可中断。`id` 固定为 `task`（L24）；`BACKGROUND_DESCRIPTION`（L25-L30）补充工具描述，强调前台默认、后台异步及不得轮询；`BACKGROUND_STARTED`（L31-L35）和 `BACKGROUND_UPDATED`（L36-L41）是返回给模型的操作指令文本。
   `BaseParameterFields`（L43-L52）要求非空语义上的字符串字段 `description`、`prompt`、`subagent_type`，可选 `task_id`（仅用于恢复既有子 agent session）和 `command`（触发命令，仅作元数据/输入保留）。`BaseParameters`（L54）不含后台开关；导出的 `Parameters`（L56-L62）在其上增加可选布尔 `background`，默认由后续 `=== true` 判定为 false。

2. **`renderOutput`（L64-L79）**  
   输入为 `{sessionID, state, summary?, text}`，其中 state 只能是 `running|completed|error`。错误态使用 `task_error` 标签，其他状态使用 `task_result`；输出是带 `<task id="..." state="...">` 外壳、可选 `<summary>`、正文标签及正文的换行拼接字符串。没有 XML 转义或长度截断逻辑，ID、summary、text 的安全/大小由上游保证；这也是前后台、成功失败的统一可核对格式。

3. **导出 `TaskTool` 的服务装配（L81-L91、L361-L369）**  
   `Tool.define("task", Effect.gen(...))` 在构造时取得 `Agent.Service`、`BackgroundJob.Service`、`Config.Service`、`Session.Service`、`Scope.Scope`、`RuntimeFlags.Service`、`Database.Service`。返回对象导出 `description`、`parameters`、可选 `jsonSchema`、`execute`。实验开关开启时 description 追加后台说明且不提供旧 `BaseParameters` JSON schema；关闭时 description 仅为 `DESCRIPTION`，`jsonSchema` 由 `ToolJsonSchema.fromSchema(BaseParameters)` 生成，从而在非实验模式隐藏 `background` 参数。`execute` 调用 `run(params, ctx).pipe(Effect.orDie)`，Effect 失败会被转成致命错误；业务错误因此必须在 run 内尽早返回。

4. **`run`：前置检查与子会话建立（L92-L198）**  
   - 读取配置（L96），`runInBackground = params.background === true`（L97）。后台实验开关关闭时立即失败，错误文本要求设置 `OPENCODE_EXPERIMENTAL_BACKGROUND_SUBAGENTS=true`（L98-L102）。
   - 从 `ctx.sessionID` 取父 session；沿 `parentID` 链逐级 `sessions.get` 并计数 depth（L104-L110）。当 depth >= `cfg.subagent_depth ?? 1` 时失败（L111-L117）；默认 1 允许直接子任务，但禁止继续嵌套。父链读取失败会直接失败。
   - 未设置 `ctx.extra.bypassAgentCheck` 时调用 `ctx.ask`（L119-L129），权限名为 `task`，pattern 为请求的 `subagent_type`，always 为 `*`，metadata 带 description/type。权限询问先于 agent 类型存在性检查。
   - `agent.get(subagent_type)`（L131-L134）找不到时失败并明确指出非法 agent 类型。
   - 若提供 `task_id`，尝试 `sessions.get(SessionID.make(...))`；任何 Cause 都被吞掉并视为不存在（L136-L138），因此无效/过期 task_id 会静默走新建路径。随后用 `deriveSubagentSessionPermission` 从父权限和 agent 规则派生子权限（L139-L142）。若子 agent 未声明 `todowrite` 或 `task`，分别追加全量 deny；配置中的 `experimental.primary_tools` 也全部追加 deny（L143-L155），并用精确三元组去重。
   - `nextSession` 复用找到的 session，或以 `parentID=ctx.sessionID`、标题 `description (@name subagent)`、agent 名和合并权限创建新 session（L156-L172）。创建发生在 promptOps 存在性检查之前，故后续失败可能留下已持久化子 session。
   - 从父消息读取 `MessageV2.get` 并注入当前 Database（L174-L177）；消息不是 assistant 时失败（L178）。继承父消息的 variant（L179）。模型优先使用 `next.model`，否则继承父 `modelID/providerID`（L181-L184）。组装 metadata：父 session、子 session、模型；后台模式额外写 `background: true`（L185-L190）。通过 `ctx.metadata` 发布标题和 metadata（L192-L195）。最后要求 `ctx.extra.promptOps` 存在且可转为 `TaskPromptOps`，否则失败（L197-L198）。

5. **`runTask`：一次子 agent 执行（L200-L225）**  
   先解析 `params.prompt` 为 parts（L201），再调用 `ops.prompt`：新 `MessageID.ascending()`、目标 session、模型、仅在继承模型时传 variant、agent 名和 parts（L202-L212）。若返回 assistant error，优先取 error.data.message 字符串，否则取 error.name，并包装为带 task_id 的失败（L213-L219）。否则查找最后一个 error 状态的 tool part，存在则同样失败（L220-L223）。成功值为最后一个 text part 的 text；没有文本时返回空字符串（L224-L225）。函数没有内部重试或并发 fan-out，一个 task invocation 对应一个 prompt effect。

6. **后台结果注入与通知（L227-L265）**  
   `inject(state,text)`（L227-L254）重新读取当前父 session，使用其 agent（回退 `ctx.agent`）和原 variant，构造 synthetic text part，将 `renderOutput` 包在父会话的新 prompt 中；该 prompt 被 `Effect.ignore` 丢弃错误并 `forkIn(scope,{startImmediately:true})`，所以通知异步发送，不阻塞任务终态。`notify(jobID)`（L256-L265）等待 `background.wait`，完成态注入 output，error 态注入 error，其他状态不注入；等待本身在 scope 中立即 fork。后台任务完成后，父 session 可能已变化，注入时取最新 parent agent。

7. **启动、更新与返回（L267-L319）**  
   `background.extend({id: nextSession.id, run: runTask()})` 若返回 true，表示已有同 ID 后台 job 被追加/恢复上下文；立即返回 `BACKGROUND_UPDATED`，metadata 标记 background 和 jobId（L267-L281），不会再次 start。否则 `background.start`（L284-L297）登记 id/type/title/metadata，并配置 `onPromote`：同时更新调用者 metadata 为后台态并注册 `notify`；`run` 在被中断时调用 `ops.cancel(nextSession.id)`。`backgroundResult`（L299-L314）统一返回 running 外壳，区别只在 summary。若本次请求显式 `background=true`，先 `notify(info.id)` 再立即返回 started 结果（L316-L319）；通知等待在后台 scope 中运行，主调用不等待完成。

8. **前台等待、提升与取消（L321-L358）**  
   前台创建 `EffectBridge`（L321），得到子 session cancel effect（L322），`onAbort` 通过 `runCancel.fork(cancel)` 异步触发取消（L324-L326）。`Effect.acquireUseRelease` 注册 abort listener（L328-L332），主体用 `Effect.raceFirst` 竞争 `background.wait(id)` 与 `background.waitForPromotion(id)`（L333-L337）。若被提升为后台，返回 running；error 返回 `result.error` 或默认 “Task failed”；cancelled 返回 “Task cancelled”；否则返回 completed，正文为 output（L338-L345）。释放阶段只在 Effect exit 含 interrupt 时并行执行子 prompt cancel 与 `background.cancel`（L347-L351），并始终移除 abort listener（L352-L356）。因此普通业务失败不自动 cancel，显式中断才取消；race 的先后决定前台调用收到终态还是后台运行态。

## 状态、取消、恢复与副作用

- 状态机可观察为：未建 job → `background.start` 的 running → completed/error/cancelled；也可由 `extend` 复用 running job，或由 `waitForPromotion` 转为后台。`renderOutput` 对外只暴露 running/completed/error 三态，cancelled 通过 Effect failure 暴露。
- 取消有三条路径：调用者 abort 事件触发 `ops.cancel`；Effect interrupt 的 release 同时调用 `ops.cancel` 和 `background.cancel`；`background.start` 的 run 被中断时也执行 `ops.cancel`（L296-L297）。取消是协作式的，没有超时、强杀或自动重试。
- “恢复”只由 `task_id` 查找既有 session 及 `background.extend` 两种语义提供；查找失败会新建 session，不能证明旧任务存在。没有显式 checkpoint、幂等键或重试计数。
- 持久化/外部副作用包括 `sessions.create`、`sessions.get`、`MessageV2.get`、`ctx.metadata`、`ops.prompt` 及父会话 synthetic 通知；agent 权限询问和子权限 deny 影响后续工具副作用。`promptOps` 检查晚于 session 创建，需防止半成品 session。
- 并发方面，后台主调用立即返回，`notify` 和父会话注入在共享 `Scope` 中 fork；前台只等待首个 race 结果。`background.extend/start/wait/cancel` 的具体锁、队列容量和持久化不在本文件确认。

## 源内测试与行为判据

源文件及同目录可见内容未包含测试。可独立验证的判据：

1. 实验开关关闭时 `background=true` 必须返回指定错误，且 `jsonSchema` 不接受该字段；开启时描述含后台说明。
2. 父链深度达到 `subagent_depth` 必须拒绝；直接子任务在默认 1 下可创建。
3. 未授权 agent 类型先收到 `ctx.ask`，随后未知类型报错；`bypassAgentCheck` 才跳过询问。
4. 子 agent 缺少 `todowrite`/`task` 时其 session 权限必须含全量 deny，primary_tools 也必须 deny。
5. assistant error、最后 tool error、无 text parts 分别映射为失败、失败、空字符串。
6. 后台 start/extend 立即返回 running；wait 完成后父会话收到 synthetic `<task ...>`；前台 abort 必须同时触发 prompt cancel 与 background cancel。

## zenpi Rust 映射

- `TaskPromptOps` 建议落在 `src/core.rs` 的 `SubagentTaskOps` trait（对照 `Agent::process_with_cancel` L5327-L5351、`run_active_turn_cancelable` L3642-L3678），实现 `resolve_prompt_parts`、`prompt`、`cancel`，由 `Agent`/会话宿主注入；不要把 provider HTTP 细节塞进 task 调度器。
- `Parameters` 对应 `src/protocol.rs` 新增带 `description/prompt/subagent_type/task_id/command/background` 的 `TaskRequest`，复用 `MAX_TEXT_BYTES`、ID 校验和 `Command` 解析（L214-L265、L601-L653）；background 功能开关应在解析/能力声明处体现，而不是只在执行后拒绝。
- `BackgroundJob` 对应 `src/runtime.rs` 的 `BackgroundRunner`、`CancellationToken`、`JobOutcome`（L49-L93、L158-L185、L237-L321）。`start/extend/wait/waitForPromotion` 需要一个按 session/job ID 的表层 adapter；队列满映射 `SubmitError::QueueFull`，关闭映射 `Closed`。前台 race 可用事件接收与 promotion 状态实现。
- 父子 session 可复用 `src/session.rs` 的 `SessionStore`、`SessionRecord` 和已有 `Turn.parent_id`（L146-L166、L327-L335），但当前没有与 TypeScript `parentID/agent/permission` 等价的 session 树字段，建议新增结构化 session metadata/event，并为 task_id 恢复加入“存在且属于当前父会话”的校验，避免源实现静默降级造成误复用。
- `ctx.ask` 映射 `src/approval.rs` 的 `ApprovalCoordinator::request/request_response`（L159-L219），`subagent_type` 作为受控 permission pattern；拒绝/取消必须转为结构化 `AgentError`，并在执行前持久化 approval 结果。
- 子 agent 模型选择对照 `src/providers/mod.rs` 的 `Protocol`/route（L22-L104）与 `src/providers/registry.rs` 的 `ModelDescriptor`/`ModelRegistry`（L59-L147）。zenpi 当前 provider registry 只描述 provider/model 能力，没有 agent type catalogue、agent 专属 model、variant 继承策略；建议新增 `AgentSpec {name, model, permissions}`，模型缺省时沿父 turn 的 provider+model。
- 工具权限执行对照 `src/tool_runtime.rs::execute_tool_batch`（L168-L347）和 `src/core.rs` 的工具准备/取消路径：task 子 session 的 deny 规则应在 `ToolContext`/registry 构造时落地，不能只在 prompt 文本中提示。`src/tool_runtime.rs` 的取消结果是“未执行”，可作为子任务 cancelled 的判据。
- 后台事件/协议返回对照 `src/headless.rs` 的 `run_async_streams`（L806-L837）及 bounded event mailbox（L862-L1065），把 running/completed/error 和 job_id 编成 `StdioEvent`/`StdioResponse`，并沿用重放与容量限制；`renderOutput` 可实现为 `TaskResult` 结构后再序列化，避免手写未转义 XML。
- 取消与关闭必须接入 `src/runtime.rs` 的 grace/shutdown 语义（L379-L424）：协作式取消不是回滚，超出 grace 的任务可 detach；需要明确 task session 的 terminal event，防止 headless 重连将未知结果误判为成功。
- `src/session.rs` 的 `OperationOutcome::{Succeeded,Failed,Cancelled,Interrupted,UnknownOutcome}`（L51-L96）可承载任务终态，但源文件没有重试/幂等策略；若 Rust 增加 retry，必须增加显式用户确认或 idempotency key。`src/headless.rs` 的 session/replay 持久化可承载通知，但源实现的 synthetic parent prompt 需要单独 event 类型。
- `src/providers/**`（`anthropic.rs`、`google.rs`、`connection.rs`、`registry.rs` 等）只负责协议路由、能力和模型目录，不应直接实现子 agent 生命周期；验证点是 task 调度器能在 provider 不变和 provider 切换两种模型选择下生成相同的 session/permission 记录。

## 未决问题

- `Tool.define`、`BackgroundJob.Service`、`Session.Service`、`Agent.Service` 的具体实现、锁粒度、队列容量和重连语义不在本源文件中。
- `DESCRIPTION`（task.txt）的完整文本及 `Tool.Context`、`ctx.ask/metadata/abort` 的错误/持久化保证未在此确认。
- `deriveSubagentSessionPermission` 的冲突优先级、`next.permission` 的默认规则，以及 `task_id` session 是否必须属于当前父 session 未由本文件约束。
- `background.extend` 的“更新”是追加 prompt、复用未完成 job 还是恢复持久化 job，及 `waitForPromotion` 的竞态优先级需查看 BackgroundJob 实现。
- `MessageV2.get`、`ops.prompt` 返回 parts 的顺序保证、tool error 的数据结构和 provider 重试策略需由相应模块确认。

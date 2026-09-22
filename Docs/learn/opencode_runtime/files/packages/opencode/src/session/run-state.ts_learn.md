# OC-027 — packages/opencode/src/session/run-state.ts

- source_id/item_id: `OC-027`
- source_path: `packages/opencode/src/session/run-state.ts`
- source_hash: `75ed3e7f02474f4ff8ac88b5f24b75ffa7cdb9829770377e7c33d248df43936e`
- source_bytes: `5506`
- source_lines: `151`
- coverage: 已从字节 `0-5505`、行 `L1-L151` 按顺序读完，包含全部 import、注释、类型、函数、导出和末尾导出别名。

## 完整行为复盘

文件实现一个按 `SessionID` 复用运行器的 `Effect` 服务。依赖 `BackgroundJob.Service` 与 `SessionStatus.Service`，并把每个会话的活动工作集中到一个 `Runner<SessionV1.WithParts>` 中（`L1-L9`、`L29-L34`）。

- `Interface`（`L11-L25`）定义四个公开能力：`assertNotBusy(sessionID)` 只返回 `void` 或 `Session.BusyError`；`cancel(sessionID)` 请求取消且没有业务错误类型；`ensureRunning(sessionID, onInterrupt, work)` 确保工作运行并返回 `SessionV1.WithParts`；`startShell(sessionID, onInterrupt, work, ready?)` 也返回结果，但可能失败为 `Session.BusyError`。`onInterrupt` 与 `work` 都是 `Effect.Effect<SessionV1.WithParts>`，所以中断路径本身能产生会话部件结果；`ready` 是可选 `Latch.Latch`（`L14-L24`）。
- `Service`（`L27-L27`）是 `Context.Service`，服务键为 `@opencode/SessionRunState`，使实现可由 Effect 环境注入。
- `layer`（`L29-L109`）在 Effect 生成器中取得后台任务与状态服务，创建一个 `InstanceState`。状态初始化函数取得当前 `Scope`，建立 `Map<SessionID, Runner<SessionV1.WithParts>>`，并注册 finalizer（`L35-L49`）。finalizer 对所有 runner 并发执行 `runner.cancel`，`concurrency: "unbounded"` 且 `discard: true`，随后清空 map；因此作用域关闭会取消全部会话运行器，不等待结果值，也不保留索引。
- `runner(sessionID, onInterrupt)`（`L52-L69`）先读共享状态并按会话查找已有 runner；存在时直接返回同一实例，保证同一 `SessionID` 的工作串联到同一状态机。不存在时用共享 `scope` 调用 `Runner.make`：`onIdle` 删除 map 项并写入 `{ type: "idle" }`（`L60-L63`）；`onBusy` 写入 `{ type: "busy" }`（`L64-L65`）；保存调用方的 `onInterrupt`。创建后立即写入 map 再返回（`L66-L69`）。这意味着空闲 runner 会被淘汰，下次调用重新建 runner；忙状态由 Runner 回调驱动。
- `assertNotBusy(sessionID)`（`L71-L75`）读取 map，仅当已有 runner 且其 `busy` 为真时失败；失败值由 `busyError` 构造。没有 runner 或 runner 空闲都视为可开始，检查本身不创建 runner、不改变状态。
- `cancel(sessionID)`（`L77-L86`）先调用 `cancelBackgroundJobs(background, sessionID)`，所以会话取消同时覆盖进程内 runner 与匹配的后台任务。随后查找 runner；不存在时显式写入 `idle` 并返回；存在时执行 `existing.cancel`。取消没有超时参数，是否完成由底层 Runner/Effect 决定。
- `ensureRunning(sessionID, onInterrupt, work)`（`L88-L94`）取得或创建 runner，然后转发 `.ensureRunning(work)`；返回其 `SessionV1.WithParts`。同一会话的并发调用共享 runner，具体排队/忙语义由 `Runner` 实现。
- `startShell(sessionID, onInterrupt, work, ready?)`（`L96-L105`）取得 runner，调用 `.startShell(work, ready)`；将标签为 `RunnerBusy` 的错误转换成 `Session.BusyError(sessionID)`，其他错误不在本函数捕获范围内继续传播。`ready` 原样传入，表示 shell 启动同步点。
- `cancelBackgroundJobs(background, sessionID)`（`L111-L143`）是会话关联后台任务的闭包遍历。先一次性 `background.list()`；`pending` 初始含目标 `sessionID`，`cancelled` 记录已成功取消的 job id（`L115-L117`）。`matches` 只接受 `status === "running"`、未在 `cancelled` 中，且满足：job id 在 `pending`，或 `metadata.sessionId` 在 `pending`，或 `metadata.parentSessionId` 在 `pending`（`L118-L124`）。每轮并发、无界地对当前 batch 调用 `background.cancel(job.id)`；取消成功后才在 `Effect.tap` 的同步段把 job id 加入 `cancelled` 与 `pending`，并把其 `metadata.sessionId` 加入 `pending`（`L125-L140`）。然后基于同一 `jobs` 快照重算 batch，直到为空（`L141-L143`）。因此会沿 job id、session id 和 parent session id 扩散到后代；取消失败会使 Effect 失败并停止后续流程，取消成功但列表快照仍标记 running 时由 `cancelled` 集合避免重复。
- `busyError(sessionID)`（`L145-L147`）是纯构造器，返回 `new Session.BusyError({ sessionID })`，不执行 I/O。
- `node`（`L149-L149`）把 `Service`、`layer` 及 `[BackgroundJob.node, SessionStatus.node]` 声明为可组装 LayerNode；`export * as SessionRunState from "./run-state"`（`L151-L151`）提供命名空间式模块导出。

并发语义集中在三处：按 `SessionID` 的 map 复用避免重复 runner；后台取消批次使用无界并发；作用域 finalizer 也无界并发取消全部 runner。源码没有显式锁，正确性依赖 `Effect` 的作用域/调度和 `InstanceState` 的串行访问约束。

## 状态、取消、恢复与副作用

状态只有 runner map 与 `SessionStatus` 的 `idle`/`busy` 标记；创建 runner 不持久化，空闲时从 map 删除（`L38-L48`、`L60-L63`）。`cancel` 先取消 `BackgroundJob` 中运行态任务，再取消 runner（`L77-L86`），因此外部任务取消先于会话工作取消。`onInterrupt` 是传入的恢复/中断效果，但本文件不调用它；它由 `Runner.make` 在中断语义下触发（`L59-L66`）。没有超时、重试、退避或自动恢复代码；`startShell` 只把 `RunnerBusy` 映射成稳定的 `Session.BusyError`（`L102-L105`）。

持久化方面，本文件只写 `SessionStatus.Service` 的运行态，不写 `SessionV1.WithParts`、runner 状态或后台 job 结果；恢复必须由上层 `Session`/会话存储完成。外部副作用包括 `background.list()`、并发 `background.cancel(job.id)`、`status.set`，以及 runner 的 `work`/`onInterrupt` 可能产生的会话部件。finalizer 在 scope 结束时取消所有 runner 并清空索引，但取消不是回滚：已发生的 provider、工具或文件副作用不由此恢复。`cancelBackgroundJobs` 没有二次 `list()`，只在初始快照上迭代，所以取消调用期间新出现的 job 不在本次取消保证内。

## 源内测试与行为判据

源文件及已读取的同目录内容未包含针对 `run-state.ts` 的测试，故源内未包含测试。可独立验证的判据：

1. 注入假的 `BackgroundJob.Service`、`SessionStatus.Service` 与可观测 `Runner`，同一 `sessionID` 两次 `ensureRunning` 必须只创建一个 runner；idle 回调后再次调用必须创建新 runner。
2. runner 忙时 `assertNotBusy` 与 `startShell` 均应得到带同一 `sessionID` 的 `Session.BusyError`；空闲或不存在时 `assertNotBusy` 成功。
3. 构造 running job 链：目标 session、`metadata.sessionId`、`metadata.parentSessionId` 及孙任务；调用 `cancel` 后所有可匹配任务均收到一次 `background.cancel`，非 running 或不相关任务不得取消。
4. 作用域关闭后所有 runner 均收到 `cancel`，map 为空，并且状态回调至少出现 idle/busy 的对应写入。

## zenpi Rust 映射

- `src/runtime.rs` 的 `CancellationToken`（`L56-L91`）、`BackgroundRunner`（`L237-L319`）和 `RuntimeEvent`（`L158-L184`）最接近 `Runner`：建议新增 `SessionRunState`，内部用 `HashMap<SessionId, SessionRunner>`，`SessionRunner` 持有 `JobId`、取消 token、busy 标记和 `on_interrupt` 回调；`ensure_running` 映射为提交或复用活动 job，`start_shell` 映射为带 ready barrier 的提交。`try_cancel` 与 `shutdown_and_join_with_grace`（`L319-L390`）可实现显式取消和有限关闭等待；差异是 zenpi 已明确“协作取消、不可回滚”，而源文件没有 grace timeout。
- `src/core.rs` 的 `AgentPhase`/`AgentEvent`（约 `L270-L350`）和 `submit_with_cancel`（`L3222-L3265`）是状态与工作入口。建议把 `SessionRunState` 的 idle/busy 写入转换为 `AgentEvent` 或新的 `SessionStatus` 事件；忙错误可复用 `NotSubmittedReason`，但需保留 `session_id` 以对应 `Session.BusyError`。`cancel_blueprint_worker`（`L2853-L2868`）只撤销 durable lease，不能替代本文件的进程内 runner 取消，应在 host 层串接。
- `src/session.rs` 的 `SessionStore`（`L146-L160`）、`OperationOutcome`（`L59-L65`）和 `OperationRecovery`（`L88-L96`）适合记录 start/terminal/interrupted marker。建议为每个 runner job 写 operation id 与 session id，取消落 `Cancelled`/`Interrupted`；源文件没有持久化，所以这是 zenpi 的增强，并须避免把“取消请求”误写成“已完成”。
- `src/tool_runtime.rs` 的 `execute_tool_batch`（`L168-L238`）已经要求调用方提供 `cancelled` 谓词，并在取消时阻止未开始调用、等待已开始调用；可把 `SessionRunState.cancel` 的 token 谓词传入。其批次是有界且会 join，源文件的后台取消是无界并发且只处理 `running` job，需明确这一差异。
- `src/protocol.rs` 的 `TurnMode`（`L46-L58`）和 `Command::Cancel`（`L214-L260`）可承载 `StartIfIdle`、`StartOrSteer`、`Steer` 以及 session/job cancel。建议新增 `SessionRunState` 控制器函数，由协议层把 cancel target 映射到 `SessionId`，返回 busy/cancelled/unknown 的稳定响应。
- `src/approval.rs` 的 `ApprovalCoordinator`（`L127-L187`、`L405-L440`）有 `cancel_all`、`emergency_cancel` 和 cancellation epoch；工具等待 approval 时应监听同一会话 token，否则 `run-state.cancel` 只停 runner 而 approval 阻塞仍存活。源文件不涉及审批，需保持审批取消与 job 取消的顺序可观测。
- `src/headless.rs` 已在多处处理 `Command::Cancel` 与 `CancellationToken`（例如 `L6291-L6346`），建议把每个 headless session 的 runner 注册/查找集中到新模块，再由 headless 只发控制命令；避免在 headless、TUI 各自维护 busy map。其有限 mailbox/replay 语义也应与 runner 的 terminal outcome 对齐。
- `src/providers/**`（`mod.rs`、`registry.rs` 及各 provider adapter）负责 provider 能力与请求执行，不应直接拥有 session runner。建议 provider 调用接收 `&CancellationToken`/取消闭包，在流式 chunk、重试前检查；provider 的网络取消不保证撤销已提交的远端副作用，这与源的 `onInterrupt` 回调边界一致。
- 建议落点：`src/session_run_state.rs` 定义 `SessionRunState`, `SessionRunner`, `SessionBusyError`、`ensure_running`、`start_shell`、`cancel`、`assert_not_busy`；`src/runtime.rs` 暴露可复用 runner 句柄；`src/headless.rs`/TUI host 仅负责协议与事件投影；`src/session.rs` 负责可恢复 marker。可执行差异清单是：补充按 session 的 map、idle/busy 事件、后台 job 的 session/parent 关联遍历、approval 取消联动、可选 shutdown grace，并为每项写集成测试。

## 未决问题

1. `Runner.make` 的 `ensureRunning`、`startShell`、`busy` 字段以及 `onInterrupt` 的确切触发时机不在本文件中，无法确认工作是否排队、替换还是拒绝。
2. `BackgroundJob.list()` 返回快照的并发一致性与 `background.cancel` 在 job 已结束时的行为未由本文件确认。
3. `SessionStatus.Service.set` 是否持久化、是否幂等、失败时是否影响 runner 生命周期，源码未给出。

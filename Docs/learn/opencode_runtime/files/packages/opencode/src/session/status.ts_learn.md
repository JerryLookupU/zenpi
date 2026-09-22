# OC-030 — packages/opencode/src/session/status.ts

- source_id/item_id：`OC-030` / `OC-030`
- source_path：`packages/opencode/src/session/status.ts`
- source_hash：`dbbbdee83c292379c1665a1d482b810a13754b6ae2143169b24c75ced5841b42`
- source_bytes：`1971`
- source_lines：`56`
- coverage：已按源文件顺序读取字节 `0-1970`（共 1971 字节）与行 `L1-L56`，包含全部导入、注释、类型、实现和导出。

## 完整行为复盘

文件把每个 `SessionID` 的运行状态封装成 Effect service。`LayerNode`、`InstanceState`、`EventV2Bridge` 和 `SessionStatusEvent` 的导入位于 `L1-L6`：前者提供节点依赖与实例作用域状态，后者定义状态联合类型及事件协议。

- `Info` 常量和 `Info` 类型别名（`L8-L9`）直接转出 `SessionStatusEvent.Info`。可接受值由被引用的 schema 定义：`{type: "idle"}`、`{type: "busy"}`，或 `retry` 状态（含非负 `attempt`、字符串 `message`、可选 `action` 结构和非负 `next`）。本文件不重新校验字段，也不提供默认 retry 参数。
- `Event`（`L11`）导出整个 `SessionStatusEvent` 命名空间，因此调用者使用 `Event.Status` 和 `Event.Idle`。`Status` 的数据是 `{sessionID, status}`，事件类型为 `session.status`；`Idle` 的数据是 `{sessionID}`，事件类型为已标记 deprecated 的 `session.idle`。
- `Interface`（`L13-L17`）规定服务 API：`get(sessionID)` 返回 `Effect.Effect<Info>`；`list()` 返回 `Effect.Effect<Map<SessionID, Info>>`；`set(sessionID,status)` 返回 `Effect.Effect<void>`。签名没有显式错误类型，底层 `Effect` 依赖或事件发布失败仍可使效果失败或被中断。
- `Service`（`L19`）是上下文标签为 `@opencode/SessionStatus` 的 `Context.Service`，把上述 `Interface` 注入 Effect 环境。
- 私有 `layer`（`L21-L52`）用 `Layer.effect` 构造 `Service`。`L23-L24` 从环境取得 `EventV2Bridge.Service`；`L26-L28` 通过 `InstanceState.make` 初始化一个 `new Map<SessionID, Info>()`。状态是实例作用域内存，初始没有条目；初始化函数被 `Effect.succeed` 包装，未建立磁盘持久化。
- `get`（`L30-L33`）先以 `InstanceState.get(state)` 取得 Map，再执行 `data.get(sessionID) ?? { type: "idle" as const }`。未登记的 session 返回新的 idle 默认值，但不会写回 Map；已登记的值原样返回。它不发事件、不改变状态，也不检查 session 是否存在于其他会话目录。
- `list`（`L35-L37`）读取当前 Map 后用 `new Map(...)` 返回浅复制。因此调用者增删返回 Map 不会改变服务的键集合，但其中的 `Info` 值没有深复制；实现没有排序、过滤或 idle 补全，未登记 session 不出现在结果中。
- `set`（`L39-L48`）接收 `sessionID` 与 schema 已约束的 `Info`。先取得可变 Map，随后在 `L41` 发布 `Event.Status`，所以所有状态（包括 idle）都先产生 `session.status`。当 `status.type === "idle"`（`L42-L46`）时，再发布 `Event.Idle`，然后删除键并返回；idle 会同时发新事件和兼容旧事件。非 idle（`busy` 或 `retry`）在两次发布之后执行 `data.set(sessionID,status)`（`L47`）。同一 session 的重复 set 没有去重，事件也不会因值相同而省略。
- `Service.of({ get, list, set })`（`L50`）只暴露三个操作；`Effect.fn` 名称（`SessionStatus.get/list/set`）用于追踪。`node`（`L54`）以 `LayerNode.make` 导出该层，并声明 `EventV2Bridge.node` 为依赖，确保事件桥接先就绪。文件末尾 `export * as SessionStatus from "./status"`（`L56`）提供聚合命名空间，使外部可取 `SessionStatus.Service`、`SessionStatus.node` 等。

边界与并发：idle 删除发生在两个发布成功之后；任一发布失败或效果被中断，后续 Map 修改不会执行。非 idle 的事件发布失败同样阻止写入。源码没有显式锁、CAS 或“只接受最新状态”规则；并发语义依赖 `InstanceState` 的作用域和 Effect fiber 调度，多个 fiber 对同一 Map 的先后顺序就是最终状态顺序。`list` 取得的是调用时的键值快照，之后的 `set` 不会改变该返回 Map。

## 状态、取消、恢复与副作用

状态只有内存中的 `Map<SessionID, Info>`；本文件没有超时计时器、取消 token、重试循环、恢复扫描或 journal 写入。`retry` 只是可观察状态载荷，`attempt`/`next` 的计算和再次运行由调用方（例如 session processor/run-state）负责。Effect 的通用中断语义可能在 `get/list/set` 任意 yield 点生效，但源码未捕获取消，也没有 finally 来补发 idle。

唯一外部副作用是 `EventV2Bridge.Service.publish`（`L41-L44`）。`EventV2Bridge` 会把事件路由到实例/工作区位置并转发 `GlobalBus`；因此 `set` 既更新内存状态又通知订阅者。事件顺序严格是 `session.status`，然后（仅 idle）`session.idle`，最后才是 Map 删除；订阅者可能在收到事件的瞬间观察到旧 Map。进程重启或实例状态销毁会丢失全部状态，事件本身是否 durable 由桥接/事件定义决定，而非本文件保证。

## 源内测试与行为判据

源文件及其 `src/session` 同目录未包含针对 `status.ts` 的测试（目录枚举只有实现模块与提示文件）。可独立验证的判据如下：

1. 构造 layer 后首次 `get(any)` 必须返回 `{type:"idle"}`，首次 `list()` 必须为空 Map（对应 `L26-L37`）。
2. `set(id,{type:"busy"})` 必须先收到一个 `session.status`，随后 `get(id)` 为 busy，`list()` 含该 id（`L39-L48`）。
3. `set(id,retryInfo)` 必须保留完整 retry 载荷，不得改写 `attempt`、`message`、`action` 或 `next`。
4. `set(id,{type:"idle"})` 必须按 `Status` 后 `Idle` 的顺序收到两个事件，之后 `get(id)` 回到默认 idle 且 `list()` 不含 id（`L41-L46`）。
5. 让第一次 `publish` 失败时，验证 `set` 返回失败且对应 Map 未被更新；让第二次 idle 发布失败时，验证旧 Map 条目仍存在。这检验事件先行与副作用边界。

## zenpi Rust 映射

- `src/core.rs`：`AgentPhase::{Idle,Running,Closed}`、`AgentSnapshot` 和 `AgentEvent`（尤其 `TurnAccepted`、`Provider`、`Error`）是现有状态投影。建议新增 `SessionStatusInfo`（`Idle|Busy|Retry{attempt,message,action,next}`）及 `SessionStatusStore`，把 `get/list/set` 做成 `&self/&mut self` 方法；在 `Agent` 的 turn admission、provider 错误和完成路径调用它。
- `src/session.rs`：`SessionStore::append_event` 是持久事件入口，但 TS 服务本身不持久化。若要保留跨进程状态，应增加可选的 `session_status` 事件或由 `AgentSnapshot` 重建；必须明确 idle 删除是内存视图还是 journal 记录，避免把 TS 的实例重置误当成 durable 恢复。
- `src/runtime.rs`：`BackgroundRunner` 的 `RuntimeEvent::{Started,CancelRequested,Completed}` 与 `CancellationToken` 可驱动 `Busy`/`Idle` 转换。完成标记应在发布成功结果后再置 idle，以匹配 TS 的“先事件、后状态变更”顺序；取消仍是合作式，不应声称回滚。
- `src/headless.rs`：已有异步 stdout/replay 和事件邮箱，可把状态事件包装为带 `session_id`/`turn_id` 的 `StdioEvent`。需限制事件重放预算，并保证 `session.status` 与 `session.idle` 的顺序，不能把状态查询结果当成第二个 terminal response。
- `src/protocol.rs`：`Command::Status` 和 `StdioEvent` 是协议落点。建议为 `Status` 响应定义与 `SessionStatusInfo` 对应的 serde 结构，并把未登记 id 的响应固定为 idle；显式版本兼容应沿用现有 `schema_version` 规则。
- `src/tool_runtime.rs`：没有直接等价物；它负责工具批处理、批准前置和取消传播。只需在工具批次开始/结束时调用状态 store，不能让工具执行器自行发布重复状态或自行持久化。
- `src/approval.rs`：无直接状态映射。审批等待可使会话保持 busy；若审批被取消，应由 core/runtime 统一发布 retry、failed 或 idle，不能由 `ApprovalCoordinator` 猜测 session status。
- `src/providers/**`：各 provider 的 `ProviderEvent`、网络重试与错误是 retry 信息来源；provider 模块只产出错误/流事件，不应持有跨 session 的 Map。建议由 `core.rs` 将 provider 错误转换成 `Retry{attempt,next,message,action}`，并由统一状态 owner 发布。

差异清单：TS 是每个实例一份可变 Map，Rust 当前以 `Agent`/`SessionStore` 和持久 JSONL 为主；TS 的 `get` 默认 idle 但不登记，Rust 查询 API 需显式遵守；TS idle 兼发 deprecated `session.idle`，zenpi 目前主要是 `AgentEvent`/`StdioEvent`；TS 不提供锁或 CAS，Rust 若跨线程共享应使用 owner thread 或 `Mutex/RwLock` 明确串行化；TS 的事件先行规则与 zenpi 当前部分“先 journal 再输出”路径可能不同，映射时必须写集成测试验证顺序。

## 未决问题

1. `InstanceState.make` 的具体生命周期和并发保证不在本文件中，无法仅凭 `status.ts` 判断 Map 是否会跨 workspace 共享。
2. `EventV2Bridge.publish` 的 durable 选项未在本文件传入，无法确认 `session.status` 是否会被底层事件总线持久化。
3. `Info` 的 schema 细节来自外部 `@opencode-ai/schema/session-status-event`；本文件自身不定义 retry 的数值上限或 `action.link` 约束。

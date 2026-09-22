# OC-028 — packages/opencode/src/session/schema.ts

- source_id/item_id: `packages/opencode/src/session/schema.ts` / `OC-028`
- source_path: `packages/opencode/src/session/schema.ts`
- source_hash: `fe78e7d60b0772cb62c95473f8ff4c0e68caa083bbbb4dc1cc1984217234822d`
- source_bytes: `814`
- source_lines: `26`
- coverage: 已完整读取字节 `0-813`（共 814 字节），行范围 `L1-L26`；没有跳过注释、导入、类型或导出符号。

## 完整行为复盘

该文件是 session 领域 ID 的 Effect Schema 定义层，自己不保存会话、不访问网络，也不启动任务。`L1` 导入 `Schema`；`L3` 导入 `Identifier`（实际提供 `ascending` 生成器）；`L4` 导入 `SessionV2`；`L5` 导入 `statics`。导入本身没有运行时业务分支，但后续构造器会触发 `Identifier` 的时钟/计数器/随机字节行为。

- `SessionID` 导出常量（`L7`）：直接赋值为 `SessionV2.ID`，因此运行时校验、编码/解码规则和生成能力完全继承 `@opencode-ai/core/session` 的定义，本文件没有再加前缀、长度或格式约束。它的同名类型导出（`L8`）是 `Schema.Schema.Type<typeof SessionID>`，用于把 schema 的静态类型暴露给 TypeScript；它不产生新的运行时值。输入若不符合 `SessionV2.ID`，错误由被委托的 schema 报告；具体错误结构本文件未定义。该符号没有本地默认值、重试、并发锁或副作用。
- `MessageID` 导出常量（`L10-L15`）：先以 `Schema.String` 要求输入为字符串，再通过 `Schema.isStartsWith("msg")` 要求字符串以 `msg` 开头（例如 `msg_x`；源码没有要求下划线、总长度或唯一性），随后 `Schema.brand("MessageID")` 添加 TypeScript 品牌，最后用 `pipe` 接入 `statics`。因此未知输入在解码/校验时会因非字符串或前缀不符失败；错误类型/文本由 Effect Schema 决定。品牌只收紧静态类型，不改变序列化字符串。
- `MessageID` 的静态方法 `ascending(id?: string)`（`L12-L14`）：可不传参数，也可传一个候选字符串；它调用 `Identifier.ascending("message", id)`，再交给 `s.make(...)` 制作 schema 值。未传 `id` 时，`Identifier` 按 `message` 前缀生成升序 ID；给出已有合法 `msg` 前缀时原样保留；给出其他非空前缀时 `Identifier.ascending` 抛出错误。`Identifier` 的实现还把空字符串视为未提供，因而空串会触发新 ID 生成，而不是保留空串；这个边界来自被调用实现而非本文件。生成器使用时间戳、进程内单调计数器和随机后缀，故同一进程内具有排序意图，但本文件没有跨进程唯一性承诺。`s.make` 的精确异常形式未在源内声明，应以 Effect Schema 运行时验证。
- `MessageID` 同名类型导出（`L17`）：仍是 `Schema.Schema.Type<typeof MessageID>`，即带 `MessageID` brand 的字符串静态类型；运行时仍为字符串。
- `PartID` 导出常量（`L19-L24`）：结构与 `MessageID` 相同，但前缀检查改为 `Schema.isStartsWith("prt")`，品牌为 `PartID`。接受 `prt...` 字符串，不保证下划线、长度或跨存储唯一性；非字符串/前缀不符走 schema 校验错误路径。
- `PartID` 的静态方法 `ascending(id?: string)`（`L21-L23`）：调用 `Identifier.ascending("part", id)` 后 `s.make`。无参数生成升序 `part` ID（实际字符串前缀为 `prt`）；传入非空错误前缀会抛错，空字符串按 `Identifier` 的 falsy 规则生成新值。该方法无 Promise、取消点或显式重试。
- `PartID` 同名类型导出（`L26`）：为 `Schema.Schema.Type<typeof PartID>`，提供独立品牌，防止 TypeScript 中把消息 ID 与部件 ID 混用；它们在运行时都是字符串。

并发语义方面，本文件没有锁、队列、事务或异步函数。并发安全取决于 `Identifier` 的模块级 `lastTimestamp/counter`；在典型 JS 单线程事件循环中调用是顺序的，但若被多 worker/多进程使用，不能从本文件推导全局排序或唯一性保证。`SessionID` 的并发语义则完全由 `SessionV2.ID` 决定。

## 状态、取消、恢复与副作用

文件没有 session 状态机，也没有取消、超时、重试、恢复或持久化逻辑；没有文件、数据库、网络、日志或工具调用副作用。唯一可观察副作用来自 ID 生成路径：`Identifier.ascending` 可能读取 `Date.now()`、递增模块级计数器并调用随机源；传入已有合法 ID 时通常只返回输入，不需要新随机值。schema 检查和 brand 本身是纯描述。若调用者在取消后才调用 `ascending`，本文件不会感知取消，必须由上层在 `src/runtime.rs`/任务边界阻止调用。ID 生成成功后没有回滚；若后续持久化失败，已消耗的计数器/随机值不会复原。

## 源内测试与行为判据

源文件本身没有测试，写作“源内未包含测试”。可独立验证的判据如下：

1. 用 Effect Schema 解码 `"msg_existing"`、`"prt_existing"` 应分别通过 `MessageID`、`PartID`；`"ses_x"`、数字或对象应被拒绝（对应 `L10-L11`、`L19-L20`）。
2. `MessageID.ascending()` 返回以 `msg` 开头的字符串，`PartID.ascending()` 返回以 `prt` 开头的字符串；连续调用的生成值应保持 `Identifier.ascending` 的升序意图（`L12-L14`、`L21-L23`）。
3. `MessageID.ascending("msg_given")` 和 `PartID.ascending("prt_given")` 应保留给定值；错误前缀应抛出 `Identifier` 错误。特别测试空字符串，验证其按 `Identifier` 的 falsy 分支生成新值。
4. TypeScript 编译应拒绝把 `MessageID` 赋给需要 `PartID` 的位置，证明 `Schema.brand` 的静态隔离仍生效（`L11`、`L20`）。

## zenpi Rust 映射

- `src/session.rs` 是最接近的落点：已有 `SessionHeader.session_id`、`SessionStore::session_id()`、`SessionSummary` 和 append-only JSONL 恢复。建议新增集中式 `SessionId`, `MessageId`, `PartId` newtype（或放入 `src/session.rs` 的 `ids` 子模块），实现 `Deserialize`/`Serialize`、`Display` 和 `TryFrom<String>`；验证规则分别复刻 `SessionV2.ID` 的实际契约、`msg` 前缀和 `prt` 前缀。不要把品牌语义退化为裸 `String`。
- `src/protocol.rs` 已有 `MAX_ID_BYTES`、`validate_queue_id`、`session_id`/`turn_id` 字段及 `CheckpointCursor`。应让协议边界先做长度、控制字符和空值检查，再转换为上述 newtype；`MessageId`/`PartId` 适合用于消息/工具事件的关联字段。差异是当前协议只做通用队列 ID 校验，没有 `msg`/`prt` 前缀品牌。
- `src/core.rs` 的 `Turn`、`TurnInputRequest` 和 `WorkerExecutionBinding` 已覆盖 turn/请求关联，但没有与 `MessageID`、`PartID` 等价的强类型。建议在 `Turn` 的消息记录或事件结构中增加明确的 `MessageId`，在内容部件/工具输出结构中增加 `PartId`，并在 `Turn::validate` 前完成转换；保持现有 `AgentError::InvalidTurn` 错误路径。
- `src/headless.rs` 负责 JSONL 重放、请求去重和 session reconnect。它应只序列化这些 ID 的字符串表示，重放时重新做 `TryFrom` 校验；不能依赖请求 ID 重放来重新生成 `ascending` ID，否则会产生新标识并破坏幂等。现有 `MAX_REPLAY_*` 和 in-flight 语义可承接重复请求，但本 schema 本身没有重放行为。
- `src/runtime.rs` 已提供 `CancellationToken`、完成标记和安全输入边界。ID 构造应发生在任务进入持久化/发送边界之前，并在取消检查后一次性生成；不能把 `ascending` 当作可取消或可回滚操作。若生成后任务被取消，仍应把已经写入的 ID 视为不可重用的值。
- `src/tool_runtime.rs` 的批量工具执行、取消和 unknown outcome 逻辑与 `PartID` 的“内容部件关联”可对接：每个 tool call/result 产生稳定 `PartId`，取消时保留已持久化部件并按现有 unknown outcome 规则处理，禁止因重试重新生成同一部件 ID。
- `src/approval.rs` 的 `ApprovalRequest.turn_id` 与持久化审批决定可引用 `MessageId`/`PartId`，但审批本身不应生成这些 ID，也不应在取消/拒绝路径偷偷消耗业务标识。
- `src/providers/**` 没有该 schema 的直接对应物；这些模块的 `ProviderDefinition`、`Protocol`、`ModelDescriptor` 负责路由和能力，不应参与 session/message/part ID 生成。可执行差异检查是全仓搜索 provider 代码，确保不会用 provider/model 前缀替代 `msg`/`prt`。

建议的可验证 Rust 行为：在 `src/session.rs` 添加单元测试覆盖合法前缀、错误前缀、空字符串、控制字符和 serde round-trip；在 `src/protocol.rs` 添加协议输入转换测试；在 `src/headless.rs` 添加相同 request ID 重放后 ID 不变的测试；在 `src/runtime.rs` 添加取消发生在生成前/后的边界测试。这样可明确区分本文件的 schema 校验、`Identifier` 生成副作用与 zenpi 的持久化/取消语义。

## 未决问题

1. `SessionV2.ID` 的具体格式、是否自带 `.ascending`、长度和解码错误结构不在本文件内，需继续读取 `@opencode-ai/core/session` 才能完全对齐。
2. `statics` 与 `s.make` 在当前 Effect 版本中对非法值是抛异常还是返回特定 ParseError，源文件未显式说明。
3. `Identifier` 的模块级计数器在 worker/多进程场景下是否共享不由本文件保证；跨进程排序与唯一性只能由上层持久化或存储约束确认。

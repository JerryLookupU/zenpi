# SPEC v3: 恢复流程、流式容错与错误日志改造

日期: 2026-09-21（v2 吸收 gpt-6-astra max-effort 评审的 26 条意见；v3 记录最终取舍与落地状态）
状态: 已实施（M5/M6 经评审后决定不做，理由见下）
基线: HEAD `5d6fe4c` + 工作区改动（`/new` 解除 operation 围栏，已落地）

## 落地状态（PR 链，后者依赖前者）

| 项 | 状态 | PR |
|---|---|---|
| 流式空 name/id 容错（SPEC 外前置修复） | 已实施 | #5 |
| `/new` 解除 operation 围栏 + errorlog 脚本修 bug | 已实施 | #6 |
| M7 max_output_tokens 仅显式发送（普通 turn + 压缩，Responses/Chat；Anthropic/Google 不动） | 已实施并在真实 Codex OAuth 网关验证 | #7 |
| M2 transcript 修复 + M1 解除围栏（`AgentEvent::Warning`、`automation_vetoed`、压缩否决写 journal） | 已实施 | #8 |
| M3 `/recovery abandon-all`（两集合去重、批量裁决、幂等）+ M4 `/new` 三层真实阻断源报错 | 已实施 | #9 |
| M5 内置轮转日志 | **不做**：外挂 `tools/zenpi_errorlog.py` 已覆盖 triage 需求，内置模块（锁/轮转/脱敏/接入点边界）收益边际 | — |
| M6 doctor/import 校验 | **不做**：启动链路（`src/core.rs:5867`）已警告降级，import 有 5 个写入分支、输出有 7 个消费方，性价比不足；workaround 为手工删除非法行 | — |

真实环境回归（Codex OAuth 网关，全部通过）：Responses wire 基本 turn、带 `unknown_outcome` 的 session 直接继续对话（原围栏场景）、Responses wire 工具调用（原始截图场景）、`/recovery abandon-all` 一条命令清两个挂起操作。

背景问题（本机复现过）:

1. turn 中途失败后 journal 留下 `unknown_outcome`，新 turn、`!` shell、会话树导航全部被拒，只能手敲 `/recovery abandon|retry <完整 operation ID> --yes` 解锁。`/new` 此前也被同一围栏拦截（工作区已改）。
2. turn/backend 错误主要在实时事件流；journal 有 operation 标记、工具错误 turn、`user_shell_error`、headless reconnect WAL，但没有独立的错误日志文件。
3. `config import-codex` 原样复制 `model_reasoning_effort`；当前启动链路（`src/core.rs:5867`）已对不支持的 effort 警告并置 `None` 继续启动，但 import 落盘的配置本身仍携带非法值，`config doctor` 仍是纯静态检查（`src/config.rs:1458` `status_for_profile`）。
4. Responses wire 下普通 turn（`src/core.rs:3808`）与压缩摘要（`src/core.rs:1931`）都无条件发送 `max_output_tokens`，Codex OAuth 类上游对该字段返回 400（本机抓包逐字段验证，唯一触发字段），无配置可关闭。

---

## M1: 解除 unknown_outcome 对交互路径的围栏，改为只禁止自动化路径

### 现有证据机制（已存在，不重写）

- 调度前写 `tool_execution_started`（`src/core.rs:4522-4532`），结果落盘为 `TurnRole::Tool` turn 后写 `tool_execution_finished`（`src/core.rs:4880-4886`），结果无法落盘写 `tool_unknown_outcome`（`src/core.rs:4872-4877`）
- `Agent::unknown_tool_outcomes()`（`src/core.rs:1578-1600`）扫描上述事件得出未决工具集合
- `Agent::operation_recovery()`（`src/core.rs:1608` → `src/session.rs:1489`）扫描通用 operation 标记；注意 `src/session.rs:1493` 把**尚无 outcome 的 operation（包括当前 owner 正在执行的）也算未决**

### 要改的代码

1. 交互围栏放行三处：
   - `src/core.rs:2908-2914` `submit_with_cancel`：删除 `AgentError::Recovery` 返回
   - `src/core.rs:2644-2648` user shell 入口：同样放行
   - `src/core.rs:830-841` 树导航：放行（`List` 分支本来就提前返回，不受影响）
2. 放行时的提示**不能用 `AgentEvent::Error`**：`src/view_model.rs:1136` 会把它映射成 `TurnFailed`，TUI（`src/tui.rs:5735/5770`）和 headless（`src/headless.rs:9161`）都会按终态失败处理。新增非终态事件 `AgentEvent::RecoveryWarning { pending: usize }`，补齐 `view_model.rs` 与 headless 两端的事件映射和测试，TUI 显示为系统提示行，headless 输出 `kind:"warning"` 块。
3. **治理路径不降级、不放行**（评审纠正 v1 错误）：`src/governance.rs:544` 是 `require_settled_item`，生产调用仅 `src/domain_execution.rs:2604` 的成果验收；`src/governance.rs:925` 是 `require_open`，保护 `core.rs:1509/1530`（worker 准入）、`core.rs:2512`（续租）、`domain_execution.rs:2335/2596/2643/2794`（租约检查）、`2670`（validator 预留）。这些属于自动化结算保护，"用户发起"不等于"资源已结算"，全部保持现状。

### 自动化否决（新增）

新增 `Agent::automation_vetoed() -> Option<String>`：**仅统计历史遗留未决**，即 `unknown_tool_outcomes()` 与 `operation_recovery()` 中排除"当前 owner 正在进行中的 operation"后的集合非空时返回原因（评审指出：turn 执行前已写 provider operation（`src/core.rs:3391`），不排除当前 in-flight 会导致健康状态下压缩也被误否决）。以下路径执行前检查，被否决时写 journal `{"type":"automation_vetoed","reason":...}` 并跳过：

- 自动语义压缩准备（`prepare_semantic_provider_context`，`src/core.rs:1788`；请求构造在 `1927`）。**同时必须定义否决后的降级行为**：输入仍超预算时返回明确错误，禁止继续发送超预算请求
- 手动压缩入口（`src/core.rs:2103`）与工具输出清理（`src/core.rs:959`）：逐一核对后归类
- 现有自动重试（`src/backend.rs:1646`）：这是已存在的自动化重试，不是"未来新增"，同一否决约束
- steer reissue（`src/core.rs:3149`）绕过 `submit_with_cancel`，需单独核对
- 不禁止读取类恢复：headless `replay_from`（`src/headless.rs:304`）、`CheckpointRequest::Replay`（`src/headless.rs:8834`）只重发记录，项目 `restore_checkpoint` 只恢复 owner，均不重新执行旧工具，保持放行

### 验收

turn 失败后可继续对话、`/new`、`!` shell；界面只有一条 warning；健康状态下自动压缩正常工作；遗留未决时压缩/重试被否决且 journal 可查。

---

## M2: transcript 修复（补齐未配对的 tool 结果）

### 评审纠正的 v1 错误（先读这段）

1. 只扫 `unknown_tool_outcomes()` 不够：`src/core.rs:3929` 先持久化整批 assistant tool_calls，之后才逐个写 dispatch marker；两者之间的崩溃窗口里，**尚未开始的调用不在任何集合里**。必须从选中的历史 turn 计算"assistant 发起了但无 tool 结果配对"的调用，再结合 dispatch 证据区分"未执行"与"结果未知"。
2. 正常 `UnknownOutcome` 路径下真实结果**已经落盘**（串行 `src/core.rs:3992`、并行 `4195` 都先持久化结果再查 outcome）；修复只补**缺失**的结果，不得覆盖真实结果。
3. 查重键必须是 `(turn_id, call_id)`：`src/core.rs:6298` 只保证同 turn 内 call ID 唯一，跨 turn、跨分支会重复。
4. 修复时机在批次收尾后，不能在并行结果尚未全部落盘时全量修复（否则先合成后追加真实结果）。
5. user shell 的未决结果角色是 `TurnRole::User`（现有 `resolve_tool_outcome`，`src/core.rs:1630` 已特判），不能统一合成 `TurnRole::Tool`。

### 要改的代码

1. 新增 `Agent::repair_transcript_gaps(&mut self) -> Result<usize, AgentError>`：
   - 输入：当前分支历史中所有 assistant tool_calls（metadata `tool_calls[].arguments` 在 `src/core.rs:3913`）与已有 `TurnRole::Tool`/`TurnRole::User` 结果做 `(turn_id, call_id)` 配对
   - 缺失者合成 `ToolResult::Error`（结构 `src/tools.rs:652`），message 明确"interrupted, side effects unknown"；metadata 带 `synthetic: true`、`outcome: "interrupted"`
   - 落盘不可用时不承诺修复成功，返回错误而不是假装修好
2. "transcript 已修复"与"副作用已裁决"**分开**：合成结果只保证历史良构，不消除 recovery 否决。新增事件类型要同步更新三个消费方：`src/session.rs:2141`（operation 决策投影）、`src/context.rs:452`（压缩的 outcome 标注）、`src/session_tree.rs:550`（目前拒绝 `interrupted`）
3. 调用点（`Agent::open` 不存在，v1 写错）：
   - `acknowledge_recovery`（`src/core.rs:562`，`mark_interrupted_operations` 的实际调用路径）
   - `prepare_project_with_options`（`src/core.rs:651`）
   - `resume_session_with_commit`（`src/core.rs:5183`）与 session replacement
   - turn 内批次收尾处（`src/core.rs:3999-4009`、`4201-4212` 命中 UnknownOutcome 时）

### 验收

恢复完成、提交下一请求前历史必完整；重复恢复幂等；跨 turn 同 call ID、部分落盘、非当前分支场景均有测试。

### M1 与 M2 的依赖关系（评审纠正）

方向正确：先能修复历史，才能放行。但**不必同 PR**——可以先上 M2 再上 M1，或加开关。另外 `/new`（空历史）和"首次 provider HTTP 失败（无 assistant calls）"两个场景本就不依赖 M2，v1 "同 PR 否则上游 400"的表述过强。

---

## M3: /recovery 易用性

### 要改的代码

1. `src/slash.rs:1155-1173`：增加 `abandon-all --yes` → `RecoveryAction::AbandonAll`；同步更新 usage（`src/slash.rs:717`）和错误提示（`src/slash.rs:856`）。无 ID 参数，`MAX_ID_BYTES` 不适用，保留整体输入长度与 `--yes` 校验。
2. 分派**复用现有共享 owner**：TUI（`src/tui.rs:11879`）已调用 `headless::recovery_view`（`src/headless.rs:1941`），在 headless 这一处实现 `AbandonAll`，TUI 自动获得能力，不做两套。
3. 批量逻辑：`unknown_tool_outcomes()` 与 `operation_recovery()` 两个集合可能含同一 operation，**先按 operation ID 去重**；工具项优先走 `resolve_tool_outcome`（`src/core.rs:1630`），再处理 generic recovery；逐条落盘，单条失败时报告成功 ID、失败 ID、剩余数，重入幂等。
4. inspect 摘要的数据来源纠正（v1 错误：`ToolExecutionEvidence`（`src/core.rs:102`）不含参数）：
   - 参数在 assistant metadata 的 `tool_calls[].arguments`（`src/core.rs:3913`），且可能被 `extension_tool_rewrite`（`src/core.rs:4606`）改写——摘要取**改写后**的值
   - user shell 关联 `user_shell_input`（`src/core.rs:2669`）
   - 时间取 journal 记录外层 `timestamp`，不是 `session.events()` 的内联值
   - 摘要先经 `src/security.rs` 脱敏，再按 UTF-8 字符边界截断（不是字节截断，避免切出非法 UTF-8）

---

## M4: /new 被拦截时的补救信息

### 评审纠正的 v1 错误

1. `/attach remove` 不存在（`src/slash.rs:1289` 只有 `/attach <path>`，`remove` 会被当成文件路径）；`clear_pending_attachments` 只有 API 没有命令。二选一：新增附件移除命令（含 TUI/headless 两端测试），或报错文案不提供附件移除指引。
2. `/input cancel ID` 存在但走 `input_queue_control`（`src/slash.rs:2215`），队列字段是 `QueuedInput.id` 不是 `input_id`；附件可能是 URL/file_id，没有必然存在的文件名。
3. 只改 core 文案不够：TUI（`src/tui.rs:16195`）先检查 pending requests/input tickets/scheduled work/approval/编辑器，headless（`src/headless.rs:5777`）先查项目任务队列，这两层比 `src/core.rs:5099` 的 gate 先触发。

### 要改的代码

三层各自报告真实阻断源和对应取消方式（TUI 层列出 pending request/approval 的取消命令；headless 层列出任务队列；core 层列 `Received` 状态的队列项，设置展示数量与单条文本长度上限）。验收覆盖实际 `/new` 路由（TUI 命令 + headless 命令），不直接调 Agent。

---

## M5: 内置轮转错误日志

### 评审补充后的设计

新增 `src/logging.rs`（`src/lib.rs` 注册）：

```rust
struct LoggerState { file: BufWriter<File>, bytes: u64, date: String, sequence: u32 }

pub struct ErrorLogger { dir: PathBuf, state: Mutex<LoggerState> }
// 全部可变状态在同一个 Mutex 内；多 project 共享 Arc<ErrorLogger>，
// 避免同 PID 多次 discover 打开同名文件各自计数。
```

- 文件 `$ZENPI_HOME/logs/zenpi.<YYYY-MM-DD>.<pid>[.N].log`，每行一个 JSON（固定字段 `ts/level/component/message/code/session_id/turn_id`，context 不得覆盖固定字段，单条大小上限）
- 超 10 MiB 轮转新序号，启动时清理 7 天前文件；文件权限 0600，拒绝 symlink
- flush 策略明确化：Error 级立即 flush，Warn 级可缓冲但进程退出前 flush（`BufWriter` 追加不等于落盘）
- 契约改为"**尽力记录**"：`discover()` 失败、写失败均不影响业务，但不能宣传"永远有记录"
- 脱敏**复用现有接口**（评审纠正 v1）：`src/security.rs:230` 已有 `registered_secret_values`、`redact_text`/`redact_json`（秘密值、字段名、header、URL 四类），不新增迭代器；注意 legacy 登记有 10 分钟 TTL，补长会话测试
- 接入点：在 `complete`/`complete_with_control` 的**终态边界**统一记录（区分重试尝试与最终失败），而不是搜 `backend.rs` 的 `BackendError` 构造点（`?` 传播会漏）；工具失败是 `ToolResult::Error` **值**、turn 可继续成功，需单独的工具结果接入点；`make_backend`（`src/core.rs:6150`）与 session 打开（`6151`）早于宿主初始化，TUI/headless 初始化接入会漏启动错误，Agent 持有 logger 后要能传给 backend、SessionStore、project owner

### Python 侧（评审发现的既有 bug，顺手修）

- `tools/zenpi_errorlog.py` 的 `FAILED_OUTCOMES = {"success"}` 与实际 outcome 值 `"succeeded"` 不符，会把正常 operation 误报为错误
- fingerprint 不含 turn_id/时间，原生日志无 seq 时可能误合并重复错误
- 新增 `logs` 子命令读 `$ZENPI_HOME/logs/`

---

## M6: import-codex 与 doctor 的 reasoning 校验

### 评审纠正后的范围

1. 背景收窄：当前启动链路（`src/core.rs:5867`）已对不支持的 effort 警告并置 `None` 继续启动；问题只剩"落盘配置携带非法值"与"doctor 静态误报"两点。
2. import 的全部写入分支（v1 只写了 fallback 注释行）：解析 `src/config.rs:953`、profile 导入构造 `991`、profile 路径 `1047`、flat 合并赋值 `1106`、只读 fallback `858`。每个分支写入前用 `ModelDescriptor::validate_reasoning`（`src/providers/registry.rs:84-98`）校验，不支持则置 `None` 并加 warning；注意 flat 合并是"只合并 Some"，置 `None` 可能留下旧的非法值，需显式清除。wire capabilities 复用运行时的 provider/wire 默认值与校验规则（`ProviderCapabilities::for_wire_api`，`src/backend.rs:135-150`）。
3. doctor 的 `ready=false` 是**新的产品策略**，与运行时"警告但继续"是否一致需要明确决策；输出消费方要全部同步：`ConfigSummary::display`（`src/config.rs:1232`）、`ConfigStatus::is_ready`（`1301`）、手工拼 JSON 的 `doctor_value`（`1423`）、CLI（`src/core.rs:5935/6388`）、TUI（`11429`）、headless（`7430`）；当前两端 `/doctor` 都传 `None`，未用当前项目配置。

---

## M7: Responses wire 的 max_output_tokens

### 评审确认的诊断与纠正

默认值链路（评审确认）：`src/context.rs:8` 默认预算 `128000/8192` → `Agent::new:533` → `context_budget:1721` 按模型 clamp（未知模型变 `32768/4096`）→ 普通请求 `src/core.rs:3808` 无条件传 `Some(reserved_output_tokens)`。只改 backend 无效，这点 v1 正确。纠正：

1. **压缩请求也必须覆盖**：`src/core.rs:1931` 摘要请求始终显式设置上限，普通对话修好后 `/compact` 仍会触发同样的上游 400。要为摘要请求定义兼容策略（同样仅显式发送），同时保持摘要长度验证、预算预留、费用约束一致。
2. **配置链路判断有误**：`ConfigFile/ProviderProfile/EffectiveConfig/ConfigOverrides` 都没有请求级 `max_output_tokens`，顶层配置该字段会被 `deny_unknown_fields` 拒绝。两个选项明确选一：
   - A. 新增请求级可选配置，贯穿解析与工厂
   - B. 把 `model_overrides` 里的 `max_output_tokens`（`src/providers/registry.rs:307` 已用 `sources["max_output_tokens"]=UserOverride` 记录来源）视为"发送上限"的显式信号
   推荐 B，改动面小且来源追踪已有。`ContextBudget` 是数值预算、不承担配置来源，不要为它加 `explicit` 标志（评审列出 10 个生产构造点 + 7 个测试文件会被波及）。
3. **v1 表述自相矛盾**："chat 不受影响"与"一并改为仅显式发送"不能同时成立。按 wire 分别处理：Responses/Chat 仅在显式信号时发送并保留 clamp（先 `if let Some(limit)` 再取 min，v1 的 `Option.min(u64)` 写法不成立）；Anthropic（`src/providers/anthropic.rs:382`）/Google（`src/providers/google.rs:404`）的 builder 会自行默认填值，保持现状不动（"两家协议必填"的外部断言未在线验证，SPEC 不把它当事实）。
4. 测试：加一个"拒绝 max_output_tokens 字段"的 fake endpoint，同时覆盖普通 turn 与 `/compact`；更新 `tests/stage1_model_registry.rs:607/626` 的现有断言；覆盖四种 wire、显式零值、超限值。

---

## 实施顺序

1. M2 → M1（先能修复历史，再放行；可分 PR，M1 加开关）
2. M3、M4（独立小 PR）
3. M7（含压缩路径与 fake endpoint 测试）
4. M5（含 Python 侧两个既有 bug 修复）
5. M6（先决策 doctor 策略再动手）

每个 PR 验收：`cargo fmt --check`、clippy 无新增警告、全量 `cargo test` 绿、对应新测试落地。

## v1 → v2 修订记录

- M1: 新增"排除当前 in-flight operation"（否则健康状态压缩被误否决）；warning 改用非终态事件（`AgentEvent::Error` 会被 view_model 映射成 TurnFailed）；governance 两处改为保持现状（属自动化结算保护，v1 误分类）；补全围栏清单（core.rs:959/2103/2566/3149、backend.rs:1646）
- M2: 修复范围从"只扫 unknown_tool_outcomes"改为"历史配对计算 + dispatch 证据区分"；只补缺失结果；查重键改 `(turn_id, call_id)`；批次收尾后修复；user shell 不合成 `TurnRole::Tool`；"修复"与"裁决"分离；调用点清单更正（无 `Agent::open`）；M1+M2 同 PR 要求放宽
- M3: inspect 数据来源全部更正（evidence 无参数、时间在外层、需脱敏 + UTF-8 边界截断）；分派复用 `headless::recovery_view`；两集合按 operation ID 去重
- M4: 删除虚构的 `/attach remove`；`/input cancel` 真实路径更正；补 TUI/headless 两层更早的阻断源
- M5: logger 状态合并进单 Mutex + Arc 共享；flush 策略；"永远有记录"改"尽力记录"；脱敏复用 `security.rs:230` 现有接口；接入点改为终态边界；补 Python 侧两个既有 bug
- M6: 背景更新（启动链路已警告降级）；import 分支补全（953/991/1047/1106/858）；输出消费方清单补全；doctor ready 策略列为待决策
- M7: 压缩请求纳入；配置链路改推荐 model_override 信号方案（不动 `ContextBudget`）；消除 chat wire 自相矛盾表述；Anthropic/Google 保持现状
- 行号引用全面更正（`config.rs:821→953/991/1106`、`ConfigSummary:1184→1222`、`status_for_profile:1420→1458`、`core.rs:1882/3759→1931/3808` 等，详见评审第 25 条）

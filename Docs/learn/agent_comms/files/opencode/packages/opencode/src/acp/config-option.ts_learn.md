# AC-042 — opencode/packages/opencode/src/acp/config-option.ts

- source_id: `AC-042`
- item_id: `AC-042`
- source_path: `opencode/packages/opencode/src/acp/config-option.ts`
- source_hash: `4545cf9639b12b4a9a0bd28d9a5f61b3d8e7e6706558f04c66454773813ed1a1`
- source_bytes: `6163`
- source_lines: `206`
- coverage: 已按文件顺序读取字节 `1-6163`、行 `L1-L206`（含所有注释、类型、导出和私有辅助函数）；本地 `wc -c -l` 为 `6163` 字节、`206` 行，SHA-256 与元数据一致。

## 完整行为复盘

该文件是 ACP 配置选项的纯函数层：只把 provider/model/variant/mode 数据转换为 `SessionConfigOption` 或 `ModelSelection`，不发起请求、不写状态。顶部导入 `SessionConfigOption` 类型（`L1`），`DEFAULT_VARIANT_VALUE` 导出常量固定为字符串 `"default"`（`L3`）。

- `ConfigOptionModel`（`L5-L9`）要求 `id`、显示用 `name`，`variants` 可选且是 `Record<string, Record<string, unknown>>`；variant 的 value 内容在本文件完全不解释，只有 key 被用于选择和展示。
- `ConfigOptionProvider`（`L11-L15`）包含 provider `id`、`name` 和以模型 id 为键的 `models`。provider/model 的遍历顺序会影响输出；没有去重或冲突检查。
- `ConfigOptionMode`（`L17-L21`）包含 `id`、`name`，可选 `description`。`description` 为空字符串时被视为不存在（见 `L85-L89`）。
- `ModelSelection`（`L23-L29`）把内部选择拆为 `{ model: { providerID, modelID }, variant? }`；variant 不存在代表未显式选择。

`buildModelSelectOption`（`L31-L50`）接收只读 provider 列表、当前 `{providerID, modelID}`、可选当前 variant 和 `includeVariants`。返回固定 `id: "model"`、`name: "Model"`、`category: "model"`、`type: "select"`；`currentValue` 由 `formatCurrentModelId`（`L42-L47`）生成，variant 列表由 `variantsForModel` 查出。`includeVariants` 缺省为 `false`（`L46`、`L48`）：此时当前值只有 `providerID/modelID`，选项只含每个模型的 base 项；为 true 时才把合法非-default variant 展开为额外项。当前模型不存在时 variant 列表为空，仍返回 base 字符串，不报错。选项由私有 `buildModelSelectOptions` 生成，所有读取均为同步只读操作。

`buildEffortSelectOption`（`L52-L73`）接收 variant 字符串数组和当前 variant。数组为空立即返回 `undefined`（`L56`），所以调用方可省略 effort 配置。非空时返回固定 `id: "effort"`、`category: "thought_level"`、`type: "select"` 及固定描述（`L58-L64`）。当前值规则是：当前值恰为 `"default"` 就保留；否则调用 `selectVariant`（`L64-L67`），优先保留存在于数组的当前值，其次选数组中的 `"default"`，最后选数组第一个（`L202-L206`）。选项把原数组与 `"default"` 拼接后以 `Set` 去重，保持首次出现顺序，再用 `formatVariantName` 生成名称（`L68-L71`）；因此 provider 未列出 default 时仍会显示一个显式 default。

`buildModeSelectOption`（`L75-L91`）接收 modes 和当前 mode id，返回固定 `id: "mode"`、`name: "Session Mode"`、`category: "mode"`、`type: "select"`，`currentValue` 原样使用输入（不验证是否在 modes 中）。每个 mode 映射为 `{value: id, name}`，只有 truthy 的 `description` 才展开到对象（`L85-L89`）；空 modes 合法并产生空 options。

`buildConfigOptions`（`L93-L116`）先按当前模型取得 variants，再构造 effort。返回顺序严格为 model、可用时 effort、同时存在 `modes` 与 truthy `currentModeId` 时的 mode（`L101-L115`）。`includeModelVariants` 缺省 false；`modes` 有值但 `currentModeId` 为 `""` 时整个 mode 被省略。它不校验当前模型、mode 或 variant，也不捕获异常；输入访问本身若违反 JavaScript 类型契约会直接抛出运行时错误。

`parseModelSelection`（`L118-L149`）把字符串拆成选择。它先找第一个使 `modelId.startsWith(provider.id + "/")` 的 provider（`L119`），因此 provider id 前缀重叠时是列表顺序优先。找到 provider 后去掉 provider 前缀：若剩余 `modelID` 精确命中 `provider.models`，优先作为含 slash 的完整模型 id（`L121-L124`），不会误当 variant；否则取最后一个 slash（`L126-L130`），仅当 base model 存在且其 variants 含尾段时才返回 base + `variant`（`L130-L132`）。尾段无效则把整个剩余字符串作为 modelID 返回（`L135`），保留未知路径。未找到 provider 时，若无 slash 返回 `{providerID: modelId, modelID: ""}`（`L138-L141`）；有 slash 则按首个 slash 切分 providerID/modelID（`L143-L147`）。函数不返回错误，未知值可正常流通。

`formatCurrentModelId`（`L151-L160`）先拼 `${providerID}/${modelID}`；`includeVariant` 为 false、variants 缺失或为空时直接返回 base（`L157-L159`）。需要 variant 时仍调用 `selectVariant`，所以非法/缺省 variant 会按 default、首项回退；空 modelID 会留下末尾 slash。`formatVariantName`（`L162-L167`）按 `_` 或 `-` 切分，每段首字符大写后以空格连接；连续分隔符产生空段和相应空格，大小写其余部分不改变。

私有 `buildModelSelectOptions`（`L169-L194`）按 provider 输入顺序 `flatMap`。每个 provider 内将 `Object.values(models)` 按 `name.localeCompare` 升序排序（`L173-L176`），生成 base `{value: provider.id/model.id, name: provider.name/model.name}`（`L177-L180`）。未要求 variant 或没有 variants 时只返回 base（`L181`）；否则先返回 base，再按 variant key 插入顺序展开，过滤 `DEFAULT_VARIANT_VALUE`，名称为 `Provider/Model (FormattedVariant)`（`L183-L191`）。不同 provider 的同名 value 不去重。`variantsForModel`（`L196-L200`）精确匹配 provider id 和 model id，找不到时对空对象取 key，返回 `[]`。`selectVariant`（`L202-L206`）无副作用，仅执行“合法当前值 > default > 第一个”的优先级；调用方保证非空数组时才依赖最后一项。

## 状态、取消、恢复与副作用

源内所有函数均为同步纯转换：没有 `AbortSignal`、取消检查、超时、重试、锁、线程/Promise 等并发控制，也没有文件、网络、日志或持久化副作用。输入数组和对象只读；输出是新对象/新数组。不存在错误返回类型；边界输入（未知 provider/model、空 modes、无效 variant、空 modelID）通过回退或原样字符串表达。唯一可能受运行环境影响的是 `localeCompare` 的排序规则（`L175`），这不是共享状态。恢复语义也不存在：若上层保存了 `providerID/modelID/variant`，重新调用这些函数即可重建选项；本文件不会记住上次选择。

## 源内测试与行为判据

同目录测试文件 `opencode/packages/opencode/test/acp/config-option.test.ts` 覆盖主要判据：

- `buildModelSelectOption` 的 ACP 字段、默认不展开 variant，以及 `includeVariants: true` 展开合法 variant 且排除 default（`L49-L89`）。
- effort 的非法当前值回退 default、无 default 时回退首项、空 variants 返回 `undefined`、显式 default 自动加入（`L91-L126`）。
- mode 的描述字段条件展开，以及 `buildConfigOptions` 的稳定顺序 model→effort→mode、无 variants 时省略 effort（`L128-L174`）。
- parser 覆盖普通 provider/model、variant、含 slash 的精确模型优先、含 slash 模型的尾 variant、无效尾段保留（`L176-L206`）。
- 格式化函数覆盖带/不带 variant、variant 回退和 `very_high-effort`→`Very High Effort`（`L208-L240`）。

可独立验证的判据：在 `packages/opencode` 运行该测试文件（例如 `bun test test/acp/config-option.test.ts`）；并补充空 provider、provider 前缀重叠、重复 variants、空 `currentModeId`、空 modelID 的断言，以锁定未被现有测试显式覆盖的边界。

## zenpi Rust 映射

这份源文件本身只提供“选择器/格式化器”，不提供 DAG 通信；zenpi 应复用它的确定性回退思想，同时把通信和生命周期放到已有会话/运行时层。

**配置与 provider 对照。** `src/core.rs` 的 `Agent::set_model`（约 `L1177-L1200`）和 `set_reasoning_effort`（`L1202-L1225`）是当前模型与 effort 的状态入口；`src/providers/**` 的 registry/connection 暴露模型能力（包括 reasoning effort）。建议新增 `src/config_option.rs`：定义 `ConfigOptionModel/Provider/Mode/ModelSelection` 的 Rust 等价物和 `select_variant`、`format_current_model_id`、`build_config_options`，用 `Option` 表达源函数的 `undefined`，用 `Vec` 保持顺序，并为 `localeCompare` 选择明确的 Unicode/字节排序规则。`Agent` 只在验证后调用这些纯函数，再把结果提交到现有 model/effort setter；不要把 DAG 状态塞进 provider client。

**可复用通信原语。**

1. 持久点对点消息使用 `src/protocol.rs` 的 `MailboxRequest::{Send,Receive,Claim,Complete}`（`L81-L115`）和 `src/session.rs` 的 `SessionMailbox::enqueue/list/update`（约 `L3574-L3750`）；`src/headless.rs::mailbox_slash_view`（`L2066-L2112`、执行细节约 `L8940-L9100`）已提供入口。给每个 DAG worker 一个 session id，发送目标只允许同一 session 目录中已登记的 id。parent、grandparent、直接 sibling、直接 child 都通过同一 mailbox 原语寻址；sibling 地址由 DAG 索引解析，不能由 wire path 或未验证文本伪造。
2. 进程内低延迟事件使用 `src/runtime.rs` 的 `BackgroundRunner`、有界 command/event channel 和 `RuntimeEvent::{Accepted,Started,Queued,CancelRequested,Completed,Closed}`（`L56-L76`、`L174-L195`、`L237-L300`）。provider/tool 增量和节点状态广播放入 job 事件；持久 mailbox 只承载需要跨进程/重连的命令与结果，避免把 UI 输出当作可靠队列。
3. 事实审计用 `SessionStore::append_event`、`append_turn`/`append_handoff`；父子关系可利用 `core.rs::Turn::parent_id`（约 `L141-L200`）及现有 session tree 投影（`SessionStore::tree_snapshot/tree_ancestry_turns`，约 `L895-L1068`）。建议新增严格事件类型 `dag_node_started/heartbeat/status/child_spawned/close_decided`，每条含 `session_id,node_id,parent_id,operation_id,epoch`，并用 `operation_started/finished` 的幂等模式保障重放。

**会话父子关系与 DAG 视图。** 新增 `DagNodeId`、`DagNodeRecord { node_id, session_id, parent: Option<DagNodeId>, children: Vec<DagNodeId>, state }` 和 `DagNodeState::{Pending,Running,Green,Failed,Cancelled,Closed}`（建议落在 `src/session.rs` 或独立 `src/dag.rs`）。worker session 的 header/session_id 是 mailbox 身份；`parent` 指直接父节点，祖父通过沿 parent 链得到，sibling 是同一 parent 的 children 交集，child 是 children。不要把 sibling/child 关系仅放内存：以带版本/序列号的 session event 持久化，启动时从事件恢复并校验无环、父存在和唯一 child 边。

**保活、派生和 close 的最小机制。**

- 在 `src/core.rs` 扩展 `Agent` 或新增 `DagCoordinator`：`start_node` 写 started 事件并调用现有 `register_live_owner`（`L821-L845`）；每个 lease 带 `owner_epoch`、`expires_at_ms`、`last_heartbeat`。定时器/调度 tick 调用 `heartbeat_live_owner`（`L836-L844`），超时转为可恢复的 stale，而不是默认为成功。
- `can_close(node)` 必须同时验证自身为 `Green` 且递归遍历所有 child/grandchild 均为 `Green`；任何 Running/Pending/Failed/Unknown 或缺失子记录都拒绝 close。只有判定为真，才写 `close_decided`、`finish_operation(Succeeded)`，调用 `settle_blueprint_worker`（`src/core.rs` 约 `L2827-L2845`）并释放 live owner。该门槛是 DAG 需求的硬不变量，不能由单个 worker 的“我完成了”声明绕过。
- 若 `can_close` 为假但节点仍有新工作，先 `renew_blueprint_worker`（`L2847-L2870`）/heartbeat 保活，再用 `BackgroundRunner::try_submit` 派生一个带新 `JobId`、新 `operation_id`、`parent=node_id` 的 worker；写 `child_spawned` 后才向 child mailbox 发送任务，重复投递以 `message_id/request_id` 去重。已有 Running child 不重复派生，失败/过期 child 需通过显式 recovery 决策重试。
- 调度与取消连接 `src/runtime.rs::CancellationToken`（`L56-L100`）及 `src/tool_runtime.rs::execute_tool_batch` 的 `cancelled` 检查（约 `L157-L349`）：取消只阻止新副作用并等待 cooperative worker 结束，不能把未确认的 side effect 当作回滚。tool side effect 需要 `src/approval.rs::ApprovalCoordinator`（约 `L126-L160`）和现有 durable approval 事件；DAG close 不能跳过 approval 或 recovery。
- host 层 `src/headless.rs::run_async_streams`（`L806-L865`）负责把 runtime 事件按 job 关联输出，`mailbox_slash_view` 负责跨 session 命令；完成事件之后才允许 coordinator 做上述 close/settle，防止“已发 Completed 但子任务尚未回收”。

**差异清单与可执行验证。**

1. 源 TS 没有错误/取消/持久化；Rust 新模块必须定义 `Result` 错误（未知节点、环、过期 lease、非法状态迁移）并为每个 mailbox/事件操作写幂等测试。
2. 源的 `DEFAULT_VARIANT_VALUE` 回退与“精确 slash 模型优先”应在 Rust 单测中逐项复刻；再验证 `Agent::set_model/set_reasoning_effort` 与 `src/providers/**` capability 不会接受不支持的 effort。
3. 增加 DAG 集成测试：构造 parent→child→grandchild 及 sibling，断言四类 mailbox 均可收发；只把自身标绿时 close 被拒；全部后代标绿才产生 `close_decided`/settle；运行中发送新工作会 heartbeat+派生一次，重复 request 不产生第二个 child。
4. 运行时测试应断言 `Accepted→Started→Completed→Closed` 顺序、取消产生 `CancelRequested`、队列满时保留待派生任务；session 重启后从 JSONL 恢复 lease/节点状态，未知 outcome 必须停在显式 recovery，而不能自动 close。

## 未决问题

1. 源文件只依赖 SDK 的 `SessionConfigOption` 类型，未定义其 options 的 schema 校验、传输版本和客户端对未知 `category` 的行为。
2. `providers.find(startsWith)` 的前缀重叠语义是实现现状而非明确协议；Rust 是否要拒绝重叠 provider id 需产品决策。
3. DAG 节点状态的“绿色”是否等同于 `OperationOutcome::Succeeded`，以及 child 的失败是否允许人工重试后复用同一 node id，源文件无法确认；建议采用新 operation id、保留旧 terminal 记录。
4. lease TTL、heartbeat 周期、派生并发上限和 sibling 消息顺序未由该 TS 文件规定，需要结合 zenpi 的资源/worker budget 配置确定。

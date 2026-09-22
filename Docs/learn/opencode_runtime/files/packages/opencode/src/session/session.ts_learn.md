# OC-029 — packages/opencode/src/session/session.ts

- source_id/item_id: `OC-029`
- source_path: `packages/opencode/src/session/session.ts`
- source_hash: `0c56ae3535e29cae0de51156eaba2842c0896309f1d6b12525566b4a8ba4c7f2`
- source_bytes: `35643`
- source_lines: `1016`
- coverage: 已按顺序读取完整字节范围 `1-35643`、行范围 `L1-L1016`（含注释、类型、导出与尾部 re-export）。

## 完整行为复盘

文件把会话资料、消息/part 事件和查询 API 组合成 Effect 服务。`parentTitlePrefix`/`childTitlePrefix` 是默认标题前缀（L48-L49）。`isDefaultTitle(title)` 用严格 UTC 毫秒时间戳正则判断父/子默认标题，格式之外返回 false（L51-L55）。`fromRow(row)` 将 `SessionTable` 行解码为 `Info`：summary 三字段任一非 null 才生成对象，数值缺省 0、diffs 缺省 undefined；share/revert、workspace/path/agent/model、permission、时间字段逐一做 null 到 undefined 转换，并用 `MessageID/PartID/ModelV2.ID/ProviderV2.ID` 构造品牌 ID（L57-L118）。`toRow(info)` 做反向数据库投影，缺失 cost/tokens 取 `EmptyTokens`/0，revert 缺失写 null，时间和 JSON metadata 原样落库（L120-L159）；二者需保持可逆但 `undefined/null` 语义有意不同。

内部 `getForkedTitle` 识别尾缀 ` (fork #N)` 并递增，否则追加 `#1`；非匹配标题不会截断（L161-L168）。`sessionPath(worktree,cwd)` 求相对路径并统一 `/` 分隔符（L171-L173）。`Summary`、`Tokens`、`Share`、`Time`、`Revert`、`Model` schema 定义运行时校验；`EmptyTokens` 全部为 0；`ArchivedTimestamp` 仅允许有限数（兼容历史负值），`Info` 是完整会话结构，含 `SessionID/project/workspace/directory/path/parent/title/agent/model/version/metadata/time/permission/revert`，`ProjectInfo`/`GlobalInfo` 分别扩展项目摘要和可空 project（L175-L258）。`CreateInput` 可整体省略，字段均可选；`ForkInput`、`GetInput`、`ChildrenInput`、`RemoveInput`、`SetTitleInput`、`SetArchivedInput`、`SetMetadataInput`、`SetPermissionInput`、`SetRevertInput`、`MessagesInput` 提供边界输入 schema（L260-L300）。`ListInput` 支持 directory/scope/path/workspaceID/roots/start/search/limit，`GlobalListInput` 另有 cursor/archived（L302-L321）。`Event` 将 V1 Created/Updated/Deleted/Diff/Error 常量透传（L323-L329）。

`plan(input,instance)`：VCS 项目写入 `<worktree>/.opencode/plans/<created>-<slug>.md`，否则写入 `Global.Path.data/plans`（L331-L336）。`getUsage` 对 usage token 与 metadata 做安全归一：非 finite 或负数归零；从 Anthropic/Vertex/Bedrock/Venice 多个 metadata 键兜底读取 cache write；因 AI SDK v6 input 已含 cache，计算 non-cache input 时减去读写缓存并夹到 0。输出 `tokens`（total 保留原值、input/output/reasoning/cache 分栏）和 `cost`；按 context tier 最大匹配、`experimentalOver200K` 或普通 cost 选价，百万 token 换算，reasoning 按 output 价，Copilot 合法 `totalNanoAiu` 则优先换算。Decimal 避免浮点累计溢出（L338-L405）。`BusyError` 是带 `sessionID` 的 tagged error；`NotFound` 别名为 `NotFoundError`（L407-L411）。

`Interface` 声明所有服务操作及 Effect 错误类型：list/listGlobal/create/fork/touch/get/setTitle/setArchived/setMetadata/setAgentModel/setPermission/setRevert/clearRevert/setSummary/setShare/setWorkspace/diff/messages/children/remove/updateMessage/removeMessage/removePart/getPart/updatePart/updatePartDelta/findMessage（L413-L472）。`Service` 是 Context.Service，`use` 通过 `serviceUse` 取得服务，`Patch` 允许除 time/share/summary/revert/permission 外的 Partial 字段，并对这些嵌套字段定义 null 清除语义（L474-L484）。

`layer` 注入 `Database.Service`、`BackgroundJob.Service`、`EventV2Bridge.Service`、`RuntimeFlags.Service`（L486-L497），其内部函数如下。

- `createNext(input)` 从 `InstanceState.context` 取得 project，生成 descending `SessionID`、随机 slug、安装版本、目录/路径/父子关系；标题默认是前缀加 `new Date().toISOString()`；permission 复制数组，cost/tokens 初始化为零，created/updated 为 `Date.now()`。先日志再发布 `SessionV1.Event.Created`，返回内存 `Info`；数据库最终持久化依赖事件桥（L499-L539）。
- `get(id)` 按主键查询，DB 错误 `orDie` 转致命缺陷；无行抛 `NotFoundError("Session not found: id")`，否则 `fromRow`（L540-L545）。
- `list(input?)` 取实例上下文 project，合并 workspace feature flag 后委托 `listByProject`（L546-L554）。
- `listGlobal(input?)` 构造 directory、roots、start、cursor、title like 条件；默认隐藏 archived，默认 limit 100；按 updated/id 倒序。批量查 ProjectTable 建 map，返回带 `project`（找不到为 null）的 `GlobalInfo`；DB 错误 `orDie`（L555-L595）。
- `children(parentID)` 精确查 parent_id，返回 `fromRow` 数组；错误 `orDie`（L596-L605）。
- `remove(sessionID)` 先 `get`，探测是否有实例目录；有则 `cancelBackgroundJobs`，递归删除所有 children，发布 Deleted 并调用 `events.remove`。整个主体 try/catch，失败仅记录 error 日志，因此除初始 get 外删除副作用失败会被吞掉；递归顺序是先子后父（L606-L628）。
- `updateMessage(msg)` 发布 MessageUpdated 并原样返回，带 span；`updatePart(part)` 发布 PartUpdated，part 用 `structuredClone` 防止事件消费者修改原对象，并附 `Date.now()`（L629-L644）。`getPart` 按 session/message/part 三键查 PartTable；未找到返回 undefined，找到时把 row.data 与键合并成 `SessionV1.Part`（L645-L666）。
- `create(input?)` 从实例上下文得到 directory/worktree 和 workspaceID，计算相对 session path，缺省 workspace 取实例 workspace，然后调用 `createNext`（L667-L690）。
- `fork({sessionID,messageID?})` 读取原会话并生成递增 fork 标题；复制 workspace、metadata（深拷贝），随后读取原消息。target 为指定 message 的索引或全部长度；指定 ID 不存在时 `findIndex=-1`，按实现会复制全部。逐条生成新 MessageID，重写 assistant parentID 到 idMap，逐 part 生成新 PartID/session/message 关联；compaction 的 `tail_start_id` 也通过 idMap 重写。只发布新消息/part 事件，不复制原 agent/model/summary/cost（L691-L733）。
- `patch(sessionID,info)` 先 get，再浅合并；time、share、summary、revert、permission 分别合并/清除：null 清除，undefined 保留当前（summary 等显式值覆盖）；发布 Updated，未直接写 DB（L734-L748）。`touch` 更新 updated；`setTitle` 直接标题；`setArchived` 写 archived（可传负数、undefined 表示清除）；`setMetadata` 更新 metadata+当前时间；`setAgentModel` 同时写 agent/model/给定 time；`setPermission` 复制 ruleset；`setRevert` 同时写 summary/revert；`clearRevert` 以 null 清除；`setSummary`、`setShare`（null 清除）、`setWorkspace` 都更新时间；这些包装器均 `Effect.orDie`，找不到会变成致命缺陷而非可恢复 NotFound（L749-L822）。
- `diff(sessionID)` 当前忽略参数并返回空 `Snapshot.FileDiff[]`，是明确的占位行为（L823-L827）。`messages({sessionID,limit?})` 有 limit 时直接取 `MessageV2.page` 单页；无 limit 时以 50 分页，逐页反向压入再整体 reverse，得到时间正序；page 空、无 more 或无 cursor 即止，底层 DB 错误传播（L828-L852）。`removeMessage`/`removePart` 只发布相应删除事件并回传 ID，没有在此处删除行（L853-L876）。`updatePartDelta` 发布 `MessageV2.Event.PartDelta`，无 span（L877-L887）。`findMessage(sessionID,predicate)` 同样每页 50，但每页从末尾向前扫描，故新到旧，首个 predicate true 返回 `Option.some`；遍历完返回 `Option.none`（L888-L905）。`Service.of` 在 L906-L936 汇总上述实现。

文件级 `cancelBackgroundJobs(background,sessionID)` 先列 job，只选 running 且 id、metadata.sessionId 或 metadata.parentSessionId 命中，会用 `Effect.forEach(...,{concurrency:"unbounded",discard:true})` 并发取消，取消失败被丢弃（L938-L953）。`listByProject(db,input)` 总是按 project_id 过滤；可加 workspace、path（精确或子路径 `/.../%`，必要时允许 directory 的 null path 回退）、scope=project 时跳过 directory 过滤、roots/start/search；默认 limit 100，按 updated 倒序，DB 错误 `orDie`，映射 `fromRow`（L955-L1009）。`node` 用 LayerNode 声明依赖节点，末行 `export * as Session from "./session"` 提供命名空间再导出（L1010-L1016）。

## 状态、取消、恢复与副作用

状态由 `SessionTable`/`PartTable` 行和事件桥共同承载；create/patch/update/remove 等首先发布事件，实际持久化、广播及 `events.remove` 的实现不在本文件。`remove` 会递归子会话并取消当前会话及其父子 metadata 关联的运行 job；取消是并发、尽力而为且不等待已完成 job。分页读取没有显式超时或重试；Effect 失败通常由 `orDie` 变成致命缺陷。`messages`/`findMessage` 通过 cursor 分页避免一次加载无限历史，但单次 limit 未限制上界（由 MessageV2.page 决定）。fork 是顺序生成 ID 和事件，未见事务/回滚；中途失败可能留下部分新会话事件。没有恢复/重试状态机，`BusyError` 仅定义未被本文件使用。`setArchived` 的 undefined 清除归档，Global 列表默认排除 archived。外部副作用包括 DB select、EventV2Bridge publish/remove、BackgroundJob cancel、日志和计划路径计算；`diff` 尚未读取快照。

## 源内测试与行为判据

源文件及同目录 `src/session/` 未包含测试文件，源内未包含测试。可独立验证：用内存/测试 DB 插入含 null 与非 null summary、revert、cache token 的行，断言 `fromRow`/`toRow` 的 null/0/undefined 规则；构造分页 MessageV2 数据验证 `messages` 正序和 `findMessage` 新到旧；给出不存在的 fork messageID 验证复制全部；创建带 parent/metadata 的 jobs 验证 `remove` 的递归事件及取消筛选；用非 finite/负 usage 与各 provider metadata 检验 `getUsage` 的非负 token/cost 和 Copilot 优先级；调用 `listByProject` 验证 workspace/path/scope/roots/start/search 条件和 limit 100 默认值。

## zenpi Rust 映射

- `zenpi/src/session.rs` 的 `SessionStore`（JSONL header/turn/event、`SessionError`、`fork_to`、archive/catalog 与 `list_sessions/search_sessions`，如 `L327-L434`、`L1410-L1601`、`L1655-L1724`）是 `Info`/`create/get/list/fork/remove` 的主要落点。建议新增 `SessionInfo`、`SessionPatch`、`SessionSummaryTokens`，把 `fromRow/toRow` 改为 serde/JSONL projection；用 `SessionStore::append_event` 对应 EventV2Bridge。
- `zenpi/src/core.rs` 的 `Agent` 持有 `SessionStore`、active turn、recovery 和 approval（字段约 `L409-L455`，构造/恢复约 `L607-L713`），可承载 `Service` 的业务门面：`create`/`fork`/`messages` 接到 Agent 或独立 `SessionService`，并沿用 `AgentError` 将 NotFound、I/O、取消分类。core 已有显式 interrupted/unknown outcome 与 retry-confirmation（`L1736-L1942`），比 TS 只有 `BusyError` 更强，需要保持“未知副作用不得自动重试”。
- `zenpi/src/headless.rs` 是 JSONL 外部边界，协议请求/响应和 bounded replay（文件头常量及 `TurnMode` 约 `L1-L120`）可映射 `listGlobal`、分页 cursor、setArchived；不要把事件回放与 SessionStore durable cursor 混淆。把 `SessionV1.Event.*` 映射为 `AgentEvent`/`StdioEvent`，并设置与 `MAX_SESSION_BYTES` 类似的请求/分页上限。
- `zenpi/src/runtime.rs` 的 `CancellationToken`、`BackgroundRunner::try_cancel/try_shutdown`（约 `L49-L93`、`L318-L385`）对应 `cancelBackgroundJobs`；实现 `SessionJobIndex` 按 session_id/parent_session_id 建索引，取消用 bounded channel/有界并发，记录每个失败而不是 TS 的 discard。超时采用 runtime grace，区分 cooperative cancellation 与已发生副作用。
- `zenpi/src/tool_runtime.rs` 的 `ToolBatchOptions`、cancellable batch 和“executor 不重试、owner 负责 journal”约束（文件头至 `L120`、执行段约 `L400-L445`）可作为 session message/part 更新的事件边界；`updatePartDelta` 对应 `ToolProgress`/provider delta，需保留顺序和 batch 原子判据。
- `zenpi/src/protocol.rs` 的 `TurnMode`、`CheckpointCursor`、mailbox/分页协议（`L1-L100`）适合承载 `MessagesInput.limit`、`findMessage` cursor 与 archive/filter 参数；协议层应拒绝超大 limit、未知字段，并把 `NotFound` 编码成可恢复响应。
- `zenpi/src/approval.rs` 已有 `ApprovalCoordinator` 的持久化、`cancel_all`、`emergency_cancel`、cancellation epoch（约 `L127-L198`、`L371-L446`），与 TS session permission/ruleset 不同：建议将 `Info.permission` 映射到 `ApprovalPolicy` 快照，setPermission 事件后更新 policy，并把 remembered approval 事件写入 SessionStore。
- `zenpi/src/providers/**`（`anthropic.rs`、`google.rs`、`openai.rs`、`codex.rs`、`deepseek.rs`、`connection.rs`、`registry.rs`）输出统一 `Usage`/`ProviderEvent`；在 provider 聚合层实现 `getUsage` 的 cache metadata 兼容和 Decimal/整数微单位计费，context tier 选择需显式测试。当前 provider usage 类型与 TS `Provider.Model.cost` 的 tier/experimentalOver200K 不完全同构，建议新增 `ModelPricing` 与 `UsageAccounting`。

可执行差异清单：1）补 `SessionService` trait 与 `SessionInfo` schema，覆盖 TS Interface 的 25 个方法并为每个返回 `Result`；2）为 session event 建持久化 envelope（created/updated/deleted/message/part/delta），保证 JSONL append 后崩溃可恢复；3）实现 fork 的 message/part ID 映射及 compaction tail 重写，并为中途失败加事务式临时文件或可重放标记；4）实现 project/workspace/path/root/search/archived 查询和 100 默认 limit；5）实现有界取消索引、超时 grace、重试确认和未知 outcome；6）补 usage cache 多 provider 解析、分层价格和非负 finite 约束；7）将 TS 的空 `diff` 明确标成 Rust `DiffProvider` 未实现错误或真实快照查询，禁止静默成功。

## 未决问题

1. `EventV2Bridge` 如何把 publish 映射到 `SessionTable`/`PartTable` 的最终写入、是否事务化，源文件未定义。
2. `SessionV1.Event.Diff/Error` 的生产者和消费者、以及 `Snapshot.FileDiff` 的真实 diff 来源无法从本文件确认。
3. `MessageV2.page` 的排序、cursor 稳定性和 limit 上限由同目录另一文件实现，本文件只假定其返回 `{items,more,cursor}`。
4. `BusyError`、`locationServiceMapLayer`、`SessionExecutionLocal` 在此文件中未参与公开服务行为，具体用途需查调用方。

# OC-005 — packages/opencode/src/permission/index.ts

- source_id/item_id：OC-005
- source_path：packages/opencode/src/permission/index.ts
- source_hash：5b9e4aa65290a39363722b9fae4c68080188d8ed76896afa0d96cd9dbfd2821d
- source_bytes：7861
- source_lines：223
- coverage：已从字节 0 顺序读到 7861（完整范围 [0,7861)），行范围 L1-L223，包含全部导入、类型、实现、导出和末尾命名空间导出。

## 完整行为复盘

1. **依赖与公开符号（L1-L26）**
   - LayerNode、InstanceState、EventV2Bridge 组成实例化和事件发布基础；Wildcard 执行 glob 匹配；Deferred、Effect、Layer、Context 提供异步等待、依赖注入和生命周期；os 只用于 home 目录展开（L1-L8）。
   - Event 直接别名为 PermissionV1.Event，因此事件类型是 permission.asked 与 permission.replied（L10）。
   - Interface 暴露三个 Effect 函数：ask(input) 成功 void、失败 PermissionV1.Error；reply(input) 成功 void、失败 NotFoundError；list() 返回只读 Request 数组（L12-L16）。输入的结构来自 PermissionV1.AskInput/ReplyInput：请求含可选 id、sessionID、permission、patterns、metadata、always、可选 tool 与 ruleset；回复为 requestID、reply（once|always|reject）和可选 message。
   - 内部 PendingEntry 把公开请求和 Deferred<void, RejectedError|CorrectedError> 绑定；State 保存 pending: Map<ID, PendingEntry> 与进程内 approved: Rule[]（L18-L26）。

2. **evaluate（L28-L38）**
   - 输入一个 permission、一个具体 pattern 和任意多个 Ruleset；先 flat() 按参数顺序拼接，再从尾部 findLast，只保留同时满足 Wildcard.match(permission, rule.permission) 与 Wildcard.match(pattern, rule.pattern) 的最后一条规则（L28-L33）。
   - 没有命中时返回默认 { action: "ask", permission, pattern: "*" }（L32-L37）。因此规则优先级是“最后匹配胜出”，不是按通配符具体程度排序；跨 ruleset 的后一个数组也会覆盖前一个。空数组、未知权限、无匹配都进入 ask。一个请求的多个 pattern 必须分别评估。

3. **Service、layer 与状态生命周期（L40-L65、L174-L176、L221-L223）**
   - Service 是 Context tag @opencode/Permission（L40）；layer 用 Layer.effect 构造服务，先取得 EventV2Bridge.Service，再通过 InstanceState.make<State> 创建每个实例/目录作用域的状态（L42-L47）。初始 pending 为空、approved 为空（L49-L53）。
   - 状态注册 finalizer：实例终止、dispose 或 reload 时遍历所有 pending，向每个 deferred 发送 PermissionV1.RejectedError，随后清空 Map（L54-L61）。这使等待者失败关闭而不会永久挂起；没有把 approved 写盘的逻辑。
   - 服务返回 { ask, reply, list }（L174），node 声明依赖 EventV2Bridge.node（L221）；最后 export * as Permission from "." 提供命名空间式导入（L223）。

4. **ask（L67-L107）**
   - 先取同一实例的 approved 与 pending，拆出 ruleset，其余字段组成 request（L67-L70）。按 request.patterns 顺序逐项调用 evaluate(request.permission, pattern, ruleset, approved) 并记录 info 日志（L72-L74）。遇到第一条 deny 立即失败，错误携带 ruleset.filter(...) 后的、只匹配该权限名的输入 ruleset；此前的 ask pattern 不会创建 pending（L75-L79）。allow 继续检查，其他动作（通常 ask）把 needsAsk 置真（L80-L82）。
   - 所有 pattern 都 allow 时立即成功 void，不发事件、不写 pending（L84）。只要有一个需要询问，则使用调用者 id 或 PermissionV1.ID.ascending()（L86），把 session、权限、patterns、metadata、always、tool 复制成公开 info，不把 ruleset 放进请求（L87-L95）。
   - 创建带 RejectedError|CorrectedError 类型的 Deferred，写入 pending，发布 Event.Asked，然后等待 deferred；Effect.ensuring 保证等待被中断或完成后删除该 id（L98-L106）。回复前 list() 能看到这份 info。代码没有重复 ID 检查，显式重复 id 会覆盖 Map 中旧 entry；实现依赖调用方保证 ID 唯一。
   - 事件发布发生在 ensuring 建立之前；若事件桥本身失败，ask 会失败且该 pending 可能残留，正常事件桥则不会出现此边界。

5. **reply（L109-L167）**
   - 按 requestID 查 Map；不存在立即失败 PermissionV1.NotFoundError({requestID})（L109-L113）。找到后先删除自身 pending，再发布 Event.Replied，事件载荷含原 sessionID、requestID 和回复动作（L115-L119）。
   - reject 分支：有非空 input.message 时向自身 deferred 发送 CorrectedError({feedback})，否则发送 RejectedError（L121-L127）。随后扫描剩余 pending，只处理同一 sessionID：逐项删除、发布 "reject" 回复事件并失败为普通 RejectedError（L129-L139）。因此一个 session 的一次拒绝会取消该 session 的所有其他等待；其他 session 不受影响。
   - 非 reject 先成功自身 deferred（L142）。once 到此返回，不改变 approved（L143）。always 则对 existing.info.always 的每个 pattern 追加 {permission: existing.info.permission, pattern, action:"allow"} 到 approved，不去重且只使用 always，不是 patterns（L145-L151）。
   - 接着只扫描同 session 的其他 pending；当其每一个 item.info.patterns 都能在当前 approved 中评估为 allow 时，删除它，发布回复 "always"，并成功其 deferred（L153-L166）。这是一种同 session 的批量唤醒；其他 session 保持 pending。reply 的 Map 删除和事件发布顺序意味着事件失败会发生在 deferred 完成之前。

6. **list（L169-L172）**
   - 只取当前实例状态的 pending，按 Map 插入迭代顺序返回每个 entry 的 info；返回快照数组，不暴露 deferred 或可变 Map（L169-L172）。没有过滤 session、超时或分页。

7. **路径与规则转换函数**
   - expand 只展开开头的 ~/、精确 ~、$HOME/、开头 $HOME 为 os.homedir()；中间出现的 ~ 不变（L178-L184）。$HOMEfoo 也会按前缀规则拼接 home，调用方需自行避免歧义。
   - fromConfig(permission) 遍历 ConfigPermissionV1.Info 的原始键顺序（L186-L197）。字符串值变成 {permission:key, pattern:"*", action:value}；对象值的每个 [pattern, action] 变成规则，并对 pattern 调用 expand。空配置返回空数组；不排序、不按 specificity 合并。
   - merge(...rulesets) 仅 flat() 连接并保留原顺序（L200-L202），没有去重、冲突检测或复制深层对象；调用者应把默认规则放前、配置/批准规则放后。
   - disabled(tools, ruleset) 把 edit/write/apply_patch 归一为权限 edit，三个 MCP 资源工具归一为 read，其余工具名原样作为权限（L204-L210）。对每个工具只看 ruleset.findLast 的最后一个匹配权限规则；只有该规则同时是 pattern === "*" 且 action === "deny" 才加入返回 Set（L210-L213）。例如仅拒绝 rm * 不会隐藏整个 bash 工具，后续 wildcard allow 也可覆盖前置 deny。
   - visibleTools 先求隐藏集合，再过滤 Record<string,T> 的键值并重建对象，保留未禁用工具及其值（L216-L219）。

## 状态、取消、恢复与副作用

- 等待模型是 Deferred，没有内置 timeout、重试、退避或持久化；超时应由外层 Effect 通过中断实现，中断会触发 ask 的 ensuring 删除 pending（L101-L106）。实例 finalizer 则把全部 pending 失败为 RejectedError（L54-L61）。
- reply 没有幂等重放：第一次删除 pending 后，重复回复得到 NotFoundError（L111-L119）。always 的批准只存于当前 InstanceState 的内存 approved 数组；reload/dispose 会丢失，源文件没有恢复机制或磁盘写入。
- 并发协调没有显式锁、原子 Map 操作或队列；同步 JS 段落中的插入/删除顺序决定结果，Effect 调度负责挂起与唤醒。应特别验证重复 ID、同 session 多请求同时 reply、reply 与 instance dispose 竞态。pending 与 approved 是实例隔离的，测试显示不同 directory 的请求互不相见。
- 外部副作用只有 EventV2Bridge 的 Asked/Replied 发布、Effect.logInfo 日志和 os.homedir() 环境读取；工具本身不在此文件执行。reject 会为级联取消的每条请求各发一条 replied 事件（L129-L137）。

## 源内测试与行为判据

源文件本身没有测试代码；相邻测试 /Users/wangweiyang/GitHub/opencode/packages/opencode/test/permission/next.test.ts 覆盖了可独立验证的判据：

- fromConfig 的字符串/对象、空配置、home 展开、键顺序与 fallback-first 规则见 next.test.ts L79-L191；应断言 ~/x、$HOME/x 展开后可被 evaluate 命中。
- merge 只拼接且保序、后 ruleset 覆盖默认 ask/allow 见 L195-L276。
- evaluate 的 exact/glob、通配权限、最后匹配胜出、无匹配默认 ask 见 L280-L448。
- disabled 的工具归一、全局 deny、后置 allow 覆盖、只拒绝具体 pattern 不隐藏工具见 L452-L554。
- ask 的 allow 立即成功、deny 为 DeniedError、ask 进入 pending、请求字段与 Asked 事件见 L558-L695；reply 的 once/reject/带 message 的 CorrectedError/always 批准、同 session 级联和事件见 L699-L964。
- directory 隔离、dispose/reload 失败关闭、未知 requestID、先 ask 后 deny 不产生 pending、实例 reload 清理 pending 见 L966-L1174。独立验证还应检查重复 ID 覆盖风险和事件发布失败边界，因为现有测试未覆盖它们。

## zenpi Rust 映射

- **建议落点**：新增 src/permission.rs 并在 src/lib.rs 导出，定义 PermissionAction { Allow, Deny, Ask }、PermissionRule、PermissionRequest、PermissionReply { Once, Always, Reject }、PermissionService。PermissionService 以 workspace/session owner 为作用域，持有 BTreeMap<PermissionId, PendingPermission>、批准规则向量和 Condvar/channel；纯函数 evaluate、from_config、merge、disabled、visible_tools 可直接单测。Wildcard.match 应明确采用与 TypeScript 相同的 glob 语义并以最后匹配为准。
- **src/approval.rs 对照**：现有 ApprovalCoordinator 已有 pending、respond_with_request、cancel_all、emergency_cancel、remember 与持久化前 fail-closed 语义（L163-L242、L289-L340、L366-L429），可复用其等待/取消骨架；但它的请求是单工具 ApprovalRequest，回复只有 Allow/Deny + remember，没有 patterns、always、同 session 级联或 CorrectedError。建议让新 PermissionService 负责规则层，再由 core.rs 的最终工具审批继续经过 ApprovalCoordinator，避免把 provider 名称当作授权。
- **src/core.rs 对照**：ToolRuntime 已保存 approval、approval_policy（L441-L450），set_approval_policy 从 session 事件恢复 remembered policy（L1496-L1518），prepare_tool 在 L4650-L4860 先做工具/技能/不可变 gate，再调用 ApprovalPolicy::decide_after_preflight，必要时创建请求、等待、持久化 approval_resolved 并才允许 dispatch。可在 prepare_tool 的 gate 与 ApprovalCoordinator 之间插入 PermissionService::evaluate；deny 应变成 ToolFailure::PolicyDenied，ask 则暴露 patterns/metadata，always 后把批准规则写入会话恢复数据。
- **src/headless.rs 对照**：drain_approval_events（L4177-L4248）把 coordinator 请求投影为 JSONL approval_request/view 事件；异步循环维护每个 project 的 coordinator（L4660-L4679），EOF 用 cancel_all、显式 shutdown 用 emergency_cancel（L4682-L4696）。新增权限服务时应沿同一 project owner 选择、事件序列和 replay 路径发布 Asked/Replied；同 session 的级联回复必须为每条被取消请求发 terminal event。
- **src/protocol.rs 对照**：现有 Command::Approve { approval_id, decision, remember } 与 approve|approval 解析（L187-L192、L257-L261、L501-L513）可作为传输入口。要映射 once|always|reject，建议新增版本化 PermissionReply 字段（保留旧 Approve 兼容），校验 ID、可选 feedback/message，并把 unknown request 映射成明确协议错误。
- **src/session.rs 对照**：append_event/events 是持久化入口（L1207-L1223）；当前 ApprovalPolicy::with_remembered_events 只恢复工具级 approval_resolved 且拒绝 worker 来源。若要等价 approved: Rule[]，需追加 permission_replied 或 permission_rule_added 事件，在 session reload 时按事件顺序重建规则；dispose/reload 必须像 TypeScript finalizer 一样唤醒并拒绝所有 pending。
- **src/runtime.rs 对照**：CancellationToken 是协作取消，后台 runner 的 queue、cancel、shutdown 和有限 grace period 位于 L49-L99、L308-L360。把 token 的 is_cancelled 注入权限等待；取消只保证等待者失败和不再 dispatch，不能回滚已开始副作用，符合源文件没有重试/回滚的事实。
- **src/tool_runtime.rs 对照**：execute_tool_batch（L157-L249）接收已由 host prepare 的 ToolBatchDecision，按源顺序执行并在取消/不确定副作用时停止；它不应自行推断权限。将 PermissionService 的 allow/deny/ask 结果转换为 ExecuteApproved 或 Reject(ToolFailure)，保留现有 dispatch 前 preview/gate 再检查。
- **src/providers/** 对照：anthropic.rs、openai.rs、google.rs、codex.rs、deepseek.rs 及 registry.rs 目前没有 permission/approval 规则；它们只描述 provider wire/capability。权限评估应留在 host/core，provider 只能产生待检查的 tool call，避免 provider 响应绕过本地规则。

差异清单：TypeScript 是按 InstanceState 的目录作用域、Effect Deferred 和内存 approved 工作；zenpi 是跨 headless/TUI 的进程级 coordinator、Condvar、JSONL 事件和 session journal。TypeScript 的 always 是 pattern 规则且自动唤醒同 session，Rust 当前 remember 是按 tool 的持久 allow/deny；TypeScript reject + message 产生 CorrectedError，Rust 目前只有 ApprovalError::UnknownRequest/Cancelled/Invalid；TypeScript evaluate 明确支持多 pattern 与最后匹配，Rust 需补齐此纯规则层。验证时应同时覆盖 workspace 隔离、取消/EOF fail-closed、持久化失败不放行、同 session 级联和重复回复。

## 未决问题

- InstanceState.make 是否保证同一实例内 Effect 并发访问 Map 的串行化，源文件未给出；重复 id 的覆盖行为也未被防护或测试。
- EventV2Bridge.publish 失败时，ask 在进入 Effect.ensuring 前可能留下 pending，reply 在 deferred 完成前可能已删除 pending；事件桥错误下的恢复策略需由上层确认。
- Wildcard.match 的转义、路径分隔符和 glob 边界语义定义在外部模块，本文件无法确认。

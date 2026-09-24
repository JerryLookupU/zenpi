# AC-044 — opencode/packages/opencode/src/acp/directory.ts

- source_id/item_id: opencode/packages/opencode/src/acp/directory.ts / AC-044
- source_path: opencode/packages/opencode/src/acp/directory.ts
- source_hash: c4f0f38ef433713afdd50f89d7ecf2ce8130f5104dc7606a2ff3f090726109b4
- source_bytes: 7273
- source_lines: 212
- coverage: 已从首字节到末字节完整读取（字节 1-7273），覆盖源文件全部 L1-L212，含导入、类型、注释、实现、LayerNode 导出及最终重导出。

## 完整行为复盘

文件实现 ACP 目录快照的类型、构造、加载和按目录缓存。依赖均通过 Effect Context 注入；加载阶段把项目实例上下文固定到后续 provider/agent/command 查询中。

- 导入与类型边界（L1-L11）：依赖 Agent、Command、InstanceRef、InstanceStore、Provider、Effect 原语及 ProviderV2/ModelV2 类型。InstanceBootstrap 在 L4 被导入但本文件没有使用。错误类型只以 type-only 的 ACPError.Error 参与 Effect 失败通道；实现不在此处展开错误枚举。
- ModelOption（L13-L18）：只读模型选择项，包含 providerID/providerName/modelID/modelName。它是展示投影，不携带变体或执行句柄；字段均必填。
- ModeOption（L20-L24）：只读模式 id/name，description 可省略。加载器把可见、非 subagent agent 投影成此结构。
- ModelVariants（L26）：NonNullable<Provider.Model["variants"]> 的别名，保证使用变体时不是 null/undefined。
- DefaultModel（L28-L31）：provider 与 model 的稳定 ID 对；可选字段只在快照顶层出现。
- Snapshot（L33-L42）：目录快照的完整输出。包括原始 providers、排序后的 modelOptions、以 providerID/modelID 为键的 variantsByModel、模式列表及有效默认模式、命令列表，以及可选 defaultModel。快照本身没有时间戳或版本号，因此刷新语义由服务层决定。
- LoaderInterface.load（L44-L46）：输入目录字符串，输出 Effect.Effect<Snapshot, ACPError.Error>；没有显式环境参数，环境由 Layer 提供。
- Interface.get/refresh/variants（L48-L52）：get(directory) 读取缓存或首次加载；refresh(directory) 强制重新加载并替换缓存；variants(snapshot, model) 是纯查询，找不到键返回 undefined，不产生 Effect 失败。
- Loader Context.Service（L54）：服务标签为 @opencode/ACPDirectoryLoader，承载 LoaderInterface。
- Service Context.Service（L56）：服务标签为 @opencode/ACPDirectory，承载 Interface。
- modelKey（L58）：纯函数，将 DefaultModel 拼成 providerID/modelID。ID 内若自行含斜杠会产生歧义，源内没有额外转义或校验。
- variants（L60）：用 modelKey 索引 snapshot.variantsByModel；缺失模型、无变体模型和键不匹配均统一得到 undefined。
- build（L62-L105）：输入目录、provider 映射、mode 数组、默认模式 ID、命令数组和可选默认模型，返回同步的 Snapshot。
  - 模型选项（L70-L85）：遍历所有 provider 的 models，先生成带临时 id 的中间对象，再交给 Provider.sort，最后只保留四个 ModelOption 字段。排序完全委托给 Provider.sort；空 provider/model 集合得到空数组。
  - 变体索引（L91-L97）：只收集 truthy 的 model.variants，键是 provider/model 组合；无变体的模型不占索引。重复键理论上来自重复 provider/model ID，后写入值会覆盖先前值，源内没有冲突报错。
  - 默认模式（L98-L101）：若输入默认 ID 在 modes 中则保留；否则采用 modes[0]?.id，空数组时退回原输入 ID，因此可能得到一个不在可用模式中的字符串。
  - 其他字段（L87-L104）：原样保留 directory/providers/modes/commands 的对象引用或数组引用；defaultModel 只有传入时才通过展开属性写入。函数没有深拷贝、路径规范化或输入验证。
- loaderLayer（L107-L142）：构造 Loader 的 Effect Layer。
  - 初始化依赖（L109-L114）：从 Context 取得 InstanceStore.Service、Provider.Service、Agent.Service、Command.Service；缺失依赖时 Layer 构造失败。
  - load（L115-L139）：Effect.fn("ACPDirectoryLoader.load") 接收目录。先执行 store.load({ directory }) 得到实例上下文 ctx（L116-L118），再在内层 Effect 中并行执行 provider.list()、agent.list()、agent.defaultInfo()、command.list()、provider.defaultModel().pipe(Effect.option)（L119-L123）。并发选项为 { concurrency: "unbounded" }，这些查询互不依赖；任一失败会使整体失败，defaultModel 没有值则转成 None 而不是错误。
  - modes 映射（L127-L133）：过滤 item.mode !== "subagent" 且 item.hidden !== true；保留 agent 名称为 id/name，只在有描述时设置 description。
  - 默认值与命令（L134-L137）：默认模式取 defaultAgent.name；命令用 toSorted((a,b)=>a.name.localeCompare(b.name))，不改变原始 commands 数组；仅 Some 默认模型写入快照。最终通过 Effect.provideService(InstanceRef, ctx)（L138-L139）把本次目录实例上下文限定给内层查询，避免不同目录共享错误上下文。
- 内部 layer（L144-L202）：构造 Service，持有 SynchronizedRef<Map<string, Effect<Snapshot, ACPError.Error>>>（L147-L149）。Map 的值是已由 Effect.cached 包装的 Effect，而不是已求值 Snapshot。
  - cached（L150-L170）：以目录为键执行原子 SynchronizedRef.modifyEffect。已有值立即返回同一 cached Effect（L154-L156），所以并发 get 共享一次求值；没有值时创建 Effect.cached(loader.load(directory))，并用 Effect.tapError 在加载失败时通过新的 Map 删除该目录（L156-L167），成功后写回。缓存粒度是精确字符串，目录别名不会自动合并。
  - get（L172-L174）：先取得 cached Effect，再二次 yield* 求值；命名 Effect 为 ACPDirectory.get。成功返回快照，失败透传 ACPError.Error。取消由调用方 Effect 作用域传播；没有超时。
  - refresh（L176-L194）：无论是否已有缓存，都在同步修改中创建新的 Effect.cached(loader.load(directory)) 并替换 Map。错误 tap 会删除该键；末尾 Effect.flatten 立即执行新 cached Effect。刷新期间由 SynchronizedRef 串行化 Map 修改，但旧 cached Effect 若已被其他调用持有仍可完成；源未提供版本比较或“最后刷新者”判定。
  - 服务返回（L196-L201）：暴露 get、refresh 和纯函数 variants。
- loaderNode（L204-L208）：LayerNode.make 注册 Loader，依赖 Provider.node、Agent.node、Command.node、InstanceStore.node，把服务依赖图显式化。
- node（L210）：注册 Service，唯一直接依赖 loaderNode；因此消费 ACP 目录服务会间接拉起所有 provider/agent/command/instance-store 节点。
- export * as Directory from "./directory"（L212）：以命名空间 Directory 重导出本文件全部导出符号，形成稳定模块入口；不会复制或修改缓存状态。

## 状态、取消、恢复与副作用

状态只有进程内按目录的 Map，并由 SynchronizedRef 保护；Map 值的 Effect.cached 同时承担惰性求值和同一加载任务的共享。成功快照不会持久化，重启后全部丢失；InstanceStore.load 可能读取项目配置或创建/恢复实例上下文，这是本文件唯一明显的外部读取边界。provider、agent、command 查询本身的外部副作用由各服务决定，本文件不写文件、不发网络请求、不启动线程。

调用方取消 get/refresh/load 时，Effect 取消信号向下传播；源未定义显式取消钩子、超时、重试或退避。加载失败通过 tapError 删除对应缓存项，下一次 get 会重新尝试，属于“失败后可重试”而非持久化错误缓存。成功的 Effect.cached 会保持成功值，直到 refresh 替换；refresh 自身失败会使键被删除。没有崩溃恢复日志、快照 TTL、跨进程锁或持久化一致性协议。并发语义是 Map 修改串行、不同目录可在外层并发、同一目录首次加载共享；未提供容量上限或淘汰策略，目录字符串无限增长会使进程内缓存增长。

## 源内测试与行为判据

同目录测试文件 opencode/packages/opencode/test/acp/directory.test.ts 共 188 行，直接验证本模块行为：

- 测试夹具 command/model/snapshot（L11-L84）构造 provider、两个 model（一个带 low/high variants、一个无 variants）、两个 mode、两个 command 和 defaultModel；fakeLayer（L86-L101）以 Layer.succeed(Directory.Loader) 替换真实 loader，并记录 load 调用目录。
- “two concurrent callers share one load”（L103-L115）：对同一 alpha 并发两次 get，断言 calls 只有 ["alpha"] 且两个结果对象严格相同，证明 SynchronizedRef + Effect.cached 的单次加载和共享值语义。
- “warm calls use cached data”（L117-L127）：连续 get 同一目录只调用一次 loader，证明成功结果可复用。
- “different directories get different snapshots”（L129-L142）：并发读取 alpha/beta，断言两次 load 各自发生、directory 与 defaultModel.providerID 不混淆，覆盖缓存键隔离。
- “model variant lookup works”（L144-L156）：默认模型返回 low/high 变体；替换 modelID 为 missing 时 variants 返回 undefined，覆盖 modelKey 查找和缺失边界。
- “commands and modes are included”（L158-L170）：断言命令名有序、mode description 只在存在时保留、defaultModeID 为 build，覆盖 build 的投影结果。
- “falls back when the default mode is not available”（L172-L187）：输入 hidden 默认模式且 modes 非空时回退第一项 build，覆盖 L99-L101 的默认值分支。

源内测试未覆盖：loaderLayer 的 provider/agent/command 并发失败传播、InstanceRef 注入、加载失败后的缓存删除、refresh 替换、空 modes 时保留原 defaultModeID、隐藏或 subagent agent 过滤。因此可独立验证的补充判据是：模拟 loader 失败后再次 get 必须重新 load；refresh 必须得到新快照；空 modes 保留输入默认 ID；agent mode=subagent 或 hidden=true 不进入 availableModes；不同目录的 loader 查询必须收到对应 InstanceRef。

## zenpi Rust 映射

目标是把 ACP 的“目录快照 + 按作用域缓存”思想转成 DAG worker 的通信/生命周期服务。zenpi 已有可复用原语：src/protocol.rs 定义版本化 JSONL 与 MailboxRequest::{Send,Receive,Acknowledge,Claim,Complete}（约 L1-L110），src/headless.rs 有异步事件邮箱、Mailbox 分发和请求关联（如 L4014-L4170、L6392-L6405、L8943-L9000），src/session.rs 提供追加式 JSONL、事件恢复和 session tree（L145-L160、L733-L799、L894-L1089），src/runtime.rs 的 BackgroundRunner、CancellationToken、有界命令/事件通道和 RuntimeEvent::{Accepted,Started,Queued,CancelRequested,Completed,Closed}（L40-L237）可承载 worker 运行；src/core.rs 的 Turn.parent_id、worker binding/lease、register_live_owner/claim_live_mailbox/finish_live_mailbox（约 L130-L170、L821-L918）适合作为父子会话与租约入口；src/approval.rs 的 ApprovalCoordinator 可复用于跨 worker 的需要人工决策事件；src/tool_runtime.rs 的批处理与取消结果可用于 close 前副作用收敛；src/providers/** 的 ModelRegistry/ProviderDefinition/ProviderConnection 只负责 provider/model 能力，不应承担 DAG 拓扑。

建议落点与最小机制：

- 在 src/core.rs 增加 DagNodeId、DagRelation（Parent、Grandparent、DirectSibling、DirectChild）及 DagNodeState（Running、CloseRequested、Closed、KeepAlive）。每个 worker session 记录 session_id、parent_session_id、goal_id/item_id、租约与当前节点状态；复用现有 Turn.parent_id 表达对话父项，但 DAG 边应单独持久化，避免把“会话父子”误当作全部 DAG 关系。
- 在 src/protocol.rs 扩展现有 mailbox 协议而非新造传输：消息增加 sender_session_id、relation、correlation_id、epoch/lease_id、kind（work/status/cancel/close/heartbeat）和 bounded payload。发送方只能指定 receiver session；host 根据 session tree 计算 parent、grandparent、直接 sibling、直接 child，禁止任意祖先或非直接后代越权。沿用 ttl_ms、Claim、Complete 与 MailboxOutcome，实现幂等 message ID。
- 在 src/session.rs 增加可恢复的 DagNodeRecord、DagEdgeRecord、WorkerLeaseRecord、WorkerStatusEvent；用 append-only event 记录 spawned、heartbeat、green、close_requested、closed、keep_alive、derived。恢复时重建内存 adjacency 和 mailbox 未完成集合。节点关系至少持久化 parent、直接 children、兄弟计算所需 parent children 集合以及 grandparent 引用。
- 在 src/runtime.rs 复用 BackgroundRunner 为每个可运行 worker 提供有界队列；以 CancellationToken 处理 cancel/timeout，以 RuntimeEvent::Completed 作为工作终态。最小保活机制是：worker 在 lease 未过期时发送 heartbeat；收到 close 请求后 host 计算自身、全部直接 child 和全部 grandchild 是否均为 green，条件不满足则转为 KeepAlive，不关闭 runner，并发出 derive 命令。派生新 worker 必须生成新 session_id/lease_id、写入 parent edge、继承受限 blueprint/policy digest，并通过 BackgroundRunner::submit 入队；不得复用已关闭 worker 的结果通道。
- 在 src/headless.rs 接入协议和生命周期：已有异步事件 mailbox 负责把 status/heartbeat/close/derive 事件路由到 owner；用现有请求 ID、重放/确认机制保证 reconnect 后不重复派生。close 判定应在 host 的单一 owner 线程或受锁状态机中完成，避免 sibling 同时关闭造成竞态；所有派生和关闭结果都输出相关联的 terminal/event 行。
- 在 src/core.rs 提供 DAGCoordinator（或等价 service）函数：send_relation_message、request_close、evaluate_green_closure、keep_alive_and_derive、heartbeat。evaluate_green_closure 必须检查 node 自身、所有直接 child、所有 grandchild 的终态与证据，而不是只检查 worker 自报；对缺失、过期或 UnknownOutcome 一律不能判绿。
- 在 src/tool_runtime.rs 与 src/approval.rs 对接副作用：child/grandchild 的工具调用尚未得到 Succeeded 或已被 Cancelled/Denied 明确收敛时，节点不得 close；派生 worker 继承最小的 WorkerExecutionBinding 和 approval policy digest，重新执行前要求 lease/策略校验，避免通过 sibling 消息绕过审批。
- src/providers/** 仅映射 Snapshot.providers/modelOptions/variantsByModel 的 provider catalog；可新增只读 ProviderCatalogSnapshot，但不把 provider 列表缓存当作 DAG 状态。若模型变体影响派生 worker，应把所选 provider/model/variant 写入 worker binding 或 session event，以便恢复后复现。
- 可执行验证：构造 parent P、child C1/C2、grandchild G1 的 session tree；验证 P 能向 parent/grandparent/direct sibling/direct child 发消息，不能向非直接 cousin 发消息；C1 close 在 G1 未绿时只产生 keep_alive，随后派生 D；当 P、C1、C2、G1 全部有持久化 green 事件且 lease 未过期时才产生 closed；重启后从 session.rs 事件恢复同样结论；重复 message/derive request 只得到同一 terminal replay。

与源文件的关键差异：TypeScript Effect 的 Context/Layer/惰性缓存对应 Rust 的显式 trait + Arc<RwLock/Mutex> + 有界通道；Effect.cached 没有直接等价物，需以 per-directory/per-session OnceCell 或共享 future 实现；源的目录快照无持久化，而 DAG close/keep-alive/derive 必须持久化以支持崩溃恢复；源只缓存读取结果，不定义父子通信和绿状态，因此这些规则必须由 zenpi coordinator 明确实现。

## 未决问题

- ACPError.Error 的具体成员、可重试分类和错误是否含目录路径，无法从本文件确认。
- Provider.sort 的稳定排序键及重复 provider/model ID 的正式保证，需查看 provider 实现。
- InstanceStore.load 是否创建持久化文件、是否可取消，以及 InstanceRef 的生命周期边界，需查看其实现。
- Effect.cached 在底层加载失败时对并发等待者的精确取消/缓存行为依赖 Effect 版本；本文件只明确了错误 tap 删除 Map 项，未定义跨进程语义。
- zenpi 当前 DAG “全部 child/grandchild 全绿才 close”的最终状态字段与 green 证据格式不在本源文件中，需要在上述 coordinator/session 设计中落地。


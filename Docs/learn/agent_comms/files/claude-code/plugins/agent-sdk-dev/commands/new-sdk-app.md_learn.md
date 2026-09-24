# AC-006 — claude-code/plugins/agent-sdk-dev/commands/new-sdk-app.md

- source_id/item_id：`AC-006`
- source_path：`claude-code/plugins/agent-sdk-dev/commands/new-sdk-app.md`
- source_hash：`d273539c037e63bec2be30ac0e3474371a8725d7b0c39cb9e4341534fde83fb6`
- source_bytes：`7846`
- source_lines：`176`
- coverage：已按文件顺序读取完整 `L1-L176`，字节范围 `0-7846`（含 YAML 前置区、正文、注释、命令与 URL）。

## 完整行为复盘

该源文件是 Claude Agent SDK 新项目的交互式命令说明，不包含 Rust 函数、类型或导出符号；因此“逐函数/逐导出符号”对应为逐个指令块复盘如下。文件先在 `L1-L4` 声明 `description` 与 `argument-hint: [project-name]`，命令目标是创建并配置新 SDK 应用。

1. **参考资料与版本前置（L6-L25）**：执行前必须用 `WebFetch` 读取 overview（L10-L12）、按语言读取 TypeScript/Python SDK 参考（L13-L15），再按需求读取 Streaming/Single、Permissions、Custom Tools、MCP、Subagents、Sessions 等指南（L16-L23）。安装前必须用 `WebSearch`/`WebFetch` 或 npm/PyPI 核对最新版本（L25）；这意味着外部网络查询是安装决策的输入，不能凭缓存版本继续。
2. **逐个收集需求（L27-L58）**：问题必须“一次一个、等待回答”（L29-L31、L174-L176）。顺序固定：语言 TypeScript/Python（L33-L35）；项目名（L37-L40），若 `$ARGUMENTS` 已给则跳过；agent 类型 Coding/Business/Custom（L42-L47），若前一回答已充分描述可跳过；起点是 Hello World、基础 agent 或用例示例（L49-L54）；工具链选择必须告知将使用的工具并确认，且尊重 npm/yarn/pnpm/bun 等偏好（L56-L56）。全部回答后才进入计划（L58）。边界是：缺省参数不能代替语言问题；已有详细用例只影响 agent-type 问题，不改变顺序规则。
3. **Setup Plan（L60-L105）**：计划必须包含六类输出。初始化：创建目录，TypeScript 用 `npm init -y`，`package.json` 设置 `type: "module"` 与含 `typecheck` 的 scripts；Python 创建 `requirements.txt` 或运行 `poetry init`（L64-L70）；TypeScript 建 `tsconfig.json`，Python 配置按需（L71-L72）。版本核验：TS 查 npm 包页、Python 查 PyPI，并把将安装的版本告知用户（L74-L79）。安装：TS 用 `npm install @anthropic-ai/claude-agent-sdk@latest`（或明确最新版本），Python 用 `pip install claude-agent-sdk`，随后分别检查 `package.json`/`npm list` 或 `pip show`（L81-L87）。起始文件：TS 为 `index.ts`/`src/index.ts`，Python 为 `main.py`，都要有基本 query、正确 import 与错误处理，并遵循当前 SDK 语法（L89-L94）。环境：写 `.env.example`，其中 `ANTHROPIC_API_KEY=your_api_key_here`，把 `.env` 放入 `.gitignore`，解释从 `https://console.anthropic.com/` 获取密钥（L96-L100）。可选 `.claude/` 目录及示例 subagents/slash commands 必须先询问（L102-L105）。
4. **Implementation（L106-L126）**：仅在需求收集且用户确认计划后执行（L108）。顺序为重新检查最新版本、执行 setup、创建文件、安装稳定最新版、核验版本、按 agent 类型写可运行例子、加解释性注释（L110-L116）。完成门槛是验证成功：TS 必须运行 `npx tsc --noEmit`，修复全部类型错误，确认 imports/types 正确后才继续（L117-L122）；Python 至少检查 imports 与语法（L123-L125）；任何验证未通过都不能算完成（L126）。
5. **Verification（L128-L135）**：配置完成后必须调用语言对应 verifier：TS 为 `agent-sdk-verifier-ts`，Python 为 `agent-sdk-verifier-py`（L130-L133）；审阅报告并处理问题（L134-L135）。这是独立于本地编译/语法检查的二次行为判据。
6. **Getting Started Guide（L137-L159）**：完成且验证后给出 API key 设置与运行命令：TS `npm start` 或 `node --loader ts-node/esm index.ts`，Python `python main.py`（L139-L146）；给出两种 SDK 参考链接并解释 system prompts、permissions、tools、MCP servers（L148-L152）；继续指导 prompt 定制、MCP custom tools、permissions、subagents（L154-L158）。
7. **全局硬约束与终止条件（L160-L176）**：再次强调每次安装前查最新版（L162）、TS `npx tsc --noEmit`/Python imports+syntax 必须通过（L163-L166）、核验并告知已安装版本（L167）、检查 Node/Python 版本要求（L168）、创建前检查文件/目录是否存在（L169）、尊重用户包管理器（L170）、例子须可运行并有错误处理、使用现代兼容语法、过程应互动且有教育性（L171-L173）。最后只能提出**第一个**语言问题并等待，不能一次发送多个问题（L174-L176）。

输入是 `$ARGUMENTS`、用户逐次回答、语言选择、项目名、agent 类型、起点和工具偏好；输出是计划、文件/依赖、版本告知、验证报告及启动指南。默认值只有 `$ARGUMENTS` 缺省时询问项目名、以及未显式版本时安装“latest”；不存在静默选择包管理器的授权。错误路径包括版本查询失败、目录已存在冲突、安装失败、TS 类型错误、Python import/语法错误和 verifier 报告问题，均应停在相应步骤修复。

并发语义是串行的：问题逐一等待；计划确认后步骤按序执行。verifier 是在安装和本地检查之后的独立验证阶段，源文件没有并行执行、共享状态锁或多 worker 约定。

## 状态、取消、恢复与副作用

源文件没有显式状态机、取消 token、超时、重试策略或持久化协议。隐含状态为“待询问语言 → 项目名 → agent 类型（可跳过）→ 起点 → 工具确认 → 计划确认 → 初始化/安装/写文件 → 本地验证 → verifier → 指南”（L27-L58、L106-L139）。用户未回答时必须停住；用户确认计划前不得执行安装或写入（L108）。没有恢复游标：中断后只能根据已存在目录、`package.json`、`requirements.txt`、`tsconfig.json`、源码和安装记录重新检查（L64-L87、L169）。没有源级重试次数或退避；“修复 ALL type errors until types pass”是 TS 的循环门槛（L119-L122），版本核验失败则应重新查询。

外部副作用明确包括创建目录/配置/源码/`.env.example`/`.gitignore`（L64-L72、L89-L100）、npm/pip/poetry 安装（L81-L87）以及可能创建 `.claude/`、subagents、slash commands（L102-L105）。读取官方文档和 npm/PyPI 是外部网络副作用；API key 只写占位示例，不应写真实密钥（L98-L100）。源没有定义删除、回滚、并发写保护或凭据持久化。

## 源内测试与行为判据

源文件或同目录未包含测试。可独立验证的判据是：逐次交互顺序严格为语言、项目名（参数可跳过）、agent 类型（充分描述可跳过）、起点、工具链确认；计划确认前无安装；TS 项目存在 `type: "module"`、`typecheck` script 且 `npx tsc --noEmit` 零错误；Python 的 imports 与基本语法通过；安装后 `npm list @anthropic-ai/claude-agent-sdk` 或 `pip show claude-agent-sdk` 能报告版本；启动命令可运行；对应 verifier 报告无未处理问题（L29-L58、L68-L87、L117-L135、L145-L146）。

## zenpi Rust 映射

**总体边界。** 源命令的“逐步收集、计划确认、执行、验证”可映射为 zenpi 的有界会话/运行时状态；它本身不是 SDK provider。现有 `src/headless.rs` 的 `run_headless`/`run_async_streams`（约 L556-L806）负责 JSONL 帧、request-id、重放、owner 注册与关闭；`src/core.rs` 的 `Agent`（约 L403 起）持有同步 backend、session、工具与事件；`src/runtime.rs` 的 `BackgroundRunner`/`CancellationToken`（L237-L440）适合把一个 DAG 节点变成可取消 job；`src/providers/**` 仅负责 provider 协议/路由，不应承载 DAG 拓扑或父子邮箱。

**可复用通信原语。**
- 消息/邮箱：复用 `src/session.rs` 的 `MailboxMessage`、`SessionMailbox::enqueue/list/update/find_message`（约 L3181-L3260、L3574-L3953）和 `MailboxStatus`/`MailboxAction`。每条消息有 sender、recipient、request_id、digest、TTL、sequence、claim token、结果，适合节点工作请求、完成回执、错误回执。
- 事件：复用 `src/core.rs` 的 `AgentEvent` 与 `src/headless.rs` 的有界 async event buffer；用 `StdioEvent`/`RuntimeEvent::{Accepted,Started,Queued,CancelRequested,Completed,Closed}`（`src/runtime.rs` L174-L212）发布节点生命周期和验证进度，不把 provider 流当作拓扑真相。
- 协议：扩展 `src/protocol.rs` 的 `MailboxRequest`（L81-L119）以携带 `dag_id,node_id,parent_id,edge_kind,generation,required_children`，保留 `Send/Receive/Acknowledge/Claim/Complete`；复用 `TreeRequest/TreeAction`（约 L1164-L1239）展示/选择/fork 拓扑。所有新增字段应维持 `deny_unknown_fields`、ID/文本/TTL 上限。
- 地址关系：在 session 元数据或新 `DagNodeRef` 中保存 `parent_id`、`grandparent_id`、直接 sibling 集合和直接 child 集合；发送时只允许同一 workspace、已注册且 epoch 有效的 session，正好沿用 `SessionMailbox::check_access` 的访问校验（约 L3610-L3640）。grandparent/sibling/child 都是显式 recipient，不通过模糊广播。

**会话父子关系与最小 DAG 机制。** 建议在 `src/session.rs` 增加 `DagNodeState { dag_id, node_id, parent: Option<NodeRef>, children: BTreeSet<NodeRef>, status, generation, live_epoch }`，并让每个 worker session 的 `SessionStore` 记录不可变 parent/edge 元数据；`LiveSessionRegistry`（约 L3278-L3410）的 `register/heartbeat/active/unregister` 作为保活租约。最小可执行路径是：父节点 enqueue 子节点任务；子节点 `Claim` 后运行；子节点向 parent、grandparent、直接 sibling、直接 child 各自发送有界事件/结果；完成时 `Complete`/`Fail` 回执；父节点聚合自身结果和全部后代的终态快照。

close 判据必须是一个原子聚合检查，而非单个 worker 返回成功：`node.status == Succeeded` 且节点自身、所有直接 child、递归 grandchild 的状态均为 `Succeeded`，并且没有未确认/过期 mailbox、活动 lease、未完成 generation。任何失败、超时、过期、未知状态或新增工作都令节点保持 `Alive`；保活期间 heartbeat，依据新消息派生 worker。最小派生机制可落在 `src/runtime.rs`：父 owner 调 `BackgroundRunner::try_submit`（L302-L315），以 `JobId` 绑定 `DagNodeRef`，子 job 用 `CancellationToken`；队列满返回 `SubmitError::QueueFull`，不能假装 close。派生事件写 session journal 后才视为 admitted，避免重启丢工作。

**各既有模块落点。**
- `src/headless.rs`：在 `handle_command`/异步调度附近增加 `dag_send`, `dag_claim`, `dag_complete`, `dag_heartbeat`, `dag_status` 命令；继续使用有界帧、request fingerprint、terminal replay、`run_async_streams`。将 DAG 事件写入现有 reconnect/session journal，EOF 只停止 owner，不把已入队工作改成成功。
- `src/core.rs`：在 `Agent`/`AgentEvent` 附加 node context、祖先/邻接快照和 `DAG_NODE_ALIVE/CLOSE/DERIVED` 事件；把 provider turn、工具完成和后代聚合接到一个纯函数 `evaluate_close(node_snapshot) -> Alive|Close`，避免 provider 单次成功直接关闭。
- `src/session.rs`：复用 `SessionStore` 持久化节点状态、generation 和完成证据；复用 `SessionMailbox` 的 digest/sequence/TTL/claim transition；新增幂等的 `record_child_terminal` 与 `record_derivation`，在文件锁内完成状态转移。利用 `LiveSessionRegistry` 的 owner epoch 防止旧进程完成新一代任务。
- `src/tool_runtime.rs`：将节点派生/通信视为受治理的 tool batch 或独立本地动作；沿用 `MAX_BATCH_CALLS=32`、`MAX_BATCH_BYTES=256 KiB` 和 cooperative cancellation（约 L21-L32、L168-L220），不要让一个节点无限并发派生。跨节点 side effect 仍需 `ApprovalCoordinator`。
- `src/runtime.rs`：`BackgroundRunner` 负责节点执行、FIFO pending、取消和 bounded shutdown；`JobOutcome::Succeeded/Failed/Cancelled/Panicked` 映射到节点终态，`mark_completed` 处理 late cancel。将“Alive 并派生”实现为完成事件后的父调度循环，而不是 detached thread。
- `src/protocol.rs`：在 `MailboxRequest`/`StdioRequest` 增加 DAG envelope 与节点状态查询；复用 `MAX_MAILBOX_TEXT_BYTES`、`MAX_MAILBOX_TTL_MS`、`MAX_ID_BYTES`。协议错误要返回相关 request id，不应杀死 headless 进程。
- `src/approval.rs`：节点创建 child、发送外部 provider 请求和 mutating tool 都经过 `ApprovalRequest`；`ApprovalMode::WorkerAllowAfterPreflight`、policy digest、lease id（约 L22-L56、L490-L527）可绑定 parent 授权，取消时 `cancel_all` 只产生拒绝并保留可审计回执。
- `src/providers/**`：保持 Anthropic/OpenAI/Google/Codex/DeepSeek 的 wire route 与 capability；只提供节点所需模型调用，不保存 parent/child/sibling 关系。provider deadline/错误回到 `AgentEvent`，由 core/session 聚合后决定 Alive 或 Close。

**差异清单与验证。** 源命令要求 TypeScript/Python SDK 安装、版本查询、文件生成和 verifier；zenpi 目前是 Rust session/JSONL/runtime，缺少“语言选择/项目模板”状态、DAG 邻接元数据、递归后代 close 聚合和标准化派生 receipt。可执行改造顺序：先加 `DagNodeState` 与持久化 transition；再把 DAG mailbox 字段加入 protocol 校验；接入 runtime submit/heartbeat；最后在 headless 端实现命令和重放。验证包括：同一 request_id 重发得到相同 terminal；父/祖父/直接 sibling/child 均可收发且越 workspace 被拒；任一 descendant 未绿时 `evaluate_close` 返回 Alive；全部递归后代绿且无 lease/message backlog 才 Close；Close 竞态下的新消息使 generation 增加并保活；取消、崩溃、重连后 mailbox claim 不会隐式重试，旧 owner epoch 不能完成新代任务。

## 未决问题

源文件未定义 verifier agent 的具体接口、错误报告格式、项目模板内容、SDK query API 细节、真实版本解析规则，也未说明安装失败后的回滚/重试。zenpi 映射中仍需由项目决定 DAG 状态是否写入现有 session journal 还是新表、grandchild 聚合的最大深度/节点数、sibling 通信是否允许双向、以及“全绿”是否包含 provider/tool approval 事件；这些不能从源文件确认。

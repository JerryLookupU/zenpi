# OC-053 — packages/opencode/src/tool/skill.ts

## 元信息

- source_id/item_id：OC-053
- source_path：packages/opencode/src/tool/skill.ts
- source_hash：629ada0c0a0135cb003429aec233aeb165f0b98a2bd5171986bc319c11af3c07
- source_bytes：2215
- source_lines：70
- coverage：已从字节 0 至 2215（含文件末尾）顺序读取；覆盖源文件 L1-L70，包含全部 import、注释、Schema、导出符号和实现。

## 完整行为复盘

1. 模块导入（L1-L6）：使用 Node path 做目录和绝对路径处理；从 effect 引入 Effect、Schema；注入 @opencode-ai/core/ripgrep 的 Ripgrep 服务；引入 ../skill 的 Skill 服务、./tool 的工具定义器，以及 ./skill.txt 文本资源作为 DESCRIPTION。文件本身不实现技能发现或文件读取，只编排这些服务。

2. 导出 Parameters（L8-L10）：Schema.Struct 定义唯一参数 name: Schema.String，并以 description 注释其应来自 available_skills。Schema 未声明长度、字符集或默认值；缺失、非字符串等输入由工具框架解码拒绝，空字符串等更细边界交由 Skill.require 或下游服务决定。

3. 导出 SkillTool 与构造阶段（L12-L20）：Tool.define("skill", Effect.gen(...)) 注册名为 skill 的工具。构造 Effect 先取得 Skill.Service（L15）和 Ripgrep.Service（L16），再返回 description、parameters 与 execute（L18-L21）。服务不可用时构造阶段失败；DESCRIPTION 的实际文本不在本文件内。

4. execute 输入与技能解析（L21-L26）：params 类型为 Schema.Schema.Type<typeof Parameters>，ctx 类型为 Tool.Context。先调用 skill.require(params.name)（L23-L24）得到 info。仅对 Skill.NotFoundError 做 Effect.catchTag，并以 new Error(error.message) 调用 Effect.die（L24-L25），因此未知技能是 defect 而非正常业务返回。其它错误标签没有本地转换。成功路径依赖 info.location、info.name、info.content。

5. 审批/权限（L27-L32）：解析成功后调用 ctx.ask。permission 固定为 skill，patterns 和 always 都只含 params.name，metadata 为 {}。审批发生在目录扫描前；拒绝或审批失败不会调用 ripgrep.find。单次 execute 没有显式 fork，并按 require → ask 顺序等待。

6. 目录与采样文件（L34-L43）：dir = path.dirname(info.location)，base 直接等于 dir（L34-L35）。ripgrep.find 参数为 cwd: dir、pattern: !**/SKILL.md、hidden: true、follow: false、signal: ctx.abort、limit: 10（L36-L43）。这表示隐藏项可见、不跟随符号链接、排除技能主文件且最多十项；未定义排序、超时和递归深度，均由 Ripgrep.Service 决定。目录不存在、权限错误、扫描 abort 等无本地恢复分支。

7. 返回格式（L45-L66）：返回 title 为 Loaded skill: info.name（L46）。output 是按换行拼接的字符串（L47-L61），包含 skill_content 包裹、Skill 标题、info.content.trim()、Base directory for this skill: base、相对路径说明、Note: file list is sampled.、skill_files 列表及闭合标签。每个文件通过 path.resolve(dir, file.path) 生成绝对路径并包为 file 元素（L57-L59）；列表最多十项且可以为空。metadata 只有 name 和 dir（L62-L65），不返回 location、审批结果或扫描诊断。

8. 错误收束和并发边界（L67-L70）：execute 的生成式 Effect 通过 Effect.orDie 收束（L67），所以 ctx.ask、ripgrep.find 等失败会提升为 defect，调用方不能依赖显式 Either/失败值区分原因。没有重试、批处理或并发控制；不同工具调用是否并发由上层运行时决定，本次执行内部步骤有序。

## 状态、取消、恢复与副作用

- 取消：明确的取消通道是 ctx.abort 传给 ripgrep.find（L41）。审批是否响应取消取决于 ctx.ask 实现；源文件没有额外取消回调。扫描取消后的错误会经过 Effect.orDie 变成 defect。
- 超时：无 timeout、deadline 或 retry 配置；上层若把超时映射到 ctx.abort，只能间接影响扫描。
- 恢复/重试：没有持久化游标、幂等键或恢复逻辑。重新调用会重新 require、重新审批并重新采样目录。
- 持久化：不写文件、会话、缓存或状态；metadata 只是瞬时返回值。
- 外部副作用：ctx.ask 触发权限交互；ripgrep.find 只读扫描文件系统。没有写入、命令执行或符号链接跟随。path.resolve 只解析路径字符串，不在此处校验路径仍位于技能根目录。
- 并发：文件没有启动并发任务；Effect 运行时可能并发调度不同工具调用，但单次 execute 是顺序依赖。

## 源内测试与行为判据

源文件及同目录 src/tool 文件列表中未包含针对 skill.ts 的测试。可独立验证：给定有效技能名，必须先产生一次 permission 为 skill 的请求，再以技能所在目录为 cwd 调用一次 Ripgrep.find，且 hidden=true、follow=false、limit=10、pattern 为 !**/SKILL.md、signal 绑定 ctx.abort；成功输出含 info.content.trim()、基目录说明和至多十个绝对 file 元素；未知技能走 Skill.NotFoundError 到 Effect.die(new Error(message))；审批拒绝或扫描错误不得返回正常成功对象。

## zenpi Rust 映射

- 技能模型/加载：zenpi 的 src/skills.rs 已有 SkillSet、SkillMetadata、SkillBody、SkillResource、ModelSkillTools。建议把 Parameters 映射为 SkillToolArgs { name: String }，把 Skill.require 映射为 SkillSet::load_body(name, SkillInvocation::Model, "", cancelled)，保留名称校验、正文读取和 SKILL.md 排除。
- 工具注册/执行：在 src/tools.rs 或 src/skills.rs 的 ModelSkillTool 旁增加只读 skill 定义，或将现有 load_skill 作为兼容别名；执行落点可放在 src/tool_runtime.rs 的 dispatch/execute_cancellable 路径，保证先审批后扫描。tool_runtime.rs 已提供批量决策和取消回调，可把 ctx.abort 映射为 cancelled()。
- 核心策略：src/core.rs 的 prepare_tool 已检查 skill allowlist、ToolContext::check_call_gate 和取消；应保持未知技能为拒绝/失败，并在 persist_tool_invocation 记录结果。TS 的 Effect.orDie 是 defect 语义，Rust 可映射为 ToolError::InvalidCall 或 ToolError::Cancelled，但协议层应区分取消、拒绝和内部错误。
- 审批与副作用：src/approval.rs 的 ApprovalCoordinator/ApprovalPolicy 对应 ctx.ask；ApprovalRequest 的 tool 应为 skill，arguments 仅含 name，再把拒绝作为终止结果。工具是只读扫描，仍须经过 zenpi 的审批/Blueprint gate。
- 会话/恢复：src/session.rs 是追加式 JSONL 持久化；源工具没有持久化，因此不应新增恢复记录。若统一工具审计，可沿用 tool_execution_finished，标注只读成功或取消，不伪造可恢复操作。
- 运行时/协议/无头：src/runtime.rs 的 BackgroundRunner 可提供有界队列和终止事件；src/protocol.rs 的 Command/工具结果负责传输结构化输出；src/headless.rs 负责 JSONL、重放和关闭，不应把技能正文无界写入缓存。建议结果保持 title、output、metadata.name、metadata.dir，并限制正文及列表大小。
- providers/**：src/providers/** 仅负责模型连接和路由，不应实现技能扫描。模型产生工具调用后仍由 core.rs → tool_runtime.rs 执行；验证点是 provider 只看到工具 schema 和结果，不直接获得文件系统权限。
- 可执行差异清单：1) 增加 skill 工具 schema 与 name 描述；2) 复用 SkillSet 读取正文并计算 dir；3) 接入 ApprovalCoordinator 后再调用受取消控制的 ripgrep/搜索实现；4) 排除 SKILL.md、允许隐藏文件、禁止跟随链接、限制十项并输出绝对路径；5) 为未知技能、审批拒绝、取消、扫描错误补充单元/集成判据；6) 明确 Rust 错误映射，避免把失败静默成成功。

## 未决问题

- Skill.Service.require 返回的 info.location 是否始终为文件路径、是否已做路径安全校验，源文件无法确认。
- Ripgrep.Service.find 的排序、glob 语义、错误类型以及 signal 取消时的具体错误标签未在本文件定义。
- ctx.ask 是否阻塞、如何表示拒绝/超时、always 决定是否持久化未在本文件定义。
- DESCRIPTION（./skill.txt）的实际文本和 Tool.define 的生命周期/错误包装细节需查看对应实现才能确认。

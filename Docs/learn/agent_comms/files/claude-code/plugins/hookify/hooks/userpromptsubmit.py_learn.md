# AC-014 — claude-code/plugins/hookify/hooks/userpromptsubmit.py

## 元信息块

- source_id/item_id：AC-014
- source_path：claude-code/plugins/hookify/hooks/userpromptsubmit.py
- source_hash：82c8784355826e9ecfa01c5b357c27d1246c21f0d26400d32ea962c7db4e3d01
- source_bytes：1543
- source_lines：58
- coverage：已从第 1 字节至第 1543 字节（字节区间 0–1542）顺序读取，覆盖行 L1-L58，包括注释、导入、顶层初始化、函数与入口分支。

## 完整行为复盘

1. 模块文档字符串位于 L2-L6：脚本是 hookify 插件的 UserPromptSubmit 执行器，由 Claude Code 在用户提交 prompt 时调用；它读取 .claude/hookify.*.local.md 规则并求值。该说明不执行 I/O，但定义了输入事件是 prompt 提交。
2. import os、import sys、import json 位于 L8-L10。三者分别提供环境变量/路径处理、标准输入输出与进程退出、JSON 解码编码；没有线程、异步任务、锁或重试库。
3. 顶层变量 PLUGIN_ROOT = os.environ.get('CLAUDE_PLUGIN_ROOT') 在 L12-L13 读取插件根目录。若变量为真（L14），parent_dir = os.path.dirname(PLUGIN_ROOT)（L15），随后仅当路径不在 sys.path 时，以 sys.path.insert(0, ...) 把父目录和插件根目录分别置于搜索路径首位（L16-L19）。边界是环境变量缺失、空字符串或路径重复：缺失/空值不改路径；重复值不重复插入；非空但不存在的目录仍会插入，错误延迟到导入阶段。该顶层修改是进程级全局副作用。
4. try 导入 hookify.core.config_loader.load_rules 与 hookify.core.rule_engine.RuleEngine（L21-L23）。只捕获 ImportError（L24）；发生导入错误时构造 {"systemMessage": f"Hookify import error: {e}"}（L25），用 json.dumps 写到 stdout（L26），然后 sys.exit(0)（L27）。因此导入失败仍输出机器可读 JSON，并以成功退出码结束；非 ImportError 的顶层异常不在这里处理。
5. def main() 是唯一函数，定义和入口说明在 L30-L31。其主体用一个 try/except/finally 包住整个 hook 流程：
   - json.load(sys.stdin)（L33-L34）一次性读取 stdin，并把完整 JSON 值放入 input_data。空输入、截断 JSON、非合法 JSON 或读取 I/O 异常都会抛出；代码没有大小上限、流式解析或 schema 校验，输入类型也不在本文件限制。
   - load_rules(event='prompt')（L36-L37）按固定事件名 prompt 加载用户规则。规则文件定位、解析、排序和过滤全部委托给导入模块；本文件不提供默认规则，也不把用户输入写回文件。
   - engine = RuleEngine()（L39-L40）每次调用新建求值器；engine.evaluate_rules(rules, input_data)（L41）返回 result。规则冲突、规则格式错误以及求值器内部异常均向外传播到统一异常路径；本函数不声明返回值。
   - 即使 result 是空对象/空数组，也执行 print(json.dumps(result), file=sys.stdout)（L43-L44），保证一次调用至少产生一行 JSON。没有 stderr 日志、没有显式 flush、没有输出协议版本或请求 ID。
   - 任意异常由 except Exception as e（L46）捕获，输出 {"systemMessage": f"Hookify error: {str(e)}"}（L47-L50）。捕获范围包括 JSON、规则加载、规则求值和 stdout 写入时的大多数普通异常；不会捕获继承体系外的进程级终止信号或显式 SystemExit。
   - finally 总会执行（L52-L54），再次调用 sys.exit(0)。成功、业务异常和普通 I/O 异常最终都被强制映射为退出码 0；若 stdout 已关闭，异常处理自身的 print 可能再次失败，但 finally 仍尝试退出。
6. if __name__ == '__main__': main()（L57-L58）只在脚本直接执行时调用；被 import 时不会自动读取 stdin。没有其他公开函数、类或命令行参数。并发语义是单进程、单次同步调用：本文件不创建线程/任务，不保护共享规则文件，也不保证多个进程同时运行时的顺序；并发控制若存在只能由 Claude Code、规则模块或外部宿主提供。

## 状态、取消、恢复与副作用

本源没有显式状态机、取消 token、超时、重试、检查点、持久化写入或恢复逻辑。可观察副作用只有：顶层修改当前进程 sys.path（L12-L19）；读取 stdin（L33-L34）；间接读取规则文件（L36-L37）；向 stdout 输出一行 JSON（L25-L26、L43-L44、L47-L50）；以及以 0 退出（L27、L52-L54）。取消只能由宿主终止进程，源代码不会主动轮询取消；规则或求值器内部若自行产生超时/副作用，本文件既不记录也不回滚。失败不重试，恢复需由下一次独立进程重新读取规则和输入。由于异常被转换成 systemMessage 且退出码仍为 0，调用方必须检查 stdout JSON 才能判断业务失败，不能只依赖进程码。

## 源内测试与行为判据

源文件及其同目录未包含测试（源内未包含测试）。可独立验证的判据如下：设置合法 CLAUDE_PLUGIN_ROOT 并提供最小可导入的 hookify.core 后，stdin 输入合法 JSON，应恰好得到一行 json.dumps(result)；输入空串或非法 JSON，应得到含 systemMessage 且前缀为 Hookify error: 的 JSON，退出码仍为 0；删除/破坏导入模块，应得到含 Hookify import error: 的 JSON，退出码仍为 0；以模块方式 import 时不应消费 stdin。还应核对重复路径不会重复插入 sys.path，以及规则求值异常不会穿透 main。

## zenpi Rust 映射

- 通信原语与协议：最接近的可复用原语在 src/protocol.rs。现有 MailboxRequest::{Send,Receive,Acknowledge,Claim,Complete}、MailboxOutcome（约 L81-L112）已覆盖消息投递、领取、确认和完成；StdioRequest/Command 与 JSONL 编解码适合作为 worker 控制面。将 DAG 消息定义为带 message_id、sender_session_id、recipient_session_id、relation（parent/grandparent/sibling/child）、ttl_ms、correlation_id 和载荷的严格 serde 类型，复用现有 ID/文本/TTL 上限与 validate_mailbox，拒绝未知字段。事件流可复用 StdioEvent/headless 的有界 event mailbox；协议层只解析，不授予发送者权限，身份和工作区授权应由 owner 校验。
- 会话父子关系：src/core.rs 的 Turn { parent_id: Option<String> }、Turn::with_parent（约 L141-L177）可作为消息上下文和派生 worker 的最小 lineage；节点级实现建议新增 DagNode { node_id, session_id, parent_node_id, status, lease_id, children: BTreeSet<NodeId> } 与 DagRelation。直接 sibling 由同一 parent_node_id 的 children 求得，grandparent 由 parent 的 parent 求得，直接 child 是 children，grandchild 是 child.children；不要把关系仅编码在 prompt 文本中。会话持久化落在 src/session.rs 的 append-only JSONL 事件，新增 dag_node_created、dag_status_changed、dag_message、worker_derived、dag_close_decision 记录即可，崩溃后按序重放重建索引。
- 节点关闭判据：在 src/core.rs 增加纯函数 can_close_node(node, snapshot)：只有节点自身状态为 Green，且其全部直接 child 递归为 Green（从而全部 grandchild/后代也为 Green）才返回 true；空 child 集合在自身 Green 时成立。任一后代为 Running、Failed、Cancelled、Unknown 或缺失记录都禁止 close。把判据结果写入 session 事件并让 headless 返回可核对的 reason，避免只看当前节点。
- 保活与派生的最小机制：src/runtime.rs 的 BackgroundRunner、有界队列、RuntimeEvent::{Accepted,Started,Queued,CancelRequested,Completed,Closed} 和 CancellationToken（约 L51-L99、L157-L193）可承载每个 DAG worker 的执行、心跳检查和合作取消。新增最小 DagSupervisor（可放 core.rs 或新模块并由 headless 调用）：(1) 为每个非 Green 节点保存带过期时间的 lease；(2) 在每个 poll_interval 或收到 child 状态/消息时发 keepalive 事件，刷新 lease 并重新评估后代；(3) 若当前节点仍有新工作且不能关闭，创建唯一 child_node_id/lease_id，以 Turn::with_parent 建立父链，通过 BackgroundRunner::submit 派生 worker；(4) 对同一 correlation_id 做幂等去重，避免重连重复派生；(5) lease 超时只标记 Unknown/需要恢复，不假装 Green。保活是有界、可取消的，不能依赖无限 FIFO。
- headless 与外部通信落点：src/headless.rs 已独占 stdin/stdout JSONL，具备 request ID 重放、事件缓存和有界 mailbox；在命令分发处增加 DAG message/status/keepalive/derive/close 命令，把终端响应和异步 StdioEvent 分开。利用其 reconnect journal 将发送、领取、完成和派生事件持久化；断线后按 sequence 重放，遇到未知结果时要求幂等确认。直接 sibling/parent/grandparent 的寻址应由 supervisor 根据会话图解析，客户端只提交目标 session ID。
- 会话与恢复落点：src/session.rs 负责追加记录、损坏尾行恢复和操作结果（OperationOutcome::{Succeeded,Failed,Cancelled,Interrupted,UnknownOutcome}）；将 DAG lease、状态和消息完成结果作为独立记录，先写 dispatch/claim 再执行副作用，恢复时缺少 terminal 记录一律 Unknown。父子 session 的 fork/close 仍由 session owner 串行化；close 记录必须包含当时的后代快照哈希或 sequence，防止并发 child 更新后误关。
- 核心代理与审批：src/core.rs 的 AgentEvent、submit_with_cancel、run_active_turn_cancelable_with_events 和 worker binding/蓝图取消路径可把 DAG 消息转成 agent 输入，并沿 parent_id 传递上下文。若派生 worker 会调用工具，复用 src/approval.rs 的 ApprovalCoordinator、ApprovalRequest、ApprovalDecision；每个 worker 的 lease_id/policy_digest 必须校验，取消时清理 pending approval，不能因通信消息自动获得工具批准。
- 工具与 provider 边界：src/tool_runtime.rs 适合承载 worker 工具批处理、输出捕获和 side-effect policy；它不应自行决定 DAG close。各 src/providers/** 仅实现模型连接/流式 provider 事件，适配器应把 provider 完成、错误、超时映射为 DAG 节点状态事件，检查 CancellationToken，不在 provider 内保存拓扑或直接派生 sibling。
- 与现有实现的差异清单（可执行、可验证）：当前协议已有 mailbox/tree 基础，但尚无显式 DAG 节点状态、祖先/兄弟寻址、递归全后代 Green 判定、lease 心跳和幂等派生；需补类型、serde 校验、session 事件重放索引、supervisor 调度和 headless 命令。验证应包括：父→child、grandparent↔child、直接 sibling 四种路由的 round-trip；任一 grandchild 非 Green 时 can_close_node == false；全部后代 Green 后只生成一次 close；取消/lease 超时产生可恢复的 Unknown；重放相同 message_id/correlation_id 不重复副作用；有界队列满时返回 QueueFull 而不丢失 terminal 事件。

## 未决问题

无法从该 Python 源文件确认：load_rules 的文件遍历顺序、规则格式与冲突优先级；RuleEngine.evaluate_rules 的返回 schema、是否执行外部副作用；Claude Code 对 systemMessage 的具体消费语义；以及并发提交时宿主是否复用同一进程。以上均需读取 hookify.core.config_loader、hookify.core.rule_engine 或宿主协议才能确定。

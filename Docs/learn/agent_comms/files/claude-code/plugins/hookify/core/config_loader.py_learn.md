# AC-008 — claude-code/plugins/hookify/core/config_loader.py

- source_id/item_id：claude-code/plugins/hookify/core/config_loader.py / AC-008
- source_path：claude-code/plugins/hookify/core/config_loader.py
- source_hash：9f1d083cc43883323ddf00747e7696a4fd9d478ddd63f5b43038657da8516e56
- source_bytes：9690
- source_lines：297
- coverage：已按顺序读取字节 0 至 9689、行 1 至 297，包含注释、类型、导出符号和 __main__ 入口。

## 完整行为复盘

模块导入 os/sys/glob/re 及 typing 和 dataclasses（L7-L12）；re 未使用。没有 __all__，因此 Condition、Rule、extract_frontmatter、load_rules、load_rule_file 都可直接导入。

- Condition 数据类（L15-L20）保存 field、operator、pattern 三个字符串，注释给出 command、new_text、old_text、file_path 等例子，但没有枚举校验。Condition.from_dict（L22-L29）用 data.get 构造：field 缺省为空串，operator 缺省 regex_match，pattern 缺省为空串；不检查类型、不编译正则，非字典会在 get 处失败。
- Rule 数据类（L32-L42）包含 name、enabled、event、可空 pattern、conditions、action、tool_matcher、message；conditions 用 field(default_factory=list) 每实例独立，action 默认 warn，message 默认空串。
- Rule.from_dict（L44-L84）先处理新式 conditions（L47-L55）：仅当 frontmatter 的 conditions 是 list 才逐项调用 Condition.from_dict；列表项错误类型会抛异常。旧式 pattern（L57-L73）只在 pattern 为真且 conditions 为空时转成一个 regex_match 条件：event=bash 映射 field=command，event=file 映射 field=new_text，其他事件映射 field=content。返回对象保留原 pattern，name 默认 unnamed、enabled 默认 True、event 默认 all、action 默认 warn，tool_matcher 可空；message 用 strip 去首尾空白（L75-L84）。没有动作、事件或 tool_matcher 的合法性检查。
- extract_frontmatter(content)（L87-L195）返回字典和正文。内容不以精确的 --- 开头，或 split('---', 2) 后不足三段时，返回空字典和原文（L94-L100）；第三段 strip 后作为 message（L102-L103）。解析状态为 current_key/current_list/current_dict 与 in_list/in_dict_item（L105-L114）。逐行跳过空行和 # 注释（L115-L119），用空白数计算缩进（L121-L123）。
  - 顶层键必须 indent==0、含冒号且非 - 开头；切换键时提交前一个列表/字典（L124-L138）。无值的键开启列表状态（L140-L144）；有值的键去除一层引号，并仅把 true/false 转为布尔值，其余保持字符串（L145-L152）。没有完整 YAML 的数字、null、转义或多行标量语义。
  - 列表项仅在 in_list 时处理（L154-L181）。同时有冒号和逗号时按逗号拆内联字典；只有冒号时开启多行字典；无冒号时加入字符串列表。逗号会被无条件当分隔符。缩进大于 2 且处于多行字典时补充字段（L183-L187）；其他嵌套形态可能静默忽略。循环结束提交最后列表/字典（L189-L193），返回 frontmatter/message（L195）。本函数不捕获异常，非字符串输入可在 startswith/split 处失败。
- load_rules(event=None)（L198-L241）在当前工作目录寻找 .claude/hookify.*.local.md（L209-L211），glob 顺序不排序。逐文件调用 load_rule_file（L213-L216）；event 非空时只保留 rule.event 为 all 或精确等于 event 的规则（L219-L223），随后只保留 enabled 真值（L225-L227）。IO/权限、常见解析错误和其他异常分别打印 stderr 警告并继续（L228-L239），返回启用规则列表（L241）。无缓存、去重、排序、锁或跨文件优先级。
- load_rule_file(file_path)（L244-L275）以默认文本编码一次性读取（L251-L254），extract_frontmatter 后若为空则打印缺失 YAML 警告并返回 None（L256-L258）；否则 Rule.from_dict 并返回 Rule（L260-L261）。IO/OSError/PermissionError、ValueError/KeyError/AttributeError/TypeError、UnicodeDecodeError 以及其他异常分别打印错误并返回 None（L263-L275）。没有文件大小上限、原子读取或写操作。
- __main__ 演示（L277-L297）构造含 name/enabled/event/pattern 的 markdown，打印解析结果，再打印 Rule；没有断言、退出码或复杂列表和错误分支覆盖。

## 状态、取消、恢复与副作用

状态仅存在于一次调用的局部解析变量和返回的数据类；没有取消 token、超时、重试、检查点、恢复或持久化写入。副作用只有 glob、读取本地规则文件和向 stderr 打印警告/错误（L209-L239、L251-L254）。文件按顺序独立处理，失败文件被跳过；并发调用没有锁或一致性快照，文件变化时读取结果取决于时序。Rule.action 中的 block 只是数据值，加载器不执行动作，也不产生 provider、进程或网络副作用。

## 源内测试与行为判据

源内未包含测试；同目录搜索只找到本文件 __main__ 演示和其他 hook 对加载器的调用，没有断言型测试。可独立验证：

1. 运行 python3 plugins/hookify/core/config_loader.py，应打印 enabled=True 的 Rule；bash 的旧 pattern 应生成 field=command、operator=regex_match，message 去首尾空白。
2. 对非 --- 开头或缺少第二分隔符的文本，extract_frontmatter 应返回空字典和原文；合法三段文本的 message 应 strip。
3. 显式 conditions 优先于 pattern；conditions 为空且 pattern 为真时，bash/file/其他 event 分别推断 command/new_text/content。
4. 临时目录放入启用、disabled、不同 event 和损坏文件，load_rules(event='bash') 只返回启用且 event 为 bash/all 的规则，损坏文件只产生 stderr 警告。

## zenpi Rust 映射

源逻辑可抽象为“声明式输入→受限状态对象→按事件筛选→逐项容错”。zenpi 已有更强的 mailbox、session、runtime 和 approval 基础，DAG 映射应复用它们。

- **消息/邮箱/事件/协议**：复用 src/protocol.rs:L79-L113 的 MailboxOutcome 与 MailboxRequest::{Send,Receive,Acknowledge,Claim,Complete}；入口和字段校验在 L479-L484、L544-L590。持久通道用 src/session.rs:L3169-L3194 的 MailboxStatus/MailboxMessage 与 L3570-L3710 的 SessionMailbox；更新、过期拒绝和 claim token 在 L3743-L3795。短进度用 src/runtime.rs 的有界 command/event channel 和 RuntimeEvent；src/headless.rs:L833-L905、L940-L1041 已按请求限制事件数量/字节并记录丢弃。命中、子节点结果和 close 应为事件，跨进程可靠投递走 mailbox。
- **父子会话及邻居**：src/session_tree.rs:L48-L84 的 TreeEntry.parent_id/depth/branch_id 表达父链和分支；src/core.rs 中 Turn.parent_id 的校验在 L141-L196。新增 DagNodeId、DagRelation（Parent、Grandparent、DirectSibling、DirectChild、Grandchild）和 DagNodeState，把关系与目标 node/session id 放入 mailbox payload。发送端不能自报关系：core 服务层沿 parent_id 向上查祖先、按同一 parent 查 direct children，再查 children 的 children；限制最大深度并校验同一 workspace。现有 mailbox 只按 session_id 寻址，不能自动保证 direct sibling，因此关系授权必须在 core。
- **保活和最小派生**：用 src/session.rs:L3274-L3355 的 LiveSessionRegistry::{register,heartbeat,unregister,active} 做 owner_epoch+TTL 租约；过期即不可投递。派生只提交一个带 parent node、generation、idempotency key 的 DagWork 到 src/runtime.rs:L272-L300 BackgroundRunner::spawn 或 src/tool_runtime.rs:L274-L330 的有界批处理。SessionMailbox::claim_next 明确只 claim、never starts a worker（L3361-L3409），所以由 DagSupervisor 显式 spawn。完成用 finish_claim/finish_claim_with_reply（L3440-L3559），并回送 sender mailbox。
- **close 门槛**：在 src/core.rs 增加 DagNodeStatus::{Open,Waiting,Closed,Failed} 与 close_if_green(node_id)。节点自身成功、全部 direct child 和 grandchild 成功、无未完成 descendant mailbox、无活动 approval/tool 操作时，才追加 durable dag_node_closed。任一 child 失败、超时、未开始或有新消息就保持 Waiting，heartbeat 保持 owner 活跃并派生新 worker；派生不能重复执行旧 generation。祖父/兄弟只可报告或发送输入，不能绕过 close 门槛；恢复时 journal 没有 terminal close 就按未关闭。
- **模块落点**：
  - src/headless.rs：把 DAG admission、child progress、terminal close 放入每请求有界 AgentEvent，保留 admission/tool terminal 优先级，防止进度洪泛掩盖 close。
  - src/core.rs：实现 DagSupervisor、关系校验、close_if_green、状态聚合和 owner epoch；复用 Agent 的 register_live_owner/heartbeat_live_owner/claim_live_mailbox/finish_live_mailbox（L819-L884）。
  - src/session.rs：追加 dag_node_created/edge_added/node_green/node_closed/worker_spawned 事件，复用 mailbox digest、sequence、TTL、claim token 和 append-only recovery（L3180-L3255、L3743-L3795），增加按 node/relation 的只读投影。
  - src/tool_runtime.rs：节点的多工具执行沿 L157-L167、L274-L347 的有界并发、全批取消和源顺序结果归并；dispatch 前核对 generation/approval，避免派生 worker 重放副作用。
  - src/runtime.rs：每个 DAG worker 是 BackgroundRunner job；使用 CancellationToken、Completed/Cancelled/Panicked 和 bounded shutdown（L443-L673）。detach 不能算 green，因为副作用可能仍在运行。
  - src/protocol.rs：扩展 MailboxRequest 或新增 DagRequest，携带 node、relation、generation、reply target；沿 identifier/text/TTL/page 限制校验，拒绝客户端自报 sender/authorization，并区分 transient progress 与 durable close。
  - src/approval.rs：工具副作用继续经过 ApprovalCoordinator 的可取消等待和持久化确认，未持久化即 fail-closed；把 worker binding/policy digest 与 node generation 绑定，防止 sibling 或重启 worker 借用旧授权。
  - src/providers/**：provider 只负责请求与流式结果，不创建 DAG 边、不关闭节点；结果经 runtime/headless 有界缓冲回 core，由 core 聚合 durable descendant 状态决定 green。
- **差异与验证顺序**：Python 没有关系图、取消、TTL、恢复；Rust 必须显式持久化边、owner epoch、idempotency key 和失败事件。先为 parent、grandparent、direct sibling、direct child、grandchild 各写协议关系测试，再验证“自身+全部 child/grandchild green 才 close”。注入失败 child、过期 lease、重复 spawn、取消中的 tool，节点都应保持未关闭并产生可领取新任务；重开 SessionStore 后状态仍应未关闭。调度顺序应使用 durable sequence/priority，不能依赖 Python glob 的不稳定顺序。

## 未决问题

无法从源确认 action=block 的执行语义、tool_matcher 的匹配规则、conditions 的 AND/OR 组合、是否需要完整 YAML、以及规则文件是否必须排序。DAG 还需产品确认 grandparent/grandchild 是否可跨 session、兄弟通信是否必须双向 ACK，以及“全绿”是否包括已派生但尚未开始的 worker。

# AC-026 — codex/codex-rs/core/src/external_agent_config.rs

- source_id/item_id：`codex/codex-rs/core/src/external_agent_config.rs` / `AC-026`
- source_path：`/Users/wangweiyang/GitHub/codex/codex-rs/core/src/external_agent_config.rs`
- source_hash：`9776eed65b77a2aed9931c45ef11da8f59fb2f1051978b5f2bb1d7bedd4d784d`
- source_bytes：`22848`
- source_lines：`692`
- coverage：已按文件顺序读取字节 `1-22848`、行 `L1-L692`；同目录测试 `external_agent_config_tests.rs` 亦按顺序读取 `L1-L397`。

## 完整行为复盘

文件是同步的 Claude 外部配置迁移服务，不产生 worker、不启动线程。`serde_json::Value as JsonValue`、`toml::Value as TomlValue`、`HashSet`、`OsString`、`fs/io/Path/PathBuf` 支撑 JSON/TOML 和目录操作（`L1-L8`）。两个私有指标常量记录 detect/import，名称分别为 `codex.external_agent_config.detect` 与 `codex.external_agent_config.import`（`L10-L11`）。

- `ExternalAgentConfigDetectOptions`（`L13-L17`）是输入值：`include_home` 控制用户级扫描，`cwds: Option<Vec<PathBuf>>` 控制仓库扫描；`None` 等价于空列表。`ExternalAgentConfigMigrationItemType`（`L19-L25`）的四种类型为 `Config`、`Skills`、`AgentsMd`、`McpServerConfig`。`McpServerConfig` 目前只可被描述/打指标，导入分支是空操作。`ExternalAgentConfigMigrationItem`（`L27-L32`）携带类型、可展示的 `description`、可选仓库根 `cwd`。`ExternalAgentConfigService`（`L34-L38`）保存 `codex_home`、`claude_home`。
- `ExternalAgentConfigService::new`（`L40-L47`）接收 `codex_home`，用 `default_claude_home()` 推导 Claude 家目录；没有目录创建或验证。测试专用 `new_for_test`（`L49-L55`）允许注入两个路径。
- `detect`（`L57-L74`）先在 `include_home` 为真时以 `repo_root=None` 扫描用户级配置，再按 `cwds` 顺序调用 `find_repo_root`；找不到根目录的 cwd 被跳过；任一步 I/O、JSON/TOML 错误立即返回 `io::Error`，成功返回按扫描顺序排列的迁移项。它只检测，不写目标文件；没有并发同步。
- `import`（`L76-L108`）按传入 vector 顺序分派：`Config` 调 `import_config`，`Skills` 调 `import_skills` 并把复制数打入指标，`AgentsMd` 调 `import_agents_md`，`McpServerConfig` 静默跳过；任一错误短路，之前成功的写入不会回滚，成功返回 `()`。每个分支都会调用 `emit_migration_metric`，指标不存在时无副作用。
- `detect_migrations`（`L110-L217`）是一次用户级或仓库级扫描。配置源为 `settings.json`，目标为 `config.toml`；源存在才读取 JSON，根必须是对象，转换结果为空则不报迁移。目标已有文件时读取空文件为 TOML 空表，否则解析并用 `merge_missing_toml_values` 试合并；只有仍有缺失键才产生 `Config` 项。技能源/目标分别是用户 `~/.claude/skills` 与 home 旁 `.agents/skills`，或仓库 `.claude/skills` 与 `.agents/skills`；`count_missing_subdirectories` 只比较一级目录。Agents 源在仓库优先选根 `CLAUDE.md`、再选 `.claude/CLAUDE.md`，用户级固定 `~/.claude/CLAUDE.md`；源须非空，目标须不存在或仅空白。每个发现项含路径描述、同一 `cwd`，并单独打 detect 指标。已有有效目标不会被覆盖。
- `home_target_skills_dir`（`L220-L225`）把 `codex_home` 的父目录与 `.agents/skills` 拼接；无父目录时退回相对 `.agents/skills`。这是纯路径计算。
- `import_config`（`L227-L277`）先按 cwd 找仓库根；有非空但不在仓库中的 cwd 直接 `Ok(())`，无 cwd 才采用用户家目录。源不存在、迁移表为空、合并无变化均直接成功返回。目标父目录缺失时报 `InvalidData`，否则创建目录；目标不存在就写迁移表，已存在则解析后只补缺失表项，再写回。写入格式由 `write_toml_file` 统一产生并追加单个换行；解析/序列化/读写错误均返回。
- `import_skills`（`L279-L317`）选择仓库或用户源/目标；非仓库非空 cwd 返回 `0`，源目录不存在也返回 `0`。创建目标目录，遍历一级目录，忽略非目录和已存在目标；新目录由 `copy_dir_recursive` 递归复制，返回新技能目录数。目录遍历、类型查询、复制错误短路；同名目标整体保留，不做合并。
- `import_agents_md`（`L319-L345`）选择仓库候选源或用户 `CLAUDE.md`；非仓库非空 cwd、无候选源、源为空、目标非空均返回成功且不写。目标父目录不存在时报 `InvalidData`，否则创建目录并用 `rewrite_and_copy_text_file` 写入改写内容。
- `default_claude_home`（`L348-L354`）优先 `HOME`，其次 `USERPROFILE`，都没有时用相对 `.claude`；不检查存在性。`find_repo_root`（`L356-L390`）过滤空 cwd；相对路径相对当前工作目录解析，必须先存在；文件 cwd 转为父目录；从当前目录向上寻找 `.git` 文件或目录，找到即返回，否则返回最初存在路径作为 fallback，因此“无 Git”不等于找不到仓库。
- `collect_subdirectory_names`（`L392-L406`）对非目录返回空集合，读取目录并收集一级子目录名；`count_missing_subdirectories`（`L408-L415`）返回源集合减目标集合的数量。`is_missing_or_empty_text_file`（`L417-L426`）不存在为真、目录为假、文件按 `trim()` 判空；`is_non_empty_text_file`（`L428-L434`）要求普通文件且内容非空。`find_repo_agents_md_source`（`L436-L447`）按根 `CLAUDE.md`、`.claude/CLAUDE.md` 顺序返回第一个非空文件。
- `copy_dir_recursive`（`L449-L473`）先创建目标，递归目录；普通文件若 `is_skill_md` 为真则文本改写，否则 `fs::copy`。符号链接等非目录/非普通文件被忽略。`is_skill_md`（`L475-L479`）只按文件名大小写不敏感匹配 `SKILL.md`。
- `rewrite_and_copy_text_file`（`L481-L485`）完整读文本，调用 `rewrite_claude_terms` 后覆盖写目标。`rewrite_claude_terms`（`L487-L499`）先把大小写不敏感且有边界的 `claude.md` 换成 `AGENTS.md`，再依次把 `claude code`、`claude-code`、`claude_code`、`claudecode`、`claude` 换成 `Codex`；顺序意味着前一步生成的 `AGENTS.md` 不会再次被替换。
- `replace_case_insensitive_with_boundaries`（`L501-L538`）以 ASCII 小写副本搜索，使用 `is_word_byte`（`L540-L542`：ASCII 字母/数字/下划线）判断前后边界，保留未匹配片段并替换；空 needle 或无命中返回原字符串。它按字节索引，输入必须是合法 UTF-8（`&str` 保证），非 ASCII 字节只影响边界判定。`build_config_from_external`（`L544-L582`）要求 JSON 根对象；只迁移非空 `env` 为 `[shell_environment_policy] inherit="core"` 和 `set` 表，且仅保留可转字符串的值；`sandbox.enabled == true` 才写 `sandbox_mode="workspace-write"`，其他外部字段忽略。`json_object_to_env_toml_table`（`L584-L594`）遍历键值，`json_env_value_to_string`（`L596-L604`）保留字符串、布尔、数字，丢弃 null/数组/对象。
- `merge_missing_toml_values`（`L606-L633`）递归合并两个 TOML 表：已有标量不覆盖，双方同为表才递归，缺失键克隆加入；任一根/递归节点不是表返回 `InvalidData`。`write_toml_file`（`L635-L639`）pretty 序列化，错误转 `InvalidData`，写入 `trim_end` 后加换行。`is_empty_toml_table`（`L641-L651`）仅空表为真，标量/数组均非空。`invalid_data_error`（`L653-L655`）构造 `io::ErrorKind::InvalidData`。
- `migration_metric_tags`（`L657-L672`）把类型映射为 `config`、`skills`、`agents_md`、`mcp_server_config`；只有 `Skills` 添加 `skills_count`，缺省计数为 0。`emit_migration_metric`（`L674-L688`）取全局 OpenTelemetry metrics；无 provider 直接返回，有则递增一次 counter，tag 值借用临时 vector，仅在调用期间有效。测试模块声明在 `L690-L692`。

## 状态、取消、恢复与副作用

本文件没有 async、线程、锁、channel、取消 token、超时、重试或显式恢复协议；调用是同步阻塞文件 I/O。`detect` 只读并发调用理论上可并行，但同时 import/detect 可能观察到半写文件；`import_config` 的“读、合并、写”不是原子事务，多个 importer 会丢失更新，技能复制与文本写入也没有临时文件/rename。错误按调用点立即返回，已完成的前序项目不会撤销。没有 session 持久化；唯一外部副作用是创建目录、写 `config.toml`/`AGENTS.md`、复制技能文件和发 metrics。已有非空目标坚持不覆盖；空目标可覆盖。没有 MCP 导入实现，枚举值只是兼容占位。

## 源内测试与行为判据

同目录 `external_agent_config_tests.rs` 提供可独立验证的判据：`detect_home_lists_config_skills_and_agents_md`（`L16-L69`）验证用户级三类发现及描述顺序；`detect_repo_lists_agents_md_for_each_cwd`（`L71-L109`）验证嵌套 cwd 向上寻根且每个 cwd 产生项；`import_home_migrates_supported_config_fields_skills_and_agents_md`（`L111-L165`）验证 env 类型过滤、sandbox 映射、SKILL.md/CLAUDE.md 改写和 TOML 排版；`import_home_skips_empty_config_migration`（`L167-L186`）验证无支持字段不创建目标；`detect_home_skips_config_when_target_already_has_supported_fields`（`L188-L220`）与 `detect_home_skips_skills_when_all_skill_directories_exist`（`L222-L240`）验证幂等检测；`import_repo_agents_md_rewrites_terms_and_skips_non_empty_targets`（`L242-L285`）验证边界替换和非空目标保护；`import_repo_agents_md_overwrites_empty_targets`（`L287-L307`）验证空白目标可写；`detect_repo_prefers_non_empty_dot_claude_agents_source`（`L309-L341`）与 `import_repo_uses_non_empty_dot_claude_agents_source`（`L343-L368`）验证候选优先级；`migration_metric_tags_for_skills_include_skills_count`（`L370-L379`）验证指标标签；`import_skills_returns_only_new_skill_directory_count`（`L381-L397`）验证只计新目录。可独立判据是运行该模块测试后，目标内容、项顺序、错误/幂等结果与上述断言一致。

## zenpi Rust 映射

这段源代码可复用的是“强类型迁移项 + 检测/执行分离 + 缺失字段合并 + 可选指标”的组织方式，不能直接承担 DAG 通信。zenpi 当前已有可落点如下：

- 通信原语优先复用 `src/protocol.rs` 的 `MailboxRequest::{Send,Receive,Acknowledge,Claim,Complete}`（`L90-L116`）：`recipient_session_id` 可承载 parent、grandparent、直接 sibling、直接 child 的 session/worker ID，`message_id` 做幂等键，`ttl_ms` 做保活/过期界，`MailboxOutcome::{Succeeded,Failed,Abandoned}` 表示处理终态。事件广播复用 `StdioEvent`/`AgentEvent`；高频内部消息用 `src/runtime.rs` 的有界 command/event channel，避免无界邮箱。建议新增 `src/worker_graph.rs` 的 `WorkerNodeId`、`WorkerRelation`、`WorkerMessage`、`WorkerMailbox` 和 `WorkerGraph`，将关系校验与路由集中在此处，而不是让 provider 直接互发消息。
- 会话父子关系可落在 `src/core.rs` 的 `Turn.parent_id`（`L143-L207`）和已有 `src/session_tree.rs` 的 `TreeEntry.parent_id`/`children`：新增 worker 节点时同时记录 `session_id`、`parent_worker_id`、`depth`、`branch_id`；grandparent 通过父节点再上溯，直接 sibling 由同一 `parent_worker_id` 的子集合求得，直接 child 由边表查询。必须定义边只能指向已知节点、禁止自环/环、父节点最多一个，DAG 的多子关系存为持久化事件而非只放内存。
- 最小保活/派生机制：在 `worker_graph.rs` 维护 `WorkerStatus::{Running,Green,Failed,Cancelled,Closed}`、`required_children` 与 `last_heartbeat`；实现 `can_close(node)`，只有节点自身为 `Green` 且所有递归 child/grandchild 均为 `Green` 才转 `Closed`。若存在未完成/失败后新工作，`ensure_live_or_spawn(node, work)` 保持父节点 lease，向 `src/runtime.rs::BackgroundRunner` 提交新 job 并生成新 `WorkerNodeId`，写入 `worker_spawned`/`worker_status` 事件；失败或 TTL 到期只标记不可关闭并派生替代 worker，不能静默 close。可验证断言：任一非绿后代时 `can_close=false`；全部后代绿后恰好一次 `Closed` 事件；派生 worker 有正确 parent。
- `src/headless.rs` 已有有界请求 mailbox、取消与 reconnect journal（例如 `L39-L66`、`L267-L316`、`L556-L721`），应作为外部 JSONL 宿主：增加 DAG mailbox/tree 命令的解析、事件序号和重放；断线时保留消息 TTL 和终态，不能把 stdout 缓存当作 DAG 真相。
- `src/core.rs` 的 `Agent`/`AgentEvent`/`AgentPhase`（`L275-L338`、`L402-L448`）是 worker 执行与状态投影落点。建议 `Agent` 持有 `Arc<WorkerGraph>` 或由 session coordinator 注入 graph handle；在 turn 完成、工具失败、子节点汇报时发 `WorkerMessage` 和 `AgentEvent::Handoff`/新事件，并把 close 判定放在 core 的状态转换处。现有 `WorkerExecutionBinding`/lease 校验可复用来限制子 worker 的身份。
- `src/session.rs` 是持久化落点：它是 append-only JSONL，适合追加 `worker_spawned`、`worker_message_sent`、`worker_message_completed`、`worker_heartbeat`、`worker_green`、`worker_closed` 事件；恢复时重建 graph 与未完成 mailbox。借鉴本文件的“缺失项才合并”语义，恢复不得覆盖已确认的状态；写失败必须使 close/approval fail-closed。可执行验证是重启后 `SessionStore` 仍能重建 parent/grandparent/sibling/child 邻接和未完成消息。
- `src/tool_runtime.rs` 的 `execute_tool_batch`（`L157-L170`、`L275-L349`）提供有界并发、取消轮询和“不 detach 未知副作用”的语义；worker 派生前应经过该批处理的 admission，取消后不要把未确认工具当成绿色。`src/runtime.rs` 的 `CancellationToken`（`L49-L100`）用于 cooperative cancel，`BackgroundRunner::try_submit/try_cancel/try_shutdown`（`L318-L344`）用于派生、保活和有界关闭；非合作 job 只能在 grace 后 detach，不能声称已回滚。
- `src/approval.rs` 的 `ApprovalCoordinator`/`ApprovalPolicy`（`L126-L216`、`L472-L580`）是跨 worker 副作用闸门：把 `WorkerNodeId`、operation/lease 绑定到 approval request，取消 epoch 变化时拒绝过时回应；只有持久化的人工批准才可恢复，不能因后代“绿色”绕过审批。
- `src/providers/**`（`openai.rs`、`anthropic.rs`、`codex.rs`、`google.rs`、`deepseek.rs`、`connection.rs`、`registry.rs`）只负责 provider 请求/流式结果。它们应接收带 worker binding 的 backend context 并报告进度/错误到 core；不得在 provider 层创建 DAG 边、写 mailbox 或决定 close。

差异清单（可执行、可验证）：(1) 当前 `MailboxRequest` 没有关系级广播和发送者认证，新增 graph 层校验 recipient 是否为 parent/grandparent/sibling/child，并由 owner 注入 sender；(2) 当前 `Turn.parent_id` 是会话记录关系而非 worker DAG，新增独立 worker edge 事件及恢复索引；(3) 当前 runtime 只有 job 级终态，没有递归后代绿门，新增 `can_close`/`ensure_live_or_spawn` 并测量重复 close/派生幂等；(4) 当前 session 没有 heartbeat/TTL/未完成 mailbox 恢复，追加事件并在重启测试；(5) 当前 headless replay 主要面向 stdout，需把 mailbox sequence/ack 纳入同一持久化游标；(6) approval、tool batch、provider 结果都要携带 worker ID，否则无法证明“节点及全部后代”对应同一 DAG。

## 未决问题

- 源文件无法确认 zenpi 期望的 DAG 节点 ID 格式、消息最大字节数、heartbeat 周期、TTL 默认值、最大深度/子节点数、失败后重试次数及“绿色”是否需要人工批准；这些应在 `worker_graph.rs`/`protocol.rs` 的常量和 schema 中明确并由测试锁定。
- `McpServerConfig` 在源中只有枚举和指标标签、没有导入实现（`L103-L104`、`L661-L666`）；无法据此推断 zenpi 是否需要 MCP worker 通道。

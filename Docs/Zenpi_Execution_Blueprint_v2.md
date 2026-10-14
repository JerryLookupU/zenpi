# zenpi Execution Blueprint v2 — 唯一权威执行蓝图

> 4.0.0（2026-10-14）。按 b3ehive `execution-cron-builder` 蓝图标准编写：单一权威、
> `[ ]`/`[_]`/`[x]` 三态清单、真实 `Depends` DAG、按块分层的严格层门、
> `Owned paths` 归属、可执行 Validators、同名只读 Gantt 投影。
> 本蓝图分为两大块：
> - **A 块 · Agent 内核**：TUI 与 headless 两种形态。目标是对齐 pi 的极简与 1:1 正确性，用任务级测试证明，并在 16 核上承载 2000 个并发 agent。
> - **B 块 · Android 远程**：Android 客户端遥控 Unix 主机上的 zenpi，体验与桌面 TUI 对齐，同时适配竖屏。网络设计借鉴 Tailscale：身份就是密钥，地址只是提示。
>
> 原 Stage 2 已并入 B 块，不再单独作为权威文件。
> 凡不是目标必需的内容，一律按奥卡姆剃刀 / YAGNI 冻结或删除。

```yaml
schema_version: execution-blueprint/v2
blueprint_version: 4.0.0
revision_date: 2026-10-14
status: bootstrap-active
authoritative: true
authority_switch_item: ZE-100
authority_switched_at: 2026-10-14
selector: Docs/execution/active_requirement.json
supersedes: Docs/Zenpi_Execution_Blueprint.md, Docs/stage_1_v3_pi_mono_blueprint.md, Docs/stage2_android2unix_blueprint.md
stable_id_pattern: '^ZE-[0-9]{3}$'
legacy_id_namespaces: V2, ZP, ZS1, S2
status_values: '[ ]|[_]|[x]'
blocks: 'A = agent core (ZE-0xx..4xx, 999); B = Android remote (ZE-5xx)'
layers: A0, A1, A2, A3, A4, B1, B2, B3, B4
layer_gate: strict-within-block
claim_policy: any-layer-when-depends-closed
per_item_code_loc_cap: 5000
per_item_code_loc_rule: 'estimated_loc < 5000'
loc_basis: 'implementation + test LOC touched (deletions count; pure file moves count only edited lines); docs count zero'
handoff_path: 'Docs/quality/ze/<ID>.json'
gantt_companion: Docs/Zenpi_Execution_Blueprint_v2_Gantt.md
validator: tools/validate_blueprint_v2.py
gantt_generator: tools/generate_blueprint_v2_gantt.py
target_repo: /Users/mac/Github/zenpi
target_baseline: current-worktree-with-user-changes
product_modes: tui, headless, android-remote
reference_minimal_agent: weiyangzen/pi-mono (fork of badlogic/pi-mono)
concurrency_model: one-os-process-per-agent, blocking-io, no-async-runtime
scale_target: '16 CPU cores, 2000 concurrent headless agents'
```

## 1. 权威与范围

v1（ZP）、Stage 1（ZS1）、Stage 2（S2）均已标为 `authoritative: false`，并指向本文件。ZE-100 已将 `active_requirement.json` 切到本文件，Stage 2 的选择器已停用，`/sync` 也已改为写入本文件。三份旧文件现在只保留为证据和规格参考，各旧编号的去向见 §8。

## 2. 判定"必做"的标准

一项工作满足以下任一条件才进入清单：

| 标准 | 含义 |
|---|---|
| (a) 基本任务 | 缺了它，基本编码任务（读、改、跑、修、续）在 TUI 或 headless 下会失败或给出错误结果 |
| (b) pi 对齐 | 与 pi 语义不一致，且这种不一致会损害模型表现 |
| (c) 规模 | 实测证明 2000 并发目标需要它 |
| (d) 证明 | 它是证明 (a)–(c) 或 (e) 所需的测试或门禁 |
| (e) Android 远程 | 用户明确要求 Android 遥控与体验对齐，因此属于 B 块的范围。块内同样适用奥卡姆剃刀：只做竖屏下的"核心体验对齐"，不做桌面功能的全量搬运 |

执行原则：
1. **先删后加。** 一个概念只留一个 owner（来自 codex-rust-deslop 的约束）。
2. **一进程一 agent，阻塞 IO，不引入 tokio。** 参照 redis 的 ae 事件循环：没有事件就睡到下一个截止时间，不设固定 tick。
3. **任务级测试是交付物。** 每个 A2/A3/B2/B3 项至少要有一条走真实路径的断言。
4. **冻结不等于删除。** 只有当冻结模块阻碍了核心项时，才另开条目处理。

### 2.1 A 块规模预算（ZE-403 验收阈值）

以下阈值属于 `external_reality_cap`：首轮在 16 核实机上测量后，Master 可以改写。数值是本项目自己的策略，不是照搬 redis 的默认值。

测试条件：N=2000 个 headless agent，脚本化 provider 延迟 200 ms，每个任务 3 轮。

| 指标 | 阈值 |
|---|---|
| 任务成功率 | 100% |
| 每 agent 空闲 RSS（p99） | ≤ 8 MiB，总计 ≤ 24 GiB |
| 整机 CPU：全部空闲 / 全部在等 provider | 均 ≤ 1 核 |
| 每进程空闲唤醒（`ru_nvcsw` 增量） | ≤ 5 次/秒 |
| 每 agent 线程数 | 空闲 ≤ 3，活跃 ≤ 5 |
| 每 agent fd 数 | ≤ 16 |
| 每轮额外开销 p99 | ≤ 100 ms |
| 每次工具调用的 fsync | ≤ 2 次，并报告 p99 |
| 每请求写入 WAL 的字节数 | ≤ 终态记录大小 + 1 KiB |
| 因资源上限导致的失败 | 0，报告中写明所需的 `ulimit -n`、`kern.maxfiles`、`kern.maxprocperuid` |

## 3. B 块设计（据 6 路调研定稿）

### 3.1 架构决策

| 决策 | 选择 | 理由与备选 |
|---|---|---|
| 主机形态 | 常驻的 `zenpi host serve`；同时保留"挂载在桌面 TUI 上"的模式，两者共用同一个 `ProjectOwnerPool` | 目前不带 `--desktop` 的 `host pair` 会随邀请一起在 300 s 后退出，这是 bug |
| 线协议 | 保留 remote 协议 v1 并冻结 schema（4 字节长度前缀 + 严格 JSON、CAS 写入、回执、事件 cursor），删除 `HeadlessHost` 重复的那套运行时，改为复用 headless 的准入逻辑 | 另一种做法是"TLS 里直接跑 headless JSONL"，主机侧能少 6–7k 行，但 Android 传输层要重写，而且手机与桌面就不能共享同一个 owner。这一方案记为 LATER |
| 身份 | 主机身份 `host_id = sha256(SPKI)`，证书可以轮换；设备用 AndroidKeyStore 中的 P-256 密钥（保持现状）；每次连接用 nonce 挑战签名 | 借鉴 Tailscale/iroh"按密钥拨号"的做法：私钥永不离开设备 |
| 控制面 | 二维码本身就是控制面：一次性 token 加主机公钥，再加**有序的地址提示列表**；授权与撤销由主机侧注册表负责 | Tailscale 的协调服务器只负责分发公钥和端点；zenpi 只有单用户，用不着服务器 |
| 授权 | 在桌面上确认配对后，直接授予该项目的 Control 和 Approval 权限；撤销在下一帧生效 | 现在配对后仍"无权限"，还要手动执行 4 个 grant CLI，违背"扫码即用"；那个 6 位安全码可以从二维码推算出来，没有安全价值，删除 |
| 连通性 | **T0** 局域网直连，然后 **T1** 用户自己的 Tailscale / Headscale / WireGuard 叠加网络（主机端零依赖，Android 只放行这些网段的 VPN 路由）；兜底方案在文档里写明 `ssh -R` 到自己的 VPS | T2 盲中继（只转发密文，按密钥寻址，可参考 DERP）和 T3 iroh sidecar 都是 LATER。iroh 会多出约 137 个 crate、引入 tokio，体积增加约 45%，只能做成独立 sidecar |
| 拨号 | 按提示顺序尝试，局域网优先；任何一个提示上证书 pin 不符都直接判为致命错误，不回退到下一个 | Tailscale 的经验：一旦路径会变，就绝不能再以 IP 作为安全边界 |
| 后台 | 不常驻 socket；靠 `ResumeWorker` 发审批通知（需 POST_NOTIFICATIONS 权限，不用前台服务） | 真正的推送需要 FCM 或中继，记为 LATER |

### 3.2 竖屏信息架构（对齐桌面体验）

| 桌面 | Android 竖屏 |
|---|---|
| Conversation 面板 + prompt | **主屏**：单栏对话流，加底部固定的输入框（发送 / steer / 停止三态） |
| Arch 面板 + prompt | 主屏上的 Agent/Arch 模式切换 chip，共用同一个输入框 |
| 工具状态行 / markdown | 默认折叠为一行的卡片，点开看详情；代码块可横向滚动 |
| Diff | 次级页：全屏，逐文件，可回到原滚动位置 |
| 审批模态框 | 常驻审批条加底部面板（i/N，一次点击即生效）；后台时改为带操作按钮的通知 |
| 两排项目 / worktree 标签 | 抽屉里的会话收件箱，按"需要你 / 运行中 / 空闲"分组；主屏顶部放 worktree chip |
| 斜杠命令 | 输入 `/` 弹出底部命令板，`@` 选择文件 |
| 模型 / reasoning | 顶栏的模型 chip，点开底部面板 |
| Shell | 次级页：全屏终端，键盘上方有按键行 |
| Resources / Tasks | 次级页 |
| Gantt、Learn、Goal、布局预设、并发 ± | 手机端不做，在桌面上操作 |

```
┌──────────────────────────────┐
│ ☰ zenpi/main ▾   gpt-x ▾  ⋮  │
│ [Agent|Arch]  ●wt-a ○wt-b    │
├──────────────────────────────┤
│ you: fix the flaky test      │
│ ▸ read  src/tui.rs       ✓   │
│ ▸ edit  src/tui.rs +12 -3  ⧉ │
│ ▸ bash  cargo test    ⟳ 0:14 │
│ assistant: The race is in…   │
├──────────────────────────────┤
│ ⚠ bash rm -rf target [拒][允] │
├──────────────────────────────┤
│ [/] 消息或 steer…        ➤/■ │
└──────────────────────────────┘
```

## 4. 执行协议

- **三种状态**：`[ ]` 未完成或需返修；`[_]` worker 已自测，并在 `Docs/quality/ze/<ID>.json` 写好 handoff（格式为 `src/b3.rs` 中的 `HandoffRecord`/`EvidenceRecord`，至少含 `changed_files`、`commands_run`、`validation_result`，且 `commands_run` 覆盖该项的 Validators）；`[x]` Master 集成后全部 Validators 通过。worker 不写 `[x]`。DAG 或运行时里的 `green`、`finished` 只是遥测信号，永远不会改动勾选。
- **层门（按块）**：A 块依次为 A0 已交付基线、A1 权威 / 验证器 / CI / 测试底座、A2 正确性与 pi 对齐、A3 规模、A4 验收与发布；B 块为 B1 入库与接受、B2 设计与体验、B3 连接与后台、B4 验收与发布。块内低层还有 `[ ]` 或 `[_]` 时，高层不能为 `[x]`。依赖可以跨块，但不能指向层号更高的条目。
- **领取**：`Depends` 全部闭合即可领取，不受层的限制（这是对 b3ehive"只在最低未闭合层领取"规则的显式偏离）。共享以下文件、但彼此没有依赖关系的条目，由 Master 串行合并，ZE-109 会列出这类重叠：`src/tui.rs`、`src/session.rs`、`src/headless.rs`、`src/core.rs`、`src/project_workspace.rs`、`android/.../MainActivity.kt`。
- **行格式**：`- [ ] **ZE-NNN** — 标题；layer \`A2\` | Depends: … | Owned paths: … | Validators: … | Estimated LOC: N`。
- **公共门**（每项默认必须通过 G-BUILD + G-TEST + G-RUST；B 块的项还要通过 G-ANDROID）：

| 门 | 命令 |
|---|---|
| `G-BP` | `python3 tools/validate_blueprint_v2.py && python3 tools/generate_blueprint_v2_gantt.py --check` |
| `G-BUILD` | `cargo build --locked --release` |
| `G-TEST` | `cargo test --locked` |
| `G-RUST` | 见下方代码块（来自 codex-rust-deslop） |
| `G-TASK` | `cargo test --locked --test tasks --test tasks_headless --test tui_task` |
| `G-BENCH` | `python3 tools/bench_runtime.py --samples 3` |
| `G-LOAD(N)` | `python3 tools/load_agents.py --agents N`（阈值见 §2.1） |
| `G-REMOTE` | `cargo test --locked --features remote` |
| `G-ANDROID` | `cd android && ./gradlew :app:testDebugUnitTest lint verifyRoborazziDebug` |

```sh
# G-RUST
cargo fmt --all -- --check &&
cargo clippy --locked --all-targets -- -D warnings &&
cargo clippy --locked --all-targets --all-features -- -D warnings &&
python3 tools/check_modes.py &&
! git diff -U0 origin/main -- 'src/*.rs' 'src/**/*.rs' | grep -E '^\+.*#!?\[allow\((dead_code|unused)'
# 以下不阻断合并，但 handoff 中必须列出：新增的非测试 unwrap/expect/panic!、
# 没有 `// SAFETY:` 注释的 unsafe、新增 Arc<Mutex> 或大值 clone 的理由
```

## 5. A 块 · Agent 内核

### A0 — 已交付基线（按 ZP/ZS1 原验收口径；括号内为修复其已知缺陷的条目）

- [x] **ZE-001** — Agent loop、steer/follow-up 持久队列、有界工具批次、doom-loop 防护（已知缺陷：ZE-202、ZE-210、ZE-211）；layer `A0` | Depends: — | Owned paths: `src/core.rs`, `src/input_queue.rs`, `src/tool_runtime.rs`, `src/runtime.rs` | Validators: 继承 ZS1-101、ZS1-102、ZS1-187 验收证据 | Estimated LOC: 0
- [x] **ZE-002** — append-only JSONL session、分支树、fork、`/new`、session 命名与 `-s` 恢复（已知缺陷：ZE-231、ZE-236）；layer `A0` | Depends: — | Owned paths: `src/session.rs`, `src/session_tree.rs` | Validators: 继承 ZS1-105、ZS1-106、ZS1-132、ZS1-178 | Estimated LOC: 0
- [x] **ZE-003** — 版本化语义 checkpoint 与真实 backend 摘要（已知缺陷：ZE-232）；layer `A0` | Depends: — | Owned paths: `src/context.rs` | Validators: 继承 ZS1-103、ZS1-104 | Estimated LOC: 0
- [x] **ZE-004** — Provider 层：模型 registry 与能力协商、Responses/Chat SSE、Anthropic 与 Gemini 原生 adapter、第三方供应商开放、模型生命周期状态（已知缺陷：ZE-220..227）；layer `A0` | Depends: — | Owned paths: `src/backend.rs`, `src/backend/`, `src/providers/`, `src/protocols/` | Validators: 继承 ZS1-107、ZS1-108、ZS1-109、ZS1-110、ZS1-118、ZS1-176、ZS1-186 | Estimated LOC: 0
- [x] **ZE-005** — 认证：Codex 浏览器/设备 OAuth、刷新、凭据存储、API key 解析（首启登录引导 PA10 仍是 partial，按 §6.1 冻结；未接线的入口由 ZE-108 清理）；layer `A0` | Depends: — | Owned paths: `src/auth/`, `src/tui/bootstrap.rs` | Validators: 继承 `Docs/Zenpi_Provider_Auth_Implementation.md` 与已提交的测试 | Estimated LOC: 0
- [x] **ZE-006** — 内置工具 read/list/search/find/write/edit/run_command，输出 artifact、流式脱敏、ripgrep 搜索（已知缺陷：ZE-200..207、ZE-213、ZE-214）；layer `A0` | Depends: — | Owned paths: `src/tools.rs`, `src/tool_output.rs`, `src/tool_output/`, `src/search.rs` | Validators: 继承 v1 ZP 工具项与 ZS1-111、ZS1-112 | Estimated LOC: 0
- [x] **ZE-007** — 审批协调器、写前 diff 预览、批准后摘要重校验、审计回执；layer `A0` | Depends: — | Owned paths: `src/approval.rs`, `src/security.rs`, `src/governance.rs` | Validators: 继承 v1 ZP-011 与 ZS1-124 | Estimated LOC: 0
- [x] **ZE-008** — SKILL.md 发现与按需加载、参数模板、资源原子 reload（已知缺陷：ZE-233、ZE-234）；layer `A0` | Depends: — | Owned paths: `src/skills.rs`, `src/prompt_templates.rs`, `src/resource_loader.rs` | Validators: 继承 ZS1-113、ZS1-114 | Estimated LOC: 0
- [x] **ZE-009** — 类型化 subprocess hooks 与生命周期撤销；layer `A0` | Depends: — | Owned paths: `src/extensions.rs`, `src/extension_runtime.rs` | Validators: 继承 ZS1-115 | Estimated LOC: 0
- [x] **ZE-010** — Headless stdio JSONL v2：有界帧、请求 ID 幂等、reconnect、与 TUI 共享项目上下文（已知缺陷：ZE-240..243、ZE-303）；layer `A0` | Depends: — | Owned paths: `src/headless.rs`, `src/protocol.rs` | Validators: 继承 ZS1-126、ZS1-159 | Estimated LOC: 0
- [x] **ZE-011** — TUI 输入：多行编辑、历史、粘贴折叠与粘贴突发识别、排队消息编辑、kill/yank、外部编辑器、命令/文件补全与模型选择；layer `A0` | Depends: — | Owned paths: `src/tui.rs`, `src/external_editor.rs`, `src/slash.rs`, `src/slash_actions.rs`, `vendor/crossterm/` | Validators: 继承 ZS1-122、ZS1-123、ZS1-129、ZS1-130、ZS1-131 | Estimated LOC: 0
- [x] **ZE-012** — TUI 渲染：滚动复制、退出与信号时恢复终端、Codex 交互矩阵、热区模型、按热区的 Ctrl-C 语义（已知缺陷：ZE-250..255）；layer `A0` | Depends: — | Owned paths: `src/tui.rs`, `src/render.rs`, `src/view_model.rs`, `src/layout.rs` | Validators: 继承 ZS1-125、ZS1-128、ZS1-177、ZS1-179 | Estimated LOC: 0
- [x] **ZE-013** — 项目 workspace 与 worktree 双排标签、目录选择器、SSH 工作目录、持久化（§6.1 冻结）；layer `A0` | Depends: — | Owned paths: `src/project_workspace.rs`, `src/directory_picker.rs` | Validators: 继承 ZS1-120、ZS1-121、ZS1-140、ZS1-141、ZS1-142、ZS1-146、ZS1-157、ZS1-167、ZS1-168、ZS1-169、ZS1-170 | Estimated LOC: 0
- [x] **ZE-014** — 内嵌 PTY Shell 面板、VT 字符网格、IME 热区（§6.1 冻结，已知缺陷记在 §6.1）；layer `A0` | Depends: — | Owned paths: `src/pty_shell.rs` | Validators: 继承 ZS1-151、ZS1-155、ZS1-166、ZS1-171、ZS1-172、ZS1-173 | Estimated LOC: 0
- [x] **ZE-015** — 运行时面板：Conversation/Arch 两个独立 session 与两个 prompt、Gantt 面板、信息头（§6.1 冻结）；layer `A0` | Depends: — | Owned paths: `src/tui.rs`, `src/layout.rs` | Validators: 继承 ZS1-145、ZS1-147、ZS1-148、ZS1-150、ZS1-152、ZS1-153、ZS1-156、ZS1-165 | Estimated LOC: 0
- [x] **ZE-016** — b3ehive 领域：Blueprint/Goal/Learn 记录、`/execute` `/sync` `/explore` `/addloop`、可验证外部结果（§6.1 冻结）；layer `A0` | Depends: — | Owned paths: `src/b3.rs`, `src/domains.rs`, `src/domain_store.rs`, `src/domain_execution.rs`, `src/runtime_intent.rs`, `src/sync.rs` | Validators: 继承 ZS1-116、ZS1-119、ZS1-143、ZS1-144 | Estimated LOC: 0
- [x] **ZE-017** — Resources 资源总线、局域网感知、LAN headless 集群（§6.1 冻结）；layer `A0` | Depends: — | Owned paths: `src/resources.rs`, `src/net_probe.rs`, `src/cluster.rs` | Validators: 继承 ZS1-149、ZS1-154、ZS1-158、ZS1-160、ZS1-161、ZS1-163、ZS1-164 | Estimated LOC: 0
- [x] **ZE-018** — 生产入口与单进程 footprint 预算；layer `A0` | Depends: — | Owned paths: `src/main.rs`, `src/lib.rs`, `tools/bench_runtime.py` | Validators: 继承 ZS1-117、ZS1-162 | Estimated LOC: 0
- [x] **ZE-019** — DAG 编排最小集：类型化关系、claim/ack 邮箱与重投、worker 心跳、`dag_*` 工具（已知缺陷：ZE-260、ZE-308）；layer `A0` | Depends: — | Owned paths: `src/dag.rs` | Validators: 继承 ZS1-407 | Estimated LOC: 0
- [x] **ZE-020** — Exa `websearch` 工具（没有对应的 ZS1 项，按已合入 main 的 commit e8733e6 记入基线）；layer `A0` | Depends: — | Owned paths: `src/tools.rs` | Validators: commit e8733e6 已合入 main；`tools.rs:1120` 注册 | Estimated LOC: 0

### A1 — 权威、验证器、CI 绿基线、测试底座

- [x] **ZE-100** — 权威切换：`Docs/execution/active_requirement.json` 指向本文件，旧的 Stage 1 选择器归档到 `Docs/execution/archive/`；v1、Stage 1、Stage 2 文件头改为 `authoritative: false` 并写入 `superseded_by`；停用 Stage 2 selector；`/sync` 改为优先读取激活的选择器，扫描时跳过 `_Gantt` 投影，并按目标 blueprint 的 `stable_id_pattern` 生成新 ID；CI 改为运行本文件的校验器和 Gantt 检查；layer `A1` | Depends: ZE-101, ZE-102 | Owned paths: `Docs/execution/active_requirement.json`, `Docs/execution/stage2_android2unix_active_requirement.json`, `Docs/execution/archive/`, `Docs/Zenpi_Execution_Blueprint.md`, `Docs/stage_1_v3_pi_mono_blueprint.md`, `Docs/stage2_android2unix_blueprint.md`, `src/sync.rs`, `tests/stage1_sync.rs`, `.github/workflows/ci.yml` | Validators: G-BP；`grep -l '^authoritative: true' Docs/*.md` 只返回本文件；`cargo test --locked --test stage1_sync`（含 `sync_follows_active_selector_and_blueprint_id_prefix`） | Estimated LOC: 90
- [x] **ZE-101** — 重写 `tools/validate_blueprint_v2.py`：解析 checkbox 行，拒绝重复 ID、缺失或未知依赖、依赖成环、非法状态、LOC ≥ 5000、违反层门的 `[x]`、依赖未闭合的 `[x]`、指向更高层的依赖；layer `A1` | Depends: — | Owned paths: `tools/validate_blueprint_v2.py`, `tools/test_validate_blueprint_v2.py` | Validators: `python3 tools/test_validate_blueprint_v2.py`；G-BP | Estimated LOC: 260
- [x] **ZE-102** — 同名只读 Gantt：`tools/generate_blueprint_v2_gantt.py` 生成 `Docs/Zenpi_Execution_Blueprint_v2_Gantt.md`（mermaid 图、每个 ID 恰好出现一次、带源文件摘要、不含可变 checkbox；`--check` 检测是否过期）；layer `A1` | Depends: ZE-101 | Owned paths: `tools/generate_blueprint_v2_gantt.py`, `Docs/Zenpi_Execution_Blueprint_v2_Gantt.md` | Validators: G-BP；改动任一状态后 `--check` 失败 | Estimated LOC: 160
- [ ] **ZE-104** — 共享的脚本化 provider：同一份脚本同时驱动进程内的 `ScriptedBackend`（记录请求，可在指定轮次停住以测试取消）和本地 HTTP/SSE 假服务器（覆盖 Responses、Chat、Anthropic 三种 wire）；用它替换至少 3 个重复的 mock；layer `A1` | Depends: — | Owned paths: `tests/support/mod.rs`, `tests/support/scripted.rs` | Validators: `cargo test --locked --test support_selftest` 证明同一脚本在两条路径上产生相同的请求序列 | Estimated LOC: 400
- [ ] **ZE-105** — Provider 录制回放套件：把 `Docs/quality/stage1/provider-kill319/**` 下 68 份真实 `.sse` 收进 `tests/fixtures/sse/`，按 wire 回放并断言事件序列；layer `A1` | Depends: ZE-104 | Owned paths: `tests/provider_replay.rs`, `tests/fixtures/sse/` | Validators: `cargo test --locked --test provider_replay` | Estimated LOC: 220
- [ ] **ZE-106** — TUI 生产 loop 的测试注入点：让 `run_async_with_remote` 可以注入事件源和 `TestBackend`，用脚本化按键加 ZE-104 的 provider 跑真实 loop，对 transcript 做快照；layer `A1` | Depends: ZE-104 | Owned paths: `src/tui.rs`, `tests/tui_task.rs` | Validators: `cargo test --locked --test tui_task` 跑通 prompt → 流式回答 → 退出 | Estimated LOC: 300
- [ ] **ZE-107** — 验收工作区里已经实现但没有勾选的 ZS1-180（`--auto`）、ZS1-181（思考中动画）、ZS1-182（审批确认分级与拒绝反馈）、ZS1-184（`--max-tool-iterations`）：运行它们原先登记的 Validators，通过后在本文件关闭；layer `A1` | Depends: — | Owned paths: `src/core.rs`, `src/approval.rs`, `src/tui.rs` | Validators: 原 ZS1-180/181/182/184 Validators 全部通过 | Estimated LOC: 50
- [ ] **ZE-108** — CI 绿基线与 Rust 规范门（来源：codex-rust-deslop、zenpi 自审）。内容如下：<br>• 默认 feature 和 all-features 两套 clippy 都要清零 `-D warnings`；<br>• auth 模块里未接线的登录入口（`cancel_login`、`CancelDisposition` 等 PA10 残留）删除，或移到 `#[cfg(test)]` 下，不新增任何 `allow(dead_code)`；<br>• 修掉约 21 处机械类 lint；<br>• clippy 加上 `--locked`；<br>• 诊断 host-smoke 的 `shared-projects` 失败：若是产品 bug 就修复，若只是在测已冻结的多项目功能，就从 CI 矩阵移除并记录理由；<br>• 确认扩展的 pipe 夹具稳定；<br>• `Docs/quality/rust-deslop-gate.md` 改为指向 G-RUST；<br>• 依赖数门按实测临时重设基线（ZE-520 拆出 remote 后再收紧）；<br>layer `A1` | Depends: ZE-100 | Owned paths: `src/auth/`, `.github/workflows/ci.yml`, `tools/stage1_host_smoke.py`, `tests/stage1_extension_hooks.rs`, `Docs/quality/rust-deslop-gate.md`, `tools/bench_runtime.py`，以及 clippy 输出点名的文件（清单写入 handoff） | Validators: G-RUST；PR 上连续 3 次 CI 全绿 | Estimated LOC: 300
- [ ] **ZE-109** — 执行协议机械化（来源：b3ehive bridge contract）。内容如下：<br>• 校验器要求每个 `[_]`/`[x]` 都有 `Docs/quality/ze/<ID>.json`，其摘要可复核，且 `commands_run` 覆盖该条目的 Validators；<br>• 列出共享 owned paths 但彼此没有依赖关系的未完成条目；<br>• Gantt 增加 claim 与集成状态两列，数据取自 handoff；<br>• 为 ZE-100..102 补写回溯 handoff；<br>• 把 `Docs/learn/files/b3ehive/docs/blueprint.md_transform.md` 里的指向改为本文件；<br>layer `A1` | Depends: ZE-101, ZE-102 | Owned paths: `tools/validate_blueprint_v2.py`, `tools/test_validate_blueprint_v2.py`, `tools/generate_blueprint_v2_gantt.py`, `Docs/quality/ze/`, `Docs/learn/files/b3ehive/docs/blueprint.md_transform.md` | Validators: G-BP；删除任一 handoff 或篡改其摘要后 G-BP 失败 | Estimated LOC: 220

### A2 — 正确性与 pi 对齐（每项附任务级测试）

**工具：**

- [ ] **ZE-213** — 给模型的工具结果改为纯文本（对齐 pi 的 content 与 details 分离）。各内置工具输出的文本如下：<br>• read：原文，加 `[Showing lines a-b of N. Use offset=K to continue]`；<br>• bash：输出尾部，加 `Command exited with code N`、截断说明和完整输出的路径；<br>• edit/write：一行成功信息，diff 只给 UI；<br>• search：`path:line: text`。<br>各 adapter 只发送这段文本加上 `is_error`，结构化 JSON 只留在日志和 UI 中；layer `A2` | Depends: ZE-104 | Owned paths: `src/core.rs`, `src/tools.rs`, `src/protocols/responses.rs`, `src/protocols/chat.rs`, `src/protocols/anthropic.rs`, `src/protocols/google.rs` | Validators: 请求体快照中 `read_file` 的结果是未转义的原文，且不含 `call_id`/`output_capture`；任务测试：从含引号和制表符的文件 read 出旧文本后原样 edit，能够成功 | Estimated LOC: 220
- [ ] **ZE-200** — 子进程继承用户环境：保留 `PATH`/`HOME`/`USER`/`SHELL`/`TMPDIR` 等，剔除已知的密钥变量；ripgrep、扩展、项目 shell 都用同一份环境；layer `A2` | Depends: ZE-104 | Owned paths: `src/security.rs` | Validators: 任务测试 `run_command "command -v cargo && echo $HOME"` 成功；包含密钥的环境变量不出现在子进程中 | Estimated LOC: 60
- [ ] **ZE-201** — `run_command` 对齐 pi：<br>• stdout 和 stderr 合并为单个管道；<br>• 非零退出作为 `is_error` 结果返回，带输出尾部和退出码，永不丢弃输出；<br>• 保留**尾部** 50 KB / 2000 行，全量写入 artifact；<br>• `timeout` 可选，且没有默认值；<br>layer `A2` | Depends: ZE-213 | Owned paths: `src/tools.rs` | Validators: `sh -c 'echo out; exit 3'` 返回 "out"、退出码 3 和 `is_error`；`seq 100000` 返回尾部，`truncated=true`，artifact 中有全量 | Estimated LOC: 160
- [ ] **ZE-202** — 工具超时或 IO 失败时作为错误结果回给模型，不再映射为 `UnknownOutcome`，不阻断后续 compaction；layer `A2` | Depends: ZE-201 | Owned paths: `src/core.rs` | Validators: 任务测试：命令超时 → 模型收到超时文本 → 修复后正常结束，且 `automation_vetoed()` 为 None | Estimated LOC: 80
- [ ] **ZE-203** — `read_file` 按行用 `offset`/`limit` 分页（每页 2000 行 / 50 KB），在字符边界截断，取消 512 KB 的上限；layer `A2` | Depends: ZE-213 | Owned paths: `src/tools.rs` | Validators: 2 MB 的中文文件能分页完整读出，不出现 `InvalidUtf8` | Estimated LOC: 80
- [ ] **ZE-204** — 写/编辑的单次调用上限与 `MAX_WRITE_BYTES` 对齐；工具参数超限时只让这一次调用失败，不再让整轮 `InvalidTurn`；同时复核 256 KiB 的批次上限；layer `A2` | Depends: ZE-104 | Owned paths: `src/tools.rs`, `src/core.rs` | Validators: 80 KB 的 `write_file` 成功；超限调用返回错误结果，本轮继续 | Estimated LOC: 40
- [ ] **ZE-205** — edit 规范化：剥离 BOM；按 LF 规范化后匹配，并恢复原来的行尾；采用 pi 的模糊回退，唯一性在规范化后判定；layer `A2` | Depends: ZE-104 | Owned paths: `src/tools.rs` | Validators: CRLF 文件配 LF 写的 `old` 文本能成功，且行尾保持 CRLF；弯引号用例成功；有歧义的匹配仍然拒绝 | Estimated LOC: 110
- [ ] **ZE-206** — 接受工作区内的绝对路径，以及 `~`、前导 `@` 写法；越界路径仍然拒绝；layer `A2` | Depends: ZE-104 | Owned paths: `src/tools.rs` | Validators: 工作区内的绝对路径可以读写；`/etc/passwd` 被拒绝；指向工作区外的 symlink 被拒绝 | Estimated LOC: 40
- [ ] **ZE-207** — 搜索改为单次 `rg --json`：默认按 regex 匹配，需要时用 `--fixed-strings`；删掉高级参数路径上的 256 文件列表和"每 16 个文件一批"的分发；手写扫描只在找不到 rg 时兜底；layer `A2` | Depends: ZE-213 | Owned paths: `src/search.rs`, `src/tools.rs` | Validators: 带 glob 时，第 1000 个文件里的匹配也能找到；单次调用只启动一个 rg 进程 | Estimated LOC: 300
- [ ] **ZE-214** — 默认工具集与 `--tools` 白名单：`dag_*` 只对 DAG 派生的 worker 或显式开启时暴露；`websearch` 只在解析到 key 时暴露；`--tools a,b` 限定对模型可见的工具集合；layer `A2` | Depends: — | Owned paths: `src/core.rs`, `src/tools.rs`, `src/dag.rs` | Validators: 默认请求里不出现 `dag_spawn`；`--tools read_file,run_command` 恰好暴露 2 个工具 | Estimated LOC: 70

**Agent loop：**

- [ ] **ZE-210** — 工具调用之后，模型回复空文本或只有 reasoning 时，本轮正常结束。协议层与 core 两处都要放行；layer `A2` | Depends: ZE-104 | Owned paths: `src/core.rs`, `src/protocols/chat.rs`, `src/protocols/responses.rs`, `src/protocols/anthropic.rs` | Validators: 任务测试 `empty_final_text_after_tools_completes_turn`，三种 wire 各跑一次 | Estimated LOC: 60
- [ ] **ZE-211** — 输入队列的记录数和字节数只统计未结束的输入；`recover()` 不再每次都重放全部事件；layer `A2` | Depends: — | Owned paths: `src/input_queue.rs` | Validators: 单个 session 累计 5000 次 steer/follow-up、共 8 MiB 后，仍然可以入队 | Estimated LOC: 50
- [ ] **ZE-212** — 只给 `run_active_turn_cancelable` 的事件缓冲设上限；修正 `core.rs:562` 里"default is 8"的过期注释；layer `A2` | Depends: — | Owned paths: `src/core.rs` | Validators: 100 轮都不 drain 时，缓冲长度仍不超过上限 | Estimated LOC: 20

**Provider：**

- [ ] **ZE-220** — 保留有界（4 KB）的错误响应体；新增 `BackendError::ContextOverflow`，按 OpenAI、Anthropic、Gemini 各自的已知标记分类；headless 的 `retryable` 改为读取错误本身，不再写死为 `true`；layer `A2` | Depends: ZE-105 | Owned paths: `src/backend.rs`, `src/protocols/`, `src/headless.rs` | Validators: 各 wire 的 400/413 回放识别为 `ContextOverflow`，且 `retryable=false` | Estimated LOC: 160
- [ ] **ZE-221** — `Completion` 增加 `StopReason{Stop,Length,ToolUse,Refusal}`，length、max_tokens、incomplete 都保留已生成的内容。因长度被截断的那一轮如果带有工具调用，一律不执行，每个调用回一条"output token limit reached"错误结果，循环继续；layer `A2` | Depends: ZE-105 | Owned paths: `src/backend.rs`, `src/protocols/`, `src/core.rs` | Validators: max_tokens 回放返回部分文本和 `Length`；任务测试 `length_truncated_tool_calls_not_dispatched` | Estimated LOC: 160
- [ ] **ZE-222** — 把实际在用的 Codex、OpenAI、Claude、Gemini 模型编进目录（含 tools、reasoning、真实上下文窗口）。显式指定连接但未编目的模型，不再静默关闭 tools；同步修改 `connection.rs:1265-1275` 里断言旧行为的测试；layer `A2` | Depends: ZE-105 | Owned paths: `src/providers/registry.rs`, `src/providers/connection.rs` | Validators: `openai-codex` 默认模型解析后带 tools 和 reasoning；默认 Codex 配置下的任务测试能发起工具调用 | Estimated LOC: 100
- [ ] **ZE-223** — 接受别名解析后服务端返回的模型 ID（记录下来，不拒绝）；旧 Anthropic 路径改用 `x-api-key` 加 `anthropic-version`；layer `A2` | Depends: ZE-105 | Owned paths: `src/protocols/anthropic.rs`, `src/protocols/google.rs`, `src/backend.rs` | Validators: 用别名请求、响应里是带日期的模型 ID → Ok；断言请求头 | Estimated LOC: 40
- [ ] **ZE-224** — Chat dialect 发送 `reasoning_effort`；layer `A2` | Depends: ZE-105, ZE-222 | Owned paths: `src/protocols/chat.rs` | Validators: 请求体快照中带 `reasoning_effort` | Estimated LOC: 20
- [ ] **ZE-225** — 缓存与用量：Anthropic 在 system 和最后一个用户块上加 `cache_control`；Responses/Codex 的 `prompt_cache_key` 设为 session ID；`Usage` 增加 cache_read/cache_write 字段，供 ZE-232 做真实用量锚点；layer `A2` | Depends: ZE-105 | Owned paths: `src/protocols/anthropic.rs`, `src/protocols/responses.rs`, `src/protocols/mod.rs`, `src/backend.rs` | Validators: 请求体快照；回放后断言用量字段 | Estimated LOC: 100
- [ ] **ZE-226** — 放宽脆弱的校验：<br>• Chat：允许 stop 与工具调用同时出现，允许缺 `choices`/`index`；<br>• Anthropic：忽略未知事件；<br>• Gemini：总 token 数只要不小于各项之和即可；<br>layer `A2` | Depends: ZE-105 | Owned paths: `src/protocols/chat.rs`, `src/protocols/anthropic.rs`, `src/protocols/google.rs` | Validators: 各 wire 构造的变体回放都通过 | Estimated LOC: 40
- [ ] **ZE-227** — 跨模型历史转换：目标模型或 wire 与某轮的 `native_history` 不同时，丢弃不透明的 reasoning 和签名，把可见的 thinking 转成文本，把工具调用 ID 规范化到目标 wire 的限制以内；同一模型仍按原样严格重放；layer `A2` | Depends: ZE-105 | Owned paths: `src/backend.rs`, `src/protocols/anthropic.rs`, `src/protocols/google.rs`, `src/protocols/responses.rs` | Validators: Anthropic thinking 会话 `/model` 切到 OpenAI 模型后，下一次请求成功；Gemini thought 切到 Claude 成功；同模型重放逐字节一致 | Estimated LOC: 140

**Session 与上下文：**

- [ ] **ZE-230** — 基础 system prompt。内容如下：<br>• 工具清单与使用准则，写明各工具的上限、分页方式和截断规则；<br>• 一行环境信息：cwd、日期、OS；<br>• 发现 AGENTS.md，没有时用 CLAUDE.md。查找范围是全局 `~/.zenpi/` 加上从工作区根到 cwd 的各级目录，按全局 → 根 → cwd 的顺序合并，总字节数有上限，读失败时跳过并发出事件；<br>• SYSTEM.md / APPEND_SYSTEM.md 分别覆盖或追加 system prompt。<br>本项取代 ZS1-192，不依赖 openspec；layer `A2` | Depends: ZE-104 | Owned paths: `src/system_prompt.rs`, `src/core.rs`, `src/lib.rs`, `tests/system_prompt.rs` | Validators: 用嵌套目录夹具断言合并顺序和文件头；CLAUDE.md 回退；超出上限时截断；不可读的文件被跳过；instructions 计入 token 预算；工具描述中包含 offset 续读提示 | Estimated LOC: 260
- [ ] **ZE-231** — 撕裂的末尾可以恢复：<br>• 可写打开时，把没有换行的末行复制为 `.torn` 后截掉；<br>• 列举 session 时跳过损坏文件，而不是整体失败；<br>• `write_all` 失败后截回 `observed_bytes`；<br>• 新建 session 文件后对所在目录 fsync；<br>layer `A2` | Depends: ZE-104 | Owned paths: `src/session.rs` | Validators: 写入半行后，`open_in_workspace` 加 `append_turn` 成功；有一个损坏的兄弟文件时，`/session resume-last` 仍然可用 | Estimated LOC: 120
- [ ] **ZE-232** — compaction 对齐 pi（对应 learn：`compaction.ts_learn`）。内容如下：<br>• 切点：可以在 assistant/tool-result 边界切入单个长 turn，turn 前缀单独做摘要；工具结果不能作为切点；<br>• 被摘要的区段里丢弃不透明的 reasoning 状态，不再整体拒绝；<br>• 摘要输入改为带角色标签的文本，每条工具结果截到 2000 字符，去掉元数据和签名；<br>• 历史输出上限为 floor(0.8×reserve)，前缀上限为 floor(0.5×reserve)，两者串行；摘要请求本身也要通过上下文窗口校验；<br>• 接受自由文本摘要，不再要求模型逐字回显文件列表；<br>• 新增配置 `{enabled, reserve_tokens=16384, keep_recent_tokens=20000}`；<br>• 触发条件为 `最近一次真实 usage + 估算 > window − reserve`；<br>• 收到 `ContextOverflow` 时压缩一次并重试一次；<br>layer `A2` | Depends: ZE-220, ZE-225, ZE-104 | Owned paths: `src/context.rs`, `src/config.rs`, `src/core.rs` | Validators: 1 个 prompt 加 200 次工具调用的会话可以压缩；带 Anthropic 签名的夹具可以压缩；原始工具输出超过窗口时，摘要请求仍小于窗口；首次溢出 → 恰好 1 次摘要加 1 次重试并成功；第二次溢出返回 `context_overflow` | Estimated LOC: 480
- [ ] **ZE-233** — 模板和技能展开后的文本写进用户 turn；技能正文移出 system prompt，让 system prompt 在各轮之间保持稳定以命中缓存；`selected_skills` 只看当前分支；layer `A2` | Depends: ZE-230 | Owned paths: `src/core.rs` | Validators: `/tpl a` 之后，第二轮以及重启之后的请求都包含展开后的文本，且 system prompt 逐字节一致 | Estimated LOC: 80
- [_] **ZE-234** — **（最先修：目前会导致无法启动）** 单个无效的 SKILL.md、模板或 `.gitignore` 只产生诊断信息并被跳过，不再中止加载，也不再导致启动失败；layer `A2` | Depends: — | Owned paths: `src/skills.rs`, `src/prompt_templates.rs`, `src/resource_loader.rs`, `src/core.rs`, `tests/stage1_skill_md.rs`, `tests/stage1_prompt_resources.rs`, `tests/stage1_resource_host.rs` | Validators: 一个坏的加一个好的 SKILL.md → 加载 1 个技能并给出 1 条警告（`/skills` 的 `diagnostics`）；二进制能正常启动；证据见 `Docs/quality/ze/ZE-234.json` | Estimated LOC: 40
- [ ] **ZE-235** — 删除没有生产调用方的旧 compaction（`prepare_context`、`context::restore_checkpoint`）及 `tests/context.rs`；`CompactionCheckpoint` 是 `CompactionReport` 的公开字段，要连同序列化后的报告一起迁移；layer `A2` | Depends: ZE-232 | Owned paths: `src/context.rs`, `src/core.rs`, `tests/context.rs` | Validators: `grep -rn 'fn prepare_context' src` 为空；`CompactionReport` 序列化测试通过 | Estimated LOC: 280
- [ ] **ZE-236** — session 默认行为对齐 pi（对应 learn：`session-manager.ts_learn`，并与 pi 核心能力清单交叉确认）：<br>• 没有 `--session`/`-s` 时，新建 `~/.zenpi/sessions/<cwd-key>/<ts>-<id>.jsonl`；<br>• `-c/--continue` 打开文件头中 cwd 与当前一致的最新 session，只读文件头；<br>• `--no-session` 不落盘；<br>• 默认打开路径统一走 `open_in_workspace`；<br>• `ZENPI_SESSION` 保持可用；<br>layer `A2` | Depends: ZE-231 | Owned paths: `src/core.rs`, `src/session.rs`, `src/project_workspace.rs`, `tests/cli_session.rs` | Validators: 在不同 cwd 下各跑一次 headless，得到不同的日志文件；`-c` 不会打开其他 cwd 的 session；200 个并行的默认启动中 `session_locked` 为 0；`--no-session` 不写任何文件 | Estimated LOC: 220

**Headless：**

- [ ] **ZE-240** — 异步生产 host 在 prompt 入队时就判定 `mode` 和 `expected_turn_id`（目前要等排队执行时才判定）：`start_if_idle` 在忙时立即返回 `not_idle`；layer `A2` | Depends: ZE-104 | Owned paths: `src/headless.rs` | Validators: `run_async_streams` 测试：慢 prompt 进行中，`start_if_idle` 立即被拒绝 | Estimated LOC: 60
- [ ] **ZE-241** — 事件按因果顺序输出：每个 job 一个按序号排序的单一邮箱，取代"准入、增量、工具"三条队列分批写出。领取前先写失败测试核实该问题确实存在；layer `A2` | Depends: ZE-104 | Owned paths: `src/headless.rs` | Validators: tool_progress 先于其后产生的文本增量输出 | Estimated LOC: 80
- [ ] **ZE-242** — 生命周期：<br>• SIGTERM 等同于 `shutdown`；<br>• EOF 有可配置、有上限的排空时限；<br>• close hook 在进程退出前完成；<br>• session 被占用时返回明确的 `session_locked`；<br>layer `A2` | Depends: ZE-104 | Owned paths: `src/headless.rs`, `src/core.rs` | Validators: 一轮进行中 `kill -TERM`，得到终态响应且 WAL 干净；`printf '{prompt}\n' \| zenpi --mode headless --auto` 在终态事件后以 0 退出；第二个进程打开同一个 session 时以 `session_locked` 退出 | Estimated LOC: 100
- [ ] **ZE-243** — 删除二进制不使用的同步 `run_headless`/`run_stdio` 及其中不可达的 Prompt/Steer 分支，把约 41 个测试迁到 `run_async_streams`；layer `A2` | Depends: ZE-240 | Owned paths: `src/headless.rs`, `tests/headless_protocol.rs`, `tests/headless_domain_owner.rs` | Validators: `cargo test --locked headless_` 全部通过；`grep 'pub fn run_headless' src` 为空 | Estimated LOC: 900

**TUI：**

- [ ] **ZE-250** — 修复 text→tool→text 的顺序：工具开始时结束当前流式段，结束时只收尾当前段，不覆盖工具前的文本；layer `A2` | Depends: ZE-106 | Owned paths: `src/tui.rs` | Validators: `tui_task` 快照中，工具行两侧的文本各自在正确位置且完整 | Estimated LOC: 40
- [ ] **ZE-251** — 审批的过期清扫覆盖后台标签页；中断只作用于当前 job 所属的 owner；取消投递失败（QueueFull）时，提示"未送达，请重试"，而不是"已请求中断"；layer `A2` | Depends: ZE-106 | Owned paths: `src/tui.rs` | Validators: A 标签发起审批 → 切到 B → 处理或过期后，A 不残留模态框；中断 B 不影响 A | Estimated LOC: 70
- [ ] **ZE-252** — 双击 Ctrl-C 的计数只在同一热区内累计：Shell 热区的第一次按键交给 PTY，不计入；其余保持 ZS1-179 的语义；layer `A2` | Depends: ZE-106 | Owned paths: `src/tui.rs` | Validators: 在 Shell 热区按一次 Ctrl-C，再到 Conversation 按一次，不会强杀 job | Estimated LOC: 30
- [ ] **ZE-253** — `COMMAND_SPECS` 作为唯一的命令目录：补齐 `/reasoning` `/tree` `/output` `/input` `/reload` `/skills` `/templates` 以及 `resource` 别名；`/reasoning` 也能在 headless 的 command 帧中使用，使 headless 可以在运行时调整思考级别；`/approval` 不带参数时只显示当前模式；`/yolo` 不带参数时切换开关；删除 `/review` 别名；layer `A2` | Depends: — | Owned paths: `src/slash.rs`, `src/tui.rs`, `src/headless.rs` | Validators: 每个可解析的命令名都有对应的 spec；headless 发送 `/reasoning high` 后，请求体快照发生变化；不带参数的 `/approval` 不改变模式 | Estimated LOC: 100
- [ ] **ZE-254** — 删除旧的同步 TUI 路径（`run`、`run_interactive`、`run_with_state*`、`agent=None` 分支），把 `stage1_input_host` 的端到端测试迁到 ZE-106 的注入点上；layer `A2` | Depends: ZE-106 | Owned paths: `src/tui.rs`, `tests/stage1_input_host.rs` | Validators: `grep -n 'fn run_interactive' src/tui.rs` 为空；迁移后的测试通过 | Estimated LOC: 500
- [ ] **ZE-255** — panic 时恢复终端：`TerminalGuard::enter` 安装一个串接原有 hook 的 panic hook，执行与 `leave()` 相同的恢复步骤；layer `A2` | Depends: — | Owned paths: `src/tui.rs`, `tools/tui_transcript_ux_smoke.py` | Validators: PTY smoke 中，debug 构建通过环境变量触发 panic，退出后能看到 `\e[?1049l`，且 `stty -a` 与启动前一致 | Estimated LOC: 60

**DAG：**

- [ ] **ZE-260** — DAG 正确性：<br>• worker 默认用 `current_exe()`；<br>• 存储满时 `send` 返回错误，不再静默丢弃；<br>• 写入改为临时文件 + fsync + rename 的原子写；<br>• 消息体按字节限制；<br>• 回收子进程；<br>• `dag_spawn` 限制深度 ≤ 3，且扇出不超过父节点租约；<br>layer `A2` | Depends: — | Owned paths: `src/dag.rs`, `src/tools.rs`, `tests/dag.rs` | Validators: `dag_send_full_mailbox_errors`；`dag_spawn_defaults_to_current_exe`；写到一半被截断后存储仍可读；第 4 层 spawn 被拒绝 | Estimated LOC: 130

### A3 — 16 核 / 2000 并发：空闲零开销（依据 redis ae/networking/aof）

- [ ] **ZE-300** — headless 去轮询（对应 redis ae：睡到下一个真实截止时间）：<br>• stdin、runtime、审批三类唤醒合并为一个阻塞 channel；<br>• `worker_loop` 改用 condvar，job 完成时主动唤醒；<br>• job 线程创建失败时发出类型化的 `SpawnFailed`，并继续消化排队中的任务；<br>• shutdown 之后再 submit 时显式返回 `Rejected`；<br>layer `A3` | Depends: ZE-243 | Owned paths: `src/headless.rs`, `src/runtime.rs` | Validators: 空闲进程 10 s 内 `ru_nvcsw` 增量 ≤ 50；200 个进程都卡在 10 s 的脚本化 provider 上时，总 CPU < 单核的 1%；stdout 读端暂停 5 s 期间，stdin 发出的 cancel 在 250 ms 内送达 job；注入 spawn 失败后，下一个排队任务仍能完成 | Estimated LOC: 300
- [ ] **ZE-301** — 传输层：进程内共享一个 `ureq::Agent`（keep-alive 连接池、共享 TLS 配置）；在连接器内通过 `RequestControl` 的 Arc 标志实现取消，删除每个请求的发送线程和 5 ms park 轮询；layer `A3` | Depends: ZE-105 | Owned paths: `src/backend.rs`, `src/backend/transport.rs` | Validators: `stage1_transport` 的取消测试全部通过；对同一主机连续两次请求会复用连接；每个在途请求最多占 1 个线程；等待 provider 期间唤醒次数为 0 | Estimated LOC: 180
- [ ] **ZE-302** — 工具、搜索、扩展的读取改用 `poll(2)` 阻塞等待：单个线程同时监听两个 fd 和子进程，poll 超时取"工具截止时间"与"100 ms 取消检查"中较小的一个；删除 2–5 ms 的 sleep 循环和固定 50 ms 等待；进程组信号只发给尚未被回收的 leader；改动涉及的 `unsafe` 一律加 `// SAFETY:` 注释；layer `A3` | Depends: ZE-201, ZE-207 | Owned paths: `src/tools.rs`, `src/search.rs`, `src/extension_runtime.rs` | Validators: 一次工具调用新增的线程数为 0；200 个并发 `sleep 2` 的总 CPU < 5%；不存在因 PID 复用而误杀的情况 | Estimated LOC: 220
- [ ] **ZE-303** — reconnect WAL 收敛（对应 redis aof：只对确认点做持久化，写失败视为"提交不确定"）：<br>• `Event` 记录不再写入 WAL，改放内存环形缓冲；<br>• 只对 Reserved、Terminal、Ack 三类记录 fsync，且 Terminal 先 fsync 再写 stdout；<br>• 任何一次追加失败后，日志被毒化，之后的追加都返回 `CommitUncertain`；<br>• 打开时用 write、`sync_all`、rename、目录 fsync 的方式压缩；<br>• 截断撕裂的末尾前，先复制为 `.torn`；<br>• 请求 ID 账本与 terminals 一起按 FIFO 修剪；<br>• 干净关闭且没有未知结果时，删除 WAL；<br>layer `A3` | Depends: ZE-243 | Owned paths: `src/session.rs`, `src/headless.rs` | Validators: 每个请求写入 WAL 的字节数 ≤ 终态大小 + 1 KiB；每个请求的 fsync ≤ 3 次；注入短写和 fsync 错误后仍能重新打开；干净 shutdown 后 export/delete 成功；2 万次带 ID 的 `status` 全部成功，被淘汰的 ID 返回 `replay_expired` | Estimated LOC: 260
- [ ] **ZE-304** — session 日志的 fsync 策略：只对 turn、`operation_*` 和终态记录 fsync，其他记录在下一个边界时一起刷盘。崩溃时最多丢失最后一个边界之后的记录，不会丢失任何已接受的 turn。flock 改为阻塞等待，不再用 `LOCK_NB`。注意 macOS 上 `sync_data` 走的是 `F_FULLFSYNC`，单次代价较高；layer `A3` | Depends: ZE-231 | Owned paths: `src/session.rs` | Validators: 用计数器验证每次工具调用 fsync ≤ 2 次，并报告 p99 延迟；注入崩溃后不丢失 turn | Estimated LOC: 60
- [ ] **ZE-305** — session 内存瘦身：去掉 `records: Vec<Value>` 和 `event_values` 这两份重复副本（只保留序号、类型、偏移，需要时重新解析）；缓存已校验过的 checkpoint；`tree_ancestry_turns` 不再克隆整棵树；layer `A3` | Depends: ZE-232 | Owned paths: `src/session.rs`, `src/core.rs`, `src/context.rs` | Validators: 1 万条记录的 session 常驻内存 < 文件大小的 2 倍；每次请求做全量序列化的次数为 0；截断或损坏的记录在去重之后仍会被拒绝 | Estimated LOC: 260
- [ ] **ZE-306** — session 列表和"最新 session"查找只读文件头加 mtime，并按 cwd 过滤；layer `A3` | Depends: ZE-236 | Owned paths: `src/session.rs` | Validators: 列出 500 个 session 时，字节读取计数只覆盖文件头 | Estimated LOC: 70
- [ ] **ZE-307** — 凭据元数据缓存在内存中，只在需要刷新时才取账户锁。领取前先用计数器核实问题确实存在；layer `A3` | Depends: — | Owned paths: `src/auth/resolve.rs`, `src/auth/store.rs` | Validators: 100 个并行解析请求中，每个凭据版本最多读 1 次文件 | Estimated LOC: 70
- [ ] **ZE-308** — 拆分 DAG 存储：<br>• `nodes.json` 用原子写（rename 加目录 fsync），只在拓扑或状态变化时写；<br>• 每个节点一个 append-only 的 `inbox/<node>.jsonl`，追加时 fsync，ack 用游标记录；<br>• 心跳写在独立文件里；<br>• `wait_for_message` 只检查自己 inbox 的大小，发现变大才加锁；<br>• DAG 的状态永远不会改动蓝图的勾选；<br>layer `A3` | Depends: ZE-260 | Owned paths: `src/dag.rs`, `tests/dag.rs` | Validators: 256 个线程各做 20 轮收发和心跳，无丢失、无重复；心跳不改变 `nodes.json` 的字节；未 ack 的消息会重投；2000 个等待中的 worker 合计 ≤ 0.25 核；写到一半崩溃后存储仍可解析 | Estimated LOC: 280
- [ ] **ZE-309** — TUI 渲染成本：按消息缓存已渲染的行，只让正在流式输出的那条失效；项目 checkpoint 不再包含 transcript（目前有改动时每 250 ms 序列化一遍全部 transcript）；删除 250 ms 和 20 ms 的无条件重绘，但事件循环的超时仍要包含粘贴突发的截止时间（对应 codex `paste_burst_learn`）；layer `A3` | Depends: ZE-106 | Owned paths: `src/tui.rs` | Validators: transcript 变长时单帧开销保持不变（bench）；输入或流式输出期间没有 checkpoint 写盘；普通（非 bracketed）粘贴突发在超时内刷新出来；空闲时事件循环完全静止 | Estimated LOC: 180

### A4 — 任务级验收、规模验收与发布

- [ ] **ZE-400** — 进程内 golden 任务套件（使用真实工具、临时工作区）。覆盖以下场景：<br>• 读文件后回答；<br>• 新建文件；<br>• 编辑文件并校验 diff；<br>• bash 成功，且输出进入下一次请求；<br>• bash 失败后模型修复；<br>• 同一轮发起 3 个并行工具；<br>• 需要审批的写入被拒绝，文件不落盘；同一写入获批后落盘；<br>• 在 provider 调用中取消、在工具执行中取消，日志保持一致；<br>• 轮中被杀后恢复，修复未完成的调用；<br>• 迭代上限或 doom-loop 上限止损；<br>• provider 出错后重试；<br>• 因长度截断的轮次不执行其中的工具调用；<br>• 会话中途切换模型后继续工作；<br>layer `A4` | Depends: ZE-202, ZE-203, ZE-205, ZE-210, ZE-213, ZE-221, ZE-227, ZE-232 | Owned paths: `tests/tasks.rs` | Validators: `cargo test --locked --test tasks`，每个任务都断言最终文件、日志记录和请求数 | Estimated LOC: 800
- [ ] **ZE-401** — 二进制级 headless 任务：针对 ZE-104 的假服务器启动 `zenpi --mode headless`，覆盖新建文件、bash、审批被拒、取消、杀进程后恢复、`-c` 续接；layer `A4` | Depends: ZE-400, ZE-236, ZE-240, ZE-242, ZE-303 | Owned paths: `tests/tasks_headless.rs` | Validators: `cargo test --locked --test tasks_headless` | Estimated LOC: 380
- [ ] **ZE-402** — TUI 任务快照，覆盖以下用例：<br>• text→tool→text；<br>• 跨标签页审批；<br>• 忙、闲、有草稿三种状态下分别按 Esc 和 Ctrl-C；<br>• 浮层打开时按 Esc 不会中断任务；<br>• 粘贴内容里夹带 `\n/exit`、`\n!rm` 时什么也不提交；<br>• 按键 Release 事件不产生任何动作；<br>• 粘贴折叠后再提交；<br>• 斜杠命令走真实 loop 完成往返；<br>layer `A4` | Depends: ZE-250, ZE-251, ZE-252, ZE-254, ZE-255 | Owned paths: `tests/tui_task.rs` | Validators: `cargo test --locked --test tui_task` | Estimated LOC: 380
- [ ] **ZE-403** — 2000 agent 压测 `tools/load_agents.py`：<br>• harness 用单个 `selectors`（kqueue）事件循环读取所有 stdout 管道（对应 redis ae）；<br>• 启动前先算 fd 预算（harness ≥ 3N+64，provider ≥ N+64）并上调软限制，不够则直接失败；<br>• 一个脚本化 provider 服务器，N 个 `--no-session` 或独立 session 的 headless 进程，每个任务含一次 `write_file` 和一次 `bash echo`；<br>• 报告 RSS、CPU、唤醒次数、线程数、fd、fsync 次数与延迟、磁盘写速率，以及内核限制；<br>layer `A4` | Depends: ZE-236, ZE-300, ZE-301, ZE-302, ZE-303, ZE-304, ZE-305, ZE-401 | Owned paths: `tools/load_agents.py` | Validators: `G-LOAD(2000)` 在 16 核机器上满足 §2.2 的全部阈值，报告存入 `Docs/quality/` | Estimated LOC: 340
- [ ] **ZE-404** — CI 门禁：每个 PR 运行 G-RUST、G-TASK 和 `G-LOAD(200)`；nightly 或手动运行 `G-LOAD(2000)`；layer `A4` | Depends: ZE-108, ZE-400, ZE-401, ZE-402, ZE-403 | Owned paths: `.github/workflows/` | Validators: CI 全绿 | Estimated LOC: 60
- [ ] **ZE-406** — 文档收口：<br>• README 和 `Docs/Zenpi_Execution_Spec.md` 只描述已验收的能力，列出冻结模块，并以 G-RUST 为准；<br>• v1、Stage 1、Stage 2 移到 `Docs/archive/`，同步修正引用它们的 validator 和 gantt 工具路径；<br>• 把与审计结论矛盾的 learn 笔记标为过期：`session-manager.ts_transform`（恢复语义）和 `tui.ts_transform`（合并重绘）；<br>• 在 `Cargo.toml` 的 crossterm patch 旁边注明上游 issue #1126、#1142；<br>layer `A4` | Depends: ZE-100, ZE-404 | Owned paths: `README.md`, `Docs/Zenpi_Execution_Spec.md`, `Docs/archive/`, `tools/`, `Docs/learn/files/pi-mono/`, `Cargo.toml` | Validators: G-BP；`tools/validate_blueprint.py` 和 `tools/validate_stage1_blueprint.py` 指向归档路径后仍能运行 | Estimated LOC: 40
- [ ] **ZE-999** — Master 终验：A、B 两块 全部为 `[x]`；G-BUILD、G-TEST、G-RUST、G-TASK、G-BENCH、`G-LOAD(2000)` 全部通过；Gantt 未过期，handoff 齐全；layer `A4` | Depends: ZE-400, ZE-401, ZE-402, ZE-403, ZE-404, ZE-406, ZE-599 | Owned paths: `Docs/Zenpi_Execution_Blueprint_v2.md` | Validators: 全部公共门 | Estimated LOC: 0

## 6. B 块 · Android 远程

> 现状：主机侧 `src/remote/` 约 16.6k 行，`host_pair.rs` 1.5k 行，37 个 `tests/remote_*.rs`；`android/` 有 236 个 `.kt` 文件，其中生产代码 8.1k 行，另有 611 个 JVM 测试和 137 个 instrumentation 测试。**这些全部未提交，也没有任何一项通过验收**；真机和实体局域网的回执全部为 false。B 块没有 B0 基线：现有成果先由 ZE-502 接受，之后才能计入。

### B1 — 入库与接受

- [ ] **ZE-500** — 代码入库与 CI 基线：<br>• 提交 `android/`、`src/remote/`、`src/host_pair.rs`、`tests/remote_*.rs`、`fuzz/remote_protocol/`；<br>• CI 增加 G-ANDROID，并在 CI 中运行原本 `#[ignore]` 的 8 个 `remote_android_*_interop` 测试；<br>• Stage 2 的执行脚手架（`tools/*stage2*`、`.ops/stage2_android2unix/`）中 CI 用不到的部分，原样移到 `Docs/archive/stage2/`；<br>layer `B1` | Depends: ZE-100, ZE-108 | Owned paths: `android/`, `src/remote/`, `src/host_pair.rs`, `tests/remote_*.rs`, `fuzz/remote_protocol/`, `.github/workflows/ci.yml`, `tools/`, `.ops/stage2_android2unix/`, `Docs/archive/stage2/` | Validators: `git ls-files android src/remote \| wc -l` > 0；G-REMOTE；G-ANDROID；interop 测试在 CI 中通过 | Estimated LOC: 150
- [ ] **ZE-502** — 接受已有的核心成果：<br>• 主机侧：线协议（frame、envelope v1、严格 JSON）、TLS 1.3 监听与 ALPN、二维码邀请与证书 pin 配对、设备注册表与撤销、owner 路由、事件 cursor、回放与订阅；<br>• Android 侧：`pairing/`、`security/`、`transport/`、`state/`；<br>• 运行原有的测试和 fuzz 语料，结果写进 handoff；<br>layer `B1` | Depends: ZE-500 | Owned paths: `src/remote/{frame,protocol,strict_json,listener,lan,pairing,auth,command,host_adapter,events,subscription,transcript,live_text,wakeup}.rs`, `android/app/src/main/java/zenpi/{pairing,security,transport,state}/` | Validators: G-REMOTE（含 `remote_protocol`、`remote_pairing`、`remote_auth`、`remote_events`、`remote_subscription_tls`、`remote_parity`）；G-ANDROID 的 JVM 部分全部通过 | Estimated LOC: 50

### B2 — 主机侧收敛、身份与网络、竖屏体验

- [ ] **ZE-510** — 把远程状态（回执、ack、intent）从 `project-workspace.json` 移到独立的 `remote-state.json`，保证不带 feature 的构建也能读取由 remote 构建写出的 checkpoint；layer `B2` | Depends: ZE-502 | Owned paths: `src/project_workspace.rs`, `src/remote/` | Validators: 用 `--features remote` 写出的 checkpoint，默认构建能够加载 | Estimated LOC: 400
- [ ] **ZE-511** — 把 worktree、worker、subtab 子系统从 `src/remote/` 移到 `src/worktree/`。这部分 TUI 也在用，行为不变，按 §7 冻结；目的是解除对 remote feature 开关的阻塞；layer `B2` | Depends: ZE-510 | Owned paths: `src/remote/{subtab,worker,worker_progress,worker_admission,worktree_host,worktree_runner}.rs`, `src/worktree/`, `src/tui.rs`, `src/project_workspace.rs` | Validators: G-TEST；`grep -rn 'remote::\(subtab\|worker\|worktree\)' src` 为空 | Estimated LOC: 600
- [ ] **ZE-512** — 删除 `HeadlessHost` 独立的 prompt runner 和 10 ms 轮询，远程宿主改为复用 headless 异步 host 的准入、取消和 steer 逻辑，保证同一个概念只有一个 owner；layer `B2` | Depends: ZE-243, ZE-502 | Owned paths: `src/remote/headless_host.rs`, `src/remote/host_adapter.rs`, `src/headless.rs` | Validators: `remote_headless`、`remote_headless_commands` 通过；`grep -n 'recv_timeout(Duration::from_millis(10))' src/remote` 为空 | Estimated LOC: 1200
- [ ] **ZE-513** — 常驻的 `zenpi host serve`：<br>• 监听器的生命周期与邀请解耦；<br>• 固定端口写进 identity；<br>• 默认绑定选中的网卡，而不是 0.0.0.0；<br>• 保留挂载在桌面 TUI 上的模式；<br>layer `B2` | Depends: ZE-502 | Owned paths: `src/host_pair.rs`, `src/remote/listener.rs`, `src/core.rs` | Validators: 端到端测试：配对 → 邀请过期 → 10 分钟后仍可重连；重启后端口不变 | Estimated LOC: 250
- [ ] **ZE-514** — 配对后即可使用：<br>• 桌面确认后，授予所挂载项目的 Control 和 Approval；<br>• 保留 `--list-devices` 和 `--revoke-device`，删除 4 个 grant CLI；<br>• 删除可以由二维码推出的 6 位安全码；<br>• Pair 和 Authenticate 的响应如实报告 `control_available`；<br>layer `B2` | Depends: ZE-513 | Owned paths: `src/remote/auth.rs`, `src/remote/pairing.rs`, `src/host_pair.rs`, `android/.../pairing/`, `tests/remote_auth.rs`, `tests/remote_device_grants.rs` | Validators: 新配对的设备能直接发 prompt；被撤销或重放的 token 会被拒绝；撤销在下一帧生效 | Estimated LOC: 600
- [ ] **ZE-515** — 基于密钥的身份与二维码 v2：<br>• `host_id = sha256(SPKI)`，证书轮换后 pin 仍然有效；<br>• 二维码 v2 带上与 token 绑定的有序地址提示（`lan`、`tailnet`），v1 仍可解析；<br>• Android 端已保存的主机自动迁移；<br>layer `B2` | Depends: ZE-514 | Owned paths: `src/remote/pairing.rs`, `src/remote/auth.rs`, `android/.../pairing/`, `android/.../transport/PairingClient.kt`, `android/.../security/PairedHostStore.kt` | Validators: v1 和 v2 邀请都按严格模式解析；证书轮换后仍能连接；已存主机的迁移测试 | Estimated LOC: 700
- [ ] **ZE-516** — 网络 T1 叠加网：<br>• 主机端 `--tailnet` 按网卡地址识别 `100.64.0.0/10` 和 `fd7a:115c:a1e0::/48`，加入地址提示，不调用 Tailscale API；<br>• Android 只对这两个网段放行 `TRANSPORT_VPN`；<br>• 多提示拨号：局域网优先，失败后依次尝试下一个；任何一个 pin 不符都直接失败，不回退；<br>• 用户指南补上 Tailscale/Headscale 的配置，以及 `ssh -R` 兜底方案；<br>layer `B2` | Depends: ZE-515 | Owned paths: `src/remote/lan.rs`, `src/host_pair.rs`, `android/.../lifecycle/LanSocketRouting.kt`, `android/.../lifecycle/NetworkMonitor.kt`, `android/.../transport/HostClient.kt`, `Docs/stage2_android2unix_user_guide.md` | Validators: CGNAT 和 fd7a 地址被识别为叠加网；局域网不可达时回退到 tailnet 提示；任一提示 pin 错误时直接失败；非叠加网段的 VPN 仍被拒绝 | Estimated LOC: 600
- [ ] **ZE-517** — 流式对齐：把 `live_text` 目前 4 KiB 的整行预览改为逐 token 的增量，回放时顺序正确；layer `B2` | Depends: ZE-502 | Owned paths: `src/remote/live_text.rs`, `src/remote/subscription.rs`, `android/.../state/SubscriptionReducer.kt` | Validators: interop 测试中增量有序、回放无重复也无遗漏 | Estimated LOC: 600
- [ ] **ZE-518** — 拆分 Android UI：<br>• 把 `PairingScreen`（约 60 个参数）拆到 `ui/` 包，`PairingState`（约 90 个字段）拆成子状态；<br>• 修掉每秒一次的空转重组；<br>• 草稿只以 host、session、tab 的 id 为键，不含 revision，避免远端改名时清空草稿；<br>行为保持不变；layer `B2` | Depends: ZE-502 | Owned paths: `android/.../MainActivity.kt`, `android/.../ui/`, `android/.../pairing/PairingViewModel.kt` | Validators: 137 个 instrumentation 测试的 testTag 不变且全部通过；空闲时每秒重组 0 次；subtab 的 revision 变化后草稿仍在 | Estimated LOC: 1800
- [ ] **ZE-519** — 竖屏截图测试底座：Roborazzi，分辨率 360×800 和 412×915，字号 1.0 和 1.3，覆盖明暗主题；layer `B2` | Depends: ZE-518 | Owned paths: `android/app/build.gradle.kts`, `android/app/src/test/java/zenpi/screenshot/`, `android/app/src/test/resources/golden/` | Validators: `./gradlew verifyRoborazziDebug`；主屏无横向裁切 | Estimated LOC: 400
- [ ] **ZE-530** — 对话主屏：<br>• 用 reverse 的 `LazyColumn`，自动贴底，带流式行；<br>• 工具行默认折叠；<br>• 轻量 markdown：代码块、行内代码、列表，不引入第三方库；<br>• 底部固定输入框：发送 / steer / 停止三态，配 `ImeAction.Send` 和 `imePadding`；<br>• 顶栏放 Agent/Arch 模式切换、worktree chip 和模型 chip；<br>layer `B2` | Depends: ZE-519, ZE-517 | Owned paths: `android/.../ui/ConversationScreen.kt`, `android/.../ui/Markdown.kt`, `android/.../ui/Composer.kt` | Validators: 截图黄金图；500 轮对话的 fixture 卡顿帧 < 5%；草稿在旋转后仍保留 | Estimated LOC: 1300
- [ ] **ZE-531** — 审批：常驻审批条加底部面板。面板显示 i/N 和彩色补丁，点一次即决定，有待决审批时无法关闭；可以开启生物识别确认；layer `B2` | Depends: ZE-530 | Owned paths: `android/.../ui/ApprovalSheet.kt`, `android/.../pairing/ApprovalDecisionPolicy.kt` | Validators: 滚动到任何位置审批条都可见；按队列顺序处理；摘要过期时按钮禁用；黄金图 | Estimated LOC: 450
- [ ] **ZE-532** — 抽屉收件箱：会话和项目按"需要你 / 运行中 / 空闲"分组，角标等于待审批数；项目和 worktree 的管理入口移进抽屉；layer `B2` | Depends: ZE-518 | Owned paths: `android/.../ui/SwitcherDrawer.kt` | Validators: `WorkspaceScreenTest`、`SubtabScreenTest`、`TabCapacityScreenTest` 通过；黄金图 | Estimated LOC: 600
- [ ] **ZE-533** — 次级页与路由栈：<br>• Sessions：搜索和恢复；<br>• Diff：全屏、逐文件、+/- 着色，返回时恢复原滚动位置；<br>• Terminal：按视口计算行列数，按键行在键盘上方，旋转后重新计算；<br>• Tasks；<br>• 模型面板；<br>layer `B2` | Depends: ZE-530 | Owned paths: `android/.../ui/nav/`, `android/.../ui/SessionsScreen.kt`, `android/.../ui/DiffScreen.kt`, `android/.../ui/TerminalScreen.kt`, `android/.../ui/TasksScreen.kt`, `android/.../ui/ModelSheet.kt` | Validators: 原有的 Session、Diff、Terminal、Worker、ModelStatus 屏幕测试迁移后通过；返回栈测试；黄金图 | Estimated LOC: 1800
- [ ] **ZE-534** — 输入框命令板：输入 `/` 时弹出底部面板，列出主机端的斜杠命令和资源；输入 `@` 时弹出文件选择；不支持的桌面专属命令在面板中注明；layer `B2` | Depends: ZE-530, ZE-253 | Owned paths: `android/.../ui/Composer.kt`, `android/.../ui/Palette.kt` | Validators: 在输入框中驱动 `ResourceCatalogScreenTest` 和 `WorkspaceFilesScreenTest` | Estimated LOC: 450
- [ ] **ZE-535** — 安全与无障碍：<br>• 默认开启 `FLAG_SECURE`；<br>• 支持深色和动态色；<br>• 同一个图标不再复用于多个功能；<br>• TalkBack 朗读内容不含 UUID，补上标题语义；<br>• release 构建开启 R8；<br>layer `B2` | Depends: ZE-533 | Owned paths: `android/.../ui/theme/`, `android/app/src/main/res/`, `android/app/build.gradle.kts` | Validators: Compose `enableAccessibilityChecks` 通过；release 包能安装并完成配对；深色主题黄金图 | Estimated LOC: 350

### B3 — 连接、后台与开关

- [ ] **ZE-520** — `remote` feature 开关（默认关闭）：<br>• `cfg` 覆盖 `src/remote/`、`host_pair`、rcgen、qrcode、`wakeup::notify` 和 `RemoteApprovalActor`；<br>• 去掉 rustls 中未使用的 `tls12`；<br>• 依赖数门按实测收紧；<br>• 核心 crate 永远不引入 tokio；<br>layer `B3` | Depends: ZE-510, ZE-511, ZE-108 | Owned paths: `Cargo.toml`, `src/lib.rs`, `src/core.rs`, `src/tui.rs`, `src/session.rs`, `src/approval.rs`, `src/pty_shell.rs`, `src/slash_actions.rs`, `tools/bench_runtime.py` | Validators: 默认构建中没有 rcgen 和 qrcode；`cargo tree -i tokio` 为空；`cargo clippy --locked --all-targets --features remote -- -D warnings` 通过；G-REMOTE；G-BENCH | Estimated LOC: 300
- [ ] **ZE-541** — 监听器加固：<br>• accept 改为事件驱动，不再每 10 ms 轮询；<br>• 每个 IP 只给 1 个未认证连接槽，未认证阶段总时限 5 s；<br>• 为已认证连接预留槽位；<br>layer `B3` | Depends: ZE-513 | Owned paths: `src/remote/listener.rs` | Validators: 有 N 个半开连接时，已认证客户端仍能连上；空闲时每秒唤醒 0 次 | Estimated LOC: 270
- [ ] **ZE-542** — 持久认证连接：订阅复用同一条 TLS 会话，空闲时每分钟握手不超过 1 次；订阅循环改由 wakeup 驱动，不再每 250 ms 内部调度 4 次；layer `B3` | Depends: ZE-541, ZE-512 | Owned paths: `src/remote/subscription.rs`, `src/remote/listener.rs`, `android/.../transport/HostClient.kt` | Validators: `RemoteProtocolCorpusTest`；空闲时握手计数 ≤ 1 次/分钟；断开重连后从 cursor 续传 | Estimated LOC: 800
- [ ] **ZE-543** — 设备注册表按 mtime 缓存，不再在每一帧都 flock 并解析一遍；撤销仍在下一帧生效；layer `B3` | Depends: ZE-514 | Owned paths: `src/remote/auth.rs` | Validators: 1000 帧只读取 1 次注册表；撤销测试通过 | Estimated LOC: 150
- [ ] **ZE-544** — 后台审批通知：<br>• 申请 `POST_NOTIFICATIONS` 权限；<br>• `ResumeWorker` 读到有待审批时，发一条高优先级通知，带"拒绝 / 允许一次"按钮（需要解锁），点击后深链到对应会话；<br>• 审批在别处处理后，1 s 内撤销这条通知；<br>• 不使用前台服务，也不使用 FCM；<br>layer `B3` | Depends: ZE-531, ZE-542 | Owned paths: `android/.../lifecycle/ResumeWorker.kt`, `android/app/src/main/AndroidManifest.xml` | Validators: `ResumeWorkerTest` 中有通知断言；生命周期校验脚本确认没有前台服务 | Estimated LOC: 350

### B4 — 验收与发布

- [ ] **ZE-590** — 生产端到端测试：`zenpi host serve` 配合 ZE-104 的脚本化 provider，Kotlin JVM 客户端走真实 TCP，依次完成配对 → prompt → 流式输出 → 审批 → 断线 → 回放，全程零重复、零遗漏；layer `B4` | Depends: ZE-104, ZE-514, ZE-517, ZE-542 | Owned paths: `tests/remote_host_e2e.rs` | Validators: `cargo test --locked --features remote --test remote_host_e2e`，不带 `#[ignore]` | Estimated LOC: 600
- [ ] **ZE-591** — Android 体验验收，覆盖以下必做要求：<br>• 审批通知在 2 s 内送达；<br>• 从通知深链进入会话不超过 1 s；<br>• 审批条在任何滚动位置都可见；<br>• 1 万轮对话滚动 p95 ≥ 55 fps（Macrobenchmark）；<br>• 断网 30 s 后重连，零重复、零遗漏，草稿保留；<br>• 离线时 500 ms 内渲染出缓存内容；<br>• 主要操作都在屏幕下方 40% 区域内；<br>layer `B4` | Depends: ZE-530, ZE-531, ZE-532, ZE-533, ZE-534, ZE-535, ZE-544 | Owned paths: `android/app/src/androidTest/`, `android/macrobenchmark/` | Validators: 上述每一条都有对应的 instrumentation 或 Macrobenchmark 断言 | Estimated LOC: 500
- [ ] **ZE-592** — 真机验收：<br>• 一台 API 35 模拟器；<br>• 一台实体手机连实体局域网：摄像头扫码、prompt、审批、Wi-Fi 断开后恢复、撤销；<br>• 走 Tailscale 叠加网完成一次完整运行；<br>layer `B4` | Depends: ZE-590, ZE-591, ZE-516 | Owned paths: `Docs/quality/ze/ZE-592.json` | Validators: 回执中 `physical_phone_lan_verified=true`，且 `overlay_verified=true` | Estimated LOC: 200
- [ ] **ZE-593** — 发布：<br>• 签名的 sideload APK，`apksigner verify` 通过，并做安装和升级冒烟测试；<br>• 用户指南精简为只写已验收的能力和离网用法；<br>• 记录 fat LTO 对 G-BUILD 的影响；<br>layer `B4` | Depends: ZE-592 | Owned paths: `android/app/build.gradle.kts`, `Docs/stage2_android2unix_user_guide.md`, `Cargo.toml` | Validators: `apksigner verify`；G-BP | Estimated LOC: 150
- [ ] **ZE-599** — B 块终验：B1–B4 全部 `[x]`，G-REMOTE、G-ANDROID、ZE-590、ZE-591、ZE-592 全部通过；layer `B4` | Depends: ZE-590, ZE-591, ZE-592, ZE-593 | Owned paths: `Docs/Zenpi_Execution_Blueprint_v2.md` | Validators: 全部公共门 | Estimated LOC: 0

## 7. 冻结与不做

### 7.1 冻结（保留现状，不再扩展）

| 模块 | 理由 |
|---|---|
| b3ehive 领域（`b3.rs`、`domains.rs`、`domain_store.rs`、`domain_execution.rs`、`runtime_intent.rs`）、`/goal` `/learn` `/blueprint` `/plan` `/compete` `/loop` `/execute` `/explore` | 属于编排层，不是 agent 内核，pi 也没有 |
| BentoBox 多面板、Arch/Gantt/Resources 面板、旧的 5 标签 `TabId`、alternate screen | pi 只有 transcript 加编辑器 |
| Resources、局域网探测、集群（`resources.rs`、`net_probe.rs`、`cluster.rs`）、ZS1-174/175/185 | 不在 2000 并发的执行路径上 |
| 多项目与 worktree 标签（ZE-013），以及 worktree、worker、subtab 子系统（ZE-511 移位后冻结；一个进程里跑多个 agent，与"一进程一 agent"原则冲突） | 超出 pi 的范围 |
| PTY Shell 面板（ZE-014）及其已知缺陷：master fd 没有设 CLOEXEC、fork 之后调用 `putenv`、只对 leader 发 SIGHUP | 只影响已冻结的 TUI 面板 |
| `/persona`、`/mailbox`、`/recovery`、`/session queue\|gc\|migrate`；`session.rs` 中的 mailbox；TOML 技能清单与 hooks；首次启动的登录引导（PA10） | 外观或维护类管道，与核心无关 |
| `vendor/crossterm` | **必须保留**：它修复了上游 #1126 和 #1142，等上游修复后再移除 |
| 不单独拆分 `tui.rs` | 由删除类条目逐步缩小；多个条目都要改它时，按 §4 串行合并 |
| B 块中超出核心体验对齐的部分：远程按标签页调并发、worker 任务分页和取消、worktree 移动和改名、资源 reload、加密离线缓存 v1–v4、7 页一致性复核、API 31/34 矩阵、schema 协商 | 已经实现，但不属于竖屏下的核心体验对齐；保留现状，不再扩展 |

### 7.2 不做 / 以后再做

| 项目 | 判定 | 理由 |
|---|---|---|
| question/ask-user、todowrite、webfetch、plan_exit、LSP 工具 | 不做 | pi 没有这些；模型可以直接用文字提问，而 question 工具会阻塞 headless worker |
| 权限规则服务、子代理权限派生、shell arity 审计、Agent 目录、plan/build 提醒、按 session 的运行态控制（ZS1-188/189/193..196） | 不做 | 现有审批机制已足够；一个进程跑一个 agent，天然隔离 |
| 细粒度的 LLM start/end 事件（ZS1-191）、通用认证方法与其他 OAuth（ZS1-197）、OpenSpec（ZS1-183）、动态模型目录 | 不做 | 遇到具体 bug 时再修，不搭框架 |
| revert/undo、ACP 服务端、外部 agent 配置迁移 | 不做 | pi 没有，fork 和 tree 已经覆盖 |
| tokio/async 重写、进程内多路复用、反应器框架 | 不做 | 用阻塞 channel 已经能满足"睡到下一个事件"的要求 |
| 账户服务 broker、浏览器面板、GUI 边界（旧 V2-209/401..403/501..510） | 不做 | 单用户 CLI 不需要多租户服务；GUI 方向已由 B 块替代 |
| `-p` 打印模式、pi 完整 RPC（`get_messages`、`export_html` 等） | 不做 | 单次运行由 ZE-242 的 Validator 保证（`printf … \| zenpi --mode headless --auto`），`-p` 只是一层包装 |
| 全局清理 unwrap、pedantic lint、按文件限制行数、`pub` 可见性审计、类型化全部错误、MSRV 矩阵 | 不做 | codex-rust-deslop 没有这些要求；G-RUST 只管新增代码 |
| 内嵌 tsnet/libtailscale、自己实现打洞/STUN/QUIC、Cloudflare Tunnel、ngrok、Tailscale whois 鉴权、内置语音 | 不做 | 会引入 Go 运行时，或带来数月工期、第三方账号绑定；whois 不如 zenpi 自己的设备密钥可靠；语音输入交给系统输入法 |
| 薄传输层（TLS 里直接跑 headless JSONL，主机侧可少 6–7k 行，但要重写 Android 传输层） | 以后再做 | 当 v1 协议成为维护负担时再议 |
| T2 盲中继（按 host 密钥寻址、只转发密文，约 900 行，不加依赖）、T3 iroh sidecar（独立二进制）、FCM 推送、mDNS/Wi‑Fi Direct | 以后再做 | 等用户明确拒绝使用 VPN、或需要真正的推送时再做 |
| 流中途失败后重试、tree 导航的分支摘要、多处编辑 `edits[]`、read 读图片、`--thinking`/`--system-prompt` 参数、跨 session 的输入历史、`!!` shell、离线加密缓存之外的后台同步 | 以后再做 | 有价值，但不满足 §2 的任何一条标准 |

## 8. 旧编号去向

| 旧编号 | 去向 |
|---|---|
| ZP-xxx、ZS1 `[x]` | A0 基线 ZE-001..020 |
| ZP-013 | 计入 ZE-013 |
| ZS1-180/181/182/184 | ZE-107 |
| ZS1-190 | ZE-220、ZE-221 |
| ZS1-192 | ZE-230 |
| ZS1-174/175/185 | 冻结 |
| ZS1-183/188/189/191/193..197 | 不做 |
| V2-001/002 | ZE-018、ZE-101、ZE-102 |
| V2-003/004/103/111 | ZE-004/012 + ZE-241、ZE-250 |
| V2-005/205 | ZE-011 + ZE-253 |
| V2-006、V2-201..204 | ZE-016（冻结） |
| V2-007/110/206/207 | ZE-012/013/015 |
| V2-101/107 | ZE-011 |
| V2-102/109 | ZE-241、ZE-300、ZE-301 |
| V2-104/106 | ZE-107、ZE-251 |
| V2-105/108 | ZE-231、ZE-232、ZE-236 |
| V2-112..114/119 | ZE-107、ZE-214、ZE-260、ZE-308 |
| V2-115 | ZE-303 |
| V2-116..118 | ZE-002/010 |
| V2-208 | ZE-017 |
| V2-210 | ZE-014 |
| V2-301 | ZE-240..243 |
| V2-302/303 | ZE-403 |
| V2-304 | ZE-200、ZE-231、ZE-303 |
| V2-305 | ZE-400..402 |
| V2-306 | ZE-406 |
| V2-209/401..403/501..510 | 不做 |
| S2-001/002/099 | ZE-100..102、ZE-599 |
| S2-003/010..013/019..021/030..033/037 | ZE-502，以及 ZE-510..517 的收敛 |
| S2-014 | ZE-512 |
| S2-015..018 | ZE-511（冻结） |
| S2-034..036 | ZE-518..534 |
| S2-038 | ZE-544 |
| S2-050..056 | ZE-500、ZE-590..592（其余冻结或不做） |
| S2-070..073 | ZE-593 |
| 3.x 的 ZE-103 | 并入 ZE-108、ZE-520 |
| 3.x 的 ZE-310 | 由 ZE-520 取代 |
| 3.x 的 ZE-405 | 由 B 块取代 |
| 3.0 的 ZE-208 | 撤销：经核实，生产路径不受影响 |
| 3.0 的 ZE-209 | 冻结 |

## 9. 依据索引（learn 语料与调研）

| 来源 | 落到的条目 |
|---|---|
| weiyangzen/pi-mono learn（`Docs/learn/stage1_pi_mono/files/`、`Docs/learn/files/pi-mono/`） | ZE-201、207、213、221、227、232、236、242；§7.2 中的 RPC 与 `-p` |
| weiyangzen/codex-rust-deslop（`Docs/learn/files/codex-rust-deslop/`、`Docs/research/codex-rust-deslop.md`） | §2 原则 1、G-RUST、ZE-108、ZE-302、ZE-305、ZE-512 |
| weiyangzen/b3ehive（`Docs/learn/files/b3ehive/`） | 全文格式、§4、ZE-109、ZE-260、ZE-308 |
| zenpi 自审（`Docs/learn/stage1_pi_mono/targets/zenpi/`） | ZE-108、ZE-242、ZE-300、`vendor/crossterm` |
| codex TUI 参考（`Docs/learn/stage1_pi_mono/references/codex/`） | ZE-251、ZE-255、ZE-309、ZE-402 |
| redis ae/networking/aof（`Docs/learn/subsets/redis-account-broker/`） | §2 原则 2、§2.1、ZE-300..304、ZE-308、ZE-403、ZE-541 |
| opencode runtime、agent_comms gap digest | ZE-220、230、232、260、308；其余见 §7.2 |
| Stage 2 设计（`Docs/researches/android2unix*.md`、`Docs/stage2_android2unix_*.md`） | §3、B 块全部 |
| Tailscale（how-tailscale-works、how-nat-traversal-works、key management、DERP、tsnet、Headscale） | §3.1 中的身份、控制面、连通性和拨号决策；ZE-515、ZE-516；§7.2 的网络取舍 |
| iroh 1.x 实测（新增约 137 个 crate、需要 tokio、体积约 +45%）以及 webrtc-rs、boringtun、libp2p 评估 | §3.1 的连通性分层、§7.2 |
| 移动端 agent 交互（Happy、Claude Code Remote Control、Codex 移动版、Omnara、Copilot、VibeTunnel、Blink、Termux） | §3.2、ZE-530..535、ZE-544、ZE-591 |

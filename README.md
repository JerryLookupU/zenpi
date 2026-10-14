# zenpi

zenpi is a small Rust agent runtime derived from the useful boundaries in the
official `pi-agent` TypeScript project. It has exactly two public modes:

- **TUI** for an interactive terminal conversation.
- **headless** for strict LF-delimited JSONL over stdin/stdout.

Both modes use the same core, session log, backend interface, and compact
b3ehive handoff records. There is no third `print`, `json`, `rpc`, server, or
daemon mode.

## English

### Why zenpi

zenpi keeps the agent useful when it is alone and cheap when it is composed
with other agents. A headless process can be started with one pipe, persist a
recoverable session, and exchange a bounded handoff without a broker. The TUI
uses a coalesced render loop and terminal-buffer diffing so resize and rapid
local updates do not replay the whole transcript unnecessarily. Provider calls
use the configured OpenAI-compatible endpoint.

### Quick start

```bash
# Install the `zenpi` binary (or use `cargo run --` while developing).
cargo install --path . --locked
zenpi --help

# Pair with the provider already configured for Codex.
zenpi config import-codex --profile codex
zenpi config doctor

# Interactive terminal mode, using the imported provider.
zenpi --mode tui --profile codex

# Scriptable JSONL mode.
printf '%s\n' '{"type":"prompt","id":"1","text":"Say hello"}' \
  | zenpi --mode headless --session ./session.jsonl
```

Named profiles can be inspected and switched without exposing credentials:

```bash
zenpi config list
zenpi config use codex
zenpi config doctor --profile codex --json
```

The production default is the configured OpenAI-compatible provider. zenpi
never fabricates an answer when credentials are missing: it exits before
creating a session and tells you to run `zenpi config import-codex` or set
`ZENPI_BASE_URL`, `ZENPI_API_KEY`, and `ZENPI_MODEL`. The profile is stored in
`~/.zenpi/config.toml`; on Unix the imported key is stored in the owner-only
`~/.zenpi/auth.json` (platform-specific ACL enforcement remains separate).
`config doctor` reports the effective endpoint host, API family, model, and
credential presence without printing the key. A read-only
fallback can use `~/.codex` on the first run; the explicit import persists it.
Provider quota/rate/usage-limit failures are returned as errors; zenpi never
turns them into a mock answer.

In headless mode, wait for an emitted `approval_request` event before sending
an `approve` request. Workspace write/edit approvals include a bounded unified
diff. zenpi revalidates that preview immediately before execution and returns
a `stale_preview` tool result instead of writing if the source changed while
approval was pending. Plain stdin EOF drains an accepted model turn; when EOF
leaves nobody able to answer a side-effect approval, that approval is denied
rather than hanging or silently executing.

Session startup reads have a hard 256 MiB journal-byte cap: an oversized
existing journal is rejected before parsing or permission changes. Each
backend keeps one pooled HTTP agent (keep-alive), and cancellation reaches
DNS, connect, TLS, response headers and body reads within one 250 ms poll
slice.

The Responses API is the primary wire protocol (`/responses` and
`/v1/responses`) and is consumed as SSE, including text deltas, completion
usage, and Codex gateways that insert NUL padding. Chat Completions remains an
explicit compatibility adapter. `--backend echo` exists only in builds made
with `--features dev-fixtures`; normal release binaries cannot use the mock.

**Status.** The single authority is
[`Docs/Zenpi_Execution_Blueprint_v2.md`](Docs/Zenpi_Execution_Blueprint_v2.md);
only its `[x]` items are claimed, and its §7.1 lists the frozen modules
(BentoBox panes, multi-project/worktree tabs, the PTY shell pane, b3ehive
orchestration domains) that are kept as-is. The quality gate is G-RUST (fmt,
clippy `-D warnings` for default and `--all-features`) plus the task suites
(`tests/tasks.rs`, `tests/tasks_headless.rs`, `tests/tui_task.rs`).
`python3 tools/load_agents.py --agents 2000` runs 2000 concurrent headless
agents against a scripted provider; the measured report is
`Docs/quality/load/g-load-2000.json` (idle RSS p99 5.8 MiB, all-waiting CPU
0.22 cores, per-round overhead p99 50 ms on one 20-core Mac). Android pairing
(`zenpi host pair|serve`) needs a build with `--features remote`; the default
binary leaves the remote host out. Installation requires Rust 1.88 or newer
and a local clone. Superseded v1, Stage 1 and Stage 2 records live in
`Docs/archive/`.

Session and extension lifecycle commands are available outside either runtime
mode:

```bash
zenpi session list --json
zenpi session inspect ~/.zenpi/sessions/example.jsonl --json
zenpi session fork SOURCE DESTINATION
zenpi session gc --retain-newest 20 --older-than-seconds 2592000 --yes

zenpi extension install ./my-extension
zenpi extension list --json
zenpi extension disable my-extension
zenpi extension upgrade ./my-extension-v2
```

Headless prompts can attach bounded workspace images/files without putting
binary bytes in the journal:

```json
{"schema_version":2,"type":"prompt","id":"p-2","text":"Review these","attachments":[{"kind":"image","mime_type":"image/png","path":"screenshots/ui.png"},{"kind":"file","mime_type":"text/plain","path":"logs/result.txt"}]}
```

### Headless protocol

Input and output are one JSON object per LF-terminated line. A payload may
contain U+2028 or U+2029; only LF frames a record. Diagnostics go to stderr so
stdout remains machine-readable.

Supported commands are `prompt`, typed slash `command`, `steer`, `cancel`, `approve`, `status`,
`resources`, `handoff`, `resume`, and `shutdown`. The `command` request carries a slash
command such as `{"type":"command","id":"c1","text":"/status"}` and never
enters the model turn. In the production owned/async headless
path, accepted prompts emit typed v2 progress events and one v1-compatible
terminal response. The borrowed synchronous embedding API is intentionally
smaller and does not expose in-flight cancellation. These are the current
transport guarantees, not proof that every v2 UX row is accepted. A v2 `resume` can request
`from_sequence`; duplicate in-process request IDs for ordinary terminal
operations receive the cached terminal result while that bounded cache entry
is retained (evicted or session-reset IDs may be retried), rather than repeating
provider or tool work. A replay request
re-emits its requested event suffix and is not
treated as a response-only cache hit.
Path-bearing JSONL `resume` requests, like `/session open PATH`, only switch to
an existing regular journal; a missing or symbolic-link target is rejected
without creating or modifying a file. Existing journal startup reads are
limited to 256 MiB before parsing or permission changes; valid journals within
that bound are loaded eagerly.
When a `steer` arrives before `shutdown` but is still waiting for turn
admission, the owned host gives that deferred request a bounded promotion
window; if the window expires it returns an explicit `runtime_closed` error
instead of silently dropping the request.

The common local session routes are executable in both hosts: `/session list`
lists configured journals (including the legacy `~/.zenpi/session.jsonl`),
`/session open PATH` switches an idle agent to an existing journal, and
`/session fork SOURCE DESTINATION`, `/session export SOURCE DESTINATION`, and
`/session import SOURCE DESTINATION` use explicit clean-source and
non-overwriting-destination validation. `/session gc --retain-newest N
--older-than-seconds N --yes` requires a bounded explicit retention policy,
never removes the active journal, skips domain/unowned data, and returns a
bounded receipt. The production TUI in-workspace session browser supports
bounded keyboard and mouse selection plus opening an existing session through
its owner adapter. Learn evidence/checkpoint commands
are likewise local and read-only
with respect to execution: `/learn evidence ID REPOSITORY-RELATIVE-REF`
stores a bounded hash receipt, while `/learn resume ID` reports a validated
checkpoint with `zenpi_started: false` until an external owner is present.

The first local Blueprint execution owner is also available in both hosts:
after storing a Blueprint and linked Goal, `/blueprint run ID[@VERSION]` records
one dependency-ready control-plane step, persists a bounded private receipt,
enforces the Goal budget, and can resume an interrupted receipt without a
duplicate attempt. Its response is explicitly `status: recorded` with
`execution_scope: control_plane_only`.
This is deterministic local evidence execution only; it does not run shell
commands, contact the provider, or start a b3ehive scheduler. Its `succeeded`
receipt therefore proves the local admission/recovery machinery, **not** that
the Blueprint item's requested code change or acceptance test was actually
performed. Real Blueprint work execution remains a v2 gap.

An external worker result can be imported with `/blueprint import ID[@VERSION] PATH`.
The bounded manifest must match the selected Blueprint digest and receipt identity,
report successful acceptance, and include acceptance evidence. zenpi hashes and
records the manifest; it never executes commands from it.

Example:

```json
{"type":"prompt","id":"p-1","text":"Summarize the task"}
{"type":"handoff","id":"h-1","to":"worker-b","summary":"Task is ready","artifacts":["Docs/plan.md"]}
{"type":"shutdown","id":"s-1"}
```

### Development

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets
```

Each Blueprint checklist item declares an `Estimated LOC` forecast for the
implementation/test code attributable to that item, and every value is
strictly below 5,000. This is a **per-item** forecast: it does not mean 5,000
Blueprint items and it is not a 5,000-line cap on the repository. The
aggregate Rust source inventory is reported for visibility, not used as a
project-wide cap; generated files, vendored dependencies, documentation, and
build output are not item estimates.

The authoritative checklist is
[`Docs/Zenpi_Execution_Blueprint_v2.md`](Docs/Zenpi_Execution_Blueprint_v2.md)
with its Gantt projection
[`Docs/Zenpi_Execution_Blueprint_v2_Gantt.md`](Docs/Zenpi_Execution_Blueprint_v2_Gantt.md);
the workflow policy is [`Docs/Zenpi_Execution_Spec.md`](Docs/Zenpi_Execution_Spec.md).

## 中文

zenpi 是一个轻量 Rust agent runtime，借鉴官方 `pi-agent` TypeScript 项目
中清晰的边界，但只保留两种公开模式：**TUI** 交互终端界面，以及通过
stdin/stdout 传输严格 LF-JSONL 的 **headless**。不存在第三种 `print`、
`json`、`rpc`、server 或 daemon 模式。

两种模式共享同一个 core、session 日志、backend 接口和精简的 b3ehive
handoff 记录。headless 可以通过一条管道启动、持久化可恢复会话，并在
agent 之间传递有边界的 handoff；TUI 使用合并渲染和终端缓冲区差分，减少
窗口调整及快速流式更新时的重复绘制。
生产 owned/async 路径把 provider 工作放在后台，TUI/headless 在流式响应期间
可继续处理输入和协作式取消。每个 backend 复用一个带连接池的 HTTP agent；
取消在 250 ms 的轮询间隔内覆盖 DNS、connect、TLS、响应头和 body 读取。

生产默认 backend 是配置的 OpenAI-compatible provider，不再静默使用
`echo`。首次使用先执行 `zenpi config import-codex --profile codex`，它从
`~/.codex/config.toml` 读取 URL、Responses API 和模型，从
`~/.codex/auth.json` 读取 key，并将非 secret 配置写入
`~/.zenpi/config.toml`、key 写入权限为 0600 的 `~/.zenpi/auth.json`。
随后执行 `zenpi config doctor`，再运行 `zenpi --mode tui --profile codex` 或不带
`--backend` 的 headless。也可用 `ZENPI_BASE_URL`、`ZENPI_API_KEY`、
`ZENPI_MODEL` 覆盖配置。`echo` 只存在于启用 `dev-fixtures` feature 的测试
构建；正常 release 无法启用它。缺少 provider 时 zenpi 会在创建 session
前失败，不会伪造回复。
provider 的额度、限流或 usage-limit 错误会原样作为失败返回，不会退回 mock。
headless 客户端必须等收到 `approval_request` 后再发送 `approve`。文件写入和编辑
审批会携带有界 unified diff；真正执行前再次校验文件状态，审批等待期间若文件已
变化，则返回 `stale_preview`，不会写入。stdin 正常 EOF 会等待已接收的模型请求
完成；若 EOF 后已无人能回答副作用审批，则拒绝该审批而不是挂住或暗中执行。
会话启动读取有 256 MiB 的 journal 字节硬上限：超限的已有 journal 会在解析或
修改权限前拒绝；不超过上限的有效 journal 仍会整体载入内存。

唯一权威是 `Docs/Zenpi_Execution_Blueprint_v2.md`：只有其中 `[x]` 的条目算已完成，
§7.1 列出保持现状的冻结模块（BentoBox 多面板、多项目/worktree 标签、PTY shell 面板、
b3ehive 编排域）。质量门是 G-RUST 加任务测试；`tools/load_agents.py --agents 2000`
的实测报告在 `Docs/quality/load/g-load-2000.json`。Android 配对需要 `--features remote`
构建。v1、Stage 1、Stage 2 的旧记录已移到 `Docs/archive/`。
会话可用 `zenpi session list|inspect|fork|export|import|gc` 管理；扩展可用
`zenpi extension install|list|disable|enable|upgrade|remove` 管理。headless v2
的 `prompt.attachments` 可引用工作区内的图片或文件，二进制内容不会写入日志。
两种运行模式都可执行 `/session list`、`/session open PATH`，以及带明确源和目标的
`/session fork SOURCE DESTINATION`、`/session export SOURCE DESTINATION`、
`/session import SOURCE DESTINATION`；切换只针对已有会话文件，维护操作拒绝符号链接、
脏 journal 和已存在目标，旧版 `~/.zenpi/session.jsonl` 也会被列出。`/session gc
--retain-newest N --older-than-seconds N --yes` 要求明确且有界的保留策略，不删除活动
session，跳过 domain/unowned 数据，并返回有界收据。工作区内 session 浏览器以及真正的外部执行仍会在 owner adapter 完成前明确返回错误，
不伪造成功。`/learn evidence ID REPOSITORY-RELATIVE-REF` 只保存有界 hash receipt，
`/learn resume ID` 只检查持久 checkpoint，不启动 worker。已保存 Blueprint 并关联 Goal 后，
`/blueprint run ID[@VERSION]` 在 TUI/headless 中执行一个依赖就绪的本地证据步骤，持久化
有界 receipt 并遵守 Goal budget；响应明确标记 `status: recorded`、
`execution_scope: control_plane_only`；它不调用 provider、shell 或 b3ehive scheduler。
因此 receipt 中的 `succeeded` 目前只证明本地准入、依赖和恢复机制成功，不代表该 item
要求的代码修改或验收命令真的执行完成；真实 Blueprint 工作执行仍是 v2 缺口。
外部 worker 完成 item 后，可用 `/blueprint import ID[@VERSION] PATH` 导入有界结果 manifest；
manifest 必须匹配目标 Blueprint digest 和 receipt 身份，声明验收成功并包含验收证据。
zenpi 只计算并保存 manifest checksum，不会执行 manifest 中的命令。
JSONL 的带路径 `resume` 也只允许切换已有的普通 journal；缺失或符号链接目标
会在不创建、不修改文件的情况下返回错误；已有 journal 超过 256 MiB 也会在
解析或修改权限前拒绝，不超过上限的有效 journal 会整体载入内存。
如果 `steer` 已在 `shutdown` 前被接收但仍等待 turn admission，owned 路径会在有界窗口内先完成取消/重发；超时则明确返回 `runtime_closed`，不会静默丢弃请求。
每个 Blueprint item 都为其实现/测试代码声明严格小于 5000 的 `Estimated LOC` 预估值；这里是每个 item 的
预估，不是 5000 个 Blueprint item，也不是仓库 Rust 总行数上限。仓库 Rust 总行数只作信息性盘点。
安装态 smoke 的 provider/tool 断言使用本地 OpenAI-compatible loopback fixture，证明的是生产二进制 wiring，不是线上 OpenAI/Codex 服务可用性。
在会话中还可用 `/help`、`/model`、`/models` 和 `/doctor` 查看命令、模型配置
与脱敏诊断；这些命令不会把 secret 写入输出。

## 日本語

zenpi は公式 `pi-agent` TypeScript 実装の境界設計を参考にした軽量な Rust
agent runtime です。公開モードは **TUI**（対話型ターミナル）と
**headless**（stdin/stdout の厳密な LF-JSONL）の二つだけです。`print`、
`json`、`rpc`、server、daemon という第三のモードは作りません。

両モードは同じ core、session ログ、backend インターフェース、軽量な
b3ehive handoff レコードを共有します。headless は一本のパイプで起動でき、
復元可能な session を保存し、他の agent と限定された handoff を交換できます。
TUI はフレームをまとめ、端末バッファ差分を使うため、リサイズや高速な応答
更新でも不要な全画面再描画を避けます。
本番の既定 backend は設定済みの OpenAI-compatible provider です。暗黙の
`echo` mock は使いません。最短手順は `zenpi config import-codex --profile codex`、
`zenpi config doctor`、`zenpi --mode tui --profile codex` です。最初のコマンドは
`~/.codex/config.toml` の URL、Responses API、モデルと
`~/.codex/auth.json` の key を読み、`~/.zenpi/config.toml` と権限 0600 の
`~/.zenpi/auth.json` に安全に保存します。環境変数
`ZENPI_BASE_URL`、`ZENPI_API_KEY`、`ZENPI_MODEL` も使用できます。テスト用の
`echo` は `dev-fixtures` feature のテスト build だけで有効です。通常の release
では使用できず、provider がない場合は session 作成前に失敗します。
provider の quota/rate/usage-limit エラーも mock 応答に置き換えません。
各 backend は接続プール付きの HTTP agent を 1 つ共有し、取消は 250 ms 間隔で
DNS、connect、TLS、レスポンスヘッダー、body 読み取りのすべてに届きます。

唯一の権威は `Docs/Zenpi_Execution_Blueprint_v2.md` で、`[x]` の項目だけを完了とします。
§7.1 は現状維持の凍結モジュール（BentoBox の複数ペイン、複数プロジェクト/worktree タブ、
PTY shell ペイン、b3ehive の編成ドメイン）を列挙します。品質ゲートは G-RUST とタスク
テストで、`tools/load_agents.py --agents 2000` の実測は `Docs/quality/load/g-load-2000.json`
にあります。Android のペアリングには `--features remote` ビルドが必要です。v1、Stage 1、
Stage 2 の旧記録は `Docs/archive/` に移しました。
Session は `zenpi session list|inspect|fork|export|import|gc`、extension は
`zenpi extension install|list|disable|enable|upgrade|remove` で管理できます。
headless v2 の `prompt.attachments` は workspace 内の画像・ファイルを参照し、
binary data を journal に保存しません。
両ホストで `/session list`、`/session open PATH`、および明示的な source/destination
を取る `/session fork SOURCE DESTINATION`、`/session export SOURCE DESTINATION`、
`/session import SOURCE DESTINATION` を実行できます。保守操作は既存の正常な journal
だけを source に取り、symlink、壊れた journal、既存 destination を拒否します。旧版
`~/.zenpi/session.jsonl` も一覧に含まれます。`/session gc
--retain-newest N --older-than-seconds N --yes` は明示的かつ有界な保持方針を要求し、
active journal を削除せず domain/unowned data を除外して有界 receipt を返します。production TUI の
workspace session browser は bounded selection と既存 session open をサポートします。外部実行は
owner adapter が受理されるまで明示的にエラーです。`/learn evidence ID
REPOSITORY-RELATIVE-REF` は有界 hash receipt だけを保存し、`/learn resume ID` は
検証済み checkpoint を表示するだけで worker を起動しません。
パス付き JSONL `resume` も既存の通常 journal だけを開き、欠落またはシンボリック
リンクの対象はファイルを作成・変更せずエラーにします。既存 journal の起動読み込み
には 256 MiB のバイト上限があり、超過時は解析または権限変更の前に拒否します。
上限内の有効 journal は全体をメモリに読み込みます。
外部 worker が item を完了した場合は `/blueprint import ID[@VERSION] PATH` で結果 manifest を
取り込めます。manifest は対象 Blueprint digest と receipt identity に一致し、成功した受入れと
受入れ証拠を含む必要があります。zenpi は checksum を保存するだけで、manifest のコマンドは実行しません。
`shutdown` 前に受理された `steer` が turn admission 待ちの場合、owned 経路は
限定時間内に cancel/reissue を試み、期限後は `runtime_closed` を明示して破棄を隠しません。
各 Blueprint item には実装・テストコードの `Estimated LOC` 予測（各 item が 5000 未満）を記載します。
これは 5000 個の item という意味でも、リポジトリ全体の Rust 行数上限でもありません。
インストール smoke の provider/tool 検証はローカルの OpenAI-compatible loopback fixture を使うため、
オンラインの OpenAI/Codex サービス可用性を意味しません。

## Stage 1 (pi-mono gap execution) record

Stage 1 is closed and superseded by the v2 blueprint. Its checklist is archived at
[`Docs/archive/stage1/stage_1_v3_pi_mono_blueprint.md`](Docs/archive/stage1/stage_1_v3_pi_mono_blueprint.md)
and its integration record at
[`Docs/quality/stage1/acceptance.md`](Docs/quality/stage1/acceptance.md).

Stage 1 已关闭并由 v2 蓝图取代；清单归档在
`Docs/archive/stage1/stage_1_v3_pi_mono_blueprint.md`，集成验收记录在
`Docs/quality/stage1/acceptance.md`。

## License

MIT. See [LICENSE](LICENSE).

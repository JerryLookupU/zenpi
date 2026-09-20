# Zenpi Android 连接工作机的最简方案提案

> 状态：方案提案，不代表功能已经实现。
> 研究基础：[`android2unix.md`](./android2unix.md)。
> 目标：用最小的工程和运维成本，让 Android 扫描 QR 后连接 macOS/Linux 工作机，并远程控制工作机上的 Zenpi TUI/headless 运行。

## 1. 决策结论

第一版只做：

```text
同一局域网
+ 工作机临时启动 listener
+ QR 传递地址、证书指纹、一次性 token
+ TLS 加密直连
+ 有界 JSON length framing
+ session/event replay
+ Android Compose 双层 tab
```

建议暂时不要做：

```text
ICE/STUN/TURN
公网自动发现
默认 relay
FCM 推送保活
Android 永久在线 socket
QUIC path migration
多设备复杂权限系统
```

最终产品承诺应写成：

> 在 Android 与工作机处于同一局域网，或工作机已经具备用户配置的可达地址时，Android 可以通过 QR 配对直连。工作机负责持续运行和保存 session；Android 负责控制、显示和恢复连接。

不能承诺“任意公网 NAT 下、完全不依赖任何服务器的稳定直连”。QR 只能交换地址和配对信息，不能替代 NAT 入站映射或中继。公网穿透边界见 [RFC 8445 ICE](https://www.rfc-editor.org/rfc/rfc8445.html)。

## 2. 为什么这是最优先的最小切片

| 选择 | 决策 |
| --- | --- |
| 网络范围 | 先限定 LAN；公网直连仅作为用户自行配置的地址/端口转发场景 |
| 传输 | 第一版使用 TLS/TCP；协议层保持独立，未来可替换 QUIC |
| 配对 | 工作机生成短期 QR，Android 扫描并确认指纹/短校验码 |
| 执行位置 | Agent、provider、工具、git、PTY、TUI/headless 全部在工作机 |
| Android 职责 | UI、输入、审批、事件显示、断线恢复 |
| session 事实源 | 工作机 journal、context checkpoint、event replay |
| 后台策略 | 不保证 Android socket 常驻；前台恢复后 resume/replay |
| 公网扩展 | 之后再加可选自托管 rendezvous/STUN/TURN，不作为 MVP 依赖 |

这样可以先验证真正的产品价值：Android 双层 workspace/worktree 控制、TUI/headless 远程操作、审批和 session 恢复，而不是先陷入 NAT 穿透、relay 运维和 Android 后台限制。

## 3. 最小用户流程

### 3.1 工作机端

用户在 macOS/Linux 工作机主动执行：

```bash
zenpi host pair
```

工作机执行以下动作：

1. 启动一个只绑定局域网地址的临时 listener。
2. 生成一次性随机 token，有效期建议 10 分钟。
3. 为 host identity 生成或读取持久证书。
4. 显示 QR、host 地址、端口、证书 SHA-256 指纹和 6 位短校验码。
5. 等待 Android 完成一次配对。
6. 配对完成后让 token 立即失效，并把设备公钥/设备标识保存到工作机。

如果用户没有显式执行 `zenpi host pair`，工作机不监听网络端口。

### 3.2 Android 端

Android 流程：

1. 点击“连接工作机”。
2. 扫描 QR。
3. 显示 host 名称、地址和证书指纹。
4. 让用户确认工作机显示的 6 位短校验码。
5. 通过 TLS 建立连接。
6. 发送 `hello` 和最近的事件游标。
7. 获取 workspace、worktree、session、capabilities 和当前状态。
8. 进入一级 workspace tab、二级 worktree tab 界面。

Android 不应让用户输入 provider API key，也不应把工作机的完整 session/context 放进 QR。

## 4. QR 与配对协议

QR 内容建议采用版本化 URI：

```text
zenpi://pair?v=1&host=192.168.1.42&port=43821&fingerprint=sha256%3AABCD...&token=...&expires=2026-09-20T12%3A00%3A00Z
```

字段含义：

| 字段 | 作用 |
| --- | --- |
| `v` | 配对格式版本 |
| `host`/`port` | 工作机的局域网地址和临时端口 |
| `fingerprint` | 工作机证书或公钥指纹 |
| `token` | 一次性短期配对凭证 |
| `expires` | 防止旧 QR 被长期复用 |

配对最小安全模型：

1. QR 中的 token 随机生成、短期有效、只能使用一次。
2. Android 必须校验证书/公钥指纹，禁止因为自签名证书而关闭 TLS 校验。
3. 双端显示相同的 6 位短校验码，用户确认后才完成配对。
4. 配对成功后 token 立即作废。
5. 工作机保存设备标识、设备公钥、权限和撤销状态。
6. 后续版本使用 Android Keystore 生成不可导出的设备私钥，并升级到 mTLS；MVP 可先使用 pinned host key + 一次性 token。

这套方案的安全假设是：攻击者不能同时控制用户正在查看的工作机屏幕和 Android。若设备丢失，工作机端必须提供撤销设备功能。

## 5. 传输协议

### 5.1 第一版使用 TLS/TCP

第一版不直接引入 QUIC、Iroh 或 ICE。原因是 TLS/TCP 在 Android 和 Rust 侧更容易做出可验证的最小实现，而且局域网场景不需要 NAT 穿透。

协议层不能绑定 stdin/stdout，而应抽象成：

```text
read_frame() -> bounded JSON object
write_frame(object)
```

线上 framing：

```text
u32 big-endian byte length
UTF-8 JSON payload
```

继续沿用 Zenpi 现有协议中的有界字段和 envelope：

```json
{
  "schema_version": 3,
  "id": "request-42",
  "session_id": "session-1",
  "type": "prompt",
  "text": "检查当前变更"
}
```

必须限制：

- 单帧大小；
- prompt、附件、PTY 输出大小；
- 未完成 request 数量；
- event replay 窗口；
- worktree 数量和并发范围。

### 5.2 逻辑 stream

即使第一版底层只有一个 TLS/TCP 连接，也要在业务上区分：

```text
control    命令、ack、错误、配对状态
events     增量事件、session 状态、workspace 状态
bulk       PTY 输出、diff、附件、较大的 checkpoint
```

未来切换 QUIC 时，可以自然映射到多个 QUIC stream，而不改变 Android 业务协议。

### 5.3 握手

建议握手顺序：

```json
{"type":"hello","schema_version":3,"device_id":"d1","host_id":"h1"}
{"type":"capabilities","projects":[],"max_frame_bytes":1048576,"event_head":900}
{"type":"resume","session_id":"s1","last_event_sequence":812}
{"type":"event","sequence":813,"session_id":"s1","event":{"type":"..."}}
```

工作机必须校验：

- device identity；
- 配对状态和设备撤销状态；
- schema version；
- project ACL；
- session identity；
- owner epoch；
- request ID 是否重复。

## 6. session 与断线恢复

### 6.1 工作机是唯一事实源

以下内容只由工作机写入：

- session journal；
- context checkpoint；
- provider/tool 执行结果；
- 审批状态；
- project/workspace；
- worktree；
- scheduler 并发状态。

Android 只缓存：

```text
host_id
device_id
project_id
session_id
subtab_id
last_event_sequence
last_transport_sequence
context_digest
display cache
pending request ids
```

Android 的显示缓存不是可提交的 context，也不能作为工作机 session 的替代品。

### 6.2 断线处理

连接断开时：

1. 工作机继续运行已经被接纳的 Agent turn。
2. Android 标记 `offline` 或 `stale`，不把未确认请求直接显示成失败。
3. Android 回到前台后重新建立连接。
4. Android 发送最近 `last_event_sequence`。
5. 工作机补发缺失事件。
6. 如果 replay 已经超出窗口，工作机返回 `replay_gap`，再发送完整的有界 snapshot + tail。
7. 对超时但结果未知的写命令，Android 先查询结果，不自动重做 provider、tool 或 git 副作用。

### 6.3 幂等性

每个写命令都带：

```text
request_id
session_id
project_id
expected_project_revision
expected_subtab_revision
```

工作机短期保存 request ID 的结果。相同 request ID 重试时返回原结果，不能重复创建 worktree、重复执行工具或重复提交审批。

## 7. 双层 tab 方案

### 7.1 一级 workspace

Android 使用 Compose Material 3 的 `PrimaryTabRow`：

```text
[workspace-a] [workspace-b] [workspace-c] [+]
```

点击 `+` 后填写：

- 工作目录或已有 project；
- 显示名称；
- 初始权限/访问模式。

所有 open/select/close 动作发送到工作机 owner，Android 不在本地伪造 workspace。

### 7.2 二级 worktree

当前 workspace 使用 `SecondaryTabRow`：

```text
[main] [review-1] [feature-x] [+]
```

`+` 菜单明确区分：

- 原地隔离 worktree；
- 新建 git worktree。

每个 tab 显示：

- 稳定 `subtab_id`；
- 名称；
- branch；
- root kind；
- busy/approval/error 状态；
- 当前并发数。

### 7.3 并发调整

每个 worktree 使用点按 stepper：

```text
[-]  4  [+]
```

范围暂定为 1..64，与当前 TUI 一致。但 Android 不在本地递增数字，而是发送：

```json
{
  "schema_version": 3,
  "id": "r43",
  "type": "subtab",
  "project_id": "p1",
  "subtab_id": "st2",
  "action": "set_concurrency",
  "value": 4,
  "expected_revision": 8
}
```

工作机返回：

- 新 revision；
- requested value；
- effective value；
- scheduler 是否接受；
- 完整 subtab entry；
- 对应 event。

## 8. 必须新增的 host API

不能继续依赖当前仅供交互式 TUI 使用的 `/worktree` slash 路径。需要新增版本化 typed API：

```text
SubtabControl {
  list
  select
  add_in_place
  add_worktree
  close
  move
  rename
  set_concurrency
}
```

建议每个二级 tab 使用稳定 `subtab_id`，而不是用数组 index 作为主键。index 只表示当前排序。

工作机执行 `close` 时必须：

1. 检查 tab 是否 busy；
2. 检查未完成审批；
3. 执行 git worktree remove；
4. 确认删除成功；
5. 原子写入 checkpoint；
6. 发出关闭 event。

任一步失败，都保留 tab 和错误状态，不应像当前 TUI 一样忽略 worktree 删除错误。

## 9. Android 生命周期策略

最小方案不把 Android 设计成永久在线客户端：

```text
foreground_connected
    ↓ app 进入后台
background_grace
    ↓ Doze/系统回收/网络丢失
offline_durable
    ↓ app 回到前台
resume_and_replay
```

具体策略：

- 前台页面打开时保持 TLS 连接并发送应用层 ping。
- 用户主动点击“保持连接”时，才评估带通知的 Foreground Service；不默认申请电池优化豁免。
- WorkManager 只用于网络恢复后的 resume、命令队列 flush 和缓存清理，不能用来维持低延迟 socket。
- 使用 `ConnectivityManager.NetworkCallback` 监听网络变化；网络切换时关闭旧连接，使用新 Network 重新拨号。
- Android 进入 Doze、进程被杀或设备断网时，工作机继续执行；Android 恢复后进行 replay。
- UI 明确显示 last seen、stale、replay gap 和未知结果。

Android 12+ 后台启动 FGS 受限，Android 15 的 `dataSync`/`mediaProcessing` FGS 还有时间上限，因此不能把 FGS 当作无限保活方案。参考 [Doze/App Standby](https://developer.android.com/training/monitoring-device-state/doze-standby)、[FGS 后台启动限制](https://developer.android.com/develop/background-work/services/fgs/restrictions-bg-start) 和 [FGS timeout](https://developer.android.com/develop/background-work/services/fgs/timeout)。

## 10. 实施顺序

### Phase A：工作机 host contract

- 抽象 `read_frame/write_frame`，脱离 stdin/stdout。
- 把 TUI workspace/subtab 操作收敛到 host-owned domain。
- 新增 stable `subtab_id`、revision/CAS、capabilities 和 snapshot。
- 修正 worktree close 失败处理。
- 先用 loopback TLS 或明文测试 transport 验证协议，不接公网。

### Phase B：LAN 配对

- 工作机 `zenpi host pair`。
- 临时 listener、QR、证书 fingerprint、一次性 token。
- Android 扫码、SAS 确认、TLS pinning。
- prompt、stream event、approval、resume/replay 集成测试。

### Phase C：Android UI

- Kotlin + Compose Material 3。
- `PrimaryTabRow` + `SecondaryTabRow`。
- workspace/worktree 增删和选择。
- 每个 worktree 并发 stepper。
- transcript、diff、approval、status、offline/replay 状态。

### Phase D：公网可选增强

- LAN NSD/mDNS 自动发现。
- IPv6/用户端口转发支持。
- Android Keystore 设备密钥和 mTLS。
- 评估 QUIC；只有测试证明 TCP/TLS 不够时再迁移。
- 用户自托管 rendezvous/STUN/TURN；明确标记为可选基础设施。

## 11. 验收标准

### 连接与安全

- 没有执行 `zenpi host pair` 时，工作机没有远程 listener。
- QR 过期或重复使用会被拒绝。
- 指纹不匹配会拒绝连接。
- 未配对设备无法访问 session。
- provider credentials 不会出现在 QR、Android 日志或网络响应中。

### session

- Android 断线时，工作机上的 Agent 不被错误终止。
- Android 恢复后能按 sequence 补齐事件。
- replay gap 能返回 snapshot + tail。
- 超时的副作用请求不会被 Android 自动重复执行。
- TUI 和 Android 连接同一个 session 后，transcript 和 context digest 一致。

### 双层 tab

- Android 可以创建、选择、关闭 workspace。
- Android 可以创建、选择、移动、重命名和关闭 in-place/worktree subtab。
- Android 可以独立调整每个 worktree 的 1..64 并发。
- worktree 删除失败时 tab 不会从状态中消失。
- TUI 与 Android 并发操作产生 revision conflict，而不是互相覆盖。

### 生命周期

- Android 进入后台或被系统杀掉后，工作机仍继续运行。
- Android 回到前台能重新连接并 replay。
- 网络切换后旧连接被关闭，新连接使用最新 Network 建立。
- UI 能区分 connected、stale、offline、replay_gap 和 unknown outcome。

## 12. 非目标

本提案第一阶段不解决：

- 任意公网 NAT 下的无服务器直连；
- Android 端运行 Rust TUI、git、shell 或 headless；
- Android 后台无限 socket 保活；
- 把 FCM、Tailscale DERP 或 Iroh 默认 relay 包装成“无服务器”；
- 将完整 context 下载到 Android 并让 Android 成为事实源。

## 参考资料

- 主调研：[android2unix.md](./android2unix.md)
- Android：[Compose Tabs](https://developer.android.com/develop/ui/compose/components/tabs)、[Doze/App Standby](https://developer.android.com/training/monitoring-device-state/doze-standby)、[WorkManager](https://developer.android.com/develop/background-work/background-tasks/persistent/getting-started/define-work)、[ConnectivityManager](https://developer.android.com/develop/connectivity/network-ops/reading-network-state)、[Android Keystore](https://developer.android.com/privacy-and-security/keystore)、[Network Security Config](https://developer.android.com/privacy-and-security/security-config)
- 网络标准：[RFC 8445 ICE](https://www.rfc-editor.org/rfc/rfc8445.html)、[RFC 9000 QUIC](https://www.rfc-editor.org/rfc/rfc9000.html)、[RFC 6887 PCP](https://www.rfc-editor.org/rfc/rfc6887.html)

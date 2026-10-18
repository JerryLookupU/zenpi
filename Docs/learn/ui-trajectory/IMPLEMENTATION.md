# Zenpi 轨迹视图实现说明

本文对应 `DESIGN_FINALIZED.md` 的落地实现。设计文档中的 blessed/TypeScript 方案已改为
Zenpi 现有的 Rust + ratatui 栈；数据模型、三层导航和快捷键保持设计意图。

## 入口

| 入口 | 行为 |
|------|------|
| `Alt-T` | 打开轨迹视图；只有主会话时直接进入记录列表，有 worker 时进入 Agent 总览 |
| `/trajectory`（TUI） | 同上 |
| `/trajectory`（headless） | 返回 JSON：记录数、轮次、用量、缓存命中率、平均 TTFT、`kv_cache` 问题列表 |

`Ctrl-T`（目录选择器）和 `Ctrl-G`（外部编辑器）已被占用，因此没有使用设计稿中的组合键。

## 分层

```
provider usage ──> backend::Usage { cache_read/cache_write/reasoning }   (protocols/*)
core 请求循环  ──> turn.metadata.request { started/first_token/completed,
                                           instructions/tools/context digest,
                                           context_turns, shared_turns }  (core.rs)
Turn 历史      ──> trajectory::Trajectory { records, turns, stats }       (trajectory.rs, 纯逻辑)
Trajectory     ──> tui::trajectory::TrajectoryViewer                      (tui/trajectory.rs, 仅绘制/导航)
```

- `src/trajectory.rs` 与 UI 无关（对应 ui-trajectory 的 snapshot builder + record formatting）。
- `src/tui/trajectory.rs` 只负责导航和绘制（对应 TrajectoryView / Table / Inspector）。

## 数据采集

1. **用量细分**：`Usage` 新增 `cache_read_tokens`、`cache_write_tokens`、`reasoning_tokens`
   （为 0 时不序列化，旧 journal 与 headless 输出保持不变）。
   - Anthropic：`cache_read_input_tokens` / `cache_creation_input_tokens`（`input_tokens` 仍为总输入）
   - OpenAI Chat / Responses：`*_tokens_details.cached_tokens`、`reasoning_tokens`
   - DeepSeek：`prompt_cache_hit_tokens`
   - Gemini：`cachedContentTokenCount`、`thoughtsTokenCount`
2. **每次请求的元数据**：工具调用步骤（`assistant-tool-call-*`）现在也持久化 `usage`，
   所有请求都会写入 `request`：开始 / 首 token / 完成时间，以及前缀指纹。
3. **前缀指纹**：instructions 摘要、工具集摘要，以及上下文 turn 的链式摘要。与同一会话的上一次请求
   比较，得到 `shared_turns`（共享的前缀 turn 数）。

## KV cache 校验规则

逐次比较相邻的 provider 请求：

| 标记 | 级别 | 条件 |
|------|------|------|
| `⚠` | Warn | instructions 摘要变化（系统提示词被重新渲染：skills / persona / resources） |
| `⚠` | Warn | 工具集摘要变化 |
| `⚠` | Warn | `shared_turns < previous_context_turns`（之前的 turn 被改写） |
| `ℹ` | Info | 同上，但两次请求之间发生了压缩（属预期） |
| `✗` | Miss | 前缀稳定、provider 有缓存计数，且上一次可复用 token ≥ 1024，但 `cache_read` 不足预期的 50% |
| `⚡n%` | Ok | 本次输入由缓存命中的比例 |

检查器的 Metrics 页显示判定、原因和建议。在记录列表中按 `c` 跳到下一个问题。

## 交互

- **L1 Agent 总览**：状态、轮次、请求数、token、缓存命中率、KV 标记、时长，以及共享时间轴上的甘特条。
- **L2 记录列表**：按 turn 分组（`Space` 折叠）、虚拟化窗口、跟随尾部、底部统计栏，
  `/` 搜索，`n`/`N` 跳转匹配项。
- **L3 检查器**：Content / Metrics / Raw 三个标签页（`Tab`），`←→` 切换记录，`y` 复制当前页。
- 通用：`Esc` 返回上一层，`1`/`2`/`3` 切换层级，`q` 关闭，`?` 帮助。

## 未实现 / 后续

- **暂停 / 继续 Agent**：运行时只有 cancel / steer，没有 pause 原语，需要先在 core 设计挂起点。
- **指纹的跨进程比较**：重启后的第一次请求没有 `shared_turns`；instructions / tools 摘要仍可跨进程比较。
- **worker 列表**：来自当前项目的 headless 进程快照（前 16 个立即加载，其余按 Enter 时加载，`R` 重新加载），
  尚未与 DAG 节点状态（green / red）关联。
- **其他**：导出、标记、独立的 cache 统计面板。

# OC-058 — packages/opencode/src/tool/truncation-dir.ts

```yaml
source_id: OC-058
item_id: OC-058
source_path: packages/opencode/src/tool/truncation-dir.ts
source_hash: 790159a022386c68212056a39571f53c9874d11079b0572d57c207f1cd1d9155
source_bytes: 148
source_lines: 4
coverage:
  bytes: L1-L4 / byte 1-148（完整读取）
  lines: L1-L4（含全部注释、类型、导出符号）
```

## 完整行为复盘

该文件没有函数、类、接口或自定义类型，只有两个导入和一个导出常量。按源文件顺序复盘如下：

- `import path from "path"`（L1）：加载 Node.js 的 `path` 默认导出，后续使用其 `join` 进行平台相关的路径拼接。这里没有读写文件，也没有创建目录；导入模块本身可能触发 Node 模块加载，但本文件没有额外初始化逻辑。
- `import { Global } from "@opencode-ai/core/global"`（L2）：读取 `Global` 命名导出。该文件只依赖其 `Path.data` 字段的值，并不修改 `Global` 或其中任何字段。`Path.data` 的隐含输入契约是一个可供 `path.join` 接受的路径字符串；其计算、默认位置和初始化责任属于 `@opencode-ai/core/global`，不能从本文件进一步确认。
- `export const TRUNCATION_DIR = path.join(Global.Path.data, "tool-output")`（L4）：在模块求值时同步计算并导出 `TRUNCATION_DIR`。输入是 `Global.Path.data` 和固定字面量 `"tool-output"`；输出是当前平台的 `path.join` 结果，通常表示数据目录下的 `tool-output` 子目录。绑定是 `const`，调用方不能重新赋值，但返回的字符串本身没有可变状态。

边界与默认值：子目录名固定为 `tool-output`，没有可配置参数、环境变量、长度上限、尾部斜杠规则或文件名清理逻辑。`path.join` 会按运行平台选择分隔符并规范化可处理的 `.`、重复分隔符等部分；它不会保证目录实际存在。若 `Global.Path.data` 不是 `path.join` 所需的字符串，Node 的路径 API 会抛出类型错误，导致模块初始化失败；本文件没有捕获或转换该错误。若路径字符串合法，拼接本身没有声明的失败返回值。

并发语义：ES module 通常只在同一模块图中求值一次，多个导入方共享同一个 `TRUNCATION_DIR` 字符串；不存在锁、异步任务、竞态或对目录内容的并发操作。不同进程各自加载模块时，会分别依据各自进程中 `Global.Path.data` 的值计算结果，因此跨进程一致性由全局配置而非本文件保证。

文件名虽含 `truncation`，但源内没有截断算法、大小预算、输出写入、清理或读取实现；它只定义一个供其他工具输出代码复用的路径常量（L4）。

## 状态、取消、恢复与副作用

本模块没有业务状态机。唯一状态是模块初始化后保留的常量值；没有取消令牌、超时、重试、恢复游标、幂等键或持久化记录。`TRUNCATION_DIR` 的求值是同步且一次性的，后续修改 `Global.Path.data`（如果外部允许修改）不会自动更新已经导出的字符串。源内没有 `fs.mkdir`、`fs.writeFile`、删除、网络调用或进程调用，因此不存在直接外部副作用，也不存在“目录不存在时自动创建”的承诺。

错误路径只有依赖初始化失败这一类：`Global` 导入失败或 `Global.Path.data` 传入不被 `path.join` 接受的值，错误会在模块加载阶段向上传播；本文件不做 fallback。目录创建、权限错误、磁盘满、输出截断、清理失败等均不在此处处理，应由消费 `TRUNCATION_DIR` 的上层模块负责。由于没有异步操作，取消和超时没有可观察语义；重试只能由调用方重新执行其文件系统操作。

## 源内测试与行为判据

源文件或同目录未包含测试，源内未包含测试。可独立验证的判据是：在与项目相同的 Node 运行环境中设置一个已知的 `Global.Path.data`，加载该模块后断言 `TRUNCATION_DIR === path.join(Global.Path.data, "tool-output")`；在 POSIX 与 Windows 风格路径上分别断言使用平台分隔符；断言加载模块不会创建该目录（加载前后用 `fs.existsSync` 比较）。还应验证传入非法的非字符串 `Global.Path.data` 时，模块求值阶段产生路径 API 类型错误，而不是返回 fallback 路径。验证时必须覆盖 L1-L4 的完整模块加载路径，而非只复制常量表达式。

## zenpi Rust 映射

最接近的现有实现是 `src/tool_output.rs`：它已经把原始工具输出放入宿主拥有的私有目录，并由 `SessionOutputStore::store`/`existing_store` 根据已打开的 session journal 派生目录；目录名形如 `.zenpi-output-<session_id 的 SHA-256>`，而不是全局 `Global.Path.data/tool-output`。`OutputLimits`、`CommandOutputCapture`、`ArtifactRef` 和 `OutputError` 还负责容量、TTL、完整性、取消、清理和恢复，这些能力明显超出本 TS 文件的单纯路径常量。

建议落点与可执行映射如下：

1. 若要表达“数据根目录下的工具输出目录”这一纯配置概念，可在 `src/tool_output.rs` 增加私有常量或纯函数 `fn tool_output_dir(data_root: &Path) -> PathBuf { data_root.join("tool-output") }`；函数只拼接路径，不创建目录，直接对应 L4。若 zenpi 需要跨平台稳定性，应使用 `PathBuf::join`，不要手工拼接 `/`。
2. 若路径必须绑定会话安全边界，继续以 `SessionOutputStore` 为唯一入口更合适；不要把模型输入或 `protocol.rs` 的 `path` 字段直接映射成目录。`src/core.rs` 已负责捕获、读取和清理调用，建议在那里决定何时打开 store、何时持久化 `tool_output_captured` 事件。
3. `src/session.rs` 负责 append-only journal、恢复和路径生命周期；它可以提供已验证的 session 路径给 `SessionOutputStore`，但不应把本常量的无状态拼接误认为恢复机制。`src/headless.rs` 与 `src/protocol.rs` 只需暴露已有的 tool-output 控制/读取命令，不需要新增“truncation-dir”协议字段。
4. `src/runtime.rs` 的 `CancellationToken`/`BackgroundRunner` 可承载真正的异步输出读取取消；这与 L4 同步求值不同。`src/approval.rs` 只处理副作用批准，不参与目录计算。`src/providers/**` 是 provider 路由和 wire codec，对该路径常量没有直接映射；provider 输出若需落盘，应经过 `tool_runtime.rs` 的 capture 决策及 `tool_output.rs`，不能由 provider 模块自行选目录。

差异清单：TS 版本是进程级、全局根目录、无 I/O、无配额、无恢复、无权限策略的单一字符串；zenpi 版本是会话级派生目录，带 SHA-256 身份、锁/安全路径检查、原始字节与展示快照分离、TTL 与配额、清理收据、取消和可恢复 journal。若追求行为等价，应新增一个纯的 `PathBuf` 拼接适配器并明确其只提供候选根路径；若追求 zenpi 当前安全语义，则应保留 `SessionOutputStore` 设计，不能直接照搬全局 `tool-output` 目录。可验证步骤：对适配器做 POSIX/Windows 路径单测；对 `SessionOutputStore` 做已有 capture、read、cleanup、reopen 测试；用 `cargo test` 检查取消和持久化路径，确认没有把目录计算当作目录创建。

## 未决问题

1. `Global.Path.data` 的实际来源、默认值、是否绝对路径以及是否保证字符串类型，无法从本文件确认，需要查看 `@opencode-ai/core/global`。
2. 哪些调用方消费 `TRUNCATION_DIR`、何时创建目录、截断阈值和清理策略，源文件没有提供，需检查同项目其他工具输出模块。
3. `path.join` 在目标运行时是否被 polyfill 或替换，以及多进程是否共享同一 data 根目录，无法仅由这 4 行确认。

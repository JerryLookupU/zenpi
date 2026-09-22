# OC-059 — packages/opencode/src/tool/webfetch.ts

## 元信息

- source_id/item_id：OC-059
- source_path：packages/opencode/src/tool/webfetch.ts
- source_hash：8d9efbad1ffdf8dc29dbac80eb5372ba66a8c80fccaa1606d4e41a9c929a5b6b
- source_bytes：6881
- source_lines：192
- coverage：字节 0–6880、行 1–192，已按顺序完整读取，含全部注释、Schema、导出符号和私有函数。

## 完整行为复盘

### 依赖与常量

L1-L7 导入 Effect/Schema、Effect HTTP 的 HttpClient/HttpClientRequest、htmlparser2 Parser、Tool、TurndownService、DESCRIPTION 和 isImageAttachment。L9-L11 定义 MAX_RESPONSE_SIZE=5*1024*1024、DEFAULT_TIMEOUT=30*1000、MAX_TIMEOUT=120*1000。前者限制完整响应内存，后二者是默认和硬上限。

### Parameters

导出 Parameters（L13-L22）是 Schema.Struct：url 为 Schema.String，没有 host/长度/完整 URL 校验；format 只能是 text、markdown、html，L17-L20 用 withDecodingDefault 把缺省值设为 markdown；timeout 是可选 Number，单位秒，最大值只写在注释中，Schema 没有下限或 max 约束。执行时仍会做 scheme、权限和资源限制。

### WebFetchTool

导出 WebFetchTool（L24-L26）用 Tool.define 注册名称 webfetch。构造时从 Effect 环境取 HttpClient，并派生 filterStatusOk 客户端（L26-L29）；返回 description、parameters、execute（L30-L34）。

execute 的完整顺序如下。

1. L35-L37：url 必须以 http:// 或 https:// 开头，否则抛出 URL must start with http:// or https://；不是完整 URL 解析。
2. L39-L48：先调用 ctx.ask，permission 为 webfetch，patterns 为当前 URL，always 为 ["*"]，metadata 记录 url、format、原始 timeout；许可未通过前不发网络请求。
3. L50-L50：timeout 为 min((params.timeout 或 30)*1000,120000)。大值截断至 120 秒，缺省 30 秒；无显式正数下限，负数/特殊数值的底层效果需外部确认。
4. L52-L68：按 format 生成 Accept。markdown 优先 text/markdown、text/x-markdown、text/plain、text/html；text 优先 text/plain、markdown、html；html 优先 text/html、application/xhtml+xml、plain、markdown，均带 q 回退。
5. L69-L76：设置固定浏览器 User-Agent、Accept-Language en-US,en;q=0.9 和 Accept，创建 GET request。
6. L78-L93：执行一次 httpOk.execute；只有错误是 403 且 cf-mitigated 响应头等于 challenge 时，才顺序重试一次，第二次 User-Agent 改成 opencode。无退避、无第三次尝试。整个首次请求和 fallback 共用 Effect.timeoutOrElse；超时分支为 Effect.die(new Error("Request timed out"))。
7. L95-L104：先比较 content-length 与 5 MB，再完整读取 arrayBuffer 并比较实际 byteLength；任一超限都报 Response too large (exceeds 5MB limit)。因此不是流式处理。
8. L106-L108：从 content-type 分号前部分生成小写 mime，标题为原始 URL 加原始 contentType。
9. L110-L123：若 isImageAttachment(mime) 为真，完整 body 转 Base64，返回 title、output 为 Image fetched successfully、metadata 为 {}，以及 type=file、mime、data URL 的单个 attachment；不写文件。
10. L126-L126：非图片使用 TextDecoder().decode(arrayBuffer)，编码细节由标准 TextDecoder 决定。
11. L128-L152：markdown 仅在 contentType 包含 text/html 时调用 convertHTMLToMarkdown，否则原文；text 的 HTML 分支调用 extractTextFromHTML，否则原文；html 和理论不可达 default 均原文。文本结果均是 output/title/metadata:{}，无 attachments。
12. L153-L154：生成器用 Effect.orDie 收尾，失败转为 die，源内没有结构化错误返回。

### extractTextFromHTML

私有函数（L158-L180）通过 htmlparser2 Parser 拼接纯文本。打开或嵌套于 script、style、noscript、iframe、object、embed 时递增 skipDepth（L162-L167）；仅深度为零的 ontext 被追加（L168-L170）；关闭标签在深度大于零时递减（L171-L173）。L176-L179 写入、结束解析并 trim。不会在块元素之间自动插入空格/换行，关闭标签也不检查名称，异常 HTML 应以 parser 回调验证。

### convertHTMLToMarkdown

私有函数（L182-L192）每次新建 TurndownService，配置 ATX heading、--- hr、- bullet、fenced code、* emphasis（L183-L189），移除 script/style/meta/link 后调用 turndown（L190-L191）。只有 markdown 且响应被识别为 HTML 才调用。

## 状态、取消、恢复与副作用

每次 execute 独立创建请求并完整缓冲响应，没有本地可变状态、缓存、文件写入、数据库或恢复记录。外部副作用是目标 URL 的 GET，Cloudflare challenge 时可能再发一个 GET；ctx.ask 也是宿主侧权限副作用（L39-L48、L76-L93）。图片 data URL 只存在于内存结果。

超时是整个请求加一次 fallback 的总上限，不是每次尝试分别计时（L78-L93）。没有显式 cancellation token、abort controller 或恢复分支；能否中断底层 HTTP 取决于 Effect/HttpClient。execute 内没有并发控制，多个调用是否并行由外部运行时决定。无通用重试，失败不记录重试次数、最终响应状态或 header。Effect.orDie 表示外层必须负责捕获和决定恢复；网络请求即使本地超时也不能假设远端回滚。

## 源内测试与行为判据

源文件及同目录未包含测试（同目录为 webfetch.ts 与 webfetch.txt）。可独立验证：用 Mock HttpClient 确认 scheme 错误和未授权时请求计数为零、format 缺省为 markdown；content-length 或实际 body 超过 5 MiB 均失败；403+cf-mitigated=challenge 恰好两次且第二次 UA=opencode，普通 403 不重试；HTML 中 script/style/iframe 文本被 text 移除，markdown 使用 ATX/列表/围栏代码，html 保持原文；图片响应生成可解码 data URL；慢 client 验证默认 30 秒、上限 120 秒及 fallback 共用总超时。

## zenpi Rust 映射

- 工具落点：在 src/tools.rs 的 Tool/ToolRegistry 或新建 src/tools/webfetch.rs。现有 ToolDefinition 需要对象 schema，ToolResult 用 Success/Error 关联 call_id（src/tools.rs:L502-L535、L648-L663），可把结果编码为 {title,output,metadata,attachments}。当前 builtin_effect 只有 ReadOnly、WorkspaceWrite、CommandExecution（src/tools.rs:L395-L403），网络读取应新增 NetworkRead 或明确 host policy，不能隐式获得任意联网能力。
- 执行并发：src/tools.rs:L1011-L1045 提供 ToolExecutionMode 与 invoke_cancellable；src/tool_runtime.rs:L157-L249 负责批量预算、来源顺序、取消和停止，L252-L346 负责并行只读调用并 join。webfetch 可声明只读并参与并发，但单调用内部必须保持首次请求后才 fallback 的顺序。
- 审批：ApprovalRequest 的工具、side_effect、arguments、preview、origin、policy/lease 字段见 src/approval.rs:L40-L56；只读自动决策见 L495-L528。要映射 ctx.ask 的 webfetch、URL pattern、always，应增加网络权限/host pattern 字段并校验 scheme/host，不能只按工具名记忆。headless ask/always/never 路由在 src/headless.rs:L7417-L7439。
- 网络与输出：实现 5 MiB Content-Length 预检和最多 5 MiB+1 的实际读取；复用 ToolError 的 LimitExceeded/Cancelled 等（src/tools.rs:L671-L717），必要时补 Network/Timeout。ToolContext::read_attachment 的先解析再限长读取模式见 src/tools.rs:L908-L937，但它是本地文件能力，不能代替网络客户端。图片 attachment 应限制 MIME、Base64 大小和 data URL 形状。
- 协议/providers：src/protocol.rs:L222-L227 和 L717-L732 的 attachments 是输入 prompt 附件，不是工具输出附件；应在 ToolResult 到模型消息适配层转发图片。按 src/providers/registry.rs:L175-L183、L396-L404 的 images/files/tools 能力，不支持时返回 Unsupported，不能丢弃图片。
- 取消与运行时：src/runtime.rs:L49-L99 的 CancellationToken 是协作式取消，HTTP handler 要在读取块、超时和 fallback 前检查；L357-L365 提供事件超时接收，L794-L912 的 InputBoundaryGate 保证工具批次未完成时不能消费新输入。
- 会话恢复：src/session.rs:L1422-L1556 将缺终止操作标为 UnknownOutcome，L1558-L1588 要求恢复决定后创建新 operation；网络 timeout 应记录可能已发生的远端副作用，不自动重放。src/core.rs:L6679-L6708 注册 builtin/context/approval，L6726-L6749 裁剪并结构化工具错误，应接入新网络错误码和审计字段。
- headless/providers：headless 通过共享工具、approval、runtime 路由本地调用（src/headless.rs:L7385-L7439）。src/providers/** 负责模型协议和能力，不应在 anthropic/openai/google provider 重复抓取；只在消息适配处消费 webfetch 文本/图片，并依据 registry 能力拒绝不支持附件。
- 可执行差异清单：新增等价 serde schema；增加 URL scheme/host policy；实现双重 5 MiB 限制、30/120 秒总 timeout、一次 Cloudflare UA fallback、HTML text/markdown 转换；接入 CancellationToken；把 timeout/未知网络结果写 operation journal，重试必须新 operation；补 mock HTTP、HTML、图片、超时、大小、审批测试。

## 未决问题

1. ctx.ask 的 always=["*"] 精确匹配和持久化语义不在源文件内。
2. HttpClient.filterStatusOk 对非 2xx、重定向和 body 读取错误的具体错误结构未定义。
3. Effect.timeoutOrElse 是否立即中断 socket，以及 Effect.die 如何序列化，需查看 Effect runtime/调用方。
4. isImageAttachment 的完整 MIME 集合、TextDecoder 编码策略和 Turndown 对畸形 HTML 的细节无法从本源确认。


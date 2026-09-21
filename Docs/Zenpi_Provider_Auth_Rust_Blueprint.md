# zenpi 认证、模型与协议 Rust 蓝图

> 唯一 Rust 设计定稿，2026-09-22。本 PR 只改文档，不表示下述接口已经实现或通过运行时验收。
> 本文合并认证 SPEC、转换清单和验收合同；不替换既有执行蓝图、任务状态或历史验收收据。

```yaml
schemaVersion: design-blueprint/v1
version: 1.0.0
date: 2026-09-22
status: design-final
authoritative: false
designFrozen: true
implementationAccepted: false
canonicalFor: provider-auth-protocol-rust-design
mode: extension
researchMode: targeted
chosenDirection: one-provider-file-shared-protocols-port-current-pi-auth
constraints: [docs-only-now, no-js-runtime, one-agent-loop, one-retry-owner, no-implicit-broker]
zenpiCommit: 38f15d4e1263fdf981bca7bdd85ea06ec47c9bed
piCommit: 4c8eb393c73220c742e75745df210335aedc020e
ohMyPiCommit: 260eaa35fe3480ef1090641547fefde2ad879113
codexLocalCommit: 578c1b2230288104041e880a86d0f7f3a5ca6e47
deepSeekHarnessCommit: ddefc45fbc7f8e46dd73185e68295696d1297887
```

阅读入口：[上游调查](#1-结论与范围)、[DeepSeek Harness](#13-deepseek-harness-的设计与取舍)、
[Rust 目录/函数](#3-一个接口一个-provider-文件共享协议)、[协议矩阵](#5-接入矩阵与-deepseek)、
[多模态](#6-多模态请求与历史)、[认证生命周期](#7-认证生命周期)、
[TUI/headless](#9-tuicli-与-headless)、[转换清单](#10-复用与转换清单)、[验收](#12-唯一验收矩阵)。

## 1. 结论与范围

主问题：怎样让 TUI/headless 在同一内核上正确选择模型、协议、端点和账号，并安全处理长会话认证与多模态内容？

推荐：**一个统一接口层，每个 provider 一个定义文件，共享协议适配器，认证单独管理。**
以本次固定版 Pi 的小型认证契约与 Codex OAuth 为 TS -> Rust 主线；复用 zenpi 已有 Rust 协议和运行时，以 OMP/Codex 补充安全与竞态测试。
DeepSeek 只建一个 provider 文件，选择 Chat、Responses、Anthropic Messages；不复制三套登录或流解析。

| 选择 | 收益 | 代价 | 决策 |
|---|---|---|---|
| 全量翻译 OMP | 表面结构接近上游 | 带入 KDL、broker、动态 hooks、重复存储/Loop | 不选 |
| 整包复用 codex-login | 已有 Rust OAuth | 直接依赖 11 个其他 Codex workspace crate，强绑定其产品和运行时 | 不选 |
| Pi 认证语义转换 + 现有 zenpi | 小接口、协议复用、可逐项验证 | 必须记录有意差异与真实服务资格 | 采用 |

范围：API key/Codex OAuth、TUI 管理和原子连接切换、headless、同 provider 多协议、DeepSeek 三路由、多模态输入/工具结果/历史、目录和函数职责、迁移与验收。
非目标：重写 TUI/Agent Loop、自动账号轮换、共享 Broker 实现、数据库、JS bridge、WebSocket/Lite/预热、音视频生成或转写。
“pure Rust”指业务与认证不依赖 Node/Bun/Python/Codex 子进程，不保证 TLS/系统集成等传递依赖完全没有原生 FFI。

### 1.1 调研版本与身份

Pi/OMP/DeepSeek Harness 的上游核验截止为 **2026-09-21 16:44:12 UTC**。“最新”指该时刻的默认分支 HEAD，不是永久最新，也不等同最新 release。
实施前比较上游差异，不悄悄更换输入版本；下表明确区分运行时审阅基线、源码包版本和已发布 release。

| 对象 | 核验版本 | 证据与限制 |
|---|---|---|
| zenpi | `38f15d4e1263fdf981bca7bdd85ea06ec47c9bed` | 运行时代码审阅基线，不是文档定稿时的当前 HEAD；后续文档提交不表示下述运行时接口已经实现 |
| Pi coding agent | `4c8eb393c73220c742e75745df210335aedc020e` | [main 固定提交](https://github.com/earendil-works/pi/commit/4c8eb393c73220c742e75745df210335aedc020e)；源码包版本 `0.87.0`，本轮 GitHub latest release 仍为 [v0.86.1](https://github.com/earendil-works/pi/releases/tag/v0.86.1) |
| oh-my-pi（OMP） | `260eaa35fe3480ef1090641547fefde2ad879113` | [main 固定提交](https://github.com/can1357/oh-my-pi/commit/260eaa35fe3480ef1090641547fefde2ad879113)；源码包版本 `18.2.8`，本轮 GitHub latest release 仍为 [v18.2.7](https://github.com/can1357/oh-my-pi/releases/tag/v18.2.7) |
| 本机 Codex | `578c1b2230288104041e880a86d0f7f3a5ca6e47`，2026-07-30 | `/Users/mac/code/storyn/codex/codex-rs`；工作树干净，origin 为 openai/codex；本文分析这份检出 |
| Codex 上游 main | `d583e73c4d1204f1e9f654ef87065e9f0dd07ac7` | 2026-09-21 15:47:52 UTC 核验的[远端提交](https://github.com/openai/codex/commit/d583e73c4d1204f1e9f654ef87065e9f0dd07ac7)；本机不是最新，未把本机行为冒充此版本 |
| DeepSeek | 2026-09-21 官方在线 API 文档 | 第 5 节逐项引用；没有真实模型调用，个别旧集成教程与当前 API reference 有漂移 |
| DeepSeek Harness | `ddefc45fbc7f8e46dd73185e68295696d1297887` | 截止时刻核验官方 master，未变；2026-09-17 提交，标题 release(dsh): 0.1.6-alpha.2；不是正式稳定版承诺 |

本轮增量复核：Pi 从此前 `7f06f9c` 前进 3 个提交，认证/provider/协议生产源码未变；OMP 从 `97f945c` 前进 8 个提交，
相关生产差异仅 `packages/catalog/src/discovery/devin.ts` 的 TypeScript 返回值类型断言，无 wire 行为变化。以下固定链接已同步到上表新 pin。

用户的 `pi-agent-code` 本文按官方 **Pi coding agent** 理解，包名 `@earendil-works/pi-coding-agent`；未核实另一同名官方项目。
项目已由 badlogic/pi-mono 迁移至 earendil-works/pi，见[官方公告](https://pi.dev/news/2026/5/7/pi-has-a-new-home)。

### 1.2 当前上游分层，不沿用过时路径

| 来源 | 已核对的事实 | 采用与限制 |
|---|---|---|
| Pi [auth/types.ts](https://github.com/earendil-works/pi/blob/4c8eb393c73220c742e75745df210335aedc020e/packages/ai/src/auth/types.ts) | CredentialStore.read/list/modify/delete；OAuthAuth.login/refresh/toAuth；ModelAuth | 采用小契约，list 不执行 key command/暴露秘密；不是旧 AuthStorage.getApiKey 架构 |
| Pi [auth/resolve.ts](https://github.com/earendil-works/pi/blob/4c8eb393c73220c742e75745df210335aedc020e/packages/ai/src/auth/resolve.ts) | 锁内二次检查；5 分钟预刷新；15 秒 refresh timeout；存储 OAuth 失败不回退环境 key | 转换生命周期，但不改变 zenpi legacy API key 优先级 |
| Pi [AuthStorage](https://github.com/earendil-works/pi/blob/4c8eb393c73220c742e75745df210335aedc020e/packages/coding-agent/src/core/auth-storage.ts) | storage-only，应用由 ModelRuntime 编排；JSON + proper-lockfile；每 provider 一个凭据 | zenpi 用 credential ID 支持多 profile；不把 refresh token 复制到每个 profile |
| Pi [Models](https://github.com/earendil-works/pi/blob/4c8eb393c73220c742e75745df210335aedc020e/packages/ai/src/models.ts) / [types](https://github.com/earendil-works/pi/blob/4c8eb393c73220c742e75745df210335aedc020e/packages/ai/src/types.ts) | provider 拥有 auth/目录/API factory；model 区分 provider/api/baseUrl | 采用身份与 wire 分离，继续复用 zenpi ModelRegistry |
| OMP [AuthStorage](https://github.com/can1357/oh-my-pi/blob/260eaa35fe3480ef1090641547fefde2ad879113/packages/ai/src/auth-storage.ts) | 多凭据/workspace、健康排序、sticky session、轮换/broker | 不搬账号池，只取身份与刷新竞态规则 |
| OMP [认证定义](https://github.com/can1357/oh-my-pi/blob/260eaa35fe3480ef1090641547fefde2ad879113/packages/catalog/src/compat/rules/auth/openai-codex.kdl) | KDL + OAuth engine/hooks；有额外 connector scopes | 只取必需协议字段，不搬编译器、不扩大 scopes |
| Codex [manager](https://github.com/openai/codex/blob/578c1b2230288104041e880a86d0f7f3a5ca6e47/codex-rs/login/src/auth/manager.rs) / [storage](https://github.com/openai/codex/blob/578c1b2230288104041e880a86d0f7f3a5ca6e47/codex-rs/login/src/auth/storage.rs) | AuthManager、身份守卫、reload/refresh、file/keyring/ephemeral | 借鉴；默认 File，写入非原子替换，刷新落盘无跨进程 CAS，不能照抄弱点 |

用户提供的 Codex 分析总体正确，需补四点：浏览器 1455/备用 1457；API key exchange 可选，device OAuth 不依赖它；TUI 经 App Server 发起登录；Agent Identity 是专用注册/私钥/AgentAssertion，不是普通 worker 权限。
来源：[server](https://github.com/openai/codex/blob/578c1b2230288104041e880a86d0f7f3a5ca6e47/codex-rs/login/src/server.rs)、[device](https://github.com/openai/codex/blob/578c1b2230288104041e880a86d0f7f3a5ca6e47/codex-rs/login/src/device_code_auth.rs)、[TUI](https://github.com/openai/codex/blob/578c1b2230288104041e880a86d0f7f3a5ca6e47/codex-rs/tui/src/onboarding/auth.rs)。
Codex token claims 解码不是 JWT 验签；其账户 reload 守卫不等于完整存储事务保证。

### 1.3 DeepSeek Harness 的设计与取舍

官方项目为 [deepseek-ai/deepseek-harness](https://github.com/deepseek-ai/deepseek-harness/tree/ddefc45fbc7f8e46dd73185e68295696d1297887)，不是 Rust，也不是仅有文档的项目。
它以 TypeScript/Node 和 Cordis 组织插件；profile/bundle YAML 组合模型、工具、session、loop 与入口，默认 loop 本身也可替换。
来源：[固定版架构文档](https://github.com/deepseek-ai/deepseek-harness/blob/ddefc45fbc7f8e46dd73185e68295696d1297887/docs/architecture.md)。

```text
Node launcher + profile/bundle -> Cordis 插件生命周期
  -> Agent contract / ReactLoopAgent（Inbox -> turn -> step）
  -> LlmRuntime.prepareCall / stream
  -> provider route -> adapter -> protocol encoder / SSE decoder
  -> StreamChunk -> assistant settlement -> Session log
  -> Web / headless / SDK JSON-RPC / ACP
```

| 位置 | 当前实现与证据 | 对 zenpi 的取舍 |
|---|---|---|
| [LlmAdapter / LlmRuntime](https://github.com/deepseek-ai/deepseek-harness/blob/ddefc45fbc7f8e46dd73185e68295696d1297887/packages/llm/llm/src/index.ts) | providerInfo/listModels/resolveModel/prepareCall/stream；prepareCall 绑定同一配置代次的 metadata 与 stream closure | 采用请求级 route/config/身份冻结，不把热更新的新 endpoint 与旧 secret 混用；Rust 不需要 closure/plugin 框架才能做到 |
| [DeepSeekAdapter](https://github.com/deepseek-ai/deepseek-harness/blob/ddefc45fbc7f8e46dd73185e68295696d1297887/packages/llm/llm-deepseek/src/adapter.ts) / [config](https://github.com/deepseek-ai/deepseek-harness/blob/ddefc45fbc7f8e46dd73185e68295696d1297887/packages/llm/llm-deepseek/src/config.ts) | 一个 deepseek-official route，选择 messages 或 chat-completions，默认 Messages；此提交没有原生 DeepSeek Responses adapter | 支持 provider/protocol 分离；不能据此否认最新 DeepSeek API 的 Responses，也不能声称照搬就有三 wire |
| [Pi auth bridge](https://github.com/deepseek-ai/deepseek-harness/blob/ddefc45fbc7f8e46dd73185e68295696d1297887/packages/llm/llm-pi-ai/src/auth.ts) / [login bridge](https://github.com/deepseek-ai/deepseek-harness/blob/ddefc45fbc7f8e46dd73185e68295696d1297887/packages/llm/llm-pi-ai/src/login.ts) | 将 Pi CredentialStore 接到 modifyRecord，将 AuthInteraction 接到统一登录 flow；依赖 pi-ai ^0.85.1，不是本次最新 Pi main | 进一步支持“采用 Pi 小认证契约”，但不经 DSH 再套一层 TS runtime |
| [LocalCredentialProvider](https://github.com/deepseek-ai/deepseek-harness/blob/ddefc45fbc7f8e46dd73185e68295696d1297887/packages/credentials/credentials-local/src/index.ts) | .credentials.yaml；env-like reference 与 scoped record 分开；modifyRecord 锁内重读、原子写、0700/0600 | 采用事务与秘密引用原则；zenpi 继续 JSON，不新增不透明 grant/plugin 框架 |
| [StreamChunk](https://github.com/deepseek-ai/deepseek-harness/blob/ddefc45fbc7f8e46dd73185e68295696d1297887/packages/llm/llm/src/types.ts) | block start/delta/end、tool raw JSON、usage、finish；图片为 durable reference，非长期 base64 | 借鉴有序块与流/落盘区分；保留 zenpi 事件类型，不重建第二套 |
| [ReactLoopAgent](https://github.com/deepseek-ai/deepseek-harness/blob/ddefc45fbc7f8e46dd73185e68295696d1297887/packages/core/agent-loop/src/agent.ts) | followup 入 next-turn，steer 入 next-step；prepare/stream/settle/tools 循环 | 不移植 loop；沿用 zenpi 输入/工具/会话所有者，仅参考冻结与取消测试 |
| [headless](https://github.com/deepseek-ai/deepseek-harness/blob/ddefc45fbc7f8e46dd73185e68295696d1297887/packages/bundle/headless/src/index.ts) / [SDK protocol](https://github.com/deepseek-ai/deepseek-harness/blob/ddefc45fbc7f8e46dd73185e68295696d1297887/packages/sdk/protocol/src/types.ts) | headless 是无 server 的 one-shot，--json 为提交后投影；SDK 是持久 stdio JSON-RPC；多个入口共享默认 driver | 不将其 --json 称逐 token 流，不把 SDK/ACP 加成 zenpi 第三模式；此树未发现 first-party TUI package |

它的 live deltas 在 settlement 后才成为 durable assistant message；失败 attempt 与可重放 message 分开，不能称逐 token 持久化。
普通文件会投影为文件句柄文本，text-only route 的图像可变占位；zenpi 不默认吸收这些信息丢失策略。
原生图像上传/复用/base64 fallback 是 DSH 已有额外功能，不因此扩大本蓝图首期的 Files 范围。

结论：采用其请求冻结、凭据事务、protocol/provider 分离、流与 durable facts 区分；**不复制 Everything-is-a-Plugin/Cordis 微包体系**。
它实际是一个 provider package 内分协议目录，而不是字面上所有逻辑一个文件；zenpi 按用户要求收敛为一个 provider 定义文件 + 共享 protocols，保留相同边界而减少框架。

### 1.4 有限认证与协议清单

这是已核对的上游边界，不是首期实现全部 provider 的承诺。`api_key | oauth` 是 Pi 的存储类别，不是登录交互方式，也不能据此推导请求协议。
来源：Pi [认证类型](https://github.com/earendil-works/pi/blob/4c8eb393c73220c742e75745df210335aedc020e/packages/ai/src/auth/types.ts)、[OAuth loader](https://github.com/earendil-works/pi/blob/4c8eb393c73220c742e75745df210335aedc020e/packages/ai/src/auth/oauth/load.ts)；
OMP [编译后认证 registry](https://github.com/can1357/oh-my-pi/blob/260eaa35fe3480ef1090641547fefde2ad879113/packages/catalog/src/compat/rules.json)、[定义契约](https://github.com/can1357/oh-my-pi/blob/260eaa35fe3480ef1090641547fefde2ad879113/packages/ai/src/registry/types.ts)。

| 认证方式 | 固定上游事实 | 首期或延后 |
|---|---|---|
| API key / provider 配置 | Pi 支持 key、环境变量及 provider-scoped 多字段配置；OMP 有 53 个 `api-key` login 定义 | 首期支持显式 API key 引用和既有 legacy；同一凭据可供同 provider 的授权协议 profile 引用，不授权任意目的地 |
| 浏览器授权 | Pi Anthropic、OpenRouter；Codex、Radius 可选浏览器或 device | 首期只移植 Codex 浏览器/人工完整回调；其余浏览器授权延后 |
| device 授权 | Pi Codex、Radius、GitHub Copilot、Kimi Coding、xAI、Meta | 首期只移植 Codex device；其他 device flow 延后，不因共享轮询算法即宣称支持 |
| 外部 identity | Pi AWS chain/profile、Vertex ADC/service-account；OMP 另有 Foundry/Bedrock Mantle 等 env/native hooks | 延后；外部 owner 持有和更新身份，不复制成 zenpi-owned refresh grant |
| 导入 / 外部 token | 导入和借用 token 不是上述 OAuth loader 的另一种标准登录方式 | 首期仅识别既有 OAuth 来源并引导自有登录；不复制轮换 refresh token，不执行外部 key command |

Pi 的 8 个 OAuth loader 为 `anthropic`、`openai-codex`、`github-copilot`、`openrouter`、`kimi-coding`、`meta`、`xai`、`radius`。
OpenRouter 浏览器 flow 的结果是长效 API key；Copilot 另换推理 token/endpoint；Meta 将 identity token 存入 `refresh` 字段，但 `refresh()` 实际是重新铸模型 API key，identity token 本身不能续期。
这些是 provider 特例，不把 Pi 的开放字段凭据对象原样泛化为 Rust OAuth 记录。来源：[OpenRouter](https://github.com/earendil-works/pi/blob/4c8eb393c73220c742e75745df210335aedc020e/packages/ai/src/auth/oauth/openrouter.ts)、[Copilot](https://github.com/earendil-works/pi/blob/4c8eb393c73220c742e75745df210335aedc020e/packages/ai/src/auth/oauth/github-copilot.ts)、[Meta](https://github.com/earendil-works/pi/blob/4c8eb393c73220c742e75745df210335aedc020e/packages/ai/src/auth/oauth/meta.ts)。

OMP registry 共 87 条：53 `api-key`、10 `oauth-code`、3 `device-code`、9 `custom`、12 无 login；其中含网关、搜索/工具凭据，不全是模型供应商。
`oauth-code` 为 openai-codex、anthropic、zai-coding-plan、openrouter、devin、google-antigravity、google-gemini-cli、gitlab-duo、gitlab-duo-agent、stencil；
`device-code` 为 kimi-code、xai-oauth、muse-code；`custom` 为 github-copilot、cursor、openai-codex-device、alibaba-coding-plan、alibaba-token-plan、xiaomi、perplexity、cloudflare-ai-gateway、kilo。
`custom` 不保证是 OAuth；OMP 的 `OAuthProvider` 名单由存在 login 派生，也包含贴 API key，不能按类型名称扩张首期支持。

| 协议家族 | 固定上游 inventory | 首期或延后 |
|---|---|---|
| OpenAI 公共 wire | Pi/OMP：`openai-completions`、`openai-responses` | 首期复用/补齐 Chat 与 Responses；DeepSeek 显式映射到共享适配器 |
| Anthropic / Google 原生 | Pi/OMP：`anthropic-messages`、`google-generative-ai` | 首期保留既有适配；DeepSeek Messages 复用 wire，独立验证服务参数/鉴权 |
| Codex 产品 transport | Pi/OMP：`openai-codex-responses` | 首期 HTTP SSE，专属 endpoint/auth/header，不当作任意 OpenAI Responses URL |
| 云平台 transport | Pi/OMP：`azure-openai-responses`、`bedrock-converse-stream`、`google-vertex` | 延后新增云身份/端点语义；不以兼容 JSON 代替平台适配 |
| 其他专用 chat/agent wire | Pi：`mistral-conversations`、`pi-messages`；OMP：`openrouter`、`google-gemini-cli`、`ollama-chat`、`cursor-agent`、`gitlab-duo-agent`、`devin-agent` | 延后专用适配；显式兼容 Chat profile 不等于已支持对应原生 transport |
| 非 chat runner | Pi 图像 API：`openrouter-images`；OMP 另列 image/speech/transcription/embedding/rerank/video/search/judge 等 runner API | 延后；多模态输入不等于图像/音视频生成能力 |

精确联合类型：Pi [KnownApi / KnownImagesApi](https://github.com/earendil-works/pi/blob/4c8eb393c73220c742e75745df210335aedc020e/packages/ai/src/types.ts#L17)，
OMP [KnownApi / RUNNER_APIS](https://github.com/can1357/oh-my-pi/blob/260eaa35fe3480ef1090641547fefde2ad879113/packages/catalog/src/types.ts#L9)。Pi 为 10 个 chat API 加 1 个独立 image API；OMP 为 14 个 chat API 加独立 runner 集合。
注册表只用于可审计的静态定义和验收范围；不引入 KDL 编译器、动态 hook 系统或第二套运行时。

## 2. zenpi 当前审阅结果

以下是固定 HEAD 的事实；阅读了源码/测试，但本轮未执行 Cargo 测试。

| 优先级 | 事实与源码 | 必须补齐 |
|---|---|---|
| P1 | [Agent::set_model](../src/core.rs#L1071) 只改模型；[backend_from_effective](../src/core.rs#L6029) 启动时固定 endpoint/key/wire | 跨 profile 原子切换整连接，不能只改模型名/标题 |
| P1 | [run](../src/core.rs#L6376) 在 host 启动前创建 backend，缺认证即失败 | 首次无凭据 TUI 要有独立引导，不能假设现有路径已支持 |
| P1 | [backend](../src/backend.rs#L557) 长期缓存 key；生产未统一接上 issue_secret_handle | 每请求 resolve + SecretHandle，覆盖续接/摘要/重试 |
| P1 | [发送头](../src/backend.rs#L1244) 仅区分 Google key 和 Bearer，包括 Anthropic | header policy 独立于 wire，原生 Anthropic/DeepSeek Messages 核验 x-api-key |
| P1 | [active_attachments](../src/core.rs#L3995) 只保留当前 turn，完成即清；journal 只留引用/hash | 工具续接可用不等于跨 turn/重启重放 |
| P2 | [profile 状态](../src/config.rs#L1375) 未完整反映 env/默认 URL；host doctor 用默认配置 | 状态从当前 owner 有效连接快照投影 |
| P2 | [TUI zone model](../src/tui.rs#L12515) 可能先改显示，实际请求仍取 self.model | Core Applied 后才改 UI；zone 偏好不等于独立认证路由 |
| P2 | Core 按逻辑调用计 NetworkRequests，Backend 内可重试 | 每个物理 HTTP（含刷新）重新准入 |

复用：Backend/CompletionRequest/ProviderEvent/Completion、精确 provider/model 目录、Chat 流、Anthropic/Google 原生适配、取消 transport、session、SecretHandle、Governance。
Responses 当前主要逐行解析，还需补完整 SSE framing；不能拿 Chat multiline 测试证明 Responses 已支持。

### 2.1 三种认证分开

| 名称 | 控制什么 | 不得混同 |
|---|---|---|
| 模型/provider 认证 | API key、ChatGPT OAuth、账号与 endpoint | 用户所说 Codex/agent 订阅登录属于这里 |
| Agent/worker 授权 | 工具、文件、网络、预算、lease | 继续由 [admit_blueprint_worker](../src/core.rs#L1465)、security/governance 管理，登录不扩权 |
| 远端主机认证 | SSH identity 与主机 allow-list | [cluster.rs](../src/cluster.rs) 继续独立管理；有凭据不代表获准 dispatch |

UI 分开显示“连接账号”和“Agent 权限”。模型或插件不能发起登录/导入/撤销/授信 endpoint。
被委派的工具/extension/远端 worker 不读取 auth.json、不接收 refresh token；SecretHandle 仍绑定原 worker policy，不能用 route digest 替换权限摘要。
TUI BackgroundRunner 内的 Agent/backend 是可信宿主线程，不是上述 worker；它可通过唯一宿主 AuthResolver 请求认证，不能把存储能力转交工具。
任意未隔离的同 UID 程序不在秘密隔离承诺内；不能用文件 mode 声称能抵御同用户进程读取。

## 3. 一个接口、一个 provider 文件、共享协议

```text
TUI / headless / config auth CLI
  -> Core + Runtime + Session + Tools（既有唯一执行所有者）
  -> Backend（统一接口、唯一推理重试/发送编排）
       -> ProviderDefinition + ModelRegistry -> 固定 RequestRoute
       -> AuthResolver -> CredentialStore / Codex OAuth
       -> shared Protocol encoder / stream reducer
       -> RequestControl -> 既有 Governance（每次 HTTP）
       -> backend::transport -> ureq / TLS / cancellation
```

`provider != model != protocol != auth kind != endpoint`。
一个 provider 可选多 wire；多个 provider 可用同一个 wire。统一语义不等于全部发 OpenAI JSON。
有效能力 = 模型声明、协议实现、具体产品/端点、任务策略的交集；新连接的未知模型不默认有 tools/vision。
兼容例外：现有 legacy 路径允许未知模型图片（见 tests/stage1_model_registry.rs），本轮不悄悄收紧。
严格门控适用于显式新连接；将 legacy 迁入新连接时提示能力差异，另行确认并补测试，不能同时承诺收紧和零行为变化。

### 3.1 目标 Rust 目录

保持一个 Cargo crate。以下为目标结构，本 PR 不创建源码文件。移动现有代码只为让同一协议被复用，不顺手重写。

```text
src/
  backend.rs                  # 统一请求/事件、发送与重试编排，兼容现有公共接口
  backend/
    transport.rs              # 保留 HTTP/TLS/取消/线程回收
    chat_stream.rs            # 迁移期复用，最终并入 protocols/chat.rs
  providers/
    mod.rs                    # ProviderDefinition + 静态注册/查找
    registry.rs               # 保留模型目录、能力、来源、digest
    connection.rs             # profile/model -> route、端点/身份/header 校验
    openai.rs                 # 一个 provider 一个定义文件
    codex.rs                  # ChatGPT 产品定义、专属 endpoint、OAuth 绑定
    deepseek.rs               # 三协议映射、DeepSeek 能力和参数规则
    anthropic.rs              # 服务定义；现有协议实现迁出后不复制
    google.rs                 # 服务定义；现有协议实现迁出后不复制
  protocols/
    mod.rs                    # 静态 wire 分发，不增加第二个 Backend trait
    content.rs                # 借用现有 Turn/Attachment 的有序内容视图
    chat.rs                   # 现有 Chat 编码/流逻辑迁入
    responses.rs              # 普通/Codex/DeepSeek Responses 共用 framing/reducer
    anthropic.rs              # 从现有 providers/anthropic.rs 迁入原生协议
    google.rs                 # 从现有 providers/google.rs 迁入原生协议
  auth/
    mod.rs                    # AuthBinding/Status/Error/Event，窄门面
    store.rs                  # 私有 JSON、锁、原子写、revision
    resolve.rs                # 请求级解析/刷新/认证失败恢复/撤销编排
    codex.rs                  # 浏览器/device/code exchange/refresh
    callback.rs               # loopback callback 与 manual URI 验证
  tui/bootstrap.rs            # 未认证时仅登录/选连接，不启动任务运行时
  security.rs                 # 保留 SecretHandle、原 policy、脱敏，补 expiry/scope
  config.rs                   # 保留旧配置兼容，增加非秘密 auth/route 引用
  core.rs                     # 原子选连接、上下文、工具/摘要共用 backend
  protocol.rs                 # headless typed 控制和脱敏回执
  slash.rs / slash_actions.rs # 共享命令解析和状态投影
  tui.rs / headless.rs        # 宿主交互，不各写一套认证状态机
  session.rs                 # 非秘密连接选择、内容引用和恢复
```

“一个 provider 一个文件”指**服务定义**，不是把一个品牌所有网络/存储/解析塞进单文件。
Codex OAuth 在 auth/codex.rs；Codex Responses 差异在共享 responses.rs 的显式 dialect 分支，不再复制一个流引擎。
暂不按品牌建 crate，不做动态 provider 插件/KDL compiler，也不新增 Gateway/Transport/Store 单实现 trait。

### 3.2 每个文件的函数职责

下表除标“现有”外均是拟议接口；命名用于后续实现对照，不是假装已有代码。

| 文件 | 类型 / 函数 | 输入 -> 输出与禁止事项 |
|---|---|---|
| backend.rs | 现有 complete_with_control；新增 complete_with_request_control、内部 apply_auth | 请求 -> 事件/结果；唯一重试；发送前借用 SecretHandle，不公开 token getter |
| providers/mod.rs | ProviderDefinition、provider_definition | provider ID -> 静态定义；未知 provider 需显式兼容配置，不猜品牌 |
| providers/registry.rs | 现有 ModelRegistry/ModelDescriptor | 模型能力/来源/digest；不保存凭据 |
| providers/connection.rs | resolve_connection、resolve_model_route、validate_destination、connection_status | 配置 -> validated route；不登录、不改 session |
| providers/openai.rs | definition、routes | API key、Responses/Chat 的服务规则，不管理 ChatGPT token |
| providers/codex.rs | definition、routes、validate_options | 固定 OAuth 目的地、Codex dialect/能力；不做 code exchange |
| providers/deepseek.rs | definition、routes、map_reasoning、validate_options | 三 wire、header/字段/能力差异；不复制 encoder，不任意 extraBody 覆盖安全字段 |
| providers/anthropic.rs | definition、routes | 原生 API key/header/endpoint 规则；不内置第三方 Claude 订阅 token |
| providers/google.rs | definition、routes | Gemini API key/endpoint 规则；不把 GCP ADC 当普通 key |
| protocols/mod.rs | encode_request、read_response | 有限 wire 分发，共用现有 BackendError/ProviderEvent |
| protocols/content.rs | content_parts、validate_content、validate_media_scope | 已验证 Turn/内容 -> 有序视图和能力检查；不读取文件、不持 ToolContext、不隐式丢图片 |
| protocols/chat.rs | encode_request、read_stream | 现有 Chat body/SSE/tool/reasoning；保留兼容测试，不读秘密 |
| protocols/responses.rs | encode_request、reduce_event、read_stream、map_call_id | 共用 Responses；Codex/DeepSeek dialect 明确能力差异，不到处判断域名 |
| protocols/anthropic.rs | 现有编码/历史验证/流函数 | 原生 Messages 内容、tool_use/result、签名语义 |
| protocols/google.rs | 现有 endpoint/编码/历史验证/流函数 | 原生 Gemini parts/function/signature 语义 |
| auth/mod.rs | AuthBinding、AuthStatus、AuthError、AuthInteraction、LoginState/Outcome | binding 只有引用；状态不含 token/登录 URL；challenge 仅给可信发起宿主 |
| auth/store.rs | read、list_status、modify、revoke、with_refresh_lock | credential ID + expected_revision -> 快照/提交；不做 HTTP/key command |
| auth/resolve.rs | resolve_for_request、refresh_if_needed、recover_unauthorized、revoke_credential | 凭据生命周期；不重发推理、不选择另一个账号 |
| auth/codex.rs | begin_browser_login、begin_device_login、poll_device_login、exchange_code、refresh_token、parse_token_response | typed flow/token；有界 HTTP/取消；不改默认 profile |
| auth/callback.rs | bind_loopback、wait_callback_or_manual、validate_callback | 注册 URI/state -> 一次 code/拒绝/取消；关闭 listener 并 join |
| tui/bootstrap.rs | run_auth_bootstrap | 未配置 -> 选定连接或退出；不启动 Agent/资源 runner |
| core.rs | prepare_initial_agent、prepare_connection_change、commit_connection_change；扩展现有 materialize_attachments | 完整初始化一次；候选检查 -> durable event -> infallible swap；附件经既有 ToolContext 权限读取，不下放协议层 |
| session.rs | 扩展现有 model_selected 的版本化连接快照 | 保存非秘密选择、恢复校验；不把 journal endpoint 直接当可信配置 |
| protocol.rs / slash_actions.rs | ConnectionSelect/ConnectionStatus 语义 | TUI/headless 同 owner；status/doctor 不调用模型或刷新 |

### 3.3 接口框架（非可编译实现）

```rust
pub(crate) enum Protocol {
    ChatCompletions,
    Responses,
    AnthropicMessages,
    GoogleGenerativeAi,
    OpenAiCodexResponses, // 共享 Responses 实现的专属 dialect
}

pub(crate) enum AuthBinding {
    LegacyApiKey,
    StoredApiKey { credential_id: String },
    CodexOAuth { credential_id: String },
    Anonymous,
}

pub(crate) struct ProviderConnection {
    pub profile: String,
    pub provider: String,
    pub protocol: Protocol,
    pub endpoint: String,
    pub auth: AuthBinding,
    pub header_policy: AuthHeaderPolicy,
    pub config_revision: u64,
}

pub(crate) struct RequestRoute {
    pub connection: ProviderConnection,
    pub model: ModelDescriptor, // 复用 registry 类型
    pub route_digest: String,
    pub identity_scope: String,
}

pub(crate) struct AuthContext {
    pub secret: Option<SecretHandle>,
    pub account_id: Option<String>,
    pub credential_revision: Option<u64>,
}

// 合同签名；control 由现有执行所有者提供。
fn resolve_for_request(
    binding: &AuthBinding,
    route: &ValidatedRoute,
    control: &mut RequestControl<'_>,
) -> Result<AuthContext, AuthError>;
```

AuthHeaderPolicy 先用有限枚举 Bearer/XApiKey/GoogleApiKey/Codex/None；服务定义选默认值，兼容网关可显式选已有 API key 策略，不 override OAuth 受保护头。
AuthContext 是 API key/Codex 的有限请求投影，不是 Pi ModelAuth 的全等翻译；任意 auth headers、credential 派生 endpoint、AWS/ADC 不能塞进此结构绕过 ValidatedRoute，须独立新增产品适配并验收。
ValidatedRoute 构造和 apply_auth 为 crate-private；任意字符串不等于“受信任”。
route_digest 覆盖 provider/wire/规范端点/非秘密选项/auth 引用/规则版本；token 轮换不改变模型 digest 或账号身份。
identity_scope 绑定 provider、credential ID、account/workspace 和非秘密 identity_generation；email 不当身份。
API key secret 被替换时必须生成新 identity_generation，即使 credential ID 未变也不继承旧 file ID/native scope；不能公开 secret hash 充当身份。
同账号 OAuth access-token refresh 只变 credential_revision，不变 identity_generation；登录成另一账号则变身份作用域。
旧公共 OpenAiWireApi 四变体保留为兼容入口，显式映射到新内部 Protocol；Codex 新配置直接选专属变体，不增加第二个协议执行器。
不能顺手改变低层默认 Chat、生产默认 Responses 的历史契约；新增 DTO 字段涉及 Rust struct literal 的源码迁移，在实现 PR 列出所有调用点，不声称零源码破坏。

### 3.4 定义到实现的最小函数合同

以下为同 crate 合同草图，不创建 plugin trait 或运行时工厂。ProviderDefinition 是静态数据；复杂字段映射留在有限 provider 函数内。

```rust
struct ProviderDefinition {
    id: &'static str,
    definition_version: u32,
    routes: &'static [RouteRule],
}
struct RouteRule {
    protocol: Protocol,
    dialect: Dialect,
    endpoint: EndpointRule,
    header_policy: AuthHeaderPolicy,
    capabilities: ProviderCapabilities,
    options: OptionPolicy,
}
fn resolve_model_route(
    definition: &ProviderDefinition,
    connection: &ProviderConnection,
    model: &ModelDescriptor,
    identity: &AuthIdentitySnapshot,
) -> Result<ValidatedRoute, BackendError>;
fn encode_request(
    route: &ValidatedRoute,
    content: &PreparedContent<'_>,
    request: &CompletionRequest<'_>,
) -> Result<EncodedBody, BackendError>;
fn read_response(
    route: &ValidatedRoute,
    reader: &mut dyn std::io::Read,
    sink: &mut dyn FnMut(ProviderEvent) -> Result<(), BackendError>,
) -> Result<Completion, BackendError>;
```

- EndpointRule 只有 `Fixed` 与 `PrefixAndOperation`；后者明确 suffix，Google 还编码 model 与 streaming operation。完整 URL 必须在 validate_destination **之前**产出并冻结，encoder 不能改 URL/auth。
- Dialect 有 Default/Codex/DeepSeek；Protocol 的 Codex 变体映射 Responses 编解码 + Codex dialect。OptionPolicy 只列实际支持的 reasoning/output cap/tool/response-format 约束，不开放任意 extraBody。
- AuthIdentitySnapshot 来自 auth 本地只读检查，不含秘密；无效/撤销身份不能构造可发请求的 route。身份与配置检查不触发 refresh。
- ValidatedRoute 持有最终 URL、model/能力、非秘密 route/config/identity 摘要和规则版本；仅 connection 模块可构造。EncodedBody 只含有界 JSON bytes 和 Content-Type，不含 auth/目标 URL。
- encode_request 是纯验证/序列化；read_response 只做有界解码/流归并，不请求凭据、不刷新、不重发。transport 检查总响应与逐帧上限，decoder 的错误不会直接打印未过滤正文。
- prepare 顺序：冻结路由/权限 -> Core 物化内容 -> 能力、历史、body/大小检查 -> resolve auth -> 每 send 准入 -> 注头/发送。unsupported 输入不得先刷新、探测端点或发任何 HTTP。

可信宿主在请求准入时生成 RequestScope，涵盖 `owner_id/session_id/turn_or_operation_id/purpose/route_digest/identity_scope/policy_digest?/lease_id?`。
purpose 有 Turn/ToolContinuation/SemanticCompaction；独立登录不冒充模型 turn。worker 请求必须带原 policy/lease，普通交互请求仍经既有 owner 权限。
后台续接/compact/resume 不读取后来切换的 active project 账号；一次操作 route/identity 不变，同账号 refresh 可更新 credential_revision。
整个请求只借用短时 SecretHandle，with_secret 仍验证原 policy，并增验 expiry、route 与 identity；无秘密 Serialize/Debug 或 String getter。

## 4. 配置与模型路由

1. legacy profile 保持既有 CLI > env > user config 优先级；新显式 OAuth 不受通用 OPENAI_API_KEY/ZENPI_API_KEY 覆盖。
2. profile 决定默认 provider/wire/端点/auth_ref；精确 model_routes 可覆盖某 model 的 wire/base_url，但不能偷偷换凭据身份。
3. 同一 profile/model 只能一条 route；同模型多协议用不同 profile，共享一个 credential ID。
4. model_overrides 仍管能力，model_routes 只管路由，不维护两套 capabilities。
5. 项目配置、模型输出、extension 不得授信 OAuth endpoint/替换 auth_ref；用户级设置或可信显式 CLI 才可选账号。
6. 所有 override 之后再次校验目的地/能力，冻结 route；显式 profile 失败不回落默认 profile。

base_url 是 API 前缀，不一定只是域名；/v1、/anthropic/v1、/v1beta 不能猜。
旧配置保留 normalize 行为；新路径只接受服务规则规定的 API 前缀，用 URL parser 追加一次明确 suffix，不用 ends_with 猜测完整 endpoint。
拒绝 userinfo/fragment/不允许 query/控制字符/歧义路径；OAuth 仅受审 HTTPS origin/path，禁止重定向。

### 4.1 拟议配置

新字段实施后才能使用；secret 永不写 TOML。

```toml
[profiles.codex]
backend = "openai"
provider = "openai-codex"
wire_api = "openai_codex_responses"
model = "<explicit-available-model-id>"
auth_method = "oauth"
auth_ref = "cred_codex_example"

[profiles.deepseek-chat]
backend = "openai"
provider = "deepseek"
wire_api = "chat_completions"
base_url = "https://api.deepseek.com"
model = "deepseek-flash"
auth_method = "api_key"
auth_ref = "cred_deepseek_example"

[profiles.deepseek-responses]
backend = "openai"
provider = "deepseek"
wire_api = "responses"
base_url = "https://api.deepseek.com"
model = "deepseek-flash"
auth_method = "api_key"
auth_ref = "cred_deepseek_example"

[profiles.deepseek-messages]
backend = "openai"
provider = "deepseek"
wire_api = "anthropic_messages"
base_url = "https://api.deepseek.com/anthropic/v1"
model = "deepseek-flash"
auth_method = "api_key"
auth_ref = "cred_deepseek_example"
```

Anthropic SDK 的 DeepSeek 示例前缀是 /anthropic，SDK 追加 /v1/messages；zenpi 裸 HTTP 前缀明确带 /v1。
这是由官方 [Anthropic 示例](https://api-docs.deepseek.com/guides/anthropic_api/)和 [Files API 的 SDK /v1 说明](https://api-docs.deepseek.com/guides/files_api/)推导的完整 Messages 路径，仍需 fixture 精确断言。

auth_method 为 legacy_api_key/api_key/oauth/none，缺省保留旧行为；api_key/oauth 必须匹配 auth_ref 类型。
显式 stored API key 不受无关全局 key 偷换；legacy 才保持旧 key 优先级。none 必须显式且无引用，首期只允许用户确认的本地兼容端点，不等于缺 key。
Codex OAuth 不接受任意 base_url；共享 API key 也必须由用户确认允许目的地，不能改 endpoint 后自动带 key 发往新服务。

### 4.2 字段、匹配和校验

| 字段 | 新显式连接合同 |
|---|---|
| provider / model | 复用既有身份校验：provider <=128、model <=256 字节，非空、无空白/控制字符；不由模型名字猜 provider |
| auth_method | 缺省 legacy_api_key；api_key/oauth 必须带 auth_ref；none 禁 auth_ref/auth_env/auth_header 的认证值 |
| auth_ref | 不透明 credential ID，ASCII 字母/数字/下划线/连字符，1..128 字节；只在用户配置或可信管理命令设置，无 token 内容 |
| auth_env | 仅 legacy_api_key 保留既有语义；显式 api_key/oauth 与 auth_env 同时出现拒绝，不悄悄忽略 |
| auth_header | 可省略，取 RouteRule；API key 可选 bearer/x_api_key/google_api_key，仍需该 route 许可；OAuth 禁用户覆盖；不是任意 headers 表 |
| wire_api | 沿用现有四值及别名解析，新加 openai_codex_responses；显式新连接必须有 wire 或 provider 内置唯一默认，未知 provider 必填 |
| base_url | <=2048 字节的 API 前缀，移除单个尾斜线后严格比较；新连接拒绝 query/fragment/userinfo、点段、编码斜线/反斜线歧义；生产 HTTPS，本地 none 仅显式 loopback HTTP |
| model_routes | profile 内最多128条；仅 exact model、wire_api、base_url；同 model 重复报错，无 glob、优先级排序或备用路由；继承同一 auth_ref/header policy |
| model_overrides | 保留原模型能力格式，不因 model_routes 重复建目录；未知模型需显式 capability override 才启用 tools/images/files |
| timeout_seconds / max_retries | 保留 1..3600 秒 / 0..10 次及既有默认值；不能绕过请求剩余 deadline/治理预算 |

ProviderProfile 继续 deny_unknown_fields；新增字段也接入配置读写、doctor、测试，不仅 backend 构造器。
无 auth_method 的 legacy profile 不接受 auth_ref/auth_header/model_routes，避免配置拼写误差意外升级路径。
项目级配置可以在已批准连接内选 model/非秘密参数，但不能重绑定 provider/wire/base_url/auth_ref/header/目的地授权。
用户名/profile alias 沿用现有 config 校验，不把 alias 当 credential ID 或 account ID。

```toml
[[profiles.deepseek-chat.model_routes]]
model = "deepseek-v4-pro"
wire_api = "responses"
base_url = "https://api.deepseek.com"
```

以上表示该精确 model 在此 profile 走 Responses，其他 model 仍用 profile 的 Chat 默认；若同一 model 要手动选三 wire，使用三个 profile，而非三条冲突 route。
解析固定顺序：用户 profile + 获准 CLI 选择 -> exact model route -> 内置服务规则 -> 能力交集 -> 已授权目的地 -> 冻结。
API key 的 allowed_destinations 存在私有 credential 中，内容为规范 origin + 路径前缀边界 + 允许 wire/header 集合；不能只比较字符串 starts_with。
新增目的地必须独立可信用户确认并递增 credential revision，读取/修改 TOML 不会自动授权。内置 DeepSeek 三路由可在添加 key 时作为明确显示的一组同时授权。
custom provider 没有隐式目录能力或任意 endpoint 探测；要接入 /v1/responses 网关，应填其 API 前缀 /v1 和 Responses wire，并显式授权该目的地。
登录可先保存无 model 的 profile，状态为 needs_model；bootstrap 或带 --model 的可信管理命令补齐并校验后才创建 Agent。不得使用占位模型发请求。

## 5. 接入矩阵与 DeepSeek

### 5.1 首期目标

“目标支持”不是当前全部可用。每行验收 header/body/stream/tools/modalities/errors 和 headless，不只验证构造器。

| provider / 产品 | wire / 完整位置 | 认证 | 边界 |
|---|---|---|---|
| OpenAI API | Responses /v1/responses；Chat /v1/chat/completions | API key Bearer | 复用现有协议，按 model 验证能力 |
| ChatGPT/Codex | https://chatgpt.com/backend-api/codex/responses | OAuth Bearer + account header | 专属产品/dialect，不能当普通 API key Responses |
| DeepSeek | https://api.deepseek.com/chat/completions | API key Bearer | 当前 model 如 deepseek-flash、deepseek-v4-pro，能力逐模型 |
| DeepSeek | https://api.deepseek.com/responses | 同一 key，独立 route | stateless Responses，不继承 OpenAI/Codex 全部能力 |
| DeepSeek | https://api.deepseek.com/anthropic/v1/messages | 同一 key，XApiKey | Anthropic-compatible body，不是 Claude OAuth |
| Anthropic API | 原生 /v1/messages | XApiKey + anthropic-version | 复用 native adapter，不复用第三方订阅 token |
| Gemini API | models/...:streamGenerateContent | x-goog-api-key | 复用 native adapter，ADC 是另一认证类型 |
| 自定义兼容网关 | 显式前缀 + 已实现 wire | scoped key / 显式本地 none | 不猜模型/协议，不接受任意 raw JSON 绕过校验 |

OpenRouter、OpenCode Zen/Go、火山、百炼等能使用已有 wire 不代表其所有套餐/地域已验收；本轮未重查全部产品，不做内置“已支持”声明。
Vertex/Bedrock/Azure/Copilot/Claude OAuth、图像生成/STT/TTS/video 是独立产品/认证/协议工作包。保留未来意图，不以旧研究矩阵充当最新支持证明。

### 5.2 不需要第四个 RawMessages 协议

| 用户称呼 | 实际请求 | adapter |
|---|---|---|
| OpenAI messages | model + messages，Chat Completions | Chat |
| Anthropic messages | model + system + messages + max_tokens，内容/工具结构不同 | AnthropicMessages |
| Responses | model + instructions + input，不是 messages 换 URL | Responses |
| 最原始 messages / curl | 直接 HTTP 发送上述 Chat body，没有 SDK | 同一 Chat，不新增 RawMessages |

若“原始”指另一私有 schema，必须给出 endpoint/body/stream/error 规范和 fixture 才新增协议。
内置 DeepSeek Responses 只生成 https://api.deepseek.com/responses；/v1/responses 不属于内置别名，配置为内置路由时预网络拒绝。
确有兼容网关需要时使用显式 custom profile 和凭据目的地授权；不自动探测、改路径重试或切协议。本规则不宣称服务端一定拒绝该别名，只是不把缺证据的别名变成产品保证。
来源：[Responses reference](https://api-docs.deepseek.com/api/create-response/)、[Chat reference](https://api-docs.deepseek.com/api/create-chat-completion/)。

### 5.3 不是只改 base URL

- Responses 无状态，不依赖 previous_response_id/conversation/store；completed/incomplete/failed 是终态，没有 Chat 的 [DONE]。
- 不用服务端忽略的参数假装完成硬限制；unsupported builtin tools/encrypted reasoning/background 等在本地拒绝。instructions 保持系统语义，不转为该端点视作 user 的 developer。
- Chat 工具续接保留所需 reasoning_content 与成对 call ID；thinking/reasoning_effort/max_tokens 按此 route 处理，不把 Codex 删除字段规则套给 DeepSeek。
- Anthropic-compatible 不等于原生全语义；document/redacted_thinking 不默认允许；is_error/预算等被忽略时严格请求拒绝或经显式可见降级，不能静默丢语义。
- 不借 claude-* 别名假装实际调用 Claude；记录请求与返回 model 的区别，服务端未披露时不虚构实际映射。

来源：[Responses 指南](https://api-docs.deepseek.com/guides/responses_api/)、[Chat reference](https://api-docs.deepseek.com/api/create-chat-completion/)、[Anthropic 兼容表](https://api-docs.deepseek.com/guides/anthropic_api/)。
官方 [OMP 集成页](https://api-docs.deepseek.com/quick_start/agent_integrations/oh_my_pi/)仍有旧模型/能力例子；冲突以当前 API reference 和 fixture 核验，不机械复制最新 upstream catalog 或旧教程。

## 6. 多模态请求与历史

保留 Turn.content 和 CompletionRequest 兼容入口，不另造公共 Message/AgentEvent。
新增 crate-private 借用视图 ContentPart::{Text, Image, File}，在既有版本化 metadata 保存**有序**引用、角色和 tool 归属；旧文本读为一个 Text。
输入图片、工具返回图片、provider file ID 不能压平成提示文字。native reasoning/signature 是独立 opaque annotation，不混入普通内容。
Audio/Video 先明确 unsupported，不能把 bytes 当图片或因 provider 有别的产品便宣称聊天支持。

| route | 当前 zenpi 基线 | 目标 / 明确限制 |
|---|---|---|
| Chat | 图片，不通用支持 File | DeepSeek 图像型 file_id 单独能力，不宣称 PDF 支持 |
| Responses | image/file 编码基础 | 区分 input_image/input_file；DeepSeek document input 不允许 |
| Codex | 未接入 | 先文本/function tools；图片须 model+fixture 通过，上传/文件不默认启用 |
| Anthropic | 限定 MIME 的图片 bytes/URL，无 file ID | 保留 tool_result 内容；DeepSeek image file source 单独 beta 门控 |
| Gemini | 限定 MIME、已物化 image bytes | 不隐式抓 URL/upload；文件 URI/音视频另验收 |

DeepSeek 当前 deepseek-flash 支持图片及 Responses tool output 图片，不把视觉能力推给 v4-pro。
Files API 是图片上传，文件属于 API key，不是通用 PDF/音视频接口。见 [Vision](https://api-docs.deepseek.com/guides/vision/)、[Files](https://api-docs.deepseek.com/guides/files_api/)。
Pi/OMP 聊天 text/image 也不是任意多模态；Pi [transformMessages](https://github.com/earendil-works/pi/blob/4c8eb393c73220c742e75745df210335aedc020e/packages/ai/src/api/transform-messages.ts)的图片占位降级不默认照抄。

### 6.1 内容引用与恢复合同

1. journal 仅存版本化引用、MIME、长度、hash、所属 turn/tool、media scope；不存 base64 bytes/token/带签名参数的完整 URL。
2. 当前 turn 的工具续接使用已验证 bytes 快照；下一 turn/重启由现有 Agent::materialize_attachments -> ToolContext::read_attachment 在 host-fixed workspace 和权限内重读并校验 hash，再把 bytes 交协议层。缺失/改变/无权限返回 attachment_unavailable，不能用新文件悄悄替代。
3. 仅内存附件、敏感 URL 不保证重启重放；必须重附或显式批准无附件继续并保留缺失说明，不把摘要称为原图。
4. file ID 绑定 provider、credential identity 和存储命名空间；跨账号禁止。DeepSeek 同 key 跨其 API 家族共享图片须 provider 规则+测试许可，泛化共享默认拒绝。
5. reasoning/signature/response ID 采用更严格的 provider/protocol/model/connection/account scope；切换前先校验，不兼容新 session 或显式转换，不能删签名冒充兼容。
6. 首期不自动上传/下载、不建媒体数据库；用户提供 scoped file ID 与实现 Files CRUD 不同，未实现上传不能显示成功。
7. compact 保留引用/缺失状态；目前语义摘要 text-only，不叫图像重放。要求再次理解图片须显式加入获准内容并验收，否则拒绝。

沿用更保守的本地限制：每次请求（含历史）8 附件、单个 10 MiB、合计 20 MiB；新连接 JSON body 上限32 MiB，序列化前 checked arithmetic 计算 base64 膨胀。
超限拒绝，不能静默删历史图片；上下文压缩/新会话是独立显式动作。legacy 原限制保持，其迁移差异纳入 A01/A15。
MIME 不符/无效 base64/冲突 source/角色或工具图像不支持均在网络前拒绝。
URL 不带 key；如将来由 zenpi 拉取图片，须独立网络审批/SSRF 校验，不因多模态获得内网权限。

### 6.2 有序 DTO 与实际数据通路

不新造一套 Message/Turn；在现有 Turn.metadata 的 `zenpi_content_v1` 字段保存以下严格版本化 DTO。该字段存在即为协议内容权威，Turn.content 只作兼容显示，不再重复发送一遍文本。

```rust
struct StoredContentV1 {
    version: u8, // 固定1
    parts: Vec<StoredPart>,
}
enum StoredPart {
    Text { text: String },
    Image { media: MediaRef },
    File { media: MediaRef },
}
struct MediaRef {
    id: String,
    mime_type: String,
    filename: Option<String>,
    source: MediaSource,
}
enum MediaSource {
    WorkspaceFile { path: String, size_bytes: u64, sha256: String },
    ProviderFile { file_id: String, media_scope: MediaScope },
    Ephemeral { handle_id: String },
}
struct PreparedContent<'a> {
    turns: Vec<PreparedTurn<'a>>, // 借用原 turn ID/role/tool_call_id，不改 transcript
}
enum PreparedPart<'a> {
    Text(&'a str),
    Image(PreparedMedia<'a>),
    File(PreparedMedia<'a>),
}
enum PreparedMedia<'a> {
    Bytes { data: &'a [u8], mime_type: &'a str, filename: Option<&'a str> },
    HttpsUrl(&'a str),
    ProviderFile { file_id: &'a str, scope: &'a MediaScope },
}
```

MediaSource 用 serde tagged enum + deny_unknown_fields，恰好一个 source；字段不是可以同时设置的 path/url/file_id 三个 Option。
role/turn_id/tool_call_id 由外层 Turn/ToolResult 决定，附件不可自称 system 或伪造工具归属；tool call/result 必须唯一且成对。
每条内容最多256 parts，文本沿用入口/会话限制；路径/URL/file ID/MIME 沿用 InputAttachment 上限。PreparedTurn 持有有序 PreparedPart，不能按 text/images 分组重排。
Core 在会话写入前按已有 workspace/工具策略验证 WorkspaceFile 并记 hash；PreparedContent 只借用本次已经物化的 RequestAttachment bytes，不授权协议层再次读取路径。
Ephemeral handle 指向宿主内存的 bytes 或完整 HTTPS URL；journal 不保存 URL 秘密。重启后失效，恢复给 attachment_unavailable，不把 handle 当路径。
URL 交 provider 获取时只能验证语法/允许来源与声明 MIME，不能声称已检查远端 bytes/MIME/hash；URL 指向内容也可能变化，需要不可变证据的任务拒绝该来源。zenpi 不自行下载。
ProviderFile 由可信入口绑定当前获准 identity scope；从历史恢复必须逐项比较，用户输入不能靠自填 scope 绕过检查。

| 入口/阶段 | 必须修改的现有通路 |
|---|---|
| 旧 prompt text + attachments | 转为 Text 后接原顺序附件；不改变旧输入 wire |
| 新 headless prompt content | schema_version=2；按有序 parts 转 Turn；与 text/message/attachments 互斥，拒绝混填 |
| ToolResult::Success | 添加可选 typed content（默认 None）；output:Value 保留。Core 在 compact output 前提取、验证并物化 content；content 存在时是模型工具结果权威，output 仅兼容记录，不能再重复发送 |
| 旧 ToolResult / Error | 无 typed content 时仍发送原 output/error 文本；任意 output JSON 的 path/image 字段都不授予读文件权限 |
| compact / tool 续接 | compact 只处理显示文本/旧 output，不截断媒体引用、tool_call_id 或原始 content；摘要不能取代图像重放 |
| 恢复/跨协议 | 先验证 DTO 版本、角色、scope、权限、hash、能力，再 encode；未知版本/不可表示内容显式失败 |

新 headless 输入 parts 固定 `{type:"text",text}` 或 `{type:"image"|"file",mime_type,source:{type:"path"|"url"|"file_id",value}}`；不接收客户端指定的内部 handle/identity/hash。
该公共输入 DTO 命名 InputContentPart，放在既有 backend.rs 类型门面供 protocol.rs/ToolResult 复用；StoredContentV1/PreparedContent 留在 protocols/content.rs 内部，不能把 private 类型暴露在公共 ToolResult 字段。
每个对象 deny_unknown_fields，url 仅 HTTPS image，inline base64 首期不新增公共输入；已有已验证 bytes 仍可由宿主物化。
工具 typed content 使用同一输入 DTO，由注册工具的 ToolContext 校验；媒体输出不自动扩大工具权限。A14/A15 必须覆盖这条真实 Core 数据通路，不能只测 encoder。

## 7. 认证生命周期

### 7.1 浏览器和 headless device

首期支持 stored API key 和 zenpi 自己的 Codex OAuth。采纳 Pi 同 provider 两种登录方式，不创建两份 Codex 账号。
来源：[Pi Codex OAuth](https://github.com/earendil-works/pi/blob/4c8eb393c73220c742e75745df210335aedc020e/packages/ai/src/auth/oauth/openai-codex.ts)、[device polling](https://github.com/earendil-works/pi/blob/4c8eb393c73220c742e75745df210335aedc020e/packages/ai/src/auth/oauth/device-code.ts)。

```text
Browser: PKCE/state -> loopback -> 授权 URL -> callback/manual URI
  -> code exchange -> 校验 token/账号 -> 存 credential -> profile 引用
Device: usercode -> verification URI/code -> 有界轮询
  -> authorization code + verifier -> 同一 exchange/持久化
```

- PKCE 用 OS CSPRNG/SHA-256/base64url 无 padding；32 随机字节可得合法 43 字符 verifier；state 独立高熵、一次性匹配。
- 默认注册回调 http://localhost:1455/auth/callback，仅 loopback 监听，IPv4/IPv6 与 URI 匹配，不绑定 0.0.0.0。端口冲突提示 device，不杀占用者、不调用其 /cancel、不随机换未注册端口。
- callback 限定 method/path；错误 state 不消耗登录，匹配 state 的 OAuth error 终止；manual 必须完整匹配 URI+state，不接受裸 code；只有一个赢家。
- 浏览器/device 总期限 15 分钟，单次 token HTTP 15 秒并受更短 deadline/cancel 约束；device 尊重 interval/slow_down，Codex 特定 pending 不泛化到所有 provider。
- auth HTTP 复用受控 transport，响应最多 256 KiB；raw error/无效 JSON 不直接打印；JWT claims 仅解析必要身份/expiry，不当验签。
- scope/client/redirect/header 使用下表固定定义；服务资格为发布门，不伪造 originator/version/attestation。
- 不需要额外 API key exchange；OAuth 直接用于 Codex 专属 endpoint。

导入与登录分开：保留 legacy API key import-codex 幂等；OAuth 首期只识别并引导独立登录，不静默复制外部 refresh token。
Pi auth 文件/key command/export 与 OMP 的 [CLIProxyAPI import](https://github.com/can1357/oh-my-pi/blob/260eaa35fe3480ef1090641547fefde2ad879113/packages/coding-agent/src/cli/auth-broker-cli.ts)不是官方 Codex auth.json 的通用安全共享实现。
未来迁移必须转移唯一刷新所有权；外部宿主管理的 token 只进内存，刷新交宿主，不写其文件。
首期不实现任意 !command helper、不自动扫描第三方 auth、不运行 Codex 子进程取 token。采纳 Pi 代码行为不等于兼容其磁盘格式。

### 7.1.1 Codex 定义的字面协议合同

以下字段集中在 providers/codex.rs 的 `definition_version=1`，由 auth/codex.rs 消费；不是用户配置、项目参数或模型可覆盖的 OAuth discovery。

| 操作 | 固定合同 |
|---|---|
| authorize | GET https://auth.openai.com/oauth/authorize；response_type=code；client_id=app_EMoamEEZ73f0CkXaXp7hrann |
| scope / redirect | `openid profile email offline_access`；http://localhost:1455/auth/callback；S256 |
| authorize query | state/code_challenge；id_token_add_organizations=true；codex_cli_simplified_flow=true；originator=zenpi |
| code exchange | POST https://auth.openai.com/oauth/token；form grant_type=authorization_code、client_id、code、code_verifier、原 redirect_uri；无 client_secret |
| refresh | 同 token URL，form grant_type=refresh_token、refresh_token、client_id；不重新 authorize |
| device begin | POST https://auth.openai.com/api/accounts/deviceauth/usercode；JSON {client_id}；要求非空 device_auth_id/user_code，interval 为有效数字或数字字符串 |
| device 展示 | https://auth.openai.com/codex/device 与 user_code；device_auth_id 不展示 |
| device poll | POST https://auth.openai.com/api/accounts/deviceauth/token；JSON {device_auth_id,user_code}；成功必须有 authorization_code/code_verifier |
| device code exchange | 复用 code exchange，redirect_uri=https://auth.openai.com/deviceauth/callback |
| SSE | POST https://chatgpt.com/backend-api/codex/responses；Bearer、chatgpt-account-id、Content-Type:application/json、Accept:text/event-stream、OpenAI-Beta:responses=experimental |
| 标识 | originator:zenpi；真实 zenpi 版本/平台 User-Agent；session-id/x-client-request-id 可选非秘密关联 ID，禁止把 token 当 ID |

device 第一次立即 poll，间隔至少1秒；缺失/非有限/非正 interval 拒绝，slow_down 持久增加5秒；poll sleep 可取消并受15分钟总期限限制。
**仅 device poll** 的403/404、deviceauth_authorization_pending 表示 pending；begin 的404为不可用，推理403为权限失败，不混用分类。
error 字段允许字符串或 `{code}`，只映射白名单稳定错误，不直接转发上游说明/正文；其余非成功/不合法响应终止。

初次 exchange 要求非空 access_token/refresh_token 和有限正 expires_in；以毫秒向下取整并 checked arithmetic，不能得到零/溢出期限。
若 access JWT 存在合法 exp，使用更早期限；无可用期限拒绝，不用8天兜底。access token 的 `https://api.openai.com/auth.chatgpt_account_id` 必填。
JWT 解码只读必要 claims，不把未验签值当授权证明；可信度来自固定 HTTPS token endpoint，身份仍须和已有记录比较。
refresh 必须匹配 issuer/client/account，已有 user ID 时也匹配；缺少原来必需的身份或换账号拒绝，不自动改 identity scope。
refresh 响应缺少新 refresh_token 时保留旧值是 zenpi 的有意增强，不声称逐字复制 Pi。

表中 headers 是固定上游的兼容集合，不是 live 实验证实的最小必需集合。官方确认 browser/device 与 API key 模式，但第三方使用 public client 的资格、zenpi originator、区域/套餐/device 开关要在 P6 验收。
无资格时明确 unsupported/login_failed，禁止回退账号/API key/别的 endpoint。[官方认证说明](https://learn.chatgpt.com/docs/auth)、[device 登录合同](https://learn.chatgpt.com/docs/app-server#3b-log-in-with-chatgpt-device-code-flow)。

### 7.1.2 登录任务与交互

```rust
enum LoginState {
    Preparing, AwaitingAuthorization, Exchanging, Committing,
    Succeeded, Cancelled, Failed, CommitUncertain,
}
enum AuthInteraction {
    Notify(LoginNotice),
    Prompt { flow_id: LoginFlowId, prompt_id: u64, prompt: AuthPrompt },
    CancelPrompt { flow_id: LoginFlowId, prompt_id: u64 },
}
fn begin_login(request: LoginRequest, control: &mut LoginControl<'_>) -> LoginOutcome;
fn cancel_login(flow_id: LoginFlowId) -> CancelDisposition;
fn exchange_code(flow: &PendingFlow, control: &mut LoginControl<'_>) -> Result<PendingCredential, AuthError>;
fn refresh_token(old: &PrivateCredential, control: &mut RequestControl<'_>) -> Result<PendingCredential, AuthError>;
fn to_request_auth(saved: &CommittedCredential, scope: &RequestScope) -> Result<AuthContext, AuthError>;
```

LoginControl 含取消、deadline、独立发送预算和私有 interaction 回调，不是模型 ProviderEvent；AuthPrompt 是浏览器 URL/manual URI 输入或 device challenge，不 derive 通用 Serialize/Debug。
取消某个 prompt 只结束该输入等待；用户选择“取消登录”取消整个 flow。callback/manual 先成功者发 CancelPrompt 收起另一输入，迟到结果必须匹配 flow_id/prompt_id，不能影响后来登录。
同一宿主最多一个活跃登录流程，bootstrap/生产 TUI/CLI 复用同一任务状态机；不新建通用任务调度框架。
PendingCredential 私有且不能注头；store 成功提交并回读得到 CommittedCredential 后，纯 to_request_auth 才能发放 SecretHandle。
取消在提交前胜出则不写 credential；持久化已确定成功即报告 Succeeded（即使取消稍晚），不假报 Cancelled。rename/fsync 结果未知为 CommitUncertain，只做存储核对，不再次交换 code/发起登录。
credential 已成功而 profile 写失败返回成功凭据 + binding_failed，可按 ID 修复；这不是认证失败，也不自动切换当前连接。

### 7.2 存储、权限和事务

用现有用户私有 JSON，不引入数据库或 keyring 插件层；legacy 根字段保留，新数据放 zenpi_auth_v1。
profile alias 与 credential ID 分开；身份以 issuer/provider + account/workspace 为准，email 仅显示。

```json
{
  "zenpi_auth_v1": {
    "version": 1,
    "revision": 1,
    "accounts": {
      "cred_example": {
        "kind": "oauth",
        "provider": "openai-codex",
        "definition_version": 1,
        "issuer": "https://auth.openai.com",
        "client_id": "app_EMoamEEZ73f0CkXaXp7hrann",
        "identity_generation": "synthetic-identity-generation",
        "revision": 1,
        "state": "active",
        "account_id": "example-account",
        "access_token": "<secret-only-in-auth-file>",
        "refresh_token": "<secret-only-in-auth-file>",
        "expires_at_ms": 0
      }
    }
  }
}
```

示例已过期。private DTO 不 derive Debug，不向事件公开 Serialize；api_key 类型使用独立 key/允许目的地，不能把 access_token 当 key。
除非必需不长期存 id_token。unknown version/重复 JSON key/坏 JSON 不能当空库覆盖。
上限：文件 4 MiB、128 账号、单 token 16 KiB，在读取/分配前限制。

Unix 目录 0700，凭据/临时/稳定锁文件 0600；拒绝 symlink、非普通文件、不可信 owner/父目录，以 no-follow/目录 FD 防检查后替换。
private temp -> sync -> rename -> parent fsync；复用 atomic_write_if_changed 思路但补锁/revision，单独 rename 不等于跨进程事务。
auth 与 config 非单事务：先存 credential 后写 profile；后者失败留下可诊断未绑定账号，用 ID/revision 修复，不重复登录。
新格式只显式登录/迁移写入，先停止旧 binary；旧 deny_unknown_fields/写回行为不兼容新格式。Windows ACL/锁未验收前 OAuth 持久化 fail closed。

store 只用一个具体实现，接口语义固定如下；private DTO 仅允许经审核的专用存储序列化，不复用日志 JSON 序列化。

| 操作 | 合同 |
|---|---|
| read(id) | 有界读取/校验版本，返回私有快照或 NotFound/明确错误；不刷新、不隐式创建 |
| list_status() | 只返回脱敏 ID/provider/状态/到期/绑定 profile；零网络，不执行命令 |
| modify(id, expected_revision, mutation) | 锁内重读，CAS；mutation 为 Keep/Replace/Revoke。Keep 不写不增版；Replace 成功增账号及文件 revision；Revoke 清秘密、保留 tombstone 并增版 |
| 提交结果 | 返回持久化后的 CommittedCredential/撤销回执；CAS 冲突为 Conflict；落盘确定失败为 StorageFailed；结果不明为 CommitUncertain；没有 best-effort success |
| replace identity | 新 key 或重新登录生成新 identity_generation；允许目的地修改不改变身份但增 credential revision；同账号 refresh 保留 identity_generation |
| 刷新状态 | active / refresh_in_flight / refresh_uncertain / login_required / revoked；InFlight 带 attempt_id、开始时间、原 identity_generation、准备刷新前 revision |

所有这些是内部存储状态；UI ready/expired/needs_model 等由非秘密快照与当前时钟投影，不将状态查询变成认证修复。
auth.json 和配置文件不能原子提交，因此管理回执必须分别携带 credential_committed/profile_bound，不用单个模糊 success 掩盖部分成功。

### 7.3 刷新并发与不确定结果

```text
resolve -> 检查 destination/type/revocation/expiry
  有效 -> 短时 SecretHandle（expiry + 原 policy + route）
  将过期 -> credential OS refresh lock（可取消）
    -> 短事务锁二次读；peer 已刷新则使用它
    -> 持久化 RefreshInFlight(attempt_id, expected_revision)
    -> 释放全局锁，持账号锁做一次 token HTTP
    -> 全局短锁 CAS 提交 token/expiry/revision，清 marker
    -> 释放账号锁，发放已持久化的凭据
```

借鉴 Pi 5 分钟 skew/15 秒 refresh timeout；锁顺序固定 credential -> global file transaction，网络不占全局文件锁。
注入测试时钟；expires_at <= now+5分钟触发一次预刷新，但不要求新 token 必须再活5分钟。新 token 必须在发放时尚有效，并满足调用者显式 minimum_validity；否则 auth_login_required，不在同一 resolve 内循环刷新。
墙钟计算 OAuth expiry，锁/HTTP/总期限用单调时钟；status/list 不刷新，等待 peer 后重新读 revision/expiry/撤销状态。
OS lock 用稳定 inode，不靠 mtime/unlink 解锁；进程退出由 OS 释放。
login replace/logout 在全局事务按 expected_revision 更新，在途刷新 CAS 失败不能覆盖新登录或复活撤销。
删除保留防 ABA 的 revision/tombstone，不删后用同 ID 从版本 1 重建；不保留多份 refresh token 备份。

RefreshInFlight 只是正确性标记，不是新队列。崩溃/超时可能发生在服务端轮换后，结果未知变 RefreshUncertain，不重复发旧 refresh token。
marker 的 temp sync/rename/parent fsync 全部成功后才能打开 token socket；提交 CAS 同时匹配 marker 后的 credential revision、attempt_id、identity_generation。
只有 transport 能证明请求未发送时才可安全清 marker；收到有效结果但落盘失败不发放内存 token，不重发旧 refresh token。写 uncertain 也失败时保留的 InFlight 本身 fail closed。
发现 peer 已成功提交则用新版本，否则要求重新登录。明确 invalid_grant、refresh_token_expired/reused/invalidated 只使原 CAS 版本 login_required，不误禁用 peer 新凭据。
本地 revoke 先持久化 tombstone，再撤销句柄并尽力取消活跃请求；首期不实现服务端 revoke，不能报告远端吊销/停止/退款成功。
每次 HTTP 的最后认证预检重新读取 revision/撤销状态，其他进程注销后不能一直沿用缓存。预检通过后与撤销竞态的请求视为在途，仅尽力取消，不承诺瞬时跨进程终止所有 socket。
多个 profile 共用凭据时先显示影响范围；从 profile 解绑与撤销整个 credential 不同。

## 8. Codex wire、重试和准入

### 8.1 共享 Responses 实现中的 Codex dialect

| 项目 | 首期合同 |
|---|---|
| endpoint/headers | 完整字面集合见 7.1.1；固定 ChatGPT origin/path，Bearer + account + SSE/beta/真实客户端标识；禁 header override/redirect |
| body | stream=true、store=false，full-history instructions/input；不植入 OMP 提示词 |
| output cap | 不发送不支持的 max_output_tokens/max_completion_tokens；要求远端 hard cap 时拒绝，不能偷偷删字段后假称限制生效 |
| tools | function call/result、稳定合法 call ID；不补造孤立工具结果 |
| reasoning | model/route 允许的 effort；encrypted_content 按 scope 保存，不转普通文本 |
| 不包含 | append/previous_response_id、WebSocket、压缩、native compact、custom/computer tools |

主来源是 Pi [API adapter](https://github.com/earendil-works/pi/blob/4c8eb393c73220c742e75745df210335aedc020e/packages/ai/src/api/openai-codex-responses.ts)，OMP [request transformer](https://github.com/can1357/oh-my-pi/blob/260eaa35fe3480ef1090641547fefde2ad879113/packages/ai/src/providers/openai-codex/request-transformer.ts)只补边界案例。
合法短 call ID 保留；其他用版本化安全前缀+SHA-256，最长 64 字符，call/result 共用映射并检测碰撞；原 session ID 不变。

SSE 覆盖跨 read UTF-8、CRLF、多行 data、当前兼容 padding；帧/工具参数和总响应有界（沿用 4 MiB）。
delta 必须在终态前可见；只有成功终态、完整参数/schema 及 Core 校验后才能执行工具。
failed/incomplete/EOF/冲突终态不能算成功；sink 取消立即停止事件、关闭/join，错误终态仍由既有宿主唯一投影。
opaque history 仅成功且 Session 提交后保存；digest 是一致性检查，不是防同 UID 篡改的签名。

### 8.2 唯一重试所有者

Backend 最多发送 1 + max_retries 次推理 HTTP；401 修复最多一次且占同一推理预算，不叠加循环。token HTTP 另计物理网络预算，不能隐身在这一次数中。
先 guarded reload，同账号已有新 token 则用它，否则一次 refresh；之后由 Backend 决定是否重发。
任何 ProviderEvent 已交付后禁止自动重放，即便仅 created。API key 401 不刷新，403/权限/地域/额度不刷新或换号。
max_retries=0 可预刷新，不在响应 401 后重发。semantic compaction 继续无自动重试。
429/5xx/网络错误仅既有策略、无事件、同 route/account、deadline/预算允许时有界重试；无输出不证明远端未执行。

### 8.3 每次物理 HTTP 准入

```rust
pub enum RequestEvent {
    BeforeHttpSend { kind: HttpSendKind, attempt_id: String },
    Provider(ProviderEvent),
}
pub enum HttpSendKind { Inference, TokenRefresh }
pub struct RequestControl<'a> {
    pub scope: &'a RequestScope,
    pub cancelled: &'a dyn Fn() -> bool,
    pub deadline: std::time::Instant,
    pub dispatch: &'a mut dyn FnMut(RequestEvent) -> Result<(), BackendError>,
}
// Backend trait 新方法的形状；所有公开签名涉及的类型也必须公开，但不含秘密。
fn complete_with_request_control(
    &self,
    request: CompletionRequest<'_>,
    control: &mut RequestControl<'_>,
) -> Result<Completion, BackendError>;
```

一个 dispatch 同步回调串行传发送准入与 provider 事件，避免两个闭包同时可变借用 Session owner；BeforeHttpSend 不写成模型 ProviderEvent，不触发“已交付内容禁止重放”。
共用 control，但必须分两条顺序，不能要求 refresh 在取得新凭据后才准入：

- TokenRefresh：marker 持久化 -> 核对 attempt/revision/identity、取消与期限 -> dispatch(BeforeHttpSend(TokenRefresh)) -> 固定 token endpoint HTTP -> CAS 提交。不发放 refresh token 的公共 SecretHandle。
- Inference：取得已提交凭据/SecretHandle -> 最终取消/身份/撤销/期限预检 -> dispatch(BeforeHttpSend(Inference)) -> 注头发送。

两者均经既有 Governance/Session owner，拒绝则不开 socket；刷新若尚未发送可按7.3清 marker。旧外层逻辑网络计数移入此钩子，不双重扣费。
refresh 计网络次数、不计模型轮次/input tokens；锁等待和 peer 复用不计 send。独立登录 CLI 使用自身有界网络预算。
正常模型/刷新次数 1/0，预刷新 1/1，401 修复可 2/1；未知远端结果不自动退额。
保留旧 Backend 方法供既有调用方；生产所有请求改用新方法，不能把旧 complete_with_control 的调用包装成虚假 per-send 准入。
新 trait 方法默认返回 backend_configuration/controlled_request_unsupported，不静默执行无 control 的 backend；Echo 显式实现零 HTTP 事件路径，真实 HTTP backend 实现全部 send hook，更新所有受治理测试替身。
RequestScope 为公开非秘密类型、字段/构造受可信宿主约束，由 control 携带；backend 必须与已验证 route/执行 owner 比较，不因调用者填了摘要就信任。
不让 control 回调临时选择当前项目凭据；一个操作不能因 refresh 换 scope。
不新建 scheduler/账本；认证线程不能直接写 SessionStore。取消覆盖 callback/device sleep/锁等待/HTTP/backoff。

错误至少区分 auth_login_required、auth_revoked、auth_mode_mismatch、auth_destination_denied、auth_refresh_uncertain、auth_storage_failed、auth_lock_timeout、provider_unsupported_capability、provider_permission_denied、provider_usage_limit。
保留已有 BackendError code；不靠显示文案分类。HTTP 错误限长/白名单/脱敏，request ID 去控制字符；debug 不绕过保护。

## 9. TUI、CLI 与 headless

### 9.1 管理命令

以下新命令实施后才可使用；已有正确启动形式是 zenpi --profile codex，不是 zenpi profile codex。

```text
zenpi config auth list [--json]
zenpi config add auth codex [email] [--alias NAME] [--model MODEL] [--device] [--no-browser]
zenpi config add auth apikey <base_url> <provider> --stdin [--alias NAME] [--model MODEL]
zenpi config doctor --profile codex [--json]
zenpi pair revoke --profile codex --yes
zenpi config use codex
zenpi --profile codex
```

key 从 stdin 读，不推广 argv 明文；email 仅提示，不代表身份。冲突不覆盖，replace 需显式确认/expected_credential_revision。
登录不自动切换会话或已有 default_profile；config use 只改后续启动默认。
list/doctor 本地只读，不 refresh/probe/key command；状态区分 ready/expired/revoked/unconfigured/uncertain。
pair revoke --profile 解析该 profile 的整个 credential，先列所有受影响 profile；--yes 表示确认该范围，不是只解绑这一 profile。成功回执写 local_revoked，remote_revoked=false；解绑不是此命令的同义词。
授权 URL/device code 只交给发起 CLI 的 stderr/终端或 TUI 私有界面，不进 journal/通用日志/status JSON；stdout 不混人类提示。

### 9.2 TUI 首次登录与管理

入口：/auth 看连接账号，/login [provider] 登录，/profile <name> 换整连接，/model 选当前连接内模型。
选择器展示 profile/provider/model/wire/脱敏账号；Agent 工具权限仍独立展示。
未认证时只进入 tui/bootstrap.rs 的 run_auth_bootstrap：开放登录/选连接/退出，拒绝 prompt/shell/项目恢复，不启动 Agent 或资源/LAN/Gantt runner。
只捕获明确未配置/需登录，不将坏 TOML、权限或 session 错误伪装为未登录。
bootstrap 释放 TerminalGuard 后走一次 prepare_initial_agent；不把整个 Core 改成 Option<Backend>，不复用暴露广泛 slash 操作的通用 run_with_state。
已有会话登录由受控后台任务驱动同一 auth 函数；取消回收 listener/轮询。成功不自动切正在运行的请求，撤销先确认影响并收束相应执行。

### 9.3 原子选择与恢复

```text
ConnectionSelect(owner_id, session_id, profile, model?, expected_selection_revision)
  -> owner + host 队列栅栏：idle，无 pending input/attachment/approval/recovery
  -> prepare：构造候选，校验 model/reasoning/history/权限/预算/zone override
  -> Session append 一条版本化 model_selected 完整选择快照
  -> infallible swap backend/model/project_overrides/connection
  -> Applied；TUI 才更新菜单和 profile-scoped layout
```

不重建整个 Agent，保留 session/input port/审批/资源/预算 owner。仅 phase==Idle 不足以覆盖 host 已排队未取锁任务。
selection revision 可复用该 session 最后一次选择事件的序号；它不同于配置 revision 和 credential revision。
候选准备结束后在 owner 锁及队列栅栏内再次核对 owner/session/selection revision；OAuth refresh 不使选择命令无故过期，旧会话 UI 回执不能应用到新会话。
拒绝保留旧 UI/backend/选择；pending 输入/附件默认拒绝切换，用户先处理。同 profile /model 也重算 model_routes。
扩展现有 model_selected，而不是写两个可能半提交的事件：保留 model/descriptor/digest/reasoning，新增 selection_version/profile/provider/protocol/auth_kind/credential_ref/identity_scope/配置摘要。
journal 不存 token/headers，恢复从用户配置重新校验 endpoint，不直接信任日志中的 URL。
恢复先选连接再恢复 model；显式 CLI 连接选择优先，但与保存 scope 冲突必须可见并做历史检查，不静默跟随 default_profile。
更新 selected_model_for_session 及新会话复制选择逻辑；旧事件无连接字段仍走旧行为，旧 binary 忽略新字段不等于安全降级。
journal 结果不确定则冻结此 owner 的管理写操作和全部新推理，先恢复核对，不当确定失败重做。
headless 发起的 model_selected 还保存 request_id/request_fingerprint/previous_selection_revision；fingerprint 为规范化非秘密命令、project/owner/session/目标 profile/model/expected revision 的摘要，不含 token。
Session 选择事件先持久化，再更新 ReplayState terminal cache；崩溃后先恢复选择，按事件调和已预留 request ID 并返回原 Applied，不再次切换。相同 ID 不同 fingerprint 拒绝。
事件落盘结果不明先验证完整记录/序号；确定未提交才可返回 Rejected，无法证明则保持 CommitUncertain。恢复所需配置改变/凭据丢失进入 blocked，不自动使用默认连接。

切换只作用于指定当前 owner，不广播给已有 Arch/其他项目/worker。
Arch 是独立 Agent/session，项目池缓存 overrides；新 owner 需明确继承当前选定连接还是用户默认，不能偶然沿用陈旧 pool 快照。
默认新项目继承创建动作所属 owner 的连接快照，已有项目恢复自己的已保存选择；全局 default 的修改另用 config use。
当前 owner 的未收束子任务期间拒绝切换；认证切换不改变 worker policy/lease。zone 仅偏好时明确标识，不声称已是独立路由。

### 9.4 Headless

运行前可独立 CLI --device 登录；缺凭据/失效 fail closed，稳定错误/退出状态，不自动开浏览器等待粘贴。
新增 ConnectionSelect/ConnectionStatus typed 控制复用同 owner，带 request ID、owner/session ID 和 expected_selection_revision，返回 Applied/Rejected/CommitUncertain；不能将控制当 prompt。
不接收明文 token/password 的普通 JSONL；外部宿主私密凭据通道属后续独立工作。
生产 binary 验证 startup、工具续接、compact、resume、取消和选择，不只单测 encoder。

### 9.4.1 JSONL 合同

沿用现有 StdioRequest -> Command -> StdioResponse，协议版本仍为2；在既有 status 响应新增 capabilities 数组声明 connection_v1/ordered_content_v1，不新增自发 startup 握手。
客户端未见 capability 不发送新动作；旧服务端按既有 unknown command 拒绝，不能猜测支持，不创建第三个 RPC 协议。
connection 请求只允许既有 envelope 的 type/schema_version/id/project_id/session_id 及 connection；body/action 字段严格校验，不能混 prompt/approval/tree。

```json
{
  "schema_version": 2,
  "type": "connection",
  "id": "select-17",
  "project_id": "project-a",
  "session_id": "session-a",
  "connection": {
    "action": "select",
    "owner": "discussion",
    "profile": "deepseek-responses",
    "model": "deepseek-flash",
    "expected_selection_revision": 12
  }
}
```

owner 必须为 discussion 或 arch，不接受任意外部 owner_id；project_id 沿用宿主项目解析，省略时在 admission 固定当时项目，不能执行时再次读 active project。
select 要求 schema_version=2、非空 id/session_id/profile 和非负 u64 expected_selection_revision；model 可省略，取目标 profile 的有效 model，无 model 拒绝。
status 同样要求 v2/id/owner，允许 session_id 作为只读栅栏；禁止 profile/model/expected_selection_revision。返回具体 project/owner/session，绝不刷新。
合法 ID/文本/行大小继续用既有协议限制。旧 schema_version=1 的 prompt/status 语义不变，但不支持 connection 新动作。

```json
{
  "schema_version": 2,
  "type": "response",
  "id": "select-17",
  "command": "connection",
  "success": true,
  "code": "ok",
  "data": {
    "outcome": "applied",
    "project_id": "project-a",
    "owner": "discussion",
    "session_id": "session-a",
    "selection_revision": 18,
    "profile": "deepseek-responses",
    "provider": "deepseek",
    "model": "deepseek-flash",
    "protocol": "responses"
  }
}
```

保留 StdioResponse 的 data/error 互斥，不为失败同时塞 data.outcome。内部 ConnectionOutcome 有 Applied/Rejected/CommitUncertain，wire 映射如下：

| 结果 | success / code / payload |
|---|---|
| Applied 或重复 ID 的已提交结果 | true / ok / data.outcome=applied 及原提交快照 |
| Status | true / ok / data.outcome=status，当前选择/脱敏 auth 状态/blocked reason/selection revision |
| Rejected | false / typed code（connection_busy、connection_stale、connection_invalid 或原 auth/provider 错误）/ error 为脱敏说明，无 data |
| CommitUncertain | false / connection_commit_uncertain / error；owner 阻塞新推理，重试相同 ID 只核对，不重做 |

客户端失败后通过 status 获取现状，不从错误文字猜实际账号。终态仅一条 response，过程更新如需发出复用现有 StdioEvent，不将登录 URL/device code 放入通用事件。
新 prompt.content 的校验、媒体授权和恢复见6.2；connection 命令只管理非秘密引用，不开放普通 JSONL 写 token 的后门。

## 10. 复用与转换清单

路径相对第 1.1 节固定仓库。每个实现 PR 标 C-ID、保留语义、有意差异和 fixture；不 vendor 整仓。

| ID | 当前来源 / 符号 | Rust 落点 | 处置 |
|---|---|---|---|
| C01 | Pi packages/ai/src/auth/types.ts：CredentialStore/OAuthAuth/AuthInteraction | auth/mod.rs, store.rs | 转小契约，不增单实现 trait，多 profile 用 ID |
| C02 | Pi packages/ai/src/auth/resolve.ts：resolveProviderAuth/resolveStoredOAuth | auth/resolve.rs | port 生命周期/二次检查，保留 legacy key 优先级 |
| C03 | Pi packages/coding-agent/src/core/auth-storage.ts：AuthStorage | auth/store.rs | 行为移植，重写原子事务/OS 锁/revision/不确定结果 |
| C04 | Pi packages/ai/src/auth/oauth/openai-codex.ts：createAuthorizationFlow/exchangeAuthorizationCode/refreshAccessToken | auth/codex.rs | TS -> Rust，最小 scopes，现有 HTTP/crypto 依赖 |
| C05 | 同文件 startLocalOAuthServer/loginOpenAICodex；OMP packages/ai/src/registry/oauth/callback-server.ts | auth/callback.rs | port state/cancel；补 loopback/总期限/严格 manual URI/限长 parser |
| C06 | Pi packages/ai/src/auth/oauth/device-code.ts：pollOAuthDeviceCodeFlow；Codex 参数 | auth/codex.rs | port pending/slow_down/deadline/二段交换 |
| C07 | Pi credentialsFromToken/openaiCodexOAuth.toAuth | auth/codex.rs, resolve.rs | typed token/身份，不照抄 raw JSON 错误、不当 JWT 验签 |
| C08 | Pi packages/ai/src/models.ts、providers/openai-codex.ts；OMP catalog Model | providers/*.rs | 一 provider 一定义文件，保留 zenpi registry |
| C09 | Pi packages/ai/src/api/openai-codex-responses.ts：buildRequestBody/buildBaseCodexHeaders/buildSSEHeaders | protocols/responses.rs、providers/codex.rs | 只 SSE，共享 parser；auth 注入留给受控发送边界 |
| C10 | Pi packages/ai/src/api/transform-messages.ts；OMP Codex request-transformer | protocols/content.rs、各 provider 规则 | 请求时转换，不改 transcript、不补造结果/丢图 |
| C11 | OMP packages/ai/src/auth-storage.ts、auth/sqlite-credential-store.ts 及 race tests | auth/store.rs + fixtures | 只取 CAS/peer rotation/身份规则，不搬 SQLite/lease 服务/排名 |
| C12 | 本机 Codex login/auth manager、model-provider auth/bearer_auth_provider | auth/resolve.rs、backend::apply_auth | 取身份守卫/短时请求认证，不搬 App Server/Agent Identity/PAT |
| C13 | zenpi Chat/Responses/Anthropic/Google | protocols/*.rs | 必要迁移与共享；保持原测试，不重写已可用协议 |
| C14 | DeepSeek 当前官方协议/兼容表 | providers/deepseek.rs、protocols + fixtures | typed 规则；catalog 不是协议事实来源 |
| C15 | zenpi core/config/session/slash/TUI/headless/security/transport | 现有模块 + bootstrap | 必须新接线：原子选择、准入、expiry、恢复/附件引用 |
| C16 | OMP KDL/discovery/broker、Pi/OMP advanced transport/其他产品模态 | 无首期目标 | 暂缓，不先建空目录 |
| C17 | DeepSeek Harness LlmRuntime.prepareCall、credentials-local.modifyRecord 及 request-freeze/protocol/records tests | connection/auth store/现有运行时测试 | 取配置冻结、事务和隔离用例，不搬 Cordis/Node/Loop/SDK |

依赖取舍：Cargo.lock 已含 getrandom/url/httparse 的传递依赖，直接使用时声明兼容版本并核验 MSRV 1.88/lockfile。
不手写密码/URL/HTTP parser；Unix 锁优先已有 libc 或现有安全封装；不为 auth 引入 Tokio/Node/全量 server 框架。
SecretHandle 补 expiry/scope，不新增公开 String token getter、不承诺密码学级内存擦除。

### 10.1 许可与更新

Pi/OMP 实质代码移植保留各自 MIT 版权/许可，目标文件注明固定 SHA/修改范围。
复制 Codex 独立 helper 保留 Apache-2.0 和适用 NOTICE；转换语言不消除这些义务。
来源：[Pi LICENSE](https://github.com/earendil-works/pi/blob/4c8eb393c73220c742e75745df210335aedc020e/LICENSE)、[OMP ai LICENSE](https://github.com/can1357/oh-my-pi/blob/260eaa35fe3480ef1090641547fefde2ad879113/packages/ai/LICENSE)、[Codex LICENSE](https://github.com/openai/codex/blob/578c1b2230288104041e880a86d0f7f3a5ca6e47/LICENSE)、[Codex NOTICE](https://github.com/openai/codex/blob/578c1b2230288104041e880a86d0f7f3a5ca6e47/NOTICE)。
本 PR 无上游实现复制。实施前记录 upstream diff，变化要重新审 C-ID/测试，不用浮动 main 链接充当输入版本。软件许可不等于 OAuth 服务接入许可。

## 11. 实施、冲突与回退

各阶段均未开始；文档批准不自动派 worker/cron/Broker 或操作真实凭据。

| 包 | 依赖 | 工作 / 退出门槛 |
|---|---|---|
| P0 | 实现另行授权 | 使用本定稿输入，核对许可/client 条件、旧行为基线、备份策略，不碰真实 token |
| P1 | P0 | provider 定义/共享协议目录、连接/header/能力；四协议回归和默认值/key 优先级不变 |
| P2 | P1 | Pi auth 转换、store/锁/SecretHandle、CLI browser/device；本地 fixtures |
| P3 | P1/P2 | Codex SSE、DeepSeek 三 wire、每 send 准入/单一重试，续接/摘要正确 |
| P4 | P3 | TUI bootstrap、owner 选择、headless、journal 恢复；真实 binary 离线验收 |
| P5 | P3/P4 | 多模态有序引用/工具图像/跨 turn/恢复；不支持时零网络拒绝 |
| P6 | P1-P5 | 用户另行授权真实登录/推理/刷新/注销/安装测试，独立 live receipt |

| 冲突 | 决策 |
|---|---|
| 旧提案“导入 OAuth” | 改为识别+独立登录；显式转移/外部 owner 另验收，两份 refresh token 不叫安全共享 |
| 旧“别名即账号” | profile 是连接别名，credential ID 是凭据引用，多 profile 可共享但不合并权限 |
| 自由 base URL 与 OAuth | 普通 scoped key 可配置，Codex OAuth 固定目的地，冲突零网络拒绝 |
| 未来 Broker 禁客户端直连 | 本文为现有 direct-client 基础阶段；未来显式 Broker 将 auth/transport owner 移入服务，不双写、不服务断开回退直连 |
| native history 与换 model/wire/account | 先验证再提交，不兼容拒绝/新会话，不删签名冒充兼容 |
| 新配置/journal 与旧 binary | 新数据显式写；旧读写新格式不承诺，升级停旧进程 |
| 官方有协议文档但未现场调用 | 标目标/文档支持，不标 live accepted；冲突记录并 fixture 验证 |

各包独立提交，未迁移 legacy profile 继续旧行为。
代码逐提交可回退；数据回退先停进程，只恢复合适旧配置/API key，**不能用旧 OAuth 备份覆盖已轮换 refresh token**。
本地注销不等于上游吊销；未知提交结果先恢复核对，不重复登录/推理掩盖。

## 12. 唯一验收矩阵

以下全部未验收。新测试名为拟议，文件存在且执行后才能报告通过。

| ID | 要证明的行为 | 目标测试 |
|---|---|---|
| A01 | legacy CLI/env/config、默认 Chat/Responses、unknown field、OAuth 不被 key 偷换 | config + auth_config |
| A02 | 同模型三 profile/三 wire，精确路由，重复/歧义 URL 拒绝；热更新不混用不同代次 endpoint/身份/secret | provider_connections |
| A03 | 四类 header 正确，OAuth redirect/跨 host/header override 零网络拒绝 | auth_destination |
| A04 | PKCE/state、错误 state 不消耗、OAuth error、callback/manual 单赢家 | auth_codex |
| A05 | loopback/双栈/端口冲突/no-browser/cancel/deadline，无残留 listener | auth_codex |
| A06 | 固定 client/URL/query/form、device 首次 poll/interval/slow_down/403/404 endpoint 分类、二段交换 | auth_codex |
| A07 | 5 分钟边界、checked expiry/身份、短于5分钟的新 token、保留未轮换 refresh token、status 零网络/日志无秘密 | auth_resolve |
| A08 | 两进程同账号只刷新一次，不同账号不被全局网络锁串死 | auth_store |
| A09 | refresh vs login/logout、CAS loss/ABA/peer rotation、多 profile revoke | auth_store |
| A10 | 坏 JSON/重复 key/权限/symlink/磁盘满/崩溃、marker fsync 先于 socket、已轮换落盘失败不发放 RAM token、不确定不重发 | auth_store |
| A11 | Codex body/header/call ID/encrypted history/hard cap，不补造结果 | provider_codex |
| A12 | SSE UTF-8/CRLF/multiline/delta 时序，EOF/failed/incomplete/冲突不可成功 | provider_codex + stage1_chat_stream |
| A13 | DeepSeek 三 endpoint/body/terminal/tools/reasoning，unsupported 语义拒绝 | provider_deepseek |
| A14 | 有序 headless/tool DTO 到 Core/metadata/encoder、无重复文本、compact 不截断媒体、全历史大小/角色/能力交集、audio/video 拒绝 | provider_multimodal |
| A15 | 跨 turn/restart 的 ToolContext 权限/hash/缺失、换 key identity generation、跨账号 file ID/native scope | provider_multimodal + session_recovery |
| A16 | 首 ProviderEvent 后不重试、401 最多一次、403/额度不刷新、send 预算/取消、受控 trait 默认拒绝/Echo 零 HTTP | auth_retry + governance |
| A17 | TUI bootstrap 无任务副作用、flow/prompt 迟到结果、取消对持久化边界、binding_failed、UI Applied 后更新 | auth_host + stage1_host |
| A18 | headless schema/envelope/data-error 互斥、capability、幂等选择、selection 与 credential revision 隔离、跨 session/host queued 栅栏 | auth_host + headless_protocol |
| A19 | 冻结 scope 的续接/compact/resume、选择事件到 replay cache 的崩溃恢复、不确定时拒绝新推理、Arch/项目/子任务隔离 | auth_host + stage1_semantic_compaction |
| A20 | 用户另行授权 Codex/DeepSeek 真调用、刷新/logout/relogin，记录版本/端点/模型 | 人工 live receipt，默认 CI 不运行 |

上游测试输入（已阅读、未运行）：Pi packages/ai/test/models-runtime.test.ts、packages/coding-agent/test/auth-storage.test.ts、
packages/ai/test/{openai-codex-oauth,oauth-device-code,openai-codex-stream,openai-responses-tool-result-images,cross-provider-handoff}.test.ts；
OMP packages/ai/test/{auth-storage-oauth-refresh-race,auth-storage-codex-workspace-identity}.test.ts；
本机 Codex login/tests/suite/{auth_refresh,login_server_e2e}.rs；
DeepSeek Harness packages/core/agent-loop/tests/{request-freeze,tool-order,cancel,shutdown-drain}.spec.ts、packages/llm/llm-deepseek/tests/protocol.spec.ts、packages/credentials/credentials-local/tests/records.spec.ts。
只用合成 token、本地 HTTP、可控时钟/取消点，不读取真实用户 auth。

实现阶段既有回归命令（本轮文档 PR 未运行）：

```sh
cargo fmt --all -- --check
cargo test --locked --test config --test security --test backend
cargo test --locked --test stage1_model_registry --test stage1_chat_stream
cargo test --locked --test stage1_anthropic --test stage1_gemini --test stage1_transport
cargo test --locked --test stage1_reasoning_owner --test stage1_semantic_compaction
cargo test --locked --test headless_protocol --test headless_event_budget --test session_recovery
cargo test --locked --test stage1_runtime_modes --test stage1_host --test stage1_project_config
cargo test --locked --all-targets --all-features
cargo clippy --locked --all-targets --all-features -- -D warnings
```

## 13. 定稿与文档唯一性

| 范围 | 唯一入口 / 整理方式 |
|---|---|
| 当前认证/provider/protocol/目录/函数/转换/验收 | 本文；spec-provider-auth-rust.md 与 research/oh-my-pi-ts-rust-port-2026-09-21.md 草稿合并后删除，不保留三份规格 |
| feat-auth-plan-goal-compact.md | auth 细则和重复验收换链接，Plan/Goal/compact 保留 |
| feat-provider-account-routing.md | 旧底层认证/协议矩阵换链接，保留未来账号路由合同 |
| Broker/scheduler 规格 | 独立未实现需求保留，明确后续部署阶段，不误删 5000 客户端等需求 |
| 历史执行蓝图/research/quality | 保留证据与执行权威，不将旧笔记改成“最新”，不清理无关文档 |

本轮仅调研、合并规格和文档静态检查；没有实现/登录/读凭据/跑模型/安装 binary。
以下 D1-D5 已作为本次设计定稿采用，不再保留并行候选；实现与真实凭据操作仍需另行授权：

- D1：按 P0-P6 完成 Codex、已有四协议、DeepSeek 三路由、TUI/headless；不以“全部模型支持”代替逐路由验收。
- D2：独立 OAuth 登录，不静默复制第三方 refresh token；旧 API key import 不变。
- D3：单 crate、每 provider 一定义文件、共享协议、同步 ureq、Unix 私有 JSON/OS 锁，不加数据库/插件框架。
- D4：TUI 无凭据走 bootstrap；headless 首次登录独立 CLI device，模型会话不自动开浏览器。
- D5：多模态默认严格拒绝不支持/缺失内容；Files 上传、音视频与占位降级需另行批准。

### 13.1 原始需求逐项闭合

| 用户需求 | 定稿位置 / 转换与验收 |
|---|---|
| 1. TUI 切模型认证与 agent 认证 | 2.1 区分身份/权限；3.4 请求 scope；9 登录/选择生命周期；C15，A17-A19 |
| 2. OpenAI Codex auth | 7 字面 browser/device/store/refresh 合同；8 Codex wire 与重试；C01-C07/C09/C12，A04-A12/A16 |
| 3. 多模型、多协议、headless 接线 | 3 接口、4 配置、5 接入矩阵、9.4 JSONL；C08/C13-C15，A01-A03/A13/A18 |
| 4. Rust 专用目录、每文件职责、方便扩展 | 3.1-3.4 单 crate/每 provider 定义文件/共享协议；10 逐文件来源与迁移；C01-C17 |
| 5. 多模态传输/请求 | 6 DTO、用户/工具入口、能力、物化、scope、持久化/恢复；C10/C13-C15，A14/A15/A19 |
| 6. 采纳 Pi 认证/架构并优化 TS -> Rust | 1.2/1.4、7 小契约而非开放 records/hook 框架；10 复用/移植/重写/暂缓与许可；C01-C10 |
| 7. Pi/OMP 最新实现 | 1.1 固定核验截止时刻及 SHA/release 区分；1.4 有限 inventory；10.1 升级审查，未把所有上游能力标成 zenpi 已支持 |
| DeepSeek Harness 追加研究 | 1.3 真实实现/采用与不采用，C17；3.4/7/9.3 冻结、事务及持久化边界 |
| 文档唯一、不要 slop、最后 PR | 本节归并边界；同一个 PR 只改 Docs，不删独立未来需求与历史证据 |

### 13.2 设计冻结与发布门

本版本冻结的是可实施合同，`authoritative:false` 只表示不接管既有执行蓝图/任务调度权威；不表示本文仍是候选草案。`implementationAccepted:false` 必须等实际验收后才能改变。
没有尚待选择的主要架构方案。OAuth public client 接入资格、自有标识接受情况、套餐/区域/device 权限是 P6 外部发布门；未通过就不发布对应登录能力，不通过换账号/协议“绕过”。
DeepSeek 内置只承诺官方 /responses；/v1/responses 未被证实为内置别名，不阻塞按明确路径实现，也不推断服务端必拒绝。
本地 fixtures/Cargo 测试与 live receipt 分开记录；本文没有运行时通过声明。后续每项设计变更更新本文件的版本/C-ID/A-ID，不再复制另一份 auth SPEC。

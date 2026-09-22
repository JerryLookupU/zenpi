# Route Decision — opencode_agent_runtime

route_policy: `auto` (mode `understand`)

## Decisions

| Surface | Route | Reason |
|---|---|---|
| `session/**`, `provider/**` (turn loop, LLM stream, compaction, retry) | high_reasoning | Concurrency, cancellation, provider protocol and persistence semantics; wrong reading changes zenpi runtime correctness. |
| `tool/**` (registry + built-ins) | high_reasoning for `tool.ts`, `bash.ts`, `edit.ts`, `write.ts`, `read.ts`; standard for the rest | Tool execution boundary, sandboxing and output limits are correctness-critical. |
| `permission/**` | high_reasoning | Approval decisions gate every side effect. |
| `agent/**` | standard | Declarative agent definitions. |

## Worker Composition

- `learn_agent`: `claude` (Claude Code `-p`), `--permission-mode bypassPermissions`, `--add-dir` source + target repo.
- Concurrency: up to 12 workers (`LEARN_WORKERS`, hard cap 12).
- Unit of work: one source file per worker; retry up to 2 times on missing/short artifact.
- Folder synthesis: one worker per represented folder after all file notes pass master validation.
- Master lane: the cron script validates artifacts, writes receipts, updates indices, and is the only actor that writes `[x]`.

## Escalation

Escalate a file to a second worker attempt when: artifact < 1200 bytes, missing `## 完整行为复盘` or `## zenpi Rust 映射`, missing source path/hash, or placeholder text. After two failed attempts the row stays `[ ]` and the run reports it.

## Evidence

- Per-item receipts: `receipts/<item>.worker.json`, `receipts/<item>.master.json`.
- Worker logs: `.cron/<item>.worker.log`, prompts: `.cron/<item>.prompt.md`.

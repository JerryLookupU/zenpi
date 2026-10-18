# Shared Host Prompt File References

Provisional S2-019/S2-036 progress, observed 2026-10-02T02:28:07Z. Previous
goal turn fixed Android page scope during branch replacement: progress. Full
native Android/QR/LAN/Rust desktop objective remains active; all 36 rows [ ].

## Implementation

Move the existing TUI explicit @ reference parser into slash_actions and share
one prompt_attachments resolver between TUI and HeadlessHost provider workers.
TUI completion and input queue checks import the same unchanged parser. Both
workers resolve references after entering captured SubtabExecution, then pass
the same InputAttachment values into ordinary Agent admission/materialization.
Previously the non-terminal remote host sent reference text without files.

Preserve quoted-path escapes, UTF-8 offsets, word boundaries, slash-command
literal arguments, eight-reference and 4096-byte path caps. Existing
attachment_for_path rejects traversal, absolute paths, symlink components,
missing/non-file sources and files over 10 MiB. Agent still performs aggregate
byte/model-capability checks and reopens bytes under ToolContext. No upload,
provider file-id authority, Android filesystem execution or new journal writer.
Steering remains text-only. Accepted task may fail preparation; accepted is
runner admission, not successful file/provider execution.

New signed-registry HeadlessHost test selects a real child directory with a
different same-named primary file, sends a quoted reference, captures actual
provider RequestAttachment bytes, and confirms exact retry invokes provider
once. Six invalid references (traversal, unclosed, missing, oversized sparse
file, excessive count and Unix symlink) fail before provider invocation. It
then runs the shared desktop helper/Agent path and checks matching input,
bytes and SHA256, restored primary ToolContext and no raw file bytes in journal.
This proves owner/worker parity, not a new TUI event-loop or phone network gate.

## Verification

Cwd /Users/mac/Github/zenpi, tests use
ZENPI_HOME=/private/tmp/zenpi-attachment-parity-20261002.

- cargo test --offline --locked --test remote_parity --test remote_workspace
  --test remote_subscription_tls --test remote_subtab --test runtime:
  **69 pass** (4/25/15/10/15).
- remote_headless_commands with explicit test-runner --skip for
  headless_runner_executes_real_http and
  production_plain_host_routes_authenticated_tls: **5 pass, 2 filtered out**.
  Focused final attachment test rerun: **1 pass**. Excluded real socket tests
  remain unverified, not skipped-as-passing or counted in total.
- cargo test --offline --locked --lib attachment_identity_tests: **8 pass**,
  including three new shared parser/resolver tests. --lib file_completion:
  **3 pass**, retaining TUI completion behavior.
- cargo check --offline --locked, cargo fmt --all --check, git diff --check and
  strict blueprint validator pass. Existing compiler warnings remain.
- Five Python smoke/lifecycle/blueprint modules: **53 pass**.

First filtered headless regression had four passes and one assertion failure:
shutdown test assumed Running progress meant its backend already entered. The
worker persists Running before workspace preparation. Test now waits boundedly
for the second actual backend invocation before shutdown, preserving its
original two-invocation assertion and unknown-outcome behavior. Final five-test
run passes. Initial --skip command omitted Cargo's -- separator and failed
argument parsing; corrected command above is the executed evidence.

SHA-256 source:

```text
src/slash_actions.rs b25311396acf885bc7b7fbf9d24255219ff130da8f7b3a30719d5e0cc688533c
src/tui.rs f977fed6cfcb0e7f5399a9d0ed4571cebb493af6c8568ef2b82761e956c9c81c
src/remote/headless_host.rs 7faffcf8ee281b4b3f1208f148e5ae3eb03909b334a5b2809bfe1153cbaadfe3
tests/remote_headless_commands.rs 19dabe6549cddf0c47a89d3ba0817e6c3b899be02ff2bdaef4300afbf08971e4
```

Android source unchanged this turn; no new APK/device result inferred from
previous compilation. No current-source camera QR/same-LAN phone acceptance,
resource-control/media upload parity, effective worktree quotas, provider
delta replay/ACK, interactive PTY, API31/34/35 lifecycle or signed release/
migration completion. No agents, permission bypass, commit/push, unrelated
rollback or acceptance promotion.

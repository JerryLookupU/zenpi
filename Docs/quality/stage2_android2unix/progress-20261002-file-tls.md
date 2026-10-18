# File Navigation TLS and Local Attachment Ownership

Provisional S2-019/S2-036 progress observed 2026-10-02T02:39:47Z. Previous
goal turn added host-workspace file picker: progress. Full Android/QR/LAN/Rust
desktop objective remains active; all 36 rows remain [ ].

## Implementation

Three files_list tests now traverse real authenticated rustls TLS1.3/ALPN/P256
proof and HostAdapter over bounded fragmented memory pipes. A selected child
root contains docs/file name.txt, hidden entry, symlink and primary-only file.
Root/nested results return exact relative metadata only; no file contents,
hidden/symlink/primary path or absolute root. A device revoked after first list
gets Unauthorized/null on next read. A deliberate local selection change gets
RevisionConflict/null for the old subtab revision. Journal unchanged; checkpoint
unchanged after accounting for the intentional local selection write. These
are encrypted transport tests, not TCP/LAN/Android socket or camera gates.

Remote prompt admission now refuses Busy before durable reservation if the
desktop Agent holds local staged attachments. Shared prompt_input used by both
TUI and non-terminal remote workers rechecks under the actual Agent lock,
then resolves only remote inline references in the captured workspace. It does
not merge/consume/clear staged local references. This closes the admission-to-
execution race while preserving local composer behavior. Exact retry returns
its old durable receipt even if local references were staged afterward.

New domain test verifies Busy, unchanged journal/checkpoint, retained staged
reference, clear-then-admit, retry without resubmit, and worker-time refusal.
Actual HeadlessHost attachment test additionally stages the primary-root
same-named file, verifies no provider invocation on refusal, explicitly clears,
then reads only selected child bytes. Shared helper does not add another writer.

Android picker options now require current prompt authority and no accepted/
running prompt, as well as online/no-pending state, preventing insertion into
text-only steering if a task starts while dialog is open. Compose test checks
offline directory/file controls disabled. Headless background read fixture now
also invokes browseFiles. Instrumentation sources compile only.

## Verification

Cwd /Users/mac/Github/zenpi; Rust tests use
ZENPI_HOME=/private/tmp/zenpi-file-tls-20261002.

- cargo test --offline --locked --test remote_subscription_tls: **18 pass**,
  including three new files_list encrypted transport cases.
- cargo test --offline --locked --test remote_workspace --test remote_parity
  --test remote_protocol --test remote_subtab --test runtime:
  **67 pass** (27/4/11/10/15).
- remote_headless_commands explicit test-runner skip of two real socket tests:
  **5 pass, 2 filtered out**. Final focused attachment rerun uses actual staged
  primary file rather than URL reference: **1 pass**. No network test counted
  as a pass.
- zsh tools/stage2_android_source_check.sh
  /private/tmp/zenpi-file-tls-20261002/android: exit 0, main/unit/device compile;
  **151 JVM tests** pass in 28 classes. No device execution.
- Five Python smoke/lifecycle/blueprint modules: **53 pass**.
- cargo check --offline --locked, cargo fmt --all --check, git diff --check and
  strict blueprint validator pass. Existing compiler warnings remain.

SHA-256 source:

```text
src/remote/command.rs 4bc4be6be60488af44349de92941b40ffaaabb3856546db59c7a169c4fd93389
src/remote/headless_host.rs 24e85add37d8ff510d22f6ec7e26c215e5d69f12d7c63e4e0541955c386f715d
src/tui.rs 2da3b5d35781a25f3951ecd4c232966b698fe1df0a722ae741dd945e0c5fd294
tests/remote_subscription_tls.rs 375b98e477edeb4756749ef741f94f5fdc31bdb19e6528cf694c343e756b35e1
tests/remote_workspace.rs 7a8359dda4737164ec1808abbdb5ec02496eb3caa951a58c7538d703a612f51d
tests/remote_headless_commands.rs b73c5b00bdb19d014042fe8bf738c90b7976b746c612fd0e3114f5de08172be1
MainActivity.kt afcf0f5b0b74aeadf8e40a0f647b9323036d684a3794201120edbeaa6a22b987
```

Compiled evidence:

```text
main.jar b2f8a5fc654fed0c16fa579b8aa90cbde8b7efd63de9867e228636f4ffe5fe48
unit.jar 0452300dca4aa2b565559579cfb9249deef115d51072a7adfd5c8a1b058a7260
instrumented.jar 77591b95d3b517614be49eb4f7d4218e02c06bff0deb9402b21328623b2c3ff3
```

No Gradle retry this turn; previous turn's wrapper-lock EPERM remains latest
packaging evidence, not a current-source APK pass. No physical QR/same-LAN
phone acceptance, effective worktree quotas, full resource/routing parity,
provider delta replay/ACK, interactive PTY, API31/34/35 lifecycle or signed
release/migration completion. No agents, permission bypass, commit/push,
unrelated rollback or acceptance promotion.

# Host-Owned Remote New Session

Provisional S2-019/S2-036 progress on 2026-10-03. Previous goal turn made
retained-terminal-page progress. This turn adds actual session replacement
through the canonical host transaction; no nested agents, worker admission,
checklist promotion, commit/push or cleanup. Dirty changes preserved.

## Implementation

SessionNew is a typed Control write with empty arguments and current project/
session/request/owner/workspace revision. Headless and TUI route it to the same
NewSessionPlan. Before reservation, host refuses busy/admitted work, main-tab
mismatch, pending durable/ephemeral inputs, staged attachments and worker owners.
No client path or replacement identity is accepted.

A null workspace receipt reserves the fingerprint before replacement. The
new mapping and success receipt share the existing CAS checkpoint transaction,
advancing workspace revision once. Old conversation stays intact. Exact retry
resolves before current owner/session freshness, including restart; changed
identity conflicts, reserved incomplete outcomes remain UnknownOutcome without
another replacement. Preparation uses a cooperative one-second deadline, not
a hard deadline for synchronous filesystem/extension operations.

The canonical new-session commit now refreshes the pool's input/approval control
handles from the actual replaced Agent. This also fixes local new-session
callers retaining the old input port. TUI refreshes transcript/metadata only
when the session actually changes, not on replay. Old PTY cannot be relabeled.
Android adds capability decoding, exact persisted request/receipt validation,
main-tab confirmation and foreground/pending fences. It refreshes the selected
project and retrieves a fresh snapshot instead of reusing the old cursor.
Capability is not cached.

## Completed Verification

Rust uses ZENPI_HOME=/private/tmp/zenpi-resource-reload-test-home.

- rtk cargo check --offline --locked: pass before final control-handle addition;
  final test builds below compile the changed Rust source. Existing warnings.
- rtk cargo test --offline --locked --test remote_workspace remote_new_session
  -- --test-threads=1: passed after correcting the restart audit assertion.
  Tests old journal retention, same Agent replacement, old port isolation,
  Control/Read/Busy, scoped seven-field result, once-only revision, exact retries,
  restart replay, ID conflict, stale epoch, reserved unknown and revoke.
- rtk cargo test --offline --locked --test remote_workspace --test session_new
  -- --test-threads=1: final 79 passed across two suites, 45.49 seconds. Includes
  pool input-port refresh assertion and existing canonical local transactions.
- rtk cargo test --offline --locked --test remote_workspace --test session_new
  --test remote_protocol --test remote_auth -- --test-threads=1: 105 passed
  before final control-handle/pending-queue checks. Final workspace/session suites
  rerun separately. Protocol marks SessionNew as an owner/CAS write.
- rtk cargo test --offline --locked --lib existing_terminal_projects_only
  -- --test-threads=1: final one passed. Production TUI replacement updates its
  session cursor, invalidates old PTY projection and does not erase newer view
  messages when replaying the receipt.
- rtk cargo test --offline --locked --test remote_headless_commands
  headless_new_session -- --test-threads=1: final one passed. Mailbox control
  capability, same owned Agent replacement and exact receipt retry verified.
- rtk cargo test --offline --locked --test remote_subscription_tls
  --test remote_parity -- --test-threads=1: 45 passed. Existing in-memory TLS
  regression, not current production TCP/device session-replacement acceptance.
- zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-session-new-verified-20261003: production/unit/instrumented
  Kotlin sources compile and all 287 JVM tests pass. New cases verify persisted
  empty args, exact old/new result, numeric revisions, authority/main-tab and
  cache/foreground/offline fences. Narrow 320 dp Compose confirmation/changed
  scope/offline test compiled, not executed on a device.
- cargo fmt --all -- --check; git diff --check: passed.
- python3 tools/validate_stage2_android2unix_blueprint.py --strict: passed,
  36 unfinished items. Structural validator does not prove completion.

Initial domain test compared new journal before/after restart; canonical owner
startup appends its normal selection audit. Corrected to compare immediately
before/after receipt replay, preserving the no-repeat assertion. Initial TUI
test used iterator len; corrected to count. Control-handle addition briefly
missed its project field, corrected before final tests. Failed runs not claimed
as passes. No socket bypass, fresh APK or connected-device acceptance. Every
observed command handle is terminal.

## Digests

```text
src/project_workspace.rs 65fe4fb6064018f389e487dac9c2414b9592b1c8478b159b3d99067e9fd010ca
src/remote/sessions.rs 404973857414d758217994323c8fe283a2429e793432936aea762522d4b5b72e
src/remote/protocol.rs 4a7e159d9de566f1586cda2dab029713b949be7c34d99c052275247902fdfb3f
src/remote/host_adapter.rs de5ba9adc01491e447ddb8ec49c2b81b8b4631d05dafd4fddc67755baf833ddc
src/remote/headless_host.rs 64d88aa3062a019fec408a32b26b784d148e57fcc78d46d3c5c6ca58dbe1f480
src/tui.rs 9ce5ba007a2ffafe5d35667aed59e86c9af3b51eea4cccae512fe8d25df16f45
SessionCommandCodec.kt 049c036ef209b0e2a87689be131cab53ba603827728313e4e576ce2ffd25a636
MainActivity.kt 032743f7ab879f4e26ad9a051bcd010a4c75ce99e6dbaa361e50917d9fe4d920
main.jar 86e9ed483f2ae4a406eeb4742e67c4b403f04895f6eefd569660a49d1e583cae
unit.jar 5fd914d02bf88f0e21ac6b39baad1f069d37887cb1e9392452aa697acff6119c
instrumented.jar 16c991491e18c566969cc538969585501a41a3a7beb43f15cdaedddd910ace1e
```

Goal remains active. Session catalogue/resume/rename/delete and settings parity,
terminal emulation, current production provider/tool E2E, camera/direct LAN,
connected lifecycle and signed APK/release remain incomplete or unverified.

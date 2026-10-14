# Current Session Name Control And Synchronization

Provisional S2-019/S2-036 progress at 2026-10-03T18:32:37+0800. Previous goal
turn is progress: APK/source attribution guard and tests changed authoritative
state. This turn adds actual current-session name control. Baseline is dirty
worktree at HEAD 1a8ff1acf72e0816fad503a05df2b1d18e07361b. No nested
agents, worker admission, checkbox promotion, commit/push or cleanup.

## Implementation

SessionRename is a typed Control write routed through existing TUI/headless/
shared-pool ownership. Exact arguments are name/journal_revision. It requires
current session/main-subtab/owner/workspace CAS and next journal sequence, and
refuses fresh writes while execution, durable/ephemeral inputs, attachments or
retained worker owners are pending. Names reuse SessionStore validation, with
remote path/credential refusal and same-project duplicate-name check.

The host checkpoint binds the request fingerprint before a reserved journal
receipt, calls existing append-only SessionStore rename, then appends success.
Uncertain binding/append leaves UnknownOutcome; it cannot automatically replay
the rename. Exact retry returns the original receipt without overriding a
later desktop rename. Legacy journal recovery now restores the typed request
binding. Workspace revision/session identity stay unchanged. Response includes
request/project/session/name and journal revision after rename, before receipt.

Workspace listings add nullable session_name; historical catalog entries add
nullable name. Unsafe local names are withheld, not rewritten. Existing
subscription metadata naturally carries changed names without transcript turn
events. TUI refreshes its actual name and matching cached summary, not its
transcript. Android validates optional metadata and persisted request/receipt,
displays current/history names and confirms a validated edit. Foreground,
pending, offline and changed-scope fences apply. Existing offline cache drops
names rather than changing its migration contract in this turn.

## Verification

All commands run from /Users/mac/Github/zenpi. Rust commands set
ZENPI_HOME=/private/tmp/zenpi-resource-reload-test-home.

- rtk cargo test --offline --locked --test remote_workspace --test remote_protocol
  --test remote_auth --test remote_subscription_tls --test remote_parity
  --test session_new -- --test-threads=1: 154 passed, six suites, 43.48 seconds,
  exit 0. Rename scope/Read/Control/Busy/stale journal/exact retry/restart/revoke,
  unknown reserved recovery and TLS canonical-owner tests included.
- rtk cargo test --offline --locked --test remote_workspace remote_session_rename
  -- --test-threads=1: final one passed after additional duplicate-name and
  sensitive-local-name assertions. No duplicate journal side effect.
- rtk cargo test --offline --locked --test remote_subscription_tls
  --test remote_parity -- --test-threads=1: final 47 passed, two suites,
  12.70 seconds, exit 0. Added local desktop rename subscription test proves
  safe metadata update with unchanged transcript cursor and no read writes.
  Authenticated fragmented memory TLS rename/retry/catalog/resume test passes.
  Neither test proves real TCP/LAN or Android socket execution.
- rtk cargo test --offline --locked --test remote_headless_commands
  headless_new_session -- --test-threads=1: one passed, exit 0. Production
  mailbox rename/listing/retry, new session and resume use the same Agent.
- rtk cargo test --offline --locked --lib existing_terminal_projects_only
  -- --test-threads=1: one passed, exit 0. TUI name follows actual journal;
  exact retry preserves view messages, session-new/resume regression retained.
- rtk cargo check --offline --locked: final exit 0, zero errors, 20 existing
  warnings. Initial check failed on comparing Result with non-PartialEq error;
  corrected to is_ok_and, without changing SessionError or claiming a pass.
- zsh tools/stage2_android_source_check.sh /private/tmp/stage2-session-rename-20261003:
  exit 0, production/unit/instrumented Kotlin compilation and 293 JVM tests
  passed. New strict receipt, UTF-8 bound, authority, cache and optional metadata
  tests pass. SessionRenameScreenTest compiles at 320 dp but is not device-run.
- python3 tools/test_stage2_android_build.py: 10 passed. --verify-only still
  refuses missing current APK provenance (exit 1), as intended. No unchanged
  blocked Gradle/ADB/TCP retry or permission bypass.
- cargo fmt --all -- --check; git diff --check; strict Blueprint validator:
  exit 0. All 36 items remain unfinished; structural pass is not acceptance.
All observed command handles are terminal.

## Digests

```text
src/remote/sessions.rs aa3074c21c61b64c354f7f6928decab593e3e684144e60733270be8f5b06eac7
src/remote/protocol.rs c02286127cdf0248649090a09653adb35829e885f8682318d06a69ac2122fe1b
src/remote/host_adapter.rs f0c0b0e81a443f6233b4bc26b889815d254c5b9548a76995a6a54c1f51689ab0
src/remote/headless_host.rs 2c53d926d4307b53dd0621cdf3d55a393f5ac4e0497544c0f115676d94ddc399
src/project_workspace.rs ac5517c33b41b42655ad1fe99d4f228f81547b2c07ece14dd689c6d5ef345fab
src/tui.rs 79ae375cf405b5af3296b076700bf1eb3c77364dd6a20bc950a47b8847096ee9
tests/remote_workspace.rs d48a00f880c7f8a8ac21cc5bd6622da6163d5a42d82394f5b8a520b7ae03810e
tests/remote_subscription_tls.rs 7b7df204d715dfa3f1c2e730d6324fee3d67fef1963e4e7d28a240207fa12f0a
SessionCommandCodec.kt 1203ed12b91d3a996828d96f910ba43731d9d262fcdddf7ceceb2ec2829d84f3
MainActivity.kt ba68ba68efaf4e3c332997207450fc7e7ce0eee13aeda2e599396e28b6c43ec7
main.jar 2b8a40eff37869a2558ae93d0965d18e35b47737f26be7bf6b92e30904dead27
unit.jar c384d7f3b11514660c53b16bf20e38deb7154b640d934b70260e4e8f9a8e2351
instrumented.jar 0159b95660d33e5e3559c2e295a0c1a6c4c5b6ecbb81d32504a2627f8dde0c88
```

Goal remains active. Historical rename/delete, broader settings/terminal parity,
current source APK/device interaction, physical camera/direct LAN, connected
lifecycle and signed reproducible release remain incomplete or unverified.

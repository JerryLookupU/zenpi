# Workspace Commit Subscription Wakeups

Provisional S2-017/S2-018/S2-020/S2-036/S2-054 progress. The previous goal turn
verified approval/PTY wakeups. This turn completes workspace/subtab notification
coverage and exercises committed display changes over the shared authenticated
TLS subscription. The full QR/LAN/native Android objective stays active; all
36 Blueprint rows remain `[ ]`. No nested agents, commit or push.

## Changes

ProjectOwnerPool notifies only after successful owner checkpoint CAS and state
publication. Workspace changes wake previous and current session scopes,
including a closed project's watcher. New/resumed sessions also wake retained
projects whose workspace revision changed. Local subtab changes wake only the
owning session; remote changes commit a request receipt/workspace revision and
wake all retained workspace sessions. Identical local state, stale revision,
invalid candidate, failed checkpoint write/CAS and exact remote retry do not
produce a workspace commit notification. Notifications contain no payload,
grant no authority and do not advance transcript cursor or ACK.

Two new remote_subscription_tls cases gate the first encrypted batch, commit
an actual local or signed-device-authorized remote rename, then resume the
same subscription. Each sees one fresh display revision, matching workspace
revision across listing/subtabs, no transcript events and cursor remaining zero.
The remote case also verifies exact retry produces no second notification.
Transport remains bounded fragmented in-memory rustls TLS 1.3, not TCP/LAN.

Regression inspection exposed an old headless restore fixture whose directory
journal now fails before owner preparation: legacy request-ID import must read
every journal before adopting the checkpoint. The fixture now uses readable
journals plus invalid selected-project configuration to test the intended
owner-preparation failure/recovery. A separate new test preserves the unreadable
journal case and verifies fail-closed non-adoption, unchanged durable bytes and
no notification. Production import behavior was not relaxed to make it pass.

## Verification

Observed 2026-10-02T00:14:16Z onward; dirty worktree preserved. Cargo commands use
`env ZENPI_HOME=/private/tmp/zenpi-workspace-wakeup-20261002 cargo test --offline
--locked` with the following `--test` selections:

- remote_subtab 10, remote_workspace 22: **32 passed**.
- remote_parity 4, remote_subscription 10, remote_subscription_tls 9,
  tui_project_workspace 28: **51 passed**. The initial accompanying session_new
  run passed 13; final source after adding cross-project /new notification test
  passed 14 in the regression below.
- project_workspace 10, remote_approval_wakeup 1, remote_auth 9, remote_diff 1,
  remote_events 4, remote_terminal 3, remote_worktree 6, session_new 14:
  **48 passed**. One independent-process helper ignored in its parent suite
  is executed by the parent process test.
- headless_project_workspace final run: **19 passed, 1 filtered** using
  `-- --skip busy_diff_uses_selected_project_through_new_replay_switch_and_cancel`.
  Initial run had 17 passes and two failures: the outdated restore fixture
  repaired above, and busy_diff's TCP bind denied by the sandbox. The skipped
  network case remains unverified, not accepted.
- remote_headless_commands attempted full run: **2 passed, 2 failed**.
  Both failures occur before provider/listener startup at TcpListener::bind
  (lines 198/525), `PermissionDenied: Operation not permitted`. No replacement
  mock is counted as production/network acceptance.
- Final distinct fully passing Rust cases: **150 across 15 suites**, excluding
  the mixed remote_headless_commands run and the blocked socket case.
- `cargo check --offline --locked`, `cargo fmt --check`, `git diff --check`
  pass. Check reports 19 existing library warnings and one vendor warning.
- `python3 -m unittest discover -s tools -p 'test*stage2*.py'`: **44 passed**.
  Strict Blueprint validation passes, 36 unfinished rows.
- `zsh tools/stage2_android_source_check.sh
  /private/tmp/zenpi-android-workspace-wakeup-20261002`: main/unit/instrumented
  Kotlin source compiles; **69 JVM tests pass**. This is not Gradle, DEX,
  manifest merge, APK packaging, Android Keystore or connected device evidence.
  Android source unchanged this turn.

All observed command handles completed. No new APK/physical QR/LAN execution
or release acceptance claimed. Existing historical device receipts unchanged.

Verified source SHA-256:

```text
src/project_workspace.rs 5e6378458dfc92f65a23a8be53f70f833433de00f921a5c08a6dd3cc9185a4ef
tests/remote_subtab.rs 9e368e86c3ce00415c0d9bc1b3ca051545beb88df5441be56517687ab238c94e
tests/remote_workspace.rs c2f71da97560c7aca49e195f7201141e84ced797babe8802b79feba876d4be97
tests/remote_subscription_tls.rs 86b53c26ec746fe7aed53a8a87cb11d15f407659f82f976d068089e52c9e4c8f
tests/tui_project_workspace.rs 8bf101623c874f849b11c3049337a93f50a9f97deb8c7b09ee07a42d882e074a
tests/session_new.rs d4ff8b7271b242acc27dff7b9a7da20d13fad7ab7995a21e7b4b17ed5cd7b334
tests/headless_project_workspace.rs 56f3ba35cc4d47999a36fec46680be7a7d7b97ae9741445e2dcb11d340c29907
```

Remaining gates include fresh APK/device subscription screenshots, physical
phone QR over same-WiFi LAN, socket shutdown/deadline matrix, complete durable
stream-delta backpressure, executable independent-worktree scheduling 1..64,
interactive PTY/bulk, supported API lifecycle/reset/Keystore, migration/uninstall
and signed release/security. This note is not Master acceptance.

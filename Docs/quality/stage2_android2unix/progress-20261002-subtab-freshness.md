# Subtab Revision Publication Fence

Provisional S2-016/S2-033/S2-035/S2-038 progress. Previous goal turn fenced
background Android writes. Current inspection confirmed local commit_subtabs
increments/persists its own revision without changing remote workspace or
session transcript counters. Android cache publication previously compared
only journal/event counters, leaving a stale read able to regress selected
tab/name/order despite those counters being equal. Full QR/LAN/native Android
and Rust desktop goal remains active; all 36 Blueprint rows remain `[ ]`.

## Changes

ProjectionCommitPolicy now compares persisted subtab revision before its
optional journal handling when selected project and session match. A missing
next subtab view, foreign subtab project or lower revision is refused; u64
numeric syntax/cap validation is reused. Different project/session scopes
remain independent. Cache schema/authority unchanged: version 3 already
stores subtab revision but no live command permission. Existing store save,
resume CAS and write-confirmation display paths inherit the check.

SubscriptionReducer invokes the same policy before setting projection/done
or accepted-page counters. Three policy JVM tests cover equal/newer transcript
with older tabs, absent tabs, authority-free cache restore, scope changes,
optional journal, maximum/malformed counters and foreign scope. One reducer
test rejects stale transient tab page without replacing the first display,
then accepts a later newer final page. New ResumeStoreTest source checks
all three actual encrypted-store commit paths preserve bytes and pending
envelope after stale tab refusal, then accept a newer tab view.

## Verification

Cwd /Users/mac/Github/zenpi, observed 2026-10-02T01:13:10Z UTC.

- `zsh tools/stage2_android_source_check.sh
  /private/tmp/zenpi-android-subtab-freshness-20261002`: main/unit/instrumented
  Kotlin source compilation exits 0; **121 JVM tests, 25 classes pass**.
  New encrypted-store device test source compiles only, not executed.
  Existing icon/safe-call/fixture warnings remain.
- `env ZENPI_HOME=/private/tmp/zenpi-subtab-freshness-20261002 cargo test
  --offline --locked --test remote_subtab --test remote_subscription
  --test remote_subscription_tls --test tui_project_workspace`: **58 pass**,
  exit 0 (10/10/10/28). Actual owner/subtab transitions and TLS memory stream
  tests cover host behavior, not real TCP/Android LAN. Rust source unchanged.
- Python smoke tests 34, lifecycle verifier tests 11, Blueprint tests 5:
  **50 pass**. Strict Blueprint validation, cargo fmt and git diff checks pass.
- `./gradlew :app:assembleDebug :app:assembleDebugAndroidTest --offline
  --no-daemon`, cwd android: exit 1 before compilation, wrapper ZIP lock under
  /Users/mac/.gradle denied (Operation not permitted). No new APK or connected
  evidence. No agents, permission bypass, commit/push or historical receipt
  modification; prior dirty changes preserved.

SHA-256:

```text
ProjectionCommitPolicy.kt 1c5413d9ce7cbb16f9001a50e028603a5856a644002fa1d4809bdef6cb47a19f
SubscriptionReducer.kt fea3bc9614e628921401d1aac8c7e8cb3c43bf8f1dc2afc35d44580c58bed60b
ProjectionCommitPolicyTest.kt ed336431cb8e4572472bbe8376cafb4e01d5b527bf0738a8b66bf74e7da6c9e2
SubscriptionTest.kt a6ed27bb90e481ab18d539a838dcbf6cc3c91bf854d58e7f54ec72e2217d09cf
ResumeStoreTest.kt ea3661af51c7b21e1d2b79eea0d02bf34ab7e1a4f3ef7a3bce7d4cecf4e8cf39
main.jar c487f003010ddec1c7f6bdb9cac20cd362c234d0c5a58275493f01e3eb8d99fa
unit.jar a8e2c5155e779d34ddbbe55fd77d5fe8f6c1548657ff7f259edc6791a3755fdd
instrumented.jar 79aded3b985819cac4fc4b26808aa1acfa49da02ca2eb421d0fe7c1317958049
```

Residual scope: this refuses monotonic subtab regression, not all equal-
revision inconsistencies, workspace listing order across owner epochs or
host checkpoint rollback. Missing same-session tabs are conservatively refused.
Physical camera QR/same-WiFi LAN, fresh API31/34/35 APK/Keystore/lifecycle,
full stream delta replay/backpressure, independent-worktree scheduling 1..64,
interactive PTY/bulk and signed release/migration remain required.

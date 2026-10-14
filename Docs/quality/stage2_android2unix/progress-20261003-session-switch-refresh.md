# Authorized Session Switch Refresh

Provisional S2-020/S2-033/S2-038/S2-054 progress at 2026-10-03T18:47:41+0800.
Previous goal turn is progress: connected session workflow source/harness and
tests added. This turn found and repaired a runtime sync gap: desktop/another
client replacing the session ended the old subscription with owner_mismatch,
which Android classified as authority loss and stopped auto-refreshing. A
normal authorized session transition therefore required manual intervention.
No nested agents, worker launch, checklist promotion, commit/push or cleanup.

## Repair

New typed session_changed failure is emitted only when fresh authorized
workspace metadata contains the subscribed project with a different current
session. No new ID/content/authority is returned. Missing/denied project stays
unauthorized, and ordinary owner/write mismatch remains owner_mismatch. Races
during metadata sampling or between sampling/replay get one fresh authorized
sample before classification; unchanged/missing evidence keeps the refusal.

Android converts only correlated read-only subscribe refusal to a scoped
SessionChangedException. Foreground/network/reset/pending-write/revoke/scope
policy gates recovery. The ViewModel clears transient readers and old in-memory
projection/cursor, then freshly authenticates and reads the same project,
without trusting a replacement ID from the failed response. Persisted old
display remains until verified new snapshot commit. No mutation retry or ACK
is triggered from this signal. Revocation/true ownership failures still fence
authority. Older clients may fail closed on the additive code and need manual
refresh. Source protocol/client/operator docs record this compatibility limit.

## Verification

Commands ran from /Users/mac/Github/zenpi, Rust with
ZENPI_HOME=/private/tmp/zenpi-resource-reload-test-home.

- zsh tools/stage2_android_source_check.sh /private/tmp/stage2-session-switch-20261003:
  exit 0, production/unit/instrumented Kotlin compilation and 297 JVM tests
  pass. Four new tests cover correlated subscription conversion, dropping old
  cursor/views, offline/background/reset/foreign/pending fences and no failed
  subscription publication. Device SessionHostTest nullable revision arguments
  now requireNotNull; its prior constructor warnings removed.
- rtk cargo check --offline --locked: exit 0, zero errors, 20 existing warnings.
- rtk cargo test --offline --locked --test remote_subscription
  --test remote_subscription_tls --test remote_protocol --test remote_auth
  --test remote_parity -- --test-threads=1: earlier 87 passed, five suites,
  15.50 seconds after correcting the initial failures below. Final tests add
  metadata-sampling race and fresh-current-snapshot assertions.
  Final rerun: 88 passed, five suites, 15.54 seconds, exit 0. Authenticated TLS
  owner replacement ends the old subscription with SessionChanged; a newly
  authenticated read obtains the empty current-session snapshot/cursor zero
  without modifying either journal. Sampling/replay race refusals also pass.
- python3 tools/test_stage2_session_android_smoke.py: 5 passed, exit 0.
- python3 tools/test_stage2_android_build.py: 10 passed, exit 0.
- cargo fmt --all -- --check; git diff --check; strict blueprint validator:
  exit 0, all 36 items unfinished. No product acceptance implied.

Initial Rust test build failed because fixture moved session_id before new
switch assertions; corrected to clone. Then a prior subscription scope test
expected the now-replaced OwnerMismatch classification; updated the specific
authorized-project/new-session case, preserving foreign model scope refusal.
Failed runs are not counted as passes. No unchanged blocked Gradle/ADB/TCP
retry, stale APK install or permission bypass.
All observed required command handles are terminal.

## Digests

```text
protocol.rs 178d474747e9b74dc368613d1b4f9a6681d3afb361aa862b8d98de844f7c6b84
subscription.rs aa0115efb94b1240dbe58f91aca05e1b75e89c82e7c0417c0befece6fcdf11c5
remote_subscription.rs e2ecd670ad990b796677cd0deab715bc4c611410ba50f687036abba3e5e523c8
remote_subscription_tls.rs d74bf0bc3d3fcc2546c0f80538757711717f7825401709a3d00cf4d34386223a
SessionChangePolicy.kt 686bc72d582b207c12a5b64fcd2162f8dd9dc04e76a3cd89b13e53d7ae4fa7bd
SessionChangedException.kt 1e64aab9c24e9eb4c6e984b9f15631ccc7935e7924ca4fc516231b53eab4ee1d
PairingViewModel.kt b12c3265c983302d003fba53aa976c210cb723c9cb8cdf845191ca3368474a3d
HostClient.kt 0af4e5a4fcb6ac23ed336b9214c582843653093eeb3160db6869fe29b5ae8762
main.jar 47c82440c3ee67695a4d5b3668332923cc2eba6c7341b6cd0c4b4ebf2a6d2f55
unit.jar 21c5a06d549ece9f0ed6ea17138ef5bac2adb58d31933c3ea492737d47975b06
instrumented.jar c7ab2cfc44402a5066797dd7e666383abd93ffb821a37bccac8c075e6a8b0ed0
```

Goal remains active. Fragmented memory TLS and JVM checks are not current
device TCP/LAN evidence. Current APK, physical camera/direct LAN, connected
lifecycle, full parity and signed reproducible release remain open gates.

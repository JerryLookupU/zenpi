# Offline Authority Boundary

Provisional S2-033/S2-036/S2-038/S2-053/S2-055 progress. The previous product
turn implemented Android offline authority loss; the intervening RTK request
verified an existing installation but did not change product state. This turn
rechecked the current source and compiled artifact digests below. No checklist
promotion, nested agents, worker launch, commit/push or cleanup.

## Behavior

PairingViewModel strips live in-memory authority when leaving the foreground
or observing a network-route change. Owned connections and auxiliary readers
are cancelled and generation-fenced. Transcript, pairing identity and durable
cursor remain available as display-only data; owner/revision, approvals,
terminal/live stream and control capabilities are not carried into recovery.
Only a freshly authenticated host projection can restore those capabilities.

An unresolved workspace command or revocation is not resolved by disconnect.
Its exact identity remains available for explicit resolution; no replacement
write or automatic mutation retry is introduced. Unit coverage rejects old
cancel/steer/approval commands after offline and failed-authentication states,
while preserving the explicit same-request retry contract.

NetworkLifecycleTest now checks the stripped authority at real network loss
and foreground stop. SessionHostTest extends the existing cancel/reattach phase
with a foreground-stop/resume verified-projection assertion. These device
tests are compiled, not executed against the current source APK. The session
harness still requires six ordered phases and seven instrumentation tests.

## Evidence

The prior cached-source command completed with exit 0 and 307 JVM tests:
`zsh tools/stage2_android_source_check.sh /private/tmp/stage2-offline-authority-20261003`.
Current source and retained jar SHA-256 digests were re-read at
2026-10-03T19:40:15+0800 and match that compilation. No claim is made that jar
hashes alone substitute for current APK packaging or device runtime.

```text
PairingViewModel.kt a4f4d57e16031cf0e106815538c7953e7bbb34ea9c7b9f16f97d39043aefcf50
ConnectionCancelTest.kt 7f480dff886ac0d70e2b2b9e9e46ea49c015cef92a4845d3e0a51613249217a6
NetworkLifecycleTest.kt 5397a4ec20521a5e66c4e8c61b798709ce17863d7b0dc1b59f01d1dfd944ec86
SessionHostTest.kt f89606c3d75add592ce80d77fa3854fbf4dac73ba7ff926b9ec67186d3ef5bd8
main.jar a47495338eba971973ad3301f9cd4f02320a01e5b6ae17dd61b0456075a693de
unit.jar 7d1daf4a19f1b58fd3f100eca53e210789cb706dcb81c52b102ef8fad4783418
instrumented.jar cbc09fee7b9002c52cc236e334c4e09a1360101bd5fc0be680b176d0d3d893ff
```

Fresh verification this turn also completed with exit 0:
`zsh tools/stage2_android_source_check.sh /private/tmp/stage2-offline-release-recheck-20261003`.
It recompiles production, unit and instrumented Kotlin, passes all 307 JVM
tests and produces exactly the same three jar digests above. Existing compiler
deprecation/nullability warnings remain. The six-phase/seven-test session
harness validator has six unit tests passing; no instrumentation run is claimed.

The current APK provenance check fails before device use because no successful
current Gradle build receipt exists. Historical APKs are not installed or
credited to these sources. Fresh APK, connected lifecycle, physical-camera
QR/direct LAN and full parity/release gates remain open; goal remains active.

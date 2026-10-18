# Cancel Retains Pairing And Supports Explicit Reattach

Provisional S2-033/S2-036/S2-038/S2-054 progress at
2026-10-03T19:16:19+0800. Previous goal turn was progress: automatic session-read
race recovery and cache-contract correction changed source and tests. This
turn inspected current socket ownership, route monitor, pairing client and
ViewModel cancel/background/refresh behavior. No nested agents, worker launch,
checklist promotion, commit/push or cleanup.

## Repair

For an existing paired host with no unresolved request, cancel() reset the
entire in-memory PairingState even though encrypted pairing remained on disk.
Explicit refresh then had no endpoint and could not reconnect until metadata
was loaded by a new ViewModel. Connection cancellation was incorrectly behaving
like a partial forget operation.

ConnectionCancelPolicy now preserves paired endpoint/device and bounded durable
display/cursor, while removing all transient readers and live capabilities
through the strict cache codec. ViewModel invalidates the connection generation,
cancels and drops its Job handle before publishing Offline. No encrypted store
write or key deletion is performed. Unknown writes and pending revoke IDs remain
unresolved, and only an explicit exact-request retry may reconcile a write.
Unpaired invitation cancellation still returns to empty state. Replacement
invitation cancellation retains the prior saved-host reconciliation path.

Three new JVM tests cover idempotent stop, pairing/display retention, stripped
authority/readers, unknown write/revoke identity and unpaired reset. Existing
ForegroundReadTest now verifies cancel keeps the encrypted bytes unchanged,
retains paired offline display and cannot start a background refresh.
Production SessionHostTest now cancels after desktop-origin session switching,
verifies retained pairing without Control authority, explicitly refreshes and
requires same-session authenticated recovery without another host mutation.
The prepared session harness requires six ordered phases and seven successful
instrumented tests, with an explicit cancel/reattach receipt field. This is
compiled workflow preparation, not a device execution claim.

## Verification

All commands ran from /Users/mac/Github/zenpi.

- zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-connection-cancel-20261003: exit 0, production/unit/
  instrumented Kotlin compilation and 306 JVM tests pass, 1.235 seconds.
  Device tests are compiled only; no Compose runtime/screenshot/LAN acceptance.
- Python session harness six, APK provenance ten, desktop harness fourteen,
  headless harness nineteen, lifecycle validator fifteen and backup validator
  five tests pass, all exit 0. Total 69 deterministic Python tests.
- git diff --check and strict blueprint validator: exit 0, all 36 items remain
  unfinished. No Rust source changed; prior Rust tests are not a new run.
- python3 tools/stage2_android_build.py --verify-only: exit 1, missing current
  APK provenance. No unchanged Gradle/ADB/TCP retry, stale APK install or
  permission bypass followed. All required command handles are terminal.

## Digests

```text
ConnectionCancelPolicy.kt e0edcd2737006f9b40bc1b50d37a8f4da843c35f27d62881cf75d72925181c37
PairingViewModel.kt 0655c51f3be2c9cab82e85cbf09b8f3698c951c2e26b6606ed13952f8a16c64f
ConnectionCancelTest.kt 86b40a84c205a85a27fe676a17561f0ef31bb6750c41d6e73d337da87c46c39e
ForegroundReadTest.kt 8b4fd6de76e2cf0d0a2ef1dc3f37739062292faa558f527dbaa2de140061599f
SessionHostTest.kt 9d838aae34d9534ed8a3259312488f3f9fcfa98e6b78ab92048884d9f794c8cb
stage2_session_android_smoke.py bacca53c16b59e2dddd74bc9f71e389ee978857de98137ef19712c69e7eb723e
main.jar c04c3f20f08de3dbfa9b8bb75ae162f49931296ea5071a3167a84e5357d5913d
unit.jar 1517711bf22aefaa6d9119cbee4c64c4176fc18a1469e93bb0df41a894a2b4da
instrumented.jar 30a3773c34d3141af3fc599b7aed59b7300184a0c24e952defacec868c457bb3
```

Goal remains active. Current source-attributed APK, real scanner/physical
direct LAN, connected lifecycle/UI, complete parity and signed reproducible
release remain incomplete or unverified. No whole item is accepted here.

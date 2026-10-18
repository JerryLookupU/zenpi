# Android Monotonic Invitation Expiry

Provisional S2-031/S2-032/S2-052 progress on 2026-10-04. Previous desktop QR
expiry/layout turn changed source and verified behavior, so it was progress.
Current execution skill, dirty worktree, invitation parser, confirmation UI,
credential admission, pairing handshake and scanner evidence verifier inspected.
No workers/nested agents admitted; no authoritative checkbox promotion.

## Implementation

Android previously used wall time only for a parsed invitation's freshness.
Backdating the phone clock could extend its local confirmation window or revive
an object after observed expiry. Each parse now captures elapsedRealtimeNanos
at entry and the initially remaining wall-clock lifetime, bounded to 300 seconds.
Both wall and elapsed checks must pass; elapsed realtime includes device sleep.
Observed expiry/invalid clock permanently retires that parsed object. Synchronised
freshness protects checks across UI/IO contexts. No invite is serialized and no
QR field changes; host expiry/monotonic issuer/one-use checks remain authoritative.

Countdown uses the smaller remaining lifetime, and an independent UI tick keeps
the monotonic deadline visible even when wall seconds do not change. Code entry
and confirmation remain disabled after retirement. Existing confirmation,
storage-lock credential admission and pre-token handshake use the same check.
Valid receipts arriving after token submission remain valid; no automatic token
retry or cancellation publication is introduced.

Injected clocks keep historical parser/wire corpora deterministic without using
Android framework stubs. Regression covers rollback not extending/reviving,
exact monotonic expiry, invalid wall observations, backwards monotonic values,
signed monotonic wrap and maximum wall expiry overflow. Storage-wait and signing
cases advance elapsed time while wall time is rolled back, proving no credentials
or token admission. The prepared existing Compose test advances its injected
deadline, refuses confirmation and refuses revival when the injected clock resets.
Scanner harness now requires its unique new UI marker; missing/duplicate/old
marker outputs refuse. No current device execution is claimed.

## Verification

- Final current Android source check recorded below: production/unit/instrumented
  compile and 436 JVM tests pass: zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-invite-monotonic-current-20261004; exit 0.
  Instrumented code compiles only; existing
  deprecation warnings remain.
- Python Stage 2 discovery: 126 pass; scanner verifier: 12 pass;
  lifecycle verifier: 21 pass; Blueprint verifier: 5 pass.
- Strict Blueprint and git diff --check pass; all 36 authoritative rows [ ].
- Rust sources unchanged; current debug host verify-only attribution passes.
- APK verify-only refuses missing provenance. No unchanged blocked Gradle retry,
  historical APK install, camera/ADB/physical LAN or connected acceptance run.

```text
PairingInvite.kt 745b591b52b651f6f17d08ef319de518136a1930fddaded70a9b2459e1925fce
MainActivity.kt f3f8f3bc0c63bc133af14cda8e379756eb7f250516f6cb3f9fcfcbab711a611e
PairingInviteTest.kt 10a6efd6dc360cde3e34d75d346b6a9b9c0a571d92eb356485cc3333fc73ce8b
PairingHandshakeTest.kt 89b6ac33bc910590b410fba7366fe1c4daa20356a2c20ceca9528c518e8bc96b
PairingCredentialAdmissionTest.kt e48ec2671f05c20bd9917d769e9a5a7792393dce2cfa54bacecc9da1265adcab
PairingScreenTest.kt aaf8fd24d7b07b567d00a4a2d59d64de95cff8c9875195ae03e1c00954f3e15c
stage2_scanner_android_smoke.py 0ad51649c8f9bf0d355819c80a05574c42b33279697bbecf4955588838302fd5
main.jar 4632f229e3b15c3f6c41389714c8429d7056783c698a63cc21b51a3da8b4f161
unit.jar 3bb4eaafad70b8842e39e9342e6442d0771162fcd6c9c933303f944d168a26a2
instrumented.jar 1cc5d04a18c43c6701c04bc98023cefe2db88775126d23ead91a81e460cf5d65
debug-host-receipt d3aeb4ad7a6c6c1427246ca98464d1072f66663a0030fb231c8682a53d5b16f3
debug-host-inputs 6e853386d38b5ab7bd6667713d8c4eb057979731687da65a84cd99ce38a0b884
debug-host-binary 7218257a1d568a74bd8309b9682db79805f9a0eb74476504d15cb8daa0f7d694
```

Goal remains active. Current-source APK, physical QR/direct LAN bidirectional
sync/recovery, connected API lifecycle matrix, complete parity and signed release
remain incomplete or unverified. Source/fixture gates do not prove those
requirements or entire-item acceptance.

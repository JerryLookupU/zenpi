# Background And Route Interruption Reconcile Pairing Storage

Provisional S2-031/S2-037/S2-038 progress on 2026-10-03. Previous goal turn
changed first-pair cancellation and verification, so it was progress. Current
source was inspected before this repair. No nested agents, checkbox promotion,
commit/push, permission bypass or unrelated rollback.

## Repair

Foreground loss and network replacement previously cancelled pairing and
published an unknown-outcome error without reconciling an already-owned
credential save. The same empty-UI/saved-host race repaired for explicit cancel
also applied to these interruptions. They now use a shared storage reconciliation
barrier ordered after the already-owned commit, with generation fencing.

An existing record restores offline display-only state. Foreground and a usable
network permit a new authenticated read, never a repeat of the one-use pair
request. An absent record retains the interrupted invitation and unknown-outcome
error. Scanner admission/refresh stay fenced during reconciliation, incoming
links queue, and explicit reset supersedes a late storage read.

PairingCancelTest now covers explicit cancel, background interruption and
network loss. Each lifecycle case exercises both absent and completed saves.
The fixture holds an actual coroutine/storage commit boundary; it does not
claim any network result. Cleanup uses the production local reset. Desktop
suites require 24 base plus 51 control tests, 75/76 without/with Doze.
Lifecycle receipts require each distinct named recovery phase; missing,
duplicated or unscoped text cannot satisfy the gate.

## Verification

- Stage2 Python: 107 passed; lifecycle receipt verifier: 21 passed.
- Physical QR verifier: six passed; instrumentation/Blueprint tests: five each.
- Final Kotlin source gate /private/tmp/stage2-pair-interruption-final-20261003
  exits zero: main/unit/instrumented sources compile, 378 JVM tests pass.
  The new lifecycle regressions compiled only, not device-executed.
- Rust debug/release current-source build provenance, formatting, whitespace
  and strict Blueprint pass. All 36 checklist rows remain unfinished.
- APK verify-only refuses missing provenance. No stale APK installation,
  unchanged blocked Gradle/ADB retry, physical scan or direct LAN pass occurred.

```text
PairingViewModel.kt 123d5371903e51a6bbfe486beaf91e882987b727c4463dd22db08b6171e6dad6
PairingCancelTest.kt bf3b69c6bc805b9e22ad78ca33d7e16610bc54ea415bd0be00676f57e126f9a2
stage2_desktop_android_smoke.py 19a20d51e9c2fddddb30b8baab120d9d5b9a3519d75d818e189286fea9519f1b
validate_stage2_android_lifecycle.py 19b311174942255eb99994e0a6477f7718daee19543ad33e95d9527ea8d93044
test_validate_stage2_android_lifecycle.py 46d7b6346ff327d85a2023483f3013da0cc739bac741a6a21540babe7cd2d106
main.jar abb9199f95ec212fbd407fdb61f5e4ebe66f0aca8803669a28f46f93d72fa3fc
unit.jar a2a24570e2dffdd018d635b161598dad8b0ab38eb3d269fe7355ed89784f395f
instrumented.jar 6bad0e9c7c8000fec3d2257016a15c7f48a589136b241589fd831db263d065be
```

Goal remains active. Current APK and device execution, physical QR/direct LAN
desktop sync, full parity, supported API lifecycle matrix and signed release
remain unaccepted. The held storage tests are regression preparation only.

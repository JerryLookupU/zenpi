# Invitation-Bound Pairing Confirmation

Provisional S2-031/S2-032/S2-052 progress on 2026-10-03. Previous goal turn
changed terminal exit/reopen source and tests, so it is progress. Current scanner
and confirmation source were inspected before this repair. No nested agents,
worker launch, checklist promotion, commit/push or unrelated cleanup.

## Product Change

The safety-code draft formerly used rememberSaveable keyed only by fingerprint
and expiry. A same-host invite refreshed in the same second can keep both keys
while changing its one-time token. The draft is now transient and keyed to the
exact invitation object: replacement clears it, and page/process reconstruction
requires human entry again. Tokens are not written into saved Compose state.

PairingConfirmationPolicy is shared by Compose and the ViewModel. It requires
confirmation phase, a fresh invitation, six exact ASCII digits matching the SAS,
no reset and no pending workspace/revoke. Invalid/expired confirmation returns
before connection state, device-key creation or network registration. Foreground
and network requirements still apply in the ViewModel; PairingClient independently
rechecks freshness and SAS. QR/deep-link parsing and host token policy are unchanged.

## Verification

- Three new JVM tests cover exact/invalid ASCII code, expiry boundaries and
  clock extremes, phase/reset/pending fences, and a replacement with identical
  fingerprint/expiry but a different token/SAS.
- Existing PairingScreenTest exercises draft clearing on same-host replacement,
  reset/revoke/expiry disabled confirmation and manual reentry. ScannerResultTest
  verifies invalid ViewModel confirms leave confirmation and saved host unchanged.
  Both are source-compiled only; no device test count increase.
- Final source check /private/tmp/stage2-pairing-confirmation-final-20261003
  exits 0: main/unit/instrumented compile and 378 JVM tests pass. Existing Rust
  terminal QR pixel fixtures still decode using Android ZXing as part of JVM
  tests, but that is not a physical camera or LAN result.
- Stage2 Python discovery: 101 passed; scanner verifier: 11 passed; desktop
  verifier: 17 passed; lifecycle verifier: 18 passed; instrumentation/Blueprint
  tests: five each passed. Strict Blueprint, formatting and git diff --check
  pass. All 36 checklist items remain unfinished.
- Rust debug/release verify-only pass; Rust sources/binaries were unchanged.
  APK verify-only refuses missing provenance. No unchanged blocked Gradle/ADB
  retry, APK install or physical scanner/LAN execution occurred.

```text
PairingConfirmationPolicy.kt 541c3f7a7f962b2ee9a0c6393b04e8c60886d41ca4271dd47ac27ec1bb9367da
PairingViewModel.kt 2ac4b2a86e9266f5c0ae6c6ea420eb8755d0e643c929830ab21e18141e12b583
MainActivity.kt 2602d8fc646c0e45b2cb5c998238c3e78e8c7c7513cbaef14cae703217a65f50
PairingConfirmationPolicyTest.kt 9ce1ce8b7ea412552f4865837cdd276ba643eea223b7e3dbcbcb5ad384a99e3c
PairingScreenTest.kt b49be617eeaed6659a6d09591e265a7d235196923fb1e71856dce0147c5c875f
ScannerResultTest.kt 42f7a3faeefff4c4321df2d158bd06942860e47bd0ef4a7a8e9e8b42899e91f5
main.jar 77481c863ef0f2ba79568da8eb60ae0bb74b4846ee1c65ccb3dfcbd5a16bae7e
unit.jar a2a24570e2dffdd018d635b161598dad8b0ab38eb3d269fe7355ed89784f395f
instrumented.jar 2b51566af5336edce32d94395448debcefdf45481a878ae4645121374b716794
```

Goal remains active: current APK, actual camera QR/direct LAN desktop sync,
supported API lifecycle matrix, full parity and signed release remain unaccepted.

# Invitation Freshness At Credential Admission

Provisional S2-031/S2-037 security progress on 2026-10-03. Previous turn repaired
key/reset serialization and produced executable JVM evidence, so it was
progress. Current worktree was inspected again. No nested agents, checkbox
promotion, commit/push or unrelated rollback.

## Repair

Confirmation checked invitation freshness before launching key admission, but
could then wait on storage. An invitation expiring during that wait still
created a key before PairingClient rejected it. PairingCredentialAdmission now
runs the fresh-invite check after acquiring storage and checking cancellation
and generation, immediately before its identity loader. Transport freshness
checks remain separate, including the check before sending the one-use request.

The new JVM regression holds the mutex with a valid invitation, advances a
controlled clock to exact expiry, releases storage and verifies the loader is
never called. A one-second-before-expiry control verifies valid admission still
works. This covers local side-effect admission, not camera/network acceptance.

## Verification

- Stage2 Python discovery: 107 passed; lifecycle verifier: 21 passed.
- Kotlin source gate /private/tmp/stage2-key-admission-expiry-20261003 exits
  zero: main/unit/instrumented sources compile and 383 JVM tests pass. Device
  sources compiled only. Physical verifier six and instrumentation/Blueprint
  tests five each pass.
- Rust debug/release attribution, formatting, whitespace and strict Blueprint
  pass. All 36 checklist rows remain unfinished.
- APK verify-only rejects missing build provenance. No stale APK install,
  unchanged failed Gradle retry or physical camera/LAN result is claimed.
- Operator device availability was requested for the outstanding physical
  camera gate; no connected device or supplied serial is assumed.

```text
PairingCredentialAdmission.kt 3e97e48a9fb09d17187d02a839863f2595ae4d820dd101e254526563fc4ba66d
PairingViewModel.kt 77ef1078f6a91eae1c7976b9f61b69d346e42ab8b8915a66ccc1a8dce2c23864
PairingCredentialAdmissionTest.kt df55b2e7c798ef59205df6bbae9d36115f1c5f28e6bbb635a95fcd18c35c07d7
main.jar a15e4aefd39a6365734ad39c3cabc6f55384413ad5bd816e031418a00a2503fe
unit.jar ea58fb6025947826eb5bf8e1f650ec3a0adf4e9194dfb5fe1b67805ee35e8541
instrumented.jar 500db487dae058d98e84d09416e6ee052c3d4c730b68161bde5ce95ffbd8292a
```

The full goal remains active. Current APK, actual physical QR/direct LAN sync,
complete parity, supported API lifecycle and signed release remain unaccepted.

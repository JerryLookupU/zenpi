# Transport Requires Explicit Device Identity

Provisional S2-032/S2-037/S2-038 security progress on 2026-10-03. Previous turn
repaired background/network pairing reconciliation and produced current-source
evidence, so it was progress. Current sources were inspected again. No nested
agents, checkbox promotion, commit/push or unrelated rollback.

## Repair

HostClient.inspect defaulted to DeviceIdentity.loadOrCreate even though its
purpose is authenticated reading of a saved host. A caller omitting identity
could create a replacement key when the saved key was missing. PairingClient
also had a key-creating default evaluated before invitation freshness/SAS
validation. Both entry points now require an explicit identity. Whole-source
compilation verifies all current call sites supply one.

The production ViewModel remains the sole main-source loadOrCreate caller,
inside foreground/fresh-invite confirmation admission. Existing-host and
WorkManager resume paths load existing credentials, never recreate them.
LanSocketRoutingTest extends its preconnect regression: wrong SAS and foreign
saved device ID must throw before binding, the supplied key remains unchanged,
and valid explicit identity still reaches binding then closes on bind failure.

The five routing tests are now in the explicit desktop base suite, 29 base plus
51 control methods, 80/81 total without/with Doze. Its named identity preconnect
phase is required by the harness and lifecycle verifier, not just a boolean.
This is acceptance preparation; device tests have not executed here.

## Verification

- Stage2 Python discovery: 107 passed; lifecycle receipt verifier: 21 passed.
- Kotlin source gate /private/tmp/stage2-explicit-device-identity-final-20261003
  exits zero: all main/unit/device-test sources compile; 378 JVM tests pass.
  Routing device assertions compiled only. Physical verifier six,
  instrumentation/Blueprint verifier five each pass.
- Rust debug/release attribution and formatting pass; Rust unchanged.
- Strict Blueprint and diff whitespace pass; 36 rows remain unfinished.
- APK verify-only refuses missing current provenance. No stale install,
  unchanged blocked Gradle/ADB retry or physical camera/LAN result is claimed.

```text
HostClient.kt 888fb6413ab9b2dc902f7065bb757c065bbf8f60b8ba7ce9950b3aab21b967d4
PairingClient.kt 9dc871b6e19a104b25cc408e30a2dd09b758a49669275baf7d898967747bd828
LanSocketRoutingTest.kt 243eebcd1f14f65e972715a3211f19a812d2e8f2e5be4f740735eef9902a97ca
stage2_desktop_android_smoke.py b47bebe25c8444640a0bbe838f5242820a571ee1b17fdf599223baacb9f32a37
validate_stage2_android_lifecycle.py 6b944d39f595d3dbbda889de56a4843dae96933ce05677d8c3095c8cedeeabe8
main.jar e529664fd6a949e6866272b55ee204460598df3e15166e81818606124517a6d8
unit.jar a2a24570e2dffdd018d635b161598dad8b0ab38eb3d269fe7355ed89784f395f
instrumented.jar 500db487dae058d98e84d09416e6ee052c3d4c730b68161bde5ce95ffbd8292a
```

Goal remains active. Current-source APK, actual camera/direct physical LAN sync,
complete parity, supported device lifecycle matrix and signed release remain
unaccepted.

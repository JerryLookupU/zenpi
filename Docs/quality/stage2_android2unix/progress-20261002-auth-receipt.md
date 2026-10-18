# Strict Android Authentication Receipt

Provisional S2-031/S2-032/S2-052 progress observed 2026-10-02T06:32:37Z.
Previous goal turn made progress (Main-owner callback serialization).
HEAD 1a8ff1acf72e0816fad503a05df2b1d18e07361b; dirty changes preserved.
All 36 authoritative rows remain [ ]; full goal active. No worker admission,
subagents, commit, push, cleanup or acceptance.

## Implementation

HostClient previously used Gson asBoolean on authentication success and checked
device_revision only when self-revoking. AuthenticationReceipt now validates the
success envelope and result before ResponseReader returns to workspace/ACK/
command followup. Exact control envelope fields are required; routing, owner,
revision, sequence and code metadata are refused even when null. The result
allows only authenticated, device_id, control_available and optional positive
lexical-u64 device_revision. authenticated must be the boolean true, saved
signing-device ID must match, and control_available must be the boolean false.
Unknown fields, null/fractional/coerced/zero/overflow revision fail closed with
fixed diagnostics. Legacy absent revision permits ordinary reads, but self-
revoke refuses because no authenticated revision fence exists.

Self-revoke constructs its expected_revision from the validated numeric string,
not an unchecked mutable JSON element. PairingHandshake now also rejects an
otherwise successful control_available=true registration receipt. Authentication
and QR pairing cannot substitute for separately granted project ACLs. Production
Rust host already returns this exact read-only authentication receipt; no host
permission or operation changes were required. Protocol documents the boundary.

## Verification

Cwd /Users/mac/Github/zenpi. Final commands exit 0:

- zsh tools/stage2_android_source_check.sh
  /private/tmp/zenpi-auth-receipt-android-final: **207 JVM tests**, main/unit/
  instrumented sources compile. Seven new tests cover current/max/legacy
  revision, missing/unknown/authority fields, boolean coercion, exact identity,
  numeric/null/type failures, real FrameCodec/ResponseReader correlation with
  zero followup on malformed success, and envelope routing/failure metadata
  injection. Existing PairingHandshake adversarial matrix also now rejects
  true control grant. Device sources are compile-only; not a new APK.
- env ZENPI_HOME=/private/tmp/zenpi-auth-receipt rtk cargo test --offline --locked
  --test remote_auth --test remote_protocol --test remote_pairing
  --test remote_device_grants --test remote_subscription_tls: **75 tests**.
  Existing host authentication/QR/ACL/production grant and encrypted rustls
  fragmented-memory-stream tests remain green; not live Android/Rust LAN proof.
- env PYTHONPATH=tools python3 -m unittest
  tools.test_stage2_desktop_android_smoke tools.test_stage2_headless_android_smoke
  tools.test_stage2_scanner_android_smoke tools.test_validate_stage2_android_lifecycle
  tools.test_validate_stage2_android2unix_blueprint tools.test_validate_stage2_android_backup:
  **63 tests**.
- cargo fmt --all -- --check; git diff --check; python3
  tools/validate_stage2_android2unix_blueprint.py --strict: exit 0.

Final cached-source artifact SHA256:

```text
main.jar 322a7c495a4fa3a9eb3ceb79df9a134de74af5c426e02a5b11871943a66fb952
unit.jar 4d2037a7a646dfcd7a63c6e250fefcba234d6bab7979eb391f4ec659a2102431
instrumented.jar c3df43871171aeeef3f9a58157a55f490737c232fbc962d5c27b9fb808ddecdb
```

Source SHA256:

```text
AuthenticationReceipt.kt 27b84e9f66dae27bce63a6094de56019a54ad63a092660c9e1ed3b2c5b391753
HostClient.kt 49fc8d95dd196dbf0a303e426022e59f23fdf11e1cd1ec3bfa614f9668e7e1ae
PairingHandshake.kt 7ac034f27df4999d0d3f42cf20ed67cc0c0c41655a560d6e2ce802e1e622928c
AuthenticationReceiptTest.kt aeea01aeea5b7b379c2d1ae6cd7ffe35c74cb7ebfc95d7b531af1b9ab77e031d
PairingHandshakeTest.kt 35cfc424d4f6f6946c370bae40e81b4bdea19abc798fbc9f8596aaadc9495980
```

## Residual Scope

Current APK, real camera QR/pinned Android-to-desktop TLS, physical LAN restore,
supported API 31/34/35, full scheduler/settings/interactive PTY and signed
release remain unfinished. Rust in-memory TLS and Kotlin framing tests do not
replace cross-language socket/device acceptance. No socket/Gradle/adb bypass.
This note is provisional progress, not an item acceptance receipt.

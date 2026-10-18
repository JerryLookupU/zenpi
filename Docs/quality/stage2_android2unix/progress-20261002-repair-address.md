# Same-Identity QR Re-Pair Recovery

Provisional S2-031/S2-033/S2-037/S2-038/S2-053/S2-073 progress observed
2026-10-02T06:14:18Z.
Previous goal turn made progress by adding remote Diff private-path filtering
and authenticated TLS tests. HEAD 1a8ff1acf72e0816fad503a05df2b1d18e07361b;
dirty user changes preserved. All 36 blueprint rows remain [ ]; full goal
active. No worker admission, subagents, commit, push, cleanup or acceptance.

## Implementation

PairedHostStore.save now preserves the strict display/cache cursor record when
both host certificate fingerprint and device ID match. This allows a confirmed
QR re-pair to the same desktop at a replacement LAN address or port without
discarding cached messages/subtabs/cursors. The existing cache codec strips all
live grants, owner epochs, prompt-running authority, model/terminal/stream
previews. Fingerprint or device changes discard display; unresolved workspace
or revocation intents refuse the save. Every save writes a fresh encrypted
record, invalidating background resume stamps even at the same endpoint.
Old endpoint foreground commits still fail exact-host matching.

PairingViewModel parses QR before cancelling a live connection or replacing
host state. Invalid QR stores only a fixed scanner diagnostic. Valid replacement
confirmation holds the previous host as an offline, nonauthorizing display;
Cancel restores it and rereads encrypted metadata under the existing storage
mutex to reconcile an already-in-flight pairing save. Forget clears the held
state. Pairing now uses the same explicitly injected device identity as refresh
and revoke; production default still uses the Keystore identity. Confirmed
re-pair consumes only the store's sanitized cache, not the old live projection.
No QR token is stored and no host grant is created or expanded.

Added scanner confirmation cancellation/invalid replacement device tests and
same-identity encrypted-store tests for endpoint CAS, old endpoint rejection,
same-endpoint re-pair fencing, pending workspace/revoke preservation and changed
fingerprint cache clearing. Scanner smoke count is 19 and requires both new
phase markers; desktop selected-suite counts include the extra store test.
Protocol and Chinese guide describe recovery and identity boundaries.

## Verification

Cwd /Users/mac/Github/zenpi. Commands exit 0:

- env ZENPI_HOME=/private/tmp/zenpi-repair-address rtk cargo test --offline
  --locked --test remote_auth --test remote_pairing --test remote_device_grants
  --test remote_subscription_tls: **61 tests**. New signed-device re-pair test
  reopens the real registry, consumes one invite, preserves exact grant/name/
  revision/creation data and registry bytes, permits only the original project,
  and refuses revoked identity before invite consumption. Existing encrypted
  TLS/ACL/pairing/production grant suites remain green.
- zsh tools/stage2_android_source_check.sh
  /private/tmp/zenpi-repair-address-android-final: **182 JVM tests**, main/unit/
  instrumented source compilation. Four new JVM policy tests verify same pin
  plus device, display/cursor retention, removal of live authority/previews,
  changed identity isolation, unresolved intent refusal, first/empty-ACL state.
  Device/Keystore/Compose tests compile only, not executed; this is not an APK.
- env PYTHONPATH=tools python3 -m unittest
  tools.test_stage2_desktop_android_smoke tools.test_stage2_headless_android_smoke
  tools.test_stage2_scanner_android_smoke tools.test_validate_stage2_android_lifecycle
  tools.test_validate_stage2_android2unix_blueprint tools.test_validate_stage2_android_backup:
  **63 tests**. Required-marker negative tests automatically cover new phases.
- cargo fmt --all -- --check; git diff --check; python3
  tools/validate_stage2_android2unix_blueprint.py --strict: exit 0, 36 unfinished.

Source SHA256:

```text
RePairPolicy.kt 40d859abbbae3aacd3d37b581a42ff11f1df5d2e5eb3578be10a49998710886f
PairedHostStore.kt a68e031201dbd79ca50d52db94de374a2e4e9122ac6372af3a9dc59d1984d8d1
PairingViewModel.kt 29d2ce59e2827f46e9d505aed3436ac4747c4dd79e3091a4b1a87afc05f905a7
RePairPolicyTest.kt cee43475b44e469bcb685824a7c3c1e9b49924a6c67ab52ebfcaaad4b4607749
ScannerResultTest.kt dd5069d79772f8280e80d38b4c081fc00b29c997c60eb48709fbce42707e3641
ResumeStoreTest.kt 9c76794d485a69102eac2d374e4ca74e2f21db8bf7e196a6ad956bf428c7a839
tests/remote_auth.rs 27d4e4dc6b514e9a1cc9e2ed551e8023aaacecc4b92732368cdd49d0b692891c
```

Final cached-source Android artifact SHA256:

```text
main.jar 9bbfa5ed9d7aca71bba52b8d2b2d40352fe2e347a51db36daf114ffad699e0eb
unit.jar 70bb7ba1ad21e9d1f5a8d1b60c6dcad11ffa6e982900441c605d9d4d96c81b0b
instrumented.jar 7af77256025688ffcd02a653f98bdd98c436ec66a4aeeda32a79a874ef7789c0
```

## Residual Scope

Not proof of actual Android camera scanning, physical LAN address switching,
device process recreation, Keystore execution or release packaging. Scanner
and encrypted metadata device tests still need supported API 31/34/35 runs.
Pending-write re-pair remains deliberately fenced, not an alternate endpoint
recovery operation. Full concurrency scheduling, interactive remote PTY,
settings parity and signed releases remain unfinished. No restricted socket,
Gradle daemon or adb bypass. This note is not a Master acceptance receipt.

# Revocation Cleanup Identity Fence

Provisional S2-037/S2-052/S2-056 progress, 2026-10-04. Previous goal turn added
and executed current Kotlin/Rust session replacement interoperability, so it
was progress. This turn inspected execution skill, dirty integration, Blueprint
parity/network/release gates, Activity/lifecycle, pairing and revoke cleanup.
No nested agents, unrelated reset, checklist promotion or restricted-runtime
bypass. Current APK attribution remains missing.

## Repair

Confirmed self-revoke and committed-receipt reconciliation previously invoked
DeviceIdentity.wipe with only an alias. DeviceIdentity resolves that alias on
each operation; a key replaced after host confirmation but before local cleanup
could therefore be removed by a receipt belonging to the previous identity.

DeviceIdentity now offers wipe(expectedDeviceId). It checks the alias's current
public device ID through the typed AuthenticationProof boundary and removes the
key only if it still matches. Check/removal holds the same Companion monitor
as synchronized loadExisting/loadOrCreate. Unconditional wipe now uses that
monitor too, preventing app-local creation/removal from interleaving between
the identity check and delete. No signing key is exported or newly created.

Both ViewModel revocation completion paths check coroutine cancellation after
storage dispatch, then invoke finishRevoke with the saved revoked device ID.
PairedHostStore already preserves its pending marker when key removal throws:
missing/replaced identity therefore leaves encrypted metadata/request unchanged
and does not delete the replacement. Explicit local Forget remains an
unconditional reset, not a host revocation claim. External changes to Keystore
are outside this app-local synchronization; no atomic external transaction is
claimed and a later explicit reset may still discard local credentials.

Added DeviceRevokeTest real-Keystore assertions: persist pending revoke, delete
the old key, refuse cleanup with unchanged metadata bytes; create a distinct
key under the same alias, refuse old cleanup and verify the replacement and
pending marker survive; matching expected identity can delete its own key.
The actual DeviceIdentity API and instrumented method compile and javap shows
both wipe overloads and the new test. Device execution remains required.

## Verification

Commands ran from /Users/mac/Github/zenpi against current dirty integration.

- zsh tools/stage2_android_source_check.sh /private/tmp/stage2-revoke-key-fence-20261004:
  exit 0; Kotlin production/unit/instrumented compilation, 463 ordinary JVM
  tests; current Rust read fixture plus explicit rename/P256/worktree/session
  lanes pass. Existing warnings remain. Real Keystore assertions compiled only.
- Python Stage 2 discovery: 132 pass. Lifecycle verifier: 21 pass. Exact desktop
  instrumented base count raised from 30 to 31 for the new method; reduced
  historical receipts cannot satisfy the current suite identity.
- rtk cargo test --locked --offline --test remote_auth --test remote_security --test remote_pairing --quiet:
  exit 0; 32 top-level tests plus one bounded FIFO child (RTK aggregate 33
  pass/7 filtered). This is focused host auth/security, not a full Rust suite.
- Strict Blueprint, Rust format and git diff --check pass; 36 rows remain [ ].
  APK verify-only still refuses missing provenance. No old APK was installed;
  unchanged Gradle socket-policy restrictions were not retried or bypassed.
- Debug host rebuilt after Rust invocations; separate verify-only passes.

```text
DeviceIdentity.kt 3ef989f49141fd28deb924ac72de7d05e07672358a493418030109c97b30abfc
PairingViewModel.kt 859dbc60f6a2aecfa04ec9c21156824d6ba8467340777e2278e812042128abf0
DeviceRevokeTest.kt 17eb15dcb32976e06920505b0a957f224bf51a3b6e3695ddaea168c4d80e0306
stage2_desktop_android_smoke.py 13bc6e02573b796e3360b97f37935ba24283decd1dfdcb73e325be5fa53c4738
main.jar 44bd07b76fbcff0a6526d71cc3096c0f47a53b91ea2fb107254ab4ff3ea5c20c
unit.jar d72db3cafe5180cd83cdbd2f04c9ed4e5655ef0dc5b8f999d291527af9ec11eb
instrumented.jar a158d058233870c485602549c2bf234b63b42c2238f09ab48bf0796460f5aa17
debug-host-receipt 3fe4784266c36187878c149333878752939a0549e5c42a19337bfa8d5ef0b0f3
debug-host-binary 701d3e597cd640090cdc254f6ad2f810b986dbd626e42179ca3da6d8e50c5d77
```

Goal remains active. Current APK, physical camera QR/direct-LAN bidirectional
sync/recovery, connected API lifecycle matrix, full parity and signed release
remain incomplete or unverified. The guarded cleanup implementation and source
regressions do not accept an entire Blueprint item or prove device behavior.

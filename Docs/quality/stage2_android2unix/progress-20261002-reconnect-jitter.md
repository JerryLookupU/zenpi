# Android Foreground Reconnect Jitter

Provisional S2-038/S2-053 progress observed 2026-10-02T06:36:20Z.
Previous goal turn made progress (strict authentication receipt validation).
HEAD 1a8ff1acf72e0816fad503a05df2b1d18e07361b; dirty changes preserved.
All 36 authoritative rows remain [ ]; full goal active. No worker admission,
subagents, commit, push, cleanup or acceptance.

## Implementation

Foreground PairingViewModel attach previously retried on deterministic 4/8/16/
32-second waits. ReconnectBackoff now implements full jitter with failure
ceilings 1/2/4/8/16/32/60 seconds, sampling uniformly from zero through each
ceiling and saturating its exponent. Multiple clients no longer have identical
scheduled retry delay by construction, though random collisions remain possible.
Healthy renewal retains its two-second interval. Pending replay tails retain
200 ms catchup only without a failure; failed reads never use that bypass.

Confirmed subscription publication now resets failures immediately after its
cache save and generation recheck, rather than waiting for the whole exchange
to finish. Final successful attach also resets. Failed cache save does not.
Background/network/replacement cancellation interrupts the coroutine wait;
route recovery or explicit refresh starts a new bounded attach sequence.
Existing exact-command retry remains outside the read retry loop, and no
pairing token, unconfirmed write or revoke is automatically resubmitted.
WorkManager's separate three-attempt scheduling policy is unchanged.

Eight added JVM tests cover every growth/cap threshold, saturation under 10000
failures, zero/middle/maximum samples, client desynchronization with deterministic
samplers, invalid sampler bounds, success reset/event catchup, cancellation
without another read, failure-versus-confirmed cache publication using the
production owner callback, and healthy intervals never sampling failure jitter.
Protocol, Android implementation notes and Chinese guide now describe timing.

## Verification

Cwd /Users/mac/Github/zenpi. Final commands exit 0:

- zsh tools/stage2_android_source_check.sh
  /private/tmp/zenpi-reconnect-jitter-android-final: **215 JVM tests**;
  main/unit/instrumented sources compile with existing warnings. This is cached
  source verification, not Gradle/APK or device execution.
- env ZENPI_HOME=/private/tmp/zenpi-reconnect-jitter rtk cargo test --offline
  --locked --test remote_subscription --test remote_subscription_tls
  --test remote_faults --test remote_auth: **63 tests**. Existing host auth,
  subscription cancellation/fault and encrypted rustls memory-stream tests pass;
  no physical LAN failure/reattach proof is claimed.
- env PYTHONPATH=tools python3 -m unittest
  tools.test_stage2_desktop_android_smoke tools.test_stage2_headless_android_smoke
  tools.test_stage2_scanner_android_smoke tools.test_validate_stage2_android_lifecycle
  tools.test_validate_stage2_android2unix_blueprint tools.test_validate_stage2_android_backup:
  **63 tests**.
- cargo fmt --all -- --check; git diff --check; python3
  tools/validate_stage2_android2unix_blueprint.py --strict: exit 0.

Final artifact SHA256:

```text
main.jar df3022d73c91379e3069e1ca8fe61f0fc090789dee64b0d82bd9cc56aa708456
unit.jar d6895b2d2e88ba02c8b47c0f08363d54f5a71ca5cc7e5bd75673566783576307
instrumented.jar c3df43871171aeeef3f9a58157a55f490737c232fbc962d5c27b9fb808ddecdb
```

Source SHA256:

```text
ReconnectBackoff.kt a68447d8aa3bc6cf138a1ad091a8b764699976fd3de3a3b36d950cf60cfec13e
PairingViewModel.kt 1853262d4fd0f33dd3d3eba48a8adbd03321205ad9e58a9709f569e8a4c24c3c
ReconnectBackoffTest.kt df89fa04c2140e3aca05f3f0eeee2fa1698453881c5c601eab93643368e322ca
Docs/stage2_android2unix_protocol.md ecb2a3f9823b40449c0892ebb32e497862e1aa7336728624664464987f2f6d8f
Docs/stage2_android2unix_user_guide.md ed0a078dc2185ff2d7530bb1e214b4d1f1d8431d5d5d94153422cad347dccd72
Docs/stage2_android2unix_android.md 4ed5d297ae0ac2bea63e8831d1c70585f132499b4a3e609ab608c0cc5b0718cd
```

## Residual Scope

Need current APK/API 31/34/35 device and actual LAN loss/route-change/recovery
tests to observe the complete ViewModel/network integration. Random jitter
reduces synchronization, not a guarantee that client retries never coincide.
No universal reconnect guarantee on unavailable routes or arbitrary NAT.
Camera QR, full concurrency scheduler/settings/interactive PTY and signed
reproducible release remain unfinished. No restricted network/Gradle/adb
bypass. This provisional note is not an item acceptance receipt.

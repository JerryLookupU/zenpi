# Snapshot Terminal Consistency

Provisional S2-020/S2-033/S2-052/S2-053 progress at 2026-10-03.
Previous goal turn was implementation progress. Current-source replay inspection
found that the gap-snapshot branch ignored terminal phase/journal metadata.
No nested agents, worker launch, checklist promotion, commit/push or cleanup.

## Implementation

Android HostProjectionCodec now validates previous project/session scope before
both incremental and snapshot replay. Gap snapshots must agree with the
completion on event cursor, journal sequence and phase, with explicit envelope
session/project checks. Snapshot event cursors must be absent together with
their scope (legacy compatibility), or present as exactly session_id plus
next_sequence; partial, null and extended cursors are refused. Invalid data
never reaches the caller's projection publication/cache path.

Three JVM tests cover contradictory phase/journal completions, wrong snapshot
envelope/previous scope and malformed optional cursor pairs. SubscriptionReducer
is exercised with a snapshot plus transient view: rejection preserves its
prior projection/completion state, while a valid snapshot commits its transcript
and transient view together. Persisted cache keeps the valid transcript/cursor
but not transient stream text or owner authority. Existing branch replacement
at equal physical event cursor remains valid. These are framed-memory/source
checks, not connected TLS, camera or LAN evidence.

## Verification

- zsh tools/stage2_android_source_check.sh /private/tmp/stage2-snapshot-terminal-20261003:
  exit 0; main/unit/instrumented source compiles, 340 JVM tests pass. Existing
  deprecation/nullability warnings remain; instrumentation was not run.
- python3 -m unittest discover -s tools -p 'test_stage2_*.py': exit 0,
  86 pass; expected invalid-LAN argparse fixtures emit errors.
- Blueprint validator unit tests: exit 0, 5 pass. Strict validator and
  cargo fmt --all -- --check pass; all 36 checklist rows remain unfinished.
- ZENPI_HOME=/private/tmp/zenpi-resource-reload-test-home cargo test --offline
  --locked --test remote_events --test remote_subscription --test remote_faults
  -- --test-threads=1: exit 0; 8 event, 14 subscription and 3 deterministic
  frame-fault tests pass (25 total). These exercise host-domain and in-memory
  fault behavior, not the production listener or physical network. During the
  run the confirmed exec handle remained live through an output gap; it was
  polled without spawning a replacement, and subsequently completed.
- Host release --verify-only passes. Android build --verify-only exits 1 with
  missing APK build provenance. No old APK used or blocked Gradle retry.

```text
main.jar 6bd905731a276f7ffef4cbdfab2e6abe0cc532163d405ad028f1880bf435109c
unit.jar a9ef160665315283a168ebf7c5555b728ab8188c02849cf4a900fb1f559e5906
instrumented.jar c5ca3ab9f31fa8c95714356f94b6e5f17247d0711e6daa808a907d20b6328093
HostClient.kt 140e0ff9def33ae57a0e1657b54e08dd421f35988a6516857bcfcd7473415706
ReplayTest.kt cc1e4703c026cfb7129fb8974aca8052435f46b55148652de21d45622a25dcee
SubscriptionTest.kt ec7703b40035b805f59130ba4ac7a3c39b6ac5ffea52122d5d8989b2f7b9a491
HostProjectionTest.kt 48cc86ac6350fc37fef7f872b3e87cc396d4d9fcba2f33859d6a32802cdf9aeb
```

Goal remains active. Current-source APK, production desktop-to-Android TLS sync,
physical QR/direct LAN, connected API lifecycle and signed packaging/upgrade
remain required acceptance surfaces.

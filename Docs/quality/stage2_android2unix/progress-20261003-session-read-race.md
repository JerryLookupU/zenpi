# Automatic Read Session Race And Cache Contract

Provisional S2-020/S2-033/S2-038/S2-053/S2-054 progress at
2026-10-03T19:12:53+0800. Previous goal turn was progress: Android draft and
confirmation scope repairs changed authoritative source and tests. This turn
inspected live HostClient/ResponseReader, cache/store policies, WorkManager,
network routing and host refusal behavior. No nested agents, worker launches,
checklist promotion, commit/push or cleanup.

## Repairs

SessionHostTest's newly added convergence check incorrectly compared a live
workspaceRevision against encrypted cache workspaceRevision. ProjectionCacheCodec
intentionally strips workspace owner/revision and capability metadata, so this
could never converge on a device with a non-null live revision. It now compares
the exact encoded display/cache contract instead. Live projection and desktop
checkpoint retain independent revision checks. New JVM coverage proves cached
display/cursor equality while stripping authority/name/live text, and rejects
changed session, cursor or transcript. Prior progress text claiming cache
workspace-revision matching is superseded by this finding and corrected docs.

Automatic foreground attach may observe the old session in workspace_list,
then race a desktop switch before ACK/subtab_list/resume/inspect. Host correctly
returns owner_mismatch for the stale request, but Android formerly interpreted
this as permanent authority loss even when the project remained authorized.
SessionChangeProbe now makes one new correlated authenticated workspace_list
on the same connection for only those automatic reads. A valid listed same
project with a changed session creates a scoped SessionChangedException. It
does not resend the failed read, publish its unconfirmed frames or retry any
write. Missing/unchanged project retains the original refusal, and revoked or
malformed probe data fails closed. No nested probing occurs for workspace_list.
Explicit page readers, background resume and all mutation/revoke paths stay
outside this route. Existing ViewModel foreground/network/pending/reset/scope
gates control recovery, followed by fresh authentication for the new snapshot.

Five JVM probe tests exercise permitted operations, single unscoped probe,
immutable original request, denied/nonautomatic/mutation exclusions, missing
or unchanged projects, malformed/duplicate/foreign data and revocation. A
ResponseReader test buffers old-session frames before a refused response and
proves zero publication while the new listing classifies the race.
Rust authenticated session-switch coverage now checks stale ACK/resume/
subtab_list/inspect still refuse OwnerMismatch and a fresh authorized listing
reveals the current session without mutating either journal. Host wire error
semantics are unchanged. Network/protocol/client docs record the extra race.

## Verification

All commands ran from /Users/mac/Github/zenpi. Rust used only controlled
ZENPI_HOME=/private/tmp/zenpi-resource-reload-test-home.

- zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-session-read-race-final-20261003: exit 0, production,
  unit and instrumented Kotlin compilation; 303 JVM tests pass, 1.224 seconds.
  Earlier cache-only check passed 298; intermediate probe check passed 302.
  Only the final 303 run covers all current sources. Instrumented tests are
  compiled, not device-executed or screenshot acceptance.
- rtk cargo test --offline --locked --test remote_subscription_tls
  authenticated_tls_session_switch -- --test-threads=1: exit 0, one passed,
  42 filtered, 0.09 seconds. Fragmented memory TLS/owned dispatch, not LAN.
- rtk cargo test --offline --locked --test remote_subscription
  --test remote_subscription_tls --test remote_auth -- --test-threads=1:
  exit 0, 69 passed, three suites, 14.64 seconds.
- Python session harness six, APK provenance ten, desktop harness fourteen,
  headless harness nineteen, backup validator five and lifecycle validator
  fifteen tests pass, all exit 0.
- cargo fmt --all -- --check; git diff --check; strict blueprint validator:
  exit 0; all 36 items unfinished.
- python3 tools/stage2_android_build.py --verify-only: exit 1, missing
  current APK provenance. No stale APK install, unchanged Gradle/ADB/TCP retry
  or sandbox bypass occurred. All required command handles are terminal.

## Digests

```text
SessionChangeProbe.kt 666c63a34f11a6e660e16be8f96dc99338e5ab5ecdc9b40ceb5183daa345347d
HostClient.kt 71ead280fc7955a32e9e74b307290ea80335653601b1a1847cf3c9bb89bdb3b0
SessionChangeProbeTest.kt 260bae08a531e1b61e8c4968115f29b0228e91e6daa943630ea16c7a3eafbb50
ProjectionCacheTest.kt ea08220abaff94073fe4797aeafa0f3eb6e3205997941a25f1eb130ddf32aa77
SessionHostTest.kt becdff933d57120997e5230e51291936b0835331c26b8a5f0ba17d7d785f1f60
remote_subscription_tls.rs 0ce08b2b3bdf00556c6ba21b546c8a59eec4de35b2bf51233ad733c7769bda28
main.jar a4a999347f1cf1de1b77d0937157e7a545e89a5f1e1f19d0dfe6ea7ea6181659
unit.jar 7beb1d9daa85c7d167fca73c6f37058d30bedd43f5ed85c006ef8dfd0826a01c
instrumented.jar 503946103d414ec29c54f1b36289e8c4065558d936021b1fa97a741d4e9d9a86
```

Goal remains active. Current APK, real scanner/physical direct LAN, connected
UI/lifecycle, full parity and signed reproducible release remain incomplete or
unverified. These source/protocol repairs do not substitute for those gates.

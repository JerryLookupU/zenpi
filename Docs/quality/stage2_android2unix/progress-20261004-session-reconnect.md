# Foreground Session-Switch Recovery Budget

Provisional S2-038/S2-053/S2-054 progress on 2026-10-04. The preceding Android
goal turn changed invitation expiry and reconnect behavior, so it was progress.
The intervening RTK request verified the installed tool, not Android completion.
Current execution skill, blueprint, dirty worktree, Gradle diagnostics, APK
builder, foreground refresh, session-change policy and route policy inspected.
No worker/nested agent admitted; no authoritative checkbox changed.

## Implementation

The current foreground catch path previously called refresh again after a
verified desktop session change. That replaced its generation and allocated a
new ReconnectBackoff, bypassing the common failure wait and losing any existing
failure budget. Recovery now clears the old live projection/cursor and transient
views, changes only the read target, increments the existing backoff and waits
before continuing the same coroutine. The next read reauthenticates and requests
a fresh snapshot. No captured mutation, invite token or revoke is resubmitted.
Pending writes, wrong scopes, offline/background and reset states still refuse
automatic recovery. Successful persisted publication remains the reset point.
An unconfirmed second session race cannot clear another cursor or restart this
recovery branch. Cancellation interrupts its wait through the existing coroutine
and generation fences.

Also recorded here is the prior turn's reconnect floor: failed foreground reads
sample uniformly from 250 ms through exponential ceilings 1/2/4/8/16/32/60 s.
The floor prevents zero-delay loops, including minimum random draws after a
publication reset. Healthy replay/renewal remains 200 ms/2 s. Backoff tests cover
the floor, cap, sampler range and cancellation at both minimum/maximum waits.
The new session policy test checks that recovery retains the prior failure
budget, refuses an unconfirmed second switch and resets only on publication.
This is policy/JVM evidence, not an executed ViewModel network recovery receipt.

## Verification

Commands ran from /Users/mac/Github/zenpi. Source validation finished before
2026-10-03 18:35:13 UTC (2026-10-04 local time); source hashes below bind this
provisional result. Existing deprecation warnings remain.

- zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-session-reconnect-20261004: exit 0; production, unit and
  instrumented sources compile; 439 JVM tests pass. Instrumented tests did not run.
- python3 -m unittest discover -s tools -p 'test_stage2*.py': exit 0; 126 pass.
- python3 tools/test_validate_stage2_android_lifecycle.py: exit 0; 21 pass.
- python3 tools/test_validate_stage2_android2unix_blueprint.py: exit 0; 5 pass.
- python3 tools/validate_stage2_android2unix_blueprint.py --strict: exit 0;
  36 [ ], zero [_]/[x].
- rtk cargo fmt --all -- --check and git diff --check: exit 0.
- python3 tools/stage2_host_build.py --verify-only: exit 0; current debug host
  attribution remains valid; Rust source was not changed in this turn.
- python3 tools/stage2_android_build.py --verify-only: exit 1; missing APK
  provenance. Existing Gradle diagnostics show global wrapper-lock write denied;
  prior isolated-cache diagnosis also records local daemon socket denial. No
  unchanged build retry, restriction bypass, stale APK installation or ADB run.

```text
HEAD 1a8ff1acf72e0816fad503a05df2b1d18e07361b (dirty integration, no commit)
blueprint d76e70e369f11f0fd3d1905d56d79c2a89401b92215da48ad85314f390d96d97
spec 403e6339b0318a420d93600fc52926acf6a9e46ec899bd9a9edb03c235190a28
PairingViewModel.kt 43a908e2c3d5a7eecd5c1bffbdd55816676b4f57609380d40b83a6a98977b2f6
SessionChangeTest.kt 29a0b430edbb51715b116ea29be6c05d9c6a86c214dc07e93c764f38134af9ce
ReconnectBackoff.kt 94186117e66e4639168742b1742f25e59a4cfbdae304975b33b418f46112d809
ReconnectBackoffTest.kt 8aabf6727cf816517f03fb954d0ae156186d1a68e6b04a675f04281c27307cb4
main.jar 1efa07e8ade820918ef0095a3b9e57710a9c97e50846ccec13b8762b56d913d2
unit.jar ca785f054ae9c9a4590a3c364a08a556cf191a047ce01f69460d304da7936703
instrumented.jar 1cc5d04a18c43c6701c04bc98023cefe2db88775126d23ead91a81e460cf5d65
debug-host-receipt d3aeb4ad7a6c6c1427246ca98464d1072f66663a0030fb231c8682a53d5b16f3
debug-host-inputs 6e853386d38b5ab7bd6667713d8c4eb057979731687da65a84cd99ce38a0b884
debug-host-binary 7218257a1d568a74bd8309b9682db79805f9a0eb74476504d15cb8daa0f7d694
```

Goal remains active. Current-source APK, physical camera QR/direct LAN
bidirectional sync and recovery, connected API 31/34/35 lifecycle matrix,
complete simultaneous-client parity and signed release remain incomplete or
unverified. Neither source compilation nor policy test counts accepts an entire
Blueprint item or proves these device/production transport requirements.

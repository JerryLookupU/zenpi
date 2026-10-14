# Stage 2 Fresh Disk Owner Recovery

Recorded 2026-10-04 UTC. Related S2-014/S2-019 routing, S2-036 client controls,
S2-052 security and S2-056 recovery. Partial evidence, all 36 Blueprint rows [ ].
Cwd /Users/mac/Github/zenpi, dirty-worktree HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b. Existing changes preserved; no nested
agents, reset, push, permission bypass or stale APK installation.
Blueprint SHA-256 d76e70e369f11f0fd3d1905d56d79c2a89401b92215da48ad85314f390d96d97;
specification SHA-256 403e6339b0318a420d93600fc52926acf6a9e46ec899bd9a9edb03c235190a28.

## New Evidence

Previous prompt/steering/cancel lane recreated HeadlessHost while retaining the
same Agent Arc. Strengthened it to construct a distinct Agent from disk session
and restore_checkpoint; Arc identity and project/session identity are checked.
RecoveryBackend would fail any execution and counts invocations: zero during
receipt lookup. Original three receipts match exactly. Recovered queue records
equal pre-restart records, including Applied steering and its application IDs.
Software fixture device is then revoked. All three identical original requests
refuse Unauthorized despite existing receipts; journal unchanged and backend
still uncalled. Kotlin consumes actual fragmented refusals and checks recovered
queue data, receipt identity, and ordered revoked requests.

No production runtime code changed. New Agent disk recovery is stronger than
same-Agent host restart, but is not OS process-crash, network reconnect, physical
Android Keystore, external provider or tool-effect acceptance.

## Verification

Commands ran from cwd above. Source checker controls ZENPI_ANDROID_* fixture
paths and isolated ZENPI_HOME=$OUT/prompt-home. No user config/credentials changed.

- zsh tools/stage2_android_source_check.sh /private/tmp/stage2-fresh-owner-recovery-20261004:
  exit 1 compiling new Kotlin assertion: Gson JsonArray is Iterable, not List
  for takeLast. Corrected to structured toList().takeLast; failure log retained.
- zsh tools/stage2_android_source_check.sh /private/tmp/stage2-fresh-owner-final-20261004:
  exit 0, 529 ordinary JVM tests, all source compilations, production QR corpus,
  every explicit interop lane, six-test desktop target and production tool-loop
  pass. Extended fresh-owner lane one Rust test/four Kotlin phases pass.
  No source/test edits after final run.
- python3 -m unittest discover -s tools -p 'test_stage2_*.py': exit 0, 137 pass.
- python3 tools/test_validate_stage2_android_lifecycle.py: exit 0, 25 pass.
- cargo fmt --check, git diff --check and strict Blueprint: exit 0.
- python3 tools/stage2_host_build.py and --verify-only: exit 0 after Cargo lanes.
- python3 tools/stage2_android_build.py --verify-only: exit 1, missing APK
  provenance; unchanged Gradle/ADB permission failures not retried.

Evidence JSON:
/private/tmp/stage2-fresh-owner-final-20261004/android-prompt-results.json.

```text
remote_headless_commands.rs aa9562e63dcea6b1454537b38c5b6f044365ad595a9111a033ba3ebdc61e0826
AndroidRustPromptTest.kt 228e5195f90f93bbe335f28ea86a91c31495ea0437db120abb6a2258174eeb2a
final-log 8a5a4605c7e08e3028e7f6ed627af2bec7fca688f7cc105edf069a1af7655170
prompt-results 86dee5f1c97539a36613aea2dd3df8750100c78fb1b96ff81cdcb96b66ff8ee0
python-log 3541981da88a0257a22d76d06e9dd4f50a5e503febcea44b1d18270b1c067e9d
unit.jar d91a66ede7270f474f3ae133b602c931c9c57fe521d1eac79354b7f01c286d11
debug-host-receipt 7645c79249cd9ba04b1f871b2fb1a2ce23c4198157f5ae14037e83941bcae458
```

Goal active and incomplete. Current APK, physical camera QR/direct-LAN sync
and recovery, connected API31/34/35 lifecycle, complete desktop parity and
signed reproducible release remain required; fixture recovery is not substitute.

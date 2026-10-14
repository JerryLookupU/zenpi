# Local Reset Session History Join

Provisional S2-038/S2-055/S2-056 progress, 2026-10-04. The intervening RTK
verification proved the installed Cargo proxy works but did not change product
state. This continuation revalidated current dirty integration, execution skill,
Blueprint, Android reset ownership and device-evidence consumers, then repaired
an actual missing cancellation join. No nested agents or checkbox promotion.

## Repair And Coverage

PairingViewModel.forget captured the connection and nine read jobs before
closeDiff cancelled and cleared their fields. Session history was cancelled by
closeSessionHistory but not captured or joined. Credential deletion could run
while that task's cancellation cleanup was still unfinished. The reset now also
captures sessionHistoryJob and cancelAndJoin waits before entering credential
removal under the existing storage mutex. No transport, protocol or host state
was weakened to accommodate the repair.

LocalResetTest's existing two-job regression now holds real coroutine cleanup
for the connection and all ten currently retained read-job fields independently.
It releases them one at a time, checking reset remains pending, saved host is
unchanged and signing identity exists before each release. Session history is
last, so omission of its join cannot be hidden by another held job. Final checks
require every job completed, reset cleared and both host store/key absent.
This test is instrumented-source compiled only, not device executed.

The test method identity changed without changing method count; desktop base
suite remains 31. The desktop harness and lifecycle validator now require
local_reset_all_owned_jobs_joined_verified. Old two-read phase/boolean receipts
cannot satisfy the new gate. Validator regressions exercise missing, duplicate,
unstructured and old phases plus an old receipt boolean.

The barrier covers these captured fields, not an assertion that every job ever
cancelled by another UI operation remains tracked. Nor does it prove cancellation
of already-running WorkManager network reads, uninstall, backup restoration,
power-loss durability or host revocation. Existing storage reset fences remain.

## Verification

Commands ran from /Users/mac/Github/zenpi, HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b, dirty integration preserved.
Observed time during verification: 2026-10-03T20:22:04Z, Oct 4 local time.

- zsh tools/stage2_android_source_check.sh /private/tmp/stage2-reset-all-jobs-20261004:
  exit 0; production, JVM and instrumented Kotlin source compilation; all 466
  ordinary JVM tests; current Rust read fixture and explicit Android-to-Rust
  rename, P256 proof, worktree and session lanes pass. Existing warnings remain.
  Source compilation does not validate resource merge, DEX, APK or device UI.
- python3 -m unittest discover -s tools -p 'test_stage2*.py': 132 pass, exit 0.
- python3 tools/test_validate_stage2_android_lifecycle.py: 22 pass, exit 0.
- rtk cargo test --locked --offline --test remote_events --test remote_faults --test remote_subscription --test remote_protocol --test remote_frame --quiet:
  55 pass, 1 ignored across five suites, exit 0; not socket/LAN acceptance.
- rtk cargo fmt --all -- --check, git diff --check and strict Blueprint pass.
  Blueprint still contains 36 [ ], zero [_], zero [x].
- APK verify-only refuses missing build provenance. Restricted Gradle/socket
  attempts were not repeated and historical APKs were not installed.
- Initial host verify-only correctly refused a binary changed by Cargo tests;
  current-source debug host was then rebuilt after those tests and separate
  verify-only passes. This is source attribution, not release acceptance.

```text
PairingViewModel.kt 3a4a4b8b927df5b34c584b91814829af06af4b6647f0eb106290a77cc7334528
LocalResetTest.kt b9bd87160ca5ccffa5d2f280ae9028c268fa62af4e5de4211b95461ec6d58da8
stage2_desktop_android_smoke.py 965b7256c1a6fb7d3e65a253a8c04f88ffe575a2bf6951efe3c2d09571c1e5eb
validate_stage2_android_lifecycle.py 4bea63085dd599457eb4c79afe11bd280e54bde5806d6a54909ebef6872b4ea6
test_validate_stage2_android_lifecycle.py 2302982ad9f6f7c32d75b7e4598a46e4286f0b16ed90775713a00256c1e2c81a
main.jar 91d7cafebb35b93edcbf99c8433cdda4a54065602219a5d940687e6f93452c89
unit.jar 2a06663fc58d3da438ea50f1b8dd164457632148b852d937f3b9688a0a0351d4
instrumented.jar ed1fe0c04d835c2b1cb84dce652e0bf40a96f498fff6c5fd9500128cfa9151a8
debug-host-receipt 54d40f4cc3022bb9680182148f2403e4ed6307f5eeb76b4134a1e40a3992b744
debug-host-binary 701d3e597cd640090cdc254f6ad2f810b986dbd626e42179ca3da6d8e50c5d77
```

The full objective remains active and incomplete: fresh APK attribution,
physical QR/direct-LAN bidirectional sync and recovery, connected API 31/34/35
lifecycle, full parity and signed release still require acceptance evidence.
This focused repair does not accept an entire Blueprint item.

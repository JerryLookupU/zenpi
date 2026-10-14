# Foreground Storage Admission

Provisional S2-031/S2-033/S2-037/S2-038/S2-055/S2-056 progress. Previous goal
turn fixed background storage cancellation and was progress. Current skill,
Blueprint, strict authority/build evidence, foreground pairing/save/intent/
confirmation/revoke call sites, shared store monitor and device test/harness
contracts inspected. Existing dirty integration preserved; no nested agents,
reset, commit, push or checkbox promotion.

## Defect and Fix

Foreground ViewModel writes checked generation/cancellation before entering
PairedHostStore, but its internal shared monitor may block behind another
store operation. Cancellation or connection replacement during that wait
could still allow a later credential/cache/pending-intent publication or
revocation key deletion. The prior background-only callback did not cover
these foreground methods.

save/saveProjection/beginWorkspace/finishWorkspace/beginRevoke now accept the
same optional checkActive callback. Each checks after acquiring monitor
ownership and forwards admission to the last check before AtomicFile.startWrite.
finishRevoke checks under ownership and again immediately before signing-key
removal; once admitted it retains existing durable reset behavior. Defaults
preserve direct store/test callers and trailing key-removal lambda compatibility.

Every corresponding production ViewModel write now passes captured coroutine
context and generation through storageAdmission. Host-success paths additionally
recheck captured device identity. Intent and typed refusal resolution check
context/generation without incorrectly requiring authentication for a local
refusal clear. Cancellation propagates; already-admitted atomic commits are
not promised rollback and existing pairing/reset reconciliation is retained.

Added ForegroundStoreTest with six operation paths and two instrumented tests.
The first holds the actual monitor, observes the IO thread BLOCKED, cancels
the coroutine and releases ownership. The second refuses at final admission.
Both assert identical encrypted bytes/state and no key-removal callback, then
admit the operation normally and check its intended result. Each namespace
and thread/job is fixture-owned with explicit join/wipe cleanup. These tests
compile but have not been device-executed.

Desktop harness has 35 base/87 workspace+Doze tests and requires both named
foreground storage phases/gates. The lifecycle verifier requires exact named
completion, unique full-line phase and boolean gate; tests reject missing,
duplicate, substring-only or removed-gate evidence. No historical reduced
suite receipt is promoted.

## Verification

Cwd /Users/mac/Github/zenpi, HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b. Final UTC
2026-10-03T22:07:05Z (Oct 4 local time).

- zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-foreground-storage-admission-20261004: exit 0 via
  pipefail/rg; QR fixtures, current main/unit/instrumented Kotlin compile,
  475 ordinary JVM tests and all current cross-language lanes pass.
- Python Stage 2 discovery: 135 pass; lifecycle verifier tests: 25 pass.
- Backup and Blueprint validator tests: 5 each. Strict Blueprint, Rust fmt
  and git diff --check pass; all 36 authoritative rows remain [ ].
- Actual debug Rust host rebuilt after Cargo lanes; separate verify-only
  confirms attribution. Binary/source inventory unchanged by Android-only fix.
- APK verify-only refuses missing provenance; no unchanged blocked Gradle/ADB
  retries, permission bypass, stale APK install or physical camera/LAN claim.

```text
PairedHostStore.kt f11779dc162c6e9e8f0c5c3201dd84a1ba0246352d30526439dc91e03d4a71a5
PairingViewModel.kt 94d6f6b08b3ef45d0fb6d198795028fc1d9d1e8f23751249170702f23b24a29a
ForegroundStoreTest.kt 247155bea36a1042994090f525e3465390527bcb74465c2198c853b702de9068
stage2_desktop_android_smoke.py 8c4c2f5f06815c4989996bf63ec9e0041534c6dd418b6a70a5a4afb0593de059
validate_stage2_android_lifecycle.py 4999edcdb0d13487c2d4157109ce538c1e8bcc9387a140aa7280e73319b96260
test_validate_stage2_android_lifecycle.py 8b0a54a296d8e40d944b6d675d3c380fc0f98503fbb7f4e8a93989b6220f3247
main.jar 03c17b54436bd68ba797eb5254389c256a26b3e784d52826926585b4dd05db13
unit.jar 1251c4a3ae844f3aa9ace8a4cbfe7fbdd75f7b18aabec0a4ef5db57c42bf2544
instrumented.jar f4d12e6a318318059210506a6a3538fa58fc525b911e3fb6f71fa9129e0ee7f4
debug-host-receipt 73b27607e33d30a02111dfa8b1922d36984ddf97950c85fe2b22db866ea09baf
debug-host-binary 8a826eef42d30492ace13ea37eb454d6c7819bf91251beb6942dcd683cd039e9
```

Goal active and incomplete. Source-attributed APK, real camera QR/direct-LAN
bidirectional synchronization and recovery, connected API31/34/35 lifecycle,
complete parity, migration/reset/uninstall and signed reproducible release
remain required unaccepted gates.

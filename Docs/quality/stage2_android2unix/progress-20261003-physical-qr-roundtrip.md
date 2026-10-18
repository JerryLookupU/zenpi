# Physical QR Android Prompt Roundtrip Gate

Provisional S2-031/S2-037/S2-051/S2-054 progress on 2026-10-03.
The preceding RTK check verified a local tool, not the product goal. This turn
revalidated the unfinished physical workflow and repaired its failing evidence
test before extending its device acceptance scope. Baseline HEAD is
1a8ff1acf72e0816fad503a05df2b1d18e07361b; integration remains an uncommitted
worktree. No worker admission, nested agent, checkbox promotion or commit/push.

## Workflow

PhysicalQrHostTest still uses the production camera scanner and requires the
operator to compare and enter the desktop safety code. Public expected
endpoint/certificate pin/project arguments are allowed; invitation URI, token
and safety code injection are forbidden. Existing app installations and
pairing/Keystore state are not replaced.

After registration, the device must see no ungranted projects. The fixture
grants Read for its temporary project, and Android renders the production
provider-seeded desktop transcript. The unchanged journal hash is captured
before granting Control to that same device/project. The actual Compose
message input and Send message button then submit STAGE2_PHYSICAL_ANDROID_PROMPT.
Android must render STAGE2_PHYSICAL_DESKTOP_ANSWER in the original session.
The answer comes from a loopback provider fixture through the production Rust
desktop runner, not an Android Echo handler. This tests the intended path when
executed; it does not assert the path has executed here.

Receipt v3 requires all seven ordered phases, the full named JUnit start/success
pair and production local reset completion. Its saved-evidence gate requires
three registry records (ungranted -> Read -> Read/Control), consecutive revisions,
the same device/project, unchanged Read journal, preserved original journal
prefix and exactly two appended user/assistant turns. Exactly one actual
provider request, a matching durable admission and running/succeeded progress
events bind the prompt to its device/request/project/session. The harness waits
boundedly for the durable completion event because UI rendering may precede it.
Provider input is parsed as actual latest user content, not a substring anywhere
in serialized JSON. Old v1/v2 receipts cannot satisfy the new roundtrip gate.

The initially failing negative test revealed that the shared binding helper does
not check admission result project/session ownership. The physical gate now
checks the production result shape explicitly. It also rejects malformed turn,
binding, admission and progress collections and Boolean provider counts. The
general desktop evidence helper is unchanged in this narrowly scoped repair.

## Verification

- Physical verifier: 9 deterministic tests, including foreign admission and
  completion owners, missing/malformed receipts, wrong Control grants and
  structured provider text. Stage2 Python discovery: 110 passed.
- Lifecycle verifier: 21 passed; Blueprint verifier tests: 5 passed.
- zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-physical-qr-roundtrip-final-20261003: exit 0, main/unit/device
  sources compile and 385 JVM tests pass. Device test is compiled, not executed.
  Existing Compose icon/API deprecation warnings remain.
- Rust debug source attribution, rtk cargo fmt --all -- --check,
  git diff --check and strict Blueprint pass. All 36 rows remain [ ]. Rust
  sources/binary are unchanged by this roundtrip work.
- python3 tools/stage2_android_build.py --verify-only: exit 1, missing current
  APK provenance. No stale APK installation, unchanged blocked Gradle/ADB
  retry, physical camera scan or direct LAN run was performed.

```text
stage2_physical_qr_android_smoke.py 3b7ab18f6ba0917ca29a67284ff5d2e83c31aac9be7af8db4f363f3db05d7d48
test_stage2_physical_qr_android_smoke.py 8e833f38679e2f31cdf6b429be6c6f2ba5fb40106f441a9f2e8d538cd1948971
PhysicalQrHostTest.kt 5ee2d3efc12b897d72e259e865779717cdd8b39d50242c0bc7bf27e260533de4
main.jar b13cd1f0fbd5ea97bf87bfcc47020fa4eec3628e625cf5b1f7fac224a80fe9d3
unit.jar d6e97fcb83b5e200852163830fcb7fe9a9ca8cfe88947267c5098edb718ea188
instrumented.jar 41eca19771a311a8bdab5ea643f6d50f1744faedbf709c7797def9f27916488c
debug-host-receipt 7020b95d66d5de27c6e83e403914087f6731954002d9fe0b2f56c19f5b81367a
```

Goal remains active. A current-source APK, actual physical QR/direct LAN
roundtrip, complete desktop parity, API 31/34/35 lifecycle and signed release
acceptance remain incomplete or unverified. This gate does not accept an entire
Blueprint item and is not cryptographic attestation of a device run.

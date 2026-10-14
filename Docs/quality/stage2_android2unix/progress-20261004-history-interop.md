# Android/Desktop Conversation History Interop

Provisional S2-036/S2-054 progress. Previous goal turn fixed history read
admission and rechecked APK/ADB restrictions, so it was progress. Current skill,
Blueprint/specification, worktree, host transcript tests and current Kotlin
history policy/codec inspected. Existing changes preserved; no nested agents,
reset, push or checkbox promotion.

## Added Evidence

New AndroidRustHistoryTest encodes current framed history requests from actual
workspace/inspect scope and decodes fragmented ProjectRuntimeHost responses.
The new explicit remote_android_model_interop history lane uses 70 durable
session turns and an authenticated Read-only software P256 fixture device.
Five pages at offsets 0/16/32/48/64 cover every turn ID in exact order, counts
16/16/16/16/6. Old history-0 is absent from the 64-turn live snapshot but can be
selected through a transient validated history projection without changing
that live projection. Kotlin checks read policy, chain, scope/digest rejection,
redaction and page counts.

Foreign sessions fail owner_mismatch for each page. After full coverage,
appending a host turn makes the old journal revision fail revision_conflict;
revocation then fails unauthorized. Kotlin decodes these actual refusal frames.
Reads preserve the actual journal bytes and workspace revision, and responses
omit fixture credentials/absolute roots. No provider or socket is invoked.

The source checker excludes this integration class from ordinary JVM discovery
and explicitly executes it with freshly compiled Kotlin classes. Fixture skips
or ignored-by-default Cargo registration do not count as execution evidence.

## Verification

Cwd /Users/mac/Github/zenpi; HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b; UTC 2026-10-04T00:13:18Z.

- zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-history-interop-20261004: exit 0; 506 ordinary JVM tests,
  main/unit/instrumented Kotlin compilation, current terminal QR decoding and
  all explicit current Rust/Kotlin lanes pass. Model/resources/files/diff/
  history target now explicitly executes 5 tests, all passing.
- Retained history exchange JSON inspected: five successful pages, five foreign
  session refusals, final journal-change and revoke refusals.
- Python Stage 2 discovery 135 pass; lifecycle verifier tests 25 pass.
- cargo fmt --check, git diff --check and strict Blueprint pass; 36 rows [ ].
- Debug host rebuilt after Cargo lanes and verify-only passes.
- APK verify-only refuses missing provenance. No unchanged Gradle/ADB retries,
  stale APK installation, permission bypass or connected UI claim.

```text
remote_android_model_interop.rs dcfd878b650f446ba54fe8243a75b136a1d5016a0545d5322ed310dacd1d4be6
AndroidRustHistoryTest.kt 4c98a06db476806d549546c6fb9b7f7e9e8ad4d75d58b23dcf55a8bd92ee3c40
stage2_android_source_check.sh 74cf817aadc3a84803b1a9842ec2e88033b4e55f9df9c7ff8af38f560e913767
source-log 61b01319b0c642f6506d5cd05b3830c72b3492a0c61bbb4f8cdf1dbcc79554a7
history-results 7a977729a3c477bc557edabd42258d29a630067819ae2cd4d2a5f7fa779466d9
main.jar 5911009d658d4543d87f7b2a0b8d67eef910130686e58ff1d479f41df75818fe
unit.jar 24008d61f564cd2d87d9705be9260d5288f0b922c61d687b928ee64cb7d686b2
instrumented.jar e8fea581c770f2ffda2803e3e8a546b670c831e1c51b7b005e91fdee161e0a4b
debug-host-receipt 7852beae4d61e1af69549b6de8c67c5fe72883138a5ddd12e8915d49516587e3
debug-host-binary 51cf433e9ebfd59aeebdbf46f844c0603872e798f8e4b6443308aed8627eca67
```

Goal active and incomplete. This is actual desktop journal/ACL and JVM framing
evidence, not listener TLS, Android Keystore, current APK or physical QR/direct
LAN sync/recovery. Historical-message detail device workflow, connected API
lifecycle, complete desktop parity and signed reproducible release remain gates.

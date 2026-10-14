# Conversation Read Admission And Device Gate Recheck

Provisional S2-036/S2-038/S2-054 progress. Previous goal turn implemented session
catalog read admission and was progress. Current execution skill, Blueprint/
specification, worktree, Android build/device gates and conversation history
codec/UI/ViewModel/tests inspected. Existing changes preserved; no nested
agents, reset, push or checkbox promotion.

## Device Gate Recheck

Actual python3 tools/stage2_android_build.py failed exit 1 at the global Gradle
wrapper zip lock with Operation not permitted. Current diagnostic receipt and
private logs retained; no provenance published. Actual SDK adb devices -l also
failed exit 1: server cannot install tcp:5037 smartsocket listener, Operation
not permitted. This prevents device enumeration; it does not prove that no
physical device exists. No bypass, stale APK installation or simulated success.

## Source Fix

Conversation backward/message navigation already had scoped history policy,
but initial read/refresh used separate UI/ViewModel guards. Initial codec
requests accepted selected IDs without a unique listed project/session or
canonical u64 journal revision; forward reads did not require the same open
history navigation gate. Refresh UI omitted reset scope via mutationEnabled.

Added SessionHistoryReadPolicy.browse and SessionHistoryCodec.readable, shared
by entry/refresh/ViewModel. They require connected unique current scope and
canonical u64 journal revision, with scan/reset/loading/pending write/revoke/
open transcript-message fences. Forward uses the existing open error-free
valid-chain navigation gate. Read-only/running-task browsing remains eligible;
stale/broken/failed pages can refresh against current authority, not navigate.
Codec fails before dialog/key work. No journal mutation or write capability.

Three JVM tests cover read-only/running/error refresh, lifecycle/unknown writes,
missing/duplicate/foreign scope, canonical journal bounds including exact u64
maximum, closed/stale/broken pages and initial request rejection. Extended the
existing conversation Compose case without adding a named device test.

## Verification

Cwd /Users/mac/Github/zenpi; HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b; UTC 2026-10-04T00:08:43Z.

- zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-history-read-admission-20261004: exit 0; 506 ordinary JVM
  tests, main/unit/instrumented compilation, current terminal QR decoding and
  all explicit current Rust/Kotlin lanes pass. Model/resources/files/diff 4
  pass; device UI compilation is not connected execution.
- Python Stage 2 discovery 135 pass; lifecycle verifier tests 25 pass.
- cargo fmt --check, git diff --check and strict Blueprint pass; 36 rows [ ].
- Debug host rebuilt after Cargo lanes; verify-only passes.
- Actual APK/ADB gate failures above remain unaccepted external constraints.
  Gradle attempt preceded this source edit and is failure diagnosis only.

```text
SessionHistoryReadPolicy.kt ec1630a34332d57a744b9e585ea4ff052197e040979e34610e92f9cdd985a8e6
SessionHistoryCodec.kt 44db0e88e0ef2d2eeef426c78523be0053c5b5d4a39e0eff77b20a2b84f0d1f1
PairingViewModel.kt 2738852260b47dc7ef973af17109790c4a6f907d870facc887ed7caa734f050a
MainActivity.kt 3282c6503c0dfa2c792184671d38189c88576bc52e64442fba91204238743331
SessionHistoryReadPolicyTest.kt 62683aae40b82db6822ce6edbd8215f195251abae9c64fac8b80279cf300a565
SessionCatalogScreenTest.kt 4205bdf8868a69249268fd74328d228485380f00f284495853709e38d55972ff
source-log d8ce28682d9661856ec316789a9b31b5e81e797843c670184354eccd6f893d74
main.jar 5911009d658d4543d87f7b2a0b8d67eef910130686e58ff1d479f41df75818fe
unit.jar ed471b02f16c411c9fb28f444f2014a944246003668a14e4360f49cd7c5b2a73
instrumented.jar e8fea581c770f2ffda2803e3e8a546b670c831e1c51b7b005e91fdee161e0a4b
failed-apk-attempt ba4b8e342e4679e51762b5e5e15a9dcc7e5e2a99f0919832256e5d285ba4b557
debug-host-receipt 7104c70f7643a814e7791ef51e167c496e6ca31309d1c95c7116794a8530dd3e
debug-host-binary 51cf433e9ebfd59aeebdbf46f844c0603872e798f8e4b6443308aed8627eca67
```

Goal active and incomplete; meaningful source progress remains possible.
Current APK, physical QR/direct-LAN bidirectional sync/recovery, connected API
lifecycle, complete desktop parity and signed reproducible release remain gates.

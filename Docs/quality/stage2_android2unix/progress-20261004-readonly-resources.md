# Read-Only Android Resource Browsing

Provisional S2-036/S2-054 progress. Previous goal turn implemented session
resume admission, so it was progress. Current skill, Blueprint, dirty worktree,
resource codec/host Read access, UI controls, ViewModel and Compose/source tests
inspected. Existing changes preserved; no nested agents, reset, push or checkbox
promotion.

## Defect and Fix

Host resources_list is a read operation and the ViewModel supports it without
prompt authority. Android nevertheless placed its sole entry inside the prompt
composer and tied refresh/paging/reload buttons to prompt control and idle prompt
status. A Read-only device could not use the host's resource metadata capability.

Added ResourceReadPolicy shared by Compose and ViewModel. It permits connected
scoped metadata browsing independently of prompt/reload capability and fences
scanner/reset/loading/pending write/revoke states. Scoped navigation also needs
a successful current catalog. Insertion into the task draft separately requires
prompt control and no accepted/running prompt. Reload retains its independent
advertised capability and existing captured catalog/CAS admission; it no longer
needs unrelated prompt authority. No send/reload is triggered by metadata reads.

Read-only clients now have a Browse host resources entry. They can view entries,
refresh failures and navigate pages without a composer, send button, resource
insertion or reload authority. Existing controlling composer entry is retained
without duplicate read-only entry. Active task execution can still fence writes
without unnecessarily hiding read-only paging.

Three JVM tests cover read-only browsing/insertion/reload separation, running
prompt behavior, scope uniqueness and lifecycle/unknown-write fences, and
refresh after a failed catalog. Existing Compose tests were extended, preserving
named instrumentation count: read-only metadata/navigation/error refresh,
absence of send/reload, and separate reload control without prompt control.
Current AndroidRustResourceTest consumes the actual Rust Read-device page and
checks read-only navigation policy. Compose tests compiled only.

## Verification

Cwd /Users/mac/Github/zenpi; HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b; UTC 2026-10-03T23:11:51Z.

- zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-readonly-resources-20261004: exit 0; 491 ordinary JVM tests,
  current QR fixtures, main/unit/instrumented compilation and all explicit
  Rust/Kotlin lanes pass, including actual Read-device resource continuation.
- Python Stage 2 discovery: 135 pass; lifecycle verifier tests: 25 pass.
- Rust fmt, strict Blueprint and git diff --check pass. All 36 rows remain [ ].
- Debug host rebuilt after source-check Cargo lanes; source/binary unchanged
  by Android-only fix and current attribution refreshed.
- APK verify-only refuses missing provenance. No unchanged Gradle/ADB retry,
  permission bypass, old APK installation or physical QR/LAN acceptance claim.

```text
ResourceReadPolicy.kt 3d9b0f126eaa66242734c432ebb624f3f61f493997781a528137fc633a3a2ac8
PairingViewModel.kt ccce3d65f6615dd3fc2969b77ff1ee3ba40e3dc86092d31b2ba799191553a891
MainActivity.kt c16c87c8743fb3b9cf2a6ad7bdf10f7859250486c71e327cf03ae662c6b0a54e
ResourceReadPolicyTest.kt aae4ec43984f0d4f7122be2691956036117ed2a1796371924caf89a7d65cd158
AndroidRustResourceTest.kt 533ef39f2b58a2767015d2dcb3f412a7c4c64b708d0f856b00ae2c4a20079fe9
ResourceCatalogScreenTest.kt 64462afb9948ce3f1221b618e1c1e3420d6353cafbb0b52bf7a96ed68e8b8fce
source-log 1bce9f77d23682796887e6c7773c7ed374becf40d0052ca995e0d762f888aad9
resource-results 4a8a12b885bfdee903fa6f717d4649f95fe7cb3728b8ac1178c32f97601f5fa5
main.jar ba6d394d8bbc63b904513e541a6e20354f144b0ea36f08eeb178ad4bcb78039e
unit.jar 581cee668b55734bbb3e4877b859eb96324bf0a82403ae0e0b3e18ae0df67e4e
instrumented.jar c7e3cffee4e260a7c8f2f9b4de442a00512824d18d1c25d9f900336da3e9c62e
debug-host-receipt 6f6d3b029dd5292f9c02198243f71b88974a6f02ef544cc56f6233e00ecdf3eb
debug-host-binary 51cf433e9ebfd59aeebdbf46f844c0603872e798f8e4b6443308aed8627eca67
```

Goal active and incomplete. Current APK, physical QR/direct LAN bidirectional
sync/recovery, connected API lifecycle, complete desktop parity and signed
reproducible release remain required unaccepted gates.

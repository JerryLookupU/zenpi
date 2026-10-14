# Current Session Resume Admission

Provisional S2-036/S2-054 progress. Previous goal turn added resource interop
and reload admission, so it was progress. Current execution skill, Blueprint,
dirty integration, session read/resume/retry paths, catalog codec, dispatch,
Compose and interop tests inspected. Existing changes preserved. No nested
agents, commit, push or checklist promotion.

## Change

New session_resume dispatch previously checked current session control and main
tab but did not revalidate the displayed catalog snapshot or selected target.
A changed catalog need not switch the active session or workspace revision.
The host would reject a stale snapshot, but Android could first persist that
obsolete command as a newly admitted intent.

Dispatch now requires a present successful nonloading catalog, current unique
project/session listing scope, the captured snapshot digest and exactly one
noncurrent target. SessionCatalogCodec.sameScope also checks the unique listed
project/session, not only top-level selection fields. Its shared resumable
predicate refuses a target equal to the current session even with a false
current flag, and rejects duplicate target records including mixed current
flags. SessionCommandCodec and Compose use the same target/scope predicate.
The dialog additionally fences scanning/reset/inconsistent catalog state.

Exact pending-envelope retry still precedes new-command checks and remains
subject to foreground/network/reset/scanner/revoke fences. It is not rebuilt,
retargeted or automatically submitted. Host CAS remains the final authority.

Two new JVM tests cover missing/loading/failed/foreign/new snapshot/removed/
current/duplicate catalog targets, inconsistent or duplicate project listing,
refreshed command and exact pending retry. Current AndroidRustSessionTest now
uses the actual Rust-mutated catalog to refuse the old new-command admission
while retaining the saved retry identity. Existing Compose test adds foreign
catalog/listing and scan/reset disabling. Compose compiled only, not executed.

## Verification

Cwd /Users/mac/Github/zenpi; HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b; UTC 2026-10-03T23:07:49Z.

- zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-session-admission-current-20261004: exit 0; 488 ordinary
  JVM tests, current QR fixtures, production/unit/instrumented compilation and
  all explicit current Rust/Kotlin lanes pass. Prior initial run was superseded
  after duplicate/current target hardening; no source edit followed final run.
- Python Stage 2 discovery: 135 pass; lifecycle verifier tests: 25 pass.
- Strict Blueprint, Rust fmt and git diff --check pass; 36 rows remain [ ].
- Debug host rebuilt after Cargo lanes and attribution refreshed, unchanged
  host source/binary hash for this Android-only fix.
- APK verify-only refuses missing provenance. No unchanged Gradle/ADB retry,
  permission bypass, old APK installation or physical QR/LAN acceptance claim.

```text
SessionCatalogCodec.kt 17ea3b5f82826cd0f971b587d5d6c0d1bbb3c228885572515bc9759aa0c59634
SessionCommandCodec.kt 837349d302c068eac5248ff5b3c760d2223a4dfb30baaeab72515d5abbde9075
WorkspaceDispatchPolicy.kt 1850a6f4ae4a832d0e43396fb010adb811cc08c89bb394bb14225d302be410ef
MainActivity.kt a620b26eb421b53659648da30e77ca5feee628b9f779fa15871d03ba4118046a
SessionCommandTest.kt 8361dc95c50759b089a7a0d209a8c8c78efd0907d90118317692fb5a26287d6a
AndroidRustSessionTest.kt ddabf9823f8cf6e888d49ba6c71d4db667a1f4dacb13b71ad3dd1a416925f27b
SessionCatalogScreenTest.kt ee039cb666b140e14c0c9788c03fc67e1d3852f385562a7b64a0c5d26e5d7a34
source-log 1bf66ac1efebea01e92f7d07d492d7769d85ae722ea815298acf4abb08aba89b
session-results 4298386970253e21e8ccc1860362f03f9f86b84ce2c871312b6f49645c1719bf
main.jar c9208fc2a17134b7c9db95e20c3fa972696a960a9602bc11bcfe4f12366084d5
unit.jar 6cc26c23e68c371de9f6efaf251b504ab3f8248935c3ad7558b328a721e119c6
instrumented.jar 094dd770d5b7793b6f47150ae8a2f60b97b8d18a1e6690cd7b3db7a0461f5c30
debug-host-receipt d98bd5d4aa2e79c7691bdee7fb80a37bebe4b11cce8f86e42aaa173f598cbd25
debug-host-binary 51cf433e9ebfd59aeebdbf46f844c0603872e798f8e4b6443308aed8627eca67
```

Goal active and incomplete. Current APK, physical camera QR/direct LAN two-way
sync and recovery, connected API lifecycle, complete desktop parity and signed
reproducible release remain required unaccepted gates.

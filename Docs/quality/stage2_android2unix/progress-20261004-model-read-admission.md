# Model Read Admission And Read-Only Catalog

Provisional S2-031/S2-036/S2-038/S2-054 progress. Previous goal turn added real
worker/tool interoperability and was progress. Execution skill, current
Blueprint/specification, dirty worktree, model UI/ViewModel/codec and actual
model interop lane inspected. Existing changes preserved; no nested agents,
reset, push or checkbox promotion.

## Source Fix

Model entry and reads omitted scanner state, and initial wire construction
accepted selected IDs without verifying listed project membership. Added
ModelReadPolicy shared by UI, ViewModel reads and configuration preflight:
connected, no scanner/reset/loading/pending write/revoke, one uniquely listed
selected project/session. Successful open scoped catalogs can continue;
current selection revision or catalog snapshot replacement fences old pages.
Errors can refresh but not continue. ModelStatusCodec request/decode/merge and
sameScope reject missing, foreign-session and duplicate selected projects.

Catalog display/paging was nested under modelControl and inaccessible to
Read-only clients. Moved it outside the mutation-grant condition. Read-only
clients can inspect the catalog and page it; catalog choices stay disabled
without modelControl and apply controls retain registry/selection/reasoning
checks. Running host sessions remain inspectable. Exact unresolved write
resolution stays in WorkspaceDispatchPolicy and is unchanged.

Two new JVM tests cover scanner/reset/pending/load refusal, read-only/running
admission, refresh versus failed continuation, replaced catalog/selection,
unlisted/mixed duplicate scope and merge refusal. Actual Rust/Kotlin model
interop assertions exercise read-only/scanner admission against host views.
Existing Compose cases add Read-only catalog paging and scanner/reset/duplicate
scope disabled controls plus error refresh recovery. No new named device case;
instrumentation is compiled only, not executed.

## Verification

Cwd /Users/mac/Github/zenpi; HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b.

- First full checker /private/tmp/stage2-model-read-admission-20261004 exit 0:
  517 ordinary JVM tests, 54 production terminal QR cases, all three source
  compilations and explicit Kotlin/Rust lanes, including six-test desktop
  target and actual tool-loop target.
- Final /private/tmp/stage2-model-read-final-20261004 complete checker exit 0
  after configuration preflight/instrumented assertion edits: 517 ordinary
  JVM tests, 54 QR cases, all source compilations and all explicit lanes pass.
  Instrumentation compiled only, no connected-device execution inferred.
- Python Stage 2 discovery 137 pass; lifecycle verifier tests 25 pass.
- cargo fmt --check, git diff --check and strict Blueprint pass; 36 rows [ ].
- APK verify-only refuses missing provenance. No unchanged Gradle/ADB retry,
  permission bypass, stale APK installation or physical camera/LAN acceptance.
- Debug host build and verify-only pass after Cargo lanes.

```text
ModelReadPolicy.kt cd3001cb3e188e7f15061b86a4cba226b797221161a2a38ee9d29e1216afa7dc
ModelStatusCodec.kt d702bf36d89e4001acf5817482d04e95fc44486ccf0e50e822774eca0e4c8d42
ModelReadPolicyTest.kt b222540d4ec9afe87b3041dc8036129dddf7a3051e8c2b513312cae1e2d92e81
MainActivity.kt 28d24fa97941e38d97ee81fb33d9a9b3fe57f6818387f8e494624760656266f0
PairingViewModel.kt 31185d28eadfaef150722a0d554ae04950f3d970b0c257d6ff0605fb5d738d4f
ModelStatusScreenTest.kt 723f4bc152ffb9148f2289bf39e5e1ac18bf4208621c460e1b606bc3b56d918b
AndroidRustModelTest.kt 0f9929b66cb0df43c4b70ac1f8c33d893124e3adc1939a11a80c479799ec6532
final-source-log 2291889af63b02be22afa9ade5e74f5db90393ba733954e782f713ab70b0c0d0
python-log 83fc10b5cc18d9d5f61b8b154a95855f1e07d3071606310920e6c1e23f75cb3c
main.jar 64d59669a6c243973bd432ec20b37bdb59421ad2876fa8bb1256454747cd621f
unit.jar 4443e49a01337f85b5c4e59809d9434493796d9fff9774befe68253e43527b9d
instrumented.jar 721919fbee6d167cc0bea2c7e23fc4f80fe52be44a8f9c55fa344ebb059deb0e
debug-host-receipt 4201c5c4828975e7753fbf6406584cd37024e8d16d9dcc7ce2b80dd0d6ecbd26
```

Goal active and incomplete. Current APK, physical QR/direct-LAN bidirectional
sync/recovery, connected API31/34/35 lifecycle, complete desktop parity and
signed reproducible release remain required. Source/JVM evidence is not device
or LAN acceptance, and no whole Blueprint item is promoted.

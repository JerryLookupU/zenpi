# Kotlin Rust Session Replacement Interoperability

Provisional S2-019/S2-036/S2-054/S2-056 progress, 2026-10-04. Previous turn
repaired authenticated identity boundaries and isolated subtab configuration
tests, so it was progress. This continuation inspected the current dirty
worktree, execution skill, Blueprint, Kotlin session/catalog codecs, Rust
session transactions and the current APK gate. No nested agents, unrelated
reset or checklist promotion.

## Current Owner Transaction Lane

New remote_android_session_interop.rs creates actual original/history journals,
production OpenAiCompatibleBackend/Agent/ProjectOwnerPool and a P256-authenticated
Control device. AndroidRustSessionTest decodes Rust's real workspace/inspect/
subtab/catalog projections and builds production SessionCommandCodec requests.
Rust parses these framed commands and executes HostAdapter. Kotlin validates
the returned wire receipts in five-byte fragments with ResponseReader and
WorkspaceReceiptCodec, then decodes the current host projection and catalog.

Five sequential steps cover new session, stale-catalog refusal, history resume,
original resume and another new session. Every successful replacement first
refuses a locked owner as busy without consuming the request identity. The
same request then succeeds after unlock, advances workspace revision once,
changes the actual journal and input port, preserves subtab identity and
selection, and returns an identical immediate retry receipt. Stale revision
and changed same-ID envelope are refused. The pool is dropped and recreated
from the current checkpoint after each success: retry still returns the exact
original receipt without another journal change or revision increment.

Owner epochs are intentionally process-local: a session replacement retains
the live pool owner, but restart rotates it. Durable exact-request receipts
remain usable across that epoch change. Kotlin checks the old command cannot
be admitted as a fresh write against the restarted projection. The old catalog
cannot build a resume after session replacement. A separately added journal
invalidates a captured catalog digest before resume; the refusal leaves the
current owner/view/revision unchanged. Original and historical journal bytes
remain unchanged throughout all steps. Resumed content and empty new-session
views are checked by Kotlin, not only by Rust receipt assertions.

Ten explicit Kotlin helper executions (five generation/five validation) pass;
25 real response frames are validated. The source checker runs this lane after
read, rename, proof and worktree interoperability. Ordinary JVM test counts
exclude all phase helpers; normal Cargo ignores this lane without freshly
compiled JVM classes, and normal Gradle assumption-skips its helper. No
provider request, TLS/socket, Android Keystore, camera, APK, physical LAN,
simultaneous desktop UI or full product parity is proven by this source lane.

## Verification

Commands ran from /Users/mac/Github/zenpi against current dirty integration.

- zsh tools/stage2_android_source_check.sh /private/tmp/stage2-session-interop-verified-20261004:
  exit 0; Rust current read fixture; Kotlin production/unit/instrumented
  compilation; 463 ordinary JVM tests; explicit rename/P256/worktree/session
  lanes pass. Existing warnings remain. Instrumented tests compiled only.
- Python Stage 2 discovery: 132 pass. Lifecycle verifier: 21 pass.
- rtk cargo test --locked --offline --test remote_subtab --test remote_migration --quiet:
  exit 0; 17 top-level tests (11 subtab, 6 migration), three isolated subtab
  child executions; RTK aggregate 20 pass/30 filtered. Not a whole Rust suite.
- Rust format, git diff --check and strict Blueprint pass; all 36 rows remain
  [ ]. APK verify-only still refuses missing provenance. No unchanged Gradle
  socket-policy retry, prior APK install or device acceptance assertion.
- Debug host rebuilt after all Rust invocations; verify-only passes separately.

Initial session lane failed because its Kotlin harness incorrectly expected
the owner epoch to remain unchanged across restart. Inspection confirmed the
production pool generates a fresh owner nonce at startup. The harness now
checks retained owner before restart and a different owner afterwards, while
requiring the durable receipt to remain identical. Production behavior and
receipt validation were not relaxed to make this test pass.

```text
remote_android_session_interop.rs 1400e35026a98b86aec8963304806903b23ca9e903f02e59d30c7101a1406d44
AndroidRustSessionTest.kt cd3ed1248d0134c1b44892387bff5468f3af123b3acc0dd1f60664b8ddef9e98
stage2_android_source_check.sh 5f71b598bc208c24f9bb979059b7824ea81a7ae6c7e5603db395454759c4a142
android-session-results.json d261818e296946ee37eaaedc75ae3e55dbf9b1641cda06dc0aabb1dea46e474d
main.jar 6ef1c8589a2f8bf4c5d722553621c256a71e82fd3e11945699f35dbb633e5ad8
unit.jar d72db3cafe5180cd83cdbd2f04c9ed4e5655ef0dc5b8f999d291527af9ec11eb
instrumented.jar dbba336085f0166529df062ba407376e7746c0f182356e41f46ba9fe6c319ebf
debug-host-receipt 152163c3668c7baf3981b6c14711724a2a5cfaf17023da3589a6cb8daeb0ade3
debug-host-binary 701d3e597cd640090cdc254f6ad2f810b986dbd626e42179ca3da6d8e50c5d77
```

Goal remains active. Current APK, physical QR/direct-LAN bidirectional sync and
recovery, connected API lifecycle matrix, full parity and signed release remain
incomplete or unverified. This transaction lane does not accept an entire
Blueprint item or supersede previous wider-suite failures.

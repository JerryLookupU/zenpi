# Selected Project Access Loss

Provisional S2-033/S2-034/S2-037/S2-038/S2-052/S2-053 progress at
2026-10-03T21:13:12+0800. Previous goal turn produced verified terminal history
scope changes. Inspection found a successful authenticated listing that omitted
the selected project threw a generic transport exception: foreground Error
could retain old cancel/approval controls, and background resume retried an
unavailable project. No nested agents, worker launch, checklist promotion,
device launch, commit/push or unrelated cleanup.

## Changes

Validated selection raises SelectedProjectUnavailableException after project
listing and active-ID validation. This local typed result is not a new wire code
and does not assert device revocation; a project may have closed or its ACL
changed. Malformed listing, generic text, timeouts and confirmed session-switch
recovery remain distinct.

AuthorityLossPolicy recognizes the typed loss. Existing foreground handling
cancels dependent reads, removes all old controls/transient views, retains
display/cursor, identity and unresolved requests, and stops that read loop.
ResumeRetryPolicy treats it as terminal; ResumeReadPolicy does not issue a fresh
session-recovery read. A transient in-memory unavailable-selection flag makes
the next foreground/default or manual refresh request an authorized current
selection instead of the removed project. Successful publication clears it.
The flag is not persisted; a new process must authenticate and detect loss again.
Unknown writes and revokes retain their original request, never become success
or get replaced by a new mutation through this path.

Three new JVM tests cover typed selection loss versus malformed metadata,
preserved pending identity with no new controls, and framed workspace-list
selection stopping before simulated ACK/read/commit followups. Existing retry
and session recovery tests check terminal/no-extra-read handling. Existing
PairingScreenTest checks that Refresh host selects null on loss and the current
ID normally, without changing instrumentation count. These are policy/reader
fixtures and compiled Compose tests, not a connected ACL-switch workflow.

## Verification

- zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-project-access-loss-verified-20261003: exit 0; production,
  JVM and instrumented Kotlin compile; 361 JVM tests pass. Existing warnings
  remain; Compose/instrumented tests compile only.
- env ZENPI_HOME=/private/tmp/zenpi-project-access-loss-home cargo test --offline
  --locked --test remote_auth
  short_lock_release_reloads_current_permissions_instead_of_a_cached_grant
  -- --exact --test-threads=1: exit 0, one pass. Registry authentication reload
  fixture, not current TCP/Android/LAN acceptance.
- Python Stage 2 discovery: 93 pass; lifecycle verifier: 17 pass;
  instrumentation verifier: five pass; Blueprint verifier unit tests: five
  pass. cargo fmt --all -- --check, git diff --check and strict Blueprint
  validator pass; all 36 checklist rows unfinished.
- Debug host attribution rebuilt after Rust testing; verify-only passes.
  Release verify-only passes unchanged. Android --verify-only still refuses
  missing APK provenance. No stale APK, unchanged blocked Gradle/socket retry
  or permission bypass. All observed sessions are terminal.

```text
main.jar a4ada458b9844220aeb5dc7c7d8379bcae5a17d7e72f778c5db57ab8d858d76f
unit.jar 69bad05f0867d51ed0fdc744a13a93815de348a4b37e2001cc4b16c79cbf055f
instrumented.jar 48d4d849f6f4ddb1aa429a94e5731f2ab78729e44e9ac7514b3e3b0a72b9145e
FrameCodec.kt 0ac9d28b0bfad4cecb634cae4c7817d066e8f413435d7233574defa5a6b4194c
HostClient.kt 2cbd9fc09ff74442086cded41dafc2488167256a14b7c360230d6cec18611643
AuthorityLossPolicy.kt a5cc06daa708330b57271bd4aacd748be193e0d102f5fc4ea4832405b71d2397
PairingViewModel.kt 3a6ff00cbc89a4b91484ba5fd99e230abfe6bcdd0c447e4c9ed033e9026caa7d
ResumeWorker.kt c1655ae796d518dc96dbea4cdda8a3c745f2d5e80811d7d5c7c7ee57f79651db
MainActivity.kt 842cab92a72be07398944e04266a7fa3aba3f3ae4fe3dc1b49e5674ca770a845
HostProjectionTest.kt 2074fe0cf149604aba9c2aff9ea6ad2edcb0096aea2cf3022ddad224d2a88d23
debug build receipt eb50719bdaf4e0d740fa890b1bbf9ccecbb6a660a5986e979a911a04d3f6edce
debug binary 0146a6732f7d5ffb1be4ab532e3724dc6dcf75b90caadcc1234384fe9a6fa788
```

Goal remains active. Current-source APK, production desktop/Android TLS sync,
physical QR camera/direct LAN, API 31/34/35 connected lifecycle, full parity and
signed install/upgrade require current behavioral evidence. Source checks are
not a substitute for those end-to-end requirements.

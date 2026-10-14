# Host-Backed Session Conversation Search

Provisional S2-036/S2-054 progress on 2026-10-04. The preceding RTK-only turn
verified tooling but did not advance product behavior. This turn inspected the
current dirty worktree, execution skill, authoritative Blueprint, parity matrix,
desktop search, host catalog and Android catalog/UI/device harness. No workers
or nested agents were admitted. All 36 authoritative rows remain unfinished.

## Implementation

The earlier name/ID-only remote search left the desktop conversation search
unavailable on Android. The same Read-authorized session_list route now also
matches user/assistant conversation text after redaction. It excludes system/tool
turns, tool arguments and arbitrary events. Matching before truncation preserves
full-text discovery within the bounded catalog; redacting before matching avoids
using hidden values as a search existence oracle. Existing same-workspace,
journal validation, ACL, byte/count/deadline bounds remain in force. Exhaustion
fails resource_limit rather than presenting a partial catalog as complete.

Search results add matching-turn counts and at most four 240-code-point snippets.
Controls other than newline/tab are stripped. Unfiltered metadata is unchanged.
The catalog digest is still computed over the complete unfiltered catalog before
search projection, preserving the existing resume transaction and stale-journal
refusal. No read changes journals, workspace revision, current session or cursor.

Android validates search fields together, integer counts against turn_count,
snippet count, UTF-8/code-point limits and controls. Legacy metadata-only search
responses remain readable. The session dialog displays counts and bounded
snippets with the existing confirmation and offline/authority fences.

The prepared connected SessionHostTest now uses the production search button to
find the seeded historical transcript, checks exact scope/digest/count/snippet,
unchanged projection and no pending write, then resumes that search result. Its
unique success marker is emitted only after restored conversation verification.
The harness requires that marker plus the updated narrow Compose snippet/UI
marker; missing/duplicate markers fail. These are prepared acceptance paths,
not connected-device evidence.

## Verification

- Focused remote session tests: 4 pass.
- env ZENPI_HOME=/private/tmp/stage2-body-search-home-20261004 rtk cargo test
  --offline --locked --test remote_workspace -- --test-threads=1: 70 pass.
  This uses an owned writable config home, not the sandbox-denied default home.
  Includes multi-page body results, redacted secrets not matching, excluded
  roles/foreign workspace, revoke refusal and unchanged journal/revision.
- Initial Android source check under /private/tmp/stage2-body-search-20261004:
  production/unit/instrumented sources compile; 425 JVM tests pass. A final
  source check additionally includes the prepared connected search workflow:
  zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-body-search-final-20261004; exit 0, 425 JVM tests pass.
  Instrumented sources compile only; existing deprecation warnings remain.
- Python Stage 2 discovery: 125 pass; session harness: 8 pass;
  lifecycle verifier: 21 pass; Blueprint verifier: 5 pass.
- Strict Blueprint, cargo formatting and git diff --check pass. No checkboxes
  promoted; 36 [ ], zero [_], zero [x].
- Real offline locked debug host build completed and verify-only confirmed
  current source/binary attribution.
- APK verify-only still refuses missing provenance. No fresh Gradle/ADB/physical
  camera/TCP/LAN run, stale APK install or connected success is claimed.

```text
sessions.rs 3382e3bfd8e38c5041c4088bcb6dfae11ffa6e0ec52bbe3654dc00c498e24169
SessionCatalogCodec.kt a2cb4577074c2a6469fc89a16a6381f59298d5266e5b9adb62d36e6952d34dee
MainActivity.kt 8d71175b75b973c53499bea83398427aede7f976d1c12bf1bc35c4585059b40f
remote_workspace.rs eb1dd8434597cc2e691faadb87d89f58a7f2015c1f7c7ba0051cfe6d5f9dab4f
SessionCatalogTest.kt 52f13600a33e49668e2154cf0b7652f080e4fd2e8ea4d8051b38055068b531f7
SessionHostTest.kt 838b267f19f476c381356325632a587ad1f80a9e00f81c057d5ce770739004fe
stage2_session_android_smoke.py 221114d619becf4a02c54815cdb9cfad78ff2a592387874a9bef2effdab69f54
main.jar 2e6313a276682d737c2c70ea93d4bc3eb66e433374b21066c526853f55f0ec62
unit.jar fc13dbf963f50557e081bb6f5990dd45e40792265925ffc657206f71a5559e2c
instrumented.jar 10643dd000e3d0acaca84dc9510452f3b378c71ebf22658489c9abbf96060edb
debug-host-receipt 223bcdfd6474da7c14e694cfc502d0ee26878a98366cd65daa171073148ed9af
debug-host-inputs f1421aebb4cfd3ea0857fb5b359d7e58e637a68627448962f247ad344cf67075
debug-host-binary 658d6c19cb8f1e5a022bfea6854c089afd7ae31c530226b6367c1ea8926c14f0
```

Goal remains active. Current-source APK provenance, physical QR/direct LAN
bidirectional synchronization and recovery, connected API lifecycle matrix,
complete desktop parity and signed release remain incomplete or unverified.
Source checks do not establish any of those gates or entire-item acceptance.

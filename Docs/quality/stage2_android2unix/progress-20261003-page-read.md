# Paged Task Output And History Reads

Provisional S2-036/S2-052/S2-054 progress at 2026-10-03T20:02:03+0800.
Previous goal turn added page-only cancellation but explicitly left completed
later-page reads unavailable. This turn connects those tasks to existing
bounded output/history/tool-argument read codecs. No nested agents, worker
launch, checklist promotion, commit/push or unrelated cleanup.

## Implementation

Completed list rows now offer output and history icons without needing worker
control capability. WorkerReadPolicy resolves the validated completed target
from the current summary or successful current list page. Current conflicting
summary evidence takes precedence. An opened reader captures a transient
WorkerReadScope with task identity plus owner/revision; it survives closing the
list so continuation and history-message/tool-argument readers remain usable.
It never survives project/session/owner/revision or conflicting subtab identity
change, authority loss, background/network boundary, reset or a pending write.

The existing HostClient/read codecs receive a temporary read projection with
that one task, keeping their request/result/subtab/digest validation intact.
This projection is not published as main state, stored or ACKed. The durable
display codec drops worker statuses as before; no task is re-executed and no
write capability is created. Main transcript readers preserve their existing
path. History navigation/dismissal retains the selected read scope only when
the same history/selected turn/tool identity remains valid; other dismissals
clear it. Late invalid read results close the reader rather than leaving it
loading indefinitely.

User/developer docs now describe the implemented later-page read flow while
keeping current-device and real-host acceptance open. Old progress receipts
remain historical and are not rewritten as new verification.

## Verification

- Final fresh source command:
  zsh tools/stage2_android_source_check.sh /private/tmp/stage2-page-read-final-20261003
  exit 0, production/unit/instrumented Kotlin compile, 332 JVM tests pass.
  Existing compiler warnings remain. Initial compilation also passed 332 but
  preceded final scope-precedence/late-result repairs; only final jars below
  represent current source.
- Four new JVM tests cover page-only read requests and output receipt scope,
  no main/cache/write-authority changes, closed-list continuation, owner/
  revision/session/identity and offline/reset/revoke boundaries, malformed/
  duplicate/conflicting page/summary evidence and returning to selected history.
- New Compose test covers second-page output/history callbacks for a device
  without control permission and loading/offline disabled states. Device source
  compiled only; no connected run or screenshot is claimed.
- Python Stage 2 tooling: 86 pass. Backup/lifecycle/Blueprint validator tests:
  5/15/5 pass. Strict Blueprint, cargo fmt --all -- --check and git diff --check
  pass. All 36 authoritative rows remain unfinished.
- Release host source attribution remains current, binary
  9f1ff46c2fd82ccad171534cbde89b3f7c6b49babaef8fd0f7686ad3019caaa8.
  APK verify-only refuses missing successful current build provenance. No
  repeated unchanged Gradle attempt or historical APK install.

All required command handles from this turn are terminal.

```text
WorkerReadPolicy.kt fb3f99b4d94ef8db0781db43580d39814463f82b712da04098be1aa47c3acda4
PairingViewModel.kt c468860f381da4f6607a6e721a736caec3f701bdc2175faf0abc0b4273f27076
WorkerHistoryNavigation.kt e8e2c325668e2392b446d5cff51f1701a970728ef20bca886c0f1812971e278e
AuthorityLossPolicy.kt b95577851a9d78801aedc7bc249a2c8adb686b975cbda0b29a3a14f0e727aae3
MainActivity.kt 50251d198f7508bd3c13c0146242e5d2aca082b93d24ed364580c655bc76ab4e
WorkerReadPolicyTest.kt b04dccaf6e5eaa4e927cac2e4109c4cf0fba0457d2bf5cb9e41a5d92a712e3ef
WorkerStatusPageScreenTest.kt a1619123c5e5fd927c46875f0f7263cbc6e7396600fa0d1acb6f2162ff3119b9
main.jar 5010211e637098ec154373104b3edb1c6ddd1325befeade4c1012da08079b383
unit.jar 599ba0655a48ce88cb3e1a05f06889f13c1efd6577f781a36faa100c9c272ba1
instrumented.jar 0ad2db8bff6a91bf295b2eb0ffcb8f420207ebfbf22140a859a87305a408ae5a
```

Goal active. Fresh APK, real-host later-page output/history/cancellation,
physical camera QR/direct LAN, connected lifecycle/full parity and signed
install/upgrade/reproducibility remain required open gates.

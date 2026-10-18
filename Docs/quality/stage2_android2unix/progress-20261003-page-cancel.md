# Paged Worktree Task Cancellation

Provisional S2-036/S2-052/S2-054 progress at 2026-10-03T19:57:22+0800.
Previous goal turn hardened shared negotiation. Current production inspection
found the task-list pager showed all task IDs but provided no actions; cancel
construction only searched the bounded 16-entry summary. This turn connects
page-only active task cancellation to the existing durable write/receipt path.
No checklist promotion, nested agents, worker launch, commit/push or cleanup.

## Implementation

WorkerTargetPolicy selects an eligible cancellation target. Current summary
evidence wins over a stale page, including completed/inactive states. Duplicate
or foreign summary identity fails closed rather than falling through to page
evidence. Page-only targets require Connected, an open successful page, matching
selected project/session and worker-control capability. Loading/error/offline
pages do not supply cancellation authority. Unknown outcomes require actual
runtime_active evidence from the host.

HostClient's cancel command overload binds an explicit validated task to the
current project/session/owner/revision. The ViewModel and pre-persistence
dispatch gate use the same target policy. The request keeps the original task
ID and follows existing persistence, deduplication, confirmation and exact
pending retry behavior. No page status is copied into the main projection or
offline cache, and no original task is resubmitted.

Task-list rows now have a stop icon for eligible active tasks. Pending write/
revoke, loading/error/offline and current-summary completion disable it.
Workspace owner/revision changes close the page and fence its reader; a late
page result with mismatched owner/revision is not published and closes its
view, rather than leaving an indefinite loading state.

Output/history for completed tasks only on later pages is still unavailable;
the existing read codecs require summary evidence. That gap is documented in
the Android notes and user guide, not presented as feature-complete paging.

## Verification

Four new JVM tests cover exact task-bound envelope/explicit retry, page state/
scope/capability fences, summary precedence and unknown runtime evidence, plus
foreign/completed/duplicate targets. New Compose device test exercises the
second-page stop callback and disabled states; it is compiled, not device-run.

Python Stage 2 tool discovery: exit 0, 86 pass. Backup/lifecycle/Blueprint
validator tests: 5/15/5 pass. Strict Blueprint validator, cargo fmt --all --
--check and git diff --check pass. All 36 rows remain unfinished. Release host
source attribution verifies unchanged binary
9f1ff46c2fd82ccad171534cbde89b3f7c6b49babaef8fd0f7686ad3019caaa8.
Current APK verify-only refuses missing successful build provenance; no stale
APK install or unchanged blocked Gradle retry.

Initial source compilation passed 328 tests but preceded the final late-read/
duplicate-summary repair. Final fresh source verification is recorded below
once its live handle completes; initial jars are not current-source evidence.

Final command:
`zsh tools/stage2_android_source_check.sh /private/tmp/stage2-page-cancel-final-20261003`
completed with exit 0, fresh production/unit/instrumented compilation and
328 JVM tests passing. Existing compiler warnings remain. All command handles
needed for this turn are terminal.

```text
main.jar 0583c46999960ea9f9334e7b222a9392a40bff8b1d93f5d048b83ef889daec18
unit.jar a386f7cc2c12b3e785141e74a28d03f5f144de66b53ee840048e3eac6557b934
instrumented.jar bb3f6c77497ba1931b09665f46256eb88b3bc7bdfbf0ccc35b0ada447d0aa3e1
```

```text
WorkerTargetPolicy.kt b0f0d87278f220237a357405a8b2ed504636d4bf30c61f0957477aa374918970
PairingViewModel.kt de1d8f8cd9515ccb33dda14f912a89f9721cb03294b8103aa4b4160a3820f3e5
WorkspaceDispatchPolicy.kt 2740008fb1b400f222b6d54c1b234e08ce5a0e48150eaf7a8315b6ec11ceb2ba
MainActivity.kt 8fad9daaf06b664f8027dc4c0f621deeb87ab3553eece559b55a4b5d5550232a
HostClient.kt e3865792df1d07a86739d2e8579189c1f1fc84014573a65865aac6de12914ef4
WorkerTargetPolicyTest.kt b5ed5b6bcb211a2d6b9b22fb6a588a4489c4ab7568117f17bb038dbc2df8943b
WorkerStatusPageScreenTest.kt fa9ad4d4c509ab3ff88c0a58f673e5dc101fa07fe74e61e65d3ef7c4b90fc6a8
```

Goal active. Fresh APK, page-only real-host cancellation, camera QR/direct LAN,
connected lifecycle/full parity and signed install/upgrade remain open gates.

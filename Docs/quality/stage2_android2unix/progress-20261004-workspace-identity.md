# Stage 2 Workspace Identity Before Capability Filtering

Recorded 2026-10-04. Partial S2-034/S2-035/S2-054 evidence; no checklist
promotion. Canonical cwd /Users/mac/Github/zenpi, dirty-worktree HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b. Previous goal turn made progress
(approval cancellation regression); this turn adds production Kotlin guards.
Existing user changes preserved; no nested agents, reset, push or bypass.
Blueprint SHA-256 d76e70e369f11f0fd3d1905d56d79c2a89401b92215da48ad85314f390d96d97;
spec SHA-256 403e6339b0318a420d93600fc52926acf6a9e46ec899bd9a9edb03c235190a28.

## Change And Scope

WorkspaceCommand.create previously filtered by ID and workspaceControl in the
same predicate. A duplicate ID without that capability could be ignored, so
construction succeeded despite an ambiguous target. Existing dispatch already
refused the duplicate; no demonstrated host mutation bypass is claimed. Now
construction requires exactly one addressed project before checking capability.
Unique non-selected projects remain valid switch targets with their own session.
The explicit current Kotlin/real Rust workspace lane repeats this refusal on
actual desktop snapshots, preserving primary CRUD, local CAS and exact retries.

Subtab select/rename/move/close and concurrency construction also require one
addressed tab. A non-null move destination must exist once at construction and
dispatch, rather than passing any-match. Moving to the end remains supported.
Ordinary JVM regressions cover mixed-session/capability duplicates, tab and
destination ambiguity/removal, valid cross-project selection, and exact pending
retry without relaxing scanner fences.

Primary Compose tabs now refuse ambiguous target IDs and scanner/reset states.
Open/close buttons use the existing mutation guard used by confirmation dialogs;
selected project display uses unique lookup. A new WorkspaceScreenTest covers
these states and restoration to a valid switch. It compiled but was NOT run on
a device. The explicit workspace instrumentation count increases 51 -> 52;
full desktop+workspace+Doze expects 88 named tests. Historical device receipts
are not promoted to current suite evidence.

## Verification

All commands run from canonical cwd. Source checker uses its isolated OUT and
ZENPI_HOME fixture directories, cached SDK/compiler dependencies, not Gradle or
APK packaging. No user credentials or terminal configuration changed.

- Intermediate zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-workspace-identity-20261004: exit 0; final UI button
  guard edits occurred during this run, so it is not final source attribution.
- Final zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-workspace-identity-final-20261004: exit 0, 531 ordinary
  JVM tests, main/unit/instrumented compilation, production QR corpus, all
  explicit interop lanes, six-test desktop target, real worker-tool lane and
  seven-phase prompt/approval/cancel lane. No source/test edits after final run.
- First Python suites detected the newly added screen test with old fixed
  counts: 137-test suite exit 1 (86 expected vs 87 actual without Doze), lifecycle
  suite exit 1. Preserved failed Python log, updated count to include the test.
- python3 -m unittest discover -s tools -p 'test_stage2_*.py': final exit 0,
  137 pass; /private/tmp/stage2-workspace-identity-python-final-20261004.log.
- python3 tools/test_validate_stage2_android_lifecycle.py: final exit 0, 25 pass.
  These are validator fixture tests, not connected API matrix execution.
- cargo fmt --check and git diff --check: exit 0.
- python3 tools/validate_stage2_android2unix_blueprint.py --strict: exit 0,
  36 [ ], zero [_], zero [x].
- python3 tools/stage2_host_build.py and --verify-only: exit 0 after Cargo lanes.
- python3 tools/stage2_android_build.py --verify-only: exit 1, missing APK
  provenance. Unchanged Gradle/ADB permission blockers not retried or bypassed.

Final log: /private/tmp/stage2-workspace-identity-final-20261004.log.
Current workspace exchanges: /private/tmp/stage2-workspace-identity-final-20261004/android-workspace-results.json.

```text
HostClient.kt a7be76dcc884c9b99be2ca7a225b1a911ca1a3c680399ec973268eb2e9caa922
WorkspaceDispatchPolicy.kt a5a3926452ef7dd81fe041aecdc7cd744e1f111b0d14485a94b560b03934ee39
MainActivity.kt 4257806b87e3af80bbbbf1bf4ec9ae79d36c1bced5bd151d23d139f4d2744921
WorkspaceCommandTest.kt 43cb22ebbf8e1f68c2babd24795b2b8dccc46b3c02befe8739f7a6cfbfb9a7ce
AndroidRustWorkspaceTest.kt e0056d6e8276c2ff7cfe7e6b53e4470a1f74ac9af66a610ad7d01f1ae4d2f6f3
WorkspaceScreenTest.kt 5d4f6e7c71bcb38350f5e8e2898d95267e944fcbb216b9820fbb9f45c7ccd562
stage2_desktop_android_smoke.py 26dda9b987b68303bcbc4772c46eed86105a940e387a1844216cc80d7501cf32
final-source-log 37d9c0414b567c0abe6d7ea2702b0de3b8bc8c8788ae151be7d5e814ffcbfba9
workspace-results 20fd1e5e5bd2b98a589b82ae94a6219d279dd430639e3fe55cfccfe36d7d8ab4
python-log 1a6b9184bb155ce955b319482ef493aaffe071d3ab9c466dfeda9edead10fd3e
debug-host-receipt 1332ff30ca37074e285ddbb95655f71ef8efd0d60d5a7824142fd08a1fe27bb9
```

Goal remains active/incomplete: current APK, physical camera QR -> direct-LAN
bidirectional sync/recovery, connected API31/34/35, complete desktop parity and
signed reproducible release are still required. Source and JVM checks neither
prove those gates nor justify claiming the full product delivered.

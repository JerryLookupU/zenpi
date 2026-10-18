# Stage 2 Explicit Worker Cancellation Without Submission Tabs

Recorded 2026-10-04. Partial S2-035/S2-036/S2-054 evidence. Previous goal turn
made progress (worker composer scope); this turn repairs cancellation coupling.
Canonical cwd /Users/mac/Github/zenpi, dirty HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b. Blueprint remains 36 [ ], zero [_]/[x].
Existing changes preserved; no nested agents, reset, push or permission bypass.
Blueprint SHA-256 d76e70e369f11f0fd3d1905d56d79c2a89401b92215da48ad85314f390d96d97;
spec SHA-256 403e6339b0318a420d93600fc52926acf6a9e46ec899bd9a9edb03c235190a28.

## Behavior And Boundaries

Worker cancellation previously constructed a dummy workerPrompt before changing
its payload to cancel. This required new-task active-tab admission even though
the wire cancellation targets only an explicit request ID. A task still known
to be running could have its cancellation control enabled while construction
failed on absent submission tabs. cancelWorker now builds its own request
envelope with current project/session, worker capability and owner/revision.
It does not borrow an active subtab or add subtab arguments.

Summary cancellation resolves task ID before state filtering. Target-object
construction refuses an existing summary unless its one current record equals
the supplied target. A page may supply an ID absent from the bounded summary.
WorkerTargetPolicy similarly resolves page ID before project/session/canCancel
checks, refusing a mixed terminal or foreign duplicate rather than ignoring it.
JVM coverage checks invalid submission tabs still allow an explicit known stop,
while wrong session, scanner ownership, duplicate IDs and capability/lifecycle
fences remain effective; exact pending retry remains distinct.

Expanded existing WorkerStatusPageScreenTest covers mixed duplicate page
records and restoration to one valid target. It compiled, NOT device-executed;
no method/count changed. Rust's actual worker cancellation isolation and owner
tests also ran, but do not by themselves prove this newly constructed Kotlin
cancel traversed a production listener or stopped a physical-LAN task.

## Verification

Commands from canonical cwd. Source checker uses isolated OUT and ZENPI_HOME
fixture directories and cached compiler/SDK; it is not Gradle/APK packaging.

- Intermediate zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-worker-cancel-independent-20261004: exit 0. Final screen
  assertion fixture adjusted during execution; final attribution is the repeat.
- Final zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-worker-cancel-independent-final-20261004: exit 0, 534 JVM
  tests, main/unit/instrumented compilation, production QR corpus, all explicit
  Kotlin/Rust lanes, six-test desktop target, real worker-tool loop and
  seven-phase prompt/approval/cancel lane. No source/test edits after final run.
- python3 -m unittest discover -s tools -p 'test_stage2_*.py': exit 0, 137 pass;
  /private/tmp/stage2-worker-cancel-independent-python-20261004.log.
- python3 tools/test_validate_stage2_android_lifecycle.py: exit 0, 25 pass;
  validator fixtures only, not connected API31/34/35.
- First rtk cargo test --locked --offline --test remote_worker_owner: exit 101,
  12 failures due default configuration directory write PermissionDenied.
  No production change or permission bypass. Repeat with
  ZENPI_HOME=/private/tmp/stage2-worker-cancel-owner-home-20261004 and uncompressed
  cargo test --locked --offline --test remote_worker_owner: exit 0, 12 pass;
  /private/tmp/stage2-worker-cancel-owner-20261004.log.
- ZENPI_HOME=/private/tmp/stage2-worker-cancel-host-home-20261004 cargo test
  --locked --offline --lib
  remote::worktree_host::tests::desktop_cancel_targets_one_subtab_across_devices_and_retains_noncooperative_ownership
  -- --exact: exit 0, one pass; /private/tmp/stage2-worker-cancel-host-20261004.log.
- cargo fmt --check, git diff --check and strict Blueprint: exit 0.
- python3 tools/stage2_host_build.py and --verify-only: exit 0 after Cargo lanes.
- python3 tools/stage2_android_build.py --verify-only: exit 1 missing APK
  provenance. Unchanged Gradle/ADB permission failures not retried/bypassed.

Final source log: /private/tmp/stage2-worker-cancel-independent-final-20261004.log.

```text
HostClient.kt 45009bebfb03870684971f0b0dddf59b793b0db9f6f4cab028641a3ecb572181
WorkerTargetPolicy.kt 5d36fef1cce821ad5db9824b5456ff282b87a80e4da89cfe105544de995c9bae
WorkerTargetPolicyTest.kt 6725ff1addfef2b31c731e67053bdde7bfb73585da752fb9207cc15f070456e6
WorkerStatusPageScreenTest.kt bcb178ba4cca46528182c5655660df71f8360b4b4f8f102ddc72c1fa2b21f5e8
final-source-log a2581f13dc1c9855802075c1fb96a3f1ca6adb38889775b9f0e58f1b502dd30a
python-log ce41d33bc456e2f8f66e9cc44f93c94b644d67ca699ecc740e4950fea13c4e9f
worker-owner-log 3ba9437d973e8c1b6500faac870a94b0a2c77e5bc043901950686d0843086502
worker-cancel-log 4e953037c8b646c417a343252a97f22e13c43d4a8eb4af96b7c0de6c52e1c700
main.jar d9a1b86e557eab0706d505e88efac24b2d4848b45e527521066a63dc6ee017fc
unit.jar ec25aefaed9e8f842164c83f15cb9b4e359dd58cce27832200f5718dd7445675
instrumented.jar ae2533253ff8960f5747ed80792387729660119c4ad7ba86682f03c29ef3b821
debug-host-receipt a6ace1b71f955d31a086d6441d66fdc85f8486bdf71254286532d75fc42f037a
```

Goal active/incomplete. Current APK, physical camera QR/direct-LAN bidirectional
sync/recovery, connected API31/34/35, complete desktop parity and signed
reproducible release remain required. These scoped cancellation checks do not
replace any product acceptance gate.

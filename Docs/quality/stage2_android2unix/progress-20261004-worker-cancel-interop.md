# Stage 2 Kotlin Cancel Frame Drives One Real Worker Runner

Recorded 2026-10-04. Partial S2-019/S2-035/S2-036/S2-054 evidence. Previous
goal turn changed cancellation construction; this turn adds actual current
Kotlin -> Rust runner evidence rather than claiming unit envelopes prove stop.
Canonical cwd /Users/mac/Github/zenpi, dirty HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b. All 36 Blueprint rows remain [ ].
Existing changes preserved; no nested agents, reset, push or permission bypass.
Blueprint SHA-256 d76e70e369f11f0fd3d1905d56d79c2a89401b92215da48ad85314f390d96d97;
spec SHA-256 403e6339b0318a420d93600fc52926acf6a9e46ec899bd9a9edb03c235190a28.

## New Evidence

New AndroidRustWorkerCancelTest is excluded from ordinary JVM suite and run
explicitly by a new ignored WorktreeHost Rust test in the source checker.
Two actual WorktreeRunner lanes on main/sibling tabs execute controlled closures
that observe runtime cancellation. Production live task statuses and owned
snapshot feed Kotlin with no submission tab projection. The fixture explicitly
advertises worker_control because low-level owned listing does not advertise
controller capabilities; this capability fixture is not listener proof.

Kotlin constructs its request-targeted cancel, rejects duplicate target/scanner
preflight, and writes the production framed envelope. Rust decodes exactly one
frame, refuses foreign-device and stale-revision attempts, then admits the
original request. Only the target closure observes cancellation once; its
actual completion becomes cancelled while sibling remains running. Two starts
and one cancellation are checked. Exact immediate retry returns equal receipt.

After releasing/joining the sibling, drop the old pool/Agent and construct a
fresh Agent from the disk journal plus restored checkpoint. Original cancel
retry returns the third identical receipt with a callback that panics if any
new cancellation is attempted. Journal bytes stay unchanged and no worker
owners are prepared. Kotlin validates correlated fragmented refusal/accepted
frames, equal receipts, and the target-cancelled/sibling-running observation.

This exercises real runner/cancellation/admission/durable-receipt code with
controlled closures. It is not external provider/tool execution, OS process
crash, production TCP/TLS listener, Android Keystore, camera QR or physical LAN.
No production logic or authority policy changed in this turn.

## Verification

Commands run from canonical cwd; source checker controls isolated OUT and
ZENPI_HOME=$OUT/worker-cancel-home for the new lane. Existing user config intact.

- Initial zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-worker-cancel-interop-20261004: exit 101. Prior lanes and
  Kotlin generation passed; new test incorrectly treated revision 0 as stale,
  but fixture's current revision was 0, so cancellation was correctly accepted.
  Failed log retained. Changed stale test value to current revision + 1.
- Final zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-worker-cancel-interop-final-20261004: exit 0. 534 ordinary
  JVM tests, main/unit/instrumented compilation, production QR corpus, prior
  explicit interop lanes, six-test desktop target, real worker-tool and
  seven-phase prompt lane. New worker-cancel lane: one Rust test/two Kotlin
  phases passed. No source/test edits after final run.
- python3 -m unittest discover -s tools -p 'test_stage2_*.py': exit 0, 137 pass;
  /private/tmp/stage2-worker-cancel-interop-python-20261004.log.
- python3 tools/test_validate_stage2_android_lifecycle.py: exit 0, 25 pass;
  validator fixtures only, not device matrix execution.
- cargo fmt --check, git diff --check and strict Blueprint: exit 0, 36 unfinished.
- python3 tools/stage2_host_build.py and --verify-only: exit 0 after Cargo lanes;
  inventory changed because cfg(test) fixture lives under src, not because
  production cancellation policy changed.
- python3 tools/stage2_android_build.py --verify-only: exit 1 missing current
  APK provenance. Unchanged Gradle/ADB permission blockers not retried/bypassed.

Final log: /private/tmp/stage2-worker-cancel-interop-final-20261004.log.
Wire/runner evidence: /private/tmp/stage2-worker-cancel-interop-final-20261004/android-worker-cancel-results.json.

```text
worktree_host.rs d8e112ffa0ad79d1e0c65b8ee401f1768eac258e258432e6d1a28f2b0c409d47
AndroidRustWorkerCancelTest.kt e0f5be4a8609ca07667cfab62c9bea1c59922fac51557f6fc757b8be31d33245
stage2_android_source_check.sh 6defdb3dac7cdddff21909ad655cbed9284019c628e1cb828b06578caa166d40
final-log 5de4584464eb989f6fc243665d62eae70ff7432d7e7afd65a466b3b2395e021e
worker-cancel-results 035a63c4e8d62c6e785d941bca433c5b0b6def563a408e969aee96f39d311dde
python-log 92d823a651cee5ec5806418df708793dad9a518098f0b7428c8d5308acafe4da
main.jar d9a1b86e557eab0706d505e88efac24b2d4848b45e527521066a63dc6ee017fc
unit.jar a18561c315342c8a8a90883a990cac50e67458a4c9bf6303a4789e8c7ccfe916
instrumented.jar ae2533253ff8960f5747ed80792387729660119c4ad7ba86682f03c29ef3b821
debug-host-receipt b18cb48fa2e7ba1ccec58fd2b19841436306a89820ca47eaca38c83dcc001a66
debug-host-inputs 3fe54d0466defb6d1ed16591ff7d19548bc97c1ef1320cd02ad06400d1a3129e
debug-host-binary 93c9dbed178e6c1b105be48dca206fccec1aff04ff34544495499a1dcd15fdc0
```

Goal remains active/incomplete: current APK, physical camera QR/direct-LAN
bidirectional sync/recovery, connected API31/34/35 lifecycle, complete desktop
parity and signed reproducible release still require authoritative evidence.
Controlled worker cancellation is one partial gate, not product completion.

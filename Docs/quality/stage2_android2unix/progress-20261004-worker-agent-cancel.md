# Stage 2 Kotlin Worker Cancellation Through Actual Agent Execution

Recorded 2026-10-04. Partial S2-019/S2-035/S2-036/S2-054 evidence. Previous
goal turn repaired catalog construction; this turn strengthens worker stop
evidence from custom runner closures to actual Agent/backend/session execution.
Canonical cwd /Users/mac/Github/zenpi, dirty HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b. All 36 Blueprint rows remain [ ].
Existing changes preserved; no nested agents, reset, push or bypass.
Blueprint SHA-256 d76e70e369f11f0fd3d1905d56d79c2a89401b92215da48ad85314f390d96d97;
spec SHA-256 403e6339b0318a420d93600fc52926acf6a9e46ec899bd9a9edb03c235190a28.

## Stronger Evidence

The explicit AndroidRustWorkerCancelTest/WorktreeHost lane now prepares two
independent real Agents with controlled backends and uses production
spawn_observed_prompts, immutable execution selection, input processing,
provider-event forwarding and task history persistence. No custom runner
closure stands in for Agent execution. Backend counts two actual invocations
and observes exactly one cancellation. Kotlin's current cancel frame stops
its explicit target, while sibling live preview remains available/running.

After release, sibling finishes successfully with exactly one assistant answer.
Cancelled target's real worker journal has exactly one user turn and no
assistant turn. Master session has no task turns. Fresh disk Agent/pool receipt
recovery, no new owners/cancellation, foreign-device and stale-revision refusal
and equal exact receipts from prior lane remain covered. Kotlin checks the
finished cancelled/succeeded pair and Agent journal evidence in addition to
correlated responses. No production cancellation logic changed.

The backends are controlled local test doubles, not external providers; no
actual tool effect, Android Keystore, production listener/client, OS process
crash, camera QR or physical LAN claim is made. Fixture-owned low-level listing
still advertises worker_control explicitly as documented in previous report.

## Verification

Commands run from canonical cwd. Source checker controls isolated OUT and
ZENPI_HOME=$OUT/worker-cancel-home; no user config/provider credentials used.

- Initial /private/tmp/stage2-worker-agent-cancel-20261004 checker exit 101:
  test compile failed using SubtabExecution.root; corrected to tab.root.
- /private/tmp/stage2-worker-agent-cancel-final-20261004 checker exit 101:
  actual cancellation passed, but preview assertion lacked newline. Existing
  LivePromptText intentionally publishes complete lines, so incomplete delta
  remained withheld. Added newline to fixture delta/expectation; did not relax
  production redaction or publication rules. Failed logs retained.
- Focused repeat with current cached compiled Kotlin classes and explicit
  ZENPI_ANDROID_INTEROP_JAVA/CLASSPATH plus isolated
  ZENPI_HOME=/private/tmp/stage2-worker-agent-focused-home-20261004:
  cargo test --locked --offline --lib
  remote::worktree_host::tests::android_cancel_frame_stops_one_running_worker_and_restores_receipt
  -- --ignored --exact: exit 0; /private/tmp/stage2-worker-agent-focused-20261004.log.
  Final complete source attribution is the run below, not this cached shortcut.
- zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-worker-agent-cancel-verified-20261004: exit 0; 536 ordinary
  JVM tests, main/unit/instrumented compilation, production QR corpus, explicit
  interop lanes, six-test desktop target, real worker-tool/prompt lanes and
  upgraded cancellation lane (one Rust test/two Kotlin phases). No source/test
  edits after verified run. Instrumentation compilation is not execution.
- python3 -m unittest discover -s tools -p 'test_stage2_*.py': exit 0, 137 pass;
  /private/tmp/stage2-worker-agent-cancel-python-20261004.log.
- python3 tools/test_validate_stage2_android_lifecycle.py: exit 0, 25 pass;
  validator fixtures, not connected API matrix.
- cargo fmt --check, git diff --check and strict Blueprint: exit 0.
- python3 tools/stage2_host_build.py and --verify-only: exit 0 after Cargo lanes.
- python3 tools/stage2_android_build.py --verify-only: exit 1 missing APK
  provenance. Unchanged Gradle/ADB permission blockers not retried/bypassed.

Final log: /private/tmp/stage2-worker-agent-cancel-verified-20261004.log.
Agent/cancel evidence: /private/tmp/stage2-worker-agent-cancel-verified-20261004/android-worker-cancel-results.json.

```text
worktree_host.rs 726aaae296d48458639094f2511bd82d0135400ff3a1569a03542cdc0288e1fc
AndroidRustWorkerCancelTest.kt 9b30acef86c18244de640a55220edab4acc7bd702341fd81ec582d2cd9d895aa
verified-log 3368f2c2c24689b78867e283f47877435e3d7519335fca97ccca8a1fbaedbbba
worker-cancel-results e70d51b7649a4a930d0b1d1143c9db2f069c3f6bb4b221d208346177618635e2
python-log c7617d5500de3f62141bb0f1cf927f89f14cc80964835366e84c6e81e8749615
focused-log ea97b491c11e7fbb16f0182acc14d2524e63924f74865086cdf808c079daee8d
unit.jar bbe237b33fa3d60b678394ba2c9ce9a53aa6651f814754a73fd424cd968ef24d
debug-host-receipt a88fa45871e0decd711703e82147513694ed664a7da33fb6dd42bfaca21d5a2f
debug-host-inputs 86003558fd969be9ad93f7a3ceac3c0b81263209fa064bacf478da6cdaadb5eb
debug-host-binary 712e04a9f1096463cd42ab09314733f081fc9d87d17f359bf96cffc4b3332858
```

Goal active/incomplete. Current APK, physical camera QR/direct-LAN bidirectional
sync/recovery, connected API31/34/35 lifecycle, full desktop parity and signed
reproducible release remain required; controlled Agent cancellation does not
replace those product acceptance gates.

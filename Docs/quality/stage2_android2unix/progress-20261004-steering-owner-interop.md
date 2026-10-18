# Stage 2 Kotlin Steering Applied By Real Owner

Recorded 2026-10-04 UTC. Related S2-014/S2-019 owner routing, S2-036 client
controls and S2-054 parity. Partial evidence only, all 36 Blueprint rows [ ].
Cwd /Users/mac/Github/zenpi, dirty-worktree HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b. Existing changes preserved; no nested
agents, reset, push, permission bypass or stale APK installation.
Blueprint SHA-256 d76e70e369f11f0fd3d1905d56d79c2a89401b92215da48ad85314f390d96d97;
specification SHA-256 403e6339b0318a420d93600fc52926acf6a9e46ec899bd9a9edb03c235190a28.

## Implemented Evidence

Extended the existing explicit AndroidRustPromptTest/headless lane with Kotlin
steering generated from actual running host listing. Read-only and different
Control devices refuse it; stale revision, slash-command steering and changed
payload under the same request ID refuse. Identical request repeats retain one
durable input and the same accepted receipt. Uncertain responses remain null
and are not treated as accepted receipts by Kotlin.

First run proved one durable queue entry and restart receipts. Strengthened
the controlled backend next: after steering receipt, the fixture releases the
first completion, permitting the production Agent input boundary to consume
the input. The second backend request contains ANDROID_OWNER_STEERING exactly
once; the first contains none. Recovered queue has exactly one Steer record,
status Applied and an applied turn ID. The same task remains running until
Kotlin-generated cancellation. Two backend calls total, no extra job. Prompt,
steering and cancel receipts match across admission/retry/restart; restart
lookups do not rewrite journal. Kotlin validates all fragmented wire responses
and final queue/application evidence.

This is an in-process HeadlessHost and deterministic cooperative backend,
not physical TLS/LAN, Android Keystore, external provider or tool-effect proof.
The prior report's one-call prompt/cancel fixture is historical evidence; the
current extended lane expects two calls because steering reaches a new provider
boundary within the original task. No production behavior was weakened.

## Verification

- zsh tools/stage2_android_source_check.sh /private/tmp/stage2-steering-owner-20261004:
  exit 0, initial durable-queue extension passed.
- zsh tools/stage2_android_source_check.sh /private/tmp/stage2-steering-owner-applied-20261004:
  exit 0, 529 ordinary JVM tests, production QR corpus, all source compilations,
  all explicit Kotlin/Rust lanes including applied steering, six-test desktop
  target and production tool-loop passed. Four Kotlin phases in new lane,
  one explicit Rust test passed. No code/test edits after applied run.
- python3 -m unittest discover -s tools -p 'test_stage2_*.py': exit 0, 137 pass.
- python3 tools/test_validate_stage2_android_lifecycle.py: exit 0, 25 pass.
- cargo fmt --check, git diff --check and strict Blueprint: exit 0.
- python3 tools/stage2_host_build.py and --verify-only: exit 0 after Cargo lanes.
- python3 tools/stage2_android_build.py --verify-only: exit 1, missing APK
  provenance; unchanged Gradle/ADB permission failures not retried.

Commands ran from cwd above. Source checker controls ZENPI_ANDROID_* fixture
paths and isolated ZENPI_HOME=$OUT/prompt-home. No user config/credentials
modified. Evidence JSON:
/private/tmp/stage2-steering-owner-applied-20261004/android-prompt-results.json.

```text
remote_headless_commands.rs b3b54e8bff0b2fb039bad17a9bff6fcfdcd208964840d8c0f75f8abe72175dda
AndroidRustPromptTest.kt 42facd9f8626b7f7a8926f23df766cac4dead0f83573757454eec52cfea02776
applied-log efe82dea58dfa6013f2b50848d113b69a0eed7599f1edf0627f2dff54b7ca0d7
prompt-results c4c3df24e9750eec8882b41b25271da394119cbcb702b63b1f13647daf68b320
python-log c8d85a0a00a23267dad5ced9fd5ff54c9a822c598817a01f61de870ad234d090
unit.jar 2b7523da834221fc28f4fb8abbd7486f0601972b949a3edfec04e234d993e5c1
debug-host-receipt 286ac6aeb539bcb48296e877e66ca80617286970b0de847498a7878d2b4945e6
```

Goal active and incomplete. Current APK, physical camera QR/direct-LAN
bidirectional sync/recovery, connected API31/34/35 lifecycle, complete desktop
parity and signed reproducible release remain required.

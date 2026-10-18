# Stage 2 Session Scope Progress

Recorded 2026-10-04 UTC. Related requirements: S2-014/S2-015 host owner
boundaries, S2-030 Android client, S2-032 transport and S2-033 projection state. This report is
partial behavioral evidence, not whole-item acceptance. Authoritative IDs and
gates remain in Docs/stage2_android2unix_blueprint.md; all 36 rows remain [ ].

## Authority

Cwd /Users/mac/Github/zenpi; integrated dirty-worktree HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b. Existing changes preserved; no nested
agents, reset, push or checkbox promotion.

Blueprint SHA-256 d76e70e369f11f0fd3d1905d56d79c2a89401b92215da48ad85314f390d96d97.
Specification SHA-256 403e6339b0318a420d93600fc52926acf6a9e46ec899bd9a9edb03c235190a28.

## Implementation

Session catalog/history and transcript scope checks previously matched project
ID and session together. A second identical project ID naming a foreign
session could therefore be ignored. SessionCatalogCodec.readable now requires
the selected project ID exactly once and a matching session; catalog/history
request, decode, sameScope, transcript retention and session new/rename/resume
use this admission. SessionChangePolicy also refuses scanner-time recovery and
unlisted/ambiguous projects. UI new/rename entry and confirmation controls
disable during scanning/reset or ambiguous scope rather than offering a
command that the codec will reject.

SessionReadScopeTest adds two JVM cases covering initial/continuation reads,
message/worker retention in both directions and all session commands. Positive
command fixtures include valid owner/revision/tab metadata, so refusal cannot
be explained by unrelated missing authority. Actual Rust/Kotlin session and
historical journal lanes assert mixed-duplicate request/decode refusal on real
host projections. Compose catalog/history/new/rename assertions cover disabled
controls; these are compiled only, not device-executed.

## Verification

All commands ran from the cwd above. No environment credentials were added.
Source checker controlled ZENPI_ANDROID_* fixture paths and isolated ZENPI_HOME
fixture directories as defined in tools/stage2_android_source_check.sh.

- rtk cargo test --locked --offline --test remote_parity --test remote_migration:
  exit 0, 11 passed in two suites. Log /private/tmp/stage2-parity-migration-20261004.log.
- zsh tools/stage2_android_source_check.sh /private/tmp/stage2-session-scope-20261004:
  exit 0, 521 JVM tests and all explicit Kotlin/Rust lanes.
- zsh tools/stage2_android_source_check.sh /private/tmp/stage2-session-scope-final-20261004:
  exit 0 after positive command fixture and actual history assertions added.
- zsh tools/stage2_android_source_check.sh /private/tmp/stage2-session-scope-ui-20261004:
  exit 0 after UI guards; 521 JVM tests, production QR corpus, all three source
  compilations, explicit Kotlin/Rust lanes, six-test desktop target and actual
  production tool-loop unit lane pass. No test-source edits after this run.
- python3 -m unittest discover -s tools -p 'test_stage2_*.py': exit 0, 137 pass.
- python3 tools/test_validate_stage2_android_lifecycle.py: exit 0, 25 pass.
- cargo fmt --check, git diff --check and strict Blueprint verifier: exit 0.
- python3 tools/stage2_host_build.py: exit 0 after final Cargo lanes; current
  debug receipt below. Earlier verify-only passed with identical input/binary
  digests. Source attribution is not reproducible release acceptance.
- python3 tools/stage2_android_build.py --verify-only: exit 1, missing APK build
  provenance. No unchanged Gradle/ADB permission retry or stale APK installation.

```text
SessionCatalogCodec.kt ce8492397fb1f62d1fe02c2fceeb382d7463e8db00c3e81804fddbcbb29a7b30
SessionHistoryCodec.kt bc11d4f530416ca8541726dd3597a4c87e3b0b7ca409095dba83f16a426d1a93
SessionCommandCodec.kt ffaf65f1a18d3a18d3d531851b28428d66d3cc7567560dd101fe8184980f7e25
TranscriptPageCodec.kt f183eb141aea4440b8400abffc639e6a5947c31b147a11a2aee0214f6d083102
SessionChangePolicy.kt bd216ca205584b7acd14d8d47286e46c4f8081a0bf93f38c203ff68f2e68208d
MainActivity.kt 4a35734a2abac44af75c1b534960d49302dff3a73188cca74cf58726be24305b
final-ui-log 51f13a9ed9a95fed7e11ded4c76cec4a05857704e73021383176d0976be7193e
python-log 9db9c1ea6a020b3ac61dc51616bacf207ae163df0c6bbbbfb87d585fc0a90dbc
main.jar 918c253eb4b62377984eb69b231947e8124bf25c3ea0e4fc87d765408807ad23
unit.jar 230e86267a4476a6381c0241e2aa9d1999de375e5839ce4081e1c81e227ccfb9
instrumented.jar 5765d3b506765f2db1b7471b2075224b7aa4bd8f4909490f2e92c5b179541872
debug-host-receipt 7e98c289b3ab7c70e7e2db6b239104aadc14b6a3890e99d456f8a53f4d248537
```

Goal remains active and incomplete. Current APK, physical QR/direct-LAN
bidirectional communication and recovery, connected API31/34/35 lifecycle,
complete desktop parity and signed reproducible release remain required.
Source/JVM evidence does not replace those gates.

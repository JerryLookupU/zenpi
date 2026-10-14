# Stage 2 Terminal Identity Progress

Recorded 2026-10-04 UTC, related to S2-032 transport, S2-036 PTY projections,
S2-054 parity and S2-052 fail-closed admission. Partial evidence only; no
whole-item promotion. Blueprint remains the sole checklist, 36 rows [ ].

## Authority And Changes

Cwd /Users/mac/Github/zenpi; integrated dirty-worktree HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b. Existing work preserved; no nested
agents, reset, push, permission bypass or stale APK installation.

Blueprint SHA-256 d76e70e369f11f0fd3d1905d56d79c2a89401b92215da48ad85314f390d96d97;
specification SHA-256 403e6339b0318a420d93600fc52926acf6a9e46ec899bd9a9edb03c235190a28.

TerminalPageCodec now rejects a selected project ID appearing more than once,
even when only one duplicate has the selected session. TerminalCommandCodec
creation requires that same unique scope and exactly one actual active tab.
Input/resize already use the shared terminal resolver and inherit the refusal.
WorkspaceDispatchPolicy revalidates addressed project ID uniqueness before
matching session, requires unique addressed tabs and resolves terminal control
through the full scoped terminal resolver. This also protects other fresh
commands in the shared dispatch boundary. Exact pending-envelope resolution
remains intentionally before projection validation; scanner/reset/foreground/
network guards still fence retries, and a different request is never admitted.

Terminal UI no longer displays/control-enables ambiguous project/tab scope.
Input, resize, keys and terminal creation disable during scanning. Existing
history policy disables initial and continuation reads in those lifecycle
states. No host-side permission relaxation or new Android shell execution.

Two new JVM tests cover mixed project duplicates and missing/duplicate active
tabs, read/write/open/retention, fresh shared commands and saved retry identity.
Actual Rust/Kotlin PTY lane checks initial request, response decode, continuation
and control refusal against a real host projection. Existing Compose history
and input/open cases extend scan/duplicate disabled controls. Instrumentation
is compiled only, not executed on a device.

## Verification

Exact commands below ran from the cwd above. Source checker uses its declared
ZENPI_ANDROID_* fixture paths and isolated ZENPI_HOME fixture directories;
no credentials or user configuration were modified.

- zsh tools/stage2_android_source_check.sh /private/tmp/stage2-terminal-identity-20261004:
  exit 0; 523 ordinary JVM tests, production QR corpus, main/unit/instrumented
  compilation and every explicit Kotlin/Rust lane passed. Includes actual host
  PTY input once-only effects/resize/redacted history, six-test desktop target
  and production tool-loop lane. No source/test edits after this run.
- python3 -m unittest discover -s tools -p 'test_stage2_*.py': exit 0, 137 pass.
- python3 tools/test_validate_stage2_android_lifecycle.py: exit 0, 25 pass.
- cargo fmt --check, git diff --check and strict Blueprint: exit 0.
- rtk cargo test --locked --offline --test remote_terminal --test remote_workspace:
  exit 101 under ordinary HOME; terminal 9 pass, workspace 47 pass/25 fail.
  Failures included configuration I/O Operation not permitted and downstream
  owner/worker admission refusals. Original log retained, not counted as pass.
- ZENPI_HOME=/private/tmp/stage2-terminal-identity-host-home-20261004 rtk cargo test
  --locked --offline --test remote_terminal --test remote_workspace: exit 0,
  81 pass in two suites. ConfigPaths honors this documented fixture override;
  temporary configuration is isolated and no global config is rewritten.
- python3 tools/stage2_host_build.py and --verify-only: exit 0 after Cargo
  lanes; current debug host attribution is not reproducible release evidence.
- python3 tools/stage2_android_build.py --verify-only: exit 1, missing APK build
  provenance. Existing global Gradle lock/daemon and ADB socket permission
  failures were not retried unchanged. No physical QR/LAN success claimed.

```text
TerminalPageCodec.kt 629c92a5812d576684e3e4eeb9bd0e00c805ea6b80c3f24bda3b140420c874c2
TerminalCommandCodec.kt 8c00affb79db61fd00efbf16057094f4eac394c72448640a3b4467d7ba7ad5c2
WorkspaceDispatchPolicy.kt ac340e1a00aeef208695d42e43fde3f779a7c3f080cd4eccb071670b6023896d
MainActivity.kt 1d2a83589bdea90bb9616fa955c47682d2266a1acecd5eedc3ca629ff465efba
source-log 7fec70ea65d80ad7c7fca78c489490fb03fa88fae5fc6465d9071fc325a3d36c
python-log bacb340cd36420f68b52a3357d770f8cbe6436ab010982c3ee4a04e9599ddeaf
main.jar c892f4d3fa62ab1d5dc20a7265daac923d039b63c87f5b6873564b03cd22acec
unit.jar 916a4cd797700c7bca2a8a63c2d26debd4cf6b5da556b5351061f15503d13655
instrumented.jar 82e6c7dee826c7668b8112e016ecee37d8bad445efceaad3d8572aaf8cdff165
host-failed-log b3abdf44a8571e829a516124ff9a772f7f59e806743ef51d22fc53c2b2a7d426
host-isolated-log 780fa80eedec46ef8ada448fdb590b0d1c500499ea309c758bdc086352e0e968
debug-host-receipt 71db017b6752bec3117eb27fc21f0fd570485fe6f8138ed2501981c4164d7998
```

Goal active and incomplete. Current APK, physical camera QR/direct-LAN
bidirectional sync and recovery, connected API31/34/35 lifecycle, complete
desktop parity and signed reproducible release remain required. None is
replaced by source-only JVM/PTY fixtures.

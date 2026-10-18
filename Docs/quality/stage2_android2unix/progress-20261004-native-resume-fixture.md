# Native Extension Resume Fixture

Recorded 2026-10-04. Partial S2-054/S2-099 validation repair only, no Master
acceptance. Previous goal turn was progress (worker lease Drop repair).
Current execution skill, dirty worktree, Blueprint/spec and the exact failing
fixture were inspected. No nested agents/reset, deadline relaxation, added
retry or repeated denied Gradle/ADB attempt.

## Repair

The extension session-resume test used a new Python script and retried timed-
out setup three times. The last broad run still failed at interpreter startup.
The fixture now follows tests/stage1_extension_hooks.rs's existing pattern:
an executable private shell script execs the already-built native test binary,
selecting one helper with --ignored --exact. FD3 carries JSON extension output;
libtest stdout goes to /dev/null. Production executable validation, process
groups, pipes, capability/lease checks, 1000 ms hook timeout and the original
failed checkpoint/rollback/success assertions all remain in use.

Removed setup retries entirely. The helper is ignored in ordinary discovery
because it requires the parent-owned protocol FD/environment, but the active
parent actually invokes it repeatedly during initialize/lifecycle/tool calls.
This is not a new skip of the failing parent. No non-test production function
or Android source was changed.

## Verification

All commands ran from /Users/mac/Github/zenpi, host tests with new mktemp
ZENPI_HOME directories. All observed command handles terminal.

- Parent extension-resume scope passes one test; one subprocess helper ignored
  at top level and invoked by the parent, 459 filtered.
- Two independent default-parallel library runs: each 432 pass, zero failures,
  eight ignored, 21 filtered, 17.33 and 17.09 seconds respectively. The exact
  21 exclusions are only the auth/backend listener tests from the historical
  failed log, not entire modules. All four pipe deadline parents, the resume
  parent and worker lease regressions pass in both runs. No full-library pass.
- Both parallel commands used set -o pipefail, derived --skip arguments from
  historical failures beginning auth::/backend::, then cargo test --locked
  --offline --lib -- <exact names>, with tee preserving complete logs.
- Integration command with remote_migration, remote_parity, resume_compact_owner
  and stage1_extension_hooks exits 101: first three targets pass 6/5/6;
  extension suite has 8 pass, 14 fail, one helper ignored. All 14 failures are
  explicit PermissionDenied/Operation not permitted at local listener setup,
  not protocol/lease failures. This is an unresolved integration gate, not green.
- Stage 2 Python tools 163 pass; lifecycle fixtures 25; Blueprint fixtures 5.
- cargo fmt --check, git diff --check, strict Blueprint pass; 36 [ ], no [_]/[x].
- Guarded debug build refreshed; current attribution verifies. Release and
  historical reproducibility source inventories remain stale after recent
  worker change and this source-contained test fixture edit. No old receipt
  or release bytes rewritten. APK verify-only still lacks provenance.
- No Kotlin change/new JVM/device claim this turn; previous interop remains
  scoped to its recorded source run. Physical phone query remains unanswered.

```text
core.rs 2365e727aaca23d7cdc6ce73c767eea4ca976548b8b0386e4002e7bbcbc9adb9
parallel-log 8d881cf2f2cfe3ac3f3b01941ef839d67fbcbc1ca9b65080e8e446e9bbbf48ea
repeat-log 9f5624ea37ed7cca5f05ee2f40cdc583d9bc7f816dfc8a629f8f2fe110b7c0eb
debug-binary c93f6d6385f31c092b3abaf34a6e5f959d041ad9a1e671aacdf69add23da63bd
debug-receipt 818d0af67ed118dc20c7f5943a0607d823df68c49cd515f4dd4a7cfe80aa078b
host-inputs dfb981e3cd20b6fea88156f5739deb2197d2de132483c6d2637e51f1eb16a621
```

Logs: /private/tmp/stage2-native-resume-wide-20261004.log and
/private/tmp/stage2-native-resume-repeat-20261004.log. Prior failures retained.
Goal active/incomplete: full unsuppressed Rust/network regression, current
release/repeat, APK/signing/install/upgrade, physical camera QR/direct-LAN
pairing/synchronization/recovery and connected API31/34/35 still required.

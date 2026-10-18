# Stage 2 Kotlin Prompt Produces Host Tool Effect

Recorded 2026-10-04 UTC. Related S2-014/S2-019 owner routing, S2-036 client
controls and S2-054 parity. Partial evidence only; all 36 Blueprint rows [ ].
Cwd /Users/mac/Github/zenpi, dirty-worktree HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b. Existing changes preserved; no nested
agents, reset, push, permission bypass or stale APK installation.
Blueprint SHA-256 d76e70e369f11f0fd3d1905d56d79c2a89401b92215da48ad85314f390d96d97;
specification SHA-256 403e6339b0318a420d93600fc52926acf6a9e46ec899bd9a9edb03c235190a28.

## Evidence Added

Extended current explicit Kotlin/headless owner lane: controlled backend first
completion requests production write_file, not a direct fixture file write.
The Agent invokes the existing built-in under the fixture's all-builtins and
Never-approval policy. A preexisting fixture file changes to exact expected
bytes. Journal contains exactly one matching Tool turn with call ID, tool name
and serialized success output. Kotlin validates its path, bytes and diff.
The next controlled provider context still receives applied steering exactly
once; cancellation and prior permission/conflict/refusal evidence retained.

Fresh Agent disk recovery and old command receipt lookup do not invoke backend
or change tool file; post-revoke exact retries also leave file and journal
unchanged. Exactly two controlled backend calls, one tool result, no extra job.
This proves a real filesystem tool effect in the same owner, not Android local
execution. It does not prove network TLS/LAN, Keystore, external provider or
guarded interactive approval: the fixture policy explicitly uses Never.

## Verification

Commands ran from cwd above. Source checker controls ZENPI_ANDROID_* fixture
paths and isolated ZENPI_HOME=$OUT/prompt-home. No user config/credentials changed.

- zsh tools/stage2_android_source_check.sh /private/tmp/stage2-prompt-tool-effect-20261004:
  exit 0. During compilation of earlier lanes, the newly written Rust fixture
  assertion was corrected to actual TurnRole/metadata and serialized ToolResult
  output schema before its target compiled. Not used as final attribution.
- zsh tools/stage2_android_source_check.sh /private/tmp/stage2-prompt-tool-final-20261004:
  exit 0 after final post-revoke file invariant; 529 ordinary JVM tests, all
  source compilations, production QR corpus and all explicit interop lanes,
  six-test desktop target and production tool-loop pass. Extended owner lane
  one Rust test/four Kotlin phases passed. No code/test edits after final run.
- python3 -m unittest discover -s tools -p 'test_stage2_*.py': exit 0, 137 pass.
- python3 tools/test_validate_stage2_android_lifecycle.py: exit 0, 25 pass.
- cargo fmt --check, git diff --check and strict Blueprint: exit 0.
- python3 tools/stage2_host_build.py and --verify-only: exit 0 after Cargo lanes.
- python3 tools/stage2_android_build.py --verify-only: exit 1, missing APK
  provenance; unchanged Gradle/ADB permission failures not retried.

Evidence JSON:
/private/tmp/stage2-prompt-tool-final-20261004/android-prompt-results.json.

```text
remote_headless_commands.rs dfbbda891d80d9b7cee71af0ff4ed985cf66fa3e5cf0213acb82bab2c6bb034e
AndroidRustPromptTest.kt 94304e5934411a76648089e158a051044a41d638828b7c64fec5340eee592d08
final-log d2768129e6a28e0a903f80bb99c677174406cc2760686aa984fb88bfe0f5c23f
prompt-results db00ca8f6deb1857eda5f4a7f6850e4f96b169c6d40f30576df4e4d52a5ed0d2
python-log 1b1a3bf58fd368cf32135700370ed109663048fb11e640bf5c7859c1ca3cb308
unit.jar cb1194dcd4edf2a5b43fe153a60ca509a41e7648b8a2dea8d0a760d44c7254fd
debug-host-receipt ce9d4f76ead4d1c5303de08b07644453c8bbdffff6089f0d6f5b782fe8e8b148
```

Goal active and incomplete. Current APK, physical camera QR/direct-LAN sync
and recovery, connected API31/34/35 lifecycle, complete desktop parity and
signed reproducible release remain required; fixture tool effects do not replace them.

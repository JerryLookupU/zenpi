# Kotlin Rust Worktree Interoperability

Provisional S2-017/S2-018/S2-035/S2-052 progress, 2026-10-04. The preceding
RTK installation-status turn did not change Stage 2 implementation or acceptance:
no goal progress. This continuation revalidated the dirty worktree, execution
skill, authoritative Blueprint, APK gate and current transport/control code,
then added an executable cross-language control lane. No nested agent, user
change reset, checkbox promotion or sandbox restriction bypass.

## Current Source Lane

New tests/remote_android_worktree_interop.rs owns an isolated temporary Git
repository, real SessionStore/Agent/ProjectOwnerPool, authenticated P256 device
and project Control ACL. New AndroidRustWorktreeTest builds commands with the
production Kotlin WorkspaceCommand factories from the current Rust owner's
listing, inspect and subtab projection. Rust parses the length-prefixed frames
and dispatches through HostAdapter. Kotlin parses the real response frames in
seven-byte fragments using ResponseReader and WorkspaceReceiptCodec and decodes
the refreshed production listing/snapshot/subtab projection.

Twelve sequential steps cover directory add, Git worktree add, stable-ID rename,
reorder, quotas 64 then 1, worktree selection, dirty refusal, clean removal,
directory selection/removal and main selection. All eleven successful operations
advance both revisions once, return an identical exact-retry receipt, reject a
fresh request with stale revision and reject an altered same-ID payload. Rust
checks unchanged post-retry state. Kotlin refuses admitting the old command as
a fresh write after the current projection advances.

Dirty worktree close preserves the untracked file, complete subtab state,
selection and both revisions. Clean close removes the checkout but preserves
the Git branch. Closing an in-place directory tab does not delete source files.
Final host state has only the original main tab. Twelve generation plus twelve
validation JVM helper executions pass; 45 response frames are checked.

The lane is wired into tools/stage2_android_source_check.sh after the existing
read, rename and proof lanes. Ordinary JVM counts exclude all three phase
helpers; normal Gradle assumption-skips them without supplied fixture properties.
Normal Cargo ignores the new cross-language test without freshly compiled JVM
classes. No provider call, camera, Android Keystore, socket/TLS, APK, real LAN,
runtime worker admission or full product parity is proven by this fixture.

## Verification

All commands below ran from /Users/mac/Github/zenpi against current dirty state.

- zsh tools/stage2_android_source_check.sh /private/tmp/stage2-worktree-interop-final-20261004:
  exit 0; production/unit/instrumented Kotlin compilation; 460 ordinary JVM
  tests; current Rust read fixture; explicit rename/proof/worktree lanes pass.
  Existing compiler warnings remain. Instrumented tests compiled, not executed.
- python3 -m unittest discover -s tools -p 'test_stage2*.py': 132 pass, exit 0.
- rtk cargo test --locked --offline --test remote_worktree --test remote_subtab --test remote_concurrency --quiet:
  exit 101. Concurrency 12 pass; subtab 8 pass, 2 fail at host.open because of
  configuration I/O Operation not permitted. Not a full passing regression.
- rtk cargo test --locked --offline --test remote_worktree --quiet:
  exit 0, 6 pass. Separate invocation because the preceding failed suite
  prevented Cargo from executing this suite.
- Rust formatting, git diff --check and strict Blueprint pass. All 36 rows
  remain [ ]. APK verify-only still refuses missing build provenance.
- Debug host rebuilt after test invocations; separate verify-only passes.

Two development runs failed before the final successful source gate: a Rust
test compared owned subtab values with borrowed values; then the Kotlin helper
incorrectly correlated stale-request responses against the original request ID.
Both test-harness errors were repaired. Each response is now checked against
the exact frame that produced it; production code was not relaxed.

```text
remote_android_worktree_interop.rs 35cfeeb72eff37039b6b69bfcfb67b5ddde824e5b16fbe7398b6160013be8eb4
AndroidRustWorktreeTest.kt ce68dccfc89dc27ed50ca44e83b6e31f4b5826df1cd30ab4b9ca2f1c4fdd269b
stage2_android_source_check.sh a10357159d47a96d3428964e389f5ed27714d683893bb6a8d8fd51ee41335ec9
android-worktree-results.json 6db333d19239c141404c8ee56e77dfcea3bc7862d19ad504e9c6d7da61dcad87
main.jar 66db20701d2cff5559ad457cceb93fea93c6657ff85cfe03bb94b1fd8b8d4386
unit.jar 881b992bb4620821abda6d083baa0ee2fce5746e25272201fbbc520bdd5ba0a4
instrumented.jar f3913356a11db188a5396e9e4f6c8246b7c5c3213de8127858cb44d5b5801332
debug-host-receipt 2908fd1963fd7a74bdde37438a463417a4ba93215e09b900763efe5f0ff5b9db
debug-host-binary 701d3e597cd640090cdc254f6ad2f810b986dbd626e42179ca3da6d8e50c5d77
```

Goal remains active. Current APK, physical QR/direct-LAN bidirectional sync and
recovery, connected API lifecycle matrix, full parity and signed release remain
incomplete or unverified. This source lane does not accept an entire item or
supersede the failing subtab regression or earlier full-suite failures.

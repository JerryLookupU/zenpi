# Android-Initiated Desktop Terminal

Provisional S2-019/S2-036/S2-054 progress on 2026-10-03. Previous goal work
changed task-history product source and tests, so it is progress. This turn
adds a missing user workflow rather than treating source checks as LAN proof.
No nested agents, worker admission, checkbox promotion, commit/push or cleanup.

## Product Workflow

Android's terminal icon sends an explicit terminal_open Control write when the
desktop advertises terminal_open_control. It binds project/session, active
subtab/revision, workspace owner/revision and request identity. Rust's existing
desktop owner creates and retains the PTY at the host-selected subtab root.
Android cannot supply a shell program, cwd or command to this operation.
The matching existing terminal is reused, never replaced; an unrelated
existing terminal, jobs/tasks, modal/pairing or stale scope refuses new creation.
The plain headless adapter does not advertise creation and refuses fresh opens.

The durable checkpoint binds the fingerprint; reservation precedes spawn and
completion records the terminal ID. Exact retries return the recorded result
without calling creation, even after the original terminal disappears. Failed
or ambiguous reserved creation stays unknown_outcome. Android encrypts the
exact pending request, verifies receipt scope/instance and refreshes afterward;
creation authority is never cached. Old hosts do not expose the icon.

## Verification

- Final Rust production-owner PTY regression passes: opens, reuses, inputs,
  resizes, reads output, refuses stale/busy/conflicting requests, preserves
  ambiguous effects and does not respawn on retry after instance disappearance.
  Command: rtk cargo test --locked --lib
  tui::local_remote_preview_tests::existing_terminal_projects_only_matching_live_session_and_root
  -- --exact (one passed). An earlier shorthand with --exact selected zero
  tests and was not counted. The first real run found the missing durable
  PromptBinding operation whitelist; it was repaired before the passing run.
- rtk cargo test --locked --test remote_protocol --test remote_auth
  --test remote_terminal: 34 passed. Control/Approval-only capability matrix
  explicitly tests TerminalOpen. rtk cargo check --locked passes with existing
  warnings; cargo fmt --all -- --check and git diff --check pass.
- Final Android source check /private/tmp/stage2-terminal-open-final-20261003:
  main/unit/instrumented compile, 373 JVM tests pass. Compose control tests
  cover creation and stale/reset/offline/lost-capability fences, compiled only.
- HostPairingTest now routes the production UI/ViewModel open and input path.
  It requires the same terminal ID and output. The desktop harness separately
  requires journal reservation/completion, checkpoint fingerprints, matching
  session/subtab and exact android-terminal-owned.txt bytes. Lifecycle matrix
  requires android_desktop_terminal_open_input_verified. Marker alone fails.
  This connected workflow is prepared, not executed; suite remains 71/72 tests.
- Stage2 Python discovery: 101 passed; desktop verifier: 17 passed; lifecycle
  verifier: 18 passed; instrumentation/Blueprint unit tests: five each passed.
- Debug/release binaries rebuilt with stage2_host_build.py after all Rust
  changes; final verify-only passes both. An intermediate debug verify refused
  the binary changed by cargo test, correctly requiring rebuild rather than
  trusting a stale receipt.
- APK verify-only still refuses missing provenance. No blocked Gradle/ADB
  retry, APK installation, camera scan or direct LAN execution is claimed.
  Strict Blueprint passes with all 36 items unfinished.

```text
Rust inputs 749affcdeb54968cdeb8dbcb786a4825da5d7fffcec6918a7aaa226e49acc16d
debug receipt 7332607f27938c7b9266e1fe4b3140075ead51b903d0586e22ad5ed81c715a7e
debug binary bf71af911d27a7b06a8fbb7939f00007c6919e4627420f0deb6b1b16c5099751
release receipt b8eb5cd332508b7b4b806d99807ec1b35590760a6e03fc28935d5c16a7c0ac8c
release binary bf2e008eeaa2c8a2daca440cdfc2a92d4a93075e1a6b0cb6313be84a31149dff
terminal.rs d6391a6dc1635af6f33b5611cd2cca02d13cd6bff23cfac34cff3ea58fd09dfb
tui.rs 59acf17a75994e00f130cfa6bca0fb1e0207dd1860e42d95f8cf64ecc2da16b7
TerminalCommandCodec.kt 8a9b3ac06e1d31e6231305292fb9793cc857220ff325e012dd5bc6d653eacb64
HostPairingTest.kt ad9fb8b52c0f32f2580fee35548e70ef404cd009e343a7b1f8e8eb195b19d9d3
stage2_desktop_android_smoke.py a17f21570dc44b7a7bd65903d9e77c15f5ac0751f3bbfaec34a68d543c26287c
main.jar 138bab1c5e096ecf6c19e6cfcfd48846c3688c4ff3e33da1d9e4d82dd8d537ad
unit.jar 9c66347f9f06e206729f9b992a847b0afa60d69aed67fd93387ad2660f5ab8cd
instrumented.jar 6ed294e37f552086385851d7eac29f1400aa1a9bd7a1d1008fe2b1028078a5a7
```

Full goal remains active. Current-source APK, physical QR/direct LAN desktop
sync, supported API lifecycle matrix, complete parity and signed release still
need behavioral acceptance. No current connected acceptance is inferred from
the real host PTY test or compiled Android workflow.

# Desktop Terminal Exit And Explicit Reopen

Provisional S2-019/S2-036/S2-054 progress on 2026-10-03. Previous goal turn
added terminal creation source and behavioral tests, so it is progress. This
turn found and repaired exited-shell control in the current worktree. No nested
agents, worker launch, checklist promotion, commit/push or unrelated cleanup.

## Product Change

PtyShell observes its owned child with nonblocking waitpid and stores a one-way
exit state. It reaps only that child, clears queued input, rejects checked input
and resize with BrokenPipe and does not signal an already-reaped PID on drop.
Output remains readable. A lifecycle-dirty bit survives read-only inspection
until pump wakes the session subscriber, including exit without further output.
Desktop rendering at the same scope does not automatically respawn the shell.

The current terminal projection includes running. Exited instances retain their
bounded screen/history but terminal_control is false. Android strictly decodes
the optional boolean and hides input/resize even if a stale capability remains
true. Terminal exited is displayed; explicit terminal_open creates a new host
instance at the same scope. Pending exact-request resolution still returns the
old receipt, never dispatches to the new instance or reopens automatically.
Legacy projections without running retain their prior compatibility behavior.

## Verification

- rtk cargo test --locked --test remote_terminal: nine passed, including a real
  shell exit, retained final output, rejected input/resize and no file effect.
- rtk cargo test --locked --lib pty_shell::: 13 passed, covering parser/output
  bounds and shutdown. The exact production desktop PTY regression also passes
  one test: exit projection, capability loss, stale input refusal, old retry
  without respawn, explicit distinct instance and repeat reuse.
- rtk cargo check --locked: no errors, existing 20 warnings. Rust formatting,
  git diff --check and strict Blueprint pass; all 36 rows remain unfinished.
- Final Android source check /private/tmp/stage2-terminal-exit-final-20261003:
  main/unit/instrumented compile and 375 JVM tests pass. New JVM cases cover
  lifecycle types/instance binding, exited input/resize and pending reconciliation.
  Compose exit/reopen controls are compiled, not device executed.
- HostPairingTest's production ViewModel workflow now exits the old instance,
  explicitly reopens a distinct instance and writes a second file. Desktop
  harness requires five distinct ordered open/input identities and ten journal
  reservation/completion receipts, old/new instance grouping, same scope, exit
  byte count and both exact host files. Saved command evidence includes those
  journal records and file hashes. Marker-only or unchanged-instance evidence
  fails; lifecycle matrix requires android_desktop_terminal_exit_reopen_verified.
  Suite method count remains 71/72; connected workflow is not executed here.
- Stage2 Python discovery: 101 passed; desktop verifier: 17 passed; lifecycle
  verifier: 18 passed; instrumentation/Blueprint tests: five each passed.
- Debug/release rebuilt via stage2_host_build.py after final Rust changes;
  current-source verification passes. APK verify-only still refuses missing
  provenance. No blocked Gradle/ADB retry, APK install or physical QR/LAN claim.

```text
Rust inputs 2fa078489a749991bf55b24c8d412bdf554bf9ba0cf37e78cf83f44683aecb4f
debug receipt a0d7a9f89ee828e093e4ea6d2f34b4086c857409fad1749b4eedf4db3409f671
debug binary 57b52b80beda551642c47cd1f404f2fd1400b7a16699f91401402cb77db227a8
release receipt fda97d390477c0f6ba7c65c98856e684d98564272fa7f6ef05830a1c054d5e4e
release binary d2972f8e6d72c502273188208da2d65bf030e3ac2996fdbacc5ea588cf06bd68
pty_shell.rs 397e861b3985e1054c748dc7bcd3721c25b0bc6ce129fbd38137e1a97df9ae2e
tui.rs a2cf15e11d03efe834dae46c55acf355bed9a1187d18b6457e2d9e8436230f9c
terminal.rs a247094c230260201679e110795ca14fc13225e4aae2a817fb7e3b5fa9076db5
TerminalProjectionCodec.kt 1d6ee6018e776b328d569c6c2170b6047f59aec808153db7e2f0b185588ad50b
stage2_desktop_android_smoke.py 1f50a4fa1acdb1beaf6b3b4753008f7d5f2dccc0293e0b59dbe51fc39fd97dcd
HostPairingTest.kt 01ca07ae10a6e0d3a30ded82459cf6a7db46b67be11b1400fac628035fc62df5
main.jar 58cd1ad1f0947488b230d63632cad0383dd8ed9d906c9d96c8c8fb836da5f0ed
unit.jar 529e1b6f600360f55d4d85b459d03d2e49b01e55951cf0851aed60dfe8d0a119
instrumented.jar 9d47d81262099850f4eb814e5e99ae780bf80e1be99456ff344670e0abc67891
```

Goal remains active. Current APK, actual physical QR/direct LAN desktop sync,
API lifecycle matrix, complete parity and signed release remain unaccepted.

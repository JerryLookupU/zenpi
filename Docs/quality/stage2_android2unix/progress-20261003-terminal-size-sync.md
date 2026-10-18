# Actual Desktop Terminal Geometry Sync

Provisional S2-019/S2-036/S2-054 progress on 2026-10-03. The previous goal turn
prepared multi-page connected diff acceptance. This turn fixes actual terminal
geometry projection and Android size-field synchronization. No nested agents,
worker launch, checklist promotion, commit/push or unrelated cleanup.

## Product Change

PtyShell::size reads TIOCGWINSZ under the existing owned-master lock. TUI
terminal projection includes optional exact size {rows, cols} only for valid
1..512 dimensions and the already-matching project/session/tab/root/instance.
Query failure, pre-slave-open or out-of-contract dimensions omit size. Reads
never resize, create a shell or add an admission/journal event.

Android accepts old projections without size and strictly rejects non-integer,
unknown-field, unbounded, malformed or instance-less dimensions. Rows/Columns
initialize from actual host geometry and reset on geometry changes. Output-only
updates preserve edited size fields; geometry updates preserve input drafts.
The metadata stays transient, never restoring cached terminal authority or
changing history instance/scope identity. Input/resize durable request and ACL
boundaries are unchanged.

## PTY Verification Repair

The first new real-PTY geometry regression queried before the slave had opened,
which can return ENOTTY on macOS. The fixture now waits for an actual split-marker
shell response before setting/querying geometry. No guessed-size fallback was
added to production. It verifies remote resize persists through same-geometry
desktop redraw, a changed desktop geometry takes over, invalid resize leaves
actual size unchanged and the slave's stty size independently matches.

An earlier test run stalled during PTY destruction and was explicitly interrupted
through its session handle (exit 130), not reported passed. Inspection found the
old nonblocking reader depended on EOF after killing the shell and could spin
on EAGAIN indefinitely. It now owns an atomic stop flag, checks it before reads,
backs off briefly on EAGAIN, and stops before join while the master fd remains
owned. This does not add shell process-group or descendant termination guarantees.
The release build that started before these source changes was explicitly
interrupted; no stale build receipt is claimed.

The delayed reader also exposed a fixture race: file side effects can precede
screen receipt. TUI regression now waits for actual projected terminal bytes,
not only the output file. A /bin/sh run initially failed that old assertion;
the final repaired TUI group passed both tests.

## Verification

- Real remote_terminal suite with SHELL=/bin/sh: six passed, including actual
  kernel/slave geometry and existing redaction/UTF-8/control tests. Final rerun
  with the default shell also passed all six and exited cleanly.
- TUI local_remote_preview_tests group with SHELL=/bin/sh: two passed, including
  authenticated durable resize/exact retry and scoped actual-size projection.
- Shared pty_shell::tests group with SHELL=/bin/sh: ten passed, including real
  shell output, wide-character/editing parsing and wrapped-secret projection.
- Final Kotlin source check at /private/tmp/stage2-terminal-size-final-20261003:
  main/unit/instrumentation compile passed; 364 JVM tests passed. Compose
  TerminalControlScreenTest now checks actual defaults, output-preserved draft
  and host-geometry reset, but was compiled only, not device-executed.
- Stage2 Python discovery: 95 passed; lifecycle verifier: 18 passed;
  instrumentation and Blueprint tests: five each passed. Strict Blueprint and
  git diff --check pass; all 36 checklist rows remain unfinished.
- Debug/release attributed builds and final verify-only succeeded after all
  Rust changes/tests. A debug test relink invalidated its receipt; the tool
  rebuilt it rather than manually stamping the artifact. Current APK
  verify-only still refuses missing provenance; no stale APK install or
  unchanged blocked Gradle/socket retry occurred.

```text
main.jar 9125c73ea0373cbe52c01e51d6ba514045a50e079705702f8d1c0025d2194008
unit.jar 073545d4b7fac2bf19562c92dbbb1b7492247d807f11a1cc3d94a0d6d394f32d
instrumented.jar 2db0da4fcacedcdbe8b32f53a2e7f64868e3277bc4cec9860e08e217d03877d1
src/pty_shell.rs 494db4a59135a17e9732535a81423bdb455fa2857ea8cc91d419021e3d3a4f63
src/tui.rs ea4a4d442a9a16485ed4cf761e661ef56bc30eca8896e0bad6f6dc4f33a748bc
tests/remote_terminal.rs bab1aa4d177ae2e05c6dd916da9c9da1272adcc9bcad313385ce20d682654d73
host inputs a7c06289df0dc51c54679ce1fa233f1fae858fc3e201ad7334949e24f461dcff
debug receipt 440ed29318326ccb15c28ba9faa72999c536680bd16e2651b18da06a0a9d4c95
debug binary 5042190f6c906f11262044d8b7d087294290afe9ee9d74208d981b02d0d56f35
release receipt 8ba074416a267caa1e7cb4a1deb4f66bb0ab5f974bdd68dac2848e5d3f5cfa7f
release binary e6ecf834d4aa217973f4149d76cf598242c2a174da7972f18c593a7b60ae0666
```

Goal remains active. Current-source APK, production Android/desktop TLS sync,
physical camera QR/direct LAN, API lifecycle matrix, full terminal/settings
parity and signed installation/upgrade remain required and unaccepted.

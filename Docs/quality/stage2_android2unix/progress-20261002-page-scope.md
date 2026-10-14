# Android Page Scope on Branch Replacement

Provisional S2-020/S2-036 progress, observed 2026-10-02T02:21:38Z. Previous
goal turn fixed host selected-branch projection and paged TLS: progress. Full
native Android/QR/LAN/Rust desktop goal stays active; all 36 rows remain [ ].

## Implementation

PairingViewModel reconciles an open/in-flight transcript page before publishing
each verified subscription or final projection. If project/session or target
turn identity/role/content/truncation changes, or the target disappears from
the retained display, it cancels the job, advances its generation and clears
the dialog. Page completion also checks exact target retention against current
projection, not just matching project/session. Unrelated append/status updates
retain an unchanged page. This fixes stale page display after desktop branch
switches and prevents an older pending response from reopening removed content.

TranscriptPageCodec owns the shared pure target-retention check. Two tests
cover removed/foreign/changed/duplicate targets, missing old authority and
unchanged append/status updates. A third replay test decodes branch replacement
and empty-selection snapshots with an unchanged physical event cursor, checks
ProjectionCommitPolicy accepts the newer journal, confirms no abandoned
messages or page authority survive, and verifies cache round-trip replacement.
ViewModel wiring is source-compiled; this is not a device-executed race test.

New TLS test uses real rustls TLS1.3, host certificate verification, signed
P256 device challenge and production HostAdapter over bounded fragmented memory
pipes. It reads the first redacted page, changes the actual owner tree leaf,
then verifies the next old page returns InvalidRequest/null on the same stream.
Inspect returns only the alternate turn; Resume emits a matching replacement
snapshot with physical cursor 2, followed by correlated successful terminal
response. Journal bytes are captured after the deliberate local tree-selection
write and remain unchanged by subsequent remote reads; checkpoint unchanged.
This is encrypted transport evidence, not TCP/LAN/phone/camera evidence.

## Verification

Cwd /Users/mac/Github/zenpi. Rust tests use
ZENPI_HOME=/private/tmp/zenpi-page-scope-20261002.

- cargo test --offline --locked --test remote_subscription_tls: **15 pass**.
- cargo test --offline --locked --test remote_workspace --test remote_headless
  --test remote_subscription --test remote_events --test remote_protocol:
  **53 pass** (25/1/10/6/11). Combined Rust integration coverage: **68 pass**.
- zsh tools/stage2_android_source_check.sh
  /private/tmp/zenpi-page-scope-final-20261002/android: exit 0, main/unit/device
  sources compile; **148 JVM tests** pass in 27 classes. Earlier 147-test run
  preceded the final branch-replacement test and is not final-source evidence.
- Five Python smoke/lifecycle/blueprint modules: **53 pass**.
- cargo fmt --all --check, git diff --check, strict blueprint validator pass.
  Rust production code is unchanged this turn; existing compiler warnings remain.

SHA-256 source:

```text
tests/remote_subscription_tls.rs 1f8271ef8461e1faca3743967da73a17769a6cbd9e14b584c08872a0b2e95408
TranscriptPageCodec.kt 029439e8fedcb90592bf1c86974e192cd673cad6c6f438d62e9413f5471370be
PairingViewModel.kt a23b0fca96f702392ec6f9d6faa7c996044401c6699f72f39ebf8c9290a6217b
TranscriptPageTest.kt ecd6ea8da83de2d340d8aee533c7542879502d655c6faea911349a797763883a
ReplayTest.kt 46e97f5aafb4bdcb5b1dd7fe78049f68fb3f6dcceec7c5889eaa565848404bc5
```

Compiled evidence under /private/tmp/zenpi-page-scope-final-20261002/android:

```text
main.jar 48f701455aad443ac43e53db43ff256077d63e68fb591088002d042e0c79b156
unit.jar 7a7c2c7518346969c7bc75492ef9c0310e88a6032079b26812dde72d3f727a4c
instrumented.jar 96e9dab756ed1fc648f1577e330e0df8318577f8d47411bc932e3fd1820a7bbf
```

Gradle/APK is not reattempted this turn: prior turn's wrapper EPERM remains the
latest build evidence, not a current-source packaging pass. No new APK,
physical camera QR, same-LAN device, API31/34/35 lifecycle, release/migration
acceptance. Effective worktree quotas, full routing/display parity, durable
provider delta replay and interactive PTY remain unfinished. No agents,
permission bypass, commit/push, unrelated rollback or acceptance promotion.

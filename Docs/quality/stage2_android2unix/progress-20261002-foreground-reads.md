# Foreground Read Lifecycle Boundary

Provisional S2-038/S2-055 progress observed 2026-10-02T01:53:44Z. Previous
goal turn implemented remote preview truncation and ran code/reducer gates:
progress. Full native Android/QR/LAN/Rust desktop objective remains active;
all 36 authoritative rows remain [ ].

## Changes

Public ViewModel refresh previously fenced foreground only when carrying a
write command. Pure refresh could change offline display to Connecting and
start authenticated sockets after onStop. reviewDiff similarly lacked the
explicit foreground/reset guard. Both now return before any state mutation
or launch when not foreground or local reset is active/required. Exact saved
command matching remains enforced. Normal onStart, network recovery and
post-command foreground refresh behavior is unchanged. Background reads
remain the separate bounded ResumeWorker path with encrypted-record CAS;
no unresolved-write flushing or background subscription is introduced.

Add ForegroundReadTest against actual encrypted PairedHostStore + production
ViewModel. Background pure/selected/foreign refresh and diff callbacks must
retain exact offline state/cache bytes. Extend existing controlled-running
headless instrumentation to invoke these reads during the offline interval
while provider completion is held. Add independent read-fence phase before
foreground reattach. Extend reset instrumentation to invoke both reads during
active/failed reset. These device cases are SOURCE-COMPILED ONLY this turn.

RunningStreamGate now requires preview -> displayed -> background writes
fenced -> background reads fenced -> reattached -> provider completed. Provider
cannot be released before the last required Android phase. API matrix gate
and receipt flag require read fencing; test rejects older write-only evidence.
Documentation describes this separation from WorkManager behavior.

## Verification

Cwd /Users/mac/Github/zenpi:

- zsh tools/stage2_android_source_check.sh
  /private/tmp/zenpi-foreground-reads-final-20261002: exit 0. Kotlin1.9.24,
  Compose1.5.14, SDK35 cached artifacts compile all main/unit/instrumented
  sources. **137 JVM unit tests pass** in 26 classes. No new device test
  runtime pass is inferred from those JVM tests.
- env PYTHONPATH=tools python3 -m unittest
  tools.test_stage2_desktop_android_smoke tools.test_stage2_headless_android_smoke
  tools.test_stage2_scanner_android_smoke tools.test_validate_stage2_android_lifecycle
  tools.test_validate_stage2_android2unix_blueprint: **53 pass**, exit 0.
  Gate tests verify no release before read-fence/reattach, reject skipped phase
  and old matrix evidence. These are harness tests, not live Android runs.
- python3 tools/validate_stage2_android2unix_blueprint.py --strict: exit 0,
  36 incomplete rows. git diff --check passes. Rust unchanged this turn.

SHA-256 source:

```text
PairingViewModel.kt e958d1e12efb14b58fd2ec9f8cb533f4e508251566d2671a43dd3df0578f27f7
ForegroundReadTest.kt fbf764a444506ed01ad81ef225ffb23be0e6014b335229c5b52d281146bb709f
HeadlessHostTest.kt 8f166d25fe31b6c01cc4241a5f84df40c38d8d18f564231eb3fb12ed6d40dd1e
LocalResetTest.kt 2fad4baa9cd2390635e349ba0ffbae16b416d5263b1df2ff21a5a07e4da088e8
tools/stage2_headless_android_smoke.py 210d0113624a962ca3ab3c9cd3c41454cb872834d729d66c0b35e3c1c615c1be
tools/test_stage2_headless_android_smoke.py 93f70fc2279e300527969ddb548d9d5922ed6be27c3a7dffa284cfc56cd30859
tools/validate_stage2_android_lifecycle.py 489067f3830a963eb8fb393cf26b46fe7d8f709dc68d76f4237b5b7eb7833aef
tools/test_validate_stage2_android_lifecycle.py bfe58f025da62f5a4c2122bc33e10b56ec8d345eeeb523175899607caf21fdef
```

Compiled files under /private/tmp/zenpi-foreground-reads-final-20261002:

```text
main.jar ed835deca3f9492433b3e2e71e4b38062f92f6efa379956c8c5f655a284221fa
unit.jar f7e566a45c81bc3eb1fc95de771b0e173282f0ae40e45b830bb0ba0447661ea2
instrumented.jar 970f62ac3245c3f2a3b9b9612cbf0d7138d0654680f69f3909188768e471ed63
```

No fresh Gradle/APK/device attempt or physical QR/LAN acceptance this turn;
existing wrapper-cache permission blocker is neither bypassed nor treated as
full-goal impasse while code progress is available. Fresh API31/34/35, physical
phone sync, per-worktree quota scheduler, complete TUI/headless parity,
stream delta replay, interactive PTY and release/migration remain unfinished.
No agents/commit/push or user-change rollback. Goal remains active.

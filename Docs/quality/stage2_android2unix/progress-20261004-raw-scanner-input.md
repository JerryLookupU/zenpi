# Raw Camera Invitation Admission

Recorded 2026-10-04. Partial S2-031/S2-052/S2-072 evidence only, not Master
acceptance. Previous goal turn was progress: refreshed release attribution,
independent repeat builds and current Rust/Kotlin interoperability. Inspected
the current dirty worktree, execution skill, Blueprint, Gradle/APK provenance,
camera callback, strict invitation parser and saved-host lifecycle. No nested
agents, reset, relaxed deadline, acceptance checkbox or denied Gradle/ADB retry.

## Repair

PairingViewModel.scanResult previously called acceptUri(contents.trim()).
That removed surrounding raw controls and arbitrarily large whitespace padding
before the strict parser could apply its raw-control/2048-byte admission gate.
A QR containing a padded invalid invitation could therefore reach confirmation
even though the parser rejects its unchanged content. This did not bypass SAS,
TLS pinning or host token validation, but contradicted the scanner input gate.

The callback now feeds unchanged decoded text to acceptUri, matching deep-link
admission. Valid production QR text remains accepted. PairingInviteTest covers
leading/trailing space, newline, CRLF, tab, NUL, DEL and oversized padding.
ScannerResultTest exercises the actual callback with malformed/padded content,
requiring no invite, unchanged saved host/cache and no raw-payload error echo.
Its existing test identity/count remains unchanged; it emits the new exact
scanner_raw_payload_fence_verified phase. The scanner harness requires this
phase exactly once, and Python tests refuse missing/duplicate markers even
when all previous checks pass. Security and network matrices were updated.

## Verification

All commands ran from /Users/mac/Github/zenpi. All validation handles terminal.

- zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-raw-scan-interop-20261004: exit 0; main/unit/instrumentation
  source compilation, 593 ordinary JVM tests and every explicit Rust/Kotlin
  lane pass, including current Rust QR pixels, software-P256 proofs, actual
  workspace/worktree/session/PTY owners and prompt/worker cancellation.
- Scanner harness Python tests: 17 pass. Production callback instrumentation
  assertions compiled only; no connected-device result claimed.
- Ten additional Rust targets exit zero under a dedicated ZENPI_HOME:
  remote_approval_wakeup/device_grants/diff/faults/headless/lan/live_text/
  project_inspection/subtab/terminal. --list confirms 2/5/1/3/1/8/10/5/11/9
  primary tests, 55 total. RTK reported 58 across 13 suites because the grant
  target runs three isolated child libtest invocations; 30 filtered belongs
  to those exact child invocations, not newly excluded primary tests.
- First full Python Stage 2 run exits 1: 163 tests, two host-smoke fixture
  child deadline failures (-9 versus expected -15/0) during overlapping Cargo
  work. No production timeout or test expectation changed. Host-smoke scope
  alone passes 13 tests; the subsequent full suite passes 164 tests. Keep the
  first failed observation; the startup-load explanation is an inference,
  not proof of its root cause or a claim that repeat failure is impossible.
- Lifecycle validator tests 25 pass; Blueprint tests five. Strict Blueprint,
  controller validate, cargo fmt --check and git diff --check pass. Controller
  validate launches nothing; 36 [ ], zero [_]/[x].
- Debug attribution rebuilt after Cargo lanes; debug/release verify-only pass.
  Retained independent release comparison still verifies against the current
  attributed release. Rust production sources unchanged by this repair.
- APK verify-only still refuses missing provenance. No user device answer;
  physical camera/direct-LAN and current connected API31/34/35 remain open.

```text
PairingViewModel.kt 7c191f8b6d6d57dc7d10c8094b705d719e976dd37cccb45439abb34905fa292b
PairingInviteTest.kt 9a23d85d7ccaeedd877e552d79aba4dc73aeb0c22f48dc0f19579426ad710c7f
ScannerResultTest.kt 36e5d1008fa9bb109e460a33d557a7d9b956bb070baaf132ce612fc53c45d2e6
scanner-harness 6c5e037f44d444d785328a77f709c69173dcdfd9cc9e41bbe4445ddf581fdd61
scanner-harness-tests 83bdbc3b18a1496bf05165b1efac0fbcac2aacf6f3245ea3086e4264e05da692
main.jar 4c4fcbc77c49b0626b10a5b44647f852acfab2e3bda03b4b4a1368af14d8270b
unit.jar 53579dbfbcf54a05b16f56427cf769a0276f01dda11463a5ed8d1aa958b6d946
instrumented.jar f6e33cd74cd951f10de13d7e93f3337b9d5e1d4aa7dda8cc476358299c5a2ae0
debug-receipt b3a3033c0528efbbbec89c12b777a3f879bde7f3390defcc366ebccaf2d19323
```

Goal active/incomplete. APK/signing/install/upgrade, physical camera/direct-LAN
bidirectional synchronization/recovery, current connected Android lifecycle,
supported-host runtime and full unsuppressed Rust/network acceptance remain
required; source/JVM/owner evidence cannot substitute for those gates.

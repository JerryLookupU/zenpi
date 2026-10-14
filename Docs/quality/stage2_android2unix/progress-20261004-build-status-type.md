# Build Exit Status And Full Gate Audit

Recorded 2026-10-04. Partial S2-051/S2-070/S2-071/S2-072/S2-099 evidence,
not Master acceptance. Previous goal turn made real cold-invitation queue
progress. Current execution skill, dirty checkout, Blueprint, controller and
host/Android build boundaries were inspected. No nested workers, resets,
permission bypass, signing changes or checklist acceptance marks.

## Repair

Both build-source verifiers compared decoded exit_code to zero without checking
its type. Python considers False and 0.0 equal to integer zero. A malformed
receipt with otherwise matching source/artifact metadata could therefore pass
the success gate. APK and host receipts now require type(exit_code) is int and
value zero. New fixture matrices refuse false/true/float/string/null while
retaining legitimate integer-zero receipts. No runtime/wire behavior changed.

## Full Rust Audit

- cargo test --locked ran without suppressed tests: exit 101 at the library
  target, 403 passed, 50 failed, eight ignored. It compiled in 9m36s. Failure
  output includes local listener EPERM, inherited configuration write EPERM
  and downstream worker admission assertions. Later integration targets did
  not run after the failing library. This is a failed full gate.
- A distinct library diagnostic with a fresh writable temporary ZENPI_HOME
  (env ZENPI_HOME=$(mktemp -d /private/tmp/stage2-full-gate-home-20261004.XXXXXX)
  cargo test --locked --lib) ended exit 101: 432 passed, 21 failed, eight
  ignored, zero filtered. Every remaining failure reports local socket
  Operation not permitted in auth::callback, auth::codex or backend::explicit
  test setup. No cases were skipped or promoted to passing. This separates
  the inherited configuration problem from unavailable socket acceptance.
- cargo check --locked passes with existing warnings; cargo fmt --check and
  git diff --check pass. No production Rust source was changed this turn.

## Tool And Artifact Verification

- APK builder fixture suite 20 and host builder fixture suite 13 pass.
- First concurrent Python tool run failed two host fixture-child timing tests
  (killed exit -9); not counted as passing. Host smoke fixture suite then
  passed 13, and final whole Stage 2 Python suite passed 169 in 4.329s with
  unchanged production deadlines. This does not establish production TCP.
- Debug/release host build receipts regenerated after verifier edits changed
  the attributed source inventory; both verify-only gates pass. Binary bytes
  unchanged. This is cached Cargo source attribution, not a new independent
  reproducibility comparison. Older repeat receipts can be stale after tooling
  identity changed and are not promoted as current evidence here.
- Strict Blueprint remains 36 [ ], zero [_]/[x]. Controller status showed
  zero active/live/authenticated claims, requests and queued handoffs.
- APK verify-only still refuses missing provenance. No new Gradle/ADB/phone
  run; physical QR/direct-LAN sync/recovery, signed install/upgrade, supported
  Android lifecycle and full unrestricted host gates remain required.

All launched validation sessions ended. One diagnostic directory listing
stalled and was explicitly interrupted (exit 130); it is not validation
evidence. ps/sysctl process/capacity observations were denied and are not
inferred from file timestamps. No unrelated processes were stopped.

```text
android-builder d93567f0d349335eb5b8924d1a2c1967886027a3b0f4a9d640b2c52d61c5b62e
android-builder-tests 0c0a12669799cc730ff1a11774857303bf4a4c634b685ee64b707ace8ad0bdad
host-builder 9ae5abd841576e149f8633500f494a170742411fad4af689ed6d47d4bfc6b532
host-builder-tests e8d580df97bc82098a4af1589f8d51dd3f61ba941fd3d6f49677c1baab25fdd8
host-inputs 9aa1b19a756f3a031cf5925c3b241e95bb18a1781627a44749d62a2a33028086
debug-receipt 3c17b08dced8560e8d8eb4769ca3b7969b47a5381fd49c3bb84be1bc984f320b
release-receipt eaea698082564e190c5de4577c0c80a9e52fe9783203462762a7e825ab4131f9
```

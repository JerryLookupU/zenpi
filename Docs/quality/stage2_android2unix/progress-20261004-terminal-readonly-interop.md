# Actual Read-Only PTY History Interop

Provisional S2-036/S2-054 progress. Previous goal turn implemented terminal
read admission and was progress. Current skill, Blueprint/specification,
worktree and actual PTY/Kotlin integration inspected. Existing changes
preserved; no nested agents, reset, push or checkbox promotion.

## Stronger Evidence

Prior policy assertions used a Read-only Kotlin projection while the real host
page dispatcher still had a controlling fixture identity. Extended the same
actual PTY lane to lower the registry grant to Read, reauthenticate at its new
revision, and obtain the actual host workspace/subtab listing. Only retained
screen enrichment is supplied as in the desktop mailbox; no Control is added.

All PTY pages now dispatch under that actual Read-only identity. Kotlin decodes
the host listing, rejects input/resize construction, reads fragmented history
frames and checks TerminalReadPolicy. Fresh input/resize requests from current
Kotlin command frames are sent to Rust despite lacking authority and both fail
unauthorized. A foreign-session read fails owner_mismatch. After full two-page
history, revocation fails the retained request with unauthorized; current Kotlin
also decodes each refusal. Previous command deduplication/restart receipts,
redaction, full history digest and no-repeat PTY effects remain tested. Reads
and denied operations preserve the actual owner journal/checkpoint and the
single prior file effect. The source checker retains terminal exchange JSON.

First run failed because the fixture refreshed workspace listing after owner
restart but retained the old subtab owner. Kotlin correctly refused Subtab
owner mismatch. Repaired the fixture by reading the actual restarted SubtabList
under the reader identity, without weakening the production decoder. The
complete repaired run supersedes the initial failed attempt.

## Verification

Cwd /Users/mac/Github/zenpi; HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b; UTC 2026-10-03T23:59:17Z.

- zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-terminal-readonly-repaired-20261004: exit 0; 500 ordinary
  JVM tests, main/unit/instrumented Kotlin compilation, current terminal QR
  decoding and all explicit current Kotlin/Rust lanes pass. PTY lane executes
  Read-only pages and refusal checks; model/resources/files/diff target 4 pass.
- Retained terminal exchange JSON inspected: 2 pages; input/resize unauthorized;
  foreign-session owner_mismatch and post-revoke unauthorized.
- Python Stage 2 discovery 135 pass; lifecycle verifier tests 25 pass.
- cargo fmt --check, git diff --check and strict Blueprint pass; 36 rows [ ].
- Debug host rebuilt after Cargo lanes and verify-only passes.
- APK verify-only refuses missing provenance. Instrumented UI tests compile
  only; no stale APK installation or permission bypass.

```text
remote_android_terminal_interop.rs ade69c23ef40324500719b18bdfd8ee64cd6b03346d37428b2e1c7cc7a7b1853
AndroidRustTerminalTest.kt 88e190ae4f41f6588979b0aa6dd5e61987f533b1f4ee32070d31c94d159cd183
stage2_android_source_check.sh ef957a91b27f8122ee6b4ad383ed63e27cd4e421f1c91838b3d9ef5480577d46
source-log c129224b58830d707749189a08161b7657ed6adefd1ce512eda3a711f716b650
terminal-results c5475d9b2278cb9dca34c18b74f8ff44579dd15f37d9652f9097fdc8f8883f36
main.jar 33c124ef9c6a414a245da931de98bcd80595804a150851704a0943399410efd7
unit.jar 034ecf399e102b4da0b07e9cf582a1bb8916b08a73bb42eb9ce0560b9d28c800
instrumented.jar 0f6b390e07104440701bd0d3a0edf5f1569e7734f954aff4be2d23b0ffb57ae5
debug-host-receipt d7ab1a0a02be70069a36a088a7ebbb27146814a74bf7798c8046e79529347563
debug-host-binary 51cf433e9ebfd59aeebdbf46f844c0603872e798f8e4b6443308aed8627eca67
```

Goal active and incomplete. This establishes actual host Read ACL/PTY plus JVM
framing, not production desktop listener/TLS, Android Keystore, current APK or
physical QR/direct-LAN sync/recovery. Connected API lifecycle, complete desktop
parity and signed reproducible release remain required unaccepted gates.

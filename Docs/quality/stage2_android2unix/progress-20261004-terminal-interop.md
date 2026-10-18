# Android Terminal Commands and Actual PTY History

Provisional S2-019/S2-036/S2-052/S2-054 progress. Previous goal turn added
approval interoperability and was progress. This turn reread execution skill,
dirty integration, Blueprint, Kotlin terminal command/projection/page codecs,
Rust terminal dispatch/read, actual PTY tests and desktop mailbox enrichment.
No nested agents, checkbox promotion, reset, commit or push.

## Change

Added remote_android_terminal_interop and AndroidRustTerminalTest as an explicit
source-checker lane. Kotlin builds current terminal resize/input/read frames
from the actual host workspace/snapshot/subtab data and terminal metadata.
The fixture enriches the base listing with terminal::screen plus instance ID
and control availability, matching the desktop mailbox shape but not invoking
the desktop mailbox itself. Rust decodes the frames, dispatches to an actual
owned PtyShell and verifies real files in the fixture's temporary workspace.

Wrong subtab revision, wrong terminal ID and read-only grant cannot write.
Valid resize changes the actual PTY to 33x91, observed by stty; valid input
appends exactly one ANDROID_PTY_ONCE line and generates 240 history lines.
Identical retry does not write again; changed input under the same request ID
is refused. Restored host/journal returns the original resize/input receipts
without a supplied shell. A fresh current-owner command with no shell refuses.
Journal/checkpoint bytes remain unchanged throughout duplicate/history reads,
and raw secret fixture input is not in the journal.

Kotlin requests the first retained-history page and each next page. Actual
Rust responses are read with fragmented input, validated for identity/digest/
offset/continuity, and reconstructed in Kotlin across two pages. SHA-256 of
joined UTF-8 matches the host digest, first/last lines and completion marker
are present, and the secret assignment is absent. Final shell exit drains
queued input before the once-only file and absence of conflicting effect are
checked. Android never executes a shell locally.

## Verification

Cwd /Users/mac/Github/zenpi, HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b, dirty integration preserved.
Final observed UTC 2026-10-03T21:21:48Z (Oct 4 local time).

- Preliminary source run: exit 101 at new test's no-shell assertion. Actual
  OwnerMismatch correctly preceded the intended InvalidRequest because the
  new request retained the old owner after restart. Fixed only the test's new
  request to use current owner/revision; exact old retries retain old identity.
  Focused rerun passes. No production refusal relaxed or test skipped.
- zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-terminal-interop-20261004-final: exit 0, under zsh
  pipefail/rg output filter. Current six-case QR fixtures, Kotlin main/unit/
  instrumented source compilation, 474 ordinary JVM tests and all current
  read/rename/P256/worktree/session/ACK/approval/terminal interop lanes pass.
  Terminal lane has three JVM invocations: generate and two history reads.
- env ZENPI_HOME=/private/tmp/stage2-terminal-regression-20261004 rtk cargo test
  --offline --locked --test remote_terminal --test remote_workspace --quiet:
  exit 0, 81 pass. Not full Rust suite or real TCP/Android acceptance.
- Python Stage 2 discovery: 132 pass; lifecycle validator tests: 23 pass.
- Rust fmt check, git diff --check and strict Blueprint pass, 36 rows [ ].
- Debug host rebuilt after Cargo/source lanes and separate verify-only passes.
- APK verify-only still refuses missing provenance. No Gradle/socket bypass,
  stale APK install, connected camera/LAN/lifecycle or release claim.

```text
remote_android_terminal_interop.rs dda17ddfc30f3326ad4d74e4fbf03f75b87a3fc2d321467f1a5a3b940cbe40b9
AndroidRustTerminalTest.kt 0cc40255df086cdf401dc45ea92b1273b799669b9d435b292358821dc189f701
stage2_android_source_check.sh f73a5a9bcfd416c38b6a330592b9e5e5a282bf8f25f85ba8a359827f1ada4b0d
main.jar d11429817490daef47383be790488fb2dad445c406e07ac91121277872d4f7c9
unit.jar 0f83da08e3d69684c1edde5cd9b9eec48cb0ec352120d2df9f85f8a50b1885a1
instrumented.jar 8c22b80093a6d240fc71428f32893dddb9b7a3c7fe1e5b4efaff04ca507bba8d
debug-host-receipt 115d9b54a5688d8056889d1817e9673037a13f9de8981250e730a6d40be383b1
debug-host-binary 677c02131aa670e18cd3813bb1a8016026b375a3155f9c010b0c62f29d287df4
```

Goal active and incomplete. Real PTY/filesystem effects establish that host
boundary only, not production TUI mailbox, TLS listener, Android UI/Keystore,
provider or physical LAN. Fresh APK, camera/direct-LAN two-way synchronization
and recovery, connected API31/34/35 lifecycle, complete parity, backup/reset/
uninstall and signed release still require acceptance evidence.

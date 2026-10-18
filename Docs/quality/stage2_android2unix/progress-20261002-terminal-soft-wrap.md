# Terminal Soft-Wrap Redaction

Provisional S2-020/S2-036/S2-052 progress. Previous goal turn fixed proof-byte
compatibility. This turn reattempted standard APK packaging and fixed a concrete
desktop terminal projection privacy gap. Full QR/LAN/native Android/Rust desktop
goal stays active, all 36 Blueprint rows `[ ]`. No agents, commit or push.

## Changes

PTY Screen now retains soft-wrap metadata aligned with its bounded rows. Local
content output unchanged. Remote-only content joins parser soft wraps before
existing redaction, includes retained prefix if sampled window begins inside
wrap and withholds continuations whose prefix has left the 4000-row scrollback
until a hard newline. Erase/truncate/cursor tests maintain metadata alignment.
TUI remote_terminal uses this logical view; root/session/subtab gating unchanged.
Terminal markers now include private_key/secret_key/private key; visible PEM
private-key begin/end blocks withhold their body. Wire cap remains 16KiB.

Residual risks explicitly remain: arbitrary program hard-newline obfuscation,
unknown unlabelled secrets and PEM whose begin marker is outside sampled window.
This is not a complete terminal confidentiality guarantee or remote PTY input.

## Verification

Observed 2026-10-02T00:43:54Z; dirty worktree preserved.

- Standard `./gradlew :app:assembleDebug --offline --no-daemon` in android:
  **failed before compilation**, wrapper FileNotFoundException opening
  /Users/mac/.gradle/wrapper/dists/gradle-8.11.1-bin/bpt9gzteqjrbo1mjrsomdt32c/
  gradle-8.11.1-bin.zip.lck, Operation not permitted. No new APK gate claimed.
- `env ZENPI_HOME=/private/tmp/zenpi-terminal-wrap-20261002 cargo test
  --offline --locked --lib pty_shell::tests`: **10 pass**, including 3 new
  soft-wrap/prefix-eviction/erase tests and actual shell command. Final terminal
  marker changes subsequently compiled by integration/focused library below.
- Same Cargo env with remote_terminal/remote_subscription_tls/remote_parity/
  tui_project_workspace selections: **46 pass** (5/9/4/28). Includes actual PTY
  emitting sensitive marker split at parser column boundary, logical redaction,
  unchanged local content and normal visible output. Initial test failed because
  its echoed shell command deliberately split api_/key= across quoted arguments,
  an independent arbitrary obfuscation not soft-wrap; fixture corrected to keep
  the echoed assignment marker intact while output still splits api/newline/_key
  at column512. Final test retains exact local split assertion; no production
  bypass introduced to hide the failed fixture.
- Focused library existing_terminal_projects_only_matching_live_session_and_root:
  **1 pass**, verifies real TUI-owned PTY and exact projection scope.
- cargo check offline locked, cargo fmt check, git diff check pass. 19 existing
  library/one vendor warnings; library tests eight existing warnings. Removed
  one test-only unused variable exposed during intermediate build.
- 44 Stage2 Python tests and strict Blueprint validation pass. Android source
  unchanged; prior110 JVM result not counted as a fresh APK/device result.
  All observed command handles completed. Historical receipts unchanged.

SHA-256:

```text
src/pty_shell.rs e113a08526645fb06bb5ee1cf1fb3426284910f7e3865018a1f11626ad22fdba
src/tui.rs df7bb89287f3061869fb3fd193a83afdc58042e7cae3c30db73f60e18c304e6c
src/remote/terminal.rs 52cc1ae4f2d7e12fce19c5cd04d705e3ee0d0b2b2d513090e3b9dbc1ee58c89b
tests/remote_terminal.rs c85ad4ca632f7acc3fbb0784885a90642a3c10835b93d0552a38bb9fbb79e98a
```

Not Master acceptance. Fresh APK/device pairing/subscription/reset/cache races,
physical camera QR/same-WiFi LAN, real socket deadline/shutdown, full stream-delta
backpressure, executable independent-worktree scheduling1..64, interactive PTY/
bulk, API31/34/35 Keystore/lifecycle, migration/uninstall and signed release remain
required. Redaction progress does not replace the requested communication system.

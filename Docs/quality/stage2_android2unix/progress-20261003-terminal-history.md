# Retained Terminal History Paging

Provisional S2-019/S2-036 progress on 2026-10-03. Previous turn made real
PTY input/resize progress; this turn implements retained history paging across
the Rust owner and Android reader. Dirty changes preserved. No nested agents,
new worker admission, checklist promotion, commit/push or cleanup.

## Implemented Behavior

TerminalRead is typed Read-only and admitted through the existing owner mailbox.
The TUI routes to its matching existing PTY; shared/headless without a shell
refuses. Current grant, project/session, active subtab/revision, instance and cwd
are checked before reading. Reads do not spawn shells, pump input, mutate
journals/checkpoints or advance transport/ACK cursors.

Up to 4000 retained parser lines are soft-wrap-reassembled and sanitized before
hashing and slicing. Raw/redacted document limit 8 MiB, UTF-8 page limit 8192
bytes. Exact scope and full redacted SHA-256 bind continuation. Output change
returns RevisionConflict instead of mixing different screen states. This is
retained screen history, not an unbounded command log or frozen server copy.

Android adds explicit history open/previous/next/refresh/close, selectable
monospace text, transient current page and at most seven prior pages. Page
errors retain the verified page, disable continuation and allow explicit
refresh. Normal output updates retain selection; scope/instance/authority,
background/network and pending-write fences clear it. No history is cached.

## Verification And Corrections

Rust commands use ZENPI_HOME=/private/tmp/zenpi-resource-reload-test-home.
Successful terminal checks:

- rtk cargo check --offline --locked: zero errors, 20 existing warnings.
- rtk cargo test --offline --locked --test remote_workspace terminal_history
  -- --test-threads=1: final one passed. Real PTY runs a bounded local shell
  fixture, reconstructs over two pages with first/last line assertions, omits
  marked secret and project root, verifies digest and no journal/checkpoint
  mutation. Output changes, missing/foreign instance, malformed offset/digest,
  extra args, revision/session/project, write-owner fields and revoke refuse.
- rtk cargo test --offline --locked --test remote_workspace
  -- --test-threads=1: final source all 64 passed, 20.96 seconds, after the
  fixture newline correction.
- rtk cargo test --offline --locked --lib existing_terminal_projects_only
  -- --test-threads=1: one passed, including production TUI TerminalRead routing.
- rtk cargo test --offline --locked --test remote_terminal --test remote_protocol
  --test remote_auth --test remote_subscription_tls -- --test-threads=1:
  71 passed across four suites. Existing authenticated in-memory TLS regression,
  not a TCP/device history acceptance result.
- zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-terminal-pages-final2-20261003: production/unit/instrumented
  sources compile; all 284 JVM tests pass. New cases cover Read-only request,
  contiguous UTF-8, exact identity/numbers/digest/caps, navigation substitution,
  normal output retention, instance/selection and authority clearing. Narrow
  Compose history navigation/error-refresh/offline test compiled, not executed.
- cargo fmt --all -- --check; git diff --check: pass.
- python3 tools/validate_stage2_android2unix_blueprint.py --strict: pass,
  all 36 unfinished. Structure is not completion evidence.

Earlier checks failed on a Kotlin test parenthesis and assuming a checkpoint
file must exist before any write. Both corrected. More importantly, the first
PTY fixture exposed a real residual redaction limitation: interactive zsh hard
wrap/redraw can separate a marker-bearing command prefix from quoted secret
text. Marker matching cannot reliably identify that unlabelled continuation.
The output-only fixture now runs a script from disk, so it tests actual marked
output rather than confusing terminal echo with generated output. This does
not fix or claim coverage of the echo limitation. Protocol and user guide
explicitly document hard-wrap/cursor rewrite and unlabelled-secret risk and
restrict history to trusted devices. The fixture's escaped newline was then
corrected so its 600 physical lines genuinely exercise multi-line paging.

No TCP/ADB/Gradle bypass, fresh APK, connected-device or physical LAN claim.
All observed command sessions are terminal.
Full terminal emulator fidelity, complete session/settings parity, current
production provider/tool E2E, QR camera/direct LAN, lifecycle matrix and signed
APK/release acceptance remain incomplete or unverified. Goal stays active.

## Digests

```text
src/remote/terminal.rs fbbb355a04c460490a8d6cbebe2683cb3419741747f726257434bc80a7fe1a0f
src/tui.rs a456bbb32e275e3d094629f20aff141ff9d33ace7b73025dca325cf6fbcd4b8c
src/remote/protocol.rs b5f463a1b0a0e25ab542fb3aa2cca57bd20948ebc65adc02474ddca7d1ba6cec
src/remote/host_adapter.rs a94b3c02eb265d7eb653b0c5f49462c5bd597a74bec8480a9a67a1e739b14264
TerminalPageCodec.kt 6bd75d61360d9fc363ba0f0eb10f2f7c251ba26f2a803efc23f6d6ad39f0dbc7
PairingViewModel.kt edcbc235a839ef8d1596b23302659e3b3a9bd141e7779fc5affad4833664399d
MainActivity.kt 01a60bb055b7d6b8cf645e059ddd2eb3e6cf68e8b90e2c8c0f348003d8695f2a
tests/remote_workspace.rs 8a8ece41ac3af15d0b0bd5bf2f35ecf1a1a69ad64366f0e8ef0f43a53b8cc241
main.jar 3c54305de7a1ebe8e2b5c00c6a5222dee12b1087caec31dd9744ba3e5f87943d
unit.jar d8f1c9d687544f74938ab97a16a3abf04156a5614662610614506e147c96dca3
instrumented.jar 500bd0bd23a99c9a91e1256eec2e3d4ccac82d053fc6138d09f781f7261b0726
```

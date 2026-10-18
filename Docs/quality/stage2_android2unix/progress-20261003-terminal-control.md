# Existing Desktop PTY Control

Provisional S2-019/S2-036 progress on 2026-10-03. Previous turn made actual
ordinary-tool-page implementation progress. This turn implements the missing
input/resize path rather than declaring the read-only terminal complete.
Dirty user changes preserved; no nested agents, new worker admission,
checklist promotion, commit/push, or cleanup.

## Implementation

- TerminalInput/TerminalResize are typed Control writes routed through the
  existing TUI owner mailbox to its already-open matching PTY. Neither reads
  nor remote writes create a shell; headless can resolve old receipts only.
- Every PTY has a random 64-hex instance identity. Admission checks current
  device grant, project/session, owner epoch, workspace revision, active subtab
  ID/revision, instance ID and cwd. TUI modal/pairing/admission gates apply.
- Input is nonempty UTF-8, at most 4096 bytes, without NUL; dimensions 1..512.
  Remote input admission refuses growth beyond 16384 queued bytes. Checked
  resize persists across unchanged desktop redraws until local geometry changes.
- Checkpoint fingerprint binding and reserved journal receipt precede effects.
  Exact retry returns the durable receipt after restart without resending;
  conflict fails, reserved uncertainty remains UnknownOutcome. Journal receipts
  contain no input text. Success means input admission, not command completion.
- TUI listing exposes separate terminal_control and terminal_id. Android adds
  text with optional Enter, special-key icons and row/column resize, using the
  existing encrypted pending-write flow. Fresh controls require foreground and
  connected state. Authority/instance/text are excluded from display cache.
- Direct input has the desktop user's shell privileges, bypassing model-tool
  approval. User guide explicitly limits Control to trusted devices.

## Verification

Rust commands use ZENPI_HOME=/private/tmp/zenpi-resource-reload-test-home.
Completed successful checks:

- rtk cargo check --offline --locked: zero errors, 20 existing warnings.
  Initial check found getrandom error conversion and overlapping state borrows;
  corrected before test verification.
- rtk cargo test --offline --locked --test remote_workspace terminal_control
  -- --test-threads=1: one passed, rerun after adding reserved-outcome restart.
  Uses real PTY/file effects and stty dimensions, not echo or mocked acceptance.
  Checks Read-only denial, Busy, no duplicate execution on retry, exact byte
  receipt, journal input omission, instance/revision/bounds/NUL refusal, restart
  success/unknown replay without shell and revoked-device denial.
- rtk cargo test --offline --locked --test remote_workspace
  -- --test-threads=1: final 63 passed.
- rtk cargo test --offline --locked --test remote_workspace --test remote_terminal
  --test remote_protocol --test remote_auth -- --test-threads=1: 94 passed.
  This combined run preceded the added reserved-outcome assertion; final
  workspace and protocol checks were rerun separately.
- rtk cargo test --offline --locked --test remote_protocol
  -- --test-threads=1: 14 passed with both new write operations in owner/CAS test.
- rtk cargo test --offline --locked --lib local_remote_preview_tests
  -- --test-threads=1: two passed, including production TUI listing capability,
  actual remote dispatch into owned PTY and exact retry without another effect.
- rtk cargo test --offline --locked --lib pty_shell::tests
  -- --test-threads=1: ten passed.
- rtk cargo test --offline --locked --test remote_subscription_tls
  --test remote_parity -- --test-threads=1: 45 passed. Existing authenticated
  in-memory TLS regression, not current network/device terminal-control proof.
- rtk cargo test --offline --locked --test remote_headless_commands
  -- --test-threads=1 --skip headless_runner_executes_real_http_tool_turn_and_retries_never_resubmit
  --skip production_plain_host_routes_authenticated_tls_prompt_to_the_provider:
  nine passed, two explicitly filtered.
- zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-terminal-control-final3-20261003: production/unit/instrumented
  Kotlin sources compiled; 281 JVM tests passed. Three new control cases plus
  projection checks cover exact saved requests, bytes, receipt identity/numeric
  shape, scope, limits, foreground/offline and cache authority. Narrow 320 dp
  Compose input/interrupt/resize interaction test compiled only, not executed.
  Earlier runs found missing fixture control and wrong first-tab kind; corrected
  and all checks rerun through terminal success.
- cargo fmt --all -- --check; git diff --check: pass.
- python3 tools/validate_stage2_android2unix_blueprint.py --strict: pass,
  all 36 items unfinished. Structural validation is not product acceptance.

The unfiltered headless regression command exited 101: two tests fail at
TcpListener::bind with Operation not permitted (remote_headless_commands.rs
lines 210 and 1386), before provider/network execution. One remote_headless
test and nine other command tests passed; later suites in that invocation were
not reached. They are not represented as a full pass. No socket restriction
bypass, APK packaging, connected-device run or physical LAN claim was attempted.

## Digests

```text
src/remote/terminal.rs 1384206d37d1b3e30151969f981188ceeb59ae3f682dc2be33214765fb6197d7
src/pty_shell.rs 86c934a665810bafbee61a73a7513e3d808ec8e42f56609d37e98b30fe706848
src/tui.rs 48f9cbe1a018c22f8b353558cf874316f51324acf23592a99e9acab20cc581a0
src/remote/protocol.rs 6fc4899d709e70581aa952ec9f51e480cefc29848575296c07fc11413dcf7b41
src/project_workspace.rs ebdc8dcefcdf089599b07631bc590657d2b62a003f55b04927202511f05a2398
src/remote/host_adapter.rs 627d7e971aad08ecc29f84e14fe9e5acf7da13a02e3d6d14c22c80101ae3aab9
tests/remote_workspace.rs 073609e16b7ebbab638e0771da24cd24996e292f1136e0b85c3c87559f8f7cae
TerminalCommandCodec.kt 48a6cc4b585d8633b37af310601cf1b447d333d9f4e39a8c2a24fc1602e5cc30
MainActivity.kt 262a9b2ce5f82aaa7c1a48d361b5699750552ba7705a85489b4bd745a1de9bed
main.jar dbcb6d2fa6ff52669fbc199cb8544e27416840c1fe0b632a74054496eb3ac93d
unit.jar fbd39e3f8f754345dda161f6352b467fccfb256b88e277ab24f435ec4b9ffb27
instrumented.jar 94b22dd64bb6cb47ab68abf1e2195caa67cbdffa7272be16636d43ceb2035456
```

Full goal remains active: terminal emulator fidelity, bulk scrollback, complete
session/settings parity, current production provider/tool E2E, physical QR/LAN,
connected lifecycle matrix and signed APK/release gates are incomplete or
unverified. Typed input/resize is real progress, not full S2-036 acceptance.

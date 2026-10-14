# Android Prompt Through The Existing Desktop Runner

The previous goal turn made actual revoke-process recovery progress. This turn
advances S2-019/036/051 with a production Android Message/Send control and a
typed remote prompt admission path on the desktop owner mailbox. The request
uses the existing desktop BackgroundRunner and project Agent; no network Agent
or second journal writer is created. StartIfIdle avoids silently steering a
different running turn. The project/session/workspace owner/revision and current
Read/Control ACL are checked before admission. Main-tab execution is supported;
selected non-main roots are explicitly unavailable until cwd ownership is fixed.
That restriction is not a substitute for the full worktree parity requirement.

The sole Agent journal writer saves an exact-envelope fingerprint reservation
before runner submission and an accepted/failed admission receipt afterward.
Exact retries do not resubmit, including restored journals. An interrupted
reservation or unavailable execution lock remains unknown_outcome. Accepted
means admitted to the runner, not successful provider execution. Android checks
the accepted receipt's IDs and keeps its encrypted pending request until the
following projection read succeeds. Explicit retry retains the original ID and
text; background workers do not flush it. Offline display strips prompt_control.

## Verification

`desktop-android-prompt-retry-20261002.json` passed the real desktop PTY / API 36
emulator private-interface TLS run without adb reverse. It independently decodes
terminal QR pixels, clicks the production Send control, requires the remote
answer in the same session, checks one reserved/accepted journal pair and exactly
four desktop provider calls (one remote and three local/background updates).
Nineteen initial instrumentation tests, controlled process-death replay/revoke
phases, the new-process revoke recovery test and four cache/intent fixtures pass.
The screenshot was inspected and shows the Message/Send control without overlap.

Run artifact hashes (checked unchanged across the run):

- Rust: `7f581031918496b5768b05dfb6427b5be2dd3622b507d412c62ccdb9fd8fda57`
- APK: `aa606d4a4bafbea67dd8a0eb89ed7df89bad2c83215253a08ecc6cd120dc9157`
- Test APK: `122799efbf5cf8f1e8d452a4a3d7acf201e157f39fdd0a7a90d2c7f709c282ec`
- Harness: `1acdbfb2c7052c91e8b6e5c429d358e9f4f76c7d1a17ae3ec7f75f22e9ccad6f`

Afterward the Rust admission test gained Read-only denial, busy-owner unknown
outcome and interrupted-reservation refusal cases. Five Rust suites passed 35
tests; Cargo rebuilt the local binary to
`e7871c44a3a81b862ac686dac03e1b4fe53c64589bf0a372f0e143acab8999f1`.
The receipt describes its original run binary, not this later build. No runtime
code changed after that run. Gradle debug/release unit tests each passed 32
tests and both APK assemblies passed. Rust build had 20 existing warnings and
no errors. Seven harness tests, five blueprint tests, strict validation, Python
compilation, formatting and whitespace checks passed.

Earlier failing receipts are preserved. The first exposed repeated Compose
setContent in the fixture; the test now creates one composition and switches
state. The next had a directory-action error and failed WorkManager verification.
Provider answer selection was incorrectly inspecting the last input item,
which can be an assistant record; it now selects the last user item. The
directory failure's exact transient cause is not established. The full rerun
passed without weakening those assertions. Owned emulator PID 72320/session
80702 exited with radios enabled and no reverse mappings. Temporary production
desktop/provider processes are absent.

## Remaining Full Scope

This is initial remote prompt execution, not complete session-control parity.
Steer/approve/cancel, per-job terminal results, live provider events/ACK, request
ID collision accounting across projects and operation families, interrupted
admission operator recovery, supported peer-feature negotiation, non-main cwd,
actual scheduler quotas and Git transactions remain required. Physical camera/
LAN, API 31/34/35, release and execution-controller gates remain open. All 36
blueprint items remain unchecked; no worker admission, commit, push or full-goal
completion is claimed.

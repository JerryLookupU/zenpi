# Selected Subtab Execution Progress

Partial S2-019/S2-036 work, not whole-item Master acceptance. The authoritative
36-item Blueprint remains unfinished. No commit or push.

## Implementation

The desktop runner now captures a host-owned SubtabExecution at admission for
remote prompts and local discussion model/shell submissions. Queued local inputs
capture the selection when admitted to the runner. The same project Agent,
session, tool registry, approval coordinator/policy and output-read capability
remain in place. The request-scoped ToolContext root is restored on return,
error and unwind. No process-wide chdir and no second journal writer.

Root validation occurs before remote admission and again in the worker: main
equals its project root, in-place directories remain contained/canonical and
present, and Git worktree roots remain registered with their project. Missing
or symlink-replaced roots fail instead of falling back to main. Prompt admission
journals record the selected stable ID/kind/root as host-only evidence.
Worker gates and active raw captures cannot transfer to another root. Loaded
extensions have their own initialized workspace and are rejected cross-root
until reinitialization is implemented. This limitation is not full extension
parity. Headless routing and independent per-worktree sessions/concurrency remain
unfinished; the current host model retains one session owner per project.

## Behavioral Evidence

`desktop-android-selected-root-canonical-20261002.json` passed the full production
desktop/API-36 Android harness on direct TLS to 192.168.50.14, without adb reverse.
The real terminal QR was independently decoded from pixels. Android buttons
selected directory-one and directory-two, submitted prompts and read distinct
keep.txt content through production read_file execution. Independent validation
joins canonical host admission roots, journal tool turns, provider function-call
outputs and final assistant answers. Neither directory is deleted on tab close.
The harness made 15 controlled HTTP provider calls, not Echo responses.

All 24 initial instrumented tests passed, including 12 independently verified
directory-switch receipts, steering, approval, pending-write/read-failure and
WorkManager read-only resume. Subsequent actual Android process death, cached
cursor replay, pending-write recovery and revocation recovery passed, as did
four independent cache/revoke process tests. Physical phone LAN and camera
scanning are not verified. Stream delta push is still not implemented.

Artifact hashes:

- Rust: `b70bb54610ccbce3499d4a5fbb819a7709524f2821dd48010684f7adae39b1b2`
- App: `2bc9d5a5eb448c210524359e66893a8c0d6449a700029617cfac6a7a56de4728`
- Test APK: `63322d0d5f075356b4ca4fca6315c76d7764b3a6d4e4d64b460c086207bbaa47`
- Harness: `9ff2d523e264a6300f213fb05182b492aa00b0dffcd11d002ff74b2b89252762`

Rust parity tests exercise directory model writes, registered worktree writes,
captured selection surviving later UI changes, missing/symlink refusal,
return/error/panic restoration and explicit shell output in the selected root.
The combined ten-suite Rust core/tool/approval/input/remote regression passed
174 tests with four existing ignored tests; the parity suite has four tests.
Android Debug/Release unit suites each passed 44 tests. Python smoke-evidence
tests passed 12 tests; Blueprint tests passed five. Strict Blueprint and
controller validate-only passed, launching no workers. Cargo build and fmt
passed, with existing vendor/dead-code warnings retained.

## Failed Attempts And Diagnostics

All available failed JSON receipts are preserved. Initial UI attempts read the
first directory correctly but failed to switch after scrolling a long transcript.
A callback counter proved the touch never reached the tab. A captured screenshot
showed it clipped under the top bar even though Compose performScrollTo considered
the node reachable. Scrolling the outer content to Desktop before selecting a
tab made the callback and both directory reads succeed. Tests still use real
touch callbacks, not direct ViewModel calls or semantics-only clicks.

A speculative production refresh-phase change did not fix that root cause and
was removed. It is absent from the passed artifact. Two process-recovery attempts
failed unauthorized, one early approval attempt failed, and a pre-execution
selection encountered socket_failure. These are not claimed resolved by changing
the test scroll. The stable run passed all instrumented/process gates but its
independent directory check compared /var and /private/var paths literally;
normalizing the expected path to canonical form fixed that verifier mismatch.
The subsequent canonical run passed all checks. No failure is promoted to pass.

## Remaining Scope

Full headless/Android/TUI parity, loaded extensions, attachment/resource scope,
remote Git/worktree workflow, concurrency scheduler, PTY, bulk/event delta/ACK,
physical camera/LAN, supported API matrix and release gates remain required.
The owned emulator was shut down and its process/session exit verified.

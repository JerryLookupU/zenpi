# Extension Workspace Execution Progress

Partial S2-019/S2-036 integration, not whole-item Master acceptance. All 36
Blueprint items remain unfinished. No commit or push. Controller status showed
no claims, live transports, outstanding requests or handoffs; validate-only
launched nothing. Nested agents were not used.

## Implementation

The selected-subtab wrapper now supports loaded API-1 tools and API-2 tools/hooks.
It prepares the selected root and a fresh original-root restoration generation
from the loaded catalogue, without discovering new installations or parsing
changed manifests as an implicit grant. Both initialize/registration steps must
succeed before journaling the binding and revoking the original lease. Failed
preparation/cancellation leaves the original context, registry and lease live.
The original session owner, approval coordinator/policy and builtin handlers
remain in place. Extension handlers are registered with fresh scoped leases.

The selected runtime receives the canonical root through ZENPI_WORKSPACE for
initialize, tools and hooks. Close/start bracket generation changes. Return,
error and unwind revoke the selected generation and publish the prevalidated
original-root generation. Closing the Agent inside the scope is terminal and
never reopens it. Cleanup hook errors remain isolated extension errors.
The desktop runner's cancellation token now covers scoped initialization,
including child kill/reap and backend_cancelled status classification.

Native extensions remain trusted code with their existing executable/directory
semantics, not an OS filesystem sandbox or frozen executable-byte snapshot.
The catalogue freezes declarations/permissions, not on-disk program contents.
This does not accept every extension/attachment/resource workflow.

## Production Android Evidence

`desktop-android-extension-workspace-20261002.json` passed the full production
desktop/API-36 Android harness. Direct TLS used 192.168.50.14 with no adb reverse.
The terminal QR was independently decoded from pixels. Android's real buttons
selected two directories and sent prompts through the same desktop Agent and
controlled HTTP provider. A real native extension subprocess read each distinct
keep.txt file, and tool outputs reached the provider and final Android transcript.

The independent checker joins accepted subtab admission roots, journal tool
turns/provider outputs, selected/restored generation receipts and subprocess
lifecycle/root/content records. The adjacent `.extensions.json` preserves those
records. Both selected generations have exactly one initialize, session_start,
agent_start, before_tool, tools/call, after_tool, agent_end and session_close;
both restoration generations initialize/start/close at the original root.
No invitation token or capability secret is saved in this fixture log.

All 24 initial instrumented tests passed, plus actual Android force-stop and
cached-cursor/pending-write/revocation receipt recovery, and four independent
cache/revoke processes. The run made 15 controlled HTTP provider calls, not Echo.
The desktop listener closed and the owned emulator's shutdown/exit was verified.
Physical camera scanning and a physical LAN phone remain unverified.

Artifact hashes recorded by the passed harness:

- Rust: `4a5b56e4a74320f2fd92b04f2999985ab83fca504a411cb1ea10a9e793f065f1`
- App: `2bc9d5a5eb448c210524359e66893a8c0d6449a700029617cfac6a7a56de4728`
- Test APK: `63322d0d5f075356b4ca4fca6315c76d7764b3a6d4e4d64b460c086207bbaa47`
- Harness: `11e613db158cf33ff959144f9f36ac54d499f5d875663ee008445d2ab4352d70`
- Extension log: `14916b182e6fd136d4843f370f3264814e791d6fe84a77aeb1c6d2f62c9838c9`

## Focused Regression

The combined 12 Rust suites passed 201 tests with five existing/helper ignored
tests: stage1_extension_hooks, extensions, remote_parity, remote_workspace,
remote_headless, remote_subtab, remote_worktree, core_session, tools,
approval_owner, stage1_input_queue and stage1_input_host. Extension owner tests
use real subprocesses and controlled HTTP providers. New coverage checks frozen
catalogue/no implicit grants, both prepare-failure boundaries, cancellable child
reaping, error/unwind restoration, terminal close, original human approval
coordinator/policy across roots, legacy API-1 dispatch and rewritten path escape
refusal. The existing extension resume commit test separately passed.

Python smoke-evidence tests passed 13 cases, including negative mutations of
every fixture lifecycle/root record. Blueprint tests passed five cases; strict
Blueprint/controller validation, build, cargo fmt --check and git diff --check
passed. Existing vendor/dead-code warnings remain. Android APK sources were
unchanged this turn and their immutable hashes were checked during the harness.

An early test invocation compiled before the cancellation error-mapping edit
and failed its backend_cancelled assertion with tool_error. The final corrected
run passed. A combined command initially misspelled stage1_input_hosts; Cargo
refused the nonexistent target, then the exact stage1_input_host command passed.
Neither refused/failed invocation is counted as acceptance.

## Remaining Scope

Full headless/TUI/Android command parity, independent worktree sessions and
concurrency, remote Git workflow, resource/attachment scope, PTY, bulk output,
provider delta push/ACK, supported API lifecycle matrix, physical camera/LAN
and release/security review remain required. The goal and all checklist marks
stay active/unfinished; this receipt is not full Stage 2 completion.

# Stage 2 Active Prompt Metadata Admission

Recorded 2026-10-04T09:44:17Z; partial S2-036/S2-054 evidence, not acceptance.
Previous goal turn was progress: worker cancellation summary metadata repair.
Current dirty worktree preserved; no nested agents, reset, push, denied socket
retry or permission bypass. Blueprint remains 36 [ ], zero [_]/[x].

## Finding And Change

PromptStatusCodec.decode checked typed failure and bounded active live preview,
but active() checked only scope/unique target/accepted-running state. A retained
status with failure code on running, oversized/NUL live text or truncation
without text could therefore authorize a fresh steer/cancel even though it
could not be decoded from a valid host response.

The codec now uses one local retained-status predicate after decode and during
active target selection. It checks bounded identity, known execution/failure
codes, failure only on failed state and bounded NUL-free active stream with
consistent truncation. Existing command constructors and dispatch call active(),
so invalid metadata refuses before saving/transmission. Healthy accepted/
running previews, exact 4096-byte and empty truncated preview remain usable.
Exact saved pending envelopes remain explicitly reconcilable with original ID;
scanner/lifecycle/owner/CAS/permission fences remain unchanged. No protocol,
cache schema, host state machine or mutation retry change.

Two PromptControlPolicyTest regressions cover invalid retained state and healthy
active previews through target policy, construction, fresh dispatch and exact
saved reconciliation. AndroidRustPromptTest mutates actual running-owner status
and checks refusal before generating another request. The real surrounding
Agent/provider/tool/approval/steering/cancel/restart lane remains intact. These
client preflight mutations are not additional dispatched writes or physical
Android/LAN evidence.

## Verification

- Source checker /private/tmp/stage2-prompt-metadata-20261004: process 4887
  polled through exit 0; 588 ordinary JVM tests, main/unit/instrumented
  compilation and every explicit Rust/Kotlin lane passed.
- Actual prompt lane passed its original once-only tool effect, provider
  steering occurrence, cancellation, approval and fresh-Agent receipt recovery.
- Python Stage 2 tools: exit 0, 145 tests.
- Lifecycle validator: 25 fixtures pass, not connected API31/34/35 behavior.
- Blueprint fixtures: five pass; strict authority validator: exit 0.
- cargo fmt --check and git diff --check: exit 0.
- Host debug build and verify-only after Cargo lanes: exit 0, current artifact.
- Android verify-only: exit 1, missing APK provenance. No repeat of the denied
  Gradle daemon local TCP listener attempt and no stale APK installation.

```text
PromptStatusCodec.kt 530f426281aae75f4f1cf6610a914279b7cbe5fa29a13cebeb5751cd314b828d
PromptControlPolicyTest.kt 17ea2cb47765addc9390b1e1b67357b95a8f2353de58de46750060e83138e1c3
AndroidRustPromptTest.kt 87a95c72739e7edd8a104abfb56810bec696df0dacdf8d4d62f1a73296d86bd7
source-log a4d76a170a54eadef33e2e0577361d24bc6e97f8afce1b28ea522c021f302b6c
python-log bb5ce56baced3342a8f6524181528d75a8f4e679ebb1864bf2009e7423187ba1
prompt-results 10c44dbc875adf4a07e0f48ad8ed8ad69b4bd71735cd5d94c877981330f16930
main.jar 4157870f4fa0599df6f743bb74766a56d74e306f00a383b93c85c9e157b0ccfa
unit.jar 4a54ee3d4c4df5e1d73a798983a25b58f7702a7057c23432e2d614cb1753f6e0
instrumented.jar f95a635bb1b8d33d66febe3cf15afe0db9afd4ca9d63fb00a5f126e9d1e69630
host-receipt 39c8327e978df5201f336af41c673c142757bc4b1729657dedd8144e5c6c6f43
host-inputs 8ccf956a10a2fcac07b143c4d28aa8d62d1f59af1b5623a338da499e0b09906b
host-binary 40f9f134ed62fd51a71dc6e8d24022f0122dced68faf12f6a52f8b5ea145607f
```

Goal active/incomplete. Current APK attribution, physical camera QR/direct-LAN
bidirectional sync/recovery, connected API31/34/35 lifecycle, complete parity
and signed reproducible release remain required. This repair is concrete
product progress, not full completion or a blocked-status declaration.

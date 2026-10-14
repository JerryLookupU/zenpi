# Android Unknown-Outcome Write Handling

Provisional Stage 2 progress at 2026-10-02. The dirty worktree was preserved;
no nested agents, commit, push, checklist promotion, or cleanup was done.

The Android host client now distinguishes a typed `unknown_outcome` response
for the exact pending workspace command from a malformed or merely incomplete
receipt. It raises a dedicated `WorkspaceUnknownOutcomeException`, leaving the
original durable request pending for exact retry and marking the UI failure
code as `unknown_outcome`; it never treats the response as a confirmed success.
This preserves the Rust host's ambiguous-write contract across the QR-paired
client boundary.

Verification:

```text
/bin/zsh tools/stage2_android_source_check.sh /private/tmp/zenpi-unknown-outcome-check2
```

The source check exited 0, compiled cached API 35 production/unit/instrumented
artifacts, and ran 253 JVM tests successfully. Existing Kotlin warnings remain;
there were no errors. No emulator, camera, physical LAN, or signed APK claim
is made because the sandbox cannot start the adb server.

The Rust host side also passes `rtk cargo check --offline --locked` with zero
errors (20 existing warnings). The production host-pair test remains
unusable as network evidence in this sandbox because socket/temporary-file
operations are denied.

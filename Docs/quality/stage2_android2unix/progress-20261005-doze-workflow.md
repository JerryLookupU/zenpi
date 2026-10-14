# Camera And Sync With Actual Deep Idle

Previous goal turn: progress. API 31/34/35 camera/network/bidirectional sync
actually passed against the same artifacts. This turn adds system deep-idle
verification to that coherent production Activity workflow. The dirty canonical
checkout is preserved, all Blueprint items remain unfinished, goal active.

## Verification Scope

Optional --with-doze requires --bidirectional. The instrumented test uses real
ActivityScenario CREATED/RESUMED lifecycle transitions, not a simulated UI
offline flag. After the committed Android prompt, it cancels opportunity-based
WorkManager reads, confirms OFFLINE and missing workspace/prompt/worker
authority, disconnects the emulated battery, sleeps the display, forces deep
idle and requires exact dumpsys deviceidle get deep output IDLE. After 1.5
seconds it requires the identical durable encrypted paired-host state, offline
display and no pending write. A finally block always unforces idle, restores
battery state and wakes/unlocks the owned emulator.

On Activity resume, no Refresh or new QR is injected. It requires CONNECTED,
identical committed turns/session, same saved pin/device key, retired invitation
and restored live prompt authority. It asserts the answer remains visible before
capturing a Compose/accessibility-idle screenshot. The original three provider
requests, six appended durable turns, request binding and foreground replay
verifier remain mandatory; Doze does not replace the earlier sync phases.

The harness requires three additional exact ordered phases for this mode and
only then publishes deep_idle_cache_foreground_resume_verified. Doze without
bidirectional is rejected by both CLI and public-command construction. The
existing six camera-tool tests, including that invalid combination, pass.
No Android or Rust production code was changed in this turn.

## Actual API 35 Result

emulator-camera-api35-doze-20261005.json passed, exit zero. It completed the
camera/pinned TLS/explicit grant/live desktop/background replay/Android prompt
chain and all three actual deep-idle/cache/resume phases, using the emulator
host alias 10.0.2.2 with no adb reverse mapping. Exactly three provider requests
occurred; six unique turns and original journal prefix were verified. Screenshot
was visually inspected: readable committed background answer plus Android
prompt/answer, no overlap. The emulator, provider and desktop are terminal after
fixture cleanup. A separate API 34 run is being observed, not inferred passed.

API 34 doze run subsequently passed, exit zero, using the same attributed APKs
and host. It retains the explicit owned-debug-emulator Wi-Fi association setup,
not a physical-network claim. All original sync phases and deep-idle/cache/resume
phases completed. Its final transcript screenshot was visually inspected:
readable/nonblank, committed Android and desktop turns visible. A separate
API 31 invocation is being observed before this workflow matrix is reconciled.

## Retained Matrix Verification

API 31 doze run actually passed too, exit zero; screenshot visually inspected
in its landscape system-navigation layout. API 31/34/35 each complete one
exact named JUnit start/success pair and all 14 ordered camera/network/sync/
replay/deep-idle phases. Saved instrumentation and PNG hashes, current helper
hashes, original journal prefix, exact provider/turn/request binding and replay
were independently rechecked across all three receipts. They bind the same
app/test APKs, production Rust binary and helpers, not inferred runs from another
API. No adb reverse mapping is used. Physical/direct LAN and whole-item fields
remain false in all three; the old broad-suite failure remains unchanged.

Final checks: 182 Python Stage 2 tool tests pass; strict structural Blueprint
validation passes with 36 unfinished items; current APK --verify-only passes;
git diff --check passes. Full Rust gates and fresh Android unit variants were
not run in this continuation. Final ADB inventory is empty after cleanup; all
owned emulator/desktop/provider runs are terminal, no other AVD was touched.

App SHA-256: 2267dfddf3d2c2e00c3876b17cb55a5041fc5478cc03f0145e4aa1f6409986a5.
Test SHA-256: e303bfedd17d2c046a25e82cc2785d48fc069a66e8f50d3ab92287c54d844c50.
Input inventory: 01df51ee7855eeafb4a62f7cc6188936bea38a5bc6521e1856089c9f9b218aba.
Build receipt: b25f8bdfff8bbd20fb8997d5129548917e4361b401b5073a016b3c86af59023c.

## Still Unfinished

This bounded deep-idle fixture is not a guarantee of unlimited background
socket lifetime, arbitrary NAT traversal or physical-device behavior. Physical
phone/manual SAS/camera/direct host LAN and full lifecycle/Doze acceptance,
process death, notification/FGS decisions, broader desktop/workspace regression
repairs, full Rust/Android gates, signed release and per-item Master receipts
remain unfinished. The previously failed broad desktop suite is preserved and
is not silently replaced by this additional passing workflow.

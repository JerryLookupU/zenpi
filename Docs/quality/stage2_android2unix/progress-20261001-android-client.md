# Android Pairing Client Progress

Canonical Master progress, 2026-10-01. No Blueprint checkbox advanced. This
continues the full Android / QR LAN / Rust desktop synchronization objective,
which remains active and is not satisfied by pairing transport alone.

## Canonical Implementation

Native Kotlin/Compose `android/` project now exists, with a verified Gradle
8.11.1 wrapper distribution checksum, Java 17, minimum API 31 and target/compile
35. It has strict invitation parsing, QR camera scanner integration, human
safety-code confirmation, certificate-pinned TLS 1.3/ALPN, bounded frame codec,
connection cancellation and explicit offline/confirming/connecting/accepted/
error states. It executes no provider, shell, git or tool locally. There is no
persistent pairing credential/cache yet, and no registration/control claim.

The app accepts QR or a deep link only into the confirmation screen. It never
auto-connects from an incoming URI. Debug output redacts the token. Backups and
cleartext traffic are disabled. The UI requires manual six-digit desktop code
entry and displays the certificate fingerprint. Incoming JSON refuses unknown
fields, duplicate keys, malformed UTF-8, version overflow, wrong channel,
resource overflow and invalid request/outcome shapes. The protocol remains
independent of local headless JSONL.

Build/test commands ran under `/Users/mac/Github/zenpi/android`, using the local
Java/SDK, Maven Central and Google's Android Maven repository. Documentation
fetches from developer.android.com/GitHub were unreliable; the JourneyApps
published source JAR from Maven Central confirmed ScanOptions/CaptureManager
API and permission/lifecycle handling. No project from another repository was
copied; only installed toolchain/dependency versions were inspected.

## Verification And Evidence Boundaries

- `./gradlew :app:test :app:assembleDebug :app:assembleDebugAndroidTest` exited 0.
- Debug and release unit-test variants each ran 6 tests, with zero failures.
- A production Rust binary was launched by `tools/stage2_android_pair_smoke.py`.
  API 36 Android instrumentation used the actual Android TLS/socket codec to
  refuse a wrong certificate pin, accept a valid invite and refuse a duplicate.
- Two additional Compose instrumentation tests required human safety-code input
  and showed device registration pending / host control unavailable.
- The pinned-client and Compose run reported `OK (3 tests)`, exit 0. Receipt:
  `android-pair-smoke-20261001-final.json`. This receipt predates the final
  system-insets UI change; the inset build/unit tests subsequently passed and
  a new instrumentation receipt is required for that exact APK.
  A post-insets rerun was attempted after the successful build, but the selected
  emulator's tmux session/device disappeared before boot completed. That waiting
  harness was interrupted after the missing handle was confirmed; it launched
  no host and produced no success receipt. No post-insets visual pass is claimed.
- `git diff --check`, Python harness compilation and Blueprint strict validation
  passed. The complete Rust suite/release build and signed Android release were
  not run.

The harness uses `adb reverse` for one temporary localhost port. It never claims
LAN reachability, Android camera scanning, persistent device authorization or
session synchronization. Its receipts explicitly flag those as unverified.
It cleans its temporary host identity/process and only its own reverse mapping.
It installs APKs to the selected emulator but does not delete application data.
Instrumentation can fail with adb exit zero; success additionally requires the
expected test-runner completion summary.

An initial harness failure was caused by adb's remote shell interpreting the
invite's `&` characters. `shlex.join` now quotes the remote instrumentation
argv. The failed receipt `android-pair-smoke-20261001.json` is preserved and is
not acceptance evidence. Emulator processes twice disappeared while Gradle was
building; missing tmux/PID/device proved termination before any restart. No
cause was proved. Serialized build-then-headless-emulator testing completed the
successful interoperability runs; no retry-on-observation-timeout was used.

The computer-use inventory returned an unsupported API-key browser-auth error
and could not resolve the emulator executable as an app. Android UiAutomation
captured a confirmation screenshot and Compose assertions supplied behavior
evidence. The initial screenshot showed the test activity's system-bar overlap;
`WindowInsets.safeDrawing` is now explicit in the app screen/top bar. The exact
post-fix screenshot remains a separate visual check, not assumed passing from
the previous image.

## Remaining Required Scope

S2-030 schema fixture/generation and complete scaffold acceptance, S2-031 camera
roundtrip/permissions/lifecycle, S2-032 cancellation/fault/resource matrix and
Rust/Kotlin compatibility coverage remain open. Implement persistent device
key/Keystore/ACL/revocation and host authorization, then adapt the real Rust
owner/runtime, shared workspace/worktree state, concurrency, transcript,
approval/diff/PTY projection, cursor replay and snapshot replacement. Validate
actual LAN camera scanning and simultaneous Desktop/Android synchronization,
API 31/34/35 lifecycle, privacy/security/migration and packaging gates.

The execution controller/cron also remains unaccepted and inactive. No worker
concurrency, automated completion or background development is claimed. Existing
dirty-worktree edits have not been reverted. No commit/push was made.

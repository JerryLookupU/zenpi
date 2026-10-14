# Device Identity / Authorization Progress

Canonical Master progress on 2026-10-01. The prior turn was substantive progress.
Current authoritative worktree was re-read; no implementation was inferred from
the earlier status summaries. The overall Android / QR LAN / Rust Desktop sync
goal remains active and no Blueprint item is marked accepted by this note.

## Current Behavior

`src/remote/auth.rs` implements private, bounded host-fingerprint-bound device
storage, cross-process file locks, atomic writes, tombstone revocation, revision
fencing, explicit project ACLs and Read/Control/Approval checks. A P-256 signature
proves device key possession during pairing; request ID/name/token/pin/key are
all signed. No project ACL is granted by registration. A per-connection nonce
and one-use 30-second challenge enable registered-key authentication. Pairing
and authentication are connected to the real TLS host entry, not a test server.

The production host now has local device list/revoke options that do not start
a listener. Registration response requires durable registry persistence before
claiming success, includes device ID/revision and still says host control is
unavailable. Unauthorized proof and revoked device admission fail closed.
Permission grant API is local-only; no remote ACL grant operation exists.

Android generates a non-exportable P-256 signing key through Android Keystore,
signs the exact Rust JSON-array transcript, checks the returned device identity,
and encrypts small paired-host metadata with a separate Keystore AES-GCM key in
no-backup storage. No QR token/provider credential/private key enters metadata.
Local Forget device removes local signing/encryption keys and metadata; it does
not claim to revoke a host record. Challenge signing exists but Android reconnect
and UI host restore are not yet wired. Device registration is not synchronization.

## Observed Gates

| Command / scope | Outcome |
|---|---|
| `cargo test --locked --test remote_auth --test remote_tui_pairing --test remote_protocol --test remote_frame --test remote_listener --test remote_pairing` | 34 pass |
| `cargo test --locked --test remote_auth --test remote_tui_pairing --test headless_protocol` | 52 pass |
| final `cargo test --locked --test remote_tui_pairing --test remote_auth` | 7 pass; production TLS registered authentication + local CLI revoke refusal |
| `cargo check --locked`, `cargo fmt --check`, `git diff --check` | exit 0 |
| Android `./gradlew :app:test :app:assembleDebug :app:assembleDebugAndroidTest` | exit 0; debug/release each 6 unit tests pass |
| `tools/stage2_android_pair_smoke.py --serial emulator-5580` | API 36, real Rust binary, `OK (3 tests)`, exit 0 |

API 36 instrumentation checks wrong host certificate, device registration,
duplicate token rejection, Keystore private key `encoded == null`, stable key
reload, authenticated encrypted metadata save/load/wipe and Compose safety-code/
registered-but-no-control states. Receipt `android-device-smoke-20261001.json`
binds binary/APK checksums and output hashes. It uses adb reverse and explicitly
does not verify camera scan, LAN reachability or synchronized host state.

The Rust production process test registers a signed key, authenticates it on a
fresh TLS connection, invokes the local revoke CLI while the listener is live,
then confirms fresh authentication fails. Unit tests cover project/capability
matrix, stale revision refusal, nonce replay/cross-connection refusal, corrupt/
public/symlink/identity-rotation storage and proof field tampering.

Post-insets accepted-state screenshot was captured with Android UiAutomation
and visually inspected at 320x640: app title, host pin, registration/no-control
status and Forget/Cancel controls fit without the earlier system-bar overlap.
This is a Compose state fixture screenshot, not evidence of camera scanning.
Computer use from the previous turn remains unavailable for the emulator app;
no successful computer-use interaction is claimed.

All work ran in the canonical dirty repository; unrelated modifications were
preserved. No provider requests, worker agent, cron, commit or push were started.
The owned emulator was explicitly closed via its exact adb serial after tests.
No test host or reverse mapping is intended to remain. Existing host dead-code
and vendor parentheses warnings remain outside this scope.

## Remaining Gates / Next Frontier

Do not accept S2-013/037 from these narrow tests alone: permission grant and
remote revoke UX, encrypted metadata corruption/process death and API 31/34/35
matrix, admission/storage race and directory ownership hardening need review.
The schema gained signed pairing and connection challenge fields while still
unreleased; old unsigned clients fail closed. Shared schema fixtures/migration
acceptance remains outstanding.

Next implement authenticated reconnect and attach the existing Rust project/
session owner to the remote adapter, then shared workspace/worktree revisions,
command routing, transcript/approval/diff/PTY, durable event cursor replay and
snapshot replacement. Complete actual LAN camera scanning, simultaneous local
Desktop/Android synchronization, full fault/lifecycle/security/release gates
and execution-controller acceptance rather than reducing completion to pairing.

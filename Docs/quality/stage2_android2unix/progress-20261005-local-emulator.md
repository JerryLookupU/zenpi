# Local Android Emulator Resume

The operator restarted with unrestricted local execution. The previous
permission blocker is resolved: ADB daemon starts, Gradle builds and production
Rust TLS listeners bind. The goal is active and incomplete; no Blueprint mark
was changed. Only the dedicated ZenpiStage2Api35 AVD was started, at
emulator-5584; RateVC AVDs were not touched.

## Actual Builds And Repairs

The first repaired pinned-wrapper debug/instrumentation APK build and provenance
verification pass. App SHA-256:
e772573d5d3eae0484634f60e760a1d4a5ae049227c1100157ca97aa83953d27.
Test APK SHA-256:
51bfa628beb71803279ad422bcb2e36f4acc9d7fa2ce5be42dc99f33b394469f.
Input digest:
a0c9d0941c70b4d72875e7e41bf31c0117b955997be66d9c807515d01eb8ac93.

Real Gradle tests exposed two exception-identity assertions that only worked
without coroutine debug stack recovery. Local inspection of the installed
coroutines 1.8.1 bytecode confirms recoverStackTrace calls tryCopyException.
Tests now require the original exception type/message and original cause
identity, not identity of an optional recovered wrapper. Recovery was not
disabled and production timeout/cancellation behavior was not weakened.
Debug and release unit variants each collected 610 tests: 590 passed, 20
skipped, zero failures/errors.

Connected tests exposed a production error-state bug: foreground identity
failure was fenced correctly, then overwritten with a generic network error.
PairingViewModel now retains its typed authority-loss error. The existing
wiped/mismatched-key connected regression passes.

ActivityScenario rejected lifecycle callbacks after launcher-intent shape
changed to external VIEW. The entry test now launches the production Activity
with the sanitized VIEW/BROWSABLE shape, allowing Scenario to track actual
recreation without restoring a QR URI or changing production intent handling.
Single Activity/ViewModel and consumed-intent recreation assertions pass.

An overlapping Gradle build/test attempt failed on a disappearing incremental
compile output. It was not accepted; the subsequent serialized actual APK
build succeeded. Future Gradle invocations must remain serialized.

## Connected Evidence

- scanner-api35-current-20261004.json is a retained failed first attempt.
  It identified the authority-error overwrite and stopped while the old
  ActivityScenario recreation fixture was waiting.
- scanner-api35-repaired-20261004.json passes all 29 named tests and its
  retained receipt verifier. Saved screenshot was visually inspected:
  nonblank production camera-denial screen with readable, non-overlapping text.
- android-api35-host-pair-current-20261004.json retains a failed harness
  result despite nine device tests passing: its expected count was stale at
  eight. The verifier now uses the shared named start/success identity gate for
  exactly nine tests, rejects summary-only/foreign/failed results and retains
  redacted output and completed test identities.
- android-api35-host-pair-repaired-20261005.json passes nine named tests:
  real Kotlin pinned TLS and P256 Keystore registration against the production
  Rust binary, ungranted project hidden, explicit Read grant, authorized
  provider transcript snapshot, reconnect and host revoke refusal. Its saved
  screenshot was visually inspected. Transport is explicitly adb_reverse;
  camera, physical LAN and live event synchronization remain false.
- host-release-live-20261004.json passes the real attributed release listener
  invite smoke. Network/TLS Rust targets passed 65 tests with three explicitly
  ignored production cases. Running those three explicitly gave two passes
  and one failure: production_attached_workspace_is_readable_after_pair_and_grant
  emitted no invite. The failed child discarded stderr; fixture preparation
  and startup diagnostics need investigation, not a pass or sandbox label.
- Stage 2 Python suite: 173 passed. Strict Blueprint structural validation
  and git diff --check passed; all 36 items remain unfinished.

## Network And Remaining Scope

The direct-private-interface desktop run
desktop-api35-current-20261004.json failed and remains unchanged. At startup,
Wi-Fi was enabled but not associated; only a simulated CELLULAR eth0 network
was connected. Pairing failed at LanRoutePolicy.bindStable with Network
unavailable, and the network lifecycle test had no LAN token. The 93-test
instrumentation observation then reached its bounded 300-second deadline.
This failed receipt currently lacks partial instrumentation output; the
controller/harness diagnostic retention should be repaired separately.

After that run became terminal, the dedicated AVD was connected to its existing
AndroidWifi virtual hotspot using cmd wifi connect-network. It now has wlan0
10.0.2.16/24 in addition to eth0 10.0.2.15/24. This does not create a non-default
route to the host's 192.168.50.14 interface. No default-route fallback was added
to the LAN binder, and no virtual transport is credited as a physical peer.

Remaining product acceptance includes desktop bidirectional live events,
current headless command/replay workflows, API 31/34/35 lifecycle matrix,
actual camera/direct physical LAN, full Rust gates, signed release
installation/upgrade and per-item Master receipts. Local emulator execution
is now available and is the next work path; lack of a physical phone is not a
reason to stop simulator validation.

## Headless Workflow Follow-Up

Actual headless runs were attempted, not deferred for lack of a phone:

- headless-api35-current-20261005.json failed before sending because a generic
  hasSetTextAction selector matched both the main message and worktree task
  composer. The main composer now has prompt-composer, used by all three
  ambiguous fixture selections. No control or assertion was removed.
- headless-api35-composer-repaired-20261005.json sent a real provider request
  and received a live stream. It then exposed a stale test assertion expecting
  live authority after backgrounding. The test now requires the exact cached
  projection, no workspace/model authority, no prompt/worker controls and no
  transient stream text while preserving the byte-identical durable cache.
- headless-api35-background-repaired-20261005.json passed the background and
  reattach phases, then caught an asynchronous publication race: transcript
  completion preceded the terminal prompt-status update. The bounded wait now
  requires both the committed answer and succeeded status before asserting.
- headless-api35-terminal-status-20261005.json advanced through those phases
  but failed with SSLHandshakeException / connection closed in concurrent
  authenticated reads. The listener's four-connection cap and simultaneous
  foreground subscription are a candidate cause, not yet established. No cap
  was enlarged and the failed workflow is not accepted.

The final attributed APKs are app 2267dfddf3d2c2e00c3876b17cb55a5041fc5478cc03f0145e4aa1f6409986a5
and test 48c676fd1a572b9ebd6bb00b8d3281fe4a49367362284cd637e061363bca9d6c.
Input digest: 465b7f3485d7f9460adb6da0af1ab4a5c7057bc39dc7a8ca63b0d3997b0347b7.
The earlier scanner/pairing passes bind their earlier artifacts and must not
be relabeled as final-artifact whole-workflow passes. Actual Gradle debug and
release unit variants were rerun after the composer UI change and passed.

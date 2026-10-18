# Android Release Packaging Gate

This file records the current release-packaging boundary for S2-071.

## Current Packaging Status

The current attributed debug app/test APKs are
`363f4ad92d946bff1033b54c238d5be4abd317335ce505aff3542a1598c334de` and
`128362dc54402e9f2342ac8a1eead342838d62c6f8c3f14c6752fa902d3639dc`.
The same-artifact API 31/34/35 desktop/headless matrix is verified in
`api-matrix-current-20261006.json`. These are debug emulator results, not signed
release or physical-LAN acceptance.

A fresh `./gradlew --offline --no-daemon --no-build-cache --rerun-tasks
:app:assembleRelease` succeeds without operator signing variables. The current
unsigned APK SHA-256 is
`9609d9cac4a99d133bd73a26533678e7c0bd1d055628f9816aef46b254b10507`.
No release key was created or substituted. Signing, installation, upgrade and
Android release reproducibility remain open. Historical unsigned hashes and
execution blockers below describe their earlier attempts only.

Fresh Rust release reproducibility is independently verified by
`host-release-reproducibility-fat-current-20261006.json`: two locked offline
fat-LTO builds in separate target directories produce the same
`03beda670d2f4568bd60257e0975dca8e1af126cf3fb085d95da9253eb3a8813` binary.

## Historical Development Artifacts (2026-10-05)

This summary supersedes older local-build status paragraphs below. The debug
application APK has SHA-256
`780f374c3967704173981550e72c34e67136ddc6c975689b68cebd35244706cb`;
the process-recovery/camera receipts used instrumentation APK SHA-256
`bbe5bd1e610bbd5135ad4cf8bbb974ce24fd165f37080a6c812df62e66f4ed22`.
API 35's complete 93-test initial control/Doze suite and subsequent real
process-death/revocation recovery now pass. A separate 39-test plain workflow
proves complete-cache event replay. The production camera/pinned-TLS workflow
also passes bidirectional/Doze validation on API 34 without ADB reverse, using
the emulator host alias. See [latest connected evidence](android-lifecycle.md).
These are debug emulator results, not signed release, physical LAN, release
reproducibility or upgrade acceptance. The operator signing/install/upgrade
gate remains open. After a headless dialog-button fixture repair, a fresh
attributed build emits the same app APK and instrumentation APK SHA-256
`425eff06d7957f2090c51610dd9afdc64c9e4615328fd1883b7b9a329a65ee2a`.
The actual API 35 non-TUI release-host workflow passes with that new test APK:
attachment/provider bytes, template expansion, model/reasoning selection,
stream background/foreground recovery and saved-request process death.
The earlier connected receipts remain evidence of their recorded artifacts,
not a current mixed-artifact API matrix.

## Signing Policy

The checked-in Android release build is deliberately **unsigned** when no
operator signing environment is present: no signing credential is stored in
this repository. `android/app/build.gradle.kts` sets the release variant
non-debuggable and keeps minification off for the current development preview.
For a publishable build, provide the keystore outside the repository:

```sh
export ZENPI_ANDROID_KEYSTORE=/secure/path/zenpi-release.jks
export ZENPI_ANDROID_KEY_ALIAS=zenpi
export ZENPI_ANDROID_STORE_PASSWORD='...'
export ZENPI_ANDROID_KEY_PASSWORD='...'
```

All four values are required for signing. With none, Gradle intentionally emits
an unsigned artifact; partial configuration is rejected before building.

Required commands:

```sh
cd android
./gradlew :app:assembleRelease
```

With the four signing variables set, the output is the signed
`app-release.apk`; only that signed artifact may be installed or used for an
upgrade smoke:

```sh
adb -s DEVICE_SERIAL install -r app/build/outputs/apk/release/app-release.apk
```

The unsigned artifact is suitable only for inspection or signing by the
operator. It must not be presented as an installable release and cannot be
used to prove an upgrade because Android rejects it (and a debug-signed build
cannot upgrade an installation signed by a different release key).

Current source backup validation passes. Earlier restricted-session builds
could not start Gradle's local daemon socket. That permission blocker is now
resolved and current debug/device-test APKs have been built on the local host;
the operator-signed release/install/upgrade gate remains open. No signed APK
or upgrade receipt is claimed.

The currently present `android/app/build/outputs/apk/release/app-release-unsigned.apk`
is an unsigned inspection artifact only. Its SHA-256 is recorded here for
provenance (`53f990e21f467a4d8832fd21737086fb278ae64c22783e36a8dae766f6a2eb72`);
`apksigner verify` correctly rejects it because it has no signing block. This
hash must not be used as a release or upgrade claim. A publishable APK must be
rebuilt from the current source with the operator keystore variables above,
then verified and installed on a device using the same release key.

## Signed Artifact Inspection

The read-only inspection entry point checks the exact operator-supplied
release certificate SHA-256, one signer, API 31..35 signature verification,
16 KiB native/4-byte ZIP alignment and merged manifest:

```sh
python3 tools/stage2_android_release_check.py \
  --apk android/app/build/outputs/apk/release/app-release.apk \
  --expected-signer-sha256 CERTIFICATE_SHA256 \
  --receipt /private/tmp/zenpi-release-inspection.json
```

Use the expected certificate digest from the operator's trusted release-key
record, not one copied from an arbitrary candidate APK. SDK build-tools 35.0.0
and cmdline-tools apkanalyzer are required. No keystore password is needed for
inspection and no install or signing occurs. The checker rejects debuggable/
test-only packages, unexpected permissions/exported components, missing backup
rule references, cleartext allowance and mismatched QR/launcher routes. Failed
inspection writes a failed receipt and never overwrites prior evidence.

The merged-manifest gate also rejects shared UIDs, instrumentation declarations,
duplicate/missing component identities and exported components that imitate an
allowlisted name using a different component type. The AndroidX dynamic receiver
permission, when present, must have exactly one matching signature-only
declaration/use; extra permission trees/groups, privileged/known-signer grants
and unexpected declaration attributes are refused.

The v2 inspector also decodes and checks the packaged backup-rule XML and its
manifest references, rejecting unexpected resource variants. This is static
packaged-policy evidence, not runtime backup/transfer behavior. It does not
prove source attribution, upgrade compatibility, installation, device runtime
or reproducibility; receipt fields keep those claims false. The historical unsigned APK is explicitly
rejected by the real apksigner before later inspections, not promoted to a
release. Fresh signed build and connected API/upgrade gates remain open.

## Current Device-Test APK Attribution

The existing debug/device-test APKs predate current session/terminal/source
changes. Device harnesses now fail closed unless
`python3 tools/stage2_android_build.py` has completed the actual pinned-wrapper
debug and instrumentation APK tasks against unchanged current source. A
source/APK SHA-256 provenance record is emitted only after success; source or
APK changes invalidate it. `--verify-only` performs this check without Gradle
or adb. No timestamp-only or manual stamp fallback is offered.

This guard does not sign release APKs or make a reproducibility claim. It
prevents old debug binaries from being silently credited to current source.
The builder and verify-only path additionally check RemoteSchema.kt against
the reviewed contract. A stale/malformed schema is refused before Gradle/device
work, even when a record's source hashes match; the build never auto-regenerates
edited source. Preflight failure clears the old build authority and does not
publish a Gradle attempt because no process ran.
The current sandbox attempt failed with exit 1 before packaging; no provenance
was published, and the historical APKs were not installed or overwritten.

The builder also retains each completed Gradle process's raw stdout/stderr in
unique, owner-only local files under `android/app/build/`, with byte counts and
SHA-256 digests in `stage2-device-build-attempt.json`. Timeouts retain captured
partial output with `timed_out: true` and a null exit code. These diagnostics
have `passed: false` even for exit zero: they are not APK provenance. Only the
separate successful source/APK record authorizes a device harness. Repeated
attempts update the diagnostic index but do not overwrite earlier log files.
Logs can contain local paths or environment/build errors; keep them local and
review/redact them before sharing. They are not copied into quality receipts.

## Local Build Resources And Execution Permissions

On 2026-10-04 the canonical host had Gradle 8.11.1, JDK 17, Android API 35,
build-tools 35.0.0 and a 6.4 GiB Gradle dependency cache available. A private
copy-on-write cache at /private/tmp/stage2-gradle-home-20261004 removed the
global wrapper-lock write failure without modifying the global cache. The
actual provenance-gated debug/instrumentation build then reached Gradle
startup but failed with exit 1 at DaemonTcpServerConnector.start/Net.bind0:
java.net.SocketException: Operation not permitted. No Kotlin compilation or
APK packaging task ran. This is an execution permission failure, not evidence
of insufficient host resources or a Kotlin source error.

The pinned --no-daemon build still starts a single-use daemon for its JVM
settings; it requires a permitted local TCP listener. No socket-policy bypass,
manual APK stamp or stale-artifact fallback is accepted. The builder now
distinguishes wrapper-lock permissions from daemon-listener permissions in
its diagnostic summary; raw hashed logs remain authoritative diagnostic
evidence. This classification never authorizes a retry or publishes provenance.

## Current Local Emulator Build

After the operator restarted with full local execution access, ADB started
successfully and ZenpiStage2Api35 booted as emulator-5584. The actual pinned
Gradle debug/instrumentation build now succeeds and verify-only accepts:

- app-debug.apk: 2267dfddf3d2c2e00c3876b17cb55a5041fc5478cc03f0145e4aa1f6409986a5
- app-debug-androidTest.apk: 48c676fd1a572b9ebd6bb00b8d3281fe4a49367362284cd637e061363bca9d6c
- build input digest: 465b7f3485d7f9460adb6da0af1ab4a5c7057bc39dc7a8ca63b0d3997b0347b7

Actual Gradle debug and release unit variants each collected 610 tests, with
590 passed, 20 skipped and zero failures/errors. Before the subsequent composer
selector/test repair, the API 35 scanner suite passed 29 named tests. Pinned TLS pairing, authorized snapshot display,
reconnect and host-revocation refusal passed through the real Rust binary and
Android Keystore using adb reverse. This is current simulator evidence, not
physical camera/LAN, signed packaging or the full API lifecycle matrix. The
final composer-test APK has not passed the complete headless workflow; its
latest retained failure is SSLHandshakeException during concurrent reads.
See [the local emulator report](progress-20261005-local-emulator.md).

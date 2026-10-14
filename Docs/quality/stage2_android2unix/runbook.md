# Stage 2 Operator Runbook

This is a development-preview runbook, not a release approval.

`zenpi host pair --help` (or `-h`) lists connection, grant/revoke and recovery
parameters without HOME/provider setup, identity creation or a listener.
Use the help flag alone after host pair; invalid combined flags remain errors.

1. Build the Rust desktop binary with `cargo build --locked`. For the device
   acceptance harnesses and host smoke use `python3 tools/stage2_host_build.py` to publish
   current-source host provenance, and `python3 tools/stage2_android_build.py`
   for both current-source APKs.
2. Attach a workspace and session, then run `zenpi host pair --desktop`.
3. Scan the displayed QR only from the intended Android device. Compare the
   six-digit safety code and certificate fingerprint before confirming.
4. Inspect the device ID with `zenpi host pair --list-devices` and grant the
   minimum project capability explicitly.
5. Verify the Android projection, then exercise reads before enabling control.
6. On route loss, wait for offline state and reconcile any pending write by its
   exact request ID. Never submit a replacement request with a new ID.
7. Revoke a lost device and verify the committed revoke receipt before wiping
   its local metadata.

Do not copy QR URIs, pairing tokens, private keys, or raw provider output into
logs or bug reports. A release run additionally requires the signed APK,
connected API 31/34/35 matrix, direct-LAN receipt, and security review.

Explicit bind/advertise addresses are checked against invitation address rules
before identity creation or listener startup. Link-local, multicast, broadcast,
unsafe IPv4-mapped addresses and scoped/flow-labelled IPv6 are refused, as are
advertise family/exact-bind mismatches. Wildcard bind and ephemeral port zero
remain valid; the listener supplies the actual port. Loopback is for local
fixtures, not proof that a phone can reach the desktop. These checks do not
prove an explicit address is assigned locally or reachable from the phone.

The host identity must be a private regular file owned by the current account
with exactly one hard link. A symlink, hard-linked alias, FIFO, directory,
publicly accessible or malformed identity is refused without replacement.
Back up by creating a separate private copy, never by linking the active key
file. Investigate a refusal before restoring or rotating identity; a new
identity changes the certificate pin and requires explicit phone re-pairing.

## Isolated Host Validation

Tests that open a second workspace owner prepare production configuration and
resources, even when the initial owner uses a controlled fixture backend. Use a
fresh writable temporary ZENPI_HOME for the host control tests so they neither
read nor modify the operator's ordinary config/extension state:

```sh
validation_home=$(mktemp -d /private/tmp/zenpi-control-validation.XXXXXX)
ZENPI_HOME="$validation_home" cargo test --offline --locked \
  --test remote_workspace --test remote_worktree --test remote_concurrency \
  --test remote_terminal --test remote_worktree_runner \
  --test remote_worker_owner --test remote_subtab
```

Keep the temporary root with a failed run for diagnosis; do not replace failed
assertions with skips or loosen production configuration validation. A pass
here establishes local owner/Git/PTY boundaries, not connected Android or LAN.
After Cargo tests, verify host provenance again: integration targets may rebuild
target/debug/zenpi. If its digest changed, run stage2_host_build.py before any
device harness. Never reuse a stale build receipt by editing its hashes.

The standalone host smoke also requires current-source host provenance before
launch and rechecks it after clean shutdown. Its default is the release binary:
build attribution with `python3 tools/stage2_host_build.py --profile release`
before `python3 tools/stage2_host_smoke.py --binary target/release/zenpi`.
Use --profile release --fresh on the attribution builder when a clean target
build is required; it publishes the exact fresh artifact and retains the
build target for v2 verification. This does not itself compare repeat builds.
The repeat-build checker defaults to one fresh temporary target and one fresh
repository target; a passing temporary-sibling receipt alone does not prove
the canonical release build location produces the same bytes.
After a successful repeat, run `python3 tools/stage2_host_reproducibility.py
--receipt EXISTING_PATH --verify-only --binary target/release/zenpi` to recheck
retained source/toolchain/log/artifact identity and equality with the attributed
published release. This read-only check does not rerun or rewrite evidence.
macOS release builds use build.rs to strip debug/local symbols at link time
with deterministic linking, retaining UUID and ad-hoc signing. This policy is
not applied to debug or non-macOS targets; do not hand-edit build receipts
after changing build.rs or replacing the binary.
For a debug preview, specify `--binary target/debug/zenpi` after the debug
attribution build. Missing/stale provenance refuses before identity/listener
startup; source, binary or receipt drift during smoke cannot produce a pass.
The recorded binary hash belongs to the preflight artifact, not a replacement
file found after the child exits. This check is not release reproducibility or
Android/LAN acceptance.

## Current Broad Gate Status

The 2026-10-04 broad audit compiled all 122 Cargo test executables and passed
nine local owner/Git/PTY/migration targets (146 primary tests). The complete
library execution did not pass: 423 passed, 26 failed and seven ignored.
Twenty-one failures explicitly record denied loopback fixture setup; five
extension startup/pipe deadline cases passed only in subsequent serial
focused runs. Keep both results: focused green does not erase the full gate's
parallel failure or prove production network behavior. See
[the current audit](progress-20261004-rust-wide-audit.md) for exact commands,
isolated environment roots, failure classification and log hashes.

A later exact-exclusion parallel audit (only the 21 previously denied socket
tests excluded) exposed an inherited-descriptor worker recovery lease problem.
That production lease repair and deterministic descriptor regressions pass,
as do five owner/workspace targets (115 tests). The post-repair parallel audit
still fails: 431 pass, one extension-resume interpreter-startup timeout, seven
ignored and 21 filtered. The extension test passes alone/serial, which does not
close the parallel gate. No ignored marker or timeout was changed. See
[the recovery audit](progress-20261004-worker-lease-drop.md).

The extension-resume fixture now follows the integration suite's already-built
native subprocess pattern instead of Python startup. Real process/pipe/lease
and 1000 ms hook boundaries remain, and the old setup retry is removed. Two
fresh default-parallel library runs pass 432 tests, with eight ignored helpers
and the same exact 21 denied-socket tests filtered. This narrows the previous
parallel fixture failure, not a full-library pass. A separate extension
integration run still fails 14 listener fixtures with Operation not permitted.
Current supported-host networking, APK and physical-LAN gates remain open.
See [the native fixture report](progress-20261004-native-resume-fixture.md).

The subsequent unsuppressed `cargo test --locked` audit failed at the library
target: 403 pass, 50 fail, eight ignored, with inherited configuration write
and loopback permission failures. An unsuppressed library diagnostic using
a fresh writable `ZENPI_HOME` reports 432 pass, 21 fail and eight ignored;
all remaining failures report socket `Operation not permitted`. No integration
target after the first failed library ran in that full invocation. This is
stronger failure classification, not full acceptance. See
[the build-status/full-gate audit](progress-20261004-build-status-type.md).

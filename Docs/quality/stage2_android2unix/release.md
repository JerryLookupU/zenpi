# Rust Host Release Boundary

## Latest Size And Host Gate (2026-10-05)

The default full Rust gate now passes its library stage (453 passed, eight
explicitly ignored) but initially stops at the existing Stage 1 release-size
test: the retained thin-LTO release is 9,338,448 bytes, exceeding 8 MiB. The
failure is preserved in `/private/tmp/zenpi-stage2-full-observed-20261005.log`;
that failed command is not credited as a pass.

A separate fat-LTO diagnostic build is 6,253,264 bytes and passes the unchanged
size/startup/RSS test. The release profile now uses `lto = "fat"`; no runtime
feature, size budget, timeout or TLS policy is removed or relaxed. The actual
attribution builder then rebuilds `target/release/zenpi` with the same byte
size, SHA-256
`03beda670d2f4568bd60257e0975dca8e1af126cf3fb085d95da9253eb3a8813`,
and input digest
`99f46752cac902591b520dc0a64d07ecb7ab8b117edbc4048ff8adf8456128f9`.
The canonical artifact also passes the unchanged size/startup/RSS test.

`host-release-fat-lan-interface-20261005.json` verifies the attributed release
CLI creates a bounded invite/listener on the actual Mac interface
`192.168.50.14` and exits zero. This is host-side production CLI evidence only,
not an Android connection or physical-camera/LAN gate. The later full
`cargo test --locked --no-fail-fast` exits zero. A separate native frame/host/
listener run, including ignored tests, passes 11 + 5 + 5 after repairing the
HTTP fixture's inherited nonblocking flag. Earlier extension startup failures
remain retained residual reliability observations. Fat-LTO
cross-location reproducibility and supported-host runtime remain separate gates;
earlier thin-LTO repeat receipts below cannot certify this profile change.

The Rust host release build uses the locked dependency set. A successful
single build is not proof of reproducibility:

```sh
cargo fmt --check
cargo check --locked
cargo build --release --locked
```

For device acceptance, `python3 tools/stage2_host_build.py --profile release`
publishes source/artifact attribution; `--profile release --verify-only`
checks it without rebuilding. Independent repeat builds and toolchain/input
comparison remain required before claiming reproducibility.

The standalone `stage2_host_smoke.py` now requires this attribution before
launching either the release or explicitly selected debug artifact, then
rechecks unchanged provenance after shutdown. A stale binary is refused before
opening an identity/listener; runtime artifact/source/receipt drift invalidates
the smoke result even if a valid invite was emitted and the child exited zero.

Before publishing, verify the binary hash, run the host smoke parser against
the exact artifact, and perform the production listener pairing gate in an
environment that permits TCP binds. The listener is opt-in; ordinary desktop
startup does not open a network socket.

This repository currently has source and offline host evidence but no accepted
physical-LAN receipt. Do not publish a release claim until the Android signed
APK and direct-LAN/QR gates are recorded alongside the binary hash.

## Android Artifact Inspection

`python3 tools/stage2_android_release_check.py --apk PATH
--expected-signer-sha256 SHA256 --receipt NEW_PATH` is read-only artifact
inspection, not a release builder or device installation. Its v2 receipt
requires a single expected signer, alignment, non-debuggable merged manifest,
bounded permissions/components and actual packaged backup/extraction XML.
The inspector binds manifest references to the named compiled resources,
resolves their APK paths, decodes them with the installed SDK tools and checks
all nine data domains are fully excluded on legacy/cloud/device-transfer paths.
Uninspected XML configurations are refused rather than silently using defaults.

`backup_rule_contents_verified=true` proves only those decoded packaged rule
contents for an unchanged artifact. It does not prove device backup/transfer
behavior, source attribution, upgrade, Keystore wipe or reproducible signing.
Those remain explicit separate gates. Existing resource-only fixtures are not
application builds and cannot authorize installation or release.

## Earlier Host Release Artifact

On 2026-10-04 the attribution builder completed the locked/offline release
build and verify-only accepted the current source inventory. Artifact:
target/release/zenpi, SHA-256
f32c9381024afb00099c519822a2b8a65c2c99cc5216a59e9dbcb124c35e6d6b.
Source inventory SHA-256:
8ccf956a10a2fcac07b143c4d28aa8d62d1f59af1b5623a338da499e0b09906b.
This supersedes stale-artifact availability, not release acceptance.

The exact release help path exited zero with HOME removed and a nonexistent
private ZENPI_HOME; that directory remained absent. Nine protocol/security/
event/TLS targets passed 133 primary tests, with one real-loopback frame test
ignored under its existing explicit prerequisite. Memory TLS tests do not
prove the production TCP listener or Android pinned TLS. The full-library
423-pass/26-fail audit remains unresolved. Independent clean repeat build,
supported-host runtime, current APK, physical camera/LAN and signing gates
still prevent a reproducible/product release claim. See
[the artifact report](progress-20261004-release-artifact.md).

## Fresh Local Repeat Builds

Run python3 tools/stage2_host_reproducibility.py --receipt NEW_PATH to build
twice in separate initially empty target directories. It preserves per-lane
logs/artifacts, binds unchanged source and verbose Rust/Cargo toolchain, strips
ambient build overrides, rejects wrong/symlinked artifacts and records failure
without retrying or running lane two after lane-one failure. Existing receipts
are never overwritten. This is same-host repeat evidence, not runtime or
cross-host reproducibility.

The default now places lane A under the temporary runtime parent and lane B
under the repository target directory, instead of comparing only two siblings.
The receipt records both parents and exact target paths. An explicit
--secondary-runtime-parent can select a different second build location.
The byte comparison remains exact, including Mach-O UUID and signatures;
two matching temporary siblings are insufficient for the default gate.

The earlier 2026-10-04 temporary-sibling comparison passed: both fresh artifacts have SHA-256
6b0535aad15e5dca888bb7e5ea1758abd0d6d2fa94030ab3e8f6f5919a01f529.
They differ from the earlier canonical cached artifact f32c9381...; do not
attribute clean-repeat evidence to that cached binary or overwrite its receipt.
See [the comparison report](progress-20261004-release-repeat.md).

## Earlier Canonical Fresh Publication

python3 tools/stage2_host_build.py --profile release --fresh builds in a new
empty retained target/stage2-fresh-host-* directory and atomically publishes
the exact executable to target/release/zenpi. Its v2 source receipt binds the
actual --target-dir command, retained fresh artifact and published artifact.
Verification refuses tampering with either artifact, unsafe fresh paths or a
changed source inventory. The legacy cached-build v1 format remains supported;
v2 is fresh-source attribution, not a repeat-build pass.

The earlier canonical fresh artifact hashes to
d32407fb556406fa662d1b2e4eb909bb1006ef7fd82854588ef32d6030cf34e2.
Two additional fresh /private/tmp builds still match each other at 6b0535aa...
but differ from this canonical artifact. Byte comparison shows 16 differing
bytes in LC_UUID and 32 in the embedded signature area. Output-path-sensitive
link metadata is a plausible cause, not established proof. No normalization,
signature stripping or weakened byte comparison was used to manufacture a
pass. Cross-output-path reproducibility remains open. See
[the fresh-publication report](progress-20261004-canonical-fresh.md).

The strengthened default cross-location comparison ran on 2026-10-04 and
correctly failed release_artifacts_differ after both fresh builds exited zero.
The exact artifacts still differ only at the 48 UUID/signature bytes, and both
pass strict ad-hoc signature verification. Diagnostic -reproducible and
-oso_prefix links did not repair the mismatch and were not integrated. See
[the cross-location report](progress-20261004-cross-location-repeat.md).

## Current Link-Time Release Policy

build.rs now applies -Wl,-S,-x,-reproducible only to the zenpi binary for a
macOS release profile. The release profile already requests symbol stripping;
moving that work into the deterministic link avoids the observed residual UUID
difference after stripping. Debug and non-macOS targets receive no Apple flags.
The exact linker-internal cause remains an inference, but the repair has actual
fresh cross-location evidence. LC_UUID and LC_CODE_SIGNATURE remain present;
no post-build normalization or signature removal is performed.

Two fresh independently located builds and the separately fresh canonical
publication match exactly at SHA-256:
e9cf51915b90f25ce86be549ac59d4151aff889ec5621554dfd34a38a602349e.
All three pass exact comparison and strict ad-hoc signature verification.
That historical release attribution input digest is
e8c86f1a8c78bd18a1fee402f7bfe6a2937ceb68aa10e6a2e9123eb24c6250d8;
debug attribution has also been refreshed. This is same-host/toolchain
cross-location reproducibility, not cross-host reproducibility or product
acceptance. Current APK, physical QR/LAN, connected Android, full-library
and operator signing gates remain open. See
[the repair report](progress-20261004-checkpoint-link-policy.md).

## Earlier Subscription Release Repeat

After the owner-race and per-frame subscription delivery repairs, two fresh
independent builds (temporary runtime and new repository target) match the
current attributed target/release/zenpi exactly at SHA-256:
0368bb32c96052364f93afed3436ab213aa1195441e637eb5ddaeba9c808ff95.
Both fresh artifacts pass strict ad-hoc signature verification. The successful
receipt has SHA-256
0579068b0a4d275080999cd0b497dabc139a271f6e8a9809e64cc4f1cecdb7b6,
and comparison input digest
b40cc9da99d3309bbab464d785a6f8e1029952295b4964aa11bd02293ba73a16.
An earlier attempt was correctly refused for verifier-source drift even though
its binaries matched; that failed receipt remains unchanged.

The comparison tool now supports retained-evidence verification:

```sh
python3 tools/stage2_host_reproducibility.py --receipt EXISTING_PATH \
  --verify-only --binary target/release/zenpi
```

This rechecks source/toolchain identity, exact commands, independent target
paths, retained logs/artifacts and optional current release attribution.
It does not rebuild, rewrite receipts or promote runtime/cross-host claims.
Current-source verification and publication equality passed for the successful
receipt. Same-host repeat evidence still cannot accept APK, physical QR/LAN,
connected lifecycle, full-library or supported-host gates. See
[the current report](progress-20261004-subscription-release-repeat.md).

## Earlier Worker Lease Release Repeat

After the worker lease repair and native extension-resume fixture change,
the attributed release and two fresh independently located builds match
exactly at SHA-256:
ba38f1b03712475e47cf22e9a63cd1977eaec46965fe10746eb1c78cd6453d47.
The comparison uses one temporary target and one new repository target.
Retained-evidence verification passed at the time of this receipt;
all three artifacts pass codesign --verify --strict. Comparison input digest:
583a853ec219faf13466e69be14de7731d4516c950c8f1e66424c628095dfbf9.
Receipt SHA-256:
6ca4ad1f566beb75e78734e94cab5cd9df94b41032ceeaff6ed54edc7e0858b7.

This refreshes same-host/toolchain cross-location reproducibility only.
It does not accept production TCP/LAN, APK packaging/signing/install,
connected Android lifecycle, cross-host reproducibility or full Rust gates.
See [the current validation report](progress-20261004-native-release-interop.md).

## Current Acceptance Audit Repeat

The worker-lease comparison above is now historical: build-verifier source
changes correctly invalidate its source inventory, even though executable bytes
are unchanged. A new comparison on 2026-10-04 built twice in initially empty,
independently located targets (temporary runtime and repository target). Both
builds exited zero and match the currently attributed published release exactly:
ba38f1b03712475e47cf22e9a63cd1977eaec46965fe10746eb1c78cd6453d47.

Current-source retained-evidence verification with --binary target/release/zenpi
passes. All three executables pass codesign --verify --strict. Comparison input
digest: 96ad81b583669ddc487434fe47f4fdb6f18b304b256d795702f5ba7a01087fe1.
Receipt: /private/tmp/stage2-current-release-compare-20261004-audit.json,
SHA-256: 9fb36e8fa259b37bdba376930d962e5f122b7d0939d566ff71ca5250a73d0eee.
This remains same-host/toolchain cross-location build evidence, not production
LAN, Android packaging, signing, installation or full release acceptance. See
[the acceptance audit](progress-20261004-acceptance-receipts.md).

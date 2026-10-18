# Stage 2 Production Host Smoke Attribution

Recorded 2026-10-04T08:50:49Z; partial S2-051/S2-070 evidence, not acceptance.
Canonical cwd /Users/mac/Github/zenpi, dirty baseline HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b. Existing changes preserved; no
nested agents, reset, push, socket-permission bypass or stale APK installation.
Blueprint SHA-256 d76e70e369f11f0fd3d1905d56d79c2a89401b92215da48ad85314f390d96d97;
spec SHA-256 403e6339b0318a420d93600fc52926acf6a9e46ec899bd9a9edb03c235190a28.
All 36 authoritative rows remain [ ]; zero [_]/[x]. Previous goal turn was
progress: known same-session event cursor preservation and verified source
interop. This turn repairs a distinct production-entry evidence gap.

## Finding And Change

Android acceptance harnesses already require current host source/artifact
provenance. Standalone stage2_host_smoke.py did not: it launched any supplied
executable and hashed whatever file existed afterward. The preceding smoke
defaulted to a stale release binary despite current debug attribution. Neither
that failed sandbox run nor a hypothetical green old-binary run can establish
current production behavior.

Standalone smoke now requires require_current_host before creating its
temporary identity/home or launching the child. It keeps the original path
identity rather than resolving away symlinks, supports only attributed
target/debug/zenpi or target/release/zenpi, and checks provenance again after
the bounded child exits. Missing/stale source, wrong artifact, corrupt/missing
receipt or symlink refuses preflight; source/binary/receipt change during the
run forces passed=false even after valid invitation and zero exit. Receipt
records the original attributed artifact hash and provenance, not a replacement
file's post-exit hash. Preflight refusal leaves existing evidence untouched.
This is source attribution, not atomic executable pinning against adversarial
ABA replacement or independently reproducible release proof.

The shared invite validator also rejects control bytes/oversize before URL
normalization, scoped IPv6, IPv4 broadcast and unusable IPv4-mapped IPv6 using
the underlying IPv4 address. Valid LAN, mapped LAN and loopback fixtures remain
accepted. Physical QR harness imports this same parser; its separate private
LAN/device/certificate requirements remain intact.

Tests publish real builder-shaped receipts in isolated fixture repositories
and launch bounded fixture child scripts. They exercise clean/nonzero exit,
EOF-before-exit, token/stderr redaction and credential-free child environment,
plus missing/stale/corrupt preflight and runtime source/binary/receipt changes,
receipt reformatting and binary/receipt deletion. Fixture children are not the
production Rust listener and cannot establish LAN/Android acceptance.

## Verification

- python3 -m unittest discover -s tools -p 'test_stage2_*.py': exit 0, 142 tests,
  including 13 HostInviteTests and shared physical QR parser fixtures.
  Final log /private/tmp/stage2-host-smoke-provenance-python-20261004.log.
- python3 tools/test_validate_stage2_android_lifecycle.py: exit 0, 25 validator
  fixtures, not connected lifecycle behavior.
- python3 tools/test_validate_stage2_android2unix_blueprint.py: exit 0, five tests.
- cargo fmt --check, git diff --check, strict Blueprint: exit 0; 36 [ ],
  zero [_]/[x]. No Rust/Kotlin production sources changed this turn, so the
  preceding source interop report is historical evidence, not a claimed rerun.
- python3 tools/stage2_host_smoke.py --receipt
  /private/tmp/stage2-host-smoke-provenance-preflight-20261004.json: exit 1,
  "Host source inventory is stale; rebuild before device validation".
  Default release artifact refused before any child, listener or identity.
  The requested receipt was not created, verified by test ! -e PATH, exit 0.
  No equivalent production socket retry followed.
- python3 tools/stage2_host_build.py --verify-only: exit 0, unchanged attributed
  current debug binary. The harness-only changes do not alter Rust inputs.
- python3 tools/stage2_android_build.py --verify-only: exit 1, missing APK
  provenance. No unchanged Gradle/ADB retry or bypass.

```text
stage2_host_smoke.py cc1dd2e360261b68c09e5263349e59db92fce3a45531b8c3b68f02c262ed97ac
test_stage2_host_smoke.py 6b74bc6865cf3a5a7fc5133e60e234f7adf4fa438eec96bd5501665db4dda6e4
python-log 7f3a98611714c731e7b97f224e2c3533ea197cfa58186d950ae2da2eae108f01
debug-host-receipt f33a4a57f981e8b4dfd88432fde91b96c28ec189d5f6a1c9ffd6c0804957cffa
debug-host-inputs 08312e22d68633b57228db66f7d64511bc87dc758579124ef9de588e3a466219
debug-host-binary 40f9f134ed62fd51a71dc6e8d24022f0122dced68faf12f6a52f8b5ea145607f
```

Runbook/release boundaries now specify profile-matched attribution before
standalone smoke. Goal remains active/incomplete: fresh APK provenance,
physical camera QR/direct-LAN bidirectional synchronization and recovery,
connected API31/34/35 lifecycle, full parity and signed reproducible release
are still required. Meaningful source/verification repair is available, so
the repeated environment limitation does not satisfy a blocked audit.

# Pairing IPv6 Parser Boundary

Provisional Stage 2 progress at 2026-10-02. Existing dirty changes were
preserved; no nested agents, commit, push, checklist promotion, or cleanup was
performed.

The Android QR invite parser now applies strict IPv6 literal validation to the
pairing URI itself, not only to the later socket routing path. Bracketed QR
addresses with invalid compression or zone identifiers are rejected before
safety-code confirmation, TLS, device registration, or encrypted metadata
storage. This keeps invite parsing and `LanSocketRouting` on the same literal
address policy.

Verification:

```text
/bin/zsh tools/stage2_android_source_check.sh /private/tmp/zenpi-pairing-invite-strict-ipv6
env PYTHONPATH=tools /usr/bin/python3 -m unittest tools.test_stage2_android_pair_smoke tools.test_stage2_desktop_android_smoke tools.test_stage2_headless_android_smoke tools.test_stage2_scanner_android_smoke tools.test_validate_stage2_android_lifecycle tools.test_validate_stage2_android2unix_blueprint tools.test_validate_stage2_android_backup
git diff --check
```

Cached Android source validation passed with 254 JVM tests and the Python set
passed 68 tests. Device, camera, physical LAN and Gradle-daemon execution
remain unverified in this sandbox.

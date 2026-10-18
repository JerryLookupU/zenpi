# QR Encoded Boundary Corpus

Provisional Stage 2 progress at 2026-10-02. Existing dirty changes were
preserved; no nested agents, commit, push, checklist promotion, or cleanup was
performed.

Android pairing tests now cover percent-encoded control characters and malformed
token suffixes in addition to malformed IPv6 and duplicate/origin cases. These
values are rejected before a safety-code confirmation can reach TLS, before a
device key is registered, and before encrypted host metadata is written.

Verification:

```text
/bin/zsh tools/stage2_android_source_check.sh /private/tmp/zenpi-qr-encoded-boundaries2
env PYTHONPATH=tools /usr/bin/python3 -m unittest tools.test_stage2_android_pair_smoke tools.test_stage2_desktop_android_smoke tools.test_stage2_headless_android_smoke tools.test_stage2_scanner_android_smoke tools.test_validate_stage2_android_lifecycle tools.test_validate_stage2_android2unix_blueprint tools.test_validate_stage2_android_backup
git diff --check
```

Cached Android source validation passed with 254 JVM tests and the Python set
passed 68 tests. Device, camera, physical LAN and Gradle-daemon execution
remain unverified in this sandbox.

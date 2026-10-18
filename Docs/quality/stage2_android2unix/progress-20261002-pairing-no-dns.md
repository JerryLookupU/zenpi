# Pairing Parser No-DNS Boundary

Provisional Stage 2 progress at 2026-10-02. Existing dirty changes were
preserved; no nested agents, commit, push, checklist promotion, or cleanup was
performed.

The Android QR parser now converts its final host-literal parse into a typed
pairing failure instead of allowing an unexpected `InetAddress` resolution
failure to escape as a generic exception. This keeps malformed or nonliteral
invite addresses fail-closed before TLS, registration, or local metadata writes,
and aligns parser behavior with `LanSocketRouting`.

Verification:

```text
/bin/zsh tools/stage2_android_source_check.sh /private/tmp/zenpi-pairing-parse-no-dns
env PYTHONPATH=tools /usr/bin/python3 -m unittest tools.test_stage2_android_pair_smoke tools.test_stage2_desktop_android_smoke tools.test_stage2_headless_android_smoke tools.test_stage2_scanner_android_smoke tools.test_validate_stage2_android_lifecycle tools.test_validate_stage2_android2unix_blueprint tools.test_validate_stage2_android_backup
git diff --check
```

Cached Android source validation passed with 254 JVM tests and the Python set
passed 68 tests. Device, camera, physical LAN and Gradle-daemon execution
remain unverified in this sandbox.

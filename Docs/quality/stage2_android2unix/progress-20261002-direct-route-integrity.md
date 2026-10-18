# Direct Route Integrity

Provisional Stage 2 progress at 2026-10-02. The dirty worktree was preserved;
no nested agents, commit, push, checklist promotion, or cleanup was performed.

The direct-LAN pairing harness now checks the reverse table both before the
host starts and after instrumentation completes. A run cannot be credited as
direct private-interface evidence if any late adb reverse mapping appears while
the QR/TLS tests are running. The receipt continues to distinguish direct
interface TLS from physical-phone LAN and camera evidence.

Network callback teardown remains idempotent when Android has already removed a
callback during a lifecycle transition.

Verification:

```text
python3 -m py_compile tools/stage2_android_pair_smoke.py
env PYTHONPATH=tools /usr/bin/python3 -m unittest tools.test_stage2_android_pair_smoke tools.test_stage2_desktop_android_smoke tools.test_stage2_headless_android_smoke tools.test_stage2_scanner_android_smoke tools.test_validate_stage2_android_lifecycle tools.test_validate_stage2_android2unix_blueprint tools.test_validate_stage2_android_backup
git diff --check
```

All 68 Python tests passed. Cached Android source validation previously passed
253 JVM tests. Device, camera, physical LAN, and Gradle-daemon execution remain
unverified in this sandbox.

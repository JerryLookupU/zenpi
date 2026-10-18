# Cancellable Exact-Receipt Polling

Provisional Stage 2 progress at 2026-10-02. Existing dirty changes were
preserved; no nested agents, commit, push, checklist promotion, or cleanup was
performed.

`HostClient` no longer uses a blocking `Thread.sleep` while polling a pending
steer receipt. The bounded retry delay now uses coroutine `delay`, so Android
route loss, Activity backgrounding, timeout and cancellation can close the
owned TLS socket immediately instead of waiting for the polling sleep. The
exact request ID and durable pending-envelope semantics are unchanged; this is
not an automatic write retry.

Verification:

```text
/bin/zsh tools/stage2_android_source_check.sh /private/tmp/zenpi-cancellable-receipt-poll
env PYTHONPATH=tools /usr/bin/python3 -m unittest tools.test_stage2_android_pair_smoke tools.test_stage2_desktop_android_smoke tools.test_stage2_headless_android_smoke tools.test_stage2_scanner_android_smoke tools.test_validate_stage2_android_lifecycle tools.test_validate_stage2_android2unix_blueprint tools.test_validate_stage2_android_backup
git diff --check
```

Android cached source validation passed with 253 JVM tests and the Python set
passed 68 tests. Device, camera, physical LAN and Gradle-daemon execution
remain unverified in this sandbox.

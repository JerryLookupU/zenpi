# Network Observation Publish Deduplication

Provisional Stage 2 progress at 2026-10-02. Existing dirty changes were
preserved; no nested agents, commit, push, checklist promotion, or cleanup was
performed.

`NetworkMonitor` now suppresses duplicate route-token publications across
callbacks in one monitor lifecycle. Forced initial publication still occurs on
each `start`, while repeated callback refreshes with the same published token
do not trigger duplicate ViewModel fencing or reconnect work. Route changes and
offline/null transitions still publish normally.

Verification:

```text
/bin/zsh tools/stage2_android_source_check.sh /private/tmp/zenpi-network-publish-dedupe
env PYTHONPATH=tools /usr/bin/python3 -m unittest tools.test_stage2_android_pair_smoke tools.test_stage2_desktop_android_smoke tools.test_stage2_headless_android_smoke tools.test_stage2_scanner_android_smoke tools.test_validate_stage2_android_lifecycle tools.test_validate_stage2_android2unix_blueprint tools.test_validate_stage2_android_backup
git diff --check
```

Cached Android source validation passed with 254 JVM tests and the Python set
passed 68 tests. Device, camera, physical LAN and Gradle-daemon execution
remain unverified in this sandbox.

# Literal Host Resolution Fail-Closed

Provisional Stage 2 progress at 2026-10-02. Existing dirty worktree changes
were preserved; no nested agents, commit, push, checklist promotion, or cleanup
was performed.

`LanSocketRouting.bind` now converts an unexpected literal parse failure into a
bounded `IOException` before any network selection or socket binding. Combined
with the strict literal parser and zone-scoped IPv6 refusal, QR-provided host
values cannot fall through to DNS resolution or a different network route.

Verification:

```text
/bin/zsh tools/stage2_android_source_check.sh /private/tmp/zenpi-literal-dns-failclosed
env PYTHONPATH=tools /usr/bin/python3 -m unittest tools.test_stage2_android_pair_smoke tools.test_stage2_desktop_android_smoke tools.test_stage2_headless_android_smoke tools.test_stage2_scanner_android_smoke tools.test_validate_stage2_android_lifecycle tools.test_validate_stage2_android2unix_blueprint tools.test_validate_stage2_android_backup
git diff --check
```

Cached Android source validation passed with 253 JVM tests and the Python set
passed 68 tests. Device, camera, physical LAN and Gradle-daemon execution
remain unverified in this sandbox.

# Strict IPv6 Literal Parser

Provisional Stage 2 progress at 2026-10-02. Existing dirty changes were
preserved; no nested agents, commit, push, checklist promotion, or cleanup was
performed.

The Android LAN literal guard now validates IPv6 syntax with `InetAddress` after
the character whitelist, still without accepting DNS fallback. Malformed values
such as `::::`, `1::2::3`, and `dead::beef::1` are rejected before route
selection. Valid IPv6 literals and IPv4-mapped literals remain supported, while
zone-scoped forms remain refused.

Verification:

```text
/bin/zsh tools/stage2_android_source_check.sh /private/tmp/zenpi-ipv6-strict-literal
env PYTHONPATH=tools /usr/bin/python3 -m unittest tools.test_stage2_android_pair_smoke tools.test_stage2_desktop_android_smoke tools.test_stage2_headless_android_smoke tools.test_stage2_scanner_android_smoke tools.test_validate_stage2_android_lifecycle tools.test_validate_stage2_android2unix_blueprint tools.test_validate_stage2_android_backup
git diff --check
```

Cached Android source validation passed with 253 JVM tests and the Python set
passed 68 tests. Device, camera, physical LAN and Gradle-daemon execution
remain unverified in this sandbox.

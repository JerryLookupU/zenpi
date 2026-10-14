# IPv6 LAN Literal Boundary

Provisional Stage 2 progress at 2026-10-02. Existing dirty changes were
preserved; no nested agents, commit, push, checklist promotion, or cleanup was
performed.

Android LAN routing now refuses zone-scoped IPv6 literals such as
`fe80::1%wlan0`. QR addresses are intentionally unscoped literals so the host
fingerprint/address pair remains stable across Android network handles. Link-
local scoped addresses require interface context that cannot safely survive QR
transport and are therefore rejected before DNS or socket binding.

Verification:

```text
/bin/zsh tools/stage2_android_source_check.sh /private/tmp/zenpi-ipv6-zone-check
env PYTHONPATH=tools /usr/bin/python3 -m unittest tools.test_stage2_android_pair_smoke tools.test_stage2_desktop_android_smoke tools.test_stage2_headless_android_smoke tools.test_stage2_scanner_android_smoke tools.test_validate_stage2_android_lifecycle tools.test_validate_stage2_android2unix_blueprint tools.test_validate_stage2_android_backup
git diff --check
```

Cached Android production/unit/instrumented compilation passed with 253 JVM
tests; the Python validation set passed 68 tests. Device, camera, physical LAN,
and Gradle-daemon execution remain unverified in this sandbox.

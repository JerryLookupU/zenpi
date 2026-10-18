# Direct LAN Validation Hardening

Provisional Stage 2 progress at 2026-10-02. No checklist item was promoted,
and the dirty worktree remains preserved.

The Android pairing smoke harness now validates the address policy explicitly:
only RFC1918 IPv4 and RFC4193 ULA IPv6 literals are accepted, IPv6 scope IDs
are rejected, and accepted IPv6 values are canonicalized before the QR
advertisement comparison. The direct probe selects `AF_INET` or `AF_INET6`
instead of assuming IPv4. The harness also expects all eight selected Android
instrumentation tests (one production pairing test plus seven Compose tests),
rather than the stale seven-test marker. Reverse mappings are removed only
after this run actually created one, so a failed setup cannot delete a mapping
owned by another invocation.

Validation evidence:

```text
PYTHONPATH=tools python3 -m unittest \
  tools.test_stage2_android_pair_smoke \
  tools.test_validate_stage2_android_lifecycle \
  tools.test_validate_stage2_android2unix_blueprint \
  tools.test_validate_stage2_android_backup
Ran 29 tests ... OK
python3 -m py_compile tools/stage2_android_pair_smoke.py tools/test_stage2_android_pair_smoke.py
git diff --check
```

An actual adb/LAN run remains unavailable in this sandbox because the adb
server cannot bind its local socket; this receipt makes no device or physical
LAN claim.

# Direct LAN Validation Surface

Provisional Stage 2 progress at 2026-10-02. The current dirty worktree and
user changes were preserved; no nested agents, commit, push, checklist
promotion, or cleanup was performed.

The production Android pairing harness now has an explicit direct-network
surface. `--lan-ip` is validated as a private literal, requires no reverse
mapping, verifies local interface ownership and invite-address identity, binds
the Rust host to that interface, and records direct-LAN evidence only after the
same instrumentation checks used by the QR/TLS path pass. The ordinary mode
continues to use adb reverse and records that transport separately.

Validation completed:

```text
python3 -m py_compile tools/stage2_android_pair_smoke.py
env PYTHONPATH=tools /usr/bin/python3 -m unittest tools.test_stage2_desktop_android_smoke tools.test_stage2_headless_android_smoke tools.test_stage2_scanner_android_smoke tools.test_validate_stage2_android_lifecycle tools.test_validate_stage2_android2unix_blueprint tools.test_validate_stage2_android_backup
git diff --check
```

The user guide now includes the exact direct-LAN command and distinguishes it
from adb-reverse interoperability evidence. The focused direct-LAN harness
tests and the full validation set (67 tests)
passed. Real Android/LAN execution remains unverified in
this sandbox because adb server and network fixture sockets are denied; no
physical-LAN success is claimed.

An offline Gradle attempt with a writable temporary Gradle home reached the
Gradle daemon but was refused when the daemon tried to bind its local socket
(`Operation not permitted`). The cached source-check path remains the available
Android compilation evidence in this sandbox.

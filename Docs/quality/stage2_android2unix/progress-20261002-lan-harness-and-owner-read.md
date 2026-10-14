# LAN Harness And Owner Read Boundary

Provisional Stage 2 progress at 2026-10-02. Existing dirty worktree changes
were preserved; no nested agents, commit, push, checklist promotion, or cleanup
was performed.

The direct pairing smoke harness now validates `--lan-ip` as a literal private
address, rejects loopback/wildcard/hostname inputs, requires an empty adb reverse
table, probes that the host owns the requested local interface, and verifies
that the invitation advertises exactly that address. It skips reverse mapping
only in direct-LAN mode and records LAN reachability only after the complete
instrumentation suite passes.

The shared Rust `HostAdapter` now fails closed for paged worker-status requests:
that view requires the live `WorktreeHost` controller and cannot silently fall
back to the compact workspace projection when used with a plain shared-pool
adapter. The redundant allowance for the paged view was removed from the
generic argument checks.

Verification:

```text
python3 -m py_compile tools/stage2_android_pair_smoke.py
env PYTHONPATH=tools /usr/bin/python3 -m unittest tools.test_stage2_desktop_android_smoke tools.test_stage2_headless_android_smoke tools.test_stage2_scanner_android_smoke tools.test_validate_stage2_android_lifecycle tools.test_validate_stage2_android2unix_blueprint tools.test_validate_stage2_android_backup
rtk cargo check --offline --locked
git diff --check
```

The Python suite passed 63 tests and Rust check passed with 0 errors (20
existing warnings). Device/LAN execution remains pending because the sandbox
cannot start adb or permit the Rust network fixture; no physical LAN claim is
made.

# Android Worker Status Pagination

Provisional Stage 2 progress at 2026-10-02. No blueprint item was promoted;
the canonical worktree remains dirty by design.

The Android worktree-task dialog now retains a bounded history of fetched
pages. Its previous-page action returns the cached page instead of issuing a
request with the current page's digest and an older offset, which the host
correctly rejects as a non-contiguous snapshot. The page helper verifies the
exact offset, adjacent page boundary, and project/session scope before
restoring a cached page. Forward navigation continues to send the current
page's digest and next offset, preserving the host snapshot fence. Closing or
refreshing the dialog clears the bounded history, and authority changes still
close the whole view.

Validation:

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

The cached Android source-check compilation was started in an isolated
temporary output directory and reached production Kotlin compilation with
only existing deprecation warnings. A real connected-device run is still
unavailable because adb cannot start its local server in this sandbox.

# Network Callback Teardown Safety

Provisional Stage 2 progress at 2026-10-02. Existing dirty worktree changes
were preserved; no nested agents, commit, push, checklist promotion, or cleanup
was performed.

`NetworkMonitor.stop()` now tolerates Android callback unregister races. Each
callback unregister is best-effort and idempotent, so an already-lost callback
cannot crash `Activity.onStop`, mask the offline transition, or prevent the
other local-network callback from being released. This keeps foreground,
background, route-loss and process-recreation lifecycle fencing fail-closed.

Verification:

```text
/bin/zsh tools/stage2_android_source_check.sh /private/tmp/zenpi-network-teardown-check
```

Cached API 35 production/unit/instrumented compilation passed and 253 JVM tests
passed. Existing Kotlin warnings remain. The direct Android device lifecycle
test is still pending because adb/Gradle daemon sockets are denied in this
sandbox.

The repository Python validation set also passed 68 tests, and `git diff
--check` passed.

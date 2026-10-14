# Android LAN Route Fallback

Provisional Stage 2 lifecycle progress at 2026-10-02T12:53:35Z. Existing dirty changes
were preserved; no nested agents, commit, push, checklist promotion, or
cleanup was performed.

`LanSocketRouting` now applies the same eligibility predicate to the fallback
`activeNetwork` that it applies to explicit route matches. A fallback can only
select a non-VPN, unrestricted Wi-Fi or Ethernet network. Cellular, VPN, and
restricted defaults are refused instead of silently binding the socket to an
unrelated route. The process-wide bound network is still never changed.

Verification:

```text
/bin/zsh tools/stage2_android_source_check.sh /private/tmp/zenpi-lan-fallback-check
```

The command exited 0, compiled cached API 35 production/unit/instrumented
artifacts, and ran 252 JVM tests successfully. Only existing Kotlin warnings
were emitted. Artifacts:

```text
eea21fbcfb6634e24e6a79ff6a3c6446702ff990d1a2a8908f427d1ab20c5a60  main.jar
ea6cf71f187dd065a4ada02299ff95a71dbdf09a09a3d48d6c10764812561917  unit.jar
0f50b87051d946725292401124cd8fabde3b65405d7c556e793eace8c3122e53  instrumented.jar
```

The Android emulator/device and physical-LAN route remain unverified in this
sandbox. The production Rust host pair integration test also cannot be used as
network evidence here because the sandbox denies the child fixture's socket or
temporary-file operation; no bypass was attempted.

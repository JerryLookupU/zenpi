# Side-Effect-Free Host Pair Help

Provisional S2-021/S2-070/S2-073 progress at 2026-10-03T19:27:44+0800.
Previous goal turn was progress: source-attributed host builds and device
preflight gates changed tools/tests and produced a fresh host binary. That
turn's actual CLI invocation found host pair --help failed as a missing-value
argument. This turn inspected the current parser/runtime and repaired it.
No nested agents, worker launches, checklist promotion, commit/push or cleanup.

HostPairOptions now recognizes --help/-h alone after host pair. Help does not
require HOME and returns before identity/registry/provider/workspace/listener
setup. Combined invalid/action flags remain errors, and --help used as the
value of --identity remains a path rather than accidentally triggering help.
The help lists QR/LAN setup, bind/advertise/defaults, workspace/session pairing,
minimum device grants, revoke and worktree/quota reconciliation. It explains
separate project authorization, private token handling and no Internet/NAT
claim. User guide/runbook document the safe discovery entry point.

## Verification

Commands ran from /Users/mac/Github/zenpi. Rust tests used controlled
ZENPI_HOME=/private/tmp/zenpi-resource-reload-test-home.

- rtk cargo test --offline --locked --test remote_tui_pairing host_pair_help
  -- --test-threads=1: exit 0, one passed. Both --help and -h run the real
  binary with HOME removed, isolated missing ZENPI_HOME and empty cwd; stderr
  empty, all expected options present, no invite output or created files.
  Invalid combined flags and flag-shaped identity values also checked.
- Same target filtered to host_cli_is_explicit: one passed, exit 0; filtered
  to terminal_pixels_decode: one passed, exit 0. Exact QR decoding covers IPv4
  and ULA terminal pixels, not physical camera or network communication.
- Current target/debug/zenpi host pair --help: exit 0, actual usage/options
  displayed. Prior failure documented in host-source-attribution progress is
  superseded for this current binary. No listener/identity was started.
- Python host build eight and host smoke seven tests pass, exit 0.
- cargo fmt --all -- --check; git diff --check; strict Blueprint validator:
  exit 0, all 36 unfinished.
- Host verify-only first refused source changes, then refused a binary changed
  by concurrently completing Cargo tests. This is expected attribution failure,
  not a passed build. After all tests terminal, stage2_host_build.py rebuilt
  and final --verify-only passed, exit 0. No Cargo command follows this final
  attributed build. Receipt/artifact digests below are final.
- APK verify-only: exit 1, missing current provenance. Android sources did
  not change; no fresh JVM/device pass claimed. No unchanged Gradle/ADB/TCP
  retry, stale APK install or permission bypass.

All required command handles are terminal. Existing worktree changes preserved.

```text
src/host_pair.rs 45c32d69c9935af39119b8bf9f12e2bb2baefea9b2ae597f861fbd4a95464403
tests/remote_tui_pairing.rs 158685e5ef315944839273d9ad636b8dc0ff0be0b4ea41a8b41d4b52b34d1c54
host source inventory cebe982976891747331085b9f288874fedc1bf6ed8fccfe2169747863522cf1e
target/stage2-host-build-debug.json 371985a0588ea1c77a2a0047e98fcea69939a7301968d08f06297f9850a6ceb4
target/debug/zenpi 76c53a6be1a34244e2018c8f36c1b3aa48c082ace7122dc3582e23cda16d4cb6
```

Goal remains active. Current APK, real scanner/physical direct LAN, connected
lifecycle/UI, full parity and signed reproducible release remain open. Help
and source attribution cannot substitute for product/device acceptance.

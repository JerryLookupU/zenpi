# Foreground Model Selection Synchronization

Provisional S2-019/S2-020/S2-033/S2-036/S2-054 progress observed
2026-10-02T04:33:19Z. Previous RTK-only turn verified local tool availability but
made no goal implementation change; classified no progress and followed by
current-source inspection and this implementation. Full Android/QR/LAN/Rust
desktop objective remains active. HEAD 1a8ff1acf72e0816fad503a05df2b1d18e07361b;
existing dirty changes preserved. All 36 authoritative rows remain [ ].

## Implementation

subscription::view dispatches the existing scoped, authorized ModelStatus read
after coherent workspace/subtab reads. Success projects only its existing
bounded status/catalog first page, Busy projects null, other failures terminate
the subscription. Foreign scope fails OwnerMismatch. Journal selection commits
already wake the same session; the changed-view path now carries their visible
state. No new execution route, provider call, transcript event or ACK exists.

Android accepts legacy listing/subtabs and new model_status views, uses the
strict existing codec, retains a model revision high-water mark across null Busy
samples and rejects regressions before committing the batch. HostProjection
carries transient status only; cache encoding excludes it. Open model panels
merge pushed status with direct/paged reads by scoped selection revision. Equal
catalog snapshots preserve the user's page; new snapshots/selection revisions
replace it. Existing remember keys reset stale model/reasoning drafts without
applying them. New Compose test covers this UI behavior but is not device-run.

## Verification

Cwd /Users/mac/Github/zenpi. Rust env
ZENPI_HOME=/private/tmp/zenpi-selection-sync unless explicitly noted.

- rtk cargo test --offline --locked --test remote_subscription
  --test remote_subscription_tls --test remote_workspace --test remote_protocol
  --test remote_parity --test remote_auth: exit 0, **109 pass**, six suites.
  Includes 11 subscription tests and 33 actual fragmented rustls TLS/P256 tests.
  New TLS test uses actual registry/backend, local set_model/set_reasoning_effort
  commits and one authenticated subscription; verifies exactly one changed
  selection view, unchanged transcript cursor, no transcript event, no private
  endpoint/credential, and no additional journal changes from reads.
- Independently reran current binaries remote_subscription-3e52e8790a1e76f9
  and remote_subscription_tls-4aa1bf218d94ac45 with --test-threads=1 and env
  ZENPI_HOME=/private/tmp/zenpi-selection-sync-direct: exits 0, **11 + 33 pass**.
- rtk cargo test --offline --locked --test remote_headless_commands -- --skip
  headless_runner_executes_real_http_tool_turn_and_retries_never_resubmit --skip
  production_plain_host_routes_authenticated_tls_prompt_to_the_provider:
  exit 0, **8 pass**, 2 TCP-dependent cases excluded.
- zsh tools/stage2_android_source_check.sh
  /private/tmp/zenpi-selection-sync-android-verified: exit 0, **164 JVM tests**;
  main/unit/instrumented Kotlin source compilation. Added malformed model/scope,
  monotonicity across Busy, cache omission, cursor preservation, catalog and
  read/push race tests. New Compose test compiled, not run on Android.
- env PYTHONPATH=tools python3 -m unittest
  tools.test_stage2_desktop_android_smoke tools.test_stage2_headless_android_smoke
  tools.test_stage2_scanner_android_smoke tools.test_validate_stage2_android_lifecycle
  tools.test_validate_stage2_android2unix_blueprint: exit 0, **59 pass**.
- cargo fmt --all -- --check, git diff --check, strict blueprint: exits 0.

Final compiled SHA256:

```text
main.jar fcd4be09506682f4dba2bbf1cb5c082b0a8f02106def7ecd592cd2c95461170a
unit.jar c12519e1f8aa9739407423e37825e70afe2573ceacf69636aa27d2d88539c428
instrumented.jar c031c06b4bef990dff078d18f653a4d833339b1ad1fdf967f9ca5e15bda3f5c7
```

An intermediate Rust fixture compile failed because set_model requires
Option<String>; fixed and the full listed suite reran successfully. Existing
Rust/Kotlin warnings remain. Actual TLS tests use encrypted in-memory streams,
not TCP/LAN emulation claims. Current-source APK, QR camera/LAN/device gates,
API matrix, quota scheduler, interactive PTY, full settings/account parity,
migration and signed release remain unfinished/unverified. No socket permission
bypass, worker admission, agents, commits, pushes, cleanup or goal completion.

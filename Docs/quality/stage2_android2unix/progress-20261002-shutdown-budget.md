# Shared Shutdown And Completion Persistence Budgets

Provisional S2-014/S2-019/S2-020/S2-054 progress observed
2026-10-02T05:50:24Z. Previous goal turn: progress (confirmed headless result
retention during reader contention). HEAD 1a8ff1acf72e0816fad503a05df2b1d18e07361b;
dirty baseline preserved. All 36 rows remain [ ]; full goal active. No agents,
worker admission, commits, pushes, cleanup or completion.

## Fix

Headless shutdown previously tried each owner's mutex once and gave each
runtime a new one-second join grace. Short readers could still lose terminal
progress and many noncooperative lanes could multiply the total shutdown wait.
Separate runtime-result collection from terminal persistence: all signalled
lanes share one absolute join deadline, then all collected statuses share one
250ms owner-lock acquisition window. Persistence sweeps every pending owner;
a held first project cannot hide another free result. Short readers release
and persist once; an append error/poisoned lock is not retried. Cancelled/detached
shutdown outcomes remain unknown, even after a late worker success return.

No runner/provider/tool resubmission. The shared deadlines bound runtime and
mutex waits, not uninterruptible kernel journal sync. A lock held beyond the
window still cannot be claimed resolved. Protocol documentation states these
limits instead of promising universally bounded disk IO or successful shutdown.

## Verification

Cwd /Users/mac/Github/zenpi;
ZENPI_HOME=/private/tmp/zenpi-shutdown-contention for Rust.

- rtk cargo test --offline --locked --lib remote::headless_host::tests:
  final exit 0, **8 pass**, including eight noncooperative lanes.
  New tests cover a real reader lock releasing during the shared window,
  one append/reopen; seven permanently held owners plus one free owner prove
  shared wait is not multiplied and free status is preserved. Noncooperative
  workers share one join deadline and retain unknown after late release; fixture
  threads are explicitly released and observed finished, not abandoned.
  Existing success/failure-code/shutdown/journal-conflict tests retained.
- rtk cargo test --offline --locked --test remote_device_grants
  --test remote_workspace --test remote_parity --test runtime:
  exit 0, **64 pass** (5/39/5/15).
- rtk cargo test --offline --locked --test remote_headless_commands -- --skip
  headless_runner_executes_real_http_tool_turn_and_retries_never_resubmit --skip
  production_plain_host_routes_authenticated_tls_prompt_to_the_provider:
  exit 0, **8 pass**, 2 TCP cases explicitly excluded. Production owner/mailbox
  cooperative shutdown, cancellation and deduplicated attachment workflows.
- Six Python smoke/lifecycle/blueprint/backup modules: exit 0, **63 pass**.
- cargo fmt --all -- --check; git diff --check;
  python3 tools/validate_stage2_android2unix_blueprint.py --strict: exit 0.

SHA256:

```text
headless_host.rs 2f40b63cba165d29b1ac8d6378863ffa1676ee7f3ea2a0ed0fafff4298605e3a
blueprint d76e70e369f11f0fd3d1905d56d79c2a89401b92215da48ad85314f390d96d97
specification 403e6339b0318a420d93600fc52926acf6a9e46ec899bd9a9edb03c235190a28
```

Provisional progress note, not Master acceptance receipt with output hashes.
No Android source changed; no current APK/camera/LAN/device execution claimed.
Full settings/scheduler/stepper/interactive PTY parity, API matrix, security
sign-off and signed release remain incomplete. No socket/adb/Gradle bypass.

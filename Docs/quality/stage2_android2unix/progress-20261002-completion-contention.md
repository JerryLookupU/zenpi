# Headless Completion Persistence Under Reader Contention

Provisional S2-014/S2-019/S2-020/S2-054 progress observed
2026-10-02T05:46:20Z. Previous goal turn: progress (phone device-ID display and
local permission JSON). HEAD 1a8ff1acf72e0816fad503a05df2b1d18e07361b;
existing dirty baseline retained. All 36 rows remain [ ]; full goal active.
No agents, worker admission, commits, pushes, cleanup or completion.

## Fix

Headless owner loop previously consumed Completed and tried Agent mutex only
once. A short reader lock caused live unknown publication, active-lane removal
and permanent loss of the confirmed terminal progress append. Retain one
typed completion per existing ActivePrompt and retry only mutex acquisition
on later owner-loop passes. WouldBlock makes no append or display change;
the lane stays admitted/busy so no replacement task can race its persistence.
The bounded mailbox and other project lanes continue to be serviced.

Once the mutex is acquired, attempt the original terminal append exactly once.
Success publishes its durable result; poisoned owner or append error retires
with unknown and no automatic append retry because disk outcome may be
ambiguous. No runner submission/provider/tool rerun is introduced. Shutdown
starts from a retained consumed completion and preserves it if the owner is
available; a cancelled/detached outcome remains conservative unknown on shutdown.
Shutdown with a still-held owner cannot promise a durable terminal result.

## Verification

Cwd /Users/mac/Github/zenpi; Rust
ZENPI_HOME=/private/tmp/zenpi-completion-contention.

- rtk cargo test --offline --locked --lib remote::headless_host::tests:
  final exit 0, **5 pass**, including three new deterministic cases.
  Real runtime completion is consumed, reader holds actual mutex across ten
  persistence polls: no journal change, typed result retained and running view
  preserved. Release persists success/failed+code/cancelled exactly once; new
  PromptProgress and reopened SessionStore recover the same terminal result.
  External journal length conflict causes unknown with unchanged external
  bytes and no second append. Shutdown after consumed completion retains success.
  Existing real Echo shutdown success and safe failure-code tests still pass.
- rtk cargo test --offline --locked --test remote_device_grants
  --test remote_workspace --test remote_parity --test remote_subscription_tls
  --test remote_auth: exit 0, **95 pass** (5/39/5/35/11).
  Actual mailbox prompt/tool approval allow/deny and exact retry tests plus
  authenticated fragmented TLS subscription regression remain green.
- rtk cargo test --offline --locked --test remote_headless_commands -- --skip
  headless_runner_executes_real_http_tool_turn_and_retries_never_resubmit --skip
  production_plain_host_routes_authenticated_tls_prompt_to_the_provider:
  final exit 0, **8 pass** after test-fixture-only adjustment.
  Two real TCP cases explicitly excluded, not claimed passing.
- Six Python smoke/lifecycle/blueprint/backup modules: exit 0, **63 pass**.
- cargo fmt --all -- --check; git diff --check;
  python3 tools/validate_stage2_android2unix_blueprint.py --strict: exit 0.

Initial append-failure fixture assumed open_existing meant subsequent appends
were forbidden; that assumption was false. Changed the test to an actual
external journal length conflict enforced by SessionStore rather than altering
production behavior. Final unit run passes. No Android production/source change
this turn; no APK/device rebuild, screenshot or LAN proof claimed.

SHA256:

```text
headless_host.rs a3e225c6ba418fa1648afa1aec949bc7b2a2f6ae9601f466789a42545475943a
blueprint d76e70e369f11f0fd3d1905d56d79c2a89401b92215da48ad85314f390d96d97
specification 403e6339b0318a420d93600fc52926acf6a9e46ec899bd9a9edb03c235190a28
```

Provisional progress note, not full Master acceptance receipt with output hashes.
Current APK real camera/LAN/API matrix, scheduler/stepper/full settings/interactive
PTY, security sign-off and signed release remain incomplete. No socket/adb/
Gradle permission bypass or acceptance promotion.

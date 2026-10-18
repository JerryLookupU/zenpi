# Model And Reasoning Device Gate

Provisional S2-036/S2-051/S2-054/S2-055 progress observed
2026-10-02T04:17:08Z. Previous goal turn implemented durable reasoning selection
and behavioral tests: progress. Full Android/QR/LAN/Rust desktop goal remains
active. HEAD 1a8ff1acf72e0816fad503a05df2b1d18e07361b; dirty changes preserved.
All 36 authoritative rows remain [ ]; no Master acceptance.

## Prepared Behavioral Chain

Expanded the existing Android HeadlessHostTest production ViewModel/Compose
harness, not a separate mock screen. It performs the original first attached
prompt with gpt-5.2/high, waits for its terminal result, opens model status,
changes Model ID to gpt-5.2-2025-12-11, confirms through the encrypted pending
write path, rereads status, picks low from the real reasoning dropdown, applies
and rereads again. It asserts increasing selection revision, retained model/
effort where appropriate, cleared durable pending writes and unchanged durable
transcript/event cursor. Captures headless-selection.png and reports a phase
only after these assertions. Background write probes now include both model and
reasoning setters. Existing resource paging, second literal-template prompt,
actual process-death pending recovery and exact retry remain in the chain.

Python smoke config uses the actual model_reasoning_effort config key, initial
gpt-5.2/high, and still requires exactly two provider requests. It checks the
new instrumentation phase and saves selection PNG/hash. Independent verification
requires first request model/effort gpt-5.2/high and second request
gpt-5.2-2025-12-11/low. A response echo of the chosen model is not sufficient.

Saved journal evidence must have exactly two model_selected records and two
exact reserved -> selection -> succeeded admission chains, matching sequence
numbers, operation-specific checkpoint namespace, request/device/project/session
and fingerprint. First prompt must complete before changes; second prompt must
be admitted afterward. Duplicate, missing, reordered, wrong-model/effort,
foreign-scope, malformed/null terminal, stale revision and mismatched binding
fixtures fail. Prompt evidence now expects four durable identities: two prompts
and two settings operations. ACK still counts only actual durable turns.

API 31/34/35 headless matrix now requires the new
android_model_reasoning_selection_provider_verified gate and checksum-verified
selection screenshot. Saved command verifier runs the independent provider/
journal/binding chain even when its file hash is valid. Historical receipts
lacking this chain cannot be promoted under the new validator.

## Verification

Cwd /Users/mac/Github/zenpi.

- env PYTHONPATH=tools python3 -m unittest
  tools.test_stage2_desktop_android_smoke tools.test_stage2_headless_android_smoke
  tools.test_stage2_scanner_android_smoke
  tools.test_validate_stage2_android_lifecycle
  tools.test_validate_stage2_android2unix_blueprint: exit 0, **59 pass**.
  Added selection evidence adversarial cases and lifecycle asset/provider gate
  test. Focused headless/lifecycle modules: exit 0, **34 pass**.
- zsh tools/stage2_android_source_check.sh
  /private/tmp/zenpi-selection-device-gate-android: exit 0, **160 JVM tests** in
  31 classes; main/unit/instrumented compile. Changed device harness compiled,
  not executed on an emulator/phone. Existing compiler/deprecation warnings.
- env ZENPI_HOME=/private/tmp/zenpi-selection-device-gate rtk cargo test
  --offline --locked --test remote_workspace --test remote_protocol
  --test remote_parity --test remote_subscription_tls --test remote_auth:
  exit 0, **95 pass**, no product source change to the shared selection domain.
- Responses fixture encoding tests additionally require exact initial model/
  high and selected model/low alongside their existing attachment/template
  byte checks. This checks production wire encoding without claiming HTTP.
  Exact argv: env ZENPI_HOME=/private/tmp/zenpi-selection-device-gate rtk cargo
  test --offline --locked --lib protocols::responses::tests::host_: exit 0,
  **2 pass**, 382 filtered out.
- cargo fmt --all -- --check, git diff --check, strict blueprint: exit 0.

Compiled SHA256:

```text
main.jar 7dcd1bf18c2ffc02a42b72d49f555ab24fe216c25ea3ae0934030a69e92b4499
unit.jar 80751b095c609ae6a3136f5b3598ef19290eb8a9aa896e198ece4ed27ec7599e
instrumented.jar 8771f7de8d50d888464d2f5a3a436f0dfd27ce3de59d7017d60cc8bab5da627d
```

## Unverified Scope

No current-source APK, camera/QR/LAN, actual new device phase/screenshot or
provider HTTP acceptance claimed. These are prepared gates and independently
tested validators, not fabricated execution evidence. Prior Gradle daemon
ServerSocket EPERM and adb/listener restrictions prevent device execution in
this environment; no bypass, package reinstall or unrelated process cleanup.
Full model catalogue/account settings, quota scheduler, interactive PTY,
provider-delta replay/ACK, migration and signed release remain unfinished.
No agents, worker admission, commits, pushes or goal completion.

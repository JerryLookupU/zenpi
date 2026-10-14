# Typed Remote Prompt Failure Diagnosis

Provisional S2-019/S2-032/S2-033/S2-036/S2-054/S2-072 progress observed
2026-10-02T05:20:58Z. Previous goal work: progress (private file admission and
typed failure implementation); the intervening RTK check confirmed installed
tooling and did not change project acceptance. HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b, current dirty baseline preserved.
All 36 authoritative rows remain [ ]; full Android/QR/LAN/desktop goal active.
No agents, worker admission, commits, pushes, cleanup or completion.

## Implementation

Failed prompt status optionally carries existing wire ErrorCode, never raw
Agent/provider diagnostics. InvalidTurn/Closed/NoActiveTurn maps to
invalid_request; NotIdle to busy; ToolLoopLimit/Governance to resource_limit;
other execution errors and confirmed panic to internal_error. Running, success,
cancelled and unknown status omit the field. TUI/headless terminal paths
persist before live publication; failed persistence remains unknown. Headless
shutdown now retains the code for confirmed failed/panicked runtime results,
but cancellation/join uncertainty stays unknown without diagnostic code.

Journal restoration accepts only typed known enum values with existing
device/project/session scope. Android validates optional failure_code only on
failed status, refuses raw-message/unknown/null/type/state mismatches and shows
fixed bounded labels. Cache v3 retains terminal diagnosis but never grants
prompt authority or caches active stream text. Old records without codes stay
readable in the current client; old strict clients are not claimed compatible
with new status fields. Narrow status labels use weighted two-line ellipsis.

## Verification

Cwd /Users/mac/Github/zenpi. Rust environment
ZENPI_HOME=/private/tmp/zenpi-prompt-failure; offline locked Cargo through RTK.

- rtk cargo test --offline --locked --test remote_workspace
  --test remote_subscription_tls --test remote_protocol --test remote_auth
  --test remote_parity: final exit 0, **104 pass** (39/35/14/11/5).
  New authenticated fragmented TLS fixture sends one typed failure view on
  the same subscription, no transcript event/cursor advance, no foreign-device
  status and no raw secret/path. Uses existing production journal projection
  with the shared read adapter; not a real TCP listener or actual phone claim.
- rtk cargo test --offline --locked --test remote_headless_commands -- --skip
  headless_runner_executes_real_http_tool_turn_and_retries_never_resubmit --skip
  production_plain_host_routes_authenticated_tls_prompt_to_the_provider:
  exit 0, **8 pass**, 2 TCP-dependent tests excluded. Actual headless mailbox
  attachment failures expose invalid_request and do not call provider again.
- rtk cargo test --offline --locked --lib typed_prompt_failure_survives_restart:
  exit 0, **1 pass**; true SessionStore/owner-pool reopen retains typed failure,
  no raw diagnostic journal content, no cross-device visibility, no code on
  nonfailed live records.
- rtk cargo test --offline --locked --lib shutdown_retains: exit 0, **2 pass**.
  Confirmed synthetic failing runtime result retained through real shutdown and
  journal reopen; existing real Echo prompt shutdown success still retained.
- zsh tools/stage2_android_source_check.sh
  /private/tmp/zenpi-prompt-failure-android-subscription: exit 0,
  **176 JVM tests**, main/unit/instrumented source compile. New strict codec,
  cache and subscription reducer tests; 320dp offline Compose case compiles
  only, not device executed. Existing source warnings remain.
- env PYTHONPATH=tools python3 -m unittest
  tools.test_stage2_desktop_android_smoke tools.test_stage2_headless_android_smoke
  tools.test_stage2_scanner_android_smoke tools.test_validate_stage2_android_lifecycle
  tools.test_validate_stage2_android2unix_blueprint
  tools.test_validate_stage2_android_backup: exit 0, **63 pass**.
- cargo fmt --all -- --check; git diff --check;
  python3 tools/validate_stage2_android2unix_blueprint.py --strict: exit 0.

Final Android SHA256:

```text
main.jar dadc22a4160f2b873f9512195e652bc5354f879e00cf850984ffa1fef990b2db
unit.jar f09f76bbe399bc0b59c5a15ca116cdad976a9c4f74e9c0d2147e47c85842cf62
instrumented.jar e4a4020220184b070b121e50a9b0caf59f503d880bbad0d3c3651dd7c97a1e3a
blueprint d76e70e369f11f0fd3d1905d56d79c2a89401b92215da48ad85314f390d96d97
specification 403e6339b0318a420d93600fc52926acf6a9e46ec899bd9a9edb03c235190a28
```

An intermediate TLS test failed because the shared read-only adapter does not
decorate workspace listings with runner progress. The fixture now uses the
same production PromptProgress journal projection as TUI/headless rather than
weakening the assertion or changing production ownership. Focused retest and
the complete five-suite rerun pass. This is a provisional progress note, not
a full Master acceptance receipt with captured stdout/stderr digests.

## Remaining Gates

No current APK/device execution, real QR camera/LAN/API 31/34/35 matrix or signed
release demonstrated by these checks. Full settings, scheduler quota and
interactive PTY parity remain incomplete; full security/migration/backup/
uninstall release acceptance remains outstanding. Safe codes do not classify
all execution causes or promise remediation; unknown remains ambiguous and
no provider retry was introduced. No permission bypass or checklist promotion.

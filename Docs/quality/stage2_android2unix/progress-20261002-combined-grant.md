# Explicit Control And Approval Grant

Provisional S2-013/S2-019/S2-036/S2-052/S2-054/S2-073 progress observed
2026-10-02T05:36:32Z. Previous goal turn: progress (local project-ID discovery
and actual operator guide). HEAD 1a8ff1acf72e0816fad503a05df2b1d18e07361b;
existing dirty baseline preserved. All 36 rows remain [ ]; full goal active.
No agents, worker admission, commits, pushes, cleanup or completion.

## Implementation

Existing grant CLI replaced permissions with either Read/Control or
Read/Approval, so a single phone could not both submit and decide its approval
without a later host grant swap. Added explicit local
--grant-device-control-approval ID --project ID. It replaces this device's
capabilities with Read/Control/Approval and the explicit project ACL using the
same registry CAS writer. The separate least-privilege options retain their
old behavior; duplicate/combined grant flags still refuse. Control does not
implicitly include Approval; no remote escalation operation or wildcard ACL.
CLI help and operator/protocol/Android docs reflect the explicit opt-in.

## Verification

Cwd /Users/mac/Github/zenpi. Rust ZENPI_HOME=/private/tmp/zenpi-combined-grant.

- rtk cargo test --offline --locked --test remote_device_grants
  --test remote_project_inspection --test remote_auth --test remote_parity
  --test remote_workspace: exit 0, **64 pass** (4/5/11/5/39).
  Actual production CLI persists combined capabilities across registry reopen,
  explicit project scope, old authentication revision fencing, unrelated device
  preservation, identity bytes unchanged and no provider config creation.
  Downgrade by all three old grant options removes the other capability and old
  project; revoked device cannot regain permission. Invalid/duplicate scope and
  conflicting action flags leave registry bytes unchanged.
  Two actual HeadlessHost owner/mailbox tests use a controlled non-network
  backend to request write_file under Always approval. Same CLI-granted phone
  submits and allows/denies; both capabilities appear in workspace projection.
  Allow writes exactly the fixture bytes; Deny writes nothing. Exact prompt and
  approval retry returns historical receipt without extra backend/tool action.
  approval_resolved audit includes device/request/project/session/digest and
  remember=false. This is not Android socket/camera/LAN proof.
- rtk cargo test --offline --locked --test remote_headless_commands -- --skip
  headless_runner_executes_real_http_tool_turn_and_retries_never_resubmit --skip
  production_plain_host_routes_authenticated_tls_prompt_to_the_provider:
  exit 0, **8 pass**, 2 TCP-dependent cases excluded.
- rtk cargo test --offline --locked --test approval_owner
  --test remote_approval_wakeup: exit 0, **15 pass** (14/1).
- rtk cargo test --offline --locked --test remote_tui_pairing -- --skip
  committed_tls_write_with_busy_delivery_stays_unknown_and_exact_retry_is_idempotent
  --skip production_host_pair_negotiates_and_consumes_invite_over_tls:
  exit 0, **2 pass**, 2 TCP cases excluded; combined grant parser assertion and
  actual production QR-pixel decode retained.
- zsh tools/stage2_android_source_check.sh
  /private/tmp/zenpi-combined-grant-android: exit 0, **177 JVM tests**,
  main/unit/instrumented compile. Added same-projection prompt+approval command
  test, separate capability removal negatives and offline cache authority denial.
  Instrumented/Compose compiled only. No Android production change this turn.
- Six Python smoke/lifecycle/blueprint/backup modules: exit 0, **63 pass**.
- cargo fmt --all -- --check; git diff --check;
  python3 tools/validate_stage2_android2unix_blueprint.py --strict: exit 0.

Intermediate combined workflow fixture timed out because its workspace_list
envelope lacked the required request ID. Fixed the fixture to the existing
production envelope contract, asserted successful listing, and reran the
actual workflow. Production validation was not relaxed. Final 64-test rerun
includes allow and deny cases with exact durable actor audit assertions.

SHA256:

```text
host_pair.rs 5b103609e34a91acf41bf3d0fcf570c46338bbd44f57841803668383d4bbbe8a
remote_device_grants.rs 24bf0b39b3fb90c73fda5d645044d260accc6a9bf1ae3004caa39c7a05168638
ApprovalProjectionTest.kt a9c2aab1dfba536e6538b581f201aa55b3c9a62f295616f96a85fb8b01047806
main.jar dadc22a4160f2b873f9512195e652bc5354f879e00cf850984ffa1fef990b2db
unit.jar 7aef3c59c30d84a95f649efe8a2267ee59a699554715b6041b80b7dd1b64e02d
instrumented.jar e4a4020220184b070b121e50a9b0caf59f503d880bbad0d3c3651dd7c97a1e3a
blueprint d76e70e369f11f0fd3d1905d56d79c2a89401b92215da48ad85314f390d96d97
specification 403e6339b0318a420d93600fc52926acf6a9e46ec899bd9a9edb03c235190a28
```

Provisional progress note, not full Master acceptance receipt with captured
stdout/stderr hashes. Current APK real camera/LAN/API matrix, full settings,
independent per-worktree scheduler/stepper, interactive PTY, security sign-off
and signed reproducible release remain unfinished. No bypass of socket/adb/
Gradle restrictions and no acceptance promotion.

# Local Project IDs And LAN Operator Guide

Provisional S2-015/S2-051/S2-070/S2-073 progress observed
2026-10-02T05:29:55Z. Previous goal turn: progress (typed failure synchronization,
shutdown restoration and tests). HEAD 1a8ff1acf72e0816fad503a05df2b1d18e07361b;
existing dirty baseline retained. All 36 authoritative rows remain [ ]; goal
active. No worker admission, agents, commits, pushes, cleanup or completion.

## Implementation

First pairing needs explicit project ACL, but an ungranted phone cannot read
the project ID. Added exclusive local host pair --list-projects with required
workspace/session. It runs before identity/provider/listener preparation and
prints JSON projects with stable id, local cwd and active selection. Reuses
ProjectWorkspace validation and the existing bounded O_NOFOLLOW checkpoint
reader; read-only SessionStore checks the supplied journal/workspace. Missing
checkpoint derives only bootstrap identity without creating anything. Corrupt,
oversized, linked, unsupported, invalid-ID or session-mismatched state refuses.
Saved session map requires exact known projects and absolute bounded paths.
This local projection grants no authority and is not a network operation.

CLI help includes project listing and all existing read/control/approval grant
actions. Existing desktop listener entry also refuses inspection-only options.
No wire or Android code changed in this turn.

Added Docs/stage2_android2unix_user_guide.md in Chinese, linked from Android
development docs. Covers actual debug build/install commands, desktop-owned
QR entry, explicit LAN address, expiry/refresh/Esc semantics, fingerprint/SAS,
device/project discovery, initial Read then explicit Control grant, replacement
ACL (not additive), distinct Approval, custom identity, cache/replay/unknown
outcome, exact retry, revoke vs local forget and residual product/release gaps.
Current old APKs are explicitly not current-source proof. No invented current
device/camera/LAN success or concurrency/interactive PTY parity claim.

## Verification

Cwd /Users/mac/Github/zenpi; Rust ZENPI_HOME=/private/tmp/zenpi-project-inspection.

- rtk cargo test --offline --locked --test remote_project_inspection
  --test remote_workspace --test remote_migration --test remote_subtab
  --test remote_auth --test remote_lan: exit 0, **77 pass** (4/39/6/10/11/7).
- Final rtk cargo test --offline --locked --test remote_project_inspection:
  exit 0, **5 pass**, adds guide/link/help check. **78 unique tests** across
  these six suites with latest inspection cases. Actual production binary
  verifies bootstrap/saved-project output, live-owner checkpoint preservation,
  no identity/config creation, no journal/checkpoint writes, missing/wrong
  session, symlinks, invalid/oversized/unsupported checkpoint and mapping/ID
  negatives. Parser rejects combined grant/revoke/desktop/repair actions.
- rtk cargo test --offline --locked --test remote_tui_pairing -- --skip
  committed_tls_write_with_busy_delivery_stays_unknown_and_exact_retry_is_idempotent
  --skip production_host_pair_negotiates_and_consumes_invite_over_tls:
  exit 0, **2 pass**, 2 TCP tests explicitly excluded. Real terminal QR pixels
  decode plus existing host CLI policy test; not current camera/TCP acceptance.
- Six Python smoke/lifecycle/blueprint/backup modules: exit 0, **63 pass**.
- cargo fmt --all -- --check; git diff --check;
  python3 tools/validate_stage2_android2unix_blueprint.py --strict: exit 0,
  36 unfinished rows, no acceptance promotion.

The guide test parses concrete documented CLI shapes via production parse_args,
checks real binary --help and verifies all three repository document links.
It does not execute Gradle/install/grant on a real user's device or identity.
No Android rebuild/device run needed for unchanged Android source, and none
claimed. Existing TCP/adb/Gradle socket policy restrictions not bypassed.

SHA256:

```text
src/host_pair.rs e467b30cbceb036deb125cd76df9ad05c2d18b8d11e940a70c5a2a77140e4149
tests/remote_project_inspection.rs 92610965e2d68be1aad4ade9cfa52913b9cb96b9a9cef62e3a602285bf485ebb
user_guide 7268cdb2a5dce3ecb7f63f21864cbca063d481599a88bf15f5ba871579e68ec7
blueprint d76e70e369f11f0fd3d1905d56d79c2a89401b92215da48ad85314f390d96d97
specification 403e6339b0318a420d93600fc52926acf6a9e46ec899bd9a9edb03c235190a28
```

Provisional note, not full Master receipt with captured output digests. Current
APK/real camera/LAN/API matrix, scheduler/stepper/full settings/interactive PTY,
security sign-off and reproducible signed release remain incomplete.

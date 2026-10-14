# Android Typed Authority-Loss Fence

Provisional S2-032/S2-036/S2-037/S2-038/S2-052/S2-054 progress observed
2026-10-02T06:23:51Z. Previous goal turn made progress (operation deadline
distinguished from lifecycle cancellation). HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b; dirty changes preserved.
All 36 authoritative rows remain [ ]; full goal active. No worker admission,
subagents, commit, push, cleanup or Master acceptance.

## Implementation

HostClient now carries the validated refusal code on WorkspaceRefusedException
as well as HostRefusedException. Existing one-argument constructor compatibility
is retained; generic exception-message similarity is not an authority signal.
AuthorityLossPolicy recognizes only typed unauthorized, revoked_device and
owner_mismatch. Its displayOnly transform reuses the strict cache codec, strips
grants/owner epochs/approval/terminal/model/live previews and running authority,
and retains durable turns, typed terminal outcomes, subtab display and cursors.
Unresolved command/revoke identity and signing host metadata are not removed.
Uncacheable display fails closed without discarding unresolved request identity.

PairingViewModel applies this fence in foreground refresh/write refusal and
each transient read (Diff, transcript, files, resources, model). It cancels and
generation-fences transient reads; a refusal from a separate reader also cancels
and generation-fences the main connection to prevent a late subscription from
restoring stale authority. Ordinary foreground refusal stops automatic attach;
explicit Refresh can obtain fresh authentication. Main command handling still
persists a definitive refusal before clearing its pending command; failed local
resolution preserves the pending identity with sanitized, not old-live display.
Timeout/busy/revision conflict are not conflated with revocation success.

WorkspaceDispatchPolicy now rechecks current scope, grant, owner/revision and
running/approval target for error-phase steer/cancel/approve. A prebuilt command
from before the fence cannot bypass it. The explicit exact durable-request retry
branch remains unchanged. Host authorization remains the authoritative gate.

Added narrow Compose coverage: authority loss disables an already-open approval
confirmation, removes running cancel/approval controls and transient text, but
preserves durable display. Desktop smoke control-suite count advanced to 34;
non-control count remains 16. Guide/protocol describe authentication recovery.

## Verification

Cwd /Users/mac/Github/zenpi. Final commands exit 0:

- zsh tools/stage2_android_source_check.sh
  /private/tmp/zenpi-authority-loss-android-final3: **195 JVM tests**, all main/
  unit/instrumented sources compile. Six new policy tests cover typed-versus-
  textual refusal, every control/transient field removal, cursor/terminal outcome
  retention, unresolved requests/revokes, malformed cache fallback, prebuilt
  controls with stale owner/revision/grant/target, and correlated refusal after
  an unconfirmed event through real FrameCodec/ResponseReader (zero publication,
  unchanged cursor). Compose/device sources compile only; not executed.
- env ZENPI_HOME=/private/tmp/zenpi-authority-loss rtk cargo test --offline
  --locked --test remote_auth --test remote_device_grants --test remote_subscription_tls
  --test remote_parity: **59 tests**. Existing production local grants/ACL/
  revocation and rustls encrypted fragmented-stream behavior remain green.
- env PYTHONPATH=tools python3 -m unittest
  tools.test_stage2_desktop_android_smoke tools.test_stage2_headless_android_smoke
  tools.test_stage2_scanner_android_smoke tools.test_validate_stage2_android_lifecycle
  tools.test_validate_stage2_android2unix_blueprint tools.test_validate_stage2_android_backup:
  **63 tests**.
- cargo fmt --all -- --check; git diff --check; python3
  tools/validate_stage2_android2unix_blueprint.py --strict: exit 0.

Final cached-source artifact SHA256:

```text
main.jar 2367cc2c698b9027e7892aca1f37bbe87043362bdf06a7256ca0074b23b50719
unit.jar ad4cd5918f29ef6863b8ac91a0301a04e5e715957417290a1c0bed8b7b2d839b
instrumented.jar cb6b4a7ac81a5a57faed3268ed8a30e9c6c76372fbc0e315b452346ac2b5508e
```

Source/requirement SHA256:

```text
Docs/stage2_android2unix_blueprint.md d76e70e369f11f0fd3d1905d56d79c2a89401b92215da48ad85314f390d96d97
Docs/stage2_android2unix_spec.md 403e6339b0318a420d93600fc52926acf6a9e46ec899bd9a9edb03c235190a28
AuthorityLossPolicy.kt edc7aad4b99e7bffaa884a3424b049bd3f0524500a4ae8c1f5d19d06992db204
WorkspaceDispatchPolicy.kt b6b5ca964734158486c2881e5b3785738a2cdc30db84576638b9d11e7c5f31c4
PairingViewModel.kt 2fb2b025a9727e033af8b5b689ef77965870b6c93dc6e3ad85b522cfa061e34d
FrameCodec.kt f4bf67c60690dc1cff1bef232f5082f59d1ea2ebdc98b46f936308e9332c557d
HostClient.kt b3aba65db919d358695a4cccdb1d63eb2b463daf953f590b322e427ec9c1842c
AuthorityLossPolicyTest.kt 9dccb2c3267fd9cd2914c6af78e93c5805cf3e12e4116d9583ec1602820b19b6
ApprovalScreenTest.kt 89eed1a4a24ace1edda5f9ebdba45afcc76910ba79ab64e1ea29e1f0aa7d3954
```

## Residual Scope

No current APK, device execution, physical QR scan/LAN revocation/reattach race,
API 31/34/35 acceptance or release package. Coroutines/generation integration
has source compilation plus policy coverage, not a live Android ViewModel test
against the desktop. Signing keys are deliberately retained on refusal until
explicit reset or proven self-revoke completion. Full scheduler, interactive
PTY/settings parity and release gates remain unfinished. No socket/Gradle/adb
permission bypass. Provisional progress, not full item acceptance.

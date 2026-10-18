# Device Identity And Local Permission Inspection

Provisional S2-013/S2-031/S2-037/S2-052/S2-073 progress observed
2026-10-02T05:42:01Z. Previous goal turn: progress (explicit combined permission,
same-device prompt/approval flow and regression evidence). HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b; existing dirty worktree preserved.
All 36 authoritative rows remain [ ]; full goal active. No agents, worker
admission, commits, pushes, cleanup or completion.

## Implementation

All newly paired phones currently use device name Android. Added full paired
device ID to Android's host area in four 16-character monospace lines and an
explicit copy icon with tooltip/accessibility label. It copies the exact ID,
not its formatted display or invitation token. The ID comes from saved paired
host metadata, not a new key or permission. It stays available offline, disables
copy while resetting and disappears when host state is cleared. Saved IPv6
host addresses now display brackets around the literal.

Local host pair --list-devices --json emits only devices with
id/name/revoked/revision/capabilities/projects. Existing text listing is unchanged.
JSON does not expose key/proof/creation-time/revocation-request fields and starts
no listener or provider work. --json without --list-devices refuses. User guide
now explicitly instructs comparing the full phone ID, not default name or list
position, before selecting an ACL target. Protocol/Android docs updated.

## Verification

Cwd /Users/mac/Github/zenpi; Rust ZENPI_HOME=/private/tmp/zenpi-device-identity.

- rtk cargo test --offline --locked --test remote_device_grants
  --test remote_project_inspection --test remote_auth --test remote_parity
  --test remote_workspace: final exit 0, **65 pass** (5/5/11/5/39).
  New production-binary JSON listing test has two same-name devices, differing
  grant/revoke state and explicit project scope, exact six-field whitelist,
  actual registry correspondence, no public key, unchanged registry/identity
  bytes, backward text output, parser conflict negatives and corrupt registry
  refusal without echoing raw corrupt content or rewriting it. Existing real
  headless allow/deny/deduplicated prompt/approval flow passes.
- rtk cargo test --offline --locked --test remote_tui_pairing -- --skip
  committed_tls_write_with_busy_delivery_stays_unknown_and_exact_retry_is_idempotent
  --skip production_host_pair_negotiates_and_consumes_invite_over_tls:
  exit 0, **2 pass**, 2 TCP-dependent cases excluded.
- zsh tools/stage2_android_source_check.sh
  /private/tmp/zenpi-device-identity-android-final: exit 0,
  **177 JVM tests**, main/unit/instrumented compile. New 320dp offline Compose
  ID/IPv6/copy/reset/clear-state case source-compiles only; not device execution
  or screenshot verification. Existing compiler warnings remain.
- Six Python smoke/lifecycle/blueprint/backup modules: exit 0, **63 pass**.
- cargo fmt --all -- --check; git diff --check;
  python3 tools/validate_stage2_android2unix_blueprint.py --strict: exit 0.

Intermediate combined deny fixture timed out because it repeatedly try_locked
the Agent while the headless owner needed that lock for completion persistence.
Changed the fixture to query the production workspace status mailbox (as the
phone does), with an independent bounded completion wait and diagnostic output.
No production persistence or unknown-outcome policy was relaxed. Full rerun passes.

SHA256:

```text
host_pair.rs f7ebad25f418620b428700e57b47532c4e45dd6c6913080a4acc07db97225240
remote_device_grants.rs de204cce85f8e623b55a4ff166c8522c73888b86da9fb3bfcbfd41bb0c0afbf6
MainActivity.kt b04d73a6f5562c1c17c381d721527f8d1020001c0a4ac24013bdeca99360dec7
PairingScreenTest.kt ff922cb575b7ed6e2b11489aadf772c8d9608e227ebd375423321b8c6652eff6
main.jar 042b8dde397f451337f550c43e34216c6382ac1c35af2660bedab7f0359d8d19
unit.jar 7aef3c59c30d84a95f649efe8a2267ee59a699554715b6041b80b7dd1b64e02d
instrumented.jar d5ab3994cc030b694bb64c06a641967d634848d1e9879064ed3536b6e00d828c
```

Provisional progress, not full Master acceptance receipt with captured output
hashes. JSON local listing still uses existing load_or_create host identity
behavior, like text listing; no claim it is side-effect-free on an uninitialized
identity path. Device ID is public correlation, not proof that a phone is trusted.
Current APK real QR camera/LAN/API matrix, full scheduler/stepper/settings/PTY
parity, security sign-off and signed release remain unfinished. No bypass of
socket/adb/Gradle restrictions or checklist promotion.

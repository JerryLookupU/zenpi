# Retire Invitation After Registration

Recorded 2026-10-04. Partial S2-031/S2-037/S2-038/S2-052/S2-072 evidence,
not Master acceptance. Previous goal turn was progress: raw scanner input
repair. Current worktree, execution skill, Blueprint and camera/registration/
storage/reconnect UI paths inspected. No nested agents, acceptance marks,
source rollback, permission bypass or repeated denied Gradle/ADB attempt.

## Repair

After a successful PairingClient registration and local host save, confirm()
published PairingState with the consumed invitation still attached. Foreground
subscription, errors and reconnect state copies retained that token-bearing
object even though they needed only the saved host. It was never cached and
this was not a demonstrated token leak, but unnecessarily prolonged its screen
state lifetime and kept invitation metadata in the synchronization UI.

Successful registration now publishes invite=null. Failed/incomplete/unknown
pairing retains its existing diagnostic behavior. The accepted-device notice
is outside the invitation block and requires registered=true plus a host;
accepted/connected screens display the saved address without invitation/SAS
controls. PairingScreenTest exercises both states with no invitation.

PhysicalQrHostTest observes the actual production ViewModel after manual camera
scan/SAS and saved host registration, requires matching host/registered state
and invite=null, and emits physical_qr_invitation_retired_verified before the
ungranted projection check. The physical verifier requires the phase in exact
order and refuses even an internally consistent old phase list. The scanner
UI suite similarly requires registered_host_without_invite_verified exactly
once. Existing named test counts remain unchanged.

This removes a state reference, not a promise of JVM memory erasure, Android
Intent scrubbing or recall of already transmitted data. Unknown remote outcomes
remain unknown; no key/cache/owner/ACL or reconnect protocol was weakened.

## Verification

Commands ran from /Users/mac/Github/zenpi, all handles terminal:

- zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-retire-invite-interop-20261004: exit 0, main/unit/
  instrumented source compile, 593 ordinary JVM tests and every explicit
  Rust/Kotlin lane pass, including current terminal QR, software P256 proof,
  workspace/worktree/session restart, ACK, approvals, PTY, model/resources/
  files/diff/history/workers and real-owner prompt/worker cancellation.
- Scanner verifier tests 18 pass; physical verifier tests 10. Full Python
  Stage 2 tools 165 pass, lifecycle validator fixtures 25, Blueprint fixtures 5.
- Compose and physical production callback assertions compiled, NOT executed
  on a connected device. This distinction remains required for acceptance.
- cargo fmt --check, git diff --check and strict Blueprint pass; 36 [ ], no
  [_]/[x]. Controller status has zero active/live/authenticated claims,
  outstanding requests or queued handoffs.
- Debug attribution rebuilt after Cargo lanes, debug/release verify-only pass.
  Historical current-source independent release comparison verifies against
  unchanged attributed release. No Rust production source changed this turn.
- APK verify-only still refuses missing provenance. Physical device query
  remains unanswered; no camera/LAN/API31/34/35 acceptance claim.

```text
PairingViewModel.kt 3a8c843a09628add66376b61f3af71120887d2a4dd455dbf70a9a09f7aa6775e
MainActivity.kt 94c8ad574ea080dcc2fd81e6a96f7781b1c70943d2186b714e093d562e31c24d
PairingScreenTest.kt 8aa6dc2479968c9472c34c02907a8c7c50e8d44f725a5bd0d31862899d61a92b
PhysicalQrHostTest.kt feb7364abafe40ed00beffa7c83ed2c000d9ab3b71ac6faf5ffa6c1cd81e978d
scanner-harness fcc1fbd1d3f767caf52c9543905688739584ec29c9252fb554b39b56c7c80b14
physical-harness 946f2e01023a755bbb852181896678c5ae7544dfb5c6e09126a9d47637ec0590
main.jar 04d4edd8914a30a175baf2b9b2dd3f7912daa712a7a6d70fe4216b4b4d109cc9
unit.jar 53579dbfbcf54a05b16f56427cf769a0276f01dda11463a5ed8d1aa958b6d946
instrumented.jar c4e4548778176b5a7d85f4a06cefd7d25b3656008ff36d34d00ebc0592b0c51a
debug-receipt d46f56fb1c30587e842c9b08eeb5e1821c80a1ad8209725e3c60d93b893dc1be
```

Goal active/incomplete. Current APK/signing/install/upgrade, real camera QR and
direct-LAN bidirectional synchronization/recovery, connected API31/34/35,
supported-host runtime and full unsuppressed Rust/network gates remain required.

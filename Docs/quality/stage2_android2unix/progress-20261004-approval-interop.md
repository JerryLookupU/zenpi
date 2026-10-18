# Android Approval Command Interoperability

Provisional S2-019/S2-036/S2-052/S2-054 progress. The previous goal turn
completed authentication-frame evidence and was progress. This turn reread
the execution skill, current dirty integration, Blueprint/specification,
Android foreground/background read recovery, cache commit policy, approval
projection/command/receipt code and Rust approval rendezvous/receipt tests.
No nested agents, checkbox promotion, destructive reset, commit or push.

## Change

Added remote_android_approval_interop and AndroidRustApprovalTest, wired as
an explicit source-checker lane rather than a silently assumed JVM test.
Both allow and deny run through the current Rust coordinator and the current
Kotlin command constructor. Kotlin decodes the actual host listing/snapshot,
selects its pending approval and emits the length-prefixed WorkspaceCommand.
Rust decodes it and checks project/session before host dispatch.

Wrong digest, owner, revision and a control-only device grant are refused
without answering the coordinator. Approval authority is independent from
general control authority. The valid answer succeeds even while the Agent
mutex is held, wakes the real request_response waiter, and binds the accepted
audit attribution to device/project/session/request/digest. The fixture checks
the attribution and clears the accepted marker; it does not append an audit
to the session or perform the guarded tool effect.

Exact retries return the same accepted response. An opposite decision using
the same request ID is refused. A restored host returns the original response
from its single durable approval receipt without checkpoint/journal changes.
Raw tool arguments are absent from the listing and checkpoint. Kotlin reads
the actual fragmented host frames and refuses correlated failures; mutation
of any accepted request/target/project/session/digest/decision field fails
receipt validation. A refreshed host listing has no pending approval.

## Verification

Cwd /Users/mac/Github/zenpi, HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b, dirty integration preserved.
Final verification observed 2026-10-03T21:13:20Z (Oct 4 local time).

- zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-approval-interop-20261004-final: exit 0 under a zsh
  pipefail/rg output filter. Fresh six-case Rust QR generation, actual Rust
  wire fixture, Kotlin main/unit/instrumented source compilation,
  474 ordinary JVM tests, and current read/rename/P256/worktree/session/ACK/
  approval lanes pass. Approval lane runs four JVM invocations: generate and
  validate for each decision. The final source includes altered-receipt
  negatives and coordinator audit-attribution assertions. Earlier preliminary
  source run also passed; final hashes below are authoritative for this report.
- env ZENPI_HOME=/private/tmp/stage2-approval-regression-20261004 rtk cargo test
  --offline --locked --test remote_workspace --test remote_approval_wakeup
  --test remote_auth --test remote_security --quiet: exit 0, 97 pass.
- Existing remote_approval_one_shot_decision_is_digest_fenced_durable_and_separately_authorized:
  separately passes, 1 test / 71 filtered out, not added to the 97 count.
- Python Stage 2 discovery: 132 pass; lifecycle validator tests: 23 pass.
- Rust format check, git diff --check and strict Blueprint pass; 36 rows [ ].
- Current debug host rebuilt after final Cargo/source tests, then separate
  verify-only succeeds. Source attribution only, not release acceptance.
- APK verify-only still refuses missing provenance. No Gradle/socket bypass,
  stale APK install, physical camera/LAN or connected Android run.

```text
remote_android_approval_interop.rs b76c4e091a3f060b70421d3809a2bf19fbc81c9e215974557ad112e22b07c458
AndroidRustApprovalTest.kt 61f147099a3380eff3aa668670772949d875931ac372bab94c22e0d0f15193ff
stage2_android_source_check.sh 3497b4a90e782e3a6a3916a5356967ac0d1aaa6d6432d45ae48bea8e709f62ff
main.jar d11429817490daef47383be790488fb2dad445c406e07ac91121277872d4f7c9
unit.jar c0f28760d6bbbab14be77de65d8a65555d33959f0ddf53e9f3f3256bd2bd58af
instrumented.jar 8c22b80093a6d240fc71428f32893dddb9b7a3c7fe1e5b4efaff04ca507bba8d
debug-host-receipt 46b989c1eee7040cfb0db615b1292ad19f91e1ea57b522a4f278d6a855fe9e10
debug-host-binary 677c02131aa670e18cd3813bb1a8016026b375a3155f9c010b0c62f29d287df4
```

Goal remains active and incomplete. This lane proves a coordinator/command
boundary, not provider/tool execution, persisted session audit, UI, TLS,
Keystore or physical LAN. Current APK, real camera/direct-LAN two-way desktop
synchronization/recovery, API31/34/35 lifecycle, complete parity, migration/
backup/uninstall and signed release still require acceptance evidence.

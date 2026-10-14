# Stage 2 Post-Dispatch Delivery Outcome And Authority

Recorded 2026-10-04T08:35:03Z; final host rebuild completed afterward.
Partial S2-032/S2-038/S2-052/S2-054 evidence, not acceptance. Previous goal
turn progressed equal-selection model metadata synchronization. Canonical cwd
/Users/mac/Github/zenpi; dirty HEAD 1a8ff1acf72e0816fad503a05df2b1d18e07361b.
Existing changes preserved; no nested agents, reset, push or permission bypass.
Blueprint SHA-256 d76e70e369f11f0fd3d1905d56d79c2a89401b92215da48ad85314f390d96d97;
spec SHA-256 403e6339b0318a420d93600fc52926acf6a9e46ec899bd9a9edb03c235190a28.
All 36 authoritative rows remain [ ]; zero [_]/[x].

## Finding And Change

The production Rust listener correctly returned null-result unknown_outcome
with a typed error code when dispatch might already have committed but a later
delivery authorization check failed. Android ResponseReader refused every
non-failed response with a code before HostClient outcome classification.
The write stayed unresolved, but its authorization-loss code was discarded as
a generic wire error, so stale in-memory controls could survive that failure.

WorkspaceOutcomeCodec permits only typed writes with unknown_outcome and null
result through this code check, after normal response correlation/routing.
HostClient raises WorkspaceUnknownOutcomeException carrying the optional code;
no confirmation callback or pending-intent resolution follows. AuthorityLossPolicy
recognizes unauthorized/revoked_device/owner_mismatch on this exception, strips
old controls and retains exact pending identity. Busy remains availability
ambiguity rather than revocation or no-effect proof. Coded unknown steering
responses stop receipt polling; ordinary uncoded durable steer reservation
polling remains unchanged. Unknown self-revoke also cannot confirm a local wipe.
Read/subscription responses, object-valued coded unknown results and successful
responses with error codes retain their refusal behavior.

Extracted the listener's unchanged post-dispatch envelope construction to
HostAdapter.delivery_refusal, used by production and tests. Rust security test
roundtrips Busy/Unauthorized/RevokedDevice for writes and reads: writes remain
UnknownOutcome, reads Failed, both correlated with null result. Current desktop
workspace lane actually commits each successful command, then generates all
three delivery refusals through that same constructor. Kotlin decodes fragmented
frames with the shared outcome policy, checks typed authority fencing and exact
pending identity. Existing duplicate/restart and unchanged-checkpoint assertions
remain. Those generated frames simulate delivery refusal after actual commit;
they do not execute a TCP authorization race or an Android encrypted-store race.

## Verification

All production/test edits completed before:
zsh tools/stage2_android_source_check.sh /private/tmp/stage2-delivery-outcome-20261004.
Canonical cwd, exit 0; current main/unit/instrumented compilation, 577 ordinary
JVM tests and every explicit current Rust/Kotlin lane pass. Includes committed
workspace delivery outcomes, six-test desktop target, current terminal QR,
software-P256 pairing, session/ACK/restart and actual Agent prompt/tool/running-
worker cancellation. OUT/per-lane homes isolated; credentials unchanged.
Instrumented sources compiled only, not device-run. The live handle was polled
through authoritative exit rather than restarted on silent observations.
Final log /private/tmp/stage2-delivery-outcome-20261004.log.

- cargo test --locked --offline --test remote_security: exit 0, 13 tests under
  ZENPI_HOME=/private/tmp/stage2-delivery-security-home-20261004;
  /private/tmp/stage2-delivery-security-20261004.log. Focused new test also passed.
- python3 -m unittest discover -s tools -p 'test_stage2_*.py': exit 0, 137 tests;
  /private/tmp/stage2-delivery-outcome-python-20261004.log.
- python3 tools/test_validate_stage2_android_lifecycle.py: exit 0, 25 verifier
  fixtures, not connected API31/34/35 acceptance.
- python3 tools/test_validate_stage2_android2unix_blueprint.py: exit 0, five tests.
- cargo fmt --check, git diff --check and strict Blueprint: exit 0; 36 [ ],
  zero [_]/[x]. Authority validity is not product completion.
- python3 tools/stage2_host_build.py and --verify-only: exit 0 after final Cargo
  lanes; changed production Rust source attributed to the rebuilt debug binary.
  Not release reproducibility acceptance.
- python3 tools/stage2_android_build.py --verify-only: exit 1, missing APK
  provenance. No unchanged Gradle/socket/ADB retry, stale APK install or bypass.

```text
host_pair.rs 24ee74a9e5919a580f32c85e128a6c5b7f6f153ab8b4128c9f7e58c891323eff
host_adapter.rs bf4fbb42efdc66cb3a5f6f6c8beb8262c4b913113cfcdb3f5c6cd1f8fb1b4401
remote_security.rs aa99ac42054cdad190bb14671c80dab4b0e6f4dd932bc73c43aaababf9fe2081
remote_android_workspace_interop.rs 059cb4079b914060dc1c1a5ed8846108e3563cc8d2609f0a8eaec50354c1f4f3
FrameCodec.kt f3ba814c854db77aada218e19395cf1bab6b1212ea779a90e9a54dd48708b9dc
ResponseReader.kt d70e710be4239031a5cb36ea2c5ab3ed394bc13aca1c9e6c05918f47ed57639b
HostClient.kt e33202af3a9904313b82a6d1da4f1b061cbfc455c48a15fd258a95da057be126
AuthorityLossPolicy.kt b2b24c360cbbf081548939e9b8e9c5f7748f3ae944ce0cb650e9ed8a87fd17cc
WorkspaceUnknownOutcomeTest.kt 937da7e3e55e021940d1455c53d3592ff18792ec6aba6e977d9840c628813204
AndroidRustWorkspaceTest.kt 00d8d04586bd61da1bccee67f18d966db8da4369d3595f0a7282ae939921504b
source-log a4af74140a7d106fb7f75fff832b4ecb7cb45adc5b1db29561de4a7e4a6c48c4
python-log 841b16f6fde1e698a5d438fa467b21e20b9d7aaafb2a565bbb2ccb1d3e03c5f8
security-log 7dcec6ead05604702fb2ed2f10b9d1b417f42e3e130eab456c3a0153656cb48d
workspace-results 873351b0ba88a8f4376b0142f5ced6663827595c6f09e287c2613d6a773f22e9
main.jar 8fff0437e93d09f1f9016bfe186f1048f79e4d09694520c45acc7bd87ec70e80
unit.jar 8650e09bdcd79af2176e47904680e8bdb1c92465a5ca2909c272f5ccc91a2ea9
instrumented.jar 8e803880dd04f03ad420e12d119b9cc49d3b584ef2669dc0aec78bae659de1d0
host-receipt 81517bfa74e3413beb38ab34e8a0dc575ca9b7144e2c8b9b0fb7fcf9f07b669f
host-inputs 08312e22d68633b57228db66f7d64511bc87dc758579124ef9de588e3a466219
host-binary 40f9f134ed62fd51a71dc6e8d24022f0122dced68faf12f6a52f8b5ea145607f
```

Goal active/incomplete. Fresh APK provenance, physical camera QR/direct-LAN
bidirectional sync/recovery, connected API31/34/35 lifecycle, complete desktop
parity and signed reproducible release remain required. This implementation/
interop repair is progress; completion remains unproven and blocked audit does
not pass while meaningful source work is available.

# Stage 2 Opaque Conversation Turn Identity Parity

Recorded 2026-10-04T09:03:43Z; partial S2-036/S2-054 evidence, not acceptance.
Canonical cwd /Users/mac/Github/zenpi, dirty HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b. Existing changes preserved;
no nested agents, reset, push, permission bypass or stale APK installation.
Blueprint SHA-256 d76e70e369f11f0fd3d1905d56d79c2a89401b92215da48ad85314f390d96d97;
spec SHA-256 403e6339b0318a420d93600fc52926acf6a9e46ec899bd9a9edb03c235190a28.
36 [ ], zero [_]/[x]. Prior goal turn was progress: revocation identity removal
cancellation and expanded connected acceptance gate.

## Finding And Change

Rust transcript/session-history reads admit opaque identifiers using
remote::auth::valid_identifier: nonblank, <=128 UTF-8 bytes, no controls.
Android SessionHistoryCodec instead required ASCII [A-Za-z0-9_-] for every
decoded and retained turn. A valid host conversation with Unicode or punctuation
could therefore fail history decode even though ordinary transcript snapshots,
detail requests and the host journal accepted its identity.

SessionHistoryCodec now shares one local identifier predicate between incoming
and retained page validation, matching the existing nonblank/UTF-8-byte/control
contract. IDs are not trimmed, normalized or turned into paths. No wire format,
ACL, journal revision, digest, continuity, duplicate, tool-summary or page-size
gate was weakened. Only the unwarranted ASCII restriction was removed.

SessionHistoryTest adds exact preserved punctuation/Unicode/128-byte IDs and
blank/control/129-byte refusals. Valid history selections construct and frame
full-message requests with exactly that turn ID without replacing the live
projection. The current Rust history interop fixture appends history:0/Unicode
to a real SessionStore, reads all 70 historical IDs through five Kotlin-generated
pages, and pages that old truncated message through the actual desktop owner.
Kotlin validates fragmented responses, full UTF-8 byte count/content digest,
redaction, previous pages and retained live scope; host journal append/revoke
refusals and unchanged read-only journal/workspace assertions remain.
This is actual owner/journal plus JVM framing, not Android Keystore/TLS/LAN/UI.

## Verification

zsh tools/stage2_android_source_check.sh /private/tmp/stage2-history-identity-20261004:
exit 0, 580 ordinary JVM tests, main/unit/instrumented compilation and all
explicit Rust/Kotlin lanes passed. The six-test model/read target includes the
changed history/detail lane; terminal QR, software P256, workspace/session/ACK/
restart and real prompt/tool/running-worker cancellation lanes also passed.
Process 82313 polled through exit 0 rather than restarted on observation timeout.
Per-lane homes isolated; no operator credentials changed.

- python3 -m unittest discover -s tools -p 'test_stage2_*.py': exit 0, 142 tests;
  /private/tmp/stage2-history-identity-python-20261004.log.
- python3 tools/test_validate_stage2_android_lifecycle.py: exit 0, 25 fixtures,
  not connected API31/34/35 runtime acceptance.
- python3 tools/test_validate_stage2_android2unix_blueprint.py: exit 0, five tests.
- cargo fmt --check initially identified one test-only formatting change; fixed
  it and final check passed. git diff --check and strict Blueprint exit 0,
  36 [ ], zero [_]/[x].
- Host --verify-only during source Cargo lanes refused a changed binary.
  python3 tools/stage2_host_build.py and --verify-only after their terminal exit
  both passed, restoring current debug attribution. No Rust production source
  changed; test fixture changed only.
- python3 tools/stage2_android_build.py --verify-only: exit 1, missing APK
  provenance. No repeated blocked Gradle/socket/ADB attempt or bypass.

```text
SessionHistoryCodec.kt 837c623664c1c239ed7244932d29676586330e40562ed16c04003801bcb3e3d7
SessionHistoryTest.kt f96e8eaec949f7e7047dd4d43515e7d78323138f18fefc7db83fbe7231deec4f
AndroidRustHistoryTest.kt fd9a30d6e7424b07fa3b35c9e8fabf175f1416057a333fa1e21243147978821f
remote_android_model_interop.rs eb1cc265f24702f7336c68592fe9a3fc3ba15f1cb65542501cad8de7ac9de14b
python-log 0cd2d37b7d10656d5dda0ca098a212bb61c5de3ad64a4a89d0feaf3cae00d01d
source-log 292a714277fcc00a97dc76e80be78c11951c3360dca085cee49666a445da17f4
history-results 1615dd0cae8fde0f9ce38deada79cb6c5f4013c42a8f3d64e6af858b485e263d
main.jar 0385b800a3a3d849ad9341c276d5ad066abf6efd327746f39a5d71e78112457e
unit.jar bac2711ed8f1c25f929ed9f22046a4cb53e32cce4d5d66c7a47731979ee990d4
instrumented.jar f95a635bb1b8d33d66febe3cf15afe0db9afd4ca9d63fb00a5f126e9d1e69630
host-receipt bdc98bf00b6990ad4225af70c310b3116062b95a035652691938e4146580a52d
host-inputs 08312e22d68633b57228db66f7d64511bc87dc758579124ef9de588e3a466219
host-binary 40f9f134ed62fd51a71dc6e8d24022f0122dced68faf12f6a52f8b5ea145607f
```

Goal active/incomplete. Fresh APK provenance, physical camera QR/direct-LAN
bidirectional sync/recovery, API31/34/35 connected lifecycle, complete parity
and signed reproducible release remain required. Source interoperability repair
is progress, not completion; meaningful work remains so no blocked status.

# Stage 2 Pairing Handshake Identity-Read Cancellation

Recorded 2026-10-04T08:14:59Z. Partial S2-032/S2-037/S2-038/S2-052
evidence, not acceptance. Previous goal turn progressed mapped LAN route
selection and actual desktop search interoperability. Canonical cwd
/Users/mac/Github/zenpi; dirty HEAD 1a8ff1acf72e0816fad503a05df2b1d18e07361b.
User changes preserved; no nested agents, reset, push or permission bypass.
Blueprint SHA-256 d76e70e369f11f0fd3d1905d56d79c2a89401b92215da48ad85314f390d96d97;
spec SHA-256 403e6339b0318a420d93600fc52926acf6a9e46ec899bd9a9edb03c235190a28.
All 36 authoritative rows remain [ ]; zero [_]/[x].

## Finding And Change

PairingHandshake.current checked cancellation before its identity callback,
but not after the potentially blocking Keystore read. Cancellation during
that read could allow hello, signing, token transmission or successful local
registration return before a later caller check. Socket cancellation and
ViewModel storage checks were separate safeguards, not a replacement for
handshake-stage ownership admission.

The shared handshake check now brackets every identity read with activity
checks. The new regression cancels inside each of the five reads: admission,
post-welcome, post-challenge, post-signing and post-receipt. It asserts the
exact cancellation exception, no local publication, exact signing count and
exact transmitted frames. Reads one through four never transmit the token;
read five has already sent one pair and must not claim remote rollback or
resend. Existing cancellation coverage now exercises all eight pre-send
activity checks. The late-receipt cancellation fixture uses an actual receipt
arrival marker rather than a brittle callback count. Expiry-after-accepted-
receipt compatibility and the one-use unknown-outcome policy are unchanged.

Android/protocol documentation and network/parity/security matrices record
the boundary. JVM hooks do not prove a blocked real Keystore read or physical
network cancellation; no instrumented/device acceptance is promoted.

## Verification

Production/test edits completed before launching:
zsh tools/stage2_android_source_check.sh /private/tmp/stage2-pair-read-fence-20261004.
Canonical cwd; exit 0. Current main/unit/instrumented compilation, 574 ordinary
JVM tests, regenerated current Rust terminal QR fixtures and all explicit
current Rust/Kotlin lanes pass. Includes actual software-P256 pairing proof
verification, session/ACK/restart, six-test desktop target and actual Agent
prompt/tool/running-worker cancellation. Instrumented assertions compiled
only, not device-run. OUT/per-lane homes isolated; credentials unchanged.
The live process handle was polled to authoritative exit 0, not restarted
on a silent observation. Final log /private/tmp/stage2-pair-read-fence-20261004.log.

- python3 -m unittest discover -s tools -p 'test_stage2_*.py': exit 0, 137 tests;
  /private/tmp/stage2-pair-read-fence-python-20261004.log.
- python3 tools/test_validate_stage2_android_lifecycle.py: exit 0, 25 verifier
  fixtures, not connected API31/34/35 lifecycle evidence.
- python3 tools/test_validate_stage2_android2unix_blueprint.py: exit 0, five tests.
- cargo fmt --check, git diff --check, strict Blueprint: exit 0; 36 [ ],
  zero [_]/[x]. Strict metadata validity does not establish product completion.
- python3 tools/stage2_host_build.py and --verify-only: exit 0 after source
  checker completion; debug host reattributed. Production Rust inputs unchanged.
  Verification during Cargo lanes had refused a relinked binary; the final
  receipt was built only after those lanes ended. Not release acceptance.
- python3 tools/stage2_android_build.py --verify-only: exit 1, missing current
  APK provenance. No unchanged Gradle/socket/ADB retry, historical APK install
  or permission bypass.

```text
PairingHandshake.kt 83f1a8cca0238bcf50047248d377e12791c1e68341f22be8be3e2accd805d91e
PairingHandshakeTest.kt 7e9c8292ce2e1aaa15a1da9d56df15dbc3cd0084e147c51b1b36ce9b1acd54b5
source-log 4dbb0a8802d367f5ed64705d34eea1bf94ee3f1420e213dfd38cf23bf35ca8a6
python-log 626fefc981e96823cf1c14afb2d8fe055ecff1eab09548dd7baeb18442ace636
main.jar 218291f7486295622dd81c0818e8ff2199c9913db6a1acf656fcf204268bbe38
unit.jar ac369402c7633e57b7eea5330c4a5921e912208f64bb073307933dbe80d947a2
instrumented.jar 7d5296e147572595b9301eff81d6a857db7eb1ee2ee0e4e1faf063576505c80a
host-receipt baf64502bf5338aea6b3764f5d638167d888560c7ff544cf2f44912101791d00
host-inputs 9971547e0a69a38c9a30512ee37340ab10553825f20ae10d253c5c600e7faedd
host-binary 8c8dfc56ed6956ace031c8682d713b4c917262faba1ec6a60375e3f219532b89
```

Goal active/incomplete. Fresh APK provenance, physical camera QR/direct-LAN
bidirectional synchronization/recovery, connected API31/34/35 lifecycle,
complete desktop parity and signed reproducible release remain required.
The current source fix is meaningful progress; no completion or blocked audit
passes merely because those runtime gates remain unavailable.

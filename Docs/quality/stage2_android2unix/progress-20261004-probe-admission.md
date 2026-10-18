# Stage 2 Automatic Recovery Probe And Host Audit

Recorded 2026-10-04T07:39:38Z. Partial S2-032/S2-038/S2-052/S2-053
evidence, not acceptance. Prior goal turn progressed retained replay cursors.
Canonical cwd /Users/mac/Github/zenpi; dirty HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b. Existing changes preserved;
no nested agents, reset, push or permission bypass. All 36 Blueprint rows remain
[ ]. Blueprint SHA-256 d76e70e369f11f0fd3d1905d56d79c2a89401b92215da48ad85314f390d96d97;
spec SHA-256 403e6339b0318a420d93600fc52926acf6a9e46ec899bd9a9edb03c235190a28.

## Finding And Change

SessionChangeProbe recognized automatic read operation names but did not validate
the entire refused request, exact authority-free envelope or empty inspect/
subtab-list arguments. Malformed/authority-bearing input could therefore start
a fresh listing and classify an owner refusal as recoverable session change.

Probe admission now requires exactly schema/channel/id/project/session/payload,
valid nonnull correlation ID and complete FrameCodec validation. Only ACK/resume
or empty-argument inspect/subtab_list automatic requests may probe. Noncanonical
cursor, foreign cursor session, missing/null/invalid scope or extra authority/
turn routing preserves the original refusal with zero new exchanges. No command
retry, capability expansion or replacement identity is introduced.

Added SessionChangeProbeTest eleven-case no-probe request matrix plus explicit
path-argument refusal. Existing fixtures now use complete valid frames instead
of type-only placeholders; ResumeReadPolicyTest keeps its authorized one-probe
fresh-snapshot race with a current valid resume frame.

## Verification

Host audit 2026-10-04T07:31:49Z through 07:36:07Z exited 0:
env ZENPI_HOME=/private/tmp/stage2-host-recovery-audit-20261004/home
cargo test --locked --offline --test remote_faults --test remote_subscription_tls
--test remote_subscription --test remote_security --test remote_events
--test remote_auth --test remote_protocol --test remote_frame.

Eight targets: auth 12, events 9, faults 3, frame 10 (+1 ignored real-loopback
timeout), protocol 18, security 12, subscription 17, encrypted subscription 43:
124 passed, zero failed, one ignored. The TLS target uses real rustls encryption
over fragmented bounded in-memory channels, not TCP/phone/LAN. Faults are
controlled I/O boundaries, not physical network change. No ignored test promoted.

Source run 2026-10-04T07:32:27Z through 07:39:27Z exited 0. Source was finalized
while waiting for the host audit's Cargo build lock, before Kotlin compilation;
no later Kotlin edits. Both original handles were polled to terminal without
timeout-based restart. ps observation was denied by sandbox and not bypassed.
Canonical cwd; OUT and per-lane ZENPI_HOME isolated under
/private/tmp/stage2-probe-admission-20261004; credentials unchanged.

- zsh tools/stage2_android_source_check.sh /private/tmp/stage2-probe-admission-20261004:
  current main/unit/instrumented compilation, 571 ordinary JVM tests, production
  QR corpus and explicit current Rust/Kotlin lanes pass; six-test desktop target
  and actual Agent prompt/tool/running-worker cancellation. Instrumented assertions
  compiled only, not device-run.
- python3 -m unittest discover -s tools -p 'test_stage2_*.py': exit 0, 137 tests;
  /private/tmp/stage2-probe-admission-python-20261004.log.
- python3 tools/test_validate_stage2_android_lifecycle.py: exit 0, 25 verifier
  fixture tests, not connected API acceptance.
- python3 tools/test_validate_stage2_android2unix_blueprint.py: exit 0, five tests.
- cargo fmt --check, git diff --check and strict Blueprint: exit 0;
  36 [ ], zero [_]/[x].
- python3 tools/stage2_host_build.py and --verify-only: exit 0 after Cargo lanes;
  current debug host reattributed, production Rust sources unchanged.
- python3 tools/stage2_android_build.py --verify-only: exit 1, missing current
  APK provenance. Unchanged Gradle/socket/ADB constraints not retried or bypassed;
  no historical APK installed.

Logs /private/tmp/stage2-host-recovery-audit-20261004.log and
/private/tmp/stage2-probe-admission-20261004.log.

```text
SessionChangeProbe.kt d7b8c8e30b6af6c7916623fc98079652799fd029474cdcb4d6d81b35029a2da6
SessionChangeProbeTest.kt cb5eebffb2c194bbe2004486bae2fc251b1f6109ae453a8eb65ef062fe105900
ResumeReadPolicyTest.kt 0fe445c51eacf42d0864a99edad269aeac1095674c2ddd48c7d0423839bd9178
source-log e32f47e3ebda14e58483d9196abcdcb77da1649400c22e057e01514510c56304
host-audit-log b2b4902d352682840a8c10ffba905053f9c506643e65322717654402ee6df933
python-log 64f1f5bbc273a4df7c8935cb527eabc794e86d35d183d2ea63b918679f471c76
main.jar ad913372812bff8baaada820a894b3042d898971f95da888392456b8c9c191ab
unit.jar fd2f34c8c9476d80ba92f58eed7c355efa8047e7c43e220af825cc28b036e55b
instrumented.jar 7d5296e147572595b9301eff81d6a857db7eb1ee2ee0e4e1faf063576505c80a
host-receipt e268c7ec4fea9720af076e5b2532fdcfcb02551b826af60f44a5fc6213692a42
host-inputs 373a8f5697a5bc95c10eda94c9b9b35facda736cd84bcdb0c0fa3e8703c8e58f
host-binary 2939439799b49d0e22f6072e95288a44b239e4d8eb999ad9f44bfaaf5480a861
```

Goal active/incomplete. Current APK, physical camera QR/direct-LAN
bidirectional sync/recovery, connected API31/34/35 lifecycle, complete desktop
parity and signed reproducible release remain required product gates. Prior
same-LAN phone question remains unanswered; it does not block source progress.

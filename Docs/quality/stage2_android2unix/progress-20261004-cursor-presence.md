# Stage 2 Known Resume Cursor Preservation

Recorded 2026-10-04T08:46:41Z; host smoke observed immediately afterward.
Partial S2-033/S2-037/S2-038/S2-053 evidence, not acceptance. Canonical cwd
/Users/mac/Github/zenpi, dirty HEAD 1a8ff1acf72e0816fad503a05df2b1d18e07361b.
Existing changes preserved; no nested agents, reset, push or permission bypass.
Blueprint SHA-256 d76e70e369f11f0fd3d1905d56d79c2a89401b92215da48ad85314f390d96d97;
spec SHA-256 403e6339b0318a420d93600fc52926acf6a9e46ec899bd9a9edb03c235190a28.
All 36 authoritative rows remain [ ]; zero [_]/[x].

## Prior Evidence And Finding

The previous goal implementation turn made progress on coded post-dispatch
unknown write outcomes. Its source process handle was missing at this turn's
first poll; the final log and artifacts matched the delivery-outcome report,
including its final running-worker cancellation lane. Current host attribution
also matched that report before rebuilding. It was not restarted as a live job.
The intervening RTK request verified an already-installed tool, not new Android
product behavior.

ProjectionCommitPolicy compared event sequences only when both were present.
A same-project/session snapshot with the optional legacy event cursor omitted
could therefore pass the shared live-publication freshness admission despite
discarding the known resume position. The encrypted cache codec independently
requires a cursor and would refuse encoding, so this is not evidence of a
completed end-to-end cache-loss exploit. That existing storage check did not
make the shared publication policy correct. This repair rejects the missing
cursor at the common boundary rather than relying on a later encoding failure.

The policy now requires a next event sequence whenever the previous same-scope
projection has one, independently of optional journal metadata. Malformed and
regressed counters retain their existing refusal behavior. First attach to a
legacy snapshot, upgrade from no cursor to a known cursor, and different
project/session scopes remain valid. No wire/cache schema changes, manufactured
cursor, automatic write retry or authority grant were introduced.

ProjectionCommitPolicyTest adds null/equal/advancing journal and legacy/scope
compatibility cases, with visible/durable state unchanged on refusal.
RustAndroidWireTest removes event_cursor and event_cursor_scope from the actual
current Rust adapter's initial snapshot, then checks that first-attach decode
remains compatible but a known saved cursor cannot be replaced. This is a
mutated real adapter frame, not a production TLS downgrade exercise.
ResumeStoreTest extends foreground save, background CAS and display-bearing
workspace completion refusals to this case, asserting unchanged encrypted
bytes and exact unresolved command identity. Instrumented sources compile;
these Keystore/AtomicFile assertions have not been device-executed.

## Verification

All changed Kotlin source/tests preceded:
zsh tools/stage2_android_source_check.sh /private/tmp/stage2-cursor-presence-20261004.
Exit 0, 579 ordinary JVM tests; main/unit/instrumented source compilation and
all explicit Rust/Kotlin interop lanes passed, including current terminal QR,
software P256 proof, primary workspace writes, session/ACK/restart, model and
resource/file/diff/history lanes, actual prompt/tool and running-worker cancel.
The initial fixture target ignores two special tests; the script then runs
each explicitly with the current classes. Required session 71766 was polled to
authoritative exit 0. OUT and per-lane ZENPI_HOME roots were isolated.

- python3 -m unittest discover -s tools -p 'test_stage2_*.py': exit 0, 137 tests;
  /private/tmp/stage2-cursor-presence-python-20261004.log.
- python3 tools/test_validate_stage2_android_lifecycle.py: exit 0, 25 fixtures,
  not connected API31/34/35 behavior.
- python3 tools/test_validate_stage2_android2unix_blueprint.py: exit 0, five tests.
- cargo fmt --check, git diff --check and strict Blueprint: exit 0, 36 [ ],
  zero [_]/[x]. No checklist acceptance from source-only evidence.
- python3 tools/stage2_host_build.py and --verify-only: exit 0 after all Cargo
  lanes; current debug binary provenance verified, not reproducible release.
- python3 tools/stage2_android_build.py --verify-only: exit 1, missing APK
  provenance. No stale APK installed, no Gradle/ADB bypass attempted.
- python3 tools/stage2_host_smoke.py --receipt
  /private/tmp/stage2-cursor-presence-host-smoke-20261004.json: exit 1, host
  socket blocked by sandbox before invite emission. This command defaults to
  target/release/zenpi, not the attributed debug binary above. It is failed
  runtime evidence, not source/QR/LAN acceptance; no equivalent retry followed.

```text
ProjectionCommitPolicy.kt e9b3678144c8e8f144d453fc677beecf0a1349585318698c311b8f3079ecc08d
ProjectionCommitPolicyTest.kt e9b0e7f5621577e87365398d7be833ab4650c5adc888f9fde38892baf706df36
RustAndroidWireTest.kt 5ab892e567a60c061a5feb5755791a3a57d5eb8b76f9a719c9e4f1f467036276
ResumeStoreTest.kt b392aeb2e652c55859147af10ab0a30568d6c77c135935bcb524860d7ef7a58a
source-log ed15a587e6f52cff3cdde22b9bc7934f99906f11bd578625f8647def1c8fbdf2
python-log fd468c5f73e1a31166252d88a8401da1505dce7a1fa6596524f253529b8ee837
main.jar 1d65c5ade9679f6abb4e4f65e9f87538d8f55292879633db97aeb837d7a9378f
unit.jar 3c3ed270a9f7b30654e593cbb39f977524a2d3d52e535d9ff6796bc4ac72c917
instrumented.jar f781b1c7cc72fe85b90243238fe43b8e75883fe3128b6ed082be84a12d8b9f59
rust-wire-fixture 780376d2679b71d806ff6f6168af3cbe63980774aca98ce84e3b0b5f93a9ff73
host-receipt f33a4a57f981e8b4dfd88432fde91b96c28ec189d5f6a1c9ffd6c0804957cffa
host-inputs 08312e22d68633b57228db66f7d64511bc87dc758579124ef9de588e3a466219
host-binary 40f9f134ed62fd51a71dc6e8d24022f0122dced68faf12f6a52f8b5ea145607f
failed-host-smoke fe4e048fec119cf5d80f69f795f507edc521fbe609b168e3a0b39dcfae764c12
```

Goal active/incomplete. Fresh APK provenance, physical camera QR/direct-LAN
bidirectional sync/recovery, connected API31/34/35 lifecycle, complete desktop
parity and signed reproducible release remain required. The same environment
limits are not completion evidence or a reason to discard meaningful source
work. No completion or blocked status is asserted.

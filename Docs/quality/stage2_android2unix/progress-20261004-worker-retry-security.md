# Stage 2 Recovered Worker Requests Preserve Digest And Authorization Fences

Recorded 2026-10-04. Partial S2-019/S2-035/S2-052/S2-054 evidence. Previous
goal turn added actual Kotlin worker submissions; this turn adds recovered
request conflict/revocation evidence on that real Agent lane. Canonical cwd
/Users/mac/Github/zenpi, dirty HEAD 1a8ff1acf72e0816fad503a05df2b1d18e07361b.
Existing changes preserved; no nested agents, reset, push or permission bypass.
All 36 Blueprint rows remain [ ]. Blueprint SHA-256
d76e70e369f11f0fd3d1905d56d79c2a89401b92215da48ad85314f390d96d97;
spec SHA-256 403e6339b0318a420d93600fc52926acf6a9e46ec899bd9a9edb03c235190a28.

## New Evidence

After current Kotlin submitted two actual Agent tasks, targeted cancellation,
sibling success and fresh disk Agent/pool exact receipt recovery, the same
test now changes each original submit payload while retaining its ID. Both
return RequestIdConflict. Changing original cancel's target to sibling with
the original cancel ID also returns RequestIdConflict; a panic callback proves
the conflicting retry cannot invoke cancellation.

Then the fixture revokes the original device through production DeviceRegistry.
Its two original accepted submit envelopes and exact accepted cancel envelope
all return Unauthorized despite available durable receipts. No worker owners
are prepared, backend starts remain two, cancellation observations remain one,
and session journal bytes remain unchanged. Current Kotlin validates all six
correlated refusal frames and their exact codes alongside previous accepted
receipts/state assertions. No production runtime or permission policy changed.

Controlled local backends and software identity fixtures are not external
providers, Android Keystore, OS process death, production TCP/TLS listener,
physical camera QR or direct LAN evidence. Current APK physical revoke/retry
remains an outstanding gate, not supplied by these in-process checks.

## Verification

Commands from canonical cwd; source checker controls isolated OUT and fixture
ZENPI_HOME=$OUT/worker-cancel-home. User provider config/credentials unchanged.

- zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-worker-retry-security-20261004: exit 0, 536 ordinary JVM
  tests, main/unit/instrumented compilation, production QR corpus, all explicit
  interop lanes, six-test desktop target, real worker-tool/prompt and extended
  worker submit/cancel/security lane (one Rust test/four Kotlin phases).
  No source/test edits after this run; instrumentation compilation is not
  connected-device execution.
- python3 -m unittest discover -s tools -p 'test_stage2_*.py': exit 0, 137 pass;
  /private/tmp/stage2-worker-retry-security-python-20261004.log.
- python3 tools/test_validate_stage2_android_lifecycle.py: exit 0, 25 pass;
  fixture verifier tests, not connected API matrix.
- cargo fmt --check, git diff --check and strict Blueprint: exit 0.
- python3 tools/stage2_host_build.py and --verify-only: exit 0 after Cargo lanes;
  cfg(test) source change updates attributed host inventory/build identity.
- python3 tools/stage2_android_build.py --verify-only: exit 1 missing current
  APK provenance. Unchanged Gradle/ADB permission blockers not retried/bypassed.

Final log: /private/tmp/stage2-worker-retry-security-20261004.log.
Frames/security evidence: /private/tmp/stage2-worker-retry-security-20261004/android-worker-cancel-results.json.

```text
worktree_host.rs a6e487fcd13695b80749a126269e86c6a3c95c3e5cc46e7fc1d683ce48b0efad
AndroidRustWorkerCancelTest.kt 9c152739bc8e9fa276c76345a4bd9d2ec657da79857479794c6e73ae80239863
source-log 8bb72c8d0a7d63f27e4e590660e2f8fefc785b77a556862852c48bded78ac376
worker-security-results 289272a480c090c8d8bf45d6bce4d56dfc4fe9086b2092e0a78906d9626e0fa7
python-log 7345114bb658b206ce7aa6f5ca491baff4cef03a92659e1759c9e19e4529050a
debug-host-receipt ecfbf0aea2a81f32c1dcfb2714a36d155c1789b529562a4cb3f63644ae1ea424
debug-host-inputs 373a8f5697a5bc95c10eda94c9b9b35facda736cd84bcdb0c0fa3e8703c8e58f
debug-host-binary 2939439799b49d0e22f6072e95288a44b239e4d8eb999ad9f44bfaaf5480a861
```

Goal active/incomplete. Current APK, physical QR/direct-LAN bidirectional
sync/recovery, connected API31/34/35, complete desktop parity and signed
reproducible release remain required; recovered-request security is partial
evidence, not substitute product acceptance.

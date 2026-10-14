# Stage 2 Retained Terminal Projection Integrity

Recorded 2026-10-04T06:15:55Z. Partial S2-036/S2-054 evidence, not acceptance.
Prior goal turn progressed retained file paging; this turn changes terminal
projection admission. Canonical cwd /Users/mac/Github/zenpi; dirty HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b. Existing changes preserved; no nested
agents, reset, push or permission bypass. All 36 Blueprint rows remain [ ].
Blueprint SHA-256 d76e70e369f11f0fd3d1905d56d79c2a89401b92215da48ad85314f390d96d97;
spec SHA-256 403e6339b0318a420d93600fc52926acf6a9e46ec899bd9a9edb03c235190a28.

## Finding And Change

The wire decoder validated terminal metadata, but current-terminal resolution
and Compose display trusted retained screen text, instance ID and geometry.
Modified retained data could preserve history selection or enable construction
of fresh input/resize commands despite failing the decoder's shape rules.

TerminalProjectionCodec.valid now checks canonical subtab/instance IDs, u64
revision, at-most-16 KiB control-free UTF-8 screen text and optional dimensions
in 1..512. Dimensions and exited lifecycle require an instance. Decoder,
TerminalPageCodec.terminal and Compose viewport share the predicate; existing
read policy, command creation, scope retention and fresh dispatch inherit it.
The read-policy fixture now uses a real canonical tab ID instead of "tab".

Expected/actual: bad retained ID/revision/text/geometry refuses history read,
selection retention, input/resize construction and fresh dispatch. Compose
fixture hides viewport/input/history for malformed retained metadata. Exact
saved pending writes still retry only their original envelopes for receipt
resolution. Valid exited-instance reads, legacy screen-only display, bounds,
output updates and host-authorized terminal-open remain supported. Current
real Rust PTY metadata is also mutated in Kotlin to prove the same refusals.
No protocol operation, local Android execution or device acceptance added.

## Verification

Final source run 2026-10-04T06:13:03Z through 06:15:42Z, canonical cwd, exit 0.
No production/test edits after launch. OUT and per-lane ZENPI_HOME isolated
under /private/tmp/stage2-terminal-metadata-20261004; credentials unchanged.

- zsh tools/stage2_android_source_check.sh /private/tmp/stage2-terminal-metadata-20261004:
  production/unit/instrumented source compilation, 550 ordinary JVM tests,
  production QR corpus and all explicit Kotlin/Rust lanes pass. Includes actual
  owned PTY input/resize/history, six-test desktop target, real Agent prompt/tool
  and running-worker cancellation paths.
- python3 -m unittest discover -s tools -p 'test_stage2_*.py': exit 0, 137 pass;
  /private/tmp/stage2-terminal-metadata-python-20261004.log.
- python3 tools/test_validate_stage2_android_lifecycle.py: exit 0, 25 fixture
  tests, not connected API lifecycle acceptance.
- python3 tools/test_validate_stage2_android2unix_blueprint.py: exit 0, 5 pass.
- cargo fmt --check, git diff --check and strict Blueprint: exit 0;
  36 [ ], zero [_]/[x].
- python3 tools/stage2_host_build.py and --verify-only: exit 0 after Cargo lanes;
  current debug host reattributed, no Rust source changes.
- python3 tools/stage2_android_build.py --verify-only: exit 1, missing current
  APK provenance. Unchanged Gradle/local-socket/ADB blockers not retried or
  bypassed; historical APKs not installed.

Final log /private/tmp/stage2-terminal-metadata-20261004.log.
PTY exchanges /private/tmp/stage2-terminal-metadata-20261004/android-terminal-results.json.

```text
TerminalProjectionCodec.kt 0b0a9121958398780afea19787dbed012b74aa57b0ea512238bf66513b84cea9
TerminalPageCodec.kt 77d0b1a36f87749d60e3a2aee5eacb50aaa9c4cd6ea67c827fd1c30935fdef4a
MainActivity.kt 5cd0ec2e1d5c90d8e3a11a80739a31d21d982534f97d25b9a971be944624afde
TerminalControlTest.kt 69ec1dc21463285c4a694ce1c75d2d53f166ee0b76c437d4a565a13bcdff783b
TerminalReadPolicyTest.kt b68830312e5de0a3bb1f144be3b935e3902022e625c753da075f108750585407
AndroidRustTerminalTest.kt 80776db3eb9a0ec099b7b86519e82e2a8c1139ca71fce568c1cb2b6556b8b3e3
TerminalControlScreenTest.kt 058cb55ab3848ae95806912c99df1e26891278d69034342509ae520c54dfc9f8
source-log 1424c9cf3e3ab76a3f0d20215aae72c7ec54f6402245786674f91e3e4103071a
terminal-results d547152902711f647408b8c04e38116248393c09e2aa10cb3fa642c8acee692f
python-log 0752e383cf6a3e1b4d58392044f3440b4e6eaafc3063078f1823085dff4e5d0f
main.jar 7e07dc53a052398bc1b6b5377298990757d914e9cf9f957e6a0e6e7ccf75e82f
unit.jar 9e1be260093d84460a8a56f483704131604d605bf0a7b306b239e9b8746a1752
instrumented.jar 5955a90c5cf2b527f435d25b838c674f79456705abfc4e0707b6036603cdef6e
host-receipt 2939e529e19e9fcebc14aed1bb26ca2f9561b1b2c0ff6245522f8ae78dbe997d
host-inputs 373a8f5697a5bc95c10eda94c9b9b35facda736cd84bcdb0c0fa3e8703c8e58f
host-binary 2939439799b49d0e22f6072e95288a44b239e4d8eb999ad9f44bfaaf5480a861
```

Goal active/incomplete. Source-bound APK, physical QR/direct-LAN bidirectional
sync and recovery, connected API31/34/35 lifecycle, complete desktop parity and
signed reproducible release remain required product gates. JVM/source/PTY
interop does not substitute for those gates.

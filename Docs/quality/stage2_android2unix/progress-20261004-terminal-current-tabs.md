# Stage 2 Terminal Operations Require Complete Current Tabs

Recorded 2026-10-04. Partial S2-036/S2-054 evidence. Prior goal turn progressed
LAN binding; this turn changes Android terminal read/control/open admission.
Canonical cwd /Users/mac/Github/zenpi, dirty HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b. Existing changes preserved; no nested
agents, reset, push or permission bypass. All 36 Blueprint items remain [ ].
Blueprint SHA-256 d76e70e369f11f0fd3d1905d56d79c2a89401b92215da48ad85314f390d96d97;
spec SHA-256 403e6339b0318a420d93600fc52926acf6a9e46ec899bd9a9edb03c235190a28.

## Finding And Change

TerminalPageCodec and terminal-open construction previously checked that the
active tab appeared exactly once, but did not reject duplicates elsewhere in
the same list or a list exceeding capacity. Fresh dispatch already used the
complete current list when commands contained subtab_revision, so construction,
read admission and display had weaker checks than dispatch.

TerminalPageCodec.terminal and TerminalCommandCodec open/input/resize now reuse
SubtabProjectionCodec.current. The Compose viewport also requires that current
scope. This consistently requires a unique selected project/session, 1..32
unique tabs and a present active ID, before using terminal instance/revision
scope. Existing capabilities, owner, journal and exact pending retry are not
changed; no new network operation or Android shell execution is introduced.

Expected/actual: a healthy main plus inactive in-place tab permits reads and
fresh open/input/resize; duplicated inactive IDs or 33 tabs refuse resolver,
history request, retained scope, construction and fresh dispatch. Saved pending
commands can still retry their exact envelopes, but changed request IDs fail.
Current Kotlin terminal fixture repeats inactive-duplicate refusal on actual
Rust PTY metadata and pages, including decoder/selection refusal; valid current
read-only pages still pass. Existing narrow Compose case now hides terminal
viewport/input/history for such a list and disables terminal-open. That case
was compiled, not connected-device executed.

## Verification

Run 2026-10-04T05:19:18Z through 05:21:53Z, canonical cwd; no source/test edits
after source checker launch. OUT and per-lane ZENPI_HOME isolated under
/private/tmp/stage2-terminal-current-tabs-20261004; credentials unchanged.

- zsh tools/stage2_android_source_check.sh /private/tmp/stage2-terminal-current-tabs-20261004:
  exit 0, main/unit/instrumented compilation, 540 ordinary JVM tests,
  production terminal QR corpus, all explicit current Kotlin/Rust lanes,
  actual owned PTY input/resize/history, six-test desktop target and actual
  Agent prompt/tool/worker cancellation lanes passed.
- python3 -m unittest discover -s tools -p 'test_stage2_*.py': exit 0, 137 pass;
  /private/tmp/stage2-terminal-current-tabs-python-20261004.log.
- python3 tools/test_validate_stage2_android_lifecycle.py: exit 0, 25 fixture
  verifier tests, not connected lifecycle evidence.
- cargo fmt --check, git diff --check and strict Blueprint validator: exit 0;
  36 [ ], zero [_]/[x].
- python3 tools/stage2_host_build.py and --verify-only: exit 0 after Cargo
  lanes; current debug artifact reattributed, no Rust source changes.
- python3 tools/stage2_android_build.py --verify-only: exit 1, missing current
  APK provenance. Unchanged Gradle/socket/ADB blockers not retried or bypassed.

Final log /private/tmp/stage2-terminal-current-tabs-20261004.log.
PTY exchanges /private/tmp/stage2-terminal-current-tabs-20261004/android-terminal-results.json.

```text
TerminalPageCodec.kt 7f59706932977dcb74c6c38969c29a4a33e19d1f5116e29d18e04bab894a524b
TerminalCommandCodec.kt 0ef9369e88675a9d29a6438ccf11b898724f7d73aa944270da23cfc4fb150384
MainActivity.kt dcceadd0037c9ef101da048bef4f03f5b66c785eaef0d85cd57b51460799aced
TerminalControlTest.kt a2fa0867285cbed1a4363c03326ce30dcf0e94bc12fd367b3a09a651754dc635
AndroidRustTerminalTest.kt b320a2a4722abeb6a812dce707fd0dbf656841b2c7fc8a993e773e6758f2e996
TerminalControlScreenTest.kt 42e5397ae88c793fe3be8bf837196e9882fef9a10cae27b9f83713855bb9fc19
source-log a11e5965a827ce3b105efe2c195daae9344391b9e8621be7e8a868b915467c47
terminal-results ad5a6148fd46531dbb9dbac8de8a2465c1677c2072c857e51980237211d87ffd
main.jar ed411b08052640c44888a44e13ef91e3cc1d1c03dac8e14e79abed882a0f0fc3
unit.jar f7e2763c8006928b0f050850e01cd036a1e547bc5f76629803c30607c0acace0
instrumented.jar 7dcef4cd1ad7a888e69d30c7616cdb1f34d1b853cd07c12c06d0f225b1e98283
host-receipt 32aa3a0d898a2e4d584c9164d0c2ee7d12700b116a7f802489ab2ca05b419901
host-inputs 373a8f5697a5bc95c10eda94c9b9b35facda736cd84bcdb0c0fa3e8703c8e58f
host-binary 2939439799b49d0e22f6072e95288a44b239e4d8eb999ad9f44bfaaf5480a861
```

Goal active/incomplete. Current APK, physical camera QR, direct-LAN TLS
bidirectional sync/recovery, connected API31/34/35, complete desktop parity and
signed reproducible release remain required. PTY host-side effects and Kotlin
source checks do not prove those broader product gates.

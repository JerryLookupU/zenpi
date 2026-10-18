# Current Rust To Android Wire Interoperability

Provisional S2-020/S2-032/S2-033/S2-050/S2-053 progress on 2026-10-04. Previous
goal turn repaired missing-key background authentication and host-smoke exit
accounting; it was progress. Current execution skill, Blueprint, dirty worktree,
host adapter/frame/events/subscription, Android response/replay/cache reducers,
resume policies and physical acceptance harness inspected. No nested agents,
workers, checkbox promotion or unrelated reset.

## Implementation

Added remote_android_interop Rust integration test. It creates a fixture-local
session, real P256 registered device and Read ACL, then uses production
HostAdapter dispatch, events, subscription and write_frame to capture actual
length-prefixed listing/inspect/resume/subscribe/gap responses. It appends a
local desktop turn with private PEM content, asserts the emitted plaintext
wire has no private body, and appends 300 further durable turns to force a gap
snapshot. The fixture contacts no provider and opens no socket. No signing
key, token or operator state is exported.

RustAndroidWireTest consumes these bytes through actual Android FrameCodec,
ResponseReader, HostProjectionCodec, SubscriptionReducer and ProjectionCacheCodec.
Four tests verify fragmented reads, same-session redacted replay and cache,
continuous subscription with terminal completion, 302-event cursor replacement
with the latest 64 turns, scope/cursor/journal/duplicate-snapshot refusal and
truncated terminal EOF without display publication. Initial test expectation
incorrectly required WireException for DataInputStream truncation; production
correctly raises EOFException. The assertion was corrected, not the codec.

The cached-source checker now generates a fresh fixture from the current Rust
test into its output directory before Kotlin compilation and passes that exact
path to JVM tests. Normal Gradle unit tests load the checked-in public
android-sync-wire.json. Both paths were executed; no historical fixture alone
is used to claim current-source interoperability. Generation identifiers and
timestamps vary, so fresh fixture hashes intentionally differ from stored ones.

## Verification

Commands ran from /Users/mac/Github/zenpi against the dirty integration.

- zsh tools/stage2_android_source_check.sh /private/tmp/stage2-rust-android-wire-final-20261004:
  exit 0; current Rust fixture test 1 pass; production/unit/instrumented Kotlin
  sources compile; 456 JVM tests pass. Existing warnings remain.
- Direct JUnitCore zenpi.RustAndroidWireTest with stored-resource fallback:
  exit 0; 4 pass. The complete source checker uses the fresh fixture instead.
- rtk cargo test --locked --offline --test remote_android_interop --test
  remote_subscription --test remote_subscription_tls --test remote_protocol
  --test remote_frame: exit 0; 87 pass, 1 ignored, five suites.
- python3 -m unittest discover -s tools -p 'test_stage2*.py': exit 0, 132 pass.
- Lifecycle verifier 21 pass; Blueprint verifier 5 pass; schema tool tests 4 pass.
- Rust format, strict Blueprint and git diff --check pass. All 36 rows remain [ ].
- Host debug rebuilt after Cargo test changed its executable; verify-only passes.
- APK verify-only still fails with missing provenance. No unchanged restricted
  Gradle/ADB/socket retry, stale APK installation or connected/camera/LAN run.

```text
remote_android_interop.rs 94abf721e1a024fe38fda13d890d77a4d9bb1a5b79e4c766e9a0b323bdfc3ec0
RustAndroidWireTest.kt 9e7c0c79ddf861c8aabd085a7040ec4eaa56ee657ed6a17021428ce42111a8c5
stage2_android_source_check.sh e19c0353825035a2aebd901d50b75d3c971bc161a81f8165371d86f57113b899
stored-wire-fixture ce0f97fd063946ee040054183f79ca10885e23d0ab8cd7b79e069736cd9171f8
fresh-wire-fixture 59d6d677dc6e33e91521e092f7a8a851729904e762ce29c3823660fac026a01f
main.jar f4a19f7e4072678db59c6dabb5267166ddb4d31950252c9fad9600dcfd5130b5
unit.jar fedc9719f324d49fe07525847c27d39b8a94ee1e49a06efb08038137c72fdd00
instrumented.jar 49185cec7ed267be124e9ab6a9dc3bfbfaffcf0fe3dbf1794b10b32f75a49eba
debug-host-receipt cb83a95b406034bbedd5a72ea0f2aa6320f10feafd4ed6f108b5dfc7baa9d7a5
debug-host-binary 701d3e597cd640090cdc254f6ad2f810b986dbd626e42179ca3da6d8e50c5d77
```

Goal remains active. This verifies current host-to-Android framed synchronization
logic, not Android TLS/socket interoperability, APK packaging, camera scan,
direct-LAN bidirectional writes, live provider execution or full parity.
Current-source APK, physical QR/LAN recovery, connected API lifecycle matrix,
full parity and signed release remain open. The prior full Rust suite failure
is not repaired or superseded by these focused passes.

# Event Before Revocation Receipt Interop

Recorded 2026-10-04. Partial S2-020/S2-032/S2-033/S2-052 evidence only.
Previous goal turn was progress (current release repeat and retained-evidence
verification). Execution skill, Blueprint scopes, current dirty worktree,
subscription reader and missing APK provenance were inspected. No nested
agents/reset, unchanged denied Gradle/ADB retry or stale APK installation.

## Cross-Language Boundary

The current Rust fixture now uses production serve_authorized with an actual
private DeviceRegistry and signed device identity. A test output writer revokes
that device after flushing the first transcript event, before the associated
successful page receipt. The actual next wire frame is the production
correlated Unauthorized/null-result refusal. Exactly event + failure are
present, with no successful cursor receipt or host journal modification.

The new Kotlin RustAndroidWireTest consumes that byte stream in three-byte
fragments through production ResponseReader and SubscriptionReducer callbacks.
It verifies zero publication, unchanged reducer/projection/cache/event cursor,
no subscription completion, no session recovery and display-only authority
loss preserving old durable transcript. The already-emitted event is not
committed. Both raw frame shape and end-to-end buffered reader are checked.

This is production function/device-registry/frame and JVM interop evidence,
not TCP/LAN, Android SSLSocket/Keystore or connected encrypted-store behavior.
No production behavior was changed this turn; the previous delivery repair
is now covered through the actual Android reader rather than only each side's
separate tests. Static packaged fixtures deliberately assumption-skip the fresh
fixture-only test; the commands below explicitly supplied the new fixture.

## Verification

Cwd /Users/mac/Github/zenpi. All observed command handles terminal.

- zsh tools/stage2_android_source_check.sh /private/tmp/stage2-event-refusal-20261004:
  terminal exit 0, current main/unit/instrumented Kotlin source compiled and
  every explicitly invoked Rust/Kotlin lane completed, including owned prompt,
  terminal, worktree/session/ACK/approval and worker cancellation.
- Independently reran ordinary current JVM classes: 592 pass. Current Rust
  wire class alone: 8 pass with the fresh fixture supplied.
- rtk cargo test --locked --offline --test remote_subscription
  --test remote_subscription_tls: 64 pass.
- Python Stage 2 tools: 163 pass; lifecycle fixtures 25; Blueprint fixtures 5.
- cargo fmt --check, git diff --check, strict Blueprint pass; 36 [ ], no [_]/[x].
- Source checker rebuilt the debug binary and old attribution correctly
  refused its changed bytes. Guarded debug build restored current attribution.
- Current release retained-evidence verify-only with --binary passed unchanged.
- APK verify-only remains missing provenance; no device/camera/LAN claim.

```text
remote_android_interop.rs b2294ed25d9110ba048c4b1cfc81474338dbd9e4a06164da49225dc11da604a7
RustAndroidWireTest.kt b59e755f0c660f0c0bd6607f1b202559d97e6205afcd7fca94d1d885eff1ed5f
unit.jar 9997450eb178a900f574f406fbfa41098d34f0d06c6c53bf18ee2eee19b385d8
android-sync-wire.json d710c98cd717d967e37cdafa69de6e44f6d0ebcbc63756b1e61a8e268a1a76f4
debug-binary 70335c71e5edf128f8183ec87a65e26633ce81126ae355e72c13c23e333b85e6
debug-receipt bfccd980170bf809df123cc3de52a4cf4212b29fe89f73a6ff7ccc31c978e703
```

Goal active/incomplete: current APK/signed installation/upgrade, physical QR
camera/direct-LAN pairing/synchronization/recovery, connected API31/34/35,
full-library and supported-host acceptance remain required.

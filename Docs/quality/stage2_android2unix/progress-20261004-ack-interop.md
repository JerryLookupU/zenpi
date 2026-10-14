# Cached Cursor ACK And Resume Interop

Provisional S2-020/S2-033/S2-050/S2-053 progress. Previous goal turn added
retired ViewModel job tracking and was progress. This turn inspected execution
skill, current dirty integration, network matrix, Android TLS/socket/deadline,
response correlation, ACK/cache/replay and Rust durable ACK implementation.
No new definite transport defect was inferred from that inspection. Work then
closed a cross-language evidence gap in the actual cached-cursor sync path.
No nested agents, checkbox promotion, destructive reset, commit or push.

## Implementation And Evidence

EventAckCodec now owns the request builder previously inline in HostClient.
Production HostClient and the new AndroidRustAckTest use the same builder;
exchange still supplies the request ID. No protocol shape, ACK availability,
identity or persistence-admission condition changed.

The explicit Rust remote_android_ack_interop lane supplies real current host
listing/snapshot to Kotlin. Kotlin encodes/restores ProjectionCacheCodec data,
generates ACK/current/lower/future frames and a resume request. Rust parses the
frames with production framing and dispatches with a current authenticated
read-only device. The busy owner refuses ACK; release permits ACK persistence.
Dropping and reconstructing ProjectOwnerPool restores the checkpoint. Duplicate
and lower ACK return the higher durable cursor without rewriting checkpoint;
future ACK fails invalid_request. All these reads leave journal unchanged.

A desktop turn is appended after restart. Kotlin reads fragmented Rust frames,
validates busy deferral and exact ACK receipts, applies one contiguous replay,
restores cache and generates the next ACK from its updated cursor. Rust persists
cursor 2, reconstructs host again and proves an old cursor 1 ACK cannot regress
it. Removing project permission refuses the next ACK unauthorized without
changing checkpoint or journal. Kotlin validates those final real receipts.

The source checker excludes this environment-dependent class from ordinary JVM
execution and explicitly runs its generate/validate/final phases via Rust. The
saved evidence has fixture_only true and explicitly excludes APK/TLS/LAN/device
claims. This proves domain/frame/cache compatibility, not a live reconnect,
encrypted PairedHostStore, actual process death or physical camera pairing.

## Verification

Cwd /Users/mac/Github/zenpi, preserved dirty integration, HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b. Observed verification time
2026-10-03T20:32:30Z, Oct 4 local time.

- zsh tools/stage2_android_source_check.sh /private/tmp/stage2-ack-interop-20261004:
  exit 0. Production/JVM/instrumented Kotlin source compilation; 469 ordinary
  JVM tests; current Rust read, rename, P256, worktree, session and new explicit
  ACK/resume lanes pass. New ACK lane runs three Kotlin phases and one Rust
  behavioral test. Existing compiler warnings remain. No packaging/device pass.
- A separate rtk cargo test --locked --offline --test remote_android_ack_interop --quiet
  compiled successfully but skipped its environment-dependent test (one ignored).
  It is not the passing interop evidence; the source checker explicit --ignored
  execution above is. Cargo serialized these invocations on its build lock;
  original live handles were polled, not replaced. Process inventory command
  was refused by sandbox; no process liveness claim was inferred from it.
- Python Stage 2 discovery: 132 pass; lifecycle validator: 23 pass.
- Rust format, git diff --check and strict Blueprint pass; 36 rows remain [ ].
- APK verify-only refuses missing provenance; no restricted Gradle retry or
  historical APK installation. Debug host rebuilt after Cargo tests; separate
  verify-only passes. This is source attribution, not release acceptance.

```text
EventAckCodec.kt 723d76a423fc0eebcee1478b9cc4fbb8965475dd4675cc7d86ba3942d324a907
HostClient.kt 582f471b457d6ecadf4d305bfbe227f83fd0f9e26e188405688df0852034244c
AndroidRustAckTest.kt 5df9a3f74002a7a25a880c04413a545f1d32776a03c1ab0f72c121294298db81
remote_android_ack_interop.rs 51280a579bb074924e3eb1797eb6b57be7e295e8b5fa14ffb89a5f9a742cd677
stage2_android_source_check.sh be114abd0a2faece06f1eb663c05d2168d98574afe2664d463c4f8682c223c92
android-ack-results.json 99b0a5788e31ee8b6259bffdce461d8b9dc5cfa30585b20aeda751b6b56dcc93
main.jar 1868d9f926dc02a49139f195296937f4cdf5461fb3133bd77704a5254b4f48b9
unit.jar aa0203649c55bd572cc70be177def9307d710e8a403b4b64b49386c848a53e24
instrumented.jar 8c22b80093a6d240fc71428f32893dddb9b7a3c7fe1e5b4efaff04ca507bba8d
debug-host-receipt cb6d3fcce40299e0858a6815a936ff249469f302df446a589dc4e93d7521bf60
debug-host-binary 701d3e597cd640090cdc254f6ad2f810b986dbd626e42179ca3da6d8e50c5d77
```

Full goal remains active and unachieved. Current APK, physical camera/direct-LAN
bidirectional sync and recovery, connected API lifecycle matrix, full parity,
backup/uninstall and signed release still lack required acceptance evidence.

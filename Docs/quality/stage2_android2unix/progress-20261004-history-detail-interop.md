# Historical Message Full-Body Interop

Provisional S2-036/S2-054 progress. Previous goal turn added executed conversation
history paging and was progress. Current skill, Blueprint/specification,
worktree, transcript/history codecs, policy and actual owner lane inspected.
Existing changes preserved; no nested agents, reset, push or checkbox promotion.

## Added Evidence

Extended the same history lane with a long UTF-8 history-0 message absent from
the 64-turn live snapshot. Its history preview is truncated; current Kotlin
selects it through SessionHistoryReadPolicy's transient validated projection,
generates TranscriptPageCodec frames bound to captured journal revision, and
decodes actual Read-authorized ProjectRuntimeHost full-body responses.

Three detail pages at offsets 0/8192/16384 reconstruct 21793 redacted UTF-8
bytes. Kotlin checks fragmented frames, retained backward-page policy, full
byte count/content SHA-256, terminal content, scope/digest/journal mismatch
rejection and live journal-change policy refusal. Historical selection does
not put old messages into live snapshot or grant write authority. Reads do not
append host journal data. Host append refuses both old history-list and detail
requests with revision_conflict; device revoke refuses both unauthorized.
Actual refusal frames are decoded in Kotlin. Existing 70-turn history coverage,
redaction and unchanged workspace revision assertions remain.

## Verification

Cwd /Users/mac/Github/zenpi; HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b; UTC 2026-10-04T00:22:10Z.

- Initial complete checker reached new Rust test compilation but failed E0282
  for the optional retained detail request. Added explicit Option<Envelope>;
  repaired full run supersedes the failed attempt.
- zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-history-detail-repaired-20261004: exit 0; 506 ordinary
  JVM tests, main/unit/instrumented Kotlin compilation, current terminal QR
  decoding and all explicit Kotlin/Rust lanes pass. Model/resources/files/
  diff/history target executes 5 tests; history lane includes actual detail
  generate/validate phases, not an additional assumed fixture skip.
- Exchange JSON inspected: 5 history pages, 3 detail pages, 2 journal-conflict
  refusals and 2 revoke refusals in final history step.
- Python Stage 2 discovery 135 pass; lifecycle verifier tests 25 pass.
- cargo fmt --check, git diff --check and strict Blueprint pass; 36 rows [ ].
- Debug host rebuilt after Cargo lanes and verify-only passes.
- APK verify-only refuses missing provenance. No unchanged Gradle/ADB retry,
  stale APK installation, permission bypass or connected UI claim.

```text
remote_android_model_interop.rs af9f0e3caf368d402d3e5419245373bfff387de67be5ce2ce6b202c32fedba96
AndroidRustHistoryTest.kt 940e338558276acb31c86db0765ccbabe61e5abca628c96939e34c3adbbfe8e0
source-log ddae4393b7cb8bb7a54d9a36013637b6787f5c04c3839d72ae622622b4f47a73
history-results b65624ed3df9ae934ed6fb45b1633edf5ef77db6e0f448b5c34b47dc88c43172
main.jar 5911009d658d4543d87f7b2a0b8d67eef910130686e58ff1d479f41df75818fe
unit.jar 1f48d6cdc390977c9daed58c2a336f536e871481371d5f8c44469f1b63af2f5e
instrumented.jar e8fea581c770f2ffda2803e3e8a546b670c831e1c51b7b005e91fdee161e0a4b
debug-host-receipt 7499cffa2816f690d29556cb60d4aad20fec47723bbcd605bbc2f0b58bcea33c
debug-host-binary 51cf433e9ebfd59aeebdbf46f844c0603872e798f8e4b6443308aed8627eca67
```

Goal active and incomplete. This proves actual desktop owner/journal/ACL and
JVM frame behavior, not Android UI/device Keystore, production TCP/TLS or
physical QR/direct-LAN synchronization. Current APK, connected lifecycle,
complete desktop parity and signed reproducible release remain required gates.

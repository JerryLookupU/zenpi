# Stage 2 Prompt Control Identity Progress

Recorded 2026-10-04 UTC. Related requirements S2-032 transport, S2-036
prompt/stream controls, S2-052 security and S2-054 parity. Partial evidence
only; authoritative Blueprint remains 36 rows [ ].

## Authority And Implementation

Cwd /Users/mac/Github/zenpi; integrated dirty-worktree HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b. Existing changes preserved; no nested
agents, reset, push, permission bypass or stale APK installation.
Blueprint SHA-256 d76e70e369f11f0fd3d1905d56d79c2a89401b92215da48ad85314f390d96d97;
specification SHA-256 403e6339b0318a420d93600fc52926acf6a9e46ec899bd9a9edb03c235190a28.

Cancel/steer construction previously checked existence of an active request ID,
not unique identity and target project/session. PromptStatusCodec.active now
requires a unique selected project and unique target record matching its
project/session and accepted/running state. Construction and fresh dispatch
reuse it. Prompt and worker prompt construction require unique selected scope;
worker submission additionally requires exactly one actual active subtab.

PromptControlPolicy shares scan/reset/pending/current-scope guards between UI
composers/controls and ViewModel preflight; ViewModel checks foreground/network.
Summary worker cancellation now also uses lifecycle admission. UI message,
steering and worker inputs disable when unavailable; message send rejects NUL.
Broken active-tab data no longer causes the task composer single() to crash.
Exact pending retry remains before projection revalidation in dispatch and
retains its own foreground/network/scan/reset guards, never retargeting or
regenerating the original envelope.

Two new JVM tests cover foreign/duplicate target identity, stale state,
scan/reset/offline, ambiguous project/active tabs, positive Error-phase live
controls and exact pending retry. Existing Compose steering test asserts
disabled input/send/cancel while scan/reset/ambiguous. Device assertions are
compiled only. This turn did not add a physical or network prompt lane.

## Verification

Commands ran from cwd above. Source checker uses its declared ZENPI_ANDROID_*
fixture paths and isolated ZENPI_HOME directories. No credentials or user
configuration were modified.

- zsh tools/stage2_android_source_check.sh /private/tmp/stage2-prompt-identity-20261004:
  exit 0; 529 ordinary JVM tests, production QR corpus, main/unit/instrumented
  compilation and every explicit Kotlin/Rust lane pass. Includes six-test
  desktop target and production tool-loop lane. No source/test edits after run.
- ZENPI_HOME=/private/tmp/stage2-prompt-identity-host-home-20261004 rtk cargo test
  --locked --offline --test remote_live_text: exit 0, 10 pass.
- python3 -m unittest discover -s tools -p 'test_stage2_*.py': exit 0, 137 pass.
- python3 tools/test_validate_stage2_android_lifecycle.py: exit 0, 25 pass.
- cargo fmt --check, git diff --check and strict Blueprint verifier: exit 0.
- python3 tools/stage2_host_build.py and --verify-only: exit 0 after Cargo lanes;
  current debug source attribution is not reproducible release evidence.
- python3 tools/stage2_android_build.py --verify-only: exit 1, missing APK
  provenance; unchanged Gradle/ADB permission failures were not retried.

```text
PromptStatusCodec.kt 8958f9f642d4fe9a23a622b324d30819e67a5596638ab1f02abc31ffa01bab2b
PromptControlPolicy.kt b01233a9cc7aeca57ecc395353b239e9203479770ee5ee100fbfcfdbace7efe2
WorkerTargetPolicy.kt 62c1afef7874302d94d93d564d8a468084c032c030eaa651102b2d2bc0351ac4
HostClient.kt 47a25d7854cf4b39382858d7f3cd6b1b36f65f795173fa772318bbd1d359464a
WorkspaceDispatchPolicy.kt b5963c0572e75d760276f271f404933884f06a4df67dd2528542ab6a66218413
PairingViewModel.kt d03c6f3c90c70384c678c33c4c62e51a64834f69cd1508561ec4605f2aa3f522
MainActivity.kt c7c209a9d8193439ad7328a5880e12c9441c6f4d8a7f3461af3631814d18067e
source-log ccb2ab160b4230b3e42d9b33ddf1d6566b082e0cbaa9d2a90ca67003b8e6a235
python-log a782d0c62ee109924f6ba28f5774749d23a3ab32e6cb385df29e38b1e8cbaa7a
host-test-log 2d43b0c8466245523e965830bbe739e6209fb7021893b11cbd1ec8abf94aa102
main.jar abea70cf5f840925ca447dc6d31d78c25380f670153cef4c0d4a2b7a6952fdeb
unit.jar d41a2ab30c7a46ca3c0b8fb1e7854cf3399680e73c38cb55bd7160b1f1f31d81
instrumented.jar f09ebc6d879a8b2d75da1b21208ee8900766935b362c7e0bf12d47077cbb47c8
debug-host-receipt b47a02f1b6c4dadc384cf41acfcff17ffb18c251bce1bf16b2c123bb1be3fe3f
```

Goal active and incomplete. Current APK, physical QR/direct-LAN bidirectional
sync/recovery, connected API31/34/35 lifecycle, complete desktop parity and
signed reproducible release remain required. JVM/source/host projection tests
are not physical device, production listener or task execution acceptance.

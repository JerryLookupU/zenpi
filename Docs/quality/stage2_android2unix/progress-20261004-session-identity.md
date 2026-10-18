# Stage 2 Session Identity Parity

Recorded 2026-10-04T09:21:47Z; partial session/Android parity evidence,
not Blueprint acceptance. Canonical cwd /Users/mac/Github/zenpi, dirty baseline
HEAD 1a8ff1acf72e0816fad503a05df2b1d18e07361b. User changes preserved;
no nested agents, reset, push, blocked socket retry or permission bypass.
Blueprint SHA-256 d76e70e369f11f0fd3d1905d56d79c2a89401b92215da48ad85314f390d96d97;
spec SHA-256 403e6339b0318a420d93600fc52926acf6a9e46ec899bd9a9edb03c235190a28.
Previous goal turn was progress: worker detail opaque-identity parity with
completed source verification. The intervening RTK request verified installed
rtk 0.50.0; it did not claim additional Stage 2 product acceptance.

## Finding And Change

Rust remote session catalogs and resume targets accept opaque identifiers:
nonblank, at most 128 UTF-8 bytes and no control characters. Android catalog
entries, restored session_resume arguments and replacement success receipts
instead required ASCII slugs. A valid host journal could therefore disappear
behind catalog decode failure, prevent durable resume restoration or fail
confirmation after the host had already replaced its owner.

SessionCatalogCodec now exposes a shared local predicate used by incoming and
retained catalogs, WorkspaceCommand restoration in HostClient.kt and
SessionCommandCodec receipt validation. Unicode, punctuation and whitespace
with content are preserved exactly. Project/session scope, selected main-tab,
catalog digest/target, owner/CAS, request correlation and replacement revision
fences remain unchanged. No client path interpretation, identity trimming,
schema migration or automatic mutation retry was added.

SessionCatalogTest checks valid Unicode/punctuation/128-byte targets, exact
saved requests and matching receipts; blank/control/129-byte identities refuse
catalog, retained-authority and durable-request admission. Existing negative
fixtures now use an actually invalid control-bearing identity rather than a
valid opaque slash/space identity.

The actual Rust/Kotlin session lane creates a clean header-only journal with
history:session/\u4e2d, verifies no recovery warnings, then appends its historical
turn. Kotlin consumes this real catalog and generates the resume request;
Rust replaces the real owner, reconciles duplicate/conflicting/stale requests
and returns the same durable receipt after restart. The lane preserves both
original journals byte-for-byte. This is in-process owner/journal evidence,
not physical camera, production TLS, APK or direct-LAN evidence.

## Verification

- zsh tools/stage2_android_source_check.sh /private/tmp/stage2-session-identity-20261004:
  exit 0, 582 ordinary JVM tests, main/unit/instrumented compilation and every
  explicit Rust/Kotlin lane passed, including actual session new/resume/restart,
  six-test model/read/worker target, QR, software P256, ACK, terminal, workspace,
  worktree, prompt/tool and running-worker cancellation. Process 41264 was
  polled through authoritative exit 0; silence did not trigger restart.
- python3 -m unittest discover -s tools -p 'test_stage2_*.py': exit 0, 142 tests.
- python3 tools/test_validate_stage2_android_lifecycle.py: exit 0, 25 fixtures;
  not connected API31/34/35 lifecycle behavior.
- python3 tools/test_validate_stage2_android2unix_blueprint.py: exit 0, five tests.
- cargo fmt --check, git diff --check and strict Blueprint: exit 0;
  36 [ ], zero [_]/[x]. No acceptance checkbox advanced.
- python3 tools/stage2_host_build.py and --verify-only after all Cargo lanes:
  exit 0, current attributed debug binary; not signed reproducible release.
- python3 tools/stage2_android_build.py --verify-only: exit 1, missing APK
  provenance. Inspected the retained failed Gradle attempt: global wrapper
  cache lock write was denied. No repeated blocked build or stale APK install.

```text
SessionCatalogCodec.kt bfb98e281b8f9b4b01651362a86cf13fcf1354d4be2c1f40e9212299ed374cf0
SessionCommandCodec.kt 7cabaaa104523db018f1c29ccd08a2df6d398adddea50e0b427d6d8ffd5a8f12
HostClient.kt a628cb837c8ddd8ce20ede5ae5358a5a87f16a62d30e87fbc007a5d8ffeefa70
SessionCatalogTest.kt a57516fe352ed3a45b34159766b0cae14c4dd10e5705fee09aaef7012461052d
SessionCommandTest.kt 1d88ef8074cf1695190bb0310da19648a9c4d60fb84f86432915bd2794adaf3d
AndroidRustSessionTest.kt b36574109c60a16b05436eb51ca46ba6b7c8c6887b2f2c71dcbc4282c090bbea
remote_android_session_interop.rs da833928f8f503a97d64621a6b7a5ec155d5e25cc46f494186e157e19c2b4dd2
source-log 9885b6fe100f7e5e8393b679e57684fb9c4c6a377e6fbd23de04910b94c95177
python-log 2c988c9559b6a29492494382797a1a7141d284a3d0a4a0e351f2f057e467c545
session-results bc6d7e77ff9a47e23208952842fb1e11c72b1865a49e8be9edbde0c310a7b7eb
main.jar 0abd26f5cbfda752e3050200116c1d8bce8b7e46d6f6f4eb0e165dfce720f584
unit.jar 501a50868ed85b8ada0ce24f2db93797e638d8fcbe874c50d7364f27cc741d71
instrumented.jar f95a635bb1b8d33d66febe3cf15afe0db9afd4ca9d63fb00a5f126e9d1e69630
host-receipt 31d690eee5c7531a6a89f8aad83231ba1858e355a0f9737b0f613eadb7bad180
host-inputs 08312e22d68633b57228db66f7d64511bc87dc758579124ef9de588e3a466219
host-binary 40f9f134ed62fd51a71dc6e8d24022f0122dced68faf12f6a52f8b5ea145607f
```

Goal active/incomplete. Fresh APK attribution, physical camera QR/direct-LAN
bidirectional sync/recovery, connected API31/34/35 lifecycle, complete parity
and signed reproducible release remain required. Source repair is progress;
no completion or blocked status asserted.

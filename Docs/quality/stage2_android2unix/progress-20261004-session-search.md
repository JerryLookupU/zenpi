# Host-Backed Session Name/ID Search

Provisional S2-036/S2-054 progress on 2026-10-04. Previous turn shared message
field/channel rules and passed cross-language tests, so it was progress. Current
Blueprint, parity matrix, desktop session actions, host catalog, Android catalog,
ViewModel/UI and session device harness were inspected. No workers/nested agents
admitted and no authoritative checkbox promotion.

## Implementation

Android session dialog now offers name/ID search through the existing host
session_list read. The Rust host validates a bounded trimmed/control-free query,
rejects redaction-sensitive strings and filters projected name/ID metadata
case-insensitively before sixteen-entry paging. It retains existing catalog
resource/path/ACL checks and full catalog digest. Search changes no current
session, journal, workspace revision or cursor, and filtered targets use the
existing confirmed/deduplicated session_resume mutation.

Android checks exact query echo and continuity, prohibits mixing search scopes,
resets pages/resume confirmation for new search and shows empty results.
The admitted query is transient, remains for explicit refresh after read error,
and is cleared on scope/authority loss. Search is fenced offline/loading/reset/
scan/pending operations. No historical journals are downloaded. This implements
metadata search only; desktop body search/snippets remain a separate gap.

Rust regression covers multi-page/case-insensitive names, exact ID, no results,
malformed query and unchanged journal/revision. JVM regression covers query
request/echo/paging/resume and changed-query refusal. Prepared narrow Compose
case covers search submission and offline fencing; the session harness now
requires an exact unique search-UI marker. Marker-negative verifier cases reject
old or duplicated evidence. No connected search acceptance is claimed.

## Verification

- Focused Rust search regression: 1 pass.
- Initial full remote_workspace under default config: 44 pass, 25 fail, with
  configuration I/O EPERM and dependent admission failures. Not a green gate.
- Revalidated full target with an owned writable config home and serialized
  tests: env ZENPI_HOME=/private/tmp/stage2-session-search-test-home-20261004
  rtk cargo test --offline --locked --test remote_workspace -- --test-threads=1:
  69 pass. This is deterministic host evidence, not LAN/provider/device acceptance.
- Final source command: zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-session-search-verified-20261004; exit 0. Production/unit/
  instrumented compile and 424 JVM tests pass. Existing warnings remain;
  instrumented tests compile only, not device-executed.
- Session verifier tests: 8 pass; lifecycle verifier tests: 21 pass;
  Blueprint verifier tests: 5 pass. Final Python discovery result recorded below.
- Strict Blueprint, formatting and diff-check gates passed; all 36 rows remain [ ].
- Real offline locked debug host build completed with updated source attribution.
- APK verify-only refuses missing provenance. No unchanged blocked Gradle/ADB/
  TCP retry, old APK install or physical scan/direct LAN run occurred.

```text
sessions.rs 8d419a4e8e423dcda0a967b00ac2f473a09ac6e4ffc26cf6778eb8dc64322722
SessionCatalogCodec.kt 42d419503569c5ebf5456960354dd06de91504f1fa10834e4622a2f6bde7384f
PairingViewModel.kt b89eb6a1f2706d4633a9c0acda654a2459f79bb2dc2f0719a32fcd9bed77aa8a
MainActivity.kt 153b3aecd662340576c0fb7e6773f5dd0bf4aed11b854ac2e8f54f8ccfb6c78c
remote_workspace.rs 173a38c45597a7e531fd84c9e31af5c5aea22f13d5b5aaaa5975e7465b29b34e
stage2_session_android_smoke.py dc81cd43fa92a4afc0cf4b8b2e705254c9880ab8eaf7a844ef8b63c21c93a1fd
main.jar b1a93e498479d86a00aeb3b8fa786eb9754f2e6af1af755e4172eda38285f07b
unit.jar 7b578f24ba7ae5d42c8e2c16fc31ae513b52ead2f8a8441bfc3ac993af811371
instrumented.jar e4e00214b83bc5e1d1468c4cb21eaa257845363111db379dbc773a3255cca74a
debug-host-receipt 2410bc9745f158831897fbbfbcede3fea428568b79638960773696af0ad7695a
debug-host-inputs c4a78bdde3f14bc65dc5fc7227230ac5875ddcede87cc3e649fff82398c2c693
debug-host-binary d044a25e2b07a8af6fb5f1d4f8c42d862d8a4d659c4a8d7572d18371d7716469
```

Goal remains active. Source-bound APK, physical QR/direct LAN sync/recovery,
complete parity/shared schema/API lifecycle and signed release remain incomplete
or unverified. No entire Blueprint item acceptance is claimed.

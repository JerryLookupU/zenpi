# Authenticated Connection Identity Fence

Provisional S2-016/S2-032/S2-037/S2-038/S2-052 progress, 2026-10-04. Previous
goal turn added and executed Kotlin/Rust/Git worktree interoperability, so it
was progress. This continuation inspected the dirty integration, execution
skill, Blueprint, current APK gate, subtab failure paths, HostClient and cache
publication. No nested agents, unrelated reset or authoritative checkbox change.

## Subtab Regression Repair

The preceding subtab suite had two configuration I/O failures. Agent project
preparation intentionally validates effective user/project configuration even
for EchoBackend, so host.open in those tests reached real user ConfigPaths.
The two cases now launch the same exact test in bounded independent processes
with env_clear, temporary HOME/USERPROFILE/ZENPI_HOME/CODEX_HOME, a no-auth
loopback fixture config and only platform PATH/temp variables preserved. Parent
environment is never mutated. Child timeout kills/reaps its own process.
Assertions verify config content is unchanged and auth/Codex state is not created.

A new isolated test supplies invalid project configuration and verifies that
real owner preparation refuses before project/session/subtab selection,
workspace revision, durable checkpoint or wakeup signal changes. Fixing just
the project overlay allows preparation and preserves the prior project's tab
state. Production configuration validation was not bypassed or weakened.

## Connection Repair

Authentication previously checked identity before TLS and around signing, but
not after a completed response or across subsequent authenticated requests.
Missing/replaced keys could thus leave the already authenticated socket able
to confirm a later operation or publish a page under lost local authority.

AuthenticationProof.current checks cancellation before reading the saved device
identity. HostClient now invokes it around connection routing, after TLS and
welcome/challenge, before each request, at response validation, around every
subscription consumer, after a completed exchange, after workspace confirmation
and after initial/final subscription publication. Foreground ViewModel checks
again inside its storage coroutine before subscription/final projection saves
and confirmed-write cleanup. ResumeWorker checks again immediately before its
CAS cache commit. Deletion/replacement yields typed authentication failure;
cancellation is preserved and private key diagnostics are not exposed.

Three new JVM tests cover cancellation before key access, receipt-time key
deletion/replacement for ordinary reads and subscriptions, and loss after a
first published page retaining that page's committed cursor while refusing a
later page. A new instrumented Keystore/cache test captures a completed read,
deletes/replaces its actual alias and verifies the identity boundary prevents
CAS cache mutation. The instrumented test compiled; it was not executed.

These are observed boundary checks, not an atomic transaction with external
Keystore mutations. An already sent operation may have executed on the host;
the client never infers rollback or creates a replacement key. Pending writes
remain unresolved when confirmation is fenced. The last committed display is
retained without treating it as fresh authority.

## Verification

Commands ran from /Users/mac/Github/zenpi against current dirty integration.

- rtk cargo test --locked --offline --test remote_subtab --test remote_worktree --test remote_concurrency --quiet:
  exit 0, RTK aggregate 32 pass/30 filtered. Top-level tests are concurrency 12,
  subtab 11 and worktree 6 (29 total); three subtab child test executions add
  three to the reported aggregate. This supersedes the preceding two subtab
  config failures for this exact current suite, not broader full-suite failures.
- zsh tools/stage2_android_source_check.sh /private/tmp/stage2-authenticated-key-fence-20261004:
  exit 0; current Rust read fixture; Kotlin production/unit/instrumented
  compilation; 463 ordinary JVM tests; explicit rename/P256/worktree lanes pass.
  Existing compiler warnings remain. javap confirms the new instrumented method
  is present in instrumented.jar. No APK or device execution claim.
- Python Stage 2 discovery: 132 pass. Lifecycle verifier: 21 pass. Their initial
  runs correctly rejected the stale desktop instrumented test count after the
  new ResumeStore method; BASE_INSTRUMENTATION_TESTS updated from 29 to 30 and
  both exact-suite gates then pass. Old reduced-count receipts remain invalid.
- Strict Blueprint, Rust format and git diff --check pass. All 36 rows remain
  [ ]. APK verify-only refuses missing provenance. No unchanged Gradle/socket
  restriction retry or old APK installation.
- Debug host rebuilt after tests; separate verify-only passes.

```text
remote_subtab.rs 33caca8dfc63742cd829e063d72c387c6d45db56198999384a113fd00dc658d0
AuthenticationProof.kt f9ee946b266f4694bc2829456717d4d7ef53f62d66af6ab2bcea87158a3cb229
HostClient.kt 06c709200c24ed9edde0a6686f961858292a292d10a7b58e6e6813e05321cee5
ResumeWorker.kt dab8e3aee9ad4f947e8b758525db43cbeab2dfaaa183cf09cd603b7cfc010341
PairingViewModel.kt d38eaf59b6738b4db401114551eb00a3b442bb25d9237d6330b13bb7d1df8ddb
ResumeStoreTest.kt 40fabb6dcd87523aa1371324264c00eb7ee60511beb7088c13038971920a33b4
main.jar 6ef1c8589a2f8bf4c5d722553621c256a71e82fd3e11945699f35dbb633e5ad8
unit.jar 39953a73d5650bdf3449e6c54bba7232688a57e987d0f941a682e83342e63c88
instrumented.jar dbba336085f0166529df062ba407376e7746c0f182356e41f46ba9fe6c319ebf
debug-host-receipt 74d00309e6370ff60024dff2c0692565cec2c5f11fc335167a55c1b7786e90bf
debug-host-binary 701d3e597cd640090cdc254f6ad2f810b986dbd626e42179ca3da6d8e50c5d77
```

Goal remains active. Current APK, physical camera QR/direct-LAN bidirectional
sync and recovery, connected API lifecycle matrix, full parity and signed
release are incomplete or unverified. This narrow repair does not accept an
entire Blueprint item or substitute source verification for device acceptance.

# Paged TLS and Selected-Branch Projection

Provisional S2-020/S2-036 work observed 2026-10-02T02:16:54Z. Previous
goal development turn implemented message pages: progress. The RTK installation
check was a separate user request and did not complete the Android objective.
Full native Android/QR/LAN/Rust desktop objective remains active; 36 rows [ ].

## Implementation

Transcript reads now validate selected-tree ancestry rather than physical
journal membership. Leaving a branch, including selecting an empty tree path,
refuses its next page; selecting it again allows the same digest/offset. Reads
clone bounded tree metadata, not every durable message's content. Linear reads
borrow physical history directly. Full target redaction still precedes slicing.

HostAdapter Inspect and Resume replacement snapshots now filter to the same
selected ancestry. Snapshot errors fail closed. Display truncation counts only
selected messages while event cursors remain bound to physical durable-turn
count, so branch changes do not rewind the transport cursor or edit journal
records. No new Agent, replayed tool execution or ACK is introduced.

Three new actual rustls TLS1.3/P256-authenticated bounded fragmented in-memory
transport cases route TranscriptRead through HostAdapter: reconstruct complete
redacted UTF-8 content; revoke the device after the first page and refuse the
next with Unauthorized/null; replace the digest and refuse RevisionConflict/null.
They assert exact journal and optional checkpoint bytes unchanged. This is
encrypted transport coverage, not TCP, LAN, Android or camera acceptance.

Rust helper tests admit exactly 8192-byte pages and the 4 MiB content cap, refuse
one byte over, verify empty EOF and out-of-range offsets, and reconstruct 1/2/3/4
byte UTF-8 content with redaction before offsets. The 4 MiB Turn is constructed
only for the helper unit: local journal/turn admission has smaller independent
limits. It does not prove a production journal accepts a 4 MiB turn. Kotlin
codec coverage accepts exact caps and rejects larger/fractional/negative values.

## Verification

Cwd /Users/mac/Github/zenpi. Rust ZENPI_HOME set to
/private/tmp/zenpi-paged-tls-20261002.

- cargo test --offline --locked --test remote_workspace
  --test remote_subscription_tls --test remote_headless --test remote_subscription
  --test remote_events --test remote_protocol: **67 pass** (25/14/1/10/6/11).
  Focused branch test rerun after final truncation assertion: **1 pass**.
- cargo test --offline --locked --lib remote::transcript::tests: **2 pass**.
  An earlier combined invocation with that name filter ran zero integration
  tests; only the separately unfiltered suites above count as integration runs.
- cargo check --offline --locked, cargo fmt --all --check, git diff --check and
  strict blueprint validator pass. Existing unrelated compiler warnings remain.
- zsh tools/stage2_android_source_check.sh
  /private/tmp/zenpi-paged-tls-final-20261002: exit 0; main/unit/device Kotlin
  sources compile, **145 JVM tests** pass in 27 classes. Device/Compose sources
  are compiled only, not executed.
- Five Python smoke/lifecycle/blueprint modules: **53 pass**.
- ./gradlew --offline :app:testDebugUnitTest :app:assembleDebug: exit 1 before
  compilation. Wrapper cannot write ~/.gradle/wrapper/dists/gradle-8.11.1-bin/
  bpt9gzteqjrbo1mjrsomdt32c/gradle-8.11.1-bin.zip.lck (Operation not permitted).
  No fresh APK generated and no permissions bypass attempted.

Initial TLS fixture wrongly assumed restore_checkpoint materializes a file;
fixed to compare optional checkpoint presence/bytes. Initial tree fixture
enabled migration after both branches, which correctly produced a legacy
linear chain; fixed to enable tree before selecting the root and appending
the alternate branch. Both failures were fixture mistakes, not ignored passes.

SHA-256 source:

```text
src/remote/transcript.rs 9b3ceb5323409bd45d4e5e124187934d79e128b23b791628dea8c9ab3cecdb8c
src/remote/host_adapter.rs 91332a44174bf675d7c52cec7845a52479cf4fe1ade1f08ee39e26179e83ba2c
tests/remote_workspace.rs 302f9efc3a29fb13e0a18f723475995c439b35fc1f55a6431fd31aa0f94fd862
tests/remote_subscription_tls.rs ec8522722d9b2d4b8e9f4c069d0f940f6ca9a66505c7538ab769d4dc46233fa7
TranscriptPageTest.kt 18425947555ba0f82a7590e2b714d18ea64c92416206cc68cfb6ba779174a1a3
```

Compiled evidence under /private/tmp/zenpi-paged-tls-final-20261002:

```text
main.jar dd8f17a68a55fce59e91165daed0b3897919e15e70ee99cc193a767237a0b4bf
unit.jar 72ee43ce400b5a6036bf569e6886775d695932ee484a361e53c33b6edf5a3300
instrumented.jar 96e9dab756ed1fc648f1577e330e0df8318577f8d47411bc932e3fd1820a7bbf
```

No current-source APK/device page execution, physical camera QR or same-LAN
phone run. Per-worktree quotas, complete TUI/headless parity, provider-delta
replay/ACK, interactive PTY, API31/34/35 lifecycle and release/migration remain
incomplete. No agents, commit/push, unrelated rollback or acceptance promotion.

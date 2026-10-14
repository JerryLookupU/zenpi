# Android Framed Subscription Consumption

Provisional S2-032/S2-033/S2-036/S2-050/S2-053 progress. Previous goal turn
completed workspace commit notifications with verified domain/TLS tests. This
turn makes Android's existing production response consumption directly testable
as a framed stream and tightens its final-batch and active-project validation.
The full QR/LAN/Rust-desktop/native Android goal remains active. All 36 Blueprint
rows remain `[ ]`. No nested agents, commit, push or historical receipt changes.

## Implementation

HostClient exchanges now delegate reads to ResponseReader while retaining
existing pinned TLS, device challenge, workspace outcome/receipt validation,
deadline/close/cancellation and ACK semantics. Reader consumes bounded actual
length-prefixed JSON, correlates response IDs and only calls the subscription
consumer after a complete response and successful outcome validation. Normal
control/resume returns one page. Event frames on control requests are refused.
Callback failure stops before the next page; it does not retry any mutation.

Reader and SubscriptionReducer refuse nonterminal batch 32 before publishing
it. Previously reducer accepted 32 nonterminal pages and rejected only the next
page; host already terminates at batch 32. Reducer validates refreshed active
project against the listing as well as requested project/session/subtab scope.
Unused frozen projects parameter removed from the reducer: changed display
projects must survive subsequent heartbeat/transcript updates.

New ResponseReaderTest has ten tests using production FrameCodec/ResponseReader/
SubscriptionReducer and three-byte fragmented InputStream, not an alternative
parser. Coverage: contiguous multipage cursor, every later-page truncation,
timeout after event, wrong response ID/refused outcome, foreign/gapped event,
per-page byte/frame limits, cumulative subscription bytes, terminal batch limit,
one-page normal resume, IO/cancellation publication failure, refreshed workspace
display across heartbeat and incomplete later transcript. Prior committed pages
remain visible; incomplete or refused later page does not advance publication.
SubscriptionTest adds missing active-project refusal and final-batch assertion.

## Verification

Observed 2026-10-02T00:20:39Z onward; current dirty worktree preserved.

- Final command `zsh tools/stage2_android_source_check.sh
  /private/tmp/zenpi-android-response-reader-verified-20261002` passes main,
  unit and instrumented Kotlin source compilation and **79 JVM tests** across
  19 classes. Initial intermediate runs passed 75 and 79 respectively; only
  the final run includes the final active-project check. Existing Compose icon,
  safe-call and instrumented fixture warnings remain. This cached source check
  is not Gradle/DEX/manifest/APK packaging, Android SSLSocket/Keystore, camera,
  connected lifecycle or device acceptance.
- `env ZENPI_HOME=/private/tmp/zenpi-response-reader-20261002 cargo test
  --offline --locked --test remote_subscription --test remote_subscription_tls
  --test remote_protocol --test remote_frame -- --skip
  real_loopback_timeout_remains_a_terminal_io_error`: **38 passed** (10/9/11/8),
  one socket case filtered. Rust source unchanged this turn.
- Before skipping, full remote_frame run: 8 pass, 1 failure. Failure occurs at
  TcpListener::bind (line 180), PermissionDenied/Operation not permitted in this
  sandbox, not protocol execution. The socket deadline gate remains unverified.
- `python3 -m unittest discover -s tools -p 'test*stage2*.py'`: **44 passed**.
  Strict Blueprint validation passes with 36 unfinished rows. `cargo fmt
  --check` and `git diff --check` pass. No test handle remains live.

Final source SHA-256:

```text
HostClient.kt 587b6c6e6fa3cc39cb758c00ab582a520837564080a85b1190bba5bc837e7555
ResponseReader.kt 7dd0bd4173ad496e3ea13f3aaa840b99104dfe0deb4e27235eb41581083363a6
SubscriptionReducer.kt fb64f81c357a0b7882eca1774a4708ed272c7448c15c024253baa4415c137175
ResponseReaderTest.kt ac084f9d3de89a7a6bee4a90fd159f3e3f87524da9ea19919f0115d53fdead91
SubscriptionTest.kt 0115aa9cd24079660aba11752f93b8265b3def2cdd85d9cb1ffb996f2ff2a9de
```

Main/unit/instrumented JAR SHA-256:

```text
89565ca4d6b28ce274b3d3d2eb54ce356d67f73617b0c8972d964606d58e1bd6 main.jar
57fe3f8380444e6e99a3fd0a8a70b6d4930812d8935cd6d62119d893410e8c55 unit.jar
b48c8c87a1cc6e3f25d98aa4a2be969127f7db0a3d7792ca7b53c55ea14f29b7 instrumented.jar
```

Not Master acceptance. Fresh APK/device subscription and reset screenshots,
physical QR/same-WiFi LAN, socket deadline/shutdown, complete stream-delta
backpressure, executable worktree scheduling 1..64, interactive PTY/bulk,
API31/34/35 lifecycle/Keystore, migration/uninstall and signed release remain
required. JVM bytes are useful client-path evidence, not a replacement end state.

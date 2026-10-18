# Strict Wire And Fault Progress

Provisional S2-003/S2-010/S2-050/S2-052/S2-053 work. No whole-item acceptance,
commit, push or nested agent. The complete Android QR/LAN/Rust desktop objective
and all 36 authoritative unfinished checklist rows remain intact.

## Actual Protocol Defect

Android FrameCodec already rejects duplicate fields at every JSON object level.
Rust's typed Envelope deserializer refused duplicates in typed structures but
dynamic serde_json::Value payload objects used last-member-wins. Therefore the
same wire request could be rejected by Android while Rust accepted its final
argument value. A regression test explicitly demonstrates the old typed decode
accepting arguments {limit:1, limit:128}, then the new remote byte decoder
refusing that same byte stream. This is a parser ambiguity repair, not a claimed
observed authorization exploit.

Remote Envelope::decode now runs a private Serde visitor preflight before the
existing typed decoder. Per-object decoded keys are unique, so escaped spellings
also collide. The preflight discards values rather than constructing a second
JSON tree and checks the total wire node/depth budgets during traversal. Frame
bytes remain bounded before parsing, and typed payload/channel/owner/version
limits are unchanged. The preflight uses 8192 nodes/depth 40 like Android; payload
validation remains 4096 nodes/depth 32. No auth/local JSONL/parser refactor or new
dependency was introduced. Programmatic Envelope validation is unchanged.

## Added Behavior Tests

Rust protocol tests cover dynamic arguments, nested array objects, escaped keys,
response results, event data and snapshot state duplicates. Legal repeated names
in independent objects remain valid. Two private preflight tests check exact
node/depth boundaries, one-over refusal, trailing JSON and invalid UTF-8.

New tests/remote_faults.rs uses injected caller-owned Read/Write implementations
to cover timeout, reset and would-block before/during prefix/body; clean EOF
after a complete frame versus every partial EOF; and timeout/broken pipe/zero
progress during output. Android adds matching duplicate payload cases plus
injected prefix/body read timeouts and failed writes. No mutation or durable
completion is inferred from a transport write failure.

These deterministic faults are not TLS socket, address-switch, arbitrary LAN or
provider stream fault evidence. The new remote_faults suite is only a first part
of the Blueprint's broader S2-053 suite, not that item's acceptance.

## Validation

- cargo test --offline --locked --test remote_protocol --test remote_faults
  --test remote_events: 18 passed across three suites.
- cargo test --offline --locked --lib remote::strict_json: two private preflight
  tests passed. The combined filtered invocation also filtered the integration
  suites, so it is not counted as their test coverage.
- cargo test --offline --locked --test remote_frame: eight passed, one failed
  when the real loopback test attempted TcpListener::bind and received
  PermissionDenied/Operation not permitted. A second run with that exact test
  explicitly skipped passed the eight non-socket tests; no test was weakened.
- Direct Kotlin 1.9.24 compilation and JUnit 4.13.2 execution of FrameCodec.kt
  plus FrameCodecTest.kt: eight tests passed, using existing Gson 2.11.0 and
  cached jars, output only under /private/tmp. This does not prove a rebuilt APK,
  Android instrumentation, TLS or device interoperability.
- cargo check --offline --locked: exit 0, 20 existing warnings.
- cargo fmt --check, git diff --check, all 44 Stage 2 Python tests and strict
  Blueprint validation passed. The initial new Rust formatting check failed;
  only the new/edited Rust files were formatted before rerunning it.

The first broad Rust regression invocation passed auth/diff/headless/parity/
pairing suites but stopped at a subtab test because opening another workspace
attempted configuration writes in the restricted external home. It was rerun
with ZENPI_HOME set to a new permitted /private/tmp fixture: subtab/workspace/
worktree completed with 37 passing tests across three suites. A separate repeat
of auth/diff/headless/parity/pairing completed with 21 passing tests across five
suites. No user configuration was overwritten or permissions bypassed.

The first filtered remote_headless_commands invocation used a wrong skip name
for its production TLS test; it therefore ran that test and failed at its
provider listener bind with PermissionDenied, alongside two passing tests and
one filtered test. After reading the actual test names, an explicit invocation
skipping both socket-dependent tests passed the two cancellation/registry-Busy
tests with two filtered. The socket tests remain unverified, not relabeled.
All required command handles returned terminal results before closing this turn.

## Limits And Next Acceptance

The current environment still prohibits socket listener startup and Android's
external Gradle cache locks. Host/Android network receipts from previous APK and
binary hashes remain historical, not current-artifact acceptance of this parser
change. The latest source requires normal Rust desktop/Android TLS and scanner
reruns when permitted. The scanner nonblank-capture repair remains unbuilt under
these restrictions. Real phone camera scanning/LAN, live event push, PTY/bulk,
worktree scheduling, full socket fault/replay-gap, migration and signed release
remain required by the original goal.

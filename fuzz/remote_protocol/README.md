# Remote Wire Corpus

corpus.jsonl is the single raw-wire acceptance corpus used by Rust
tests/remote_protocol.rs and Android RemoteProtocolCorpusTest. Each JSONL row
has a unique name, an expected acceptance boolean and the original JSON wire
string. Tests must decode the string directly, not parse and reserialize it
first, because that would erase duplicate fields and numeric spellings.

Run deterministic corpus and bounded seeded mutation replay:

```sh
cargo test --offline --locked --test remote_protocol
zsh tools/stage2_android_source_check.sh /private/tmp/zenpi-protocol-corpus
```

Android Gradle unit-test resources also load this directory. The mutation tests
each exercise 2048 cases with <=4096 bytes/case and assert that accepted values
roundtrip through framed/canonical codecs. Different seeded generators are
deliberate; identical acceptance is asserted on the shared original corpus.

Connection handshake rows distinguish omitted fields from explicit null:
hello/welcome/challenge are uncorrelated and unrouted, while pair/authenticate/
revoke_status add only id. Null or populated routing/authority fields refuse
before registration/authentication dispatch. Ordinary routed request/replay
optional-null compatibility is retained. The two codec unit suites also test
every forbidden envelope field across all six handshake types, including
outbound refusal before emitting a frame prefix.

schema-contract.json is a shared test contract for all 33 operation names and
their read/write classification, six execution states and twelve error codes.
Rust and Android construct the same contract-driven envelope matrix through
their production codecs. Every write refuses missing owner/revision, accepts
the maximum u64 revision structurally, and refuses event/bulk channel routing.
Failed outcomes require a known code; unknown operation/state/code values are
refused. This is structural compatibility, not ACL or operation-argument
validation. Adding a new protocol enum requires an explicit fixture/test review
in both languages.

The reviewed contract also generates Android RemoteSchema.kt, consumed by
FrameCodec for schema version, operation classification, states and error codes.
Regenerate with python3 tools/generate_stage2_android_schema.py; --check refuses
missing or stale output without writing. Gradle preBuild and the cached-source
check both require this check. Python 3 is therefore required for Android build
verification. The generator rejects duplicate JSON keys/list members, overlapping
read/write names and unsafe Kotlin identifiers. APK provenance binds generator,
contract and generated source. The contract additionally records all thirteen
message payload field sets, channels and minimal structural fixtures. Android
uses generated MESSAGE_FIELDS and MESSAGE_CHANNELS; both codecs test every
fixture, each missing field, an unknown payload field and each channel
substitution. Public pairing proof data in these fixtures is structural only,
not a valid signing identity or invitation. This does not generate operation
argument/result types or replace independently typed Rust enums and matrix tests.

This is regression/property-style bounded fuzz replay, not coverage-guided
libFuzzer execution or proof of complete protocol safety. Frame truncation,
limits, I/O faults, ACL and encrypted transport have separate test suites.

pairing-corpus.jsonl supplies shared invitation acceptance/address/safety-code
cases. terminal-qr-ipv4/ipv6.pgm are production qr_lines terminal cells rendered
as luminance images by the remote_qr_fixture example; their matching JSON binds
the URI, fixture clock, address and safety code. Android tests decode those
pixels with the same ZXing engine used by its scanner, then parse the exact URI.
The deterministic token is public fixture-only data, never an admitted token.

Regenerate pixels from current production Rust rendering when needed:

```sh
cargo run --offline --locked --example remote_qr_fixture -- fuzz/remote_protocol
```

Regeneration refreshes the fixture clock/expiry, so image hashes change. Stored
fixtures always validate against their recorded clock, not wall-clock time.
Pixel interoperability is not a physical camera or LAN connection receipt.

android-sync-wire.json records public fixture-only exchanges from the production
Rust HostAdapter, events, subscription and length-prefixed frame writer. The
Rust remote_android_interop test builds a local session and authenticated Read
grant without contacting a provider or opening a socket, captures listing and
snapshot, appends a desktop turn, emits replay and subscription, then appends
300 turns to force a bounded gap snapshot. The fixture contains no signing key,
pairing token, private PEM body or operator state.

```sh
ZENPI_ANDROID_INTEROP_OUTPUT="$PWD/fuzz/remote_protocol/android-sync-wire.json" \
  cargo test --offline --locked --test remote_android_interop
```

RustAndroidWireTest consumes the captured bytes through Android FrameCodec,
ResponseReader, HostProjectionCodec and SubscriptionReducer, checking replay,
redaction, cursor/cache consistency, gap replacement and truncated-terminal
refusal. Normal Gradle tests use the checked-in fixture. The cached-source
checker additionally generates a new fixture from current Rust into its output
directory and passes that path to the JVM tests, so this path verifies current
Rust/Android behavior rather than only a historical fixture. It does not prove
TLS interoperability with Android, APK packaging, camera, socket/LAN behavior,
provider execution or bidirectional mutation parity.

The same source checker also explicitly runs the ignored
android_generated_rename_roundtrips_through_the_current_host test after JVM
compilation. A current Rust owner supplies its listing/snapshot/subtabs to
Android SessionCommandCodec; Android writes the actual framed rename request;
Rust reads and dispatches it against that owner, then Android validates the
actual framed success/refusal receipts. The lane checks journal mutation once,
identical duplicate receipt, preservation of a later desktop rename, stale
journal refusal, same-ID changed-payload refusal and loss of Control ACL.
The Rust test is ignored by ordinary Cargo runs because it requires compiled
JVM dependencies. AndroidRustWriteTest is assumption-skipped in ordinary Gradle
runs and excluded from the ordinary cached JVM count; the source checker invokes
its generate/validate phases explicitly through Rust and must pass both.
This is real cross-language command/receipt compatibility for session rename,
not a device/TLS/LAN test or complete operation parity.

A second explicit ignored lane,
android_p256_pair_and_challenge_proofs_are_verified_by_rust, generates a fresh
production HostIdentity/InviteIssuer invite and connection challenges. Android
parses that URI and runs PairingHandshake with DeviceProofCodec and a fixture
software P256 key. Rust reads the actual hello/pair bytes, verifies the DER
signature, consumes the invitation once and registers Read-only/no-project
authority. Android-signed authentication and revoke-status bytes are verified
by current DeviceChallenge, including tampered host/token/request/name,
foreign nonce/device, reused challenge, revoked identity and wrong receipt
query refusal. The Unicode device name includes both a literal separator and
backslash escape to exercise exact signed UTF-8 agreement. No private key is
serialized or retained. The pairing receipt inside this offline handshake is
synthetic; actual TLS, Android Keystore and camera/device acceptance are not
claimed. AndroidRustProofTest is separately invoked and excluded from ordinary
cached JVM counts, with assumption skip in ordinary Gradle runs.

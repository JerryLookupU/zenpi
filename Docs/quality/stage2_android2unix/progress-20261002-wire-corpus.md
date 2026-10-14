# Shared Raw-Wire Protocol Corpus

Provisional S2-003/S2-010/S2-032/S2-050 progress observed
2026-10-02T04:47:49Z. Previous turn: progress (migration/backup implementation
and current tests). HEAD 1a8ff1acf72e0816fad503a05df2b1d18e07361b; dirty changes
preserved. Full Android/QR/LAN/Rust desktop goal active. All 36 authoritative
rows remain [ ]; no worker admission, agents, commits, pushes or cleanup.

## Implementation And Discovered Differences

fuzz/remote_protocol/corpus.jsonl is one shared 51-case raw-wire corpus. Both
Rust and Android consume the exact wire string instead of reparsing it before
ingress, preserving duplicate keys and integer spellings. Gradle unit resources
and cached source verification reference the same corpus directory.

Android numeric parsing now retains lexical tokens instead of BigDecimal
normalization; strict unsigned checks refuse 1.0/1e0/-0. Null failure code now
refuses like Rust. Pair public-key base64url re-encoding must match exactly.
Unpaired surrogates in values/keys, non-finite dynamic numbers and leading BOMs
refuse. Programmatic malformed Unicode cannot silently become question marks
when written: strict UTF-8 encoding fails before any prefix. Optional null
routing fields for subscribe/revoke-status match Rust absent semantics.

The shared corpus includes canonical/invalid pairing key representations,
surrogate pairs/unpaired keys, byte-count identifiers, unknown fields/enums,
duplicates including escaped keys, u64 max/overflow, invalid integers, scopes,
failure outcomes, channel errors, comments/trailing JSON and valid whitespace.
Both suites each mutate 2048 bounded cases and assert accepted frame/canonical
roundtrips. Separate tests accept exact payload depth/node limits and refuse
one more. Rust adds injected deadline failure at every prefix/body offset and
fragmented multi-frame output with exact flush counts. No new dependency or
coverage-guided fuzz runner was introduced.

## Verification

Cwd /Users/mac/Github/zenpi; Rust ZENPI_HOME=/private/tmp/zenpi-wire-corpus.

- rtk cargo test --offline --locked --test remote_protocol --test remote_frame
  -- --skip real_loopback_timeout_remains_a_terminal_io_error:
  exit 0, **24 pass** (14 protocol, 10 frame), **1 TCP test excluded**.
- rtk cargo test --offline --locked --test remote_subscription_tls --test remote_auth
  --test remote_workspace --test remote_parity: exit 0, **87 pass**.
- zsh tools/stage2_android_source_check.sh
  /private/tmp/zenpi-wire-corpus-android-current: exit 0, **169 JVM tests**;
  main/unit/instrumented compile. Shared corpus, 2048 mutations, exact complexity
  and malformed programmatic Unicode tests execute on JVM. Instrumented suite
  only compiles, no current Android device-run claim.
- Six Python smoke/lifecycle/blueprint/backup modules: exit 0, **63 pass**.
- cargo fmt --all -- --check, git diff --check, strict blueprint: exits 0.

Final SHA256:

```text
corpus.jsonl 5adea610ea8ee455eb08ad3e43aebfb3ff6b7acc767fb1ce303462d3f25eedff
main.jar 8b296644a0aab53d71d5f895d0e2badf52e7ddecd7f0600b3862f4cf5cea80c9
unit.jar 99184c546bc9981fe870eda10ce7d81bff51b63d1612dc2c9dc5554099676f40
instrumented.jar 889638ef4d4b8f205324458ea3e5ffd14725630e488122a2242a77f13efbe9ee
```

Intermediate evidence preserved: Rust fixture compilation initially failed on
a borrowed corpus name stored beyond its row; changed to owned names. Added
corpus/mutations exposed Android acceptance of noncanonical keys and malformed
surrogates; scalar/canonical validation fixed it. Later BOM corpus exposed
Gson's BOM tolerance; explicit rejection fixed it. Full final suites reran.
Some intermediate compiles read a corpus extended after compilation, correctly
showing missing-fix failures; no such run is claimed green final evidence.

## Remaining Acceptance

Bounded corpus replay is not exhaustive fuzzing or proof of all codec cases.
Structural key validation is not curve/proof authentication. Source/codec tests
are not APK packaging, real QR camera/LAN, API-device matrix, complete owner/
quota/settings/PTY parity, signed release, backup transport or uninstall proof.
Network/Gradle/adb permission constraints were not bypassed. No broad completion
claim and no authoritative acceptance promotion.

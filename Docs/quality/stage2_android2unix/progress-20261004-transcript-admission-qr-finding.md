# Transcript Admission And Reproducible QR Detection Finding

Provisional S2-031/S2-036/S2-038/S2-054 progress. Previous goal turn added actual
historical-body interop and was progress. Current skill, Blueprint/specification,
dirty worktree, transcript UI/ViewModel/policy/codec and scanner QR paths
inspected. Existing changes preserved; no nested agents, reset, push or checkbox
promotion.

## Source Fix

Transcript navigation and ViewModel initial reads omitted scanner state. Live
message/tool entries omitted reset and unique current scope fences. Added
TranscriptReadPolicy.browse shared by entries, ViewModel and navigation. It
requires connected unique listed project/session, no scanner/reset/loading or
pending write/revoke. Codec also rejects missing/duplicate/foreign listed scope
before encoding. Existing exact historical journal, worker task and tool-call
checks remain. Read-only/running-task clients remain eligible; failed pages
can reopen, not continue. Two JVM tests and existing Compose/history interop
assertions extended; no new named instrumented test.

## QR Finding: Not Repaired Yet

First full checker ran 508 JVM tests and failed exactly the production terminal
QR decode case, before explicit interop. Re-decoding the same pixels reproduced
ZXing NotFoundException. Added per-case assertion context to PairingCorpusTest;
the failing retained case is ipv4-loopback. A new complete checker with fresh
QR payload pixels passed 508 tests and all interop lanes. That pass does not
resolve the reproducible failure on the old pixels.

Original failing fixture URI has now=1791073423, expires=1791073723,
address=127.0.0.1:65535, fingerprint=64 lowercase a characters and fixture-only
token AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8. Produced with current
host_pair::qr_lines, scale 4; image 260x264. No real token/credential involved.
Original pixels/JSON retained under
/private/tmp/stage2-transcript-read-admission-20261004/terminal-qr/.

Dedicated JVM diagnostic /private/tmp/QrDiagnostic.kt tests the original image
with 0/16/32/64 extra white pixel margins: default and TRY_HARDER QRCodeReader
fail at every margin; PURE_BARCODE succeeds at every margin. This shows a ZXing
detector-path failure on a valid decodable symbol, not an unencodable URI.
Do not add PURE_BARCODE to camera scanning or suppress this gate: camera scenes
are not pure barcode images. Next repair must preserve the failing fixture and
address encoding/detection behavior before claiming scan robustness. This is
synthetic pixel evidence, not a physical camera result or direct-LAN finding.

## Verification

Cwd /Users/mac/Github/zenpi; HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b; UTC 2026-10-04T00:29:11Z.

- Initial checker /private/tmp/stage2-transcript-read-admission-20261004 exit 1:
  508 tests, 1 QR failure. Repeated same fixture fails, never marked passed.
- Diagnostic complete checker /private/tmp/stage2-transcript-read-diagnostic-20261004
  exit 0: 508 JVM tests, current generated QR cases, main/unit/instrumented
  compilation and all explicit Rust/Kotlin lanes pass; model/resource/file/
  diff/history target 5 pass. This attributes current transcript fix, not QR
  robustness across payloads. Instrumented cases compile only.
- Python Stage 2 discovery 135 pass; lifecycle verifier tests 25 pass.
- cargo fmt --check, git diff --check and strict Blueprint pass; 36 rows [ ].
- Debug host rebuilt after Cargo lanes; verify-only passes.
- APK verify-only refuses missing provenance; no unchanged Gradle/ADB retry,
  permission bypass, stale APK install or physical QR/LAN acceptance.

```text
TranscriptReadPolicy.kt d0ff52f598173572d68c6e4542a738556b6039a3ef83d098c2ee1c7b38f39abe
TranscriptPageCodec.kt 6afe028ae460cac6a50eea80597295c2cedbb838f8094f5b81eada6b88e1f84e
MainActivity.kt 18988c71bc8f43c72d8b1d6f2bba438d14c20835ca429561283fbd8ff575a54d
PairingViewModel.kt ecaf57e2a2560d86bf2ad93e815f2dae55f35c4095080db15fba63b291ed1171
TranscriptReadPolicyTest.kt 4e80dda30553ff0be019420580adaa8623ffd0ebc87ee6946cec3a4dfaa669da
DiffScreenTest.kt d5ec8b174902f7bde9ded745043bcd2b7959a06cacb9a18e701c1f24497712fb
AndroidRustHistoryTest.kt 00df88f4f8341128f14d72ced0f61b2c4b875e5441a45c57abc21a1e9bf615ff
PairingCorpusTest.kt ad6df0ff98fc3ebe8ecdf12a18ea8b64c41c320a5047ba91df753524f21aabb7
failed-source-log dcfd04bd5588927a104f0b91dedec122978109c2d8b9d9aab37417214788fab6
diagnostic-source-log 6323bf0a4e8de8b96b3873ca7254abd60da37a47e73bc70285bf66047a33ad65
failing-pgm b877ab32979ca2e0e4ae10d9b0d396560551941c8f97bd0a2251af090166a69b
failing-json 07a3457aa5d27d35450c0fe3920a6e8f1a7f4d041c60ea3bde522526618d3cd4
main.jar f0a4d1ee68c193a3c6f331bc746ec30a56cdd99987fe762cdec0b17c5c0d0348
unit.jar 8c22d5ec191eae94f6068d7727bbcb976cb5b30026f8c352d73427aef29f5cb3
instrumented.jar dbdd49645674f0ef83eda57398d5e8bf3f4854bc3466fddc7510455b337e16a1
debug-host-receipt 3990bb5f17ca446f1bb0c1b87b5442fee275345bc0d468769e8cdcb454c740aa
```

Goal active and incomplete. QR detector repair is an actionable next source
step. Current APK, physical QR/direct-LAN sync/recovery, connected API lifecycle,
complete desktop parity and signed reproducible release remain required gates.

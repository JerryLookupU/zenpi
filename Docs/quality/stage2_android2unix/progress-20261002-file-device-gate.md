# Device File Picker Acceptance Chain

Provisional S2-036/S2-055 progress observed 2026-10-02T02:44:54Z. Previous
goal turn verified file TLS and local staged-attachment isolation: progress.
Full native Android/QR/LAN/Rust desktop goal active; all 36 rows remain [ ].

## Implementation

HeadlessHostTest now wires production browse/close callbacks. Before first
prompt it opens composer attachment picker, navigates documents, selects
fixture note.txt, checks quoted reference in input, no pending command, same
transcript/cursor/cache display and cleared transient picker. It captures
headless-files.png and emits headless_file_picker_reference_verified before
explicit Send. Existing provider running-preview, background, reattach,
process-death, exact retry and ACK workflow remains, with two provider calls.

Harness seeds known file bytes, requires the picker phase and unchanged source
file, stores file-picker PNG with SHA256 and adds a receipt gate. Independent
attachment_evidence checks exact Responses input_text/input_file order, filename
and data URL, exact journal path/MIME/size/SHA256 references, no raw/base64 bytes
in journal, and no media part in the second provider request or attachments on
other journal turns. prompt_evidence always invokes this check; old plain-text
provider evidence is insufficient. The API31/34/35 validator now requires the
new gate and file-picker screenshot identity for each headless receipt, and
rechecks saved provider/journal evidence despite claimed booleans/checksums.
Historical receipts are not modified or promoted.

Python negative fixtures cover missing/changed/duplicate/wrong-name/wrong-path/
wrong-digest/repeated/retained attachments and raw/base64 journal leakage.
Lifecycle fixtures refuse a claimed picker without screenshot or with valid
evidence-file checksum but wrong provider bytes. Rust encoder unit checks the
exact fixture Responses payload and subsequent text-only request, confirming
the harness expectation against production serializer rather than guessing.
Rust production behavior and Android main/unit code unchanged this turn.

## Verification

Cwd /Users/mac/Github/zenpi.

- Five Python smoke/lifecycle/blueprint modules: **55 pass** (two added tests,
  negative cases included); focused headless/lifecycle intermediate **29 pass**.
- env ZENPI_HOME=/private/tmp/zenpi-file-device-gate-20261002 cargo test
  --offline --locked --lib protocols::responses::tests: **2 pass**, including
  new host_file_picker_wire test (also focused **1 pass**).
- zsh tools/stage2_android_source_check.sh
  /private/tmp/zenpi-file-device-gate-20261002/android: exit 0. Main/unit/device
  sources compile; **151 JVM tests** pass in 28 classes. Expanded production
  HeadlessHostTest is compiled only; no HTTP/provider/socket/device execution.
- cargo fmt --all --check, git diff --check and strict blueprint validator pass.
  Existing compiler warnings remain.
- ./gradlew --offline :app:testDebugUnitTest :app:assembleDebug: exit 1 before
  compilation, wrapper lock ~/.gradle/wrapper/dists/gradle-8.11.1-bin/
  bpt9gzteqjrbo1mjrsomdt32c/gradle-8.11.1-bin.zip.lck EPERM. No new APK.

SHA-256 source:

```text
stage2_headless_android_smoke.py 47691478948c4d36fb4e5fb681167272b98cea62e107fc016b93eafb2be28b28
test_stage2_headless_android_smoke.py 481b37c821157e6f0b6b6507652d7aaf5c2d087e78d8e77b531c08e287c422a8
validate_stage2_android_lifecycle.py f7b4148e95e488ab8f11e1a8c882a2a3cad50d31dd75e5184f5a18e8693b978e
test_validate_stage2_android_lifecycle.py ab224fabce3f10e53da59280a25d120a571d367f43d88e339d7eb73bd3195f66
HeadlessHostTest.kt d7cbd3773e5348b4eed28653c80fdc8a7a1c8a90ba77c83bba3a6ef67733e077
src/protocols/responses.rs d5032a39c9ed3983e8c9c97cfe84051482558030c50064ae11c3d980e84dc9d5
```

Compiled evidence:

```text
main.jar b2f8a5fc654fed0c16fa579b8aa90cbde8b7efd63de9867e228636f4ffe5fe48
unit.jar 0452300dca4aa2b565559579cfb9249deef115d51072a7adfd5c8a1b058a7260
instrumented.jar c06666515292e1f2a847142e04ac4fe1854aeb84b34f69b2218c943907b2a5d3
```

Prepared gates are not execution evidence. No current-source phone/LAN/camera
or API matrix acceptance, effective worktree quotas, full resource/routing
parity, provider delta replay/ACK, interactive PTY or release/migration
completion. No agents, permission bypass, commit/push, unrelated rollback or
acceptance promotion. Existing dirty worktree preserved.

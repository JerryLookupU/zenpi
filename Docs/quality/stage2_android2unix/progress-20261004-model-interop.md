# Android Model Selection Interoperability

Provisional S2-019/S2-036/S2-054 progress. The preceding goal turn changed the
LAN binder and added verified regression evidence, so it was progress. Current
execution skill, Blueprint, dirty worktree, resume/store/lifecycle paths, parity
matrix and model command/status/receipt code inspected. No nested agents or
authoritative checkbox promotion; unrelated changes preserved.

## Implementation

Added AndroidRustModelTest and remote_android_model_interop, explicitly executed
by the source checker. Current Kotlin consumes real desktop listing/snapshot/
model status, constructs/restores/framed-encodes each command, and Rust decodes
it through the real ProjectRuntimeHost/TuiState owner. Kotlin validates fragmented
Rust response frames and rejects altered request/project/session/value/revision
receipts. Software P256 device authentication seeds the owner fixture; no
Android Keystore, TLS socket or provider HTTP is claimed by this lane.

The test covers default null and explicit none reasoning, model change retaining
effort, local reasoning winner causing selection CAS refusal, refreshed low
selection, Read-only and admitted-project busy refusal, exact duplicate, stale
new ID, altered same-ID conflict and restored exact receipts. Rust checks actual
model/effort, selection-event sequence, unchanged workspace revision, visible
desktop model, credential nonprojection, preserved journal prefix on bootstrap
and no journal append from restored retries.

Model/reasoning mutation was previously a separate production mailbox branch;
ProjectRuntimeHost's general entry fell through to HostAdapter without the same
admitted-project fence or visible model/catalog refresh. Both now use a scoped
model branch in ProjectRuntimeHost. The production loop still computes pairing,
active-job, queued-input and worker fences and passes them into that adapter.
It does not weaken the existing durable selection CAS or receipt namespace.

## Verification

Cwd /Users/mac/Github/zenpi; HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b; UTC 2026-10-03T22:41:37Z.

- zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-model-interop-verified-20261004: exit 0; 481 ordinary JVM
  tests, main/unit/instrumented compilation, regenerated QR and all explicit
  current Rust/Kotlin lanes including the new model test pass. Device tests
  compile only. Source log and six-step model evidence hashes below.
- ZENPI_HOME=/private/tmp/stage2-model-owner-home-20261004 rtk cargo test
  --locked --offline --test remote_parity --test remote_workspace
  --test tui_project_workspace: exit 0; 105 pass across three suites.
  Initial run without writable ZENPI_HOME failed global config I/O; that result
  is not acceptance and was superseded by the isolated-home run.
- Python Stage 2 discovery: 135 pass; lifecycle verifier: 25 pass.
- Rust fmt, strict Blueprint and git diff --check pass; 36 rows remain [ ].
- Initial interop drafts failed incorrect fixture assumptions: selection revision
  is a journal sequence, not a unit counter; an explicit CLI model suppresses
  implicit restore; bootstrap may append extension metadata. The final fixture
  explicitly restores its registry-bound seed owner, checks the exact selection
  event sequence, and compares retries to the post-bootstrap journal while
  preserving all pre-bootstrap bytes. It does not weaken production restore.
- APK verify-only still refuses missing provenance. No repeat of unchanged
  blocked Gradle/ADB, permission bypass, old APK install, camera/LAN or signed
  release acceptance. Debug host attribution is refreshed after source tests.

```text
src/tui.rs d591537edad2e369deea289998b97a69c5713b2dffc18d7f76fa61c5ddb5e840
remote_android_model_interop.rs bafec5afedbc191b023b93ffb5824e63436f70227e9402d6a80dd41cbdf5671e
AndroidRustModelTest.kt 2bcf984ae4d33545ab927a6a1afca6149bccfaad2410067c47aed9eed6e4f011
stage2_android_source_check.sh 52025fc2abd10fdd7cedb38b3eba41652cd3325ad034811aa33ef86ee4327f15
source-log 76c03786ede1b671f6d136a3108c3d3a2cc1dd3c0b4f788c1048ee3e82faec43
model-results d09232ef2b04361ab29c3f0e6e3b70cd3be5e74e17f1b881331242b2ad380262
main.jar 01762f14857539ead873df69a98c61b8e51ac94aaeaecd6846d9e4bed34cff13
unit.jar 62bf6839e2047cd1187a0b220667ae2efc7a0d98ee9735d6844b2bfb755716ed
instrumented.jar f4d12e6a318318059210506a6a3538fa58fc525b911e3fb6f71fa9129e0ee7f4
debug-host-receipt 14a6e6f74d8efda9f533f06d98787032b912532f29963382aff7e934c23604fa
debug-host-binary c6c9e11b699e83b537f0c38e6f63bdc32e74f0f4a82338abd595e9715ac46fe6
```

Goal remains active and incomplete. Current APK/model dialog and provider using
the selected configuration, physical QR/direct LAN two-way sync/recovery,
supported API lifecycle, complete parity and signed release remain unaccepted.

# Android Diff Read Admission

Provisional S2-036/S2-038/S2-054 progress. Previous goal turn added executed
file metadata interop and was progress. Current skill, Blueprint, specification,
worktree, diff codec, UI, ViewModel and existing tests inspected. User changes
preserved; no nested agents, push, reset or checkbox promotion.

## Defect and Fix

Diff entry, ViewModel read/page methods and dialog controls did not consistently
fence scanning. Entry omitted reset checks; dialog allowed editing and invoking
the path IME callback even when the refresh button was disabled. The ViewModel
changed dialog state and accessed device identity before HostClient rejected an
invalid path/scope. Diff codec accepted duplicate active tab identities, and
capability-filtered project lookup could overlook mixed-permission duplicates.

Added DiffReadPolicy shared by UI and ViewModel. It requires connected current
unique advertised diff project/session and selected-tab scope, and rejects
scan/reset/loading/pending write/revoke. Successful scoped pages are required
for navigation; failed reads may start a fresh review. Prompt control and idle
task status are not prerequisites for this Read operation. Path/IME control is
disabled while fenced, and request validation now precedes UI state change and
device-key access. Codec enforces unique active tab and unique identity before
checking advertised capability. Existing network/generation/cancellation and
response-scope fences remain in place.

Three new JVM tests execute read-only/running-task review, failure refresh,
lifecycle/scope/capability/duplicate identities, pending writes and stale pages.
Existing named DiffScreenTest cases were extended, without adding named device
tests, for disabled scan/reset/scope/capability entry, path control and paging,
plus separate refresh-after-error. Compose cases compile only, not device-run.

## Verification

Cwd /Users/mac/Github/zenpi; HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b; UTC 2026-10-03T23:34:31Z.

- Final stable-source check:
  zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-diff-admission-final-20261004: exit 0; 497 ordinary JVM
  tests, current QR decoding, main/unit/instrumented compilation and all
  explicit current Kotlin/Rust interop lanes pass. Model/resources/files target
  3 pass. Earlier run also passed but preceded final duplicate-scope changes;
  only this final run attributes all current changes.
- ZENPI_HOME=/private/tmp/stage2-diff-admission-home-20261004 rtk cargo test
  --locked --offline --test remote_diff: 1 pass. This is Rust selected-root,
  authorization and stale-route evidence, not cross-language diff transport.
- Python Stage 2 discovery 135 pass; lifecycle verifier tests 25 pass.
- cargo fmt --check, git diff --check and strict Blueprint pass, 36 rows [ ].
- Debug host rebuilt after Cargo checks; verify-only passes.
- APK verify-only refuses missing provenance. No permission bypass, stale APK
  installation, physical QR/LAN or connected UI/IME execution claim.

```text
DiffReadPolicy.kt db6758f08858d823525890689c1186b4a24e3f5936a2c84ea3ab0ca7ff2830ec
DiffProjection.kt 0c0159a911b3e9364926261495b0f154846da12806e905abcbe4fe66b1e69dbb
MainActivity.kt ea67b4e60d84941f731207e020ffc8cbcece38d764505b480a7cae19a3b5bdbb
PairingViewModel.kt 6557f6eab9d744bdf5e6b942082d2d8ed68ee7bfae050856f54986b242f57cdf
DiffReadPolicyTest.kt 7fe530a3f76c446913a9229b655bb09a94d7ad1cc6c6ef00348307327111b132
DiffScreenTest.kt c56b4820abb306f7af4aa3adc6eb060534e4815410e9696bc83d991b41124ef8
source-log b36293ebaad65a28c18dd2c7f729d425da0c07d6dfbed0e4cd9dc745bf81d9d3
main.jar a790c484137137cce03a8c9157c6f50f77cdaa46f52913d81cd8b03fedea6265
unit.jar c201e224d7a731d395c7ae230a2b945fd7fc6210e1d52958ee9ae6093edbcc1b
instrumented.jar 7e7bd4b278e953570fc13cc5dde3764b63727e1f206b839b5a148c0d39ee18f4
debug-host-receipt 6038bd1d46ade921a04a9d281bd7d07e9e42e4842cb6221346b4a5afeb1bd6cb
debug-host-binary 51cf433e9ebfd59aeebdbf46f844c0603872e798f8e4b6443308aed8627eca67
```

Goal active and incomplete. Current APK, physical QR/direct-LAN bidirectional
sync/recovery, connected API lifecycle, complete desktop parity and signed
reproducible release remain required unaccepted gates.

# Real Desktop Worker Android Read Interoperability

Provisional S2-036/S2-038/S2-054 progress. Previous goal turn added worker read
admission and was progress. Execution skill, Blueprint/specification, dirty
worktree, actual desktop worker route and Android codecs inspected. Existing
edits preserved; no nested agents, reset, push or checkbox promotion.

## Actual Owner Lane

Added AndroidRustWorkerReadTest and an explicitly executed Rust owner test to
the existing real-desktop interop target. Kotlin generates bounded framed
requests; ProjectRuntimeHost/TuiState dispatches them under an authenticated
P256 device; Kotlin ResponseReader decodes three-byte-fragmented replies and
production codecs/read policies validate their contents. No forged task status
is substituted for a real runtime task or journal.

The offline echo fixture executes 18 sequential real worker prompts. Exact
retries before downgrade retain accepted receipts without rerunning. Master
conversation stays empty and desktop draft unchanged. Local registry permission
downgrade then fresh challenge authentication removes Control; all subsequent
task reads are by the same Read-only device and actual worker_control=false.
The old target is absent from the bounded 16-task summary. Status pages contain
16+2 tasks, strip outputs, and provide a transient task read scope without
publishing cached write authority.

The target has actual user/assistant turns, both truncated in the history
preview. Full assistant output and selected user-message detail each span
offsets 0/8192/16384, total 20820 bytes, with reconstructed SHA-256 and backwards
navigation verified. Host restart restores durable tasks/journals without
resubmission. The original write becomes Unauthorized after downgrade, not
blindly replayed. All worker journal bytes remain unchanged across reads,
refusals and restart. Foreign device task access is InvalidRequest, foreign
session OwnerMismatch, stale digest RevisionConflict. Revocation refuses task
list, history and detail with Unauthorized, all decoded by Android.

Initial interop failed an overstrong test assumption that the first output
page's actual turn ID was known before the host returned it. Fixed the test:
first-page worker/task/subtab/digest checks remain, and actual turn ID is bound
for continuation or an explicitly selected history message. No production
codec check was relaxed. Failure log retained, never counted as passed.

## Verification

Cwd /Users/mac/Github/zenpi; HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b.

- Final complete checker /private/tmp/stage2-worker-owner-verified-20261004:
  exit 0, 515 ordinary JVM tests, 54 production terminal QR cases, all main/
  unit/instrumented source compilation and every explicit Kotlin/Rust lane.
  Existing model/resource/file/diff/history plus new worker owner target:
  6 passed, none ignored. New test is excluded from ordinary discovery and
  explicitly run with current seed/command/reply data, not silently skipped.
- Python Stage 2 discovery 137 pass; lifecycle verifier tests 25 pass.
- cargo fmt --check, git diff --check, strict Blueprint pass; 36 rows [ ].
- Debug host build and verify-only pass after Cargo lanes. APK verify-only
  refuses missing provenance. No unchanged Gradle/ADB retry or permission bypass.

```text
remote_android_model_interop.rs 430dde83b9ea971b5aaa7e3211381afdd3459dc1b6c25a32592dd418ee36da18
AndroidRustWorkerReadTest.kt 3134ae2e5a6106f9c8039bde8ce7bfffa0399d34e6929cf95c28d53c815ee1ad
stage2_android_source_check.sh d4c2b96793846c5ed3fc51ad9e87fe8952d343b37e4cf5e156a364f6c072c5e7
final-source-log 5ac973e638b1623eab2c4ce2bc687d81fee06bfc39b122e4ee95de8a96d3886b
worker-wire-results 2e75c7d88f3c567cccf7628622157cab689041465b1eea3cd3d43dd747d032f1
python-log 78742226ae31681f032345b0dd5ff1a2218f03fd442258fa79d9e770cf10c675
main.jar d0ba8f4168d9217d9df7775d2a368ac453d3e30c5614562b4c7af4732934110a
unit.jar fb1fb871d7bd0682563aca0ab7833b31fef8e2e71d13c309b05fb582349fd516
instrumented.jar a5deb9ca7f74c5438968a4ec710572adee8a563d7ce2f37450a52b50af7abab7
debug-host-receipt 9fbb995d10384423e8dbcd408a111df96f44559d81fc56d40e319978073e4542
```

Goal active and incomplete. This closes offline real-worker read interop,
not TLS/LAN or physical camera acceptance. Echo history has two turns, so
multi-page tool-loop worker history/arguments remains further coverage. Current
APK, physical QR/direct-LAN bidirectional sync/recovery, connected API31/34/35
lifecycle, complete desktop parity and signed reproducible release still required.

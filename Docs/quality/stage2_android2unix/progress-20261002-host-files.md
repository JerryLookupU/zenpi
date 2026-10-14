# Host Workspace File Picker

Provisional S2-019/S2-036 progress observed 2026-10-02T02:34:58Z. Previous
goal turn unified host prompt attachments: progress. Full Android/QR/LAN/Rust
desktop objective remains active; all 36 rows [ ].

## Implementation

Add read-only files_list through protocol/ACL classification and shared or
owner-thread HostAdapter. Exact project/session/request identity and selected
subtab ID/revision are validated, write authority/turn/sequence refused. Hold
current registry grant and existing owner lock, validate captured workspace,
then list only immediate children using attachment path containment policy.
No file bytes, absolute roots, journal/cursor/ACK or workspace changes.

Omit hidden components/entries, symlink entries, non-UTF8/control/redacted
names and non-file/directory sources. Retain at most 64 directory-first sorted
entries, visit at most 2048 candidates, check a 250 ms cooperative deadline,
refuse result above 64 KiB. Truncation explicit. Files above existing 10 MiB
attachment limit are visible but not attachable. No recursive traversal,
pagination or OS sandbox claim; a blocked filesystem syscall is not bounded
by the cooperative deadline. Existing validate/open TOCTOU limitations remain.

Android strict scope/path/name/count/kind/size/flag codec, pinned authenticated
HostClient read-only exchange, foreground/reset/network/selection fencing and
generation cancellation. Picker is transient, never cache. Local reset joins
file job before key wipe. Composer attachment icon opens root; folders and
parent navigate; eligible file inserts an escaped quoted @ reference into the
draft without sending. Retry/loading/error/empty/truncation/close states and
offline/pending control fencing. Opening diff/transcript, writes, selection,
background or route change clears it. Prompt admission rereads selected bytes.

## Verification

Cwd /Users/mac/Github/zenpi. Rust tests use
ZENPI_HOME=/private/tmp/zenpi-host-files-20261002.

- cargo test --offline --locked --test remote_workspace --test remote_protocol
  --test remote_diff --test remote_auth --test remote_subscription_tls:
  **64 pass** (26/11/1/11/15). New real-owner/registry file test verifies selected
  child-only listing, no raw bytes/roots, shared + actual mailbox routes,
  path/symlink/missing/file-as-directory refusal, stale session/revision,
  64-entry cap, Busy, revoke and unchanged journal/checkpoint.
- cargo check --offline --locked, cargo fmt --all --check, git diff --check and
  strict blueprint validator pass. Existing compiler warnings remain.
- zsh tools/stage2_android_source_check.sh
  /private/tmp/zenpi-host-files-final2-20261002/android: exit 0. Main/unit/device
  Kotlin compile; **151 JVM tests** pass in 28 classes. Three new codec tests
  validate exact 10 MiB boundary, quoted native paths, readonly request,
  selection fences and malformed scope/path/flags/numbers/duplicates/caps.
  New Compose file-insertion/navigation/oversize test and background browse
  calls compile only; no device execution or UI screenshot acceptance.
- Five Python smoke/lifecycle/blueprint modules: **53 pass**.
- ./gradlew --offline :app:testDebugUnitTest :app:assembleDebug: exit 1 before
  compilation, wrapper lock ~/.gradle/wrapper/dists/gradle-8.11.1-bin/
  bpt9gzteqjrbo1mjrsomdt32c/gradle-8.11.1-bin.zip.lck EPERM. No new APK.

Earlier source-check runs predated final hidden-path/offline UI/Compose fixes;
only final2 jars are final-source evidence. New operation TLS/device execution
is not covered by the existing subscription TLS regression count. Directory
picker safety/source evidence does not prove physical QR or same-LAN sync.

SHA-256 source:

```text
src/remote/files.rs 326274ec1b0b441d0fe8b2121b1ffb055764780dbacaf4c887c629d53d9b51fe
src/slash_actions.rs 507631a9dc5b1cf7bc039249491cc6ce9752035baa685f7665300e5265659384
src/remote/protocol.rs ccf38ccb7755f6a0981f2aabe826b8380737062591201054e492f50e86e9b061
src/remote/host_adapter.rs be70100cca6be1b8c0e383c6a825c00fde6dcad4fa6aaf6d7d6cf79ab7fe3cd4
WorkspaceFilesCodec.kt 131603f47266a8f6a6b2f5ab21c9add986f3022fbaed483cff603e7bca48d99d
PairingViewModel.kt 36c5e59a39e72e7b21179771ed2abc78f58d46507d449780250e361be8c254c0
MainActivity.kt dd9c068627cea5660767e0f8062f6ed285390ce2de6c15fdf25a6ad4b23313eb
```

Compiled final2 evidence:

```text
main.jar 0b83b3301ba5261c6ee898ca4121d2fff39ca5b92d50aa21234e5c4bca5655cb
unit.jar 0452300dca4aa2b565559579cfb9249deef115d51072a7adfd5c8a1b058a7260
instrumented.jar 6fb61d0c0dcb6d55002307fb3cd7b25bdd5a64e91d5cdd11a88ed1c99fa4c298
```

No phone upload, recursive full archive browser, effective worktree quotas,
full resource/TUI/headless parity, provider delta replay/ACK, interactive PTY,
fresh API31/34/35 lifecycle or signed release/migration acceptance. No agents,
permission bypass, commit/push, unrelated rollback or acceptance promotion.

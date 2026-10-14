# Android Secondary Tab Cache

## Progress

The preceding goal turn implemented and verified bounded managed Git. This
turn advances S2-033/035/038: Android formerly discarded the entire secondary
projection on encrypted cache restore, making directory/worktree navigation
disappear during offline/process restoration. Cache v2 now stores the redacted
secondary display with the transcript/cursor in the existing atomic encrypted
record: stable IDs, names, kind, ordering, active ID, revision and displayed
concurrency. It never stores root paths, workspace owner/revision or secondary
capabilities. Restored project/tab control is false; offline command factories
and production ViewModel refuse mutations. Live authenticated attach replaces
the cached display and supplies current authority.

The live and cache parsers share one secondary view validator. Foreign project
bindings, invalid revisions/IDs, duplicate identities, invalid selection,
multiple/non-leading main tabs, invalid count/kind/name/concurrency, paths,
capability injection and unknown nested cache fields fail closed. Existing
strict cache v1 records still load without invented tabs and become v2 on their
next write. The AES-GCM record version/AAD, Keystore aliases and pending-command
envelope are unchanged. No host wire version or Rust code changed this turn.

## Validation

`./gradlew :app:test :app:assembleDebug :app:assembleDebugAndroidTest` passed;
debug/release each contain 30 passing unit tests. Four new cache tests cover
secondary display roundtrip/authority stripping, v1 migration, corrupted
secondary data and nested authority/field injection. An existing command test
initially expected absent secondary display; its assertion now requires exact
retained display with false controls, while still proving command refusal.

The production desktop/Android direct-private-interface harness passed all
13 initial instrumentation tests, one actual process-death network restoration
test and two independent-process cache fixtures. It decoded the actual terminal
QR, used pinned TLS without adb reverse and checked shared desktop checkpoints.
After Android moved the two real directory tabs, an independently restored
offline ViewModel retained the actual host order/selection, had no controls and
created no pending command when asked to select a tab. A Compose test verified
cached names/selection render with disabled tabs and no mutation buttons.
The isolated force-stop cache fixture now retains reordered directory/worktree
display and a pending command across independent processes, still rejecting
new cached-authority commands and preserving the exact pending envelope.
Live restore tests verify fresh attach restores only current host capabilities.

The authoritative passing receipt is
`desktop-android-secondary-cache-20261002.json`. It sets
`directory_offline_cache_verified=true` and matches all current artifact hashes:

- Rust: `fa459145d023b2678ab257053521e3992ae89c78848bf9c46ad418c7c145ce35`
- APK: `87ef698dcc7331c9ff9531459ab9962f3ff188b708add601087d4c2909054cd9`
- Test APK: `d09a1c276dd371d769fd2fcf6b97a08f9d8319ffde38d7ead0137af13fb0f21e`

Release assembly passed, but the installed NDK directory lacks
`source.properties`; Gradle packaged `libandroidx.graphics.path.so` without
stripping it. This is a packaging warning/residual release-environment issue,
not proof of signed or reproducible release acceptance. Strict blueprint
validation, five blueprint tests, four harness tests, Python compilation and
whitespace checks passed. The final screenshot was visually inspected; its
scrolled top tab-label area is still clipped, so full responsive UI acceptance
is not claimed. Owned emulator PID 90794/session 26314 exited; Wi-Fi/mobile
data were enabled and no reverse mappings existed before shutdown.

## Remaining Full Scope

Offline cached data is stale display, not execution state authority, and only
the selected workspace's secondary projection is currently cached. Remote Git
add/remove still requires durable intent/reconciliation; real per-worktree
scheduler quota and execution cwd routing remain open. Prompt, approval, diff,
PTY and live event/ACK parity, physical phone camera/LAN validation, API matrix,
release/migration/rollback and execution-controller acceptance remain required.
No blueprint item was marked accepted, no workers were launched, and no commit
or push was made. All 36 items remain unchecked and the full goal stays active.

# Current APK Source Attribution Gate

Provisional S2-030/S2-051/S2-055/S2-071 progress at 2026-10-03T18:24:10+0800.
Previous goal turn is classified as progress: canonical historical session
resume and Rust/Android tests changed authoritative source/evidence. This turn
inspected current packaging/network sources before choosing the next action.
No nested agents, worker admission, checklist promotion, commit/push or cleanup.
Dirty user/generated changes preserved. Goal remains active.

## Observed Gap

The existing debug APK is dated October 2 06:54 and device-test APK October 2
06:56, before current Kotlin/session/terminal changes. Pair, desktop, headless
and scanner harnesses hashed those binaries but did not bind them to current
source. A later successful old-binary run could be misattributed to current
features. Historical receipts are preserved, not silently upgraded.

## Change

tools/stage2_android_build.py invokes the pinned wrapper with exactly:

```text
./gradlew --offline --no-daemon --no-build-cache --rerun-tasks :app:assembleDebug :app:assembleDebugAndroidTest
```

cwd is android. Source/config/wrapper/corpus inventory is compared before and
after the actual bounded Gradle command. Successful completion and both APK
hashes are atomically recorded in android/app/build/stage2-device-build.json.
Failure/timeout/source mutation invalidates old authority and publishes none.
Generated output, local SDK/signing state are excluded, while packages named
build and resource logs inside src remain inventory inputs. Symlink sources/
artifacts and unreadable directories fail closed. --verify-only is read-only.

All four device harnesses verify this record before any adb subprocess or APK
installation, recheck it before successful evidence and include its digest in
receipts. Source additions/deletions/edits and replaced/missing APKs invalidate
it. No timestamp shortcut, manual stamp or current-JVM fallback exists.
This is local debug source attribution, not cryptographic attestation, complete
dependency/toolchain pinning, signed release or hermetic reproducibility proof.

## Verification

Commands run from /Users/mac/Github/zenpi, no signing credentials provided.

- python3 tools/test_stage2_android_build.py: 10 passed, final exit 0. Covers
  both APKs, exact Gradle tasks, stale source/artifacts, missing/failed/invalid
  record, additions/deletions, symlinks, excluded local state, included src
  build/log paths, failure/timeout/mid-build edit and all four pre-adb fences.
- python3 tools/test_stage2_android_pair_smoke.py: 5 passed, exit 0.
- python3 tools/test_stage2_desktop_android_smoke.py: 14 passed, exit 0.
- python3 tools/test_stage2_headless_android_smoke.py: 19 passed, final exit 0.
- python3 tools/test_stage2_scanner_android_smoke.py: 6 passed, final exit 0.
- python3 tools/test_validate_stage2_android_lifecycle.py: 15 passed, exit 0.
  Total 69 deterministic Python tests; no connected device execution.
- python3 tools/stage2_android_build.py: exit 1, no provenance published.
  Repeated once after adding bounded safe diagnosis; reported Operation not
  permitted. ./gradlew --offline --no-daemon --version separately confirmed
  wrapper FileNotFoundException at the user-home distribution .zip.lck file.
  This is a managed filesystem restriction, not an APK compiler/test result.
  Prior isolated-home diagnostic also records a daemon socket restriction.
  No filesystem/socket permission bypass was attempted.
- python3 tools/stage2_android_build.py --verify-only: exit 1, correctly
  refuses missing provenance; not a packaging pass.
- python3 tools/stage2_scanner_android_smoke.py --serial emulator-fixture
  --expected-api 35 --receipt /private/tmp/stage2-no-stale-scanner.json:
  exit 1 before adb, missing provenance. No APK install/device/network claim.
- python3 tools/validate_stage2_android2unix_blueprint.py --strict: exit 0,
  36 unfinished items. git diff --check: exit 0.

No Rust/Kotlin product source was changed this turn; those suites were not
rerun merely to inflate evidence. All observed execution handles are terminal.

## Digests

```text
stage2_android_build.py c3f2071433235e0e39333c5585265f5a985a4747554794020a590858c89b4cc7
test_stage2_android_build.py e6f3a39b9e41e748721121f52f6a21dd00ab1013cbd1fe25fe009c8cd9ce2a75
stage2_android_pair_smoke.py 5c36a56e59575c462ef2a4290952ce83fbbadf3ddd1818cc40ba3ae5155a21fb
stage2_desktop_android_smoke.py eb9724246712f91072f5091bf0e48cbeb132c14604d72120c0fe36075950bbc9
stage2_headless_android_smoke.py 60d1b4d2cba8f0c0b13789d5664ac8fd768d8d90634cb4d66ef973192652cf42
stage2_scanner_android_smoke.py d92c49c6835ea4675bef1326691267dd7d7655ca2c148055bcdc3b847a3b4492
historical app-debug.apk 6641c59b6ea518203e365c790ba79638b5f01ff8be573f109db3c47ca7184503
historical app-debug-androidTest.apk ed06fc527f19455fb42fc02e6ba3bd026c60b82000597b2bd175bbb8eb88c665
```

Current source-bound APK, device interaction, physical camera QR/direct LAN,
connected lifecycle and signed reproducible release remain open. Broader TUI
parity also remains incomplete. No full-goal completion or acceptance claimed.

# Session Catalog Continuity

Provisional S2-033/S2-036/S2-054 progress at 2026-10-03T20:20:28+0800.
Previous goal turn was progress: task-list continuity was implemented and
verified. Current worktree inspection found the historical session reader still
allowed arbitrary offsets and accepted backward pages solely on a digest.
This turn repairs that real client path. No nested agents, checklist promotion,
commit/push, worker launch or cleanup.

## Changes

SessionCatalogCodec restricts requests and replies to an adjacent next/previous
page, refuses empty noninitial pages and duplicate IDs shared with the current
page, and bounds valid offsets to the last possible 16-entry page of a 512-entry
catalog. Its history validator requires consecutive scope/digest/offset chains,
refuses repeats against up to seven retained forward pages, and requires a
backward reply to equal the retained preceding page. PairingViewModel validates
before issuing the read and before publishing the page; invalid replies enter
the existing catalog-changed error state. A stale successful read closes the
catalog rather than leaving an indefinite loading state.

Compose resume confirmation is now keyed by page offset as well as host,
session, catalog digest and open state. Changing pages clears the unconfirmed
target. The instrumented source test verifies it disappears both when moving
forward and when returning. Main projection/cursor/cache, request persistence,
host ACL/CAS, session resume transaction and unknown-outcome semantics are
unchanged. The client does not recompute the whole catalog digest or retain all
512 sessions. These are source/client consistency improvements, not device
or LAN acceptance.

## Verification

All commands ran from /Users/mac/Github/zenpi.

- zsh tools/stage2_android_source_check.sh /private/tmp/stage2-session-catalog-continuity-20261003:
  exit 0; main/unit/instrumented Kotlin compilation and 337 JVM tests pass.
  Instrumented compilation does not execute Compose on a device.
- ZENPI_HOME=/private/tmp/zenpi-resource-reload-test-home cargo test --offline
  --locked --test remote_headless_commands
  headless_new_session_uses_existing_owner_and_retries_original_receipt
  -- --exact --test-threads=1: exit 0, 1 pass. Controlled Echo host-domain test,
  no production binary/TLS/device execution.
- python3 -m unittest discover -s tools -p 'test_stage2_*.py': exit 0,
  86 pass; invalid-LAN argparse errors are expected negative fixtures.
- python3 tools/test_validate_stage2_android2unix_blueprint.py: exit 0, 5 pass.
- Strict Blueprint validator, cargo fmt --all -- --check and git diff --check
  exit 0. All 36 authoritative items remain unfinished.
- Host release --verify-only passes for unchanged Rust source. Android build
  --verify-only exits 1: missing APK build provenance. No stale APK used or
  unchanged blocked Gradle/socket build repeated.

```text
baseline HEAD 1a8ff1acf72e0816fad503a05df2b1d18e07361b (dirty worktree; no new integrated revision)
blueprint d76e70e369f11f0fd3d1905d56d79c2a89401b92215da48ad85314f390d96d97
specification 403e6339b0318a420d93600fc52926acf6a9e46ec899bd9a9edb03c235190a28
main.jar 09b4d5ce45a199aa29a2e7265e2c4dea06ad2ed4b81577c8b2d0771aa6bc6caa
unit.jar 041fd844c2f19a7415faf3b5fc151eba22d5460761d7526a2d6ae0bd870e3439
instrumented.jar c5ca3ab9f31fa8c95714356f94b6e5f17247d0711e6daa808a907d20b6328093
SessionCatalogCodec.kt 398f9eef4a40a4f5cd8efd4ce732266f24ae70ceb447d1ed4f977d82b019fd96
SessionCatalogTest.kt e7a080930fa71675ed5cc421189d50d06fbcc5064daa6a403841cd22380294e3
SessionCatalogScreenTest.kt 3e281a2513695eb45ede3652a5c750dd0ce659db40a9a874d72415d3778e379c
PairingViewModel.kt 07fdf56907cde5efb9b034f6b47283e248dcf628ea4b73fbf88d4ae9f29fb30c
MainActivity.kt 1d5909506bc1b2e83940b0eb8f4eeb587b8e4be1662e9de814d071919938335f
```

Goal remains active. Current-source APK, production desktop-to-Android TLS sync,
physical camera QR/direct LAN, API 31/34/35 connected lifecycle and signed
install/upgrade still require their own current behavioral acceptance.

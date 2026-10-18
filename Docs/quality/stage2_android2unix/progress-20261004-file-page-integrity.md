# Stage 2 Retained File Metadata And Paging Integrity

Recorded 2026-10-04T06:10:56Z. Partial S2-036/S2-054 evidence, not item
acceptance. Canonical cwd /Users/mac/Github/zenpi; dirty HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b. Existing changes preserved; no nested
agents, reset, push or permission bypass. All 36 Blueprint rows remain [ ].
Blueprint SHA-256 d76e70e369f11f0fd3d1905d56d79c2a89401b92215da48ad85314f390d96d97;
spec SHA-256 403e6339b0318a420d93600fc52926acf6a9e46ec899bd9a9edb03c235190a28.

## Change And Criteria

Previously retained file selection checked current project/session/subtab scope
without revalidating entry metadata or page/history shape. WorkspaceFilesCodec
now validates name/path agreement, canonical u64 size, directory/null-size flags
and exact 10 MiB attachability. Listings validate requested-directory paths,
unique entries, at-most-64 entries, snapshot, aligned bounded offsets, nonempty
continuations and full forward pages. Decoder, request, reference insertion and
retained selection share these checks. FileReadPolicy navigation and attachment
also require a consecutive same-scope at-most-seven preceding-page chain with
unique paths. Initial fresh browse remains independent for recovery.

Expected/actual: inconsistent sizes/names/directory flags cannot authorize
references or attachments. Malformed snapshot/offset/next/truncated/empty tail
and mismatched, duplicated or oversized retained histories refuse navigation.
Valid 64-entry pages, tail pages, reverse pages, quoted references and read-only
directory browsing continue to pass. Current Rust-owner/Kotlin file lane also
rejects a mutated retained snapshot while decoding real desktop responses.
Prepared Compose assertions disable malformed-size files and directories;
paging fixtures now contain real distinct entries. No device execution claimed.

## Verification

The prior checker handle was terminal with exit 1. Its log identified an extra
closing parenthesis in WorkspaceFilesScreenTest. Corrected that fixture and ran
the complete checker again; no production/test source edits after final launch.
Final source log timestamps: 2026-10-04T06:07:24Z through 06:10:49Z, exit 0.
OUT and per-lane ZENPI_HOME isolate fixtures under /private/tmp; no credentials
or provider configuration changed.

- zsh tools/stage2_android_source_check.sh /private/tmp/stage2-file-page-integrity-final-20261004:
  production/unit/instrumented source compilation, 549 ordinary JVM tests,
  production QR corpus and all explicit current Kotlin/Rust lanes pass. Includes
  six-test desktop target, real Agent prompt/tool and worker cancellation lanes.
- python3 -m unittest discover -s tools -p 'test_stage2_*.py': exit 0, 137 tests;
  /private/tmp/stage2-file-page-integrity-final-python-20261004.log.
- python3 tools/test_validate_stage2_android_lifecycle.py: exit 0, 25 fixture
  verifier tests, not connected API lifecycle acceptance.
- cargo fmt --check, git diff --check and strict Blueprint: exit 0;
  36 [ ], zero [_]/[x].
- Host verify-only initially refused a binary changed by Cargo test builds.
  python3 tools/stage2_host_build.py rebuilt/reattributed the real debug binary
  after those lanes; subsequent --verify-only passed, exit 0.
- python3 tools/stage2_android_build.py --verify-only: exit 1, missing APK build
  provenance. Unchanged Gradle/local-socket/ADB blockers were not retried or
  bypassed, and historical APKs were not installed.

Final log /private/tmp/stage2-file-page-integrity-final-20261004.log.
File exchanges /private/tmp/stage2-file-page-integrity-final-20261004/android-file-results.json.

```text
WorkspaceFilesCodec.kt 0f7f6d6bbf261747faf12f14e1c58041c25ba2b5bf45651b1c2af5cb2af3f7a7
FileReadPolicy.kt c2b661350e8ffc2999d38346f7d7b0b4725a1f3f161a021e88f465e56ac1fc11
WorkspaceFilesTest.kt e4d985cd2f1fefaddf78f365086932b6187846855e39680165476c8cc702de26
AndroidRustFileTest.kt 80eb4818ac236d3591529730f7aa5fe95339c9f413aead26b83e99890d460762
WorkspaceFilesScreenTest.kt ab3f6666a0be2bf1f61687081b70ccbdc13b5a2b43e4e7c3f83a4cd4153254a0
source-log 33265f2b9d41d5db209a80e94a7da21bc1e436d601bad1de365c1007d1857e94
file-results 7aec136cb1403b75b6982658c34f4f3b8233a531b6d9d225c5ed77435c63d0ce
python-log 4ee41282dd0549713c7a51f10e0fb2d033e50150d54ec174ec7ac3c56e2ec282
main.jar 61cf6ced90490d1846705f23652e8567c0548a658c9b3af2c25af44bd88d4f26
unit.jar 37b8a0f8e3ff2b24df321d5feb8714eea3cdb7f9a557988ec740fb50c07adf01
instrumented.jar 68d38e5dd35b839a71e1918a29b4a3a741c8d98a8a39a7a1b332377b0cf53b8e
host-receipt 19f10489a7667c6a66f608e367f437c6a2fc3bf5969339e1f1ace8184d0bb10c
host-inputs 373a8f5697a5bc95c10eda94c9b9b35facda736cd84bcdb0c0fa3e8703c8e58f
host-binary 2939439799b49d0e22f6072e95288a44b239e4d8eb999ad9f44bfaaf5480a861
```

Goal active/incomplete. Source-bound APK, physical camera QR/direct-LAN
bidirectional synchronization and recovery, connected API31/34/35 lifecycle,
complete desktop parity and signed reproducible release remain required.

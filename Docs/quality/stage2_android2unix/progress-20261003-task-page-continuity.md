# Task Page Continuity

Provisional S2-033/S2-036/S2-052 progress at 2026-10-03.
The previous goal work added later-page completed-task reads and host-domain
regression evidence. This turn fixes a client list-consistency gap discovered
in the current source. No worker/subagent launch, checklist promotion, commit,
push or cleanup.

## Change

WorkerStatusPageCodec now refuses unanchored nonzero offsets, skipped page
offsets, reads after a terminal page, and duplicate request IDs shared with
the immediately preceding page. Its bounded navigation-history validator
also rejects duplicates against up to seven retained pages, scope/digest/count
changes and discontinuous history. PairingViewModel calls that validator before
publishing a page. Invalid replies follow the existing list-changed error path
and cannot publish a new page-only read/cancel target. Reload starts at zero.
This does not recompute the complete host list digest or detect repeats outside
the retained navigation window; it preserves bounded memory and existing
host-side device/owner/request checks.

Two new JVM tests cover adjacent and nonadjacent retained duplicates, skipped
pages, unanchored offsets, terminal continuation, scope/digest/count mismatch,
and the seven-page memory bound. The older valid-page fixture was corrected to
give each task its actual distinct page-offset identity rather than repeating
task-0 on the second page.

## Verification

- zsh tools/stage2_android_source_check.sh /private/tmp/stage2-task-page-continuity-20261003:
  exit 0; main/unit/instrumented Kotlin compilation and 335 JVM tests pass.
  Existing deprecation/nullability warnings remain. Instrumented compilation
  is not connected-device execution or APK packaging.
- python3 -m unittest discover -s tools -p 'test_stage2_*.py': exit 0,
  86 tests pass. Invalid-LAN argparse errors are expected negative fixtures.
- Blueprint validator unit tests: exit 0, 5 pass. Strict Blueprint validator,
  cargo fmt --all -- --check and git diff --check pass.
- Rust host paged-status regression: cargo test --offline --locked --lib
  remote::worktree_host::tests::paged_worker_status_is_device_scoped_digest_fenced_and_strips_output
  -- --exact --test-threads=1, with
  ZENPI_HOME=/private/tmp/zenpi-resource-reload-test-home: exit 0, 1 pass.
  An initial run without writable configuration isolation failed before
  exercising the gate with configuration I/O Operation not permitted.
  This is a controlled host-domain test, not binary/TLS/device evidence.
- Host release --verify-only succeeds against the unchanged Rust source.
- Android build --verify-only still refuses missing APK build provenance;
  no old APK was installed and no unchanged blocked Gradle attempt was repeated.

```text
main.jar fc732f2ae4a3019f48f5e577d0b81499c8f8bbfdee6ca5a32f941bc5f4180e95
unit.jar 6074e22e9977b76195529275a0bdf0bf1f0b098d9d944c0dd1c94e192fe0f717
instrumented.jar 0ad2db8bff6a91bf295b2eb0ffcb8f420207ebfbf22140a859a87305a408ae5a
WorkerStatusPageCodec.kt db739fed283920d023d90fbd2f5006d8b1de87b3706399187bd3b084952a5ec7
PairingViewModel.kt 7269410a2cd7572974ff123eea6c8d17892210cf0ad94396cecb12f623e33291
WorkerControlTest.kt ae50d02af9e15d371617c0828d9c5a13858f683f673349c912def70e71d63203
```

All 36 Blueprint items remain unfinished. Current-source APK, real-host Android
TLS sync, physical camera QR/direct LAN, API 31/34/35 connected lifecycle and
signed install/upgrade acceptance remain open. Goal remains active.

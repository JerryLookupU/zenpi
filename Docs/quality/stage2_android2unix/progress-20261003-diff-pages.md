# Bounded Diff Paging

Provisional S2-019/S2-032/S2-036/S2-050/S2-052/S2-054 progress at
2026-10-03T21:26:48+0800. Previous goal turn prepared the project-access device
workflow. This turn adds a product path beyond the former 64 KiB remote diff
summary. No nested agents, worker launch, checklist promotion, device launch,
commit/push or unrelated cleanup.

## Product Contract

Existing diff Read operation accepts optional offset/digest together. Initial
0/null loads a bounded document; continuation binds its SHA-256 and UTF-8 byte
offset. Existing project/session/current active tab/revision/ACL and busy-owner
checks remain, with request ID/sequence refusal added. Each page recomputes the
sanitized scope/path/patch/status/flags document and rejects digest changes.
Pages contain at most 8192 bytes; full patch cap is 1 MiB. No frozen host copy,
provider call, file mutation or journal/event/ACK change is introduced.

The shared controlled Git reader uses a per-operation patch limit. Legacy/local
64 KiB behavior, status cap, private-path exclusions, literal paths, no external
diff/textconv/fsmonitor, cooperative deadline and child cleanup remain. A paged
directory read reaching its 32 untracked-file/status bound is explicitly
truncated. Unknown secrets and concurrent filesystem edits are not an atomic
repository snapshot; post-redaction oversize refuses instead of exceeding cap.

Android tries the paged route, falls back once only for initial invalid_request
to old-host summary, and offers previous/next controls for valid pages. History
retains seven pages with consistent scope/path/metadata/digest/total/offsets.
Errors retain verified text but disable continuation; refresh restarts. Scope,
authority, lifecycle and pending-write fences clear transient pages. No offline
cache or apply/revert write is added. DiffScreenTest is now in the explicit
desktop suite: 69 tests, or 70 with Doze (20 base plus 49 control plus one Doze).

## Verification

- Final remote_diff test: exit 0, one broad real-Git fixture passed. Reconstructs
  over eight pages beyond old summary cap, verifies full tail, immutable journal/
  checkpoint, changed digest, missing digest, malformed UTF-8 byte offset,
  1 MiB truncation and existing ACL/private-path/scope refusal. Domain adapter
  fixture, not TCP/Android/physical network.
- cargo test --offline --locked --lib slash_actions:: -- --test-threads=1 with
  ZENPI_HOME=/private/tmp/zenpi-diff-pages-home: exit 0, 31 shared regressions pass.
  Includes literal path, private exclusions, unborn repository, controlled
  cancellation/deadline/cap and child reap tests. Rerun after the final paged
  status/truncation flag change also passed all 31 tests.
- zsh tools/stage2_android_source_check.sh /private/tmp/stage2-diff-pages-final2-20261003:
  exit 0; main/unit/instrumented Kotlin compile; 362 JVM tests pass. Existing
  Compose test covers paging callbacks/error/offline/scope/history fences in
  source only, not executed on device.
- Python Stage 2 discovery: 94 pass; lifecycle: 17 pass; instrumentation: five
  pass; Blueprint tests: five pass. Strict Blueprint, cargo fmt --all -- --check
  and git diff --check pass. All 36 checklist rows unfinished.
- Early Kotlin check failed on set-expression precedence; corrected and final
  checks above pass. Failed output is not counted as passing.
- Debug attributed build and verify-only pass for new Rust sources. Release
  attributed build session 94115 observed terminal exit 0; release verify-only
  also passes. A later debug test relink invalidated its artifact hash; debug
  was rebuilt and verified again, never manually stamped. APK verify-only refuses missing current provenance;
  no stale APK or unchanged blocked Gradle/socket retry.

```text
main.jar a604419a24f99e6ba48357455066ed6bdcbd1517e442e9b52c3c3af2e3e72e7c
unit.jar 2a76300ad5f41abdebb3c2b04c8b9f95546379eda056ae4b1eead438f62de2dd
instrumented.jar 0078e4aad29c1d6655ed4ad1b02b571f6c174800f3328c7739f033a26178a7ab
src/remote/diff.rs 77482a26eba6219d920b75c8cbe7c11f06ba2701d31647e847d9c2339f65053b
src/slash_actions.rs 929796c403035c096fb03e6f395574946b9a784daab08545701abe3e91b62280
tests/remote_diff.rs 5953204311961b26d0f13530396a77eda30a9eb803caf45ead2f101c1d8cd48f
DiffProjection.kt 923025681bebadde3a26069d4fef93122f3388b196e082a194d16668dac7454c
HostClient.kt d4c99c981edad4d157772320c13045aca0c6c6da325a12588317bc0852b19878
PairingViewModel.kt 4824ff5b7b2b2d9615db947129ad3efe7f54deef40121711e02102bf48d6e88b
MainActivity.kt f58d06e7e0d1b4223d8cb58012aa032c074530ec59f7678f6c9d6800f86fc0ff
debug receipt 286347f22d43b7085f70b6986c8ac0d25d6a278d7ef8b146a5cb056403eb8136
debug binary 1aff8e1c98ee5490ab4e6d91fd578c1b94c317b99eb2302e59da2ec22ed8412d
host inputs 88d16e5b7b40331629dab164b6475d5af57c79b4403fd3d598f62a0d0af01906
release receipt e5fe7e1c5327f89a6af76c33e44dd26092e55102c31f96c79f5fed938c1b5c04
release binary a02830728091019a43f105831c6efb32aa72de8fc8976591513aefdbce15f4df
```

Goal remains active. Current-source APK, connected paging and full parity,
production desktop/Android TLS sync, physical camera QR/direct LAN, API 31/34/35
lifecycle and signed install/upgrade remain required behavioral gates.

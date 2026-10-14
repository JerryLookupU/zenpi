# Stage 2 Worker Message Identity Parity

Recorded 2026-10-04T09:09:44Z; partial S2-036/S2-054 evidence, not acceptance.
Canonical cwd /Users/mac/Github/zenpi, dirty baseline HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b. Existing/user changes preserved;
no nested agents, reset, push, blocked socket retry or permission bypass.
Blueprint SHA-256 d76e70e369f11f0fd3d1905d56d79c2a89401b92215da48ad85314f390d96d97;
spec SHA-256 403e6339b0318a420d93600fc52926acf6a9e46ec899bd9a9edb03c235190a28.
36 [ ], zero [_]/[x]. Previous goal turn was progress: ordinary conversation
history opaque-identity parity with actual Rust journal/detail interop.

## Finding And Change

WorkerHistoryCodec accepted opaque nonblank/128-UTF-8-byte/control-free turn IDs,
matching the host transcript contract. TranscriptPageCodec.decode nevertheless
required ASCII-only IDs for worker detail/output replies. A valid retained
worker history entry could construct a detail request but its reply would fail
decoding. Initial selected-turn requests also checked tool summaries but did
not validate the turn identity before frame encoding.

TranscriptPageCodec now applies one local opaque-identifier predicate to
selected live/history turn construction and worker result turn identity. It
preserves Unicode/punctuation/whitespace-with-content exactly, rejects blank,
control-bearing and over-128-byte IDs, and retains all project/session/task/
subtab/role/digest/selected-turn and continuation checks. No host authority or
protocol/cache schema change, path interpretation, identifier normalization
or automatic mutation retry was introduced.

WorkerHistoryTest covers valid incoming history, exact framed detail request,
two-page response and retained back navigation with Unicode/punctuation and
128-byte IDs; foreign turn/task/subtab and malformed selected identities refuse.
AndroidRustWorkerReadTest additionally mutates actual current Rust worker
detail/output results to exercise opaque and invalid reply IDs. These mutations
are decoder-contract checks, not host-dispatched altered IDs. The surrounding
real worker/list/history/detail/restart/read-only owner lane still runs unchanged.
No new live provider or connected Android execution is claimed.

## Verification

zsh tools/stage2_android_source_check.sh /private/tmp/stage2-worker-detail-identity-20261004:
exit 0, 581 ordinary JVM tests, main/unit/instrumented compilation and all
explicit Rust/Kotlin lanes passed, including six-test model/read/worker target,
terminal QR, software P256 proof, workspace/session/ACK/restart and real
prompt/tool/running-worker cancellation. Process 22365 was polled through
authoritative exit 0; a silent ACK observation was not treated as completion
or restarted. Per-lane temporary ZENPI_HOME roots isolated.

- python3 -m unittest discover -s tools -p 'test_stage2_*.py': exit 0, 142 tests;
  /private/tmp/stage2-worker-detail-identity-python-20261004.log.
- python3 tools/test_validate_stage2_android_lifecycle.py: exit 0, 25 fixtures,
  not connected API31/34/35 behavior.
- python3 tools/test_validate_stage2_android2unix_blueprint.py: exit 0, five tests.
- cargo fmt --check, git diff --check and strict Blueprint: exit 0, 36 [ ],
  zero [_]/[x]. Authority validation does not prove product completion.
- python3 tools/stage2_host_build.py and --verify-only after all Cargo lanes:
  exit 0, current debug artifact attributed; no reproducible release claim.
- python3 tools/stage2_android_build.py --verify-only: exit 1, missing APK
  provenance. No repeated blocked Gradle/ADB attempt or stale APK install.

```text
TranscriptPageCodec.kt 7fc52af473c8becf230bb04c911e441b6b414391b552262af023a16a140530f8
WorkerHistoryTest.kt 03c98fa1c12618df7c4ae9a1f2cb8f0ea19d00a35415d9af896a54162a540060
AndroidRustWorkerReadTest.kt f9c27659d5ce28024aaa355909e15f3c40b1e346618824535e90bb1d1e33d04e
python-log ff1f82481497cad376d51a1f5c33e2287a5fcea4393fe6d973358d6ace805c5e
source-log b7ee1933fa269f58a541c8126a83542a3e7f7a4cf695fa69ef76f6be12746aa0
worker-read-results 3ec663fd4e500d1cb07862cf4bd3b6403a8be006deeac75c4c093a2744133deb
main.jar 763a81ac456cde082e92e61dfbcacac7d9ab8efb068565a5f4929cc14cc7e161
unit.jar 23e07370f918259782d6fa8d244bb3cccf184023c510f90efbdfcb92d52bbb48
instrumented.jar f95a635bb1b8d33d66febe3cf15afe0db9afd4ca9d63fb00a5f126e9d1e69630
host-receipt ba766bd53d7cee9b4235c3c79c41e52e8666a234e2a79dc1719920168d97d471
host-inputs 08312e22d68633b57228db66f7d64511bc87dc758579124ef9de588e3a466219
host-binary 40f9f134ed62fd51a71dc6e8d24022f0122dced68faf12f6a52f8b5ea145607f
```

Goal active/incomplete. Fresh APK attribution, physical camera QR/direct-LAN
bidirectional sync/recovery, connected API31/34/35 lifecycle, complete parity
and signed reproducible release remain required. Source repair is progress;
no completion or blocked status asserted.

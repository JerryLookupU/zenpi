# Terminal History Continuation

Provisional S2-032/S2-036 progress on 2026-10-04. Previous turn repaired Busy
cache snapshot recovery and verified source gates, so it was progress. Current
terminal paging codec, ViewModel navigation, Compose history controls and host
history contract were inspected. No nested agents, worker admission or
authoritative checkbox promotion.

## Repair

Terminal continuation previously matched instance/scope/hasMore but did not
validate the retained page itself. Same-identity corrupted text/offset/digest
could authorize another request or back/forward navigation, even though newly
received pages were validated. Chain identity alone was insufficient.

TerminalPageCodec now validates retained digest, bounded offsets/total, UTF-8
byte length, 8192-byte text limit, hasMore/progress and sanitized controls.
Request and decode share prior-page admission. Selection checks validate the
displayed page as well as current terminal identity. Back/forward history checks
all retained pages, consecutive scope/digest/total/offsets and seven-page cap.
ViewModel and Compose both validate the complete retained chain before Next;
Previous uses the same chain rules. Explicit Refresh remains available when a
current terminal exists. No network request, cache/ACK advancement, shell input
or terminal creation is added by this repair.

Two new JVM regressions cover malformed previous pages, matching-identity text
corruption, invalid first page, overlong chains and invalid current-page text.
Existing UTF-8/foreign/digest/offset/page-bound tests remain. The prepared Compose
history case now checks invalid displayed text/offset/digest and corrupted prior
text disable both navigation directions while Refresh remains enabled. That
Compose case is compiled, not device-executed.

## Verification

- Revalidated current source with `zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-terminal-continuation-revalidated-20261004`: production,
  JVM test and instrumented sources compile; 412 JVM tests pass. Instrumented
  sources are compiled only, not device-executed. Existing warnings remain.
- Stage2 Python discovery: 115 passed; lifecycle verifier tests: 21 passed;
  Blueprint verifier tests: 5 passed.
- Strict Blueprint, rtk cargo fmt --all -- --check and git diff --check pass.
  All 36 rows remain [ ]. Rust production/binary unchanged; debug host build
  provenance verify-only passes.
- APK verify-only exits 1: missing current-source APK build provenance. No
  stale APK install, unchanged blocked Gradle/ADB/TCP retry, real camera scan
  or direct physical LAN execution was attempted.

```text
TerminalPageCodec.kt 362c253fc4bf18511dc20099e015e0ceff947a5d1370f28a8e5e97d70dab2a1c
PairingViewModel.kt f743a462797bb9e2a4236c5d12d9bc0b6069e3309d02b9c1de4974f3758ce051
MainActivity.kt 3a456424d7b04158c296a17eae5f333809c4d23ec387613c6feec59f8db78501
TerminalPageTest.kt 5212f789cb711ba61b07f5718b23c2703a7c3ce2912d94c97731497b0a1e30af
TerminalHistoryScreenTest.kt be2479efd13b579f479b7b0bc3323c41c977c692ffd79f4bd5cfe000576a8f12
debug-host-receipt 5ccc40c62c23203f07e0b31dcda9167132c7cf12af0b28298ffeffa18f63114c
revalidated-main.jar a059ff8dcd17a3f2d98b01a84c8f0802832488c27d5926cb4a6ec3fd6d136a2e
revalidated-unit.jar da5c92378a494e1a666dcf6583792951e47ed86407ec9c060d5fdbf458cc9b48
revalidated-instrumented.jar 29be236a1b216e061740b91afc854e820f264cc60cb16c1af021674617a09960
```

Goal remains active. Current-source APK, actual QR/direct physical LAN sync and
recovery, full parity/API lifecycle and signed release remain incomplete or
unverified. No full Blueprint item or final workflow acceptance is claimed.

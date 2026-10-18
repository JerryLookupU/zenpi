# Transcript Navigation Selection And History Integrity

Provisional S2-033/S2-036/S2-054 progress on 2026-10-03. Previous goal turn added
terminal viewport/editing keys. This turn repairs message/tool page navigation
selection and entire-history validation, plus connected suite coverage. No
nested agents, worker launch, checklist promotion, commit/push or unrelated
cleanup.

## Product Change

TranscriptReadPolicy is shared by UI buttons and ViewModel previous/next paths.
It requires exact current project/session/message/role, ordinary vs worker route,
succeeded worker request/subtab, explicit worker history turn and selected tool
call/name. It also fences loading/error/offline/reset/pending state and verifies
that the selected project remains present. It inspects the existing projection
directly without creating random request IDs during recomposition. Backend
scope, digest and current-authority checks remain unchanged.

TranscriptNavigation now checks every retained page, not just the last pair:
at most seven preceding pages, digest/scope/role/worker/tool/total agreement,
contiguous offsets and hasMore before continuation. UTF-8 byte size, page/document
caps, offsets, NUL and hasMore consistency are rechecked. Fresh pages must start
at zero. Broken intermediate or last pages disable both navigation directions;
previous navigation refuses safely instead of throwing and keeps verified text.
The histories remain transient and do not advance event cursors or add writes.

Existing Compose fixtures that enabled navigation without a selected source
message were corrected to include that message. This preserves the actual
product boundary instead of weakening validation for unrealistic fixtures.
TranscriptToolScreenTest is now in the explicit desktop control suite: 71 tests,
or 72 with Doze (20 base, 51 control, one Doze). The lifecycle matrix derives and
checks the same identities; older reduced suites cannot satisfy current gates.

## Verification

- New JVM policy tests cover ordinary/tool/worker routes, foreign or removed
  source, role/name/request/subtab changes, broken page bytes/digest/offsets,
  reset/offline/pending/load/error state and intact previous navigation.
- TranscriptNavigation tests cover full-chain continuity, premature final page,
  duplicate/reordered/intermediate foreign pages and fresh nonzero refusal.
- Compose DiffScreenTest and TranscriptToolScreenTest now cover source/foreign/
  tool-selection/reset/broken-history UI fences. These are compiled, not device
  executed. No test method count was inflated to represent new execution.
- Final Kotlin source check /private/tmp/stage2-transcript-navigation-final3-20261003
  exited 0 after all changes: main/unit/instrumentation compile and 369 JVM tests
  pass. No Compose device execution is inferred from source compilation.
- Python Stage2 discovery: 100 passed; lifecycle: 18 passed; instrumentation
  and Blueprint tests: five each passed. Strict Blueprint and git diff --check
  pass, all 36 checklist rows unfinished.
- Existing Rust debug/release verify-only passes; Rust was not changed. APK
  verify-only refuses missing current source provenance. No stale APK install,
  unchanged blocked Gradle/socket retry or physical QR/LAN execution occurred.

```text
TranscriptReadPolicy.kt efeabf336b41fcc2ac41e021b164cf72d16ecee573035cbddba48dc37662203e
TranscriptNavigation.kt 72fc3e29ac3d9ec83981e0499a4afa2945511f7bfcbbe6aa6cdbe7d175f3c896
PairingViewModel.kt ec096700743fcd93871b1de9b2a412cc31c762f72770008b5d98033941913234
MainActivity.kt 48c6e18a01a102c867087b9a378515d237c6443b4632816dadd0c45b48939a3f
stage2_desktop_android_smoke.py 74e600f8925df65baad81b8393f601d6d49b9defd4bddaa9c9390dc7dcc0a50b
main.jar 70a10a5d415289845214e94a19a0336e3d2b54099ae3424cdc9a85c82e30eb40
unit.jar fc3493501001ace8ef1475a6e581b324b2903aa2b744b4239447ceba4dd95e25
instrumented.jar 1b606e38dd313a8819697eb52190adc4e581d55bcd7c3ffb372f49754e2b500f
```

Goal remains active. Current-source APK, connected full production TLS sync,
physical camera QR/direct LAN, API lifecycle matrix, full parity and signed
install/upgrade still require behavioral acceptance.

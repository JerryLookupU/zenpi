# Composer Target Scope

Provisional S2-036/S2-052/S2-054 progress at 2026-10-03. Previous goal turn
was verified file-page selection progress. Current-source inspection found
ordinary drafts bound only to project/session, allowing unsent file references
to survive a tab transition or message text to become another request's steer.
No nested agents, checklist promotion, worker launch, commit/push or cleanup.

Message draft state now binds host identity, project/session, subtab ID/revision
and current active prompt request ID. Same-target event/preview updates preserve
it; tab revisions, new-message/steering transitions and active request changes
clear it. Worker task drafts also bind tab revision, not only tab ID. Existing
persisted unresolved envelopes remain untouched. WorkspaceCommand.prompt now
requires project session equality before creating a controlled write envelope.

Existing SessionScopeScreenTest and WorkerScreenTest methods cover preservation,
attachment-bearing drafts, revision reset, prompt/steer target changes, and new
scope input. No instrumented method count changed. New PromptScopeTest refuses
foreign project/session control and checks exact valid attachment text/envelope.

## Verification

- zsh tools/stage2_android_source_check.sh /private/tmp/stage2-composer-target-scope-final-20261003:
  exit 0; main/unit/instrumented Kotlin compiles, 349 JVM tests pass.
  Existing warnings remain. Compose source compilation is not device execution.
- ZENPI_HOME=/private/tmp/zenpi-resource-reload-test-home cargo test --offline
  --locked --test remote_headless_commands
  headless_prompt_file_references_materialize_only_in_selected_workspace_and_retry_once
  -- --exact --test-threads=1: exit 0, 1 pass. Controlled provider/domain fixture,
  not a production binary/TLS/Android/LAN run.
- Python Stage 2 tooling discovery: exit 0, 86 pass; Blueprint validator unit
  tests: exit 0, 5 pass. Strict Blueprint, cargo fmt --all -- --check and
  git diff --check pass. All 36 authoritative rows remain unfinished.
- Host release --verify-only passes for unchanged Rust source. Android build
  --verify-only refuses missing APK build provenance. No old APK installed or
  unchanged blocked Gradle/socket attempt repeated.

```text
main.jar 1a53103015e042fa0d3dc63bf16cbde8b53034b2cca073bb7d90dabe57710df1
unit.jar f90a9c9333cb669ce12dda2614d6e69c45662f2e8308cf7b05feee0e0e920096
instrumented.jar 18b7f60849cb5bc519429619b0a07c3d7a6baaa22a483851748d6064475f2277
MainActivity.kt 0e92a22a2b82c7000622b9698b68294b702a6906e2e51196103e5b40a0d48238
HostClient.kt 7fbbb0b4505d2c93f454204531810e6eef60c739a1df5b5e595301ae47c32834
PromptScopeTest.kt d2399cbfd63aed19a6c22a732d6bf594088ce6b51ca464827c4b035e1f520d40
SessionScopeScreenTest.kt 7874a2068f1b304194fa644ecae8ae71f70f782d103d6787cd210671db72f328
WorkerScreenTest.kt 6d846b224b05f8b7cd94c1bd7e661cb79665a480c2bdcc968548694844305ae8
```

Goal remains active. Current-source APK, production desktop-to-Android TLS sync,
physical QR/direct LAN, API 31/34/35 connected lifecycle and signed install/
upgrade remain unverified acceptance surfaces.

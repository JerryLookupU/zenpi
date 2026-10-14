# Resource Description Privacy

Provisional S2-019/S2-036/S2-052 progress observed 2026-10-02T03:26:29Z.
Previous goal turn implemented shared plain steering admission and verified
TUI/headless/Android behavior: progress. Full native Android/QR/LAN/Rust desktop
objective remains active. Dirty worktree baseline preserved; all 36 rows [ ].

## Finding And Repair

The prompt-template loader intentionally uses the body's first nonblank line
as a local description when explicit description is absent/empty. Newly added
remote catalog inherited that fallback, contradicting the metadata-only/body-
private boundary. Description redaction alone cannot prevent arbitrary authored
body prose from crossing the read boundary. Existing fixtures mostly declared
descriptions and missed this behavior.

Loader now retains private, nonserialized description_explicit provenance, true
only for nonblank actual frontmatter. PromptTemplate.remote_description returns
that explicit metadata or an empty string. Remote resources_list uses it instead
of the local description. Local menus/status and template expansion are unchanged.
Missing/empty descriptions still keep the old first-line fallback locally;
whitespace-only description keeps its old local value but projects empty.
Deserialized templates default private provenance to false, including a forged
description_explicit JSON field. No source reread/reparse in the network adapter,
no wire/schema field change and no secret-to-description inference.
Explicit authored descriptions remain remotely visible after existing redaction;
this is not a guarantee that authors cannot place private prose in metadata.

## Verification

Cwd /Users/mac/Github/zenpi. Rust controlled environment
ZENPI_HOME=/private/tmp/zenpi-resource-description.

- rtk cargo test --offline --locked --test stage1_prompt_resources
  --test remote_workspace --test remote_subscription_tls
  --test tui_command_palette --test remote_headless_commands -- --skip
  headless_runner_executes_real_http --skip
  production_plain_host_routes_authenticated_tls: exit 0, **94 pass** in five
  suites (resource loader/templates 13, workspace 28, fragmented authenticated
  rustls TLS 25, TUI palette 22, headless commands 6). Two socket-dependent tests
  excluded, not claimed passed. Focused final headless catalog **1 pass**.
- New template tests cover missing/empty/blank descriptions, multiline explicit
  metadata, BOM/CRLF normalization, retained local display/expansion, unchanged
  serialized shape and deserialization/forged provenance fail-closed behavior.
  Workspace read tests reject dotted-template body leakage and check explicit
  vs fallback description. TLS includes no-description template, two requests
  with no journal/checkpoint side effects, and no-description filler bodies on
  both pages. Headless catalog uses mixed explicit/fallback fillers, no body
  projection, same literal template expansion and retry/journal behavior.
- zsh tools/stage2_android_source_check.sh
  /private/tmp/zenpi-resource-description-android: exit 0, **156 JVM tests**;
  main/unit/instrumented sources compile. Main/unit Android code unchanged.
  HeadlessHostTest now asserts empty filler descriptions on both pages. Harness
  alternates bare Markdown and empty frontmatter description fixtures. This is
  compiled/prepared only, not current-device execution.
- Five Python smoke/lifecycle/blueprint modules: exit 0, **57 pass**.
- cargo fmt --all -- --check, git diff --check and strict blueprint validator:
  exit 0; 36 [ ], zero [_]/[x]. Existing unrelated warnings remain.
- ./gradlew --offline --no-daemon :app:assembleDebug
  :app:assembleDebugAndroidTest, cwd android: exit 1 before compilation at
  ~/.gradle/wrapper/dists/gradle-8.11.1-bin/bpt9gzteqjrbo1mjrsomdt32c/
  gradle-8.11.1-bin.zip.lck (Operation not permitted). No new APK/device run.

Source SHA-256:

```text
prompt_templates.rs 618d955eefdc79cfc0da27e9ada3533dcfdf7681f07df3fbae0fa1c3d1772dae
remote/resources.rs 617364d6994d40f3e80836f5bcfc7978ab2fe8db6beb05213c1ec893dd0763f4
stage1_prompt_resources.rs 839da6d18f279a04d85acb7a26f854cfd000186c2b2bc9336fa6dab610287299
remote_workspace.rs 4ce16dc04ca68e9c16bd343c7109228666461f24204aa3500367787c743e1490
remote_subscription_tls.rs 7f4685faf1dc9590d837da33fad8f3460882dd4d9a20f09cf21d50639352124b
remote_headless_commands.rs a9b51c33f624d2aaed61571236dbd466820ecb3848dab421ccc9fcd0862ce937
stage2_headless_android_smoke.py f079bb4bb4c2e2248e6f505428d9a8ec38d8d5a3a115fa80fae2876b5a340d37
HeadlessHostTest.kt 814521f4ae9879ab31f0c31f0dc78ba3e0fee3988523a09c64149994b712a302
```

Compiled SHA-256:

```text
main.jar 2555f8b127f0f85717b8fb323e8599a1fbb821e2632ff4e08ebd1fba3a13af49
unit.jar 1b53913c5c425a61247ee1464d4bca13990e83d5f2d1335189f451c99635b9c0
instrumented.jar 1ab649b5fad039f9344a0e634958ac5eb019eb64d7f5e17e364ae0218ca85f40
```

No current-source phone/camera/LAN/API-matrix acceptance, full effective worktree
quota scheduler, resource-management parity, provider-delta replay/ACK,
interactive PTY or signed release/migration completion. No agents, permission
bypass, commit/push, unrelated rollback or checklist promotion. Goal active.

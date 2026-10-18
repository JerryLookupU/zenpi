# Host Resource Catalog Progress

Provisional S2-003/S2-019/S2-032/S2-036 progress, observed 2026-10-02T02:56:44Z.
Previous goal development turn added the attachment device acceptance chain:
progress. The intervening RTK request verified rtk 0.50.0 already installed;
it did not change Stage 2 state. Full Android/QR/LAN/Rust desktop goal remains
active. Strict blueprint validation keeps all 36 authoritative rows [ ].
Current integrated HEAD is 1a8ff1acf72e0816fad503a05df2b1d18e07361b;
the dirty worktree remains the development baseline, not a committed release.

## Implementation

Added resources_list Read operation through the shared host and desktop owner
mailbox. Exact empty arguments and project/session scope, current Read ACL,
nonblocking Agent ownership, no outer write fields. It projects the loaded
snapshot's explicit Markdown skills and prompt templates, not files or bodies.
Built-in slash names remain reserved; dotted template names work. Entries carry
invocation command, bounded/redacted description and reload-required skill
availability. Generation is descriptive, not new invocation authority.
Results retain 32 entries, signal truncation and reject serialization above
64 KiB. Known source/base/project paths are removed from descriptions; arbitrary
authored paths are not covered by a general path-redaction claim.

Android strict codec, pinned TLS HostClient route, cancellable transient
ViewModel state and composer menu now select these resources. Choosing prepends
the command to plain draft arguments, retains the draft and does not send.
Existing slash/shell drafts and over-limit inputs are disabled, as are stale
resources, offline/pending-write/lost-control/active-prompt selection. Reads do
not persist metadata, advance transcript/event cursors or send ACKs. Existing
foreground boundary clearing and local-reset cancellation/join include this
job. Normal Send still enters existing owner/revision/dedup admission and Agent
hash checks/expansion; changes after listing may fail instead of silently using
new resource bytes. Remote reload, pagination and full resource controls remain
unfinished.

## Verification

Cwd /Users/mac/Github/zenpi unless explicitly noted.

- env ZENPI_HOME=/private/tmp/zenpi-resources-rust rtk cargo test --offline
  --locked --test remote_workspace --test remote_protocol
  --test remote_subscription_tls --test remote_parity
  --test remote_headless_commands -- --skip headless_runner_executes_real_http
  --skip production_plain_host_routes_authenticated_tls: exit 0, **70 pass**.
  Counts: workspace 28, protocol 11, authenticated fragmented rustls TLS 21,
  parity 4, production headless commands 6. Two socket-dependent tests excluded,
  not claimed as passed. New tests cover ACL/Busy/revoke/session/request fields,
  unchanged journal/checkpoint, mailbox routing, dotted/reserved names, stale
  restore/reload, catalog cap and TLS metadata-only reads across two requests.
  HeadlessHost executes skill/template commands through normal admission and
  captures actual Backend instructions/turns, literal arguments, exact retry
  without duplicate provider invocation and no resource bodies in journal.
- zsh tools/stage2_android_source_check.sh
  /private/tmp/zenpi-resources-android-final: exit 0, **154 JVM tests** pass.
  Main/unit/instrumented sources compile. Three new JVM tests include strict
  fields, scope, types, unsigned overflow, UTF-8, duplicate/count limits and
  draft insertion fences. ResourceCatalogScreenTest covers offline/stale
  choices and preserving draft arguments without sending: compiled only.
- Five Python smoke/lifecycle/blueprint modules: exit 0, **55 pass**.
- env ZENPI_HOME=/private/tmp/zenpi-resources-rust rtk cargo test --offline
  --locked --lib remote::resources::tests: exit 0, **1 pass**, validating
  known-path/control stripping and an exact multibyte UTF-8 truncation boundary.
- cargo fmt --all -- --check; git diff --check; strict blueprint validator:
  exit 0. Existing unrelated compiler/deprecation warnings remain.
- ./gradlew --offline --no-daemon :app:assembleDebug
  :app:assembleDebugAndroidTest, cwd android: exit 1 at wrapper lock
  ~/.gradle/wrapper/dists/gradle-8.11.1-bin/bpt9gzteqjrbo1mjrsomdt32c/
  gradle-8.11.1-bin.zip.lck (Operation not permitted). No new APK/device run.

Intermediate failing test fixtures were repaired: ProcessResult uses assistant,
initial pool need not have a checkpoint, and skills enter provider instructions
while templates replace the current provider turn. These were test expectation
fixes; no change to Agent expansion semantics or broader ownership behavior.

Compiled SHA-256:

```text
main.jar e946094168d491f2d79a1987bb3c8385a4356b187b7ab40dee8f7705cf760327
unit.jar c8cc2cddf7cf716414b4ef50fc8e46134835125c103a5b1e812ae22615ef3190
instrumented.jar ab064ba6f3484b7eec73063945c8db4b9f01ce417cb7cea01726110cca3c4426
```

Prepared Compose gates are not device execution evidence. No current-source
phone/LAN/camera/API-matrix acceptance, full per-worktree scheduler quota,
provider-delta replay/ACK, interactive PTY or signed release/migration claim.
No agents spawned, no permission bypass, no commit/push, no unrelated rollback
and no checklist promotion. Goal remains active while code progress is possible.

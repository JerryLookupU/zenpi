# Plain Steering Admission

Provisional S2-019/S2-036 progress observed 2026-10-02T03:21:01Z. Previous
goal turn prepared resource device gates and verified serializers: progress.
Full native Android/QR/LAN/Rust desktop goal remains active. HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b, dirty worktree preserved; no agents,
permission bypass, commits/push, unrelated rollback or acceptance promotion.
Strict blueprint validation leaves 36 [ ] rows, no [_]/[x].

## Implementation

Inspection found remote steering sent skill/template/slash commands and @ file
references to the plain InputPort queue without resource expansion/materializing
files, unlike normal prompt preparation. Added shared plain_queue_input in
slash_actions: nonblank UTF-8 <=64 KiB, no NUL, no leading slash/shell commands,
no explicit @ token at native whitespace boundaries. Email addresses, inline
paths/commands and parenthesized @ text remain plain text. Bare/incomplete
references also fail, avoiding silent attachment loss.

SteerAdmissions invokes the check after current ACL and exact durable receipt
lookup, before new owner/revision/active-turn validation and checkpoint/queue
reservation. Refused inputs cannot queue or mutate journals/checkpoints. A
historical now-refused exact receipt still resolves without resubmission under
current ACL, including after completion/restart; revoked devices cannot read
the receipt. Existing normal steering persistence and dedup remain intact.

Local TuiInputControls enqueue/edit now uses the same helper instead of relying
on the visible resource catalog to detect commands. Refused input retains the
existing displayed draft; terminal control sanitization still applies. Normal
scheduled/idle prompts retain attachment and resource handling. Core direct
input-queue protocol/Agent resource expansion are unchanged; this is a host
admission rule, not new queue resource support.

Android new-steering construction and Send eligibility use a matching UTF-8/
codepoint validator with Rust whitespace boundaries (including NEL/NBSP but
not Java-only control separators). Invalid drafts remain editable and cannot
persist new pending intents. Saved historical envelopes still restore through
existing strict restore rules for exact receipt resolution. New Compose test
retains refused drafts and only sends valid email-containing text: compiled,
not device-executed.

## Verification

Cwd /Users/mac/Github/zenpi; Rust uses ZENPI_HOME=/private/tmp/zenpi-steering-input.

- rtk cargo test --offline --locked --lib remote::command::steer_tests: exit 0,
  **3 pass**. New test holds the real Agent lock and verifies command/@ refusal
  without queue/ticket/journal/checkpoint changes, historical receipt lookup,
  valid plain durable steering and revoked lookup. Existing restart and lost
  ticket ambiguity tests pass unchanged.
- rtk cargo test --offline --locked --lib
  slash_actions::attachment_identity_tests: exit 0, **9 pass**. New helper test
  mirrors Android Unicode/token/64 KiB boundaries; existing file tests pass.
- zsh tools/stage2_android_source_check.sh
  /private/tmp/zenpi-steering-input-android-final: exit 0, **156 JVM tests** in
  30 classes; main/unit/instrumented sources compile. New Kotlin boundary test
  and expanded command test cover refusal/plain text and legacy envelope restore.
- Five Python smoke/lifecycle/blueprint modules: exit 0, **57 pass**.
- cargo fmt --all -- --check; git diff --check; strict blueprint validator:
  exit 0. Existing unrelated Rust/Android warnings remain.
- ./gradlew --offline --no-daemon :app:assembleDebug
  :app:assembleDebugAndroidTest, cwd android: exit 1 at wrapper lock
  ~/.gradle/wrapper/dists/gradle-8.11.1-bin/bpt9gzteqjrbo1mjrsomdt32c/
  gradle-8.11.1-bin.zip.lck (Operation not permitted). No new APK/device run.

- rtk cargo test --offline --locked --test tui_composer
  --test remote_headless_commands --test remote_parity --test remote_workspace
  -- --skip headless_runner_executes_real_http
  --skip production_plain_host_routes_authenticated_tls: exit 0, **93 pass**
  (TUI 55, headless commands 6, parity 4, workspace 28). Two socket-dependent
  tests excluded, not claimed passed.

Production HeadlessHost running-backend test now refuses resource/file steering
while Agent lock is held and provider remains active, with no extra invocation;
existing cancel/shutdown/restart checks remain. TUI test covers enqueue/edit
with no catalog and unchanged session/draft. Initial test expectations were
corrected to seed the actual displayed draft and respect existing NEL control
sanitization; no terminal behavior change was made to satisfy the test.

SHA-256 compiled evidence:

```text
main.jar 2555f8b127f0f85717b8fb323e8599a1fbb821e2632ff4e08ebd1fba3a13af49
unit.jar 1b53913c5c425a61247ee1464d4bca13990e83d5f2d1335189f451c99635b9c0
instrumented.jar 5b3bbee3fdd150d1c61701d47cf752a0f80bb89d32d5bd106313b0983f255266
```

No current-source physical phone/camera/LAN/API-matrix acceptance, full worktree
quota scheduler, resource management parity, provider-delta replay/ACK,
interactive PTY or signed release/migration claim. Temporary build limitation
does not block meaningful implementation progress; goal remains active.

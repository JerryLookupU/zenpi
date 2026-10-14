# Resource Device Acceptance Chain

Provisional S2-036/S2-055 progress observed 2026-10-02T03:14:04Z. Previous
goal turn integrated snapshot-bound resource paging: progress. Full native
Android/QR/LAN/Rust desktop goal remains active, all 36 checklist rows [ ].
HEAD 1a8ff1acf72e0816fad503a05df2b1d18e07361b with preserved dirty worktree.

## Implementation

HeadlessHostTest now wires production resource callbacks. After the existing
attachment/stream/reconnect/concurrent-read checks, it enters literal draft
arguments, opens catalog, checks the skill in the first 32 entries and absence
of the tail template, navigates second/first/second with identical snapshot,
verifies terminal Next disabled, and selects /zz-stage2-review. It asserts the
exact command/arguments in the editable field, no pending request, unchanged
turns/event cursor/cache, cleared transient catalog, captures a second-page
PNG and emits headless_resource_pages_draft_verified. The selected command is
then explicitly constructed through the normal pending request path. Existing
real process death, encrypted pending recovery, exact retry and durable ACK
checks now apply to that resource command instead of a plain second message.
Background read-fence checks also attempt catalog open/navigation.

Harness seeds 40 described filler templates, one tail template and one catalog
skill in the production project's normal resource paths. Provider call count
remains two: file attachment prompt first, selected template second. It requires
the new Android phase, unchanged template source, no literal shell execution
sentinel, resource page PNG/hash and a new receipt gate. Independent
resource_evidence checks the exact expanded provider text (including literal
$1/$(...)), no unexpanded slash in provider input, raw command in exactly one
journal user turn, exact template kind/name/generation/source suffix and source
SHA256, no expanded body in journal and no filler body sent to provider.
prompt_evidence requires this chain in addition to attachments/admission/binding/
progress/answer checks. Saved lifecycle verification re-evaluates it after
checking evidence-file SHA256; a claimed boolean alone is not acceptance.

API31/34/35 HEADLESS_GATES now requires the resource gate. Screenshot verification
requires resources_screenshot and checks PNG signature and SHA256. Historical
receipts remain untouched and do not satisfy the expanded current-source gates.

## Verification

Cwd /Users/mac/Github/zenpi unless specified.

- Five Python smoke/lifecycle/blueprint modules: exit 0, **57 pass**. Focused
  headless/lifecycle: **32 pass**. New negative cases reject unexpanded/changed
  provider text, early expansion, journal body leakage, wrong kind/name/hash/
  generation/source, duplicate/foreign provenance, filler leakage, missing or
  tampered resource screenshot and rehashed evidence with missing expansion.
- env ZENPI_HOME=/private/tmp/zenpi-resource-device-gate rtk cargo test
  --offline --locked --lib protocols::responses::tests: exit 0, **3 pass**.
  New test loads the exact fixture through production PromptTemplates and
  serializes via production Responses encoder, verifying trimming, SHA256 and
  exact wire text/literal arguments without shell execution or attachment.
- env ZENPI_HOME=/private/tmp/zenpi-resource-device-gate rtk cargo test
  --offline --locked --test remote_headless_commands -- --skip
  headless_runner_executes_real_http --skip
  production_plain_host_routes_authenticated_tls: exit 0, **6 pass**, two
  socket-dependent tests excluded. Focused resource test also **1 pass**.
  Actual HeadlessHost backend capture now uses the same tail template/arguments
  and verifies exact expanded current turn plus raw journal/provenance/hash and
  retry dedup. This is not an HTTP or Android device execution result.
- zsh tools/stage2_android_source_check.sh
  /private/tmp/zenpi-resource-device-gate-android: exit 0, **155 JVM tests**;
  main/unit/instrumented sources compile. Expanded HeadlessHostTest compiled
  only. Main and unit code unchanged this turn.
- cargo fmt --all -- --check, git diff --check, strict blueprint validation:
  exit 0. Existing unrelated Rust/Android warnings remain.
- ./gradlew --offline --no-daemon :app:assembleDebug
  :app:assembleDebugAndroidTest, cwd android: exit 1 before compilation at
  ~/.gradle/wrapper/dists/gradle-8.11.1-bin/bpt9gzteqjrbo1mjrsomdt32c/
  gradle-8.11.1-bin.zip.lck (Operation not permitted). No new APK or device run.

SHA256:

```text
stage2_headless_android_smoke.py 392278b88b5bfddcce6cb3830acfd0192f3381154c9abba2bc649537d1813614
test_stage2_headless_android_smoke.py d73a8332a12c941007cdb6d031b970d8549c4f76d736043330fe06247ee1271d
validate_stage2_android_lifecycle.py a7684309f94196a571b2b24836792ce8ef18bfc7eb692fef7ecbf41941ea4ea6
test_validate_stage2_android_lifecycle.py 5a4a16d113239575352fc1e5e1d20b9ce6766ae3924e85c25c03a4d581ddca94
HeadlessHostTest.kt cdb7c59fa5c6956cfe64262a102c02af832741587e99bb4dfa31c3264deaf5f1
responses.rs 0d5643bfa0fa8e306ac9caedbdf680d81698b0f04f25a98008015e2d3919b607
remote_headless_commands.rs ec0cca893ea7eece658d10112337312f6e633c84b3dcd47224e83b32b04f2a0e
main.jar 89162e8d0426810df51a4e730fe8ec8b8f7318df1db6f6b223ab925536d09c4f
unit.jar 1f4e931a6a674147b49fa8d590d94e7ee54b0c08d26072f9725b14fdc88684d8
instrumented.jar eee30942d1c52c026cfdf415f4fb4fbcfdded30d884242a9fef3e33f7d2bbb97
```

Prepared acceptance is not execution evidence. No current-source physical
phone/camera/LAN or API matrix run, full worktree scheduler quotas, resource
management parity, provider-delta replay/ACK, interactive PTY or signed
release/migration claim. No agents, permission bypass, commit/push, unrelated
rollback or checklist promotion. Goal remains active with implementation work.

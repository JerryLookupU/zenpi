# Remote Private File Selection Boundary

Provisional S2-032/S2-036/S2-052/S2-072 progress observed
2026-10-02T05:07:04Z. Previous goal turn: progress (file paging and tests).
HEAD 1a8ff1acf72e0816fad503a05df2b1d18e07361b; existing dirty changes preserved.
All 36 authoritative rows remain [ ]; full Android/QR/LAN/Rust desktop goal
active. No agents, worker admission, commits, pushes, cleanup or completion.

## Implementation

Existing tool credential_path helper is now crate-visible and reused by remote
legacy/paged directory discovery and remote raw attachment admission. Local
prompt_files_at keeps its previous behavior via an explicit remote flag. Known
credentials/auth JSON, credential folders, SSH key names, PEM/KEY/P12 files and
private dot-directories are omitted/rejected case-insensitively at each path
component. The remote prompt admission checks after historical receipt lookup
but before any new binding/journal/runner effect; prompt_input repeats it before
file resolution. Refusal leaves the request ID unreserved. Historical receipt
lookup never reexecutes a formerly admitted private attachment. Ordinary malformed
references retain existing runner-side failure semantics. Slash resources are
host-expanded and not reinterpreted by this raw reference check.

Android file codec mirrors this known-path defense in request, returned entry
validation and quoted-reference insertion. Host remains authoritative for any
manually typed prompt. No broad local attachment restriction was introduced.

## Verification

Cwd /Users/mac/Github/zenpi; Rust ZENPI_HOME=/private/tmp/zenpi-private-files.

- rtk cargo test --offline --locked --test remote_workspace --test remote_subscription_tls
  --test remote_protocol --test remote_auth --test remote_parity:
  exit 0, **103 pass**. Legacy and paged listings contain planted private files
  and verify omission; real authenticated fragmented TLS checks same omission.
- rtk cargo test --offline --locked --lib
  remote::command::steer_tests::remote_private_attachments: exit 0, **1 pass**.
  Checks no submission/binding/journal effect, request ID remains usable,
  historical exact receipt returns without submission, normal/local behavior.
- rtk cargo test --offline --locked --lib remote_file_listing_omits_private_paths:
  exit 0, **1 pass**; local discovery still exposes all eight fixture entries,
  both remote forms expose only ordinary note.txt; private-directory read refuses.
- rtk cargo test --offline --locked --test remote_headless_commands -- --skip
  headless_runner_executes_real_http_tool_turn_and_retries_never_resubmit --skip
  production_plain_host_routes_authenticated_tls_prompt_to_the_provider:
  exit 0, **8 pass**, 2 TCP-dependent tests excluded. Actual HeadlessHost mailbox
  and captured provider attachment test proves manually typed credentials.json
  refuses without another provider call or journal change.
- zsh tools/stage2_android_source_check.sh /private/tmp/zenpi-private-files-android:
  exit 0, **173 JVM tests**, main/unit/instrumented compile. New private path
  request/result/reference tests; device tests compile only, not executed.
- Six Python smoke/lifecycle/blueprint/backup modules: exit 0, **63 pass**.
- cargo fmt --all -- --check, git diff --check, strict blueprint: exits 0.

Final Android SHA256:

```text
main.jar 561bd94c5bd0a1f8f92512c621dda4b96807d3a247293f7b8cae3b3b786d8b6a
unit.jar d6c503c588ce8f49dc95f43f627430d6f225c79b241c471814b343142aae16bc
instrumented.jar 56325f0c91323ac6d6dd6be54690c71320686a1baa4cf5db63805b5617af0ae6
```

Intermediate unit fixture failed because the local Agent had no attachment
workspace configured; configured its existing ToolContext and reran. No
production policy relaxation was needed. Existing warnings remain.

## Residual Scope

This is a known-path boundary, not complete secret/content classification.
Files with arbitrary names, hard-link aliases, user tool instructions, local
provider policy and host-expanded resources require separate security review.
No claim of universal prevention of credential exfiltration. Full security
review, current APK/device, real QR camera/LAN/API matrix, scheduler/interactive
PTY/full settings parity and signed release remain unfinished. No permission
bypass or acceptance promotion.

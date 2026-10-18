# Snapshot-Bound File Browser Pages

Provisional S2-019/S2-032/S2-036/S2-054 progress observed
2026-10-02T05:01:21Z. Previous turn: progress (production QR/ZXing fixture and
invite parser alignment). HEAD 1a8ff1acf72e0816fad503a05df2b1d18e07361b;
existing dirty changes preserved. Full Android/QR/LAN/Rust desktop goal active;
all 36 authoritative rows remain [ ]. No agents, worker admission, commits,
pushes, cleanup or acceptance promotion.

## Implementation

The old files_list read retains its three-field bounded 64-entry behavior.
Opt-in pages add offset/snapshot_id. Initial 0/null creates a metadata snapshot;
later pages and back navigation bind it. Page size 64, scan bound 2048 visited
children, existing 250ms cooperative scan deadline and 64KiB result cap. A
partial scan fails ResourceLimit rather than asserting a hidden tail is complete.
Snapshot digest binds project/session/subtab/revision/path and the complete
allowlisted directory metadata. No contents/hash/provider operation projected.
Existing symlink/hidden/control/redaction/private path filters are reused.

Android strict codec binds pages to previous snapshot and exact scope/path,
checks offsets/full-page continuation/size/count/null terminal pages, and retains
legacy result compatibility. Initial explicit invalid_request may cause one
three-field read fallback; later pages and all other refusals do not. This is
read-only compatibility, not mutation retry. The ViewModel reuses cancellable
foreground file jobs and generation/reset/network fences; failed pages clear
stale entries. Previous/next/page number controls retain existing offline/
loading/pending/active-prompt fences. File selection remains draft-only quoted
reference insertion; actual prompt admission still reopens/validates it.

Prepared headless/Android gate places 80 sorted fillers before its attachment,
requires page 64 with identical digest and selects the original attachment from
that tail. Existing provider evidence remains bound to that exact file. This
gate and new Compose pagination test compile only, not device-executed here.

## Verification

Cwd /Users/mac/Github/zenpi; Rust ZENPI_HOME=/private/tmp/zenpi-files-pages.

- rtk cargo test --offline --locked --test remote_workspace
  --test remote_subscription_tls --test remote_protocol --test remote_auth
  --test remote_parity: exit 0, **103 pass**. New domain test traverses 140 files
  in three pages/back, malformed offsets/null snapshot/out-of-range, changed
  file size/addition, foreign session/stale subtab, capacity and revoked ACL;
  no journal changes. New authenticated fragmented actual rustls test traverses
  two pages/81 items and verifies no content/private path/cursor mutation.
- rtk cargo test --offline --locked --test remote_headless_commands -- --skip
  headless_runner_executes_real_http_tool_turn_and_retries_never_resubmit --skip
  production_plain_host_routes_authenticated_tls_prompt_to_the_provider:
  exit 0, **8 pass**, 2 TCP-dependent cases excluded.
- rtk cargo test --offline --locked --test stage1_prompt_resources:
  exit 0, **13 pass**; existing local resource behavior preserved.
- zsh tools/stage2_android_source_check.sh
  /private/tmp/zenpi-files-pages-android-complete: exit 0, **172 JVM tests**;
  main/unit/instrumented compilation. New JVM bound pages/back, scope/snapshot,
  malformed continuation and legacy request checks. New Compose read-only
  paging/offline/loading and prepared headless attachment-page tests compile.
- Six Python smoke/lifecycle/blueprint/backup modules: exit 0, **63 pass**.
- cargo fmt --all -- --check, git diff --check, strict blueprint: exits 0.

Final compiled SHA256:

```text
main.jar 905eeb694da1da0a467a4e96d7ad9d91d0f122718d59488651ca8d272a7d2954
unit.jar d3758e0c819e93b38c1630cf1dfaecec0a70449d1c333ff3149bd37c9eb55712
instrumented.jar 56325f0c91323ac6d6dd6be54690c71320686a1baa4cf5db63805b5617af0ae6
```

## Remaining Acceptance

File snapshots are metadata-only, not content identity or atomic filesystem
snapshots; prompt admission revalidates actual attachment contents. Deadline
cannot interrupt a blocked filesystem syscall. Complete directories above the
scan cap remain visibly unavailable in paged mode. No source/TLS stream result
is promoted to real device/LAN success. Current APK, camera/QR/LAN/API-device
acceptance, scheduler quota, interactive PTY, full account/settings parity and
signed release remain unfinished. No Gradle/adb socket permission bypass or
repeated blocked network probes. Existing compiler/deprecation warnings remain.

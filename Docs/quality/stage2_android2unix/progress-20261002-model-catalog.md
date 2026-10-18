# Paged Host Model Catalog

Provisional S2-003/S2-019/S2-032/S2-036/S2-054 progress observed
2026-10-02T04:24:15Z. Previous goal turn expanded the device selection gate and
independent evidence verifier: progress. Full Android/QR/LAN/Rust desktop goal
remains active. HEAD 1a8ff1acf72e0816fad503a05df2b1d18e07361b; dirty changes
preserved. All 36 authoritative rows remain [ ]; no acceptance promotion.

## Implementation

model_status now projects a paged host catalog. Initial empty arguments return
first page; exact offset/snapshot_id arguments request bound subsequent or
previous pages. At most 512 descriptors, 16 entries/page and a 16 KiB result.
Each entry has only id, lifecycle status and selectable; IDs pass existing
bounded privacy checks. selectability uses actual retained-backend/effort and
session-history validation. Manual custom/unknown ID entry remains available;
the actual selection operation still validates again at admission.

Snapshot digest binds project/session/selection revision, host-local complete
catalog metadata and validation results. Only digest and allowlisted page leave
the host. Sort and duplicate checks are deterministic. Bound offset zero permits
back navigation; stale snapshot, malformed offsets and out-of-range pages fail.
Catalog reads do not append journal/checkpoint or invoke provider. Endpoint,
credentials, prices and descriptor sources are not projected.

Android codec accepts older status shapes as no catalog, validates exact page/
entry keys, unsigned offsets, full-page continuation, unique IDs and UTF-8 order
(including non-BMP versus BMP ordering), and binds subsequent status revisions
and snapshot IDs. HostClient uses the same pinned/cancellable authenticated read
route. ViewModel uses its existing shared model job, foreground/network fences,
generation and reset join; failed pages clear status for a fresh read.

Native catalog dropdown and previous/next controls sit alongside manual Model
ID. Disabled items cannot be picked; selection changes draft only, explicit Apply
remains required. Paging preserves draft for the same selection revision. Older,
read-only, offline/loading/pending cases do not gain mutation authority. Prepared
headless/Android device gate now selects the actual model catalog entry before
Apply rather than typing the known ID; no current device execution claimed.

## Verification

Cwd /Users/mac/Github/zenpi, Rust ZENPI_HOME=/private/tmp/zenpi-model-catalog.

- rtk cargo test --offline --locked --test remote_workspace --test remote_protocol
  --test remote_parity --test remote_subscription_tls --test remote_auth:
  exit 0, **97 pass** (workspace 38, protocol 11, parity 5, fragmented actual
  rustls/P256 TLS 32, auth 11). New catalog test loads 40 real overrides, traverses
  all pages/back, checks no source version/credential/price/endpoint/path,
  no journal/checkpoint change, rejects stale selection, disabled incompatible
  model, malformed offsets and revoked access. New TLS test requests first/tail
  catalog pages through production adapter and checks same digest/no secrets.
- rtk cargo test --offline --locked --test remote_headless_commands -- --skip
  headless_runner_executes_real_http_tool_turn_and_retries_never_resubmit --skip
  production_plain_host_routes_authenticated_tls_prompt_to_the_provider:
  exit 0, **8 pass**, 2 excluded; no green complete network-suite claim.
- zsh tools/stage2_android_source_check.sh
  /private/tmp/zenpi-model-catalog-android-complete: exit 0, **161 JVM tests**
  in 31 classes; main/unit/instrumented compile. New JVM shape/continuation/
  snapshot/revision/foreign scope/duplicate/UTF-8 ordering tests. New Compose
  draft-only selection/disabled model/paging/offline test and changed headless
  catalog gate compile only, not executed on a device.
- Five Python smoke/lifecycle/blueprint modules: exit 0, **59 pass**.
- cargo fmt --all -- --check, git diff --check, strict blueprint: exit 0.
  Existing warnings remain; intermediate Kotlin shadowed name removed locally.

Compiled SHA256:

```text
main.jar c25481f70d836455f510ee4342d69f11c1bc4971de5fb26c9e73767bad4d0659
unit.jar 69a9793aab8acc11b955f220bfefcc8a5584a4382a877fce7817c4418cf3c54e
instrumented.jar 4ea3ddc2757ffe4ae102ebbf6cc07f70cb9e442dfa7f8d89b9a6b33382caab82
```

## Remaining Scope

Current-source APK, camera/QR/LAN, API-device acceptance, complete account/settings
parity, quota scheduler, interactive PTY, provider-delta replay/ACK, migration
and signed release remain unverified/unfinished. No promotion from source/JVM/
TLS stream evidence to real LAN success. Prior Gradle daemon socket EPERM/adb
restrictions remain; no bypass, package reinstall or repeated network attempts.
No agents, worker admission, commits, pushes, cleanup or goal completion.

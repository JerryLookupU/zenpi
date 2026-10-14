# Resource Catalog Paging

Provisional S2-003/S2-019/S2-032/S2-036 progress, observed 2026-10-02T03:06:10Z.
Previous goal turn integrated catalog read/selection and host expansion:
progress. Full native Android/QR/LAN/Rust desktop objective remains active;
all 36 authoritative rows stay [ ]. HEAD 1a8ff1acf72e0816fad503a05df2b1d18e07361b,
dirty worktree baseline preserved. No agents, acceptance promotion, permission
bypass, commit/push or unrelated rollback.

## Implementation

Removed the 32-entry reachability limitation by adding bounded pages to the
existing resources_list Read operation. Fresh open uses empty arguments;
navigation sends exact offset/snapshot_id, including return to offset 0.
Offsets are unsigned multiples of 32 bounded by the existing combined loader
cap (4096 skills + 1024 templates). The host counts/hashes the admitted ordered
snapshot while retaining only one 32-entry projection; results stay <=64 KiB.
Digest binds project/session, generation, projected metadata and private
source provenance/content hashes. It is opaque on wire; source fields and
individual hashes remain absent. Every page repeats ACL/session/owner checks.
Changed digest fails RevisionConflict with no page; invalid offsets fail
InvalidRequest. No provider, journal or checkpoint side effects from paging.

Android now strictly checks snapshot ID, offsets, nullable next offset and
truncated agreement in addition to existing metadata validation. A continuing
page must have 32 entries and exact next offset. Native previous/next buttons
preserve draft arguments and do not send. Only one transient page is retained;
loading disables selection/navigation, any read error clears the old page and
requires fresh open. Existing network/background/selection/reset clearing
remains. Previous-to-first navigation is snapshot-bound rather than silently
starting a different catalog. Remote reload/settings and full resource parity
remain unfinished.

## Verification

Cwd /Users/mac/Github/zenpi; controlled Rust environment ZENPI_HOME set to
/private/tmp/zenpi-resource-pages.

- zsh tools/stage2_android_source_check.sh
  /private/tmp/zenpi-resource-pages-android-final: exit 0, **155 JVM tests**.
  Main/unit/instrumented source compilation passes. New paging JVM test checks
  forward/back including offset 0, foreign scope, changed digest/generation,
  malformed numeric/offset/next/full-page boundaries and missing continuation.
  New Compose paging test verifies next/previous callbacks, loading-disabled
  navigation, retained draft and no Send: compiled only, not device execution.
- Five Python smoke/lifecycle/blueprint modules: exit 0, **55 pass**.
- env ZENPI_HOME=/private/tmp/zenpi-resource-pages rtk cargo test --offline
  --locked --test remote_workspace --test remote_subscription_tls
  --test remote_headless_commands --test remote_protocol --test remote_parity
  -- --skip headless_runner_executes_real_http
  --skip production_plain_host_routes_authenticated_tls: exit 0, **73 pass**
  in five suites (workspace 28, TLS 24, headless commands 6, protocol 11,
  parity 4), two socket-dependent tests excluded, not counted as passed.
  Final verification repeated twice with identical pass counts.
- Strict blueprint validator: exit 0, 36 [ ], zero [_]/[x]. Format and diff
  checks pass. Existing unrelated Rust/Android warnings remain.
- ./gradlew --offline --no-daemon :app:assembleDebug
  :app:assembleDebugAndroidTest, cwd android: exit 1 at restricted wrapper lock
  ~/.gradle/wrapper/dists/gradle-8.11.1-bin/bpt9gzteqjrbo1mjrsomdt32c/
  gradle-8.11.1-bin.zip.lck (Operation not permitted). No new APK or device run.

Rust integrated tests extend the real read owner/mailbox and HeadlessHost path
to tail resources; 43 unique entries across two pages include the formerly
unreachable dotted template. Both forward/back old identities reject reload.
Restore tests compare equal generation and equal projected metadata after a
body-only change, proving digest changes beyond the process-local generation.
TLS tests use actual fragmented rustls/P256 proof over bounded in-memory duplex
streams, not TCP/LAN: two-page delivery, reload refusal, and revoke before
continuation. No catalog bodies, journal/checkpoint changes or provider replay.
Final integrated suite result is recorded above.

Compiled SHA-256:

```text
main.jar 89162e8d0426810df51a4e730fe8ec8b8f7318df1db6f6b223ab925536d09c4f
unit.jar 1f4e931a6a674147b49fa8d590d94e7ee54b0c08d26072f9725b14fdc88684d8
instrumented.jar 65532778ab21041f30349749f2ffa914b37af4bb6ad31a11ae83baf042c5b69e
```

This is not current-source phone/camera/LAN/API-matrix acceptance. Worktree
scheduler quotas, full resource controls, provider delta replay/ACK, interactive
PTY, release signing/migration and full blueprint gates remain unfinished.
Historical APK/device receipts have not been promoted to this source revision.

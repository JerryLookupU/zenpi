# Cancellation Restore And Approval Scope

Provisional S2-036/S2-037/S2-052/S2-054 progress. Previous goal turn completed
the cached cursor ACK/resume cross-language lane and was progress. This turn
revalidated execution skill, preserved dirty integration, parity matrix, Android
command/receipt/projection/dispatch and Rust cancellation/approval boundaries.
No nested agents, destructive reset, checkbox promotion, commit or push.

## Repairs

WorkspaceCommand.restore's cancel target boundary previously allowed all-space
and control-character strings. Rust valid_identifier refuses blank, over-128-byte
and control-containing IDs. Android now rejects those targets before treating
the restored envelope as a pending request. Exact valid request IDs are not
normalized or regenerated. New JVM regression covers empty/space/newline/NUL/
DEL/ASCII-length/UTF-8-byte-length refusal, valid internal-space and byte-boundary
IDs and exact foreground retry with background refusal.

Approval construction previously selected a project by ID/capability without
also requiring its session to match the current projection. An inconsistent
projection could therefore construct an old-target approval with a replacement
project session. Production dispatch already refused that mismatch; the builder
now rejects it at construction too. New JVM regression covers changed selected
ID/current session/project session/capability, exact valid restore, a genuinely
fresh replacement-session target, and refusal of the old target under that view.
Host ACL/CAS/receipt boundaries were not weakened or changed.

## Verification

Cwd /Users/mac/Github/zenpi, HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b. Observed time during verification:
2026-10-03T20:36:33Z, Oct 4 local time.

- zsh tools/stage2_android_source_check.sh /private/tmp/stage2-command-scope-20261004:
  exit 0; production/JVM/instrumented Kotlin compilation, 471 ordinary JVM tests,
  explicit current Rust read/rename/P256/worktree/session/ACK interop pass.
  Existing warnings remain. No APK packaging or device assertions executed.
- rtk cargo test --locked --offline --test remote_parity --test remote_security --test remote_migration --quiet:
  exit 0, 23 pass across three suites.
- Expanded Rust invocation first failed in remote_project_inspection fixture
  preparation at pool.publish: configuration I/O Operation not permitted from
  the inherited user config location. This was a failed gate, not passed or
  silently skipped. Production configuration validation remained unchanged.
- With ZENPI_HOME=/private/tmp/stage2-command-scope-home-20261004, the same
  expanded invocation exits 0. Targets: remote_auth, remote_device_grants,
  remote_pairing, remote_workspace, remote_subtab, remote_worktree,
  remote_concurrency, remote_worker_owner, remote_project_inspection,
  remote_diff, remote_live_text, remote_terminal, remote_protocol, remote_events,
  remote_subscription, remote_faults. Common argv: rtk cargo test --locked
  --offline, each target supplied with --test, ending --quiet. RTK summarizes
  212 pass, 37 filtered, 20 result blocks including test-spawned subprocesses;
  these are not 20 independently selected targets or the full Rust suite.
- Python Stage 2 discovery: 132 pass. Lifecycle validator: 23 pass.
- Rust format, git diff --check and strict Blueprint pass; all 36 rows stay [ ].
- APK verify-only refuses missing provenance. No restricted Gradle retry or old
  APK install. Debug host rebuilt after Cargo tests; separate verify-only passes,
  not release, reproducibility or device acceptance.

```text
HostClient.kt 014062b8a8461835dfdf89ca16d5c12b52bfd79ff815b5637b01578953c8a121
WorkspaceCommandTest.kt 54954375d1f0347ca32790b182efbc38f27e4e949f2267e53c791c36fa44624b
ApprovalProjectionTest.kt fc17fe23a3289cf445789cc0d0d0a0225ba0680bd82a26cfef174f36589398af
main.jar df5faa20b3408c401082ccb5760ad3a9cd666db6785ac5d2c23b69cffc38c079
unit.jar 497725840c6394a9b8ebb7e73445178d9fa9ef3e4bc883bdbb34da4fa40c1fd1
instrumented.jar 8c22b80093a6d240fc71428f32893dddb9b7a3c7fe1e5b4efaff04ca507bba8d
debug-host-receipt ab8d4f4f671b084687f50a77f5b1dd5921c814270fbec14604519ceabbde50af
debug-host-binary 701d3e597cd640090cdc254f6ad2f810b986dbd626e42179ca3da6d8e50c5d77
```

These changes tighten local command admission, not proof of an observed LAN
race or encrypted-device recovery. Fresh APK, physical QR/direct-LAN two-way
sync/recovery, connected API lifecycle, full parity, backup/uninstall and signed
release still require acceptance evidence. The full goal remains active.

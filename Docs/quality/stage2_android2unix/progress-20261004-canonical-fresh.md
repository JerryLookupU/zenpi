# Stage 2 Canonical Fresh Host Publication

Recorded 2026-10-04T10:26:13Z; partial S2-070 evidence, not acceptance.
Previous goal turn was progress: two independent fresh release builds matched
while the canonical cached artifact differed. Dirty worktree preserved; no
nested agents, reset, signing-policy bypass or denied socket/Gradle retry.

## Change And Verification

stage2_host_build.py adds --fresh: one new empty retained target directory,
required offline/locked Cargo command, exact reported executable, unchanged
source inventory, bounded atomic publication to the standard profile path.
The v2 receipt binds the fresh target/command/hash and published binary; v1
cached receipts remain compatible. Verify-only checks both artifacts against
the current inventory and refuses wrong target paths, symlinks, changed
canonical/fresh binaries or altered commands. The source tool intentionally
does not promote fresh attribution to repeat-build or product acceptance.

Three new HostBuildTests cover fresh publication/permissions/verification,
failed/drifting builds preserving the previous binary without authority, and
symlink/path/command/artifact substitution refusal. All Stage 2 Python tools:
155 tests pass. Lifecycle validator fixtures 25, Blueprint fixtures five pass;
cargo fmt --check, git diff --check and strict Blueprint pass. 36 [ ], zero
[_]/[x]. No new Kotlin, Gradle, Android or production-listener pass claimed.

Process 81062 ran python3 tools/stage2_host_build.py --profile release --fresh
to authoritative exit 0. Fresh target:
/Users/mac/Github/zenpi/target/stage2-fresh-host-wv03p_o7. Its artifact and
published target/release/zenpi hash equally at d32407fb556406fa662d1b2e4eb909bb1006ef7fd82854588ef32d6030cf34e2.
Release verify-only exits 0; debug attribution rebuilt and verifies too.
The actual published release help exits zero with HOME removed and a missing
private ZENPI_HOME; that home remains absent. Android verify-only still exits
1 for missing APK provenance. No stale APK install or local-listener retry.

Process 29242 additionally ran the independent comparison to terminal exit 0
using /private/tmp/stage2-canonical-repeat-20261004.json and two empty target
directories at /private/tmp/stage2-release-compare-jpakcp40/a and b. Both
artifacts match 6b0535aad15e5dca888bb7e5ea1758abd0d6d2fa94030ab3e8f6f5919a01f529,
but do not match the canonical fresh target. cmp -l against the earlier
identical /private/tmp artifact reports exactly 48 byte differences: 16 in
LC_UUID at offsets 1945..1960 and 32 in embedded code signature at
9266593..9266624 (one-based). otool/codesign show equal layout/size and same
ad-hoc linker signature identity, differing UUID. Output-path link metadata
is an inference, not a proven root cause. No bytes were normalized/stripped,
no comparison was relaxed and no cross-path reproducibility claim is made.

```text
host-builder 7f2c84413c72f9c21f515f453364708f5be46b564ce144280ee7aa8c3430951d
builder-tests b1060bd717f37b78287a4c1f5d9957997f34a3181ca0ff314dcc2bf3f5963d15
release-receipt 1889902711cbaab0c25a8edb9159e61ad5adbf38181fe6eb34e1b963a983818f
host-inputs 9e65dc403e986b45fedb6b01754041757df6edd6868be7e3a1aac1ec33551052
fresh-build-log 2a7114f25f26cc822fac5960f6be4e13cdfe38927d947458da2d1e16ffab08f3
python-log 14a7be84772cb905c4d9d844b82dba0e2df7ed89a6f7162c9a42d54b38f45690
repeat-receipt 01c7922af4d5158c3f4a2c186ad08035998b68edd8fded2c10a9b7ea337c4384
repeat-log 5c4950e39110eb0023e86a616abd3d6e7e8dfc507cf27b68ff01607ffb32ff7e
release-help-log 66b4d6e11ad50c7b8af137c84314783f34ccdc3a9c844fb52c96a9be7482fcf0
debug-receipt 7b13a8ee7040df8849e25b05b34deb3b0adfa9d122fda858c8555285b71ad7ea
```

Goal active/incomplete. Canonical fresh publication is now source-bound and
testable; cross-path byte reproducibility, full-library failures, current APK,
physical QR/direct-LAN sync/recovery, connected API31/34/35 and signing gates
remain unresolved. This is progress, not complete or blocked status.

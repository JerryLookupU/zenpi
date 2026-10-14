# Stage 2 Checkpoint And Release Link Repair

Recorded 2026-10-04; partial S2-015/S2-070/S2-072 evidence, not acceptance.
Previous turn was progress: the strengthened cross-location gate exposed a
48-byte release mismatch. This turn repairs that mismatch and a checkpoint
file admission gap. No nested agents, reset, unrelated cleanup or denied
TCP/Gradle retry was used.

## Checkpoint Repair

Owner checkpoint reads previously inspected a pathname before opening it;
replacement between those operations could evade the regular-file/size check.
Unix reads now use O_NOFOLLOW | O_NONBLOCK and inspect the opened descriptor.
The checkpoint lock is also non-blocking/no-follow and must be a regular file
before flock. Invalid file/lock types refuse without checkpoint publication.
This does not establish parent-directory safety or repair directory-fsync
ambiguity after rename.

Three new Rust tests pass, including a child-process-bounded FIFO read/lock
test, symlink/directory/oversize refusals, unchanged files and normal CAS
publication. Tests ran again after final source/format edits. The selected
remote_workspace (72), remote_migration (6), remote_subtab (11), and
remote_concurrency (12) regression targets pass 101 primary tests. Child
re-executions are not counted as extra primary cases. cargo check --locked
--offline, cargo fmt --check and git diff --check pass.

## Link Policy And Fresh Builds

Additional -ld_classic diagnostics did not eliminate the cross-location
difference and were not integrated. The controlled -Wl,-S,-x,-reproducible
experiment did: it applies the existing symbol-stripping intent at link time,
before the observed UUID/signature discrepancy arises. build.rs applies those
flags only to the zenpi binary on macOS release builds. A compiled build-script
probe confirms macOS debug and Linux/Android release emit no Apple linker flag.
Actual fresh default builds are the decisive evidence; exact linker-internal
causality remains an inference, not a proven source-level diagnosis.

S2-070 now declares build.rs ownership; selector requirement digest and
generated Gantt were refreshed without changing any checkbox. Build-script
addition/change/removal provenance refusal has a new Python regression. The
full Stage 2 Python suite passes 158 tests, host-builder tests 12, lifecycle
fixtures 25 and Blueprint fixtures five. Strict Blueprint passes with all
36 items [ ], zero [_]/[x].

The first fresh comparison correctly refused source_or_toolchain_changed
after checkpoint test edits during lane A. Its failed receipt is preserved:
/private/tmp/stage2-link-time-strip-repeat-20261004.json.
The final comparison, process 17240, exited zero with unchanged inputs:
/private/tmp/stage2-link-time-strip-final-repeat-20261004.json.
Lane A is under the system temporary directory; lane B is under repository
target/stage2-release-compare-p8qxdlao/b. Both exact artifacts hash to
e9cf51915b90f25ce86be549ac59d4151aff889ec5621554dfd34a38a602349e.
The independent canonical --profile release --fresh build, process 91766,
also exited zero and published the same bytes. cmp of canonical against each
lane exits zero; all three pass codesign --verify --strict. otool confirms
LC_UUID and LC_CODE_SIGNATURE remain present. No UUID/signature removal,
post-build normalization or relaxed comparator was introduced.

Current debug attribution rebuilt successfully. Current release/debug
verify-only checks pass. The real published release host pair --help exits
zero with HOME removed and an absent private ZENPI_HOME, which remains absent.

```text
build.rs f05a7b006aff9718ad9c4dbc7c5535a0204be5dc50951674820ff8a4f9a7c6ff
project_workspace.rs 278692b5b84d498c19addf860a7fd6f93213d576a929188734eea71c436181e7
builder-tests 4492d6146814d108dd3f2e6800d0f380b9177f84ec376a521c73f51b602279b8
repeat-receipt 9cc4a7db325ff2fcb76e4305b62c963a9e72f97e6bd6633c14a29878f4c3607d
repeat-inputs 98eac8e346e8fc46688a5539586e3cecc846e0ea5efdba2d88e18f7119f5c227
release-receipt e46d4b308ae0a3ed48f5acf70fbbf4df27525af5f483762e12307515f5581e61
debug-receipt eec4a7d51c7a7d5aa4663b340800ea667d37ac42ff4753e5b1403acb75d65b30
host-inputs e8c86f1a8c78bd18a1fee402f7bfe6a2937ceb68aa10e6a2e9123eb24c6250d8
```

## Remaining Acceptance

The successful comparison proves same-machine/toolchain cross-location bytes,
not supported-host runtime, cross-host reproducibility or complete S2-070.
Android verify-only still refuses missing APK provenance. Production QR camera,
direct LAN sync/recovery, API31/34/35 lifecycle, signed install/upgrade and the
previous full-library failures remain required. The goal stays active and
incomplete; no item was accepted from narrow tests.

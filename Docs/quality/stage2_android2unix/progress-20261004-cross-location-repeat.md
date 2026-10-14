# Stage 2 Cross-Location Release Comparison

Recorded 2026-10-04; partial S2-070 evidence, not acceptance. The preceding
goal turn verified RTK availability but changed no product state. This turn
revalidated the worktree and tightened an insufficient release comparison.
User changes and the unchecked authoritative Blueprint remain preserved.

## Change

The repeat checker previously built two siblings under one temporary parent.
That passed while the repository publication target produced different
Mach-O UUID/signature bytes. The default now compares an empty temporary
target with an independently allocated empty repository target. Exact parents,
target directories, commands, artifacts and retained raw logs are recorded.
The explicit secondary-parent option is supported; symlink parents refuse
before a child process starts. Existing evidence is never overwritten.

Two new tests cover publication-location-sensitive artifact mismatch and
explicit-parent/symlink refusal. The existing success test verifies default
location separation and receipt bindings. Nine checker tests pass; the full
Stage 2 Python suite passes 157 tests. Lifecycle fixtures pass 25 and Blueprint
fixtures pass five. cargo fmt --check, git diff --check and strict Blueprint
validation exit zero; all 36 items remain [ ], with no [_] or [x].

## Actual Build Evidence

python3 tools/stage2_host_reproducibility.py --receipt
/private/tmp/stage2-cross-location-repeat-20261004.json ran through terminal
exit 1 with failure release_artifacts_differ. Both required Cargo builds
individually exited zero and source/toolchain inventory remained unchanged.

Lane A target:
/var/folders/gf/yj6lvtfj65922j9bnkfc7ddm0000gn/T/stage2-release-compare-clp3wf3f/a
Artifact SHA-256:
3d7f07c42677e213299793ae614ac7e6552f44eea6d221805da07a31685a4af2

Lane B target:
/Users/mac/Github/zenpi/target/stage2-release-compare-waxwntd8/b
Artifact SHA-256:
31895ff5932b30281dc4ce96bc584982ae4925b2a963d99252d57861cc741799

cmp -l reports exactly 48 differing bytes: offsets 1945..1960 and
9266593..9266624, matching the earlier UUID/signature locations. Both actual
artifacts pass codesign --verify --strict. Valid signatures are not evidence
of byte reproducibility. No normalization or signature/UUID removal occurred.

Controlled cargo rustc experiments against the retained earlier targets with
-C link-arg=-Wl,-reproducible and per-target -oso_prefix each completed, but
neither eliminated the difference. These were diagnostic incremental links,
not fresh release acceptance. Their flags were not integrated into Cargo.toml.
Ordinary cargo build restored the earlier target artifacts to their recorded
hashes; current canonical release/debug source-attribution checks pass.
The canonical release remains d32407fb556406fa662d1b2e4eb909bb1006ef7fd82854588ef32d6030cf34e2.
Its real host pair --help exits zero with HOME removed and a private absent
ZENPI_HOME; no listener was attempted.

## Evidence Identity And Remaining Work

```text
checker 28fed7521ceeb7cb45c037e94d16d4963cd093b03956f2e22a6570dada943fcd
checker-tests fa5751f84985cb63a57372c301d09694cd2d4215603e33e1bdc0761c5f7bdaf8
receipt 262be7168a4b1dadda822a24752dfdf24e7944d9a88ae9d3df82b2d7353edf2a
inputs d4d93a0b83c6c2942fda944a10b36450217cea032a6726a9017a21f867d3d7d4
```

Android build verify-only still exits one for missing APK provenance. The
retained prior Gradle failure is a denied local daemon TCP bind; no equivalent
denied build/listener was retried. Physical QR/direct-LAN sync and recovery,
connected API31/34/35, signed install/upgrade, full-library failures and
cross-location release reproducibility remain unverified or failing. This
turn improves the release gate and supplies fresh counterevidence; it does
not claim Android or whole-item acceptance. Goal remains active/incomplete.

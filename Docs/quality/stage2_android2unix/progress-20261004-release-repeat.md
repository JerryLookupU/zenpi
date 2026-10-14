# Stage 2 Fresh Release Repeat Comparison

Recorded 2026-10-04T10:15:35Z; partial S2-070 evidence, not item acceptance.
Previous goal turn was progress: current cached release attribution and 133
protocol/security regressions. Dirty worktree preserved; no nested agents,
reset, socket-policy bypass or repeated denied Gradle/ADB attempt.

## Implementation And Actual Runs

Added tools/stage2_host_reproducibility.py. It builds twice with offline/locked
release Cargo in separate newly created empty target directories, inventories
source and checker, records verbose Rust/Cargo identity and sanitized environment
names, hashes private raw logs and binds exactly one reported executable.
Source/toolchain drift, failure/timeout, wrong/symlink/malformed artifact report
or unequal artifacts cannot pass. Failed lane one never starts lane two; no
retry is built in. Existing receipt paths are refused before execution.
No canonical artifact or provenance is overwritten by the comparison.

Seven unit fixtures cover equal/unequal builds, failure/timeout, source and
toolchain drift, artifact substitution/malformed JSON, and existing evidence.
The complete Stage 2 Python suite passes 152 tests; lifecycle fixtures pass
25, Blueprint fixtures five. Format/diff and strict Blueprint pass; all
36 authority rows remain [ ]. No new Kotlin compilation is claimed.

Initial actual process 71649 built lane a successfully but rejected source
drift because the checker was refined during execution. It exited 1 with
source_or_toolchain_changed and never ran lane b. Its failed receipt/log remain
/private/tmp/stage2-host-reproducibility-20261004.json and .log. This is not a
passed comparison and was not restarted merely because it was silent.

After freezing the final checker, process 33690 ran to authoritative exit 0:
python3 tools/stage2_host_reproducibility.py --receipt
/private/tmp/stage2-host-reproducibility-final-20261004.json --runtime-parent
/private/tmp. Both offline/locked release builds succeeded in
/private/tmp/stage2-release-compare-ozi7bnos/a and b. Toolchain rustc 1.98.0
(88d9e12ae, LLVM 22.1.8), Cargo 1.98.0 (797e8a9bc), aarch64-apple-darwin;
same local source/toolchain/environment only. Both artifacts hash to
6b0535aad15e5dca888bb7e5ea1758abd0d6d2fa94030ab3e8f6f5919a01f529.

The earlier canonical target/release/zenpi remains f32c9381024afb00099c519822a2b8a65c2c99cc5216a59e9dbcb124c35e6d6b,
and its attribution verify-only still passes. It differs from the fresh pair;
cached source attribution does not prove equivalent build/toolchain history.
No claim is made that the canonical artifact is a clean reproducible build.
Supported-host/cross-host build comparison and canonical clean artifact handoff
remain open, as do the full-library failures recorded in the broad audit.

```text
checker e4bd2969fd5c4c9360098b52801c7853a61afe04650db74909c502543d36f34c
checker-tests 88011240c652b267b3325984e2ae9aab4628eef938be8b27a2b7b1e2b0dd26dd
final-receipt 42b78d1ca71f1d512a1976ab2fa1ec2b4c17bafcf45cdf0901c63836b6ff5703
comparison-inputs 3c45c3948ea887707007e892f1bd6d0f626f106f66dad615550886cd31530a86
python-log c7aac75f55812c7efc724f56fd6f904f13517e83dc0a48f6147eef63e50ead58
lane-a-stdout 858a216e8f20371052b456a13d2952baeb8e4d245ff13a68cad6507fe1e9f3c9
lane-a-stderr 674eb47993808f6d3b1fafab276059e184d2fa08de6f8645c68fd9d50bf2bf21
lane-b-stdout 095b9e80f488c89abc7465ef638644df27402c6fde455538a09cb21d2cb9a306
lane-b-stderr aad78b30fccceab2b6268874099381aa562ac4c967420a50f102182d8c165d13
```

Goal active/incomplete. Fresh local repeat evidence advances the release gate,
but does not prove current APK, signing, runtime listener, physical QR/direct
LAN synchronization/recovery, API31/34/35 lifecycle or complete parity.

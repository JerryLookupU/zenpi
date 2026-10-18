# Current Subscription Release Repeat

Recorded 2026-10-04. Partial S2-070/S2-099 evidence, not item acceptance.
Previous goal turn was progress (production subscription delivery refusal).
Execution skill, authoritative Blueprint, dirty worktree, host attribution,
physical QR harness/test and missing APK provenance were re-inspected. No
nested agents, reset, stale APK installation or repeated denied Gradle/ADB run.

## Execution

The first independent comparison built both lanes successfully and obtained
identical binaries, but correctly failed source_or_toolchain_changed: the
comparison verifier itself was being extended during lane B. Retained failed
receipt /private/tmp/stage2-subscription-release-compare-20261004.json has SHA
b3cd93a5b99617f09771ef4e7edb480d5f21eea114812d7d6be7ba22d222d0df.
It was not changed to passing or reused for the next execution.

After freezing the verifier source, ran from /Users/mac/Github/zenpi:

```sh
python3 tools/stage2_host_reproducibility.py --receipt /private/tmp/stage2-subscription-release-compare-verified-20261004.json
python3 tools/stage2_host_reproducibility.py --receipt /private/tmp/stage2-subscription-release-compare-verified-20261004.json --verify-only --binary target/release/zenpi
```

Both commands terminal exit 0. Initially empty independent build locations:

- /var/folders/gf/yj6lvtfj65922j9bnkfc7ddm0000gn/T/stage2-release-compare-wy4kks1s/a
- /Users/mac/Github/zenpi/target/stage2-release-compare-6hc82h0n/b

Both built artifacts and current attributed canonical release hash exactly to
0368bb32c96052364f93afed3436ab213aa1195441e637eb5ddaeba9c808ff95.
cmp canonical against lane A exits 0; both fresh binaries pass codesign
--verify --strict. Success receipt SHA:
0579068b0a4d275080999cd0b497dabc139a271f6e8a9809e64cc4f1cecdb7b6.
Comparison inputs SHA:
b40cc9da99d3309bbab464d785a6f8e1029952295b4964aa11bd02293ba73a16.
This is same-host/toolchain/output-location reproducibility, not cross-host,
production TCP, signed APK or whole-product release acceptance.

## Retained Evidence Gate

Added --verify-only and optional --binary to the comparison tool. It refuses
failed/overclaiming/stale receipts, source/toolchain changes, wrong commands,
non-independent targets, unsafe symlink paths, changed retained logs/artifacts
and mismatch with the current attributed release. It performs no build or
receipt rewrite. Thirteen unit tests include failure/drift/independence,
publication mismatch and saved receipt/artifact symlink negatives.

Physical camera harness was inspected: it requires a fresh dedicated physical
API31+ installation, no adb reverse, current APK/host provenance, visible
terminal QR, manual SAS, explicit read/control grants, local/background desktop
updates, foreground replay and Android prompt/desktop answer with durable
journal bindings. Its ten harness tests pass, but no physical execution is
claimed. Current APK provenance remains absent; existing denied wrapper/cache
and daemon-listener diagnoses were preserved, not retried without change.

## Other Verification

- Full Stage 2 Python tool suite: 163 pass (after the final symlink test).
- Lifecycle fixtures: 25 pass; Blueprint fixtures: 5 pass.
- cargo fmt --check, git diff --check, strict Blueprint: pass; 36 [ ], no [_]/[x].
- Debug/release source attribution verify-only pass; binaries unchanged.
- Current release host pair --help exits 0 with HOME unset/nonexistent private
  ZENPI_HOME; that directory remains absent. No listener/identity was started.
- No Rust/Android production source changed this turn; previous 591 JVM,
  encrypted-pipe and interop evidence remains scoped to its own source run.

Goal active/incomplete: current APK, signing/install/upgrade, physical camera
QR/direct-LAN pairing/sync/recovery, connected API31/34/35, full-library and
supported-host acceptance remain required. All observed process handles terminal.

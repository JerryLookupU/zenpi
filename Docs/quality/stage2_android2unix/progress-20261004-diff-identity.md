# Stage 2 Diff Identity And History Progress

Recorded 2026-10-04 UTC. Related requirements S2-032 transport, S2-036 diff
projection, S2-052 admission and S2-054 parity. Partial evidence only; all 36
authoritative Blueprint rows remain [ ]. No whole-item acceptance claimed.

## Authority

Cwd /Users/mac/Github/zenpi; integrated dirty-worktree HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b. Existing work preserved, no nested
agents, reset, push, permission bypass or stale APK installation.
Blueprint SHA-256 d76e70e369f11f0fd3d1905d56d79c2a89401b92215da48ad85314f390d96d97;
specification SHA-256 403e6339b0318a420d93600fc52926acf6a9e46ec899bd9a9edb03c235190a28.

## Implementation

Diff scope previously matched project ID and session together, permitting a
mixed duplicate ID to appear unique. New readable admission requires selected
ID exactly once, matching session, diff capability and one real active subtab.
Policy, initial/continuation requests, decode and selected-page checks reuse
it. Late asynchronous ViewModel results now revalidate full current selection
and advertised capability, rather than only raw selected IDs/tab revision.

History was previously checked for pairwise identity and contiguous offsets,
not each retained page's actual UTF-8 bytes. New document/chain validation
checks path, digest, byte size, offsets, total, hasMore, status bound and all
retained pages, with the existing seven-page limit. Malformed current pages
cannot build wire continuations; corrupt earlier pages cannot authorize either
navigation direction. Initial retained pages require offset zero. Closed diff
dialogs do not navigate. Legacy unpaged bounded reads remain compatible and
failed reads retain fresh refresh availability without mutation authority.

Two new JVM tests cover mixed duplicate scope, legacy/paged/continuation
refusals, malformed historical bytes/digest/offset/size/path and positive
navigation. Actual Rust/Kotlin diff lane checks duplicate refusal on real
desktop Git projections and validates intact retained chains. Existing Compose
diff case checks corrupt history disables next while fresh refresh remains
available, and ambiguous scope disables refresh. Instrumentation compiled only.

## Verification

Commands ran from the cwd above. Source checker controls ZENPI_ANDROID_*
fixture paths and isolated ZENPI_HOME directories as declared in its script.
Standalone host test used a fresh temporary configuration, not user state.

- zsh tools/stage2_android_source_check.sh /private/tmp/stage2-diff-identity-20261004:
  exit 0, 525 ordinary JVM tests; main/unit/instrumented compilation and every
  explicit Kotlin/Rust lane passed, including real desktop Git pages/change
  digest conflict/foreign session/revoke with unchanged journal, six-test
  desktop target and production tool-loop lane. No source/test edits after run.
- ZENPI_HOME=/private/tmp/stage2-diff-identity-host-home-20261004 rtk cargo test
  --locked --offline --test remote_diff: exit 0, 1 real Git test passed.
- python3 -m unittest discover -s tools -p 'test_stage2_*.py': exit 0, 137 pass.
- python3 tools/test_validate_stage2_android_lifecycle.py: exit 0, 25 pass.
- cargo fmt --check, git diff --check and strict Blueprint: exit 0.
- python3 tools/stage2_host_build.py and --verify-only: exit 0 after Cargo
  lanes. Debug host attribution is not reproducible release evidence.
- python3 tools/stage2_android_build.py --verify-only: exit 1, missing APK build
  provenance. No unchanged Gradle/ADB permission retry or physical QR/LAN claim.

```text
DiffProjection.kt e47be0ed7c4f21d7d0e513d3278669616d1ea528020bd5fbcfd1a6e2d72ff263
DiffReadPolicy.kt 698ebbf7899aa40283b824edc192162703b41822dfa4924c07f0eb7594088334
PairingViewModel.kt c2be3ef972bf25ba94c7293f5ba24410bbd8e8bcaffd3dc99dc9cf3801b205e4
source-log 4822152296e47300612c5425160e7ca7ec06c19bf7c9ad29aa373834fc6e76b5
python-log a6e2f9b48a87e660dc929afd5fc197e66d9808c3896784e49a21d8470acdbd54
host-test-log d036c2acbccc32b82404e2e030349145e8d105d444793e3d9bdc7775850eb435
main.jar ed72f8fad8ea426190b25889a0b8e6933549a1d15d56c7f9e09c5da1c5734f3c
unit.jar 0bd091923f9d0c5aa4cf86c211d71d6f17ebce2010193a3aa83602d753efa52b
instrumented.jar 420f0a28893f4cd92fba6a09b50d802068be267209f46453813b2cd31f2cbd64
debug-host-receipt a92182041f3c6b9141cfa0dca24ec9197210503e6444fa85ca52b97341ca9e56
```

Goal active and incomplete. Current APK, physical camera QR/direct-LAN
bidirectional synchronization/recovery, connected API31/34/35 lifecycle,
complete desktop parity and signed reproducible release remain required.
Source/JVM/real-Git fixtures do not replace these acceptance gates.

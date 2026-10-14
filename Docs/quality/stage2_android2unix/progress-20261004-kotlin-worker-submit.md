# Stage 2 Kotlin Worker Submission Through Actual Agent Lanes

Recorded 2026-10-04. Partial S2-019/S2-035/S2-036/S2-054 evidence. Previous
goal turn strengthened actual Agent cancellation; this turn replaces Rust-built
initial prompt envelopes with current Kotlin construction/framing. Canonical
cwd /Users/mac/Github/zenpi; dirty HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b. Existing changes preserved; no nested
agents, reset, push or bypass. All 36 Blueprint rows remain [ ].
Blueprint SHA-256 d76e70e369f11f0fd3d1905d56d79c2a89401b92215da48ad85314f390d96d97;
spec SHA-256 403e6339b0318a420d93600fc52926acf6a9e46ec899bd9a9edb03c235190a28.

## Evidence

The existing explicit worker-cancel lane now invokes two Kotlin submit phases.
Actual owned listing/snapshot/SubtabList feed current projection decoders,
workerPrompt, shared admission and FrameCodec. Fixture explicitly selects the
displayed subtab and replaces the generated UUID with stable fixture IDs before
strict restore; it does not claim real Android UI selection. Low-level listing
advertises worker_control explicitly as documented in previous reports.

Rust decodes one Kotlin frame per task, verifies complete text/tab/revision
arguments, and admits/retries each exact envelope through WorktreeHost. Two
independent real Agents execute production spawn_observed_prompts and controlled
backends. Actual input turns, newline previews, one targeted cancellation,
sibling successful answer and master/worker journal isolation remain checked.

After joining both tasks, fresh disk Agent/pool and a new empty WorktreeHost
return the original accepted submission receipts. No worker owners are prepared,
backend starts remain two, and journal bytes do not change. Kotlin validates
the two original submission receipts plus prior cancel responses and restored
equal receipts. The explicit lane now has four Kotlin phases: submit twice,
generate cancel, validate. No production runtime/policy changed this turn.

Controlled backend/fixture identity is not external provider, OS process crash,
Android Keystore, production TCP/TLS listener, actual device UI, QR or physical
LAN evidence. The generated submit-to-Agent chain is partial product evidence.

## Verification

Commands from canonical cwd. Source checker controls OUT and isolated fixture
ZENPI_HOME directories; no user provider credentials/config or terminal changes.

- Initial /private/tmp/stage2-kotlin-worker-submit-20261004 checker exit 1:
  Kotlin fixture used JsonArray.size property; compiler requires size(). Fixed
  test accessor; failed log retained, not promoted.
- zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-kotlin-worker-submit-final-20261004: exit 0, 536 ordinary
  JVM tests, main/unit/instrumented compilation, production QR corpus, all
  explicit interop lanes, six-test desktop target, real worker-tool/prompt and
  extended worker submit/cancel lane (one Rust test/four Kotlin phases).
  No source/test edits after final run. Compilation is not device execution.
- python3 -m unittest discover -s tools -p 'test_stage2_*.py': exit 0, 137 pass;
  /private/tmp/stage2-kotlin-worker-submit-python-20261004.log.
- python3 tools/test_validate_stage2_android_lifecycle.py: exit 0, 25 pass;
  validator fixtures, not connected API matrix.
- cargo fmt --check, git diff --check and strict Blueprint: exit 0.
- python3 tools/stage2_host_build.py and --verify-only: exit 0 after Cargo lanes;
  cfg(test) source under src changes attributed input/binary build identity.
- python3 tools/stage2_android_build.py --verify-only: exit 1 missing APK
  provenance. Unchanged Gradle/ADB permission failures not retried/bypassed.

Final log: /private/tmp/stage2-kotlin-worker-submit-final-20261004.log.
Frames/receipts: /private/tmp/stage2-kotlin-worker-submit-final-20261004/android-worker-cancel-results.json.

```text
worktree_host.rs 063df4d134a5d28bd647d2bb6e281c993cd83666da7696c17fadd32212e62696
AndroidRustWorkerCancelTest.kt 65cb13c0135eb528b92f74c8a337a2b653b1be5186cd41905535277588ab98df
final-log e79716f8df5ee1a127105a66475acfaddc64c477935f5a71b66d6def6693b2c2
worker-submit-cancel-results ab2906eda39b177a7969d5c0439c521adcbe2a6031353c45148f46acfe2c4f3d
python-log 2483dd1120b48d74142c3c42a8b68e85147d6ddd3739cf6b1a81cdcb16a4403a
debug-host-receipt b6376a3a9af0cba0b67ebfc52df011fe32e9953f015637a2b67e53a8f6468e92
debug-host-inputs 6b0e0e71ebcd761b5f8a5e48f03e646fd5cc3e7852857e02a8ba210da3f9edd1
debug-host-binary 1560632ca356eaa00dfec6f7b308a036d332f4c894b55426417b0099ec546113
```

Goal active/incomplete. Current APK, physical camera QR/direct-LAN bidirectional
sync/recovery, connected API31/34/35 lifecycle, full desktop parity and signed
reproducible release still require authoritative acceptance. Current-source
task submission evidence does not substitute for those gates.

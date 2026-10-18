# Later-Page Host Read Boundary

Provisional S2-036/S2-052/S2-054 progress at 2026-10-03T20:10:23+0800.
Previous goal turn connected page-only Android output/history reads. This turn
adds host-domain and framed-client regression evidence instead of promoting
UI callbacks to end-to-end acceptance. No nested agents, worker launch,
checklist promotion, commit/push or cleanup.

## New Coverage

Rust WorktreeHost test admits and completes 18 real worker pipeline tasks with
the controlled Echo backend, pages their device-scoped status list, and chooses
an actual second-page completion absent from the 16-entry summary. Its output
and history are read twice and identity/subtab/content are checked. Another
authorized device cannot read that task identity. Master and every worker file
remain byte-identical across these reads, workspace revision is unchanged,
no worker handles are recreated and no execution remains busy. This exercises
production task admission/runner/journal/read code but Echo is not a real
provider, and there is no binary/TLS/Android/camera/LAN execution in this test.

New Android JVM test feeds a fragmented, correlated history response through
ResponseReader and WorkerHistoryCodec using a page-only completed target.
It opens the scoped history reader without changing the main projection,
encrypted-cache encoding or pending writes. Foreign project/session/request/
subtab receipts are rejected without replacing valid history. This is actual
framed-memory client parsing, not a device or network receipt.

## Results

- Isolated Rust command with ZENPI_HOME=/private/tmp/zenpi-resource-reload-test-home:
  cargo test --offline --locked --lib remote::worktree_host::tests:: -- --test-threads=1
  exit 0, all 17 tests pass, 7.97 seconds. Existing dependency/library warnings
  remain. An initial run without writable isolated configuration refused worker
  admission as Busy, not a pass. After isolation, a foreign task expectation was
  corrected from Unauthorized to the existing nondisclosing InvalidRequest
  contract. No production authorization behavior was weakened.
- Python Stage 2 tooling: 86 pass; host provenance tests: 8 pass;
  backup/lifecycle/Blueprint validator tests: 5/15/5 pass.
- Strict Blueprint validator, cargo fmt --all -- --check and git diff --check
  pass. All 36 authoritative rows remain unfinished.
- Adding a cfg(test) block changes the deliberately conservative host source
  inventory; the old receipt correctly refused stale source. A fresh
  python3 tools/stage2_host_build.py --profile release completed, then
  --verify-only passed. This is attribution, not reproducible release proof.
- Current APK verify-only still refuses missing successful build provenance.
  No repeated blocked Gradle/socket attempt or stale APK installation.

```text
worktree_host.rs daf92e7fb8e26870ec2bdd27a446b38c0662cc736503d75dd23ca4442993b495
WorkerReadPolicyTest.kt 5e3a667f61f369307fdd4f91f1cf637c1d64c8b9a5c3502a754be86ec0c6e0df
release source inventory b6cac3ef3196283eeb7c6abdaf14a4e3ad2e0fd8c23e69edf9558c844ac64c0c
release receipt 6f6286411fba8e3b0f2e5363eb44633a9f974e136df063a09f59484fa59afac0
release binary 7e1b2b68b9940c8b0312598c4edc42f277f4acd5b962c185fad7600fae481a45
```

Goal active. Current APK, real-host Android later-page operations, camera QR/
direct LAN, connected lifecycle/full parity and signed install/upgrade remain
open. The Rust/Kotlin tests do not close those product gates.

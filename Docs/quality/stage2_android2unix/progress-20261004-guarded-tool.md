# Stage 2 Kotlin Approval Guards Real Tool Effects

Recorded 2026-10-04 UTC. Related S2-019 routing, S2-036 approval controls,
S2-052 security and S2-054 parity. Partial evidence, all 36 Blueprint rows [ ].
Cwd /Users/mac/Github/zenpi, dirty-worktree HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b. Existing changes preserved; no nested
agents, reset, push, permission bypass or stale APK installation.
Blueprint SHA-256 d76e70e369f11f0fd3d1905d56d79c2a89401b92215da48ad85314f390d96d97;
specification SHA-256 403e6339b0318a420d93600fc52926acf6a9e46ec899bd9a9edb03c235190a28.

## New Evidence

Current explicit Kotlin/headless owner lane now uses Always approval rather
than the prior Never fixture. Controlled first completion requests two
production write_file calls. A separate authenticated Read+Approval device has
no Control; original Read+Control device has no Approval. Actual host pending
previews feed Kotlin's approve construction and policy preflight. Control-only
decisions and wrong digests refuse before tool effects. Kotlin allows first
call and denies second. Allowed file remains unchanged before Allow, then
contains exact expected bytes. Denied file never exists. One success and one
error tool result are durable; Kotlin checks actual success path/bytes/diff.

Both approvals remain one-shot, remember=false, source host_answer. Session
audit's remote actor matches host-authenticated approver device, generated
request, project/session and full request digest. Only Allow has a matching
approval_consumed record. Exact decision receipts survive fresh-Agent disk
recovery with zero recovery backend execution, then both refuse after approver
revocation. Original prompt/steering/cancellation, one Applied steering input,
two backend calls, stopped status and no duplicate effects remain covered.

This is a controlled backend with actual Agent/coordinator/tools/filesystem;
not an external provider, physical Android Keystore or production TCP/TLS/LAN
proof. No production runtime logic changed or permission policy relaxed.

## Verification

Commands ran from cwd above, source checker controls ZENPI_ANDROID_* fixture
paths and isolated ZENPI_HOME=$OUT/prompt-home. No user credentials/config changed.

- Initial /private/tmp/stage2-guarded-tool-20261004 checker exit 101: actual
  approval phases passed, cancellation fixture expected only one text delta
  despite two backend calls. Corrected to wait for and assert cumulative two
  deltas; failure retained, not counted as pass.
- /private/tmp/stage2-guarded-tool-final-20261004 checker exit 0 before audit
  assertions were added, preserved intermediate evidence.
- zsh tools/stage2_android_source_check.sh /private/tmp/stage2-guarded-tool-audited-20261004:
  exit 0, 529 ordinary JVM tests, all source compilations, production QR corpus,
  all explicit interop lanes, six-test desktop target and production tool-loop
  pass. Extended owner lane: one Rust test/six Kotlin phases passed. No code or
  test edits after audited run.
- python3 -m unittest discover -s tools -p 'test_stage2_*.py': exit 0, 137 pass.
- python3 tools/test_validate_stage2_android_lifecycle.py: exit 0, 25 pass.
- cargo fmt --check, git diff --check and strict Blueprint: exit 0.
- python3 tools/stage2_host_build.py and --verify-only: exit 0 after Cargo lanes.
- python3 tools/stage2_android_build.py --verify-only: exit 1 missing APK
  provenance; unchanged Gradle/ADB permission failures not retried.

Evidence JSON:
/private/tmp/stage2-guarded-tool-audited-20261004/android-prompt-results.json.

```text
remote_headless_commands.rs 61a4a39926b7fc687f5a2a4f81bc1536a91449c83d28c71af35de0cd6df8faf7
AndroidRustPromptTest.kt fa60183e116360ed4ff0e1e5b731ab1810da7493d11af5091e2f1d631652b6ba
audited-log cb3ffc1877eeeee642bb07859cbb1b149526fc10690790741ba481beab30560a
prompt-results d6ecb658776f7d7e1654b4fcbce4bf1b690eeeec495bdf8993a0ad3865b1467c
python-log 622b5f8248ca7055dce15a2f21215efe238660c83bca31bc45f19d381bd9bdae
unit.jar 24784d27b2e16ff956681805a34138bff35bd464f5cd22f2f7cce1183f1cf229
debug-host-receipt 2d3a74614d71f72a8809badb0a2bb63a92711fea808ecd2fa66a424e0a766a7b
```

Goal active and incomplete. Current APK, physical QR/direct-LAN sync/recovery,
connected API31/34/35 lifecycle, complete desktop parity and signed reproducible
release remain required; controlled-tool approval does not replace these gates.

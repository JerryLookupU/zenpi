# Remote Diff Private Path Boundary

Provisional S2-036/S2-052/S2-054/S2-072 progress observed
2026-10-02T06:06:51Z. Previous goal work made implementation progress; the
intervening RTK request confirmed the existing Homebrew installation rather
than changing Stage 2. Resumed outstanding Cargo handles were terminal,
successful: 1 remote Diff test, 45 integration tests and 29 slash-action tests.
HEAD 1a8ff1acf72e0816fad503a05df2b1d18e07361b; dirty user changes preserved.
All 36 authoritative rows remain [ ]; full Android/QR/LAN/Rust desktop goal
active. No worker admission, subagents, commit, push, cleanup or acceptance.

## Implementation

Remote Diff now calls a credential-filtered variant of the existing bounded
local reader. Explicit workspace-relative known-private components refuse
before binary inspection or Git content reads. Existing credential names and
suffix globs are shared with the host tool/listing/attachment policy. Git
status and tracked diff receive case-insensitive component/descendant exclude
pathspecs; normal requested paths use native-byte literal pathspecs. Remote
Git rename/copy detection is explicitly disabled to avoid excluded filenames
in paired headers. Inherited literal/glob/case pathspec environment overrides
are removed for these remote queries. Untracked synthetic patches also check
the same credential policy before reading their target. Local desktop Diff
retains full credential-file inspection and the previous reader behavior.

Android Diff request and response-path validation reuse WorkspaceFilesCodec's
known-private-path policy. Ordinary dotfiles and literal bracket/colon names
remain valid. The client does not parse patch text to replace host filtering.
Protocol and operator guide now describe this remote-only boundary and its
known limitations. Existing scope/CAS/authentication/cancellation/byte bounds
remain in place; review does not execute a provider or mutate owner journals.

## Verification

Cwd /Users/mac/Github/zenpi; Rust environment
ZENPI_HOME=/private/tmp/zenpi-private-diff. All commands exit 0:

- rtk cargo test --offline --locked --test remote_diff --test remote_workspace
  --test remote_parity --test remote_subscription_tls: **82 pass**. Real Git
  fixtures cover a selected in-place child tab, modified/deleted/renamed private
  files, private-directory descendants, untracked keys, explicit refusal,
  normal tracked/untracked patches and unchanged local private-file inspection.
  Two added tests send Diff through actual rustls TLS 1.3 encryption over
  fragmented in-memory pipes with P-256 device proof and production HostAdapter
  dispatch. They check no known-private name/content/root escapes, explicit
  private-path refusal, current revocation refusal, and unchanged journals and
  checkpoints. These tests do not bind TCP or constitute real camera/LAN proof.
- `rtk cargo test --offline --locked --lib slash_actions::`: **31 pass**, 366
  filtered. Two new tests cover committed/unborn HEAD, literal bracket/magic/
  backslash names, ordinary dotfile admission and all four inherited Git
  pathspec environment modes in isolated child test processes. Existing
  bounded-reader/cancellation/local-path tests remain green.
- zsh tools/stage2_android_source_check.sh
  /private/tmp/zenpi-private-diff-android: **178 JVM tests**, all main/unit/
  instrumented sources compile. Instrumented tests not executed; no new APK.
- env PYTHONPATH=tools python3 -m unittest
  tools.test_stage2_desktop_android_smoke tools.test_stage2_headless_android_smoke
  tools.test_stage2_scanner_android_smoke tools.test_validate_stage2_android_lifecycle
  tools.test_validate_stage2_android2unix_blueprint tools.test_validate_stage2_android_backup:
  **63 pass**.
- rtk cargo check --offline --locked: 0 errors, 20 existing warnings.
- cargo fmt --all -- --check; git diff --check; python3
  tools/validate_stage2_android2unix_blueprint.py --strict: exit 0.

Final Android SHA256:

```text
main.jar 67913608fe58b1c82379afd62b78c9b1066838fcff084c483b201c4b9533c341
unit.jar 37219b3352ec7ecbfc18bceb335e5cdcfbd8736da88907d146c64906377645b2
instrumented.jar d5ab3994cc030b694bb64c06a641967d634848d1e9879064ed3536b6e00d828c
```

Source SHA256:

```text
Docs/stage2_android2unix_blueprint.md d76e70e369f11f0fd3d1905d56d79c2a89401b92215da48ad85314f390d96d97
Docs/stage2_android2unix_spec.md 403e6339b0318a420d93600fc52926acf6a9e46ec899bd9a9edb03c235190a28
src/remote/diff.rs 5dcdeeb5e584c703d82c7c8ecfb1e056c556038771ed525929e2d660c89f475f
src/slash_actions.rs 7e792475f0a91c058dacf9a9a5297b2f17650e81cfa11d127ffa5ece8d40ab15
src/tools.rs 69b27e0b3a2d848e208d691d90ca3674694f9764e043d91e5aaf0e7d3b607991
tests/remote_diff.rs 6bdc14c9caf4cc24e9d41e04ea3003f1f11c48b24d1d52520db514b9efd02242
tests/remote_subscription_tls.rs dde991d52aeb080de7efa035ecc547cf9cb1f33aa9f4dc7239c594a56f58ff80
DiffProjection.kt b5ea066cf830aec98e1cb2f28ceb52f962fc3853f03a6ec98889a7cfba97ba8d
WorkspaceFilesCodec.kt 6424aadcf1c7888d4f8f4b7804a229b7b21bcbdee6344553a286a5febc1f44dd
DiffProjectionTest.kt ffc263803a7169169df5bffb32f1cc98697fcc60c71b958ef242b343430f55fd
```

## Residual Scope

Known path filtering is not universal secret classification. Ordinary-named
copies, hard-link aliases and tools/resources require separate security policy
review. This does not prove full privacy or complete local/remote parity.
Current APK/device execution, actual QR camera/LAN reconnect, API 31/34/35
matrix, scheduler/interactive PTY/full settings parity and signed reproducible
release remain unfinished. No restricted network or device operation bypass.
The note is provisional progress, not an item acceptance receipt.

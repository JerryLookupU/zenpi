# Authorized Host Snapshot Progress

This is provisional Master implementation evidence, not an accepted checklist
item. The complete Stage 2 objective remains active; no checkbox is changed.

## Implemented Frontier

- HostAdapter reads the actual ProjectOwnerPool/Agent, with project Read ACLs,
  session correlation, bounded redacted transcript and busy/error responses.
- Pairing host attachment is explicit through `--workspace` plus `--session`.
  Local `--grant-device-read ID --project ID ...` replaces the Read project ACL;
  malformed device IDs and conflicting device actions are refused.
- Android restores encrypted host metadata and fetches an authenticated
  snapshot. The Compose screen renders primary tabs, phase and transcript;
  Forget is available after restore or refresh, not only invitation acceptance.
- Snapshot parsing preserves unsigned 64-bit sequence exactly and refuses
  owner/cursor mismatches, duplicate projects/turns, unsupported types/roles,
  missing selections and oversized project lists. Journal sequence is not
  advertised as an event replay cursor.

## Behavioral Evidence

`python3 tools/stage2_android_pair_smoke.py --serial emulator-5580 --with-host-state
--receipt Docs/quality/stage2_android2unix/android-host-state-smoke-20261001.json`
uses the production binary and one controlled local HTTP provider request to
create a durable headless transcript. Android API 36 instrumentation verifies:

- Wrong host pin rejected, one-use signed registration and duplicate refused.
- Non-exportable Android Keystore key and encrypted metadata save/load/wipe.
- Before local authorization, workspace list contains no project.
- After local Read grant, the exact provider-produced answer reaches Android.
- Independent authenticated reconnect returns the same bounded snapshot.
- Compose renders that real snapshot; Refresh retains project selection and
  Forget remains reachable. The 320x640 screenshot was visually inspected.
- Local host revocation refuses reconnect using the saved device identity.

The receipt contains requirement/specification identity, production/Gradle
artifact hashes, exact host/seed argv and cwd, environment names and output
hashes. The optional mode requires its own terminal instrumentation marker in
addition to `OK (3 tests)`, so accidentally skipping snapshot assertions cannot
produce a green snapshot receipt. The harness tears down its host and adb
reverse mapping. Tokens are not written into evidence.

Additional gates run: 35 Rust remote tests passed; Android Debug and Release
each passed 10 unit tests; APK/test APK assembled; five Blueprint tooling tests
and strict 36-item Blueprint validation passed. Existing unrelated Rust
warnings remain. No commit or push was made.

## Still Required

This test uses adb reverse, not camera scanning or actual LAN routing. The
standalone host session attachment is not a shared live desktop TUI pool.
Live events, bounded durable replay/ACK, remote writes with owner/revision/
request-ID fencing, workspace/worktree CRUD and concurrency, approvals/diff/
PTY, lifecycle/API 31/34/35, migrations and release gates remain unfinished.
The execution controller has zero admitted workers and is not running a cron;
no claim of active five-worker execution is made. Full blueprint acceptance
requires those product and execution gates rather than this snapshot subset.

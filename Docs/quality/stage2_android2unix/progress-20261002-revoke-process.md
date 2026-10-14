# Revocation Receipt Recovery After Actual Process Death

The preceding development turn made tab-capacity progress; this turn archived
its receipt and advances S2-037/038/053 with actual Android process death at the
post-host-revoke/pre-local-completion boundary. This is no longer only a fresh
ViewModel in the original process.

The production desktop harness independently reads the host registry and
requires durable revoked state plus its self-revoke request ID before stopping
Android. It proves exactly one live PID, issues force-stop and proves the PID
is absent. The test intentionally retains encrypted metadata, the exact UUID
intent and signing alias. A different instrumentation process must load the
existing alias rather than create a replacement, restore the intent offline
and reject background-resume admission. Foreground/network callbacks cannot
clear the intent. Normal host authentication must fail and an unrelated receipt
must return false. The production Check button resolves the original signed
receipt, after which both encrypted state and signing alias must be absent.

`desktop-android-revoke-process-final-20261002.json` passed with 19 initial
instrumentation tests, controlled live-process stop/replay and stop/revoke
phases, one passing new-process receipt test and four independent cache/intent
fixtures. The stopped revocation process was PID 5565; recovery was PID 5627.
An expected `Process crashed` instrumentation result belongs to the harness's
proved force-stop, not an ordinary passing test. Recovery still requires its
own passing test result. The direct private-interface connection used no adb
reverse and desktop listener teardown passed.

An earlier run overlapped a Cargo auth test that rebuilt the binary. Its
end-of-run hash alone was inadequate attribution. The harness now captures
artifact/harness hashes before the run and refuses any changed hash at the end.
The final rerun used unchanged artifacts, with matching current hashes:

- Rust: `139f5df25261f21040d1d1e9f105b3bed70bb4bc859d5464fbf6799c2b0030d8`
- APK: `19a3d127191ef273fd9e2713b327e09b49c0a1567f077b39a3b3c9f58d547717`
- Test APK: `f1b8d9c216c1a91c313a28409b53250e2c60d99fb62285719293148fbb49af60`
- Harness: `41b2747a2bdd5bd251fff22267ebd6ae0c5e9911f3de7fe5dc15daa730206dbb`

Gradle unit-test/debug/test-APK tasks passed (unit tests were up-to-date).
Rust auth suite passed six tests through RTK. Seven harness tests cover route
admission and process-death observation, including ambiguous PID/failed
observation rejection. Five blueprint tests, strict validation, Python
compilation and whitespace checks passed. The final transcript screenshot was
inspected; it records a scrolled projection, not full responsive UI acceptance.
Owned emulator PID 49116/session 62975 exited, with radios enabled and reverse
mappings empty before shutdown. Temporary desktop/provider hosts are absent.

Actual TLS confirmation-drop faults, revocation cancellation/error UI matrix,
cross-process storage locking and partial-wipe recovery remain open. Physical
phone camera/LAN, API 31/34/35, worktree transactions, actual scheduler quota,
prompt/approval/diff/PTY/live-event parity, packaging and execution-controller
acceptance remain required. All 36 blueprint items are unchecked. No commit,
push, worker admission or full-goal completion is claimed.

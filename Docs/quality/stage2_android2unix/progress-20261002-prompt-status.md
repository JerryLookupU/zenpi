# Remote Prompt Execution Status

The preceding goal turn made remote prompt execution progress. This turn
advances S2-019/036/051 by separating admission from running/terminal execution
status. The existing desktop runner writes running and terminal progress through
the same Agent journal writer. A bounded shared projection exposes request IDs
and status while the provider holds the Agent lock. Listings include at most 16
statuses per authorized device/project/session; another device's records are
not exposed. A fresh runtime reconstructs terminal status from the journal and
classifies unfinished durable records as unknown, not resumed work.

Android validates exact status fields, identities, duplicate IDs, recognized
states and capacity. Its UI displays accepted/running/succeeded/failed/cancelled/
unknown separately. Workspace-read status updates survive a subsequent Busy
transcript read. Cache v3 retains terminal display but converts accepted/running
to unknown offline, strips authority and reads legacy v1/v2 records.

## Evidence And Repairs

`desktop-android-prompt-status-verified-20261002.json` passed the production PTY
and API 36 emulator private-interface TLS run without adb reverse. A controlled
three-second provider delay requires Android to observe running while the
Agent lock is held, followed by succeeded. A second production Send receives
a controlled HTTP 500 and must display failed, with no pending admission request.
The harness independently checks two reserved/accepted pairs and progress
running/succeeded/running/failed in the session journal. Five desktop provider
requests comprise those two remote turns plus three ordinary updates.
All 19 initial instrumentation tests, controlled network/revoke process-death
phases, the new-process receipt test and four cache/intent fixtures pass.
Successful/failed status also survives encrypted ViewModel restoration.

Failures found and fixed rather than weakening the gate:

- An early fixture checked failed status before its pending-resolution read
  finished; it now waits for both terminal status and resolved pending state.
- A production ViewModel error branch replaced fresh running status with an
  older projection after transcript Busy; it now preserves the latest state.
- Remote provider failure restored remote text into the local desktop draft,
  causing the next local prompt to concatenate that text. Remote submissions
  no longer enter the local draft-restoration map. The next local provider
  update, WorkManager read and stopped-Android replay pass after the repair.

Earlier failing receipts remain. One callback-timeout run did not produce a
receipt; its tool output is not counted as a pass. The final run's unchanged
artifact/harness hashes match current files:

- Rust: `1ad5dcd63fa8c9d15812fd35b45b7ed50c1c98a7ec6ae84dab5acaafa8319552`
- APK: `0e534153fec1af1f761ef51b26edf3f17e2d4a47f88e35fd386cfe9db2a957f4`
- Test APK: `475afb47b2ea5cdf5d5e798ea6ed33b3e9555a4a97b5e7f7acab6f7a7a2c4b27`
- Harness: `798887e9150714abecacf4987223ed2a9450e877ef83e65f90cc2ebd6b64b063`

Rust workspace/headless suites passed 12 tests, including status visibility with
the actual owner lock held, device isolation and durable terminal reconstruction.
Rust check/build passed with 20 existing warnings, no errors. Android debug and
release suites each passed 35 tests; both APK assemblies passed. Seven harness
tests, five blueprint tests, strict validation, Python compilation, formatting
and whitespace checks passed. Screenshot inspection confirms separate success
and failure rows and a nonoverlapping composer. Owned emulator PID 90538/session
10612 exited with radios enabled and no reverse mappings; temporary hosts are
absent.

## Remaining Full Scope

Cancelled status has implementation but no production cancellation acceptance;
remote cancel/steer/approve remain required. Cancellation before worker start,
worker panic, persisted-progress failure reconciliation and host restart during
execution remain open. The status list is bounded by lexicographic request IDs,
not recent-request ordering; live-map capacity is global 128 and not a complete
multi-session retention policy. Offline status is display evidence, not execution
authority. Peer negotiation, global request ledger, selected-root execution,
scheduler quotas, Git transactions, streaming/ACK/diff/PTY parity, physical
camera/LAN, API 31/34/35, release and controller acceptance remain required.
All 36 blueprint items remain unchecked. No commit, push or completion claimed.

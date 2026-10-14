# Recent Prompt Status Retention

The preceding goal turn made durable cancellation progress. This turn advances
S2-019/020/036 by repairing display retention: random request IDs no longer
determine which execution statuses Android can see. No Blueprint acceptance,
commit or push is claimed; all 36 items remain unchecked.

The live projection is a bounded 128-entry update-ordered queue. An update
replaces that exact device/project/session/request record and moves it to the
newest position. At capacity only the oldest terminal/unknown display record
may be removed; active accepted/running records are retained. Listings put
active work first, followed by newest terminal updates, then fill remaining
slots from reverse journal order with latest-request deduplication. Each list
remains capped at 16. Live records bind session identity, and journal progress
with a foreign project/session binding is excluded. Admission receipts and
their side-effect deduplication are unchanged and are never evicted here.

The new deterministic test writes 160 terminal records with reverse-sorted IDs,
holds the actual Agent execution lock and checks that an oldest running request
still appears first. It checks new low-ID running status after capacity,
terminal update recency, other-device isolation, old-session exclusion and
bounded reverse-journal reconstruction with latest-state deduplication. A
fresh projection classifies durable unfinished work as unknown rather than
resuming it. Initial compilation rejected Value/Option comparisons; they were
corrected to compare optional string references before final passing tests.

Rust workspace/headless suites passed 16 tests; the full 11 remote suites
passed 71. Rust build passed with 20 existing warnings, no errors. Formatting,
whitespace, 12 harness/blueprint unit tests, strict Blueprint validation and
controller validate-only passed. Android production/test code did not change
and its existing APKs were reused, not represented as newly rebuilt artifacts.

`desktop-android-recent-status-20261002.json` passed the production desktop PTY
and API 36 emulator run over the direct private interface without adb reverse.
All 21 initial instrumented tests plus Android process-death resume, revoke
interruption/recovery, WorkManager read and four independent cache fixtures
passed. The screenshot was inspected: newest cancelled is followed by failed
and succeeded, with readable labels and a nonoverlapping composer. On-disk
cancel receipt, terminal journal identity and subsequent desktop update gates
remain passing. This three-request production run proves integration, not the
160-record capacity case; that case is covered by the deterministic Rust test.

Current artifact hashes match the receipt:

- Rust: `7aebb8a50b500e95cbcbbbb78968f0d5dc99bbe8923f9001c37c29032f463971`
- APK: `c7b7ea731b8f254e212342dc2d165c92023fc6a812addfd74571fe4bd7964db8`
- Test APK: `05d42f9a2f4452fc269401c417ca5a4d0e7bf220ad6085631397bd00f2315bd3`
- Harness: `d84b3eae9380d2698bdd80dbff332d6dd196e1fde851541bf7a83eef07e194ee`

Owned emulator PID 82806/session 96325 exited, with radios enabled and no reverse
mappings. Temporary desktop listener closure is checked by the harness.

Remaining scope includes multi-runner retention/admission policy (the desktop
currently admits one global runner), worker-start/panic reconciliation, killed
host recovery, full global request identity, steer/approve, selected-root
execution, streaming/ACK, diff/PTY, quotas, Git crash transactions, physical
camera/LAN peer, supported API matrix, release/migration and full controller
acceptance. Emulator/private-interface and terminal QR decode are not physical
phone scanning/LAN acceptance. The full user goal remains active.

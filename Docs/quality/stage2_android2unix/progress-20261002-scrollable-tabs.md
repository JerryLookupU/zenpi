# Scrollable Android Tab Capacity

S2-034/035 now uses horizontally scrollable primary and secondary Compose tab
rows instead of dividing the screen equally among every tab. Each target has
96..220 dp width and at least 48 dp height. Long labels use two lines with
ellipsis; accessibility retains the full name and routing retains stable IDs.

Two instrumentation tests render 64 projects and 32 secondary tabs at 320 dp
width. They verify selected end-of-list visibility, bounded targets and
scroll-to/click routing to earlier stable IDs. The encrypted-cache round trip
retains scrollable display while removing workspace/subtab mutation authority.
Both screenshots were visually inspected. Clipped neighboring tabs at the
horizontal viewport edge are expected; the selected tab is fully visible.

Android unit tests passed 31 tests in each debug/release suite; debug and test
APK assembly passed. The production private-interface desktop/Android run in
`desktop-android-scrollable-tabs-20261002.json` passed 19 initial tests, one
actual process-death replay test and four independent cache/intent process
fixtures. No adb reverse was used. Artifact and harness hashes match its receipt:

- Rust: `d2d47163cdd462b034426682172faf33b7a6bc1f23b5a0b224f832fbfae50f9f`
- APK: `19a3d127191ef273fd9e2713b327e09b49c0a1567f077b39a3b3c9f58d547717`
- Test APK: `fb297225d324327e4547cd13547c2cfe5b5ba692864b8f0e91bc14d0a32c37a2`
- Harness: `de25b33bc4273a55681776b68b219d1c48ed159a673095baa26fdaebc34d3f3d`
- Online PNG: `7db601d41afd67bddea3eec47f7ecb63906882a6e52acc197b6615e0ce3ae9e0`
- Offline PNG: `b01b4d2911915805cba0425f65e5bf8ad910b193f47a8cec71b3de2480a6c966`

This is capacity/accessibility evidence at one narrow width, not full responsive
UI acceptance, physical camera/LAN evidence, or API 31/34/35 coverage. Worktree
creation/deletion, real scheduler quota, prompt/approval/diff/PTY and live event
parity remain required. All 36 blueprint items remain unchecked.

# Emulator Network And Bidirectional Sync

Previous goal turn: progress. Camera-frame-to-TLS pairing actually passed and
was visually verified. This continuation preserves the dirty checkout and full
Blueprint scope. No item mark changes and no whole-goal acceptance.

## No ADB Reverse

Desktop's fixture constructor accepts an optional explicit advertised private
address, preserving the previous default argv. The camera harness adds an
explicit --emulator-host-network mode: the production host binds an ephemeral
wildcard port and advertises the emulator's host alias 10.0.2.2. The app uses its
unchanged LanSocketRouting matching Wi-Fi route and unchanged pinned TLS. No
route injection, cellular/default-route fallback or production emulator bypass
is introduced. The reverse table is required empty before and after validation.

Actual emulator-camera-api35-host-alias-20261005.json passed, exit zero:
production terminal QR -> real simulator camera Activity/decoder -> confirmation
and Keystore registration -> ungranted projection -> explicit Read grant ->
production desktop transcript. Endpoint 10.0.2.2:55090, adb_reverse_used false,
emulator_host_network_verified true. physical_phone_lan_verified and
direct_lan_verified remain false: the emulator alias is not the Mac's physical
LAN interface and cannot substitute for that acceptance.

## Bidirectional Workflow

Added optional --bidirectional to the same camera fixture, rather than replacing
the broad desktop suite with a narrower pass. The new workflow retains a
bounded local HTTP provider after seeding, submits a desktop-local turn through
the real PTY, requires the Android subscription to display it without Refresh,
stops the Activity while another desktop turn completes, checks unchanged
encrypted cached projection/cursor, then requires lifecycle reauthentication and
an advancing same-session replay. Android finally sends its own prompt through
the real composer and waits for the committed answer and succeeded status;
the desktop must display that same answer.

Independent durable evidence requires exactly three provider requests, six
ordered unique appended user/assistant turns, preserved original journal bytes,
one Android owner/request/fingerprint binding and exact running/succeeded
progress. This reuses the existing physical workflow's content/evidence parser,
not its physical acceptance gates. Its negative verifier fixtures still pass.
Exact ordered phases and the named JUnit start/success pair remain mandatory.

The first actual bidirectional attempt failed before scanner launch: rotation
before Scan QR caused a lifecycle/fixture launch race and the capture monitor
returned null. Its failed receipt and output remain preserved, with all gates
false. Emulator-only UiAutomation rotation now occurs after production capture
launch/decoder-factory verification; the redundant shell rotation command was
removed. This changes test orchestration only, not production scanner behavior.
The next actual run is recorded below when terminal.

## Authority Revision Finding And Actual Sync Pass

The post-launch-rotation run decoded and paired, but timed out waiting for the
desktop update. The diagnostic rerun proved the desktop callback completed and
the local provider answer was durably journaled. This was not a failed provider
or fabricated UI update: upgrading Read to Control changed the authenticated
registry revision during the subscription. DeviceRegistry.authorize rejects
revision mismatch and PairingViewModel fences that authority loss with an
explicit Refresh requirement. Those security checks remain unchanged.

The bidirectional fixture now explicitly grants Read+Control once after the
verified ungranted projection and before the verified project subscription.
Default pairing-only mode still grants Read only. It no longer upgrades
authority halfway through a test that promises no explicit refresh. Grant
changes/revocation recovery still need their own separate acceptance; this
stable-authority workflow does not claim that permissions silently refresh.

emulator-camera-api35-stable-grant-20261005.json actually passed, exit zero.
It used endpoint 10.0.2.2:63217 with an empty reverse table. Exact ordered JUnit
phases prove camera decoding/confirmation, ungranted device, desktop transcript,
desktop-local live update without a Refresh click, unchanged background cache,
foreground replay from event cursor 4 to 6 in the same session, and Android
composer answer on the same desktop. Exactly three provider requests, six
unique appended turns, preserved seed-journal prefix and one matched Android
request binding with running/succeeded progress passed the independent parser.
The final 640x320 screenshot was visually inspected: readable background answer,
Android user message and committed desktop answer, no overlapping text.

Attributed app hash:
2267dfddf3d2c2e00c3876b17cb55a5041fc5478cc03f0145e4aa1f6409986a5;
test APK ae6854884a36b0bbbdab6f3c2008cae9ad847047938d96aeb482544055bab99b;
inventory 8cd4e790de4e00fc3cb2d68dde2200a57df5b601dbe8b767b34147a2186be8dc;
build receipt bc90d8943708ca690f8d00c535ae0f83e3d4efc0ef474f510bae787d7ea9e923.
Failed intermediate receipts are unchanged. The emulator/desktop/provider are
terminal and adb reports no connected emulator after cleanup. An API 34 run of
the same current artifacts is being observed separately, not inferred passed.

## API 34 Association Preparation

The API 34 stable-grant attempt failed before instrumentation. A bounded
diagnostic rerun identified the exact prerequisite failure: its Google APIs
image rejects shell UID 2000's connect-network command with SecurityException.
No API 34 pairing/sync pass is inferred from either failed run. The harness now
records the prerequisite stage and bounded stdout/stderr for Wi-Fi, APK install
and camera-permission setup; it still redacts all invitation output.

Wi-Fi preparation first verifies ro.kernel.qemu == 1 and uses the normal shell
command. Only the known UID-2000 permission refusal permits a second association
attempt through that owned debug emulator's su 0. It does not root the ADB
daemon, change app permissions beyond the existing camera grant, inject routes
or bypass the production binder. Non-emulators and unrelated command failures
are refused. Three unit tests cover those boundaries; camera tool tests six
pass. The actual API 34 debug-association run follows below after observation.

API 34 debug-association actually passed, exit zero, using the same attributed
APKs. It completed all 11 camera/network/sync/replay phases, three provider
requests and six durable appended turns; no reverse mapping. Its screenshot was
visually inspected and the retained JUnit identity, phase order, request binding,
same-session 4-to-6 replay, artifact hashes and current helper hashes were all
reverified. The receipt explicitly records owned_emulator_debug_su for Wi-Fi
association, not an unqualified production-shell claim. A separate API 31 run
is being observed. This is not the full API lifecycle/Doze acceptance matrix.

API 31 debug-association also actually passed, exit zero. It used ordinary shell
Wi-Fi association (no debug su) and no reverse mapping. Its named test, all 11
phases, three provider requests, six appended durable turns, exact request
binding and same-session cursor 4-to-6 replay passed retained verification.
The screenshot was visually inspected: background desktop answer and Android
prompt/answer readable in the API 31 landscape system-navigation layout.
API 35 is being rerun with the current diagnostic/association helper hashes to
close this camera/network workflow matrix without relabeling its earlier pass.

Remaining: physical phone/manual camera/SAS and actual host LAN interface,
broader desktop/workspace/Doze regression failures, full Rust/Android unit and
release gates, and per-item Master receipts. The new workflow is additional
evidence, not replacement for those gates. Latest Python tool suite: 182 passed.
Strict Blueprint structure and current APK --verify-only pass; all 36 items
remain unfinished. No app or Rust production code was changed in this turn.

Final API 35 network-matrix run actually passed, exit zero, and its screenshot
was visually inspected. Retained verification for API 31 debug-association,
API 34 debug-association and API 35 network-matrix confirms the same app/test
APKs, Rust binary and current helper hashes across all three, exact named JUnit
success and 11 ordered phases each, provider/durable-turn/request identity,
same-session replay, and screenshot/instrumentation hashes. All three retain
physical/direct LAN and whole-item flags false. API 31/35 association used the
ordinary shell; API 34 used its explicitly recorded debug-emulator su command.
No unrelated emulator was started. Final adb inventory is empty; all owned
camera/desktop/provider runs are terminal and their fixture cleanup completed.

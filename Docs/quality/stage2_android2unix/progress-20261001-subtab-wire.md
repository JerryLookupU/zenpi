# Subtab Wire And Android Controls

## Change

`subtab_list` is an explicit read operation in Rust and Kotlin. It returns
project/session identity, host and subtab revisions, ordered stable tab IDs,
redacted names, kinds and concurrency. Filesystem roots never cross this
projection. Only select/rename/reorder capability is currently exposed; Git
worktree and concurrency mutation capabilities remain false.

Select, rename and move commands route through the same production owner pool
and desktop mailbox. They validate exact arguments, current device Read/Control
ACL, project/session owner, host revision and per-project subtab revision.
Current ACL remains locked through commit. Running, queued/preparing discussion
or arch owners and desktop-admitted work refuse changes. The host atomically
commits state with an exact-content device/request receipt in the existing
bounded ledger. Exact retry after restart returns the original result; changed
content under the same ID fails. No-op commands still get one durable revision
and receipt. Desktop reconciles the new stable-ID selection/order/name into its
view, and subsequent local synchronization does not undo it.

Android's strict parser rejects mismatched identities/revisions, invalid schema,
selection, IDs and concurrency, unknown fields, root leaks, unsupported
capabilities and unexpected authority. Compose displays SecondaryTabRow and
offers selection, confirmed rename and adjacent reorder. ViewModel saves each
immutable pending command before send, including both revisions, then reuses
the unknown-outcome explicit retry path. Offline cache drops subtab controls.

## Evidence

Rust control tests cover Read-only/foreign refusal, revision conflicts, exact
retry, changed-content conflict, busy owner, rename/select/move persistence,
restart retry, path-free listing, revocation and real desktop reconciliation.
Eight related suites passed with 81 tests and one existing ignored test.
Debug and release Kotlin suites each pass 25 tests; debug/app-test APK builds
pass. Compose fixture tests cover stable-ID selection/reorder/confirmed rename
and disabled controls during unknown outcomes.

`desktop-android-subtab-wire-20261001.json` passed actual Android TLS rename,
same-ID retry, and return to the original name against the production desktop.
`desktop-android-subtab-ui-20261001.json` adds the two Compose tests, with twelve
initial instrumentation tests plus network-process and two cache-process tests.
The final UI harness adds production ViewModel/button rename and inspection of
the desktop's owner checkpoint for the same selected tab. These runs use the
private host interface directly without adb reverse. The existing pairing,
workspace controls, transcript replay, network lifecycle, WorkManager,
process-death/pending recovery, revocation and teardown gates still apply.
`desktop-android-subtab-ui-final-20261001.json` passed all twelve initial tests
and all process/cache stages; both production-button and desktop-checkpoint
flags are true. Expanded negative tests also verify arch lock refusal, foreign
owner/session, index arguments and string revisions fail without mutation.
Five additional remote event/frame/pair/listener/TUI suites passed 26 tests.
Formatting, strict blueprint, Python checks and tooling fixture tests pass.
The final current-binary rerun is
`desktop-android-subtab-ui-current-20261001.json`; it passes with Rust binary
SHA-256 `a93852587b766c985340f02cdac10b66bc35422ba626dadc95914a0a716f6a53`
and confirms the production button and desktop subtab checkpoint gates.
Owned emulator/temporary host cleanup completed; reverse table was empty and
Wi-Fi/mobile data settings were restored to enabled.

## Limits

Actual TLS rename is proven; multi-tab selection/order UI is fixture-tested and
Rust same-owner-tested, not yet an end-to-end production Android multi-worktree
scenario. Git creation/removal, dirty-tree handling, safe physical moves,
concurrency admission/stepper, all operation crash boundaries and headless
command parity remain unfinished. Offline cache currently omits secondary-tab
display as well as authority. Physical camera/LAN, full session controls,
push/ACK/backpressure, Doze/system scheduling and API 31/34/35 remain open.
No item acceptance, commit, push or full completion is claimed; all 36 blueprint
items remain unchecked.

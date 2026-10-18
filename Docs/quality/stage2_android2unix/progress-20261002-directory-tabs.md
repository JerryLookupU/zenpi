# Existing Directory Tab Lifecycle

## Progress

The previous turn was implementation and verified progress: remote stable-ID
select/rename/reorder and production Android rename already worked. This turn
adds an existing-directory tab lifecycle and a real multi-tab Android scenario.
It does not replace the full S2-017 Git/isolated-workspace deliverable.

Host `subtab_add` admits only kind InPlace and an existing canonical directory
strictly under the authorized project. Exact argument keys, name/path limits,
capacity, duplicate name/root, parent traversal, absent directory, project root
and symlink escape checks run before publication. A separate
`directory_tab_control` capability distinguishes this from Git worktree control.
`subtab_close` only removes InPlace records, refuses Main/Worktree kinds, and
retains a valid stable-ID selection. Neither action creates/deletes directories,
executes Git, provisions a session or runs a provider in that directory.

The same owner-thread mailbox, current ACL lock, discussion/arch/admitted busy
gates, host/subtab revisions, exact request identity, atomic checkpoint and
durable receipt mechanisms cover both actions. Old add/close retries after
restart return the original receipt without resurrecting a closed tab or
repeating any side effect. Android supports strict pending-envelope restore,
capability validation, add/confirmed-close dialogs and the existing unknown
outcome recovery path.

## Evidence

The Rust tests verify directory containment, parent traversal, symlink escape,
absent/root/duplicate directory refusal, main/worktree close refusal, selection
retention, add/close exact retries after restart and surviving directory files.
Eight related suites passed 83 tests with one existing ignored test. Five
additional remote suites passed 26 tests. Kotlin Debug/Release suites each
passed 26 tests and debug/app-test packaging passed.

`desktop-android-directory-tabs-20261001.json` passes twelve initial Android
instrumentation tests and the existing network-process/two cache-process stages.
The production Compose scenario adds directory-one and directory-two, reorders
by stable ID, selects directory-one, closes both, and restores the main ID.
The host checkpoint must contain only main afterward; sentinel files must
survive. It uses direct private-interface TLS without adb reverse.

The stronger final fixture also checks the desktop checkpoint's intermediate
order `[ui-main, directory-two, directory-one]` and selected directory-two ID
before the close sequence. The final receipt is
`desktop-android-directory-tabs-final-20261002.json`.
That stronger run proved both directory checkpoint gates but failed later when
the invalid-workspace negative test received an ambiguous transport outcome;
the preserved pending command then correctly made the WorkManager test skip.
The failing receipt remains intact. The fixture now explicitly resolves only
the exact original pending request, with a bounded retry count, before asserting
the confirmed invalid-request result. It does not manufacture a replacement ID
or relax the production unknown-outcome behavior. The current rerun is recorded
separately in `desktop-android-directory-tabs-current-20261002.json`.
That current run passed all twelve initial tests, post-death restoration and
both independent cache-process tests. Both intermediate directory order and
final directory-close checkpoint gates are true. File-retention checks passed;
the receipt identifies the current Rust binary and APK hashes. Strict blueprint,
formatting, Python compilation, four harness tests, five tooling tests and
diff whitespace checks also passed. Temporary hosts/reverse mappings and the
owned emulator were cleaned up; Wi-Fi/mobile data remained enabled.

## Next Required Work

Scheduler inspection found `TuiState::worker_concurrency` is consumed by view
quota/planned-slot helpers, while DAG worker spawning does not yet use the
per-tab quota. Remote set_concurrency and the Android stepper stay disabled;
enabling a number-only control would not meet S2-018. Effective admission,
resource limits, live/queued/busy changes and no-N+1 tests remain required.
Git worktree add/remove, safe rollback/crash recovery, full owner/runtime cwd
parity and provisioning of isolated workspaces also remain required.

The existing-subdirectory implementation is incremental coverage, not an
alternative success definition. Full camera/physical LAN, session controls,
push/ACK/backpressure, API lifecycle and release gates remain open. All 36
authoritative checklist items remain unchecked; no acceptance, commit, push or
goal completion is claimed.

# Shared Desktop Owner Progress

Provisional Master implementation, not full Blueprint acceptance. The original
Android/LAN/QR/desktop synchronization objective remains active.

## Change

The opt-in `host pair --desktop --workspace PATH --session PATH` starts the
production TUI and attaches the listener to its existing ProjectOwnerPool via
a bounded owner-thread read mailbox. No second Agent, separate journal reader,
or replicated workspace is created. Reads expire in two seconds, the queue and
per-pass drain are capped at eight, and only read operations are admitted.
Local provider operations continue through the existing desktop runtime.

QR refresh and invitation cancellation remain visible. Esc cancels the invite
and returns to the desktop without invalidating existing paired devices. The
listener stops with the desktop. A real PTY test exposed that NO_COLOR=1 made
the half-block QR blank; QR rendering now temporarily enables its required
black/white colors and restores the prior environment-derived setting.

## Evidence

`tools/stage2_desktop_android_smoke.py` starts an actual production desktop in a
PTY. macOS Vision decodes black/white pixels reconstructed from its output,
rather than generating a fresh invite from metadata. It verifies the host pin
against the persisted certificate. Android uses this decoded invite over adb
reverse, completes signed one-use pairing, starts with no project access, then
receives an explicit local Read grant.

Android reads the actual seed provider response, reconnects, then signals the
harness to return from QR to desktop and submit a second prompt via keyboard.
The same running desktop generates one real local HTTP provider request.
Android must observe that new assistant turn, more turns, the same session ID,
and increased journal sequence. Compose renders the data. Host revocation must
return explicit unauthorized. Desktop quit must close the actual TCP port.

The harness continuously drains its PTY so terminal output backpressure cannot
artificially stop the owner thread. Initial failures from incomplete PTY frames
and an undrained PTY were investigated and fixed rather than weakening tests.
No invitation or raw terminal QR containing its token is stored in receipts.

Receipt: `desktop-android-shared-owner-20261001.json`; screenshot uses the same
stem with `.png`. The receipt includes exact desktop/seed argv/cwd, requirement
identity, artifact hashes, instrumentation hashes, environment names, timestamps
and truthful capability scope.

Remote tests passed (35); headless protocol regression passed (45); project
owner/headless/TUI workspace regression passed (57 with one intentional child
entry ignored). Android Debug/Release unit tests remain 10 per variant; API 36
instrumentation passed three tests. Strict Blueprint and five tooling tests
passed. Existing unrelated Rust warnings are unchanged. Validation harnesses
now share a per-device flock lease, preventing two instrumentation runs from
overwriting each other's APK/process state on the same emulator.

## Remaining Work

The current proof is shared-owner foreground snapshot reads. It is not camera
scanning, actual LAN routing, live event streaming/replay, or remote mutation.
Write owner/revision/request-ID fencing, workspace/worktree CRUD, approvals,
diff/PTY, lifecycle/API matrix and release acceptance remain required. No
Blueprint checkbox is accepted, no commit/push is made, and no execution workers
or cron are represented as running.

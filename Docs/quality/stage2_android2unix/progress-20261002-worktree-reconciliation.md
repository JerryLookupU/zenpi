# Worktree Intent Reconciliation Progress

Partial S2-017/S2-019 recovery integration, not whole-item Master acceptance.
The goal and all 36 Blueprint rows remain unfinished. No commit/push, nested
agents or worker launch. Controller status had no active claims, transports,
requests or handoffs; validate-only launched nothing. Existing dirty state was
preserved.

## Recovery Contract

New pre-Git null receipts retain a bounded typed intent: original envelope,
prior/candidate tab states, derived path and optional fresh branch. This is
private host checkpoint data, never the Android subtab projection. Restore
validates exact request fingerprint, project-root identity, stable tab IDs,
command keys/kind/name/target and revision transition. Settled receipts discard
the intent. Existing successful receipts remain compatible; legacy null receipts
remain unknown and readable, but cannot be reconstructed automatically.

Local exclusive host-pair actions now list and reconcile these intents:

```sh
zenpi host pair --workspace /absolute/project --session /absolute/session.jsonl --list-worktree-intents
zenpi host pair --workspace /absolute/project --session /absolute/session.jsonl --reconcile-worktree REQUEST_ID --device DEVICE_ID
```

Stop the desktop host first. The CLI requires an existing matching session and
workspace, rejects mixed pairing/device actions and malformed IDs, and runs
before identity/listener creation. No provider request or permission grant is
made. Existing ordinary owner/resource preparation still occurs; this is not
a separate raw-checkpoint editor.

Reconciliation only accepts an already completed observed Git effect. Create
requires the canonical managed registered same-repository worktree on the exact
intended branch; dirty user bytes are retained. Remove requires no leaf path or
Git registration and validates the canonical non-symlink managed container even
when the leaf is absent. Original project/session/workspace/subtab state must
still agree; busy discussion/arch owners and checkpoint CAS conflicts refuse.
It never reruns add/remove, force-deletes files/branches or overwrites newer state.

Successful repair publishes the original candidate and receipt atomically,
increments both revisions once and preserves the original request's owner value
in its result. This is essential after restart, when the new live owner nonce
differs but Android still has the original encrypted envelope. Explicit same-ID
retry returns that result under current device ACL, without another Git action.

## Behavioral Evidence

The new remote_workspace owner test uses real Git/post-checkout to create a
worktree while changing checkpoint formatting after the reservation, forcing
post-Git CAS failure. It restarts, invokes the actual production binary's list
and reconcile CLI using a configured HTTP provider URL (no provider request is
needed), verifies the original owner/revisions/result, preserves dirty user
bytes, and checks exact retry/idempotent repair. Other cases reject branch
mismatch, busy owner, absent session without creating it, reappeared removal
target, replaced managed-directory symlink, newer tab revision and corrupted
intent path. Removal repair uses an explicit reconstructed post-Git/pre-commit
checkpoint fixture; it is not claimed to be a killed real remove process.

The full direct-interface desktop/API-36 Android harness passed twice with the
new intent implementation. The later receipt is
`desktop-android-worktree-reconcile-final-20261002.json`: 24 initial instrumented
tests, 17 controlled HTTP provider calls, distinct-root worktree read, dirty
removal refusal, clean removal/branch retention, actual Android process death,
cursor/pending-write/revocation recovery and four independent cache/revoke
process cases. Terminal QR pixel decode and direct 192.168.50.14 TLS passed,
without adb reverse. Physical camera/phone LAN remain false/unverified.
This harness proves ordinary workflow regression, not Android-driven unknown
Git outcome repair; the recovery command itself has separate Rust/CLI evidence.

Passed harness hashes:

- Rust: `023c77d1719ec9103e2ea8018b25a70cf722f2ba16c419a7f939f71eaaf4915c`
- App: `08a888a3dd46991340bd51d4cceb4cb0f6851d78684652c8dc1bfb7a6dcd89d0`
- Test APK: `871288a282f7c4a7a18b1ac36e4af916bad1aa09bb8f08eba9ba9e68d5cf33b0`
- Harness: `b548bb604fdce3021e00e8b79928045f078fcfa421b7b565044e3f07abdc076b`

The later existing-session CLI guard was added after this immutable harness
artifact. It does not change the listener/worktree path and is separately
covered by the final Rust/production CLI regression. Receipt hashes are not
silently relabeled as later artifacts. Android/APK sources were unchanged this
turn. The owned emulator was shut down and its execution handle exited 0.

Six focused Rust suites passed 45 tests; Python evidence tests passed 14 and
Blueprint tests five. Build, fmt/diff checks and strict Blueprint/controller
validation passed, retaining existing vendor/dead-code warnings. The broader
final 13 core/tool/input/extension/remote suites passed 207 tests with five
existing/helper ignored tests, including the production CLI existing-session
guard and managed-container symlink refusal.

## Remaining Scope

Divergent-state repair, proving/settling no-effect failures, legacy intent
reconstruction, Android-assisted reconciliation, full crash-window coverage,
independent worktree session owners/quota, complete headless parity, PTY/bulk,
live event push/ACK, supported API lifecycle matrix, physical scanning/LAN and
release security gates remain required. No checklist completion is inferred
from these focused recovery and regression checks.

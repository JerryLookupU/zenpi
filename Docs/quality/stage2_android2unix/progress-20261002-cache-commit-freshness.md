# Android Cache Commit Freshness

Provisional S2-033/S2-037/S2-038/S2-056 progress. Previous goal turn verified
framed Android response consumption. This turn fixes a persistence/publication
gap in the existing cache path. Full QR/LAN/native Android/Rust-desktop goal
remains active, all 36 Blueprint rows `[ ]`. No agents, commit or push.

## Changes

PairedHostStore.saveProjection previously silently returned when an older
transcript cursor arrived. PairingViewModel could then treat the call as a
successful save and publish that discarded result. ProjectionCommitPolicy now
explicitly throws StaleProjectionException before encryption/write for either
journal or transcript regression in the same selected project/session. The
policy also covers finishWorkspace with a supplied projection and commitResume.
Refused finishWorkspace retains both pending intent and prior display. Background
resume still checks exact encrypted-record digest first; this policy supplements
CAS rather than replacing it. Different project/session counters are unrelated.

PairingViewModel handles stale read refusal by loading the current durable state
under generation/storage fencing and retaining unavailable status until another
host read. Rejected projection is not published or used for next-read ACK.
The workspace-confirmation callback rechecks generation inside the storage lock
before clearing a pending intent, closing a cancellation-during-lock-wait gap.

Journal advancement alone does not permit transcript cursor regression. An
initial draft considered shorter tree-branch cursors, but current Rust
events::cursor counts all durable turns (not selected branch length). Final
policy preserves that invariant. A shorter visible snapshot with unchanged
transport cursor and newer journal is accepted. Equal-cursor metadata/transient
changes remain valid; this is not complete workspace revision ordering for
concurrent equal-journal views.

## Verification

Final source verification observed 2026-10-02T00:26:01Z; dirty worktree preserved.

- `zsh tools/stage2_android_source_check.sh
  /private/tmp/zenpi-android-cache-policy-final-source-20261002`: main/unit/
  instrumented Kotlin source compiles; **85 JVM tests pass across 20 classes**.
  Six ProjectionCommitPolicy tests cover older journal with equal/higher event
  cursor, older event cursor despite newer journal, valid display replacement,
  unrelated scopes, u64 bounds/malformed numbers, transient same-cursor views,
  and production ResponseReader callback refusal without display publication.
  Existing Kotlin fixture/icon/safe-call warnings remain.
- ResumeStoreTest source updates old silent-return expectation to refusal. A
  new instrumented case checks unchanged encrypted bytes on foreground and
  background stale refusal, retained pending intent on failed completion,
  accepted shorter snapshot with stable cursor and refused later old journal.
  **Compiled only, not executed**; no Keystore/AtomicFile runtime acceptance
  claimed. Existing historical device receipts are unchanged.
- `env ZENPI_HOME=/private/tmp/zenpi-cache-policy-20261002 cargo test --offline
  --locked --test remote_events --test stage1_session_tree --test
  remote_subscription --test remote_subscription_tls`: **36 pass** (4/13/10/9).
  One tree helper ignored in parent is executed by independent-process tests.
  Rust source unchanged. Existing 19 library/one vendor warnings remain.
- `python3 -m unittest discover -s tools -p 'test*stage2*.py'`: **44 pass**.
  Strict Blueprint validation, cargo fmt check and git diff check pass.
  All observed test handles completed; no new APK/device/TCP/LAN execution.

Final SHA-256 (source paths are under android/app/src):

```text
main/java/zenpi/state/ProjectionCommitPolicy.kt b15204e691d91e3a4286ac30a1ce85b24ca0c2bcb8e2b026eb3c7a9bf702b611
main/java/zenpi/security/PairedHostStore.kt e95faa2bf625eb737381b8ad1442b66133f00e2fea4dc1f7ed365f57c8c24781
main/java/zenpi/pairing/PairingViewModel.kt 90f94e63534bafbfaf5f9381e9d14689085197dc9e1ffec666abc0425834160d
test/java/zenpi/ProjectionCommitPolicyTest.kt 9101d6b5c26a76f6affeb9b49d6d81ad110408007336afae9b5610bbbf7a6b9a
androidTest/java/zenpi/ResumeStoreTest.kt 3cefbfc1556c7e56d2e371a0eefa48d13366ec4a488f578aa3c53e177dcc4df9
a895f14efc764f645becf059d72769947ee20abab83dd3d76e7c9a40e1f8b191 main.jar
7d30d19a9b968c19e11432d88b24396fecf0e0b7a010bfe09299eab256e3d0da unit.jar
ce5b4ebf88a14154038ff6bc57eacef75488b7cb2cc8ff8590f39659395a038c instrumented.jar
```

Not Master acceptance. Fresh APK/device subscription/reset/cache races,
physical camera QR/same-WiFi LAN, socket shutdown/deadline, full stream-delta
backpressure, executable independent-worktree scheduling 1..64, interactive
PTY/bulk, API31/34/35 lifecycle/Keystore, migration/uninstall and signed release
remain required. Cache policy is progress toward sync, not a substitute goal.

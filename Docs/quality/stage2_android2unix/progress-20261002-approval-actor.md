# Remote Approval Actor Audit

The preceding request-binding increment made concrete progress and left an
attributed passing production receipt. The full Stage 2 objective remains
active; this increment does not mark any complete Blueprint item accepted.
No commit or push was made and existing user changes were preserved.

## Implementation

The authenticated remote approval adapter constructs device ID, envelope
request ID/fingerprint, project/session and complete original request digest.
The coordinator validates this bounded attribution, then claims decision and
actor in the same critical section. No remote remember/message is admitted.
The actor is not a field in ApprovalResponse or provider tool arguments.

AcceptedApproval retains the actor until the existing worker journal writer
persists approval_resolved before entering a permitted side effect. Both
built-in tool and user-shell approval persistence paths support the optional
remote object. Local responses and host cancellation have no remote actor;
losing responses cannot overwrite the winning actor. Checkpoint admission
remains distinct from successful tool execution or durable worker completion.

Persistence failure previously restored a request only when a local renderer
had revealed it, and only while the decision remained in the wait map. A
remote-only response may already have left request_response's wait, while its
AcceptedApproval is still awaiting audit. Retraction now uses that retained
request too, removes the failed actor/response and restores pending state.
This does not automatically restart a worker after a failed turn or resolve
the remote accepted admission receipt into a terminal tool result.

## Verification

The real Agent write_file Allow/Deny test now asserts exact actor content in
the durable approval audit and rejects malformed attribution before claiming
the request. A barrier-driven local Deny/remote Allow test runs eight races:
exactly one answer succeeds, exactly one audit record exists, actor belongs
only to the remote winner and file existence matches the winner. A separate
audit-failure test answers without local reveal, lets the wait return, then
checks retraction/pending restoration and that a later local answer has no
stale remote attribution. These are real coordinator/Agent tests, not proof
of the complete distributed race, killed-host or storage-fault matrix.

Final Rust remote/approval_owner suites passed 89 tests in 12 suites;
25 approval-filtered library tests passed. Rust build succeeded with the
existing 20 warnings. Fourteen harness/Blueprint Python tests, compilation,
Rust formatting, whitespace, strict Blueprint validation and controller
validate-only passed. Validate-only launched no workers. Android sources and
APKs were unchanged, so prior JVM build results are not presented as rerun.

The production harness now requires exact agreement between each remote
actor audit and its checkpoint receipt, and the original target request ID
on the Allow-only consumed record. Negative tests reject missing/foreign
actors, mismatched digests and consumed targets. The full production harness
uses terminal QR pixel decoding and private-interface pinned TLS without
adb reverse; no Rust build/test is run concurrently with that validation.

The first full harness run retained a failing receipt
desktop-android-approval-actor-20261002.json. Its live prompt/Allow/Deny/preview
path completed, but the later directory-tab workflow exhausted the explicit
same-request retries on an unknown outcome; the dependent WorkManager test
then skipped the unresolved pending write. This repeats the known reliability
issue, not an attributed passing full gate. An unchanged-artifact rerun is
separate evidence and does not erase the failure.

The unchanged-artifact rerun passed, retaining
desktop-android-approval-actor-retry-20261002.json with passed=true and
remote_approval_actor_audit_verified=true. All 23 initial instrumentation tests
and subsequent actual Android process-death replay/pending-write/revoke receipt,
WorkManager and independent cache/intent gates passed. Live preview/Allow/Deny,
prompt checkpoint identity correlation and desktop listener closure are true.
It used direct 192.168.50.14 pinned TLS without adb reverse and recorded ten
desktop provider calls; physical camera/LAN peer flags remain false.

Attributed artifact hashes:

- Rust: `23fe1bcc3cda9ed05ad3e3708f87aae7b036f4e2de77c7651ef9aebb3909af75`
- App APK: `d18249e1b70c3c673c314d1562f7a91b0ea0378b4002ae33e451e5e9662b63d4`
- Test APK: `9dcf1eb485875b3ab3153c81dcd43ab44e7204dc468225cc3532ff78a51aa5ca`
- Harness: `a91702c70d13a8db967b9eaeab27dbeb52c655d7bc25378398c3139a454aac8a`

## Remaining Scope

Historical request migration, approval timeout/cancel, distributed races,
killed-host/storage failure acceptance and uncertain terminal tool recovery
remain. The intermittent directory-operation unknown outcome is not claimed
fixed by successful same-envelope retry. Physical phone camera/LAN peer,
API matrix, selected-root execution, steering, live event push/ACK, PTY,
scheduler quotas, Git crash transactions and release/controller acceptance
remain unfinished. All 36 authoritative items stay unchecked.

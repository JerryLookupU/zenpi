# Separate Write Confirmation From Read Refresh

The previous actor-audit increment changed authoritative code and produced a
passing end-to-end receipt. This increment addresses a real Android control
boundary. The full Stage 2 objective is active, all 36 Blueprint items remain
unfinished, and no commit or push was made.

## Implementation

HostClient previously coupled successful mutation/admission receipt handling
with subsequent workspace/subtab/transcript reads. A read failure after a
valid write response kept the encrypted write envelope and displayed an
unknown write, unnecessarily offering explicit retransmission.

WorkspaceReceiptCodec now validates every supported write response before
confirmation: request ID/channel/state, exact result fields, owner, numeric
workspace/subtab revision increment, selected project semantics and bounded
identities; prompt/cancel/approve require exact admission identities and
job/target/digest/choice. Malformed, absent or unknown receipts never confirm.
Existing repeated admission checks were consolidated into this codec.

After validation, a suspending callback lets PairingViewModel persistently
clear its exact pending envelope before the first follow-up read. Its current
projection is converted to the existing read-only cache representation until
fresh authority arrives. If subsequent read/cache projection fails after that
confirmation, only read-only foreground refresh is restarted. Failed local
confirmation persistence retains pending state for explicit exact retry.
Generation cancellation still prevents stale UI publication; cancellation or
process death before local confirmation preserves the pending envelope.

This fixes conflation of confirmed write and failed read, not every underlying
transport/mailbox/lifecycle failure. Before a matching write receipt, ambiguous
outcomes still require explicit same-ID retry. No automatic write resend or
new operation authority was added.

## Verification

Two JVM cases exercise workspace/subtab and prompt/approval receipt validation,
including malformed shape, foreign ID/owner/target, wrong state, invalid job
and wrong/string revisions. Debug and Release JVM suites each passed 42 tests.
Production and instrumentation APK builds passed. Fourteen Python harness/
Blueprint tests, Python compilation, strict Blueprint validation, controller
validate-only and whitespace checks passed. Rust code/binary was unchanged;
prior Rust results are not represented as rerun.

The existing production Compose workspace test additionally submits a real
TLS subtab mutation through HostClient, durably clears its encrypted pending
envelope in the confirmation callback, injects a failure before follow-up
reads and checks pending state stays cleared. A fresh read confirms the host
mutation. This directly tests HostClient/store callback ordering; it is not
an injected PairingViewModel failure or full local-storage-fault acceptance.
Ordinary production ViewModel buttons run in the same full harness.

desktop-android-confirmed-write-20261002.json passed all 23 initial instrumented
tests and subsequent actual process-death network/pending-write/revoke receipt,
WorkManager and independent cache/intent fixtures. It requires the injected
post-receipt read-failure phase, prompt-binding correlation, remote approval
actor audit, live diff/Allow/Deny and listener closure. Ten desktop provider
requests were recorded. Independent terminal QR decoding and direct private
interface pinned TLS used 192.168.50.14 with no adb reverse; physical camera
and physical LAN peer acceptance remain false.

Attributed hashes:

- Rust: `23fe1bcc3cda9ed05ad3e3708f87aae7b036f4e2de77c7651ef9aebb3909af75`
- App APK: `9e6a73a187b023b771963849a6128cedcd0259ac07343da2f4bada593d451b58`
- Test APK: `5306657d217bb0028191505257fbd137eea8d6f49b2e68bef841408b85899c80`
- Harness: `118fe79603c1ad13367f0e4a23c932fe90dce9a4164765fac72efcbd862e0eed`

The unchanged-artifact repeated run failed the known directory-tab selection
unknown outcome and dependent background test; its failing repeat receipt is
retained. This confirms the receipt/read separation is not the full underlying
reliability fix. Transient failure-code diagnostics now distinguish bounded
receipt/transport categories without recording raw exception text, paths,
credentials or request content.

The diagnostic build's full run also passed and retained
desktop-android-confirmed-write-diagnostic-20261002.json. All 23 initial tests
and subsequent recovery gates passed with confirmed_write_read_failure_verified
true. Final APK hashes are
`505361d1ba69ff3c1ad8bc02895948dbdaeb4c0e0093fb9f8041683a8d855b2b`
(app) and
`b6248fe90ae1abddef786e77a5955a6cdc358409d7fd01212f35b012a954225d`
(test); Rust and harness hashes above are unchanged. The passing diagnostic
run did not reproduce a failure code, so attribution of the intermittent
pre-receipt directory selection failure remains unproven.

## Remaining Scope

Repeated reliability stress and detailed pre-receipt failure attribution remain.
Historical request migration, killed-host/storage faults, broader approval
timeout/cancel/distributed races, physical phone camera/LAN, supported API
matrix, selected-root execution, steering, streaming/ACK, PTY, scheduler quotas,
Git crash transactions, release and controller acceptance remain unfinished.

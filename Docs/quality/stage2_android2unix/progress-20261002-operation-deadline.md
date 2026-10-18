# Android Operation Deadline Versus Cancellation

Provisional S2-031/S2-032/S2-036/S2-038/S2-053 progress observed
2026-10-02T06:18:10Z. Previous goal turn: progress (same-identity QR re-pair
display/cursor preservation and tests). HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b; existing dirty changes preserved.
All 36 authoritative rows remain [ ]; full Android/QR/LAN/Rust desktop goal
active. No worker admission, subagents, commit, push, cleanup or acceptance.

## Implementation

PairingClient and HostClient previously wrapped their absolute operation bound
in withTimeout. Its TimeoutCancellationException inherits CancellationException;
PairingViewModel deliberately rethrows that type for user/lifecycle cancellation.
An operation's own deadline could therefore leave pairing Connecting, a read
loading, or a write/revoke unresolved without publishing its normal error state.

Both clients now use OperationDeadline, a small withTimeoutOrNull boundary
which converts only its own elapsed deadline to SocketTimeoutException with a
fixed diagnostic. A non-null completion wrapper preserves legitimate nullable
results. Outer timeout, explicitly thrown cancellation and lifecycle/user
cancellation remain cancellation. Original errors and normal values are
unchanged. The existing independent 15-second socket-close watchdog and owned
SocketLease cleanup are preserved; this does not attempt to kill arbitrary
blocking JVM code or weaken TLS pinning/proof validation.

The converted timeout reaches the existing ViewModel error handlers: transient
read loading ends, foreground attach can retry with bounded backoff, and
unconfirmed commands/revocation retain their durable identity and unknown result.
No write is automatically resent; only explicit exact-envelope retry is allowed.
Late blocking IO completion cannot escape the expired operation as confirmed.
Protocol and Chinese guide document that a timeout is not rollback and cannot
prove a one-use pairing invite remained unconsumed.

## Verification

Cwd /Users/mac/Github/zenpi. All final commands exit 0:

- zsh tools/stage2_android_source_check.sh
  /private/tmp/zenpi-operation-deadline-android-final: **189 JVM tests**;
  main/unit/instrumented sources compile with existing warnings. Seven new
  tests cover own deadline as non-cancellation, owned unconnected socket close,
  parent deadline, nested deadline, explicit cancellation without failure
  publication/retry, nullable/normal/error propagation, late blocked IO release,
  and a real WorkspaceCommand whose exact saved envelope is the only retry
  admitted by WorkspaceDispatchPolicy after timeout. The blocked fixture uses
  a latch and socket-close signal, not a bound network socket. It waits and
  joins its helper coroutine; no detached work is left behind.
- env ZENPI_HOME=/private/tmp/zenpi-operation-deadline rtk cargo test --offline
  --locked --test remote_auth --test remote_protocol --test remote_subscription_tls
  --test remote_faults: **66 tests**. Host ACL/protocol, deterministic IO faults
  and actual rustls encryption over fragmented in-memory pipes remain green.
  This is not real TCP/LAN or Android/Rust socket interoperability evidence.
- env PYTHONPATH=tools python3 -m unittest
  tools.test_stage2_desktop_android_smoke tools.test_stage2_headless_android_smoke
  tools.test_stage2_scanner_android_smoke tools.test_validate_stage2_android_lifecycle
  tools.test_validate_stage2_android2unix_blueprint tools.test_validate_stage2_android_backup:
  **63 tests**.
- cargo fmt --all -- --check; git diff --check; python3
  tools/validate_stage2_android2unix_blueprint.py --strict: exit 0.

Final artifact SHA256:

```text
main.jar bae1b09351e98082912d2e4bae7af123c9123ab8b7eb7fa307ac4d673a902859
unit.jar 2ca3ad884de1c55e02e8c9ae61f9f105d040824b45a30dfcfab4498ec2a28db7
instrumented.jar 7af77256025688ffcd02a653f98bdd98c436ec66a4aeeda32a79a874ef7789c0
```

Source/requirement SHA256:

```text
Docs/stage2_android2unix_blueprint.md d76e70e369f11f0fd3d1905d56d79c2a89401b92215da48ad85314f390d96d97
Docs/stage2_android2unix_spec.md 403e6339b0318a420d93600fc52926acf6a9e46ec899bd9a9edb03c235190a28
OperationDeadline.kt cee3743aa5c95096630ebf83466d2e667925be08972c051b6868cf080bf9721e
PairingClient.kt 40faa4ed1226280a96a9ecd1e48dae282f4d88d83682729bf9be7c74c4a9880e
HostClient.kt f7567141d5613af8ea83ffdc29ecb3ebea8e44dc5e84d575ff4d9455fdb67f07
OperationDeadlineTest.kt cc6028c3777a239a8bc887b2023e5f6b2b10dc9425e997d4812a22b3be40f867
```

## Residual Scope

No new APK, device/Compose execution, camera scan, actual LAN timeout injection,
network switching or API 31/34/35 acceptance. Blocking OS/Keystore calls may
outlive an elapsed coroutine deadline until they return; the helper does not
prove universal wall-clock preemption. Current release packaging, full
concurrency scheduling, interactive remote PTY and settings parity remain
unfinished. No permission/network/device bypass. This provisional note does
not satisfy an item's full Master acceptance gates.

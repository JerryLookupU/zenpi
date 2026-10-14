# Host Invitation Clock Admission

Provisional S2-012/S2-052 progress on 2026-10-04. The previous goal turn changed
foreground session-switch recovery and ran source/policy checks, so it was
progress. Current execution skill, dirty worktree, Blueprint, production host
pairing call path, Android scanner/confirmation, issuer and quality boundaries
inspected. No worker/nested agent admitted. All 36 authoritative rows remain [ ].

## Fix

Desktop QR rendering and Android parsing refused an invitation whose remaining
wall-clock window exceeded 300 seconds. Host InviteIssuer consumption previously
checked only wall expiry and monotonic expiry. A sufficiently backdated host
clock could therefore suppress its QR while the host still accepted the exact
token/proof. An unavailable wall clock also returned without retiring the invite,
allowing a later corrected clock to retry it.

The mutex-owned consumption path now validates the complete invitation at its
single wall observation and independently checks the monotonic deadline. Wall
expiry, invalid/out-of-window clock or unreadable clock retires the active
invitation. Later clock correction cannot restore it. A wrong token/pin/SAS on
an otherwise fresh invitation remains non-consuming; successful proof consumes
exactly once. Smaller wall rollback remains bounded by the original monotonic
lifetime. QR schema, Android token handling, registration/ACL and grant defaults
are unchanged. The production PairingHost registry callback uses this issuer.

Private deterministic tests exercise the same consumed-state helper with injected
wall/monotonic observations, without changing system time or exposing a public
clock override. Cases cover exact expiry, u64 maximum, maximum-window rollback,
unreadable clock, permanent retirement, valid maximum window, wrong proof
preservation and monotonic expiry despite a backdated wall clock.

## Verification

Commands ran from /Users/mac/Github/zenpi against the dirty integration.

- rtk cargo test --locked --offline --lib remote::pairing::admission_clock_tests:
  exit 0; 4 pass.
- rtk cargo test --locked --offline --lib host_pair::lan_selection_tests:
  exit 0; 5 pass, including recoverable expired QR and terminal layout decoding.
- rtk cargo test --locked --offline --test remote_pairing --test remote_security
  --test remote_tui_pairing: exit 101. Pairing 8 pass; security 12 pass;
  TUI pairing 3 pass/2 fail. Busy-delivery real listener setup failed at
  tests/remote_tui_pairing.rs:284 with PermissionDenied/Operation not permitted.
  Production child listener test received no valid invite and failed parsing at
  line 373. Neither failed test is accepted as TLS/production network evidence.
- Explicit non-network rerun of remote_tui_pairing with --skip
  committed_tls_write_with_busy_delivery_stays_unknown_and_exact_retry_is_idempotent
  --skip production_host_pair_negotiates_and_consumes_invite_over_tls:
  exit 0; 3 pass, 2 filtered. This does not erase the full-target failure.
- rtk cargo test --locked --offline --test remote_auth --test remote_device_grants:
  exit 0; 17 pass across two suites.
- python3 -m unittest discover -s tools -p 'test_stage2*.py': exit 0; 126 pass.
- Lifecycle verifier tests 21 pass; Blueprint verifier tests 5 pass;
  strict Blueprint, Rust format and git diff --check pass.
- Android source was not changed this turn. Previous current source gate was
  439 JVM tests plus instrumented-source compile; not a connected test run.
- python3 tools/stage2_android_build.py --verify-only: exit 1, missing provenance.
  No unchanged blocked Gradle retry, old APK install, ADB/camera/LAN success.
- After all Cargo tests completed, python3 tools/stage2_host_build.py: exit 0;
  real offline locked debug build refreshed host attribution. An earlier
  attribution was invalidated by Cargo test rebuilding the production binary;
  final verify-only below uses the refreshed binary.

```text
pairing.rs 6c43f70b05be38c513d5b0cf0e12ea4271e80710ecfaccf500b20222f5fd3655
blueprint d76e70e369f11f0fd3d1905d56d79c2a89401b92215da48ad85314f390d96d97
spec 403e6339b0318a420d93600fc52926acf6a9e46ec899bd9a9edb03c235190a28
debug-host-receipt 91315921cde2e9261bda15641051eae65e6a2ce6d9b644be1186822d3d8e359e
debug-host-inputs ebc19189a14151e3b519da34397aea4e2562f729ef42e2faff123ccd0aee9733
debug-host-binary d9ef9b027304d65c79591e8187fd5439127f17124771d25bb8a4fcf21b1b83e4
```

Goal remains active. Current-source APK, physical QR/direct LAN bidirectional
sync and recovery, connected API lifecycle matrix, full parity and signed
release remain incomplete or unverified. The socket restriction is a runtime
acceptance blocker, not evidence that the full objective has been achieved.

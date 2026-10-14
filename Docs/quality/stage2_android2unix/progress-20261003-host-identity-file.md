# Host Identity File Type And Alias Boundary

Provisional S2-012/S2-052/S2-072 progress at 2026-10-03T19:20:01+0800.
Previous goal turn was progress: connection cancellation and explicit reattach
changed Android source/tests. This turn inspected pairing handshake freshness,
receipt binding, QR parsing, host invite consumption and identity storage.
No nested agents, worker launch, checklist promotion, commit/push or cleanup.

## Finding And Repair

HostIdentity::load checked file type only after a blocking open. A FIFO at the
configured identity path could therefore block pairing startup indefinitely
before validation. Existing no-follow/owner/private-mode checks also accepted
hard-linked regular files, unlike the device registry's single-link boundary.

Unix identity loads now use O_NOFOLLOW | O_NONBLOCK and require a regular
private current-user-owned file with nlink == 1 before reading/parsing it.
Hard-linked identities are refused rather than overwritten. Existing atomic
identity creation, TLS 1.3/ALPN/certificate pin, one-time token and pairing proof
semantics are unchanged. A private independent backup copy remains permitted;
hard-linking the active key is not a supported backup mechanism.

New regression checks both identity aliases are refused without byte changes,
the original works after unlinking the alias, directories remain directories
and a FIFO is refused without a writer. FIFO loading runs in an exact-filter
child with a three-second parent deadline and kill/reap on timeout, so a
blocking-open regression cannot hang the suite. The test does not bind sockets.
Protocol, threat review and operator runbook record the boundary and clarify
that a refusal does not authorize silent key rotation.

## Verification

Commands ran from /Users/mac/Github/zenpi. Rust used controlled
ZENPI_HOME=/private/tmp/zenpi-resource-reload-test-home.

- cargo test --offline --locked --test remote_pairing --test remote_auth
  --test remote_security -- --test-threads=1: exit 0, 8 + 12 + 12 = 32 top-level
  tests pass. FIFO child also passes one exact-filter invocation, not counted
  as an additional independent top-level test. Raw Cargo output was inspected
  because RTK summarizes the child's nested test result as another suite.
- rtk cargo test --offline --locked --test remote_pairing -- --test-threads=1:
  exit 0; output says nine passed/two suites including the same FIFO child.
  Actual parent target has eight tests, all pass. Earlier RTK three-target run
  similarly reported 33 including the nested child; not claimed as 33 unique
  top-level tests.
- rtk cargo check --offline --locked: exit 0, zero errors and 20 existing
  warnings. Cargo format check and git diff --check: exit 0.
- Python APK provenance ten, host smoke seven and Blueprint validator fixture
  five tests pass, exit 0. Strict Blueprint validator passes, all 36 unfinished.
- python3 tools/stage2_android_build.py --verify-only: exit 1, missing current
  APK provenance. No stale APK/device invocation or unchanged blocked Gradle/
  ADB/TCP retry. Android source unchanged; no fresh Android test run claimed.

All required command handles are terminal. Existing user/generated worktree
changes remain preserved. No security/release sign-off is implied.

## Digests

```text
src/remote/pairing.rs ed89e6c8749b3dd48df6f2908b4c633e78cb51a4c3dfcd898b4b17d7e5b32c69
tests/remote_pairing.rs d5e7252aa2cea47b11d914dc0b17d36d1261502a47e86b961c531ec5c019fcf4
```

Goal remains active. Current source-attributed APK, real scanner/physical
direct LAN, connected UI/lifecycle, full parity and signed reproducible
release remain incomplete or unverified; host file tests cannot replace them.

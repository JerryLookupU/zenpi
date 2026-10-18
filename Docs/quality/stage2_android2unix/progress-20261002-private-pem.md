# Private PEM Projection Redaction

Provisional S2-020/S2-036 security progress. Previous goal turn added the
running-provider foreground/background/reattach acceptance gate. This turn
reproduced and repaired a real projection leak: LivePromptText used the shared
redact_text helper, which did not recognize PEM BEGIN/body/END material, so
unregistered Base64-like private-key body text could be sent to Android.
The full original QR/LAN/native Android/Rust desktop goal remains active and
all 36 Blueprint rows remain `[ ]`; no whole-item acceptance.

## Implementation And Evidence

src/security.rs now runs recognized private PEM block redaction before other
shared text redaction. Labels are case-insensitive and matched via a stack;
mismatched public/private END cannot release a private body. Missing END
withholds the rest. Nested blocks and inline blocks are covered. Public PEM
certificates/keys and ordinary LF/CRLF remain intact. The already-shared
remote preview, transcript snapshot/replay and other text redaction callers
inherit the fix. No local journal or Agent history rewrite.

New tests were first run before the repair: remote_live_text exited 101 with
both new tests failing and PRIVATE_BODY visible in actual projected output.
After repair, the every-byte-split test checks each complete prefix and final
TextDone. Desktop cumulative unfinished/private preview is checked separately.
Security tests cover five label kinds, lowercase/CRLF/missing END, inline,
public preservation, nested mismatched blocks, diff-prefixed lines and
idempotence. Durable replay reopens the actual session journal, verifies wire
redaction and cursor, and checks exact original journal bytes unchanged.
An added authenticated rustls subscription scenario delivers a redacted PEM
turn over the existing bounded encrypted memory transport, not TCP or a device.

## Verification

Cwd `/Users/mac/Github/zenpi`; observed at 2026-10-02T00:59:24Z UTC.
Controlled test variable: ZENPI_HOME=/private/tmp/zenpi-pem-preview-20261002.

- `cargo test --offline --locked --test security --test remote_events
  --test remote_live_text --test remote_headless --test remote_diff
  --test remote_terminal --test remote_subscription --test remote_subscription_tls`:
  **53 pass**, exit 0 (14/5/7/1/1/5/10/10). Includes real owned PTY tests;
  headless owner fixture does not invoke a provider.
- `cargo test --offline --locked --test diagnostics --test approval_owner
  --test session_recovery`: **62 pass**, exit 0 (3/14/45).
- Stage2 Python smoke tests 34 pass, Blueprint tests 5 pass and lifecycle
  validator tests 9 pass: **48 pass**. Strict Blueprint validator exits 0
  with 36 incomplete rows. Cargo fmt and git diff checks exit 0.
- `cargo check --offline --locked`: exit 0. Existing vendor/dead-code warnings
  remain. No APK/device re-execution this turn; previous wrapper-lock permission
  denial remains unresolved, and historical receipts are not promoted.

SHA-256:

```text
src/security.rs 5d6b1affa9e8eba57e67a800f533ab41638f8a482f2f2165970e93514b8c64cc
tests/security.rs 9ecd940735f098b0d4d27d4b59c5afdacff1ae01a43d4504c575847f9ee57652
tests/remote_live_text.rs 7056bd68b5470160907e94ebcdcfa883a29a748d045b640b0379f81aed4e007f
tests/remote_events.rs 979664a3d4b51f84680bf40567adf6725049c3cd2f1f8ece213afc0cf0480bd5
tests/remote_subscription_tls.rs 36edaa10fc7910e52045e2ccb3f3eee86b084d9daf5bb71e06034bf250280a88
```

Residual risk: unknown or obfuscated secrets, lost BEGIN before input window,
full delta transport/replay/backpressure, interactive PTY/bulk, actual 1..64
independent-worktree scheduling, fresh API31/34/35 Keystore/lifecycle APK,
physical camera QR/same-WiFi LAN and release/migration remain required.
Existing dirty changes (including earlier constant_time_equal visibility)
preserved; no agents, commit/push or permission bypass.

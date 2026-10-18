# Desktop QR Expiry Recovery And Full-Frame Layout

Provisional S2-021/S2-031 progress on 2026-10-04. Previous historical-detail
revision turn changed source and passed deterministic gates, so it was progress.
Current execution skill, Blueprint policy, dirty worktree, desktop pairing loop,
invite issuer/encoding, terminal rendering and QR tests were inspected. No
workers/nested agents admitted; no authoritative checkbox promotion.

## Implementation

The desktop pairing view previously called QR encoding before checking expiry.
Expired invitation validation propagated out of render into the desktop loop,
potentially terminating the running desktop instead of leaving Refresh usable.
Rendering now captures one wall-clock observation for invite validation,
encoding and countdown. Expired, unavailable-clock and backwards-clock-window
states hide QR and render explicit recovery text without an error exit. Rendering
does not renew/consume an invitation or change registered-device grants.
Refresh remains explicit and invalidates the old one-use token. Standalone
listener duration remains bounded; the persistent desktop retains its lifecycle.

Metadata/footer now wrap independently from QR spans. Minimum height is derived
from actual wrapped rows at the current width, including complete grant/recovery
actions. A too-small terminal shows resize dimensions with no partial QR. QR
module grid/quiet zone remain intact. Invite serialization shares a validated-
observation helper rather than racing a second wall-clock read at the last
second or duplicating the URI format.

New tests exercise expired/exact-expiry/unavailable/backwards-clock states,
explicit refresh token invalidation, complete rendering at minimum/100/140
columns for IPv4/IPv6 and both desktop/standalone labels. Tests reconstruct
pixels from the full ratatui frame and decode exactly one matching invitation.
At the exact required height QR is present; one fewer row refuses it and shows
resize guidance. Recovery text and complete fingerprint are checked and raw
token is not printed as text. This is deterministic rendered-buffer evidence,
not a physical camera, real terminal-font or LAN receipt.

## Verification

- Final host_pair::lan_selection_tests selection: 5 pass (includes two new
  expiry/full-frame layout cases).
- remote_pairing: 8 parent tests pass, plus its isolated FIFO child case passes.
  RTK aggregates that nested child as nine; authoritative parent suite has eight.
- remote_tui_pairing terminal_pixels_decode selection: 1 pass, four filtered.
  Production listener cases were not run or credited by this selection.
- Python Stage 2 discovery: 125 pass; Blueprint validator tests: 5 pass;
  lifecycle validator tests: 21 pass.
- Strict Blueprint, Rust formatting and git diff --check pass; all 36 rows [ ].
- Real offline locked debug host build completed. A later Cargo test build
  changed the binary and verify-only correctly refused the earlier attribution;
  final canonical host build/verification is recorded below.
- Android source is unchanged this turn; prior 431 JVM/source receipt is
  historical source evidence, not new APK or device acceptance. APK verify-only
  refuses missing provenance. No unchanged blocked Gradle retry, historical APK
  install or physical/ADB/LAN run occurred.

```text
host_pair.rs 4758ecd6f0e73e57e908cebbb07722bea48a2fd3b2a2eb419b8006abb5b6d189
pairing.rs c26fddcb4b24f81df90daec824cd324614b71bb3a0382ac92999b618c283ad1d
debug-host-receipt d3aeb4ad7a6c6c1427246ca98464d1072f66663a0030fb231c8682a53d5b16f3
debug-host-inputs 6e853386d38b5ab7bd6667713d8c4eb057979731687da65a84cd99ce38a0b884
debug-host-binary 7218257a1d568a74bd8309b9682db79805f9a0eb74476504d15cb8daa0f7d694
```

Goal remains active. Current-source APK, physical QR/direct LAN bidirectional
sync/recovery, connected API lifecycle matrix, complete parity and signed release
remain incomplete or unverified. No entire Blueprint item is accepted.

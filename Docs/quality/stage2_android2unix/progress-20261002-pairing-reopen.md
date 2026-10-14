# Reopen Desktop Pairing Without Restart

Provisional S2-012/S2-021/S2-054/S2-073 progress observed
2026-10-02T05:56:03Z. Previous goal turn: progress (shared headless shutdown and
completion-persistence budgets). HEAD 1a8ff1acf72e0816fad503a05df2b1d18e07361b;
existing dirty baseline preserved. All 36 authoritative rows remain [ ]; full
goal active. No agents, worker admission, commits, pushes, cleanup or completion.

## Implementation

After leaving the startup QR screen, a remote-enabled desktop previously had
no route back to pairing; an expired invitation or second phone required
restarting the desktop owner. Added explicit F3 OpenPairing TUI action. The
production remote desktop refreshes its existing InviteIssuer and reopens the
QR overlay without replacing listener/address/identity/Agent/session. Ordinary
TUI and callback-only hosts report unavailable and never create a listener.
Running/queued model work, master work, session transitions, pending/queued/
preparing input controls and pending approvals fence reopening. Modal keyboard
ownership is preserved and unsent composer text/cursor are not submitted.

Desktop QR view now accurately says project access/controls require local
device grant instead of claiming all writes unavailable. Its footer names F3
reopen; plain pairing-only mode retains write-unavailable text. User guide and
protocol explain the existing-listener refresh, idle gate and no-listener default.

## Verification

Cwd /Users/mac/Github/zenpi; Rust ZENPI_HOME=/private/tmp/zenpi-pair-reopen.

- rtk cargo check --offline --locked: exit 0, 0 errors, existing 20 warnings.
- Final rtk cargo test --offline --locked --test tui_composer
  --test tui_interaction --test tui_approval_focus --test tui_project_workspace
  --test remote_pairing --test remote_lan --test remote_project_inspection
  --test remote_parity: exit 0, **182 pass** across eight suites.
  New explicit F3 key/release/modifier/modal/draft tests; existing pairing
  refresh invalidates old token and retains identity, LAN selection, local CLI
  and owner-parity regression. These are not a production keyboard+socket run.
- rtk cargo test --offline --locked --lib desktop_pairing_render: exit 0,
  **1 pass**. Real ratatui TestBackend checks desktop/plain grant text, address,
  SAS/fingerprint, no plaintext token, nonblank black QR cells, reopen footer
  and narrow-terminal resize fallback. This is a buffer check, not camera proof.
- rtk cargo test --offline --locked --test remote_tui_pairing -- --skip
  committed_tls_write_with_busy_delivery_stays_unknown_and_exact_retry_is_idempotent
  --skip production_host_pair_negotiates_and_consumes_invite_over_tls:
  exit 0, **2 pass**, 2 TCP tests explicitly excluded. Actual terminal QR pixels
  decode and host CLI policy retained.
- Six Python smoke/lifecycle/blueprint/backup modules: exit 0, **63 pass**.
- cargo fmt --all -- --check; git diff --check;
  python3 tools/validate_stage2_android2unix_blueprint.py --strict: exit 0.

SHA256:

```text
host_pair.rs 64bef95ad5d194d9cd818ac636f1c062d9b60f188c12414e79e24c9d77a37357
tui.rs 94faebe4d6acd677f61fe3174d674da6314b5fba3041c1a8275a931d38dadec4
tui_composer.rs e07fea25cf06cb175022cc107d260bb5a9cc7b2d1565a910d1d85871d4858cfb
user_guide bc5434a3526518606f345c5ff0f852f0f8fad677039bffd524c31f17e65dd542
```

Provisional progress, not Master acceptance receipt with captured output hashes.
No current APK/device/camera/LAN exercise or process-level F3 re-pair acceptance
claimed. Android source unchanged. Full concurrency scheduler/stepper, settings,
interactive PTY, current API/device matrix, security sign-off and signed release
remain unfinished. No socket/adb/Gradle bypass or checklist promotion.

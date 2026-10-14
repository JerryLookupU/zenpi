# Remote Session Catalog And Canonical Resume

Provisional S2-019/S2-036 progress on 2026-10-03. Baseline is the dirty current
worktree at HEAD 1a8ff1acf72e0816fad503a05df2b1d18e07361b, not a clean
checkout. The previous RTK-only turn was no implementation progress toward
Stage 2; this continuation revalidated the existing session work and completed
the following source/test changes. No nested agents, checklist promotion,
commit/push, worker launch or cleanup. Goal remains active.

## Implementation

The Rust host adds project-Read session_list with 16-entry snapshot-bound pages
and Control session_resume with target session ID plus catalog digest. Catalogs
enumerate only the current session directory, exclude the domain store and
foreign canonical workspaces, refuse symlinks/duplicate IDs/unclean journals,
and enforce file/count/byte/cooperative deadline caps. Responses contain only
identity, creation time, turn count and current flag; no paths or transcript.
Catalog hashing covers all accepted durable records. Limit errors are explicit,
not a truncated result presented as complete.

Resume reuses the existing Agent preparation and replacement, rather than a
second session owner or a slash echo. It requires current main-tab/idle/input/
worker/owner/revision authority. A durable null receipt reserves the identity;
the prepared target and rechecked catalog precede the atomic mapping/success
receipt commit. Exact retries resolve before fresh session checks, also after
restart; incomplete reservations remain unknown and cannot trigger a replay.
The pool's input/approval handles and TUI cursor/transcript follow the actual
replacement. Receipt replay does not erase later view messages.

Android adds strict paged catalog and persisted-resume request/receipt codecs,
foreground/pending/scope fences, a transient session dialog, current-session
marking, confirmation and previous/next/refresh. Metadata is not persisted as
offline authority. Successful switching fetches a fresh selected snapshot.

## Verification

Commands ran from /Users/mac/Github/zenpi. Rust tests set
ZENPI_HOME=/private/tmp/zenpi-resource-reload-test-home; no credentials supplied.

- rtk cargo test --offline --locked --test remote_workspace --test session_new
  --test remote_protocol --test remote_auth -- --test-threads=1: final 106 passed
  across four suites, 29.25 seconds, exit 0. Covers 21-session paging, foreign
  scope, stale digest, Read/Control distinction, canonical resume, refreshed
  input port, exact retry/restart/revoke, malformed/empty/symlink refusal and
  journal-change continuation refusal. Earlier 106-test run also passed.
- rtk cargo test --offline --locked --test remote_headless_commands
  headless_new_session -- --test-threads=1: one passed, exit 0. Production
  mailbox performs catalog, resume and exact retry on the same Agent.
- rtk cargo test --offline --locked --lib existing_terminal_projects_only
  -- --test-threads=1: one passed, exit 0. Real TUI route reconciles session
  cursor for new/resumed sessions; exact retry preserves subsequent view data.
- rtk cargo test --offline --locked --test remote_subscription_tls
  --test remote_parity -- --test-threads=1: final 46 passed across two suites,
  11.66 seconds, exit 0. New authenticated fragmented in-memory TLS test lists
  metadata without private content/paths, resumes the canonical owner and
  verifies exact retry changes neither old nor resumed journal. Earlier
  45-test regression also passed. This is not TCP/LAN/device acceptance.
- rtk cargo check --offline --locked: final exit 0, zero errors and 20 existing
  warnings. Final check uses the isolated ZENPI_HOME above.
- zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-session-resume-verified-20261003: exit 0. Production,
  unit and instrumented Kotlin sources compiled; 290 JVM tests passed.
  SessionCatalogScreenTest compiles at 320 dp but was not device-executed.
- cargo fmt --all -- --check; git diff --check: exit 0.
- python3 tools/validate_stage2_android2unix_blueprint.py --strict: exit 0,
  36 unfinished items; structure is not product acceptance.
- python3 tools/test_validate_stage2_android2unix_blueprint.py: 5 passed.
- python3 tools/test_validate_stage2_android_lifecycle.py: 15 passed. These
  validate receipt logic only. Direct lifecycle validator invocation without
  required desktop/headless receipt arguments exited 2; not a lifecycle pass.

The previous catalog test failure was real: local read-only SessionStore
inspection tolerates malformed input and supplies a synthetic header. Fixed
the remote catalog to require an actual sequence-zero header and no recovery
warnings, before workspace filtering. Tests also verify invalid files are not
rewritten. No permission bypass or unchanged TCP/ADB retry was attempted.

## Artifact Digests

```text
src/remote/sessions.rs ffa605c9ee6acecf4f29afcb9643590f00644c5325e1708741eebe1eed3887b6
src/project_workspace.rs 3d88f165464e559d784ba3c4e6c01adcab1d86194a7bddb3e6dbcf80d7f67160
src/tui.rs 32cb949b773038ccaefa55cfe6e84588c6dbc53ae1a4453417eab0824172ad20
tests/remote_workspace.rs 1cbb332e38ff2fb82500380a3c23510caeff5155a2d0c7c422c2e0ec20590ac7
tests/remote_headless_commands.rs d858f5e71f67d6eb3cd9dba095bc3cb8021ec3f599649f2bdb6b359f5a092bd2
tests/remote_subscription_tls.rs 5f00eeb7b5421bf553eb4ef771cad9e4a16a50eb14d4ee91e486bb063af36529
SessionCatalogCodec.kt 8d0edec9b0c72aef00a82f7e55dea24686e1c1d400c61e9408ef2b05e4a6c4a4
MainActivity.kt c2da71a8e7654e2f2fa00907028348a8716d9409714080005bac2c01df59fa3e
main.jar 13a6e7c9d0a36a563d500adf7563cd8f08fcb75257e2059f9c785bb68e67d4c5
unit.jar 878ff1561330c8ee769b2da4ab3e8850d586ad19649500831f26d69f85523ef7
instrumented.jar 1bd17c7a44c279190132ebc4a38f837b58e99a00e5b2b567823ccc89e1af37f4
```

Rename/delete and full settings parity, terminal emulation, current production
provider/tool E2E, physical camera/direct LAN, connected lifecycle and signed
reproducible release remain incomplete or unverified. No checklist acceptance
is claimed by these focused tests.

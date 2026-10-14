# Current Parity And Encrypted Subscription Evidence Audit

Provisional S2-019/S2-020/S2-050/S2-052/S2-053/S2-054/S2-056 progress on
2026-10-03. Previous turn changed explicit pairing recovery stop behavior and
produced current source evidence, so it was progress. Current Blueprint,
specification, matrices and test bodies were inspected. No nested agents,
checkbox promotion, commit/push or unrelated worktree cleanup.

## Audit Result

remote_parity proves five narrow owner/root/configuration cases, not complete
Android parity. The parity matrix now maps each required behavior to its actual
deterministic target and names the missing runtime proof. It explicitly
distinguishes real rustls encryption over in-process fragmented pipes from the
production TCP listener and Android implementation. Default target success does
not prove ignored live-listener cases.

The security matrix now records explicit transport identity admission,
key/reset serialization and interrupted/cancelled pairing recovery, while
retaining actual Keystore/network acceptance as open gates. Removed a stale
embedded Kotlin test count rather than carrying historical numbers forward.

## Commands And Results

From /Users/mac/Github/zenpi:

```text
cargo test --offline --locked --test remote_parity --test remote_migration --test remote_frame --test remote_pairing --test remote_security --test remote_auth --test remote_events
cargo test --offline --locked --test remote_subscription_tls --test remote_protocol --test remote_faults --test remote_host_e2e
```

Both exited zero. Target results:

| Target | Passed | Ignored | Verified scope |
| --- | ---: | ---: | --- |
| remote_auth | 12 | 0 | Signed identity, current ACL, grants, revocation and contention |
| remote_events | 8 | 0 | Durable bounded/redacted replay, gap and branch replacement |
| remote_frame | 10 | 1 | Deterministic framing/IO faults; loopback timeout remains ignored |
| remote_migration | 6 | 0 | Atomic legacy import, refused corrupt/conflicting migration |
| remote_pairing | 8 | 0 | Identity privacy, one-use invite, expiry and shared Android vectors |
| remote_parity | 5 | 0 | Captured root/worktree execution and pending-input model guard |
| remote_security | 12 | 0 | Wrapper reruns remote_auth, not 12 additional unique cases |
| remote_faults | 3 | 0 | Injected half-read/write timeout and truncation, not sockets |
| remote_host_e2e | 2 | 3 | Provider fixture framing/response logic only |
| remote_protocol | 14 | 0 | Strict envelope, shared corpus and seeded mutations |
| remote_subscription_tls | 43 | 0 | Production subscription over real encrypted fragmented pipes |

123 target executions passed, four ignored; 111 distinct cases because the
security target includes the authentication test body. No TCP, phone, camera,
provider HTTP listener or complete parity success is inferred. The live tests
remain open rather than being silently skipped into acceptance.

Stage2 Python discovery 107, lifecycle verifier 21 and Blueprint tests five
pass. Formatting/diff/strict Blueprint pass; 36 rows remain unfinished. APK
verify-only still refuses missing provenance, before any device call.

Cargo test replaced target/debug/zenpi, and current-source attribution correctly
refused its old binary digest. The owned build tool is used to rebuild and
republish debug provenance, not manually stamp the changed artifact. Release
verify-only remained current. Existing compiler dead-code/vendor warnings are
not new acceptance failures and were not refactored here.

The debug rebuild exited zero and final verify-only passed. Its build receipt
digest is 826c7229b7cc3edc0f9342c3eb6618a5b8f7ff656018429074e3943bb8f7fd9e;
binary digest 57b52b80beda551642c47cd1f404f2fd1400b7a16699f91401402cb77db227a8,
source inventory 2fa078489a749991bf55b24c8d412bdf554bf9ba0cf37e78cf83f44683aecb4f.

## Source Identity

```text
remote_parity.rs c5875d6615a5fffd4f73226b389ca9585ccd8b67965353773f5e2c9a965fcf12
remote_migration.rs df05246533bc0bd25f7326ff3ce5920905c2abb5fabf4aa25b169bf5fd2dce2c
remote_auth.rs dda167548ad9214b9728c2d73ac18681c41be7346ba7f68ce9f959b6491ed8fe
remote_events.rs 59e75050c00a3662dd0477345685e4c1cfc87392deb3edbb7969c0288bfc516b
remote_frame.rs 9ec8210363402ac92a4b2f3daeb11d2fd064acaa9efa95a9b6326f35885d8d33
remote_pairing.rs d5e7252aa2cea47b11d914dc0b17d36d1261502a47e86b961c531ec5c019fcf4
remote_security.rs f00c67d02768f8dd5dcd740a4530ac7d7c1d4eae61cd1558d7c7e5e3a0a9a90b
remote_subscription_tls.rs 0ce08b2b3bdf00556c6ba21b546c8a59eec4de35b2bf51233ad733c7769bda28
remote_protocol.rs f88aa212cbeee2892eaba5a3e06e0dda2697237d75450fdfa7d52c8c4b0a5d68
remote_faults.rs c0ebf6ae1fbf2a922613a1116d0543461f5a431d16fe83f673f22216d47657ca
remote_host_e2e.rs d9587f73357930c94724e3bc15f0d132cb335f3fb8ec09d45121e33418418d3c
```

Goal remains active. Current APK/device, physical QR/direct LAN desktop sync,
full parity, supported API lifecycle and signed release are still unaccepted.

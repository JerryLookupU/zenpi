# Stage 2 Broad Rust Compilation And Owner Audit

Recorded 2026-10-04T09:56:39Z; verification evidence, not Blueprint acceptance.
Previous goal turn was progress: retained active prompt metadata repaired and
verified. This turn widened validation to desktop compatibility rather than
adding another narrow client predicate. Dirty worktree preserved; no nested
agents, reset, global configuration edits or permission bypass.

## Commands And Results

All commands ran from /Users/mac/Github/zenpi with private writable ZENPI_HOME
roots; logs are retained under /private/tmp. No production code was changed.

1. ZENPI_HOME=/private/tmp/stage2-full-compile-home-20261004 cargo test --locked
   --offline --no-run: process 72224 polled to exit 0, 122 test executables
   emitted; test-profile build finished in 7m37s. This proves compilation,
   not execution of 122 suites or behavior of ignored network/device tests.
2. ZENPI_HOME=/private/tmp/stage2-library-home-20261004 cargo test --locked
   --offline --lib: process 48510 polled to exit 101. 423 passed, 26 failed,
   seven ignored, zero filtered; execution took 18.23s after Cargo lock wait.
   No full-library/full-Rust pass is claimed.
3. ZENPI_HOME=/private/tmp/stage2-extension-audit-home-20261004 cargo test
   --locked --offline --lib extension_runtime::pipe_deadline_tests --
   --test-threads=1: process 15543 exit 0, four passed, one helper ignored,
   451 filtered. These are all four parent pipe/deadline cases, not a full
   library repeat.
4. ZENPI_HOME=/private/tmp/stage2-extension-resume-audit-20261004 cargo test
   --locked --offline --lib core::extension_resume_commit_tests::rejected_resume_commit_keeps_old_extension_live_until_successful_retry
   -- --exact --test-threads=1: process 10241 exit 0, one passed/455 filtered.
5. ZENPI_HOME=/private/tmp/stage2-host-owner-audit-20261004 cargo test --locked
   --offline --test remote_workspace --test remote_worktree --test remote_concurrency
   --test remote_terminal --test remote_worktree_runner --test remote_worker_owner
   --test remote_subtab --test remote_parity --test remote_migration:
   process 42274 exit 0, nine primary targets/146 tests passed. Three subtab
   child invocations each passed one already selected parent scenario; these
   are not counted as three additional independent primary tests.

Primary target counts: concurrency 12, migration six, parity five, subtab 11,
terminal nine, worker-owner 12, workspace 72, worktree six, runner 13.
These cover actual local journals/Git/PTY/owner/scheduler boundaries, not
production TCP listener, Android Keystore or direct-LAN/physical QR workflows.

## Failure Analysis And Next Gate

Of the 26 library failures, 21 auth/callback/backend cases explicitly fail at
loopback fixture setup with PermissionDenied/Operation not permitted. They are
not successful transport tests. Four extension pipe tests and one extension
resume test fail under the parallel library run with interpreter-entry/fixture
deadline evidence. Pipe logs retain host return timestamps, missing fixture
entry/PIDs and one later successful stdin variant. Their serial exact-scope
reruns all pass without source changes. This supports scheduling-sensitive
fixture startup as an inference, not proof that parallel execution is reliable
or that all extension paths are correct. Do not erase or supersede the full
library failure with these focused passes, weaken timeouts, or mark failures
ignored to manufacture a green gate.

The next full behavioral gate needs an execution environment permitting the
required local listeners and a reproducible resolution of the parallel
extension-startup failures. Current APK provenance, physical camera QR/LAN,
API31/34/35 lifecycle and signed release remain separate unmet requirements.

## Final Checks And Hashes

cargo fmt --check, git diff --check and strict Blueprint exit 0; authority
marks remain 36 [ ], zero [_]/[x]. Host debug attribution refreshed after all
Cargo test commands; verify-only exits 0. APK verify-only remains missing
provenance. No Kotlin source change or new JVM pass is claimed this turn.

```text
full-compile-log 9e3f2209fc5e106205617ae898ee9f84213fa177ec6d9c8bbca727fd1a03f84a
failed-library-log 7f16a60db17f1269f2100676eb113635a8bb2c88478d3409f6d43b774742ed5e
serial-pipe-log 47af51daf32c9f3f663c52e9b360ed2e7e896fed11162ec0e911f35eebf35443
serial-resume-log 6c280a46e919f1b6642e38f4d72ce1113f81911b432d0affbeefb6689b1afaf0
owner-audit-log e2b5d265ce289dcf990240db30c8acd43932eb16800c81f3fa99dd37b02da0b5
host-receipt a6b68d32643b0e0918dc966e236905d17746965b1a08884aa623c82de4bde79a
host-inputs 8ccf956a10a2fcac07b143c4d28aa8d62d1f59af1b5623a338da499e0b09906b
host-binary 40f9f134ed62fd51a71dc6e8d24022f0122dced68faf12f6a52f8b5ea145607f
```

Goal remains active/incomplete. New authoritative evidence changes the next
action: compilation is now covered broadly, local owner tests pass, and the
full-library failure categories are narrower than prior default-home runs.
This is progress, not a completion or blocked-status declaration.

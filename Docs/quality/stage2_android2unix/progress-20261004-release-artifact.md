# Stage 2 Current Rust Release Artifact

Recorded 2026-10-04T10:01:22Z; partial S2-070 evidence, not acceptance.
Previous goal turn was progress: broad test compilation/local owner audit
established 122 compiled test executables, 146 local owner tests passed and
the unresolved full-library 423-pass/26-fail result. Dirty baseline preserved.
No nested agents, reset, denied Gradle/ADB/listener retry or permission bypass.

## Artifact And Verification

From /Users/mac/Github/zenpi ran python3 tools/stage2_host_build.py --profile
release, process 9490 polled through exit 0. The builder uses cargo build
--offline --locked --bin zenpi --message-format=json --release, verifies
unchanged source inventory and binds the exact Cargo-reported executable.
python3 tools/stage2_host_build.py --profile release --verify-only exits 0.
This produces current artifact attribution, not independent reproducibility.

The actual release binary host pair --help exits zero with HOME removed and
ZENPI_HOME=/private/tmp/stage2-release-help-absent-20261004. The help output
lists desktop QR/connection/grant/revoke/recovery controls and explicit LAN
limits; test ! -e on that home exits zero after the command. No invite token,
identity or listener was created by this help path. No production listener
smoke was repeated after its previously established permission denial.

Ran ZENPI_HOME=/private/tmp/stage2-security-regression-20261004 cargo test
--locked --offline --test remote_protocol --test remote_frame --test remote_pairing
--test remote_auth --test remote_security --test remote_events --test remote_faults
--test remote_subscription --test remote_subscription_tls. Process 74821 exited
0, nine primary targets/133 tests passed and one frame loopback test ignored
under its original permission prerequisite. Counts: protocol 18, frame 10,
pairing eight, auth 12, security 13, events nine, faults three, subscription 17,
TLS subscription 43. One pairing child repeats an already counted parent case;
it is not an additional primary test. Auth/security share included cases, so
133 is execution count, not 133 distinct security invariants.

TLS subscription uses real rustls over memory pipes, not the production TCP
listener or Android transport. Faults establish codec/replay boundaries, not
physical LAN behavior. These passes do not erase full-library failures.

- Python Stage 2 tools: exit 0, 145 tests.
- cargo fmt --check, git diff --check and strict Blueprint: exit 0;
  36 [ ], zero [_]/[x].
- Debug host attribution refreshed after all Cargo targets; verify-only exits 0.
- Android verify-only: exit 1, missing APK provenance. No Kotlin/Gradle/device
  pass is newly claimed this turn.

```text
release-binary f32c9381024afb00099c519822a2b8a65c2c99cc5216a59e9dbcb124c35e6d6b
release-receipt 531e068bcf1c1a8bac39cfe48f0bf6b22ca5ab10d3eb2af1c6c23c5024761970
host-inputs 8ccf956a10a2fcac07b143c4d28aa8d62d1f59af1b5623a338da499e0b09906b
release-build-log a3d918c21eb2c6e110c6a10cb9c6bd7596fea5c7792166209dcc31b3ddc409f3
release-help-log 66b4d6e11ad50c7b8af137c84314783f34ccdc3a9c844fb52c96a9be7482fcf0
security-log 997aa6d4970f1aeac3f24833ff663e63e18f50da14b5d8980da2cc96bbed44a4
python-log 4a6970bb3622daf27fb50c70a1666a2c01e995b71fdef63c2c0e5b4fdfd8613d
debug-receipt c51f333294bc444534ab619fe049c30263faa5a603661ac158d4f057b5e0f06d
debug-binary 40f9f134ed62fd51a71dc6e8d24022f0122dced68faf12f6a52f8b5ea145607f
```

Goal active/incomplete. Independent reproducible host build/full behavioral
gates, current signed APK, physical camera QR/direct-LAN bidirectional sync/
recovery and API31/34/35 connected lifecycle remain unmet. Artifact creation
and broader regression evidence are progress, not full completion or blocked.

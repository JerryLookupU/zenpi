# Stage 2 LAN Source Address State

Recorded 2026-10-04T06:52:18Z. Partial S2-032/S2-038/S2-053 evidence,
not acceptance. Prior LAN implementation turn made progress; the intervening
RTK installation check did not advance product acceptance. This continuation
harvested the confirmed source-check session to terminal exit 0, revalidated
the current files and recorded its evidence. Canonical cwd
/Users/mac/Github/zenpi; dirty HEAD 1a8ff1acf72e0816fad503a05df2b1d18e07361b.
Existing changes preserved; no nested agents, reset, push or permission bypass.
All 36 Blueprint rows remain [ ]. Blueprint SHA-256
d76e70e369f11f0fd3d1905d56d79c2a89401b92215da48ad85314f390d96d97;
spec SHA-256 403e6339b0318a420d93600fc52926acf6a9e46ec899bd9a9edb03c235190a28.

## Finding And Change

The existing LAN link comparison captured source IP/prefix, interface and MTU
but omitted Android LinkAddress flags and scope. A source-address state change
could therefore leave both the binding observation and monitor inventory equal
despite changed address usability. No assertion is made that this fully models
kernel address selection or eliminates every post-observation race.

LanAddressState now captures address, prefixLength, flags and scope, shared by
LanSocketRouting's LanLinkState and NetworkMonitor's NetworkLinkStamp. Changes
observed across socket binding refuse before callers can connect, with no
default/cellular/VPN fallback. The same metadata changes the inventory revision
used to fence foreground/background recovery. Stable repeated state and source
enumeration order remain unchanged.

Expected/actual: unchanged IPv6 literal/prefix with changed flags or scope is
refused after binding and before the simulated connect; each changed state
publishes a distinct monitor revision, but its identical repeat does not.
Existing DHCP/interface/MTU, route replacement, refusal/ambiguity and stable
enumeration tests also pass. These are policy/JVM checks, not physical Android
network callback or kernel socket evidence.

## Verification

Source log created 2026-10-04T06:48:25Z, final output 06:51:05Z; terminal
session harvested in this continuation, exit 0. Canonical cwd for all commands.
OUT and per-lane ZENPI_HOME isolated under
/private/tmp/stage2-lan-address-state-20261004; credentials unchanged.
No production/test source edits after the source-check launch.

- zsh tools/stage2_android_source_check.sh /private/tmp/stage2-lan-address-state-20261004:
  exit 0; current main/unit/instrumented compilation, 557 ordinary JVM tests,
  production QR corpus and explicit current Kotlin/Rust interoperability lanes.
  Includes the six-test desktop target and actual Agent prompt/tool/running
  worker cancellation lanes. Instrumented tests compiled, not device-run.
- python3 -m unittest discover -s tools -p 'test_stage2_*.py': log reports
  137 tests, OK; original process handle no longer exists at reobservation.
  This report does not infer an exit code from that missing handle. A fresh
  rerun in this continuation exited 0, 137 tests, OK (2.827 seconds).
- python3 tools/test_validate_stage2_android_lifecycle.py: rerun exit 0,
  25 verifier fixture tests, not connected API acceptance.
- python3 tools/test_validate_stage2_android2unix_blueprint.py: rerun exit 0,
  five tests.
- cargo fmt --check, git diff --check and
  python3 tools/validate_stage2_android2unix_blueprint.py --strict: exit 0;
  36 [ ], zero [_]/[x].
- python3 tools/stage2_host_build.py and --verify-only: exit 0 after Cargo
  source lanes; current debug host reattributed, not release acceptance.
- python3 tools/stage2_android_build.py --verify-only: exit 1, missing APK
  provenance. Unchanged Gradle/socket/ADB constraints not retried or bypassed;
  no historical APK installed and no physical QR/direct-LAN claim.

Source log /private/tmp/stage2-lan-address-state-20261004.log.
Python log /private/tmp/stage2-lan-address-state-python-20261004.log.

```text
LanSocketRouting.kt f7dc5d6f7b613257d00d572c033aaa1b8dd8f5e34249420d8a4caa3ea38261b1
NetworkMonitor.kt 38e98fb3ecea5b017e058354da129477becd2d14a3c80580ac8b47c671d6d5d1
LanRoutePolicyTest.kt e38f8a271f003d64fb2ecc92a5c7cd3652795d9907829203a46c56d31ed59daa
source-log 032ec42e43e6e6b9739562e703c0d53ac6351227fc02f526897052abde270076
python-log bb23ad65dab7999e9f5d11cf4c8f171b9a267b45ce1fc84f93785085250576ff
main.jar 0329115961b42155bfd064b91699f63bce0c8e8d833162a85a5a5efa8e7e3469
unit.jar 54745dd3ee5d21930afae36b2d2b13586e766a045d3a25e70803c801d2cf1374
instrumented.jar 2d23ab49f0d6f3119aaca7789b1e03e01d197c8ca913b96d97d070521f90cbf7
host-receipt 1b906d0912a08f2072efe43f4342b466c35a14707aa606c53df3bb386d49b161
host-inputs 373a8f5697a5bc95c10eda94c9b9b35facda736cd84bcdb0c0fa3e8703c8e58f
host-binary 2939439799b49d0e22f6072e95288a44b239e4d8eb999ad9f44bfaaf5480a861
```

Goal active/incomplete. Current APK, physical camera QR/direct-LAN
bidirectional synchronization and recovery, connected API31/34/35 lifecycle,
complete desktop parity and signed reproducible release remain required.

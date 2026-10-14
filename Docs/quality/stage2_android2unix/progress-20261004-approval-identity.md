# Stage 2 Approval Identity Progress

Recorded 2026-10-04 UTC. Related requirements S2-036 approval projection,
S2-052 security and S2-054 parity. Partial evidence only; authoritative
Blueprint remains 36 rows [ ], no whole-item acceptance claimed.

## Authority And Implementation

Cwd /Users/mac/Github/zenpi; integrated dirty-worktree HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b. Existing changes preserved; no nested
agents, reset, push, permission bypass or stale APK installation.
Blueprint SHA-256 d76e70e369f11f0fd3d1905d56d79c2a89401b92215da48ad85314f390d96d97;
specification SHA-256 403e6339b0318a420d93600fc52926acf6a9e46ec899bd9a9edb03c235190a28.

Approval entry and confirmation previously omitted scan/reset guards, and
construction could ignore a mixed duplicate project ID. ApprovalProjectionCodec
current now requires unique selected project ID, matching session/capability
and exactly one request/worker-session identity equal to the displayed target.
ApprovalDecisionPolicy shares this check across UI entry, confirmation and
ViewModel preflight, with live phase and scan/reset/pending-intent guards.
Construction and fresh dispatch also revalidate unique target identity.
ViewModel preflight requires foreground and network. Different workers with the
same request ID remain distinct; duplicate identity records are refused.
Exact saved envelope resolution remains separate and retains its existing
foreground/network/scanner/reset guards, never silently creating a new decision.

Two new JVM tests exercise lifecycle, unique scope/target, separate Approval
capability, valid different-worker targets and exact saved retry. Actual
Kotlin/Rust coordinator lane checks these refusals on real projected approvals.
Existing Compose confirmation test adds scan/reset/foreign/mixed duplicate and
duplicate target refusal. Instrumentation compiled only, not device-executed.

## Verification

Commands ran from cwd above. Source checker uses declared ZENPI_ANDROID_*
fixture paths and isolated ZENPI_HOME fixture directories. No credentials or
user configuration were modified.

- Initial zsh tools/stage2_android_source_check.sh /private/tmp/stage2-approval-identity-20261004:
  exit 1 due to an extra parenthesis in the newly added JVM fixture. This is
  retained failure evidence, not counted as pass. Replaced nested fixture
  expressions with a local typed pending-target helper; no production guard
  was relaxed to pass tests.
- Final zsh tools/stage2_android_source_check.sh /private/tmp/stage2-approval-identity-final-20261004:
  exit 0; 527 ordinary JVM tests, production QR corpus, all three source
  compilations and every explicit Kotlin/Rust lane pass. Includes allow/deny
  actual coordinator one-shot/digest/ACL/restart receipts, six-test desktop
  target and production tool-loop lane. No source/test edits after final run.
- ZENPI_HOME=/private/tmp/stage2-approval-identity-host-home-20261004 rtk cargo test
  --locked --offline --test remote_approval_wakeup: exit 0, 1 pass.
- python3 -m unittest discover -s tools -p 'test_stage2_*.py': exit 0, 137 pass.
- python3 tools/test_validate_stage2_android_lifecycle.py: exit 0, 25 pass.
- cargo fmt --check, git diff --check and strict Blueprint verifier: exit 0.
- python3 tools/stage2_host_build.py and --verify-only: exit 0 after Cargo lanes.
  Current debug source attribution is not reproducible release evidence.
- python3 tools/stage2_android_build.py --verify-only: exit 1, missing APK
  provenance; unchanged Gradle/ADB permission failures were not retried.

```text
ApprovalProjection.kt 01c222f0594e9997eab15f59341fcd2182015e914b5e9d552932e2ca7c2879b0
ApprovalDecisionPolicy.kt 52b072a10b7c1ec2797b055cf566660ea1886c7a200b6d07e46affb3b06713e6
HostClient.kt c5db7de3cfaad0b340017e1ba7adfc84e7c2682264cd4755477f7a79f3565fe0
WorkspaceDispatchPolicy.kt 79628cde9af34573b56203e8a3c7f0d8af5b95af2069eba8e5d0f79b48fe7770
PairingViewModel.kt fe87ee11404de9984752476a121c656c111df3155f5808664268a7613c04640f
MainActivity.kt 6638465e01d7eaf88eeadeeebe9a00210ef42e956587077c721c48cc20cdf123
final-source-log 021ab496735aa10642984947aaa5ab107ad5a620e4b16b7fd79fb99a34a273bb
python-log b7441bb00e1cc534b56e47a58cacba6ed22794c1946ba50efeff92e1e7fdcfe2
host-test-log f12ba588bb3b7bb7835dac68eb7eccb33c3635b8c463b844775a9af788c44fc4
main.jar d218f379873827a239a2382e94995bd0273c1d121d0c4497af8916f84e6261f2
unit.jar f2c7d769ed04c1885bb823aed57fcd3bed2f3d424ae4337868df872aab48d157
instrumented.jar 3498d342b21fe01cb319ac0188914bba21625d72f029aa848cc745ca98375b6a
debug-host-receipt 81cb2b8628c75f28070d54248b149bc1f56378ce40bfa2301e968ff5a6588419
```

Goal active and incomplete. Current APK, physical QR/direct-LAN bidirectional
sync and recovery, connected API31/34/35 lifecycle, complete desktop parity
and signed reproducible release remain required. Source/coordinator fixtures
do not replace physical Android Keystore, listener or guarded tool-effect gates.

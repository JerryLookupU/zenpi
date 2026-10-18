# Packaged Android Backup Rules

Provisional S2-056/S2-071 progress on 2026-10-04. Previous goal turn changed
connection signing fences and verified source/policy behavior, so it was
progress. Current execution skill, dirty worktree, Blueprint, release checker,
Gradle signing policy, source backup rules and SDK inspection tools inspected.
No workers/nested agents admitted; no authoritative checkbox changed.

## Implementation

Release inspection previously verified only the merged manifest's backup rule
references and correctly left backup_rule_contents_verified=false. It could
not prove those references resolved to the intended complete exclusions.

The inspector now binds each manifest reference to exactly one named compiled
XML resource ID using AAPT2's no-values resource inventory. APK Analyzer checks
the XML configurations and resolves the exact APK path before decoding its
actual compiled XML. Legacy backup, cloud backup and device transfer must each
exclude all nine data domains at path '.', without includes, partial/duplicate
domains or overrides. The current package format permits only default XML
configuration; other configurations fail closed rather than leaving an
uninspected Android-selected override. Command hashes and decoded XML hashes
are included in the single-artifact inspection receipt.

Receipt schema is stage2-release-inspection/v2. Backup contents are marked
verified only after the complete unchanged signed-artifact inspection passes.
Source attribution, install, upgrade and reproducibility remain false. This
does not prove runtime backup/transfer, signing-key wipe or release acceptance.
No release key was generated or accessed and no application was installed.

## Real Tool Fixture

Read-only SDK calls confirmed APK Analyzer's compiled manifest reference format
(@ref/0x...) and resource XML command behavior. The historical app APK lacks
current backup references and is not accepted as current application evidence.

A resource-only fixture under /private/tmp/stage2-backup-resource-fixture uses
the current two backup XML files and a minimal manifest. Commands, all exit 0:

```text
/Users/mac/Library/Android/sdk/build-tools/35.0.0/aapt2 compile
  --dir /private/tmp/stage2-backup-resource-fixture/res
  -o /private/tmp/stage2-backup-resource-fixture/compiled.zip
/Users/mac/Library/Android/sdk/build-tools/35.0.0/aapt2 link
  -o /private/tmp/stage2-backup-resource-fixture/resources.apk
  --manifest /private/tmp/stage2-backup-resource-fixture/AndroidManifest.xml
  -I /Users/mac/Library/Android/sdk/platforms/android-35/android.jar
  /private/tmp/stage2-backup-resource-fixture/compiled.zip
```

Production packaged_backup helper ran with real subprocess calls against this
fixture: manifest print, aapt2 resource inventory, resources configs, each
resources value, and each resources xml. It proved nine complete exclusions
in all three branches. A separately compiled negative fixture under
/private/tmp/stage2-backup-resource-negative excludes only root in legacy rules;
the same production helper exits 1 with 'Backup branch must exclude every data
domain'. Neither fixture contains application code, signing, install or source
APK provenance; they establish real compiled-resource decoding only.

## Verification

Commands ran from /Users/mac/Github/zenpi against current dirty integration.

- python3 tools/test_stage2_android_release_check.py: exit 0; 14 pass.
  Covers exact reference identity, incomplete/duplicate/included domains,
  config/path overrides, XML errors, timeout, unavailable tool, oversized output,
  changed artifact and fail-closed receipt scope.
- python3 -m unittest discover -s tools -p 'test_stage2*.py': exit 0; 130 pass.
- Source backup policy tests 5 pass; lifecycle verifier tests 21 pass;
  Blueprint verifier tests 5 pass.
- Strict Blueprint, Rust format and git diff --check pass; 36 [ ], zero [_]/[x].
- Host verify-only attribution passes; no Rust/Kotlin/application resource
  source changed this turn. Prior Android gate: 446 JVM tests, instrumented
  compile only. It is not a fresh Gradle or connected test result.
- APK verify-only: exit 1, missing provenance. No unchanged blocked Gradle/socket
  retry, stale APK installation, ADB, physical camera or LAN acceptance run.

```text
stage2_android_release_check.py 7d18e86c69497326e1f2bdd318312611bfb4015f642d47757d4a4e3422180e5c
test_stage2_android_release_check.py db362a2d45f22750f18c8347ebcf532ed453b57163b8a3322d42ac4a4056353b
positive-resource-only.apk b389647aff1ba1f8c6c0ef59cafb7bceb2ac91753c53f8fe037964a84ac2edfc
negative-resource-only.apk aadb9cd0b3d381045fc3f1ba804bafeb90d0b119aa1a9493e91ad2d0a493c6d0
decoded-backup.xml 4d7c79acb6045b86b7aa2c04ccb35663d3ed45154b2b2dea01ca69a477eb07ff
decoded-extraction.xml 60fcfa7ac685d16e5a307e1107e68eeaaf6f56a61253470973191eea6e67b032
debug-host-receipt 91315921cde2e9261bda15641051eae65e6a2ce6d9b644be1186822d3d8e359e
debug-host-binary d9ef9b027304d65c79591e8187fd5439127f17124771d25bb8a4fcf21b1b83e4
```

Goal remains active. Current-source APK, physical QR/direct LAN bidirectional
sync/recovery, connected API lifecycle matrix, full parity and signed release
remain incomplete or unverified. Resource fixtures do not replace those gates.

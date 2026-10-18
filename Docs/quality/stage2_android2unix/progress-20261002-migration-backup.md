# Migration And Backup Protection Progress

Observed 2026-10-02T04:41:03Z. Previous goal turn: progress (foreground model
sync implementation and current tests). This turn advances S2-056/S2-030/S2-037
without completing their device/release gates. HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b; existing dirty changes preserved.
All 36 authoritative rows remain [ ]; full Android/QR/LAN/Rust desktop goal
stays active. No agents, worker admission, commits, pushes or cleanup.

## Changes

Explicit all-domain Android backup/extraction exclusions, source policy
validator plus negative tests and integration into cached Android verification.
New named Rust remote_migration target validates current owner/journal recovery
and interrupted/locked/corrupt/conflicting migration behavior. New Android
instrumented encrypted-store migration/missing-key/copy/tamper/wipe tests and
packaged backup-policy inspection; expanded legacy JVM cache migration test.
See migration.md for precise behavior and remaining acceptance requirements.

## Verification

Cwd /Users/mac/Github/zenpi; Rust ZENPI_HOME=/private/tmp/zenpi-migration.

- rtk cargo test --offline --locked --test remote_migration --test remote_subtab:
  exit 0, **16 pass** (6 new migration, 10 existing subtab).
- rtk cargo test --offline --locked --test remote_workspace --test remote_auth
  --test remote_events --test remote_protocol: exit 0, **66 pass**.
- zsh tools/stage2_android_source_check.sh
  /private/tmp/zenpi-migration-android-verified: exit 0, **165 JVM tests**;
  main/unit/instrumented Kotlin compiled, new 5 device tests compile only.
- env PYTHONPATH=tools python3 -m unittest
  tools.test_stage2_desktop_android_smoke tools.test_stage2_headless_android_smoke
  tools.test_stage2_scanner_android_smoke tools.test_validate_stage2_android_lifecycle
  tools.test_validate_stage2_android2unix_blueprint tools.test_validate_stage2_android_backup:
  exit 0, **63 pass** (4 new backup-policy methods).
- python3 tools/validate_stage2_android_backup.py: exit 0; source policy all
  9 domains excluded on legacy/cloud/transfer paths, not actual transport proof.
- /Users/mac/Library/Android/sdk/build-tools/35.0.0/aapt2 compile --dir
  android/app/src/main/res -o /private/tmp/zenpi-migration-android/resources.zip:
  exit 0. aapt2 link against API 35 android.jar with a minimal temporary
  policy-manifest.xml and resources.zip: exit 0. aapt2 dump xmltree confirms
  allowBackup=false and both resource references plus both nine-domain branches.
  Resource-only test artifact; no app classes/signing/install/launch evidence.
  Initial direct source-manifest link failed because AGP namespace normally
  supplies its package; the production manifest was not changed to work around
  it. A distinct minimal temporary manifest was used for the scoped check.
- cargo fmt --all -- --check, git diff --check, strict blueprint: exits 0.

Final hashes:

```text
main.jar fcd4be09506682f4dba2bbf1cb5c082b0a8f02106def7ecd592cd2c95461170a
unit.jar 62aaf87a23698bb3ab091b5fa189cbbdf82263ff54ae36b670bf2691e34a4c6c
instrumented.jar 889638ef4d4b8f205324458ea3e5ffd14725630e488122a2242a77f13efbe9ee
AndroidManifest.xml 96ae4a72c10b827ee3632f78317b16cf9e78f953ae82599d1844da9742aec55a
backup_rules.xml 865cae97840e81b9950d70457a05c5c1125a68364b0a83f44cd3daa030bc7e96
data_extraction_rules.xml cb029b35db0e976c087100424e16e14a6256b46962398887520e51508c3b5842
resources.zip 60b20bafc39209ef4b79b390e2c9826c7f2f738afe1094405e099135b4d7d51a
policy-resources.apk f0ff26530668f5683266b889ac733ec9fda8c5973a76722a8d253442548ae704
policy-manifest.xml 70e7974363e8d34944255d240d926c9c331d4b020ad7afec6b81dc1ce3f81810
```

Official Android documentation lookup attempted through web and curl; web
response decoding failed twice and curl DNS failed for developer.android.com.
No unverified live source quotation or current official-source claim is made.
Local API 35 resource/Kotlin compilation and source parser are scoped evidence.
Current executable APK, real camera QR/LAN, API-device matrix, actual backup/
transfer/uninstall, signed release/rollback and remaining product parity are
unfinished. Prior Gradle/adb socket restrictions were not bypassed or repeatedly
probed. Existing unrelated compiler/deprecation warnings remain.

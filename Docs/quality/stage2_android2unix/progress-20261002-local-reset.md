# Local Device Reset And Identity Recovery

Provisional S2-037/S2-038/S2-056 progress. Full Android QR/LAN/Rust-desktop
objective remains active. No Blueprint item is accepted: strict validation
still reports 36 `[ ]`, zero `[_]`, zero `[x]`. The RTK installation check was
an intervening user request, not product implementation progress. This turn
resumed the unfinished reset changes, repaired lifecycle/key boundaries and
broadened current-source verification.

## Runtime Changes

Local forgetting now requires confirmation with an explicit distinction from
host revocation. Corrupt encrypted metadata has a visible reset route. The
ViewModel drops cached state, cancels and joins its owned connection/diff jobs,
and blocks scans/incoming invites/cancel during reset. Foreground and network
callbacks update their observations without overwriting reset/error state.
Failure is retry-only, never a successful empty device or permission to pair.

A namespace-local no-backup `.reset` marker is synced before signing/AES key
removal. Store reads/writes and background snapshot/commit paths reject it,
including a new store instance after restart. Completion checks AES alias
absence and AtomicFile base/backup/new file absence before removing the fence.
Signing-key deletion also checks alias absence. Production paired refresh now
loads an existing identity instead of silently generating another key.

The marker is a fail-closed recovery fence, not remote revocation. Forgetting
unresolved requests cannot undo a desktop side effect. File-descriptor sync is
not proof of directory durability under power loss. Already-running WorkManager
network reads are not guaranteed cancelled by local reset; storage commits are
fenced. Backup/uninstall/migration acceptance and actual process-death reset
drills remain open.

## Tests And Wiring

LocalResetFilesTest has three JVM cases: new-instance intent visibility,
base/backup/new deletion checks and idempotence, marker removal failure, and
marker creation failure before destructive work. LocalResetTest has three
instrumented cases: corrupt-cache recovery with lifecycle/scan/cancel fencing,
interrupted reset blocking read/write/resume commits, and deletion failure
remaining retry-only until an explicit successful retry. PairingScreenTest
covers confirmations for both registered-device forgetting and corrupt-cache
reset. These Android tests were compiled, not executed.

ProjectionCacheTest additionally proves desktop preview, terminal output and
remote prompt streaming text are absent after cache restore, with unchanged
cursor and running status changed to unknown_outcome. This closes a previous
Compose-only assertion with a directly executed JVM regression.

Desktop smoke now includes LocalResetTest and expects 32 initial tests in
workspace-control mode (33 with Doze), or 15 without workspace control. Scanner
suite expects 13 and pairing-only suite 7 after the sixth PairingScreenTest.
Historical receipts retain their original counts/artifact hashes; these new
expected counts do not certify an old APK or any current device run.

## Verified Results

Environment: repository `/Users/mac/Github/zenpi`, HEAD
`1a8ff1acf72e0816fad503a05df2b1d18e07361b`, existing dirty worktree preserved.
Observed verification timestamp `2026-10-01T23:35:46Z` (Oct 2 local time).
Java 17, cached Kotlin compiler 1.9.24, Compose compiler 1.5.14, API 35 android.jar,
cached dependency API jars from Gradle 8.11.1 transforms. All compiler outputs
are in `/private/tmp`; no external caches were copied or modified.

- All production Android Kotlin/Compose sources compiled directly, exit 0.
  Four warnings: three pre-existing non-AutoMirrored icons and an unnecessary
  safe call. This does not validate resource merge, manifest, DEX or packaging.
- All JVM test sources compiled directly with production jar as friend module.
  JUnitCore ran all 17 test classes: **63 tests passed**, exit 0. Initial run
  before adding the transient-cache case passed 62. No Android framework jar
  was on the JUnit runtime classpath; no Keystore/UI behavior is inferred.
- All Android instrumented Kotlin/Compose test sources compiled directly,
  exit 0, with three existing warnings. Initial broad compile failed because
  the manually assembled classpath omitted ListenableFuture and friend-module
  visibility. Adding the cached ListenableFuture jar and `-Xfriend-paths`
  resolved those harness omissions; no source visibility was weakened.
- `python3 -m unittest discover -s tools -p 'test*stage2*.py'`: 44 passed.
- `python3 tools/validate_stage2_android2unix_blueprint.py --strict`: passes.
- `git diff --check`: passes.
- `./gradlew --offline :app:testDebugUnitTest` from `android/`: exit 1 before
  project execution, cannot open the external Gradle wrapper `.zip.lck` due to
  Operation not permitted. This is not an Android test failure or test pass.

Direct compiler invocation was `org.jetbrains.kotlin.cli.jvm.K2JVMCompiler`
via `/opt/homebrew/opt/openjdk@17/bin/java`, `-no-stdlib -no-reflect -jvm-target 17`,
explicit cached classpath and `-d` output below. Main/instrumented invocations
used `-Xplugin` for Compose 1.5.14. JVM/instrumented tests used
`-Xfriend-paths=/private/tmp/zenpi-android-main-local-reset-20261002.jar`.
Source lists were all `.kt` files from the respective `src/main/java`,
`src/test/java` and `src/androidTest/java` trees, discovered with `rg --files`.
JUnit invocation was `org.junit.runner.JUnitCore` over every `*Test.kt` class
in `src/test/java`, with the main/test jars, stdlib, Gson 2.11.0,
coroutines-core 1.8.1, JUnit 4.13.2 and Hamcrest 1.3 only.

This progress note is not a checksum-valid Master acceptance receipt. No fresh
APK, instrumented execution, LAN socket run, release receipt or full Rust gate
is claimed. Rust source was unchanged in this turn. No nested agent, commit or
push. All observed command handles terminalized before turn close.

## Source And Output Digests

SHA-256 at verification:

```text
Blueprint d76e70e369f11f0fd3d1905d56d79c2a89401b92215da48ad85314f390d96d97
Specification 403e6339b0318a420d93600fc52926acf6a9e46ec899bd9a9edb03c235190a28
PairingViewModel.kt 2d0e590c291bbb51d9698512d1171a5696f054e3de89e2bfa6461bfef8217f3c
LocalResetFiles.kt 988058c20b8600b4dfcaa994ce9215959232260510fc68a2984aa8bc96d13717
PairedHostStore.kt 3a0211a1608df4e4cad286a335acf28193b57079d0ccfda7c1cb138261ad6fd0
DeviceIdentity.kt ff0b7e0e6df280088e7425faf027c3166d0f9720e67412b44d2428dca2a061ea
MainActivity.kt f953237873c31a24e3ede3c5cefb6073720173e3aada8ffc55c41144b59b2b0c
LocalResetFilesTest.kt 4705af4b13feb95274df21c27db07c9e2fcc56c83aebe17bfe26f0690c483a81
LocalResetTest.kt 618cb390a4d580bff905079eb0dde904ed856cddce3d668fddd49173f19c3fae
PairingScreenTest.kt a54ea319c74418ffe33d115484e65364514d7f90de31236fc1c7b5112b309cf3
ProjectionCacheTest.kt 384a443335e635cfe88eed72cc06573e569163887c0a0a5146151653501af099
/private/tmp/zenpi-android-main-local-reset-20261002.jar 4fcd20299d6c82c822332ea6cd0cdd9fbde1abf9cd37be40042e2ba1c6f7d19f
/private/tmp/zenpi-android-all-jvmtests-20261002.jar 3e516aa9a8bb2c7ac5e05a52252a85f5792d1186e6d3f2ff9df09ae53fcf2c86
/private/tmp/zenpi-android-all-instrumentation-20261002.jar 31c7ea4456773756c5f1590e463aacf31e214d23ba7d34c3fd44dc2490ea3e34
```

Next acceptance requires a freshly rebuilt APK and supported-API execution of
reset/Keystore/UI tests, interrupted reset across actual processes, followed by
the existing authenticated desktop/LAN lifecycle harness. Physical phone camera
QR/LAN, live subscriptions, executable worktree concurrency, full PTY/bulk
interaction, migration/uninstall and signed release/security gates are still
requirements of the unchanged objective.

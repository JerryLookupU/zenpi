#!/bin/zsh
# Cached-source verification only; not Gradle, APK packaging or device execution.
set -eu
setopt null_glob
ROOT=${0:A:h:h}
cd "$ROOT"
python3 tools/generate_stage2_android_schema.py --check
python3 tools/validate_stage2_android_backup.py
python3 tools/validate_stage2_android_entry.py
CACHE=${GRADLE_USER_HOME:-$HOME/.gradle}/caches
SDK=${ANDROID_HOME:-$HOME/Library/Android/sdk}
JAVA=${JAVA_HOME:-/opt/homebrew/opt/openjdk@17}/bin/java
OUT=${1:?Usage: stage2_android_source_check.sh OUTPUT_DIRECTORY}
mkdir -p "$OUT"
OUT=${OUT:A}
scanner=("$CACHE"/modules-2/files-2.1/com.journeyapps/zxing-android-embedded/4.3.0/*/*.aar)
[[ ${#scanner} == 1 ]]
unzip -p "${scanner[1]}" classes.jar > "$OUT/scanner.jar"
cargo run --locked --offline --example remote_qr_fixture -- "$OUT/terminal-qr"
ZENPI_ANDROID_INTEROP_OUTPUT="$OUT/android-sync-wire.json" cargo test --locked --offline --test remote_android_interop
compiler=("$CACHE"/modules-2/files-2.1/org.jetbrains.kotlin/kotlin-compiler-embeddable/1.9.24/*/*.jar)
plugin=("$CACHE"/modules-2/files-2.1/androidx.compose.compiler/compiler/1.5.14/*/*.jar)
stdlib=("$CACHE"/modules-2/files-2.1/org.jetbrains.kotlin/kotlin-stdlib/1.9.24/*/*.jar)
gson=("$CACHE"/modules-2/files-2.1/com.google.code.gson/gson/2.11.0/*/*.jar)
coroutines=("$CACHE"/modules-2/files-2.1/org.jetbrains.kotlinx/kotlinx-coroutines-core-jvm/1.8.1/*/*.jar)
junit=("$CACHE"/modules-2/files-2.1/junit/junit/4.13.2/*/*.jar)
hamcrest=("$CACHE"/modules-2/files-2.1/org.hamcrest/hamcrest-core/1.3/*/*.jar)
api=("$CACHE"/8.11.1/transforms/*/transformed/*-api.jar)
future=("$CACHE"/modules-2/files-2.1/com.google.guava/listenablefuture/1.0/*/*.jar)
[[ -x "$JAVA" && -f "$SDK/platforms/android-35/android.jar" ]]
[[ ${#compiler} == 1 && ${#plugin} == 1 && ${#stdlib} == 1 && ${#gson} == 1 && ${#coroutines} == 1 && ${#junit} == 1 && ${#hamcrest} == 1 && ${#future} == 1 && ${#api} -gt 0 ]]
runtime=("${stdlib[@]}" "${gson[@]}" "${coroutines[@]}" "${junit[@]}" "${hamcrest[@]}")
compiler_cp=("${compiler[@]}" "${stdlib[@]}" "$CACHE"/modules-2/files-2.1/org.jetbrains.intellij.deps/trove4j/*/*/*.jar
    "$CACHE"/modules-2/files-2.1/org.jetbrains.kotlin/kotlin-script-runtime/1.9.24/*/*.jar
    "$CACHE"/modules-2/files-2.1/org.jetbrains/annotations/13.0/*/*.jar)
deps=("$SDK/platforms/android-35/android.jar" "${api[@]}" "${runtime[@]}" "${future[@]}"
    "$CACHE"/modules-2/files-2.1/org.jetbrains.kotlinx/kotlinx-coroutines-android/1.8.1/*/*.jar
    "$CACHE"/modules-2/files-2.1/com.google.zxing/core/3.4.1/*/*.jar
    "$CACHE"/modules-2/files-2.1/androidx.lifecycle/lifecycle-common-jvm/2.8.4/*/*.jar
    "$CACHE"/modules-2/files-2.1/androidx.annotation/annotation-jvm/1.8.1/*/*.jar)
main_sources=("${(@f)$(rg --files android/app/src/main/java -g '*.kt')}")
test_sources=("${(@f)$(rg --files android/app/src/test/java -g '*.kt')}")
device_sources=("${(@f)$(rg --files android/app/src/androidTest/java -g '*.kt')}")
compile() { "$JAVA" -cp "${(j.:.)compiler_cp}" org.jetbrains.kotlin.cli.jvm.K2JVMCompiler -no-stdlib -no-reflect -jvm-target 17 "$@"; }
compile -Xplugin="${plugin[1]}" -classpath "${(j.:.)deps}" -d "$OUT/main.jar" "${main_sources[@]}"
test_deps=("$OUT/main.jar" "${deps[@]}")
compile -Xfriend-paths="$OUT/main.jar" -classpath "${(j.:.)test_deps}" -d "$OUT/unit.jar" "${test_sources[@]}"
compile -Xfriend-paths="$OUT/main.jar" -Xplugin="${plugin[1]}" -classpath "${(j.:.)test_deps}" -d "$OUT/instrumented.jar" "${device_sources[@]}"
test_runtime=("$OUT/unit.jar" "$OUT/main.jar" "$OUT/scanner.jar" "$ROOT/android/app/src/test/resources" "$ROOT/fuzz/remote_protocol" "${runtime[@]}"
    "$CACHE"/modules-2/files-2.1/com.google.zxing/core/3.4.1/*/*.jar)
test_classes=("${(@f)$(printf '%s\n' "${test_sources[@]}" | sed 's|.*/|zenpi.|; s|\.kt$||')}")
test_classes=("${(@)test_classes:#zenpi.AndroidRustWriteTest}")
test_classes=("${(@)test_classes:#zenpi.AndroidRustProofTest}")
test_classes=("${(@)test_classes:#zenpi.AndroidRustWorktreeTest}")
test_classes=("${(@)test_classes:#zenpi.AndroidRustSessionTest}")
test_classes=("${(@)test_classes:#zenpi.AndroidRustAckTest}")
test_classes=("${(@)test_classes:#zenpi.AndroidRustApprovalTest}")
test_classes=("${(@)test_classes:#zenpi.AndroidRustTerminalTest}")
test_classes=("${(@)test_classes:#zenpi.AndroidRustWorkspaceTest}")
test_classes=("${(@)test_classes:#zenpi.AndroidRustModelTest}")
test_classes=("${(@)test_classes:#zenpi.AndroidRustResourceTest}")
test_classes=("${(@)test_classes:#zenpi.AndroidRustFileTest}")
test_classes=("${(@)test_classes:#zenpi.AndroidRustDiffTest}")
test_classes=("${(@)test_classes:#zenpi.AndroidRustHistoryTest}")
test_classes=("${(@)test_classes:#zenpi.AndroidRustWorkerReadTest}")
test_classes=("${(@)test_classes:#zenpi.AndroidRustWorkerToolTest}")
test_classes=("${(@)test_classes:#zenpi.AndroidRustPromptTest}")
test_classes=("${(@)test_classes:#zenpi.AndroidRustWorkerCancelTest}")
"$JAVA" -Dzenpi.interop.fixture="$OUT/android-sync-wire.json" -Dzenpi.qr.fixture="$OUT/terminal-qr" \
    -cp "${(j.:.)test_runtime}" org.junit.runner.JUnitCore "${test_classes[@]}"
ZENPI_ANDROID_INTEROP_JAVA="$JAVA" ZENPI_ANDROID_INTEROP_CLASSPATH="${(j.:.)test_runtime}" \
    ZENPI_ANDROID_WRITE_OUTPUT="$OUT/android-write-results.json" \
    cargo test --locked --offline --test remote_android_interop android_generated_rename_roundtrips_through_the_current_host -- --ignored --exact
ZENPI_ANDROID_INTEROP_JAVA="$JAVA" ZENPI_ANDROID_INTEROP_CLASSPATH="${(j.:.)test_runtime}" \
    cargo test --locked --offline --test remote_android_interop android_p256_pair_and_challenge_proofs_are_verified_by_rust -- --ignored --exact
ZENPI_ANDROID_INTEROP_JAVA="$JAVA" ZENPI_ANDROID_INTEROP_CLASSPATH="${(j.:.)test_runtime}" \
    ZENPI_ANDROID_WORKTREE_OUTPUT="$OUT/android-worktree-results.json" \
    cargo test --locked --offline --test remote_android_worktree_interop -- --ignored
ZENPI_ANDROID_INTEROP_JAVA="$JAVA" ZENPI_ANDROID_INTEROP_CLASSPATH="${(j.:.)test_runtime}" \
    ZENPI_ANDROID_SESSION_OUTPUT="$OUT/android-session-results.json" \
    cargo test --locked --offline --test remote_android_session_interop -- --ignored
ZENPI_ANDROID_INTEROP_JAVA="$JAVA" ZENPI_ANDROID_INTEROP_CLASSPATH="${(j.:.)test_runtime}" \
    ZENPI_ANDROID_ACK_OUTPUT="$OUT/android-ack-results.json" \
    cargo test --locked --offline --test remote_android_ack_interop -- --ignored
ZENPI_ANDROID_INTEROP_JAVA="$JAVA" ZENPI_ANDROID_INTEROP_CLASSPATH="${(j.:.)test_runtime}" \
    cargo test --locked --offline --test remote_android_approval_interop -- --ignored
ZENPI_ANDROID_INTEROP_JAVA="$JAVA" ZENPI_ANDROID_INTEROP_CLASSPATH="${(j.:.)test_runtime}" \
    ZENPI_ANDROID_TERMINAL_OUTPUT="$OUT/android-terminal-results.json" \
    cargo test --locked --offline --test remote_android_terminal_interop -- --ignored
ZENPI_ANDROID_INTEROP_JAVA="$JAVA" ZENPI_ANDROID_INTEROP_CLASSPATH="${(j.:.)test_runtime}" \
    ZENPI_HOME="$OUT/workspace-home" \
    ZENPI_ANDROID_WORKSPACE_OUTPUT="$OUT/android-workspace-results.json" \
    cargo test --locked --offline --test remote_android_workspace_interop -- --ignored
ZENPI_ANDROID_INTEROP_JAVA="$JAVA" ZENPI_ANDROID_INTEROP_CLASSPATH="${(j.:.)test_runtime}" \
    ZENPI_HOME="$OUT/model-home" \
    ZENPI_ANDROID_MODEL_OUTPUT="$OUT/android-model-results.json" \
    ZENPI_ANDROID_RESOURCE_OUTPUT="$OUT/android-resource-results.json" \
    ZENPI_ANDROID_FILE_OUTPUT="$OUT/android-file-results.json" \
    ZENPI_ANDROID_DIFF_OUTPUT="$OUT/android-diff-results.json" \
    ZENPI_ANDROID_HISTORY_OUTPUT="$OUT/android-history-results.json" \
    ZENPI_ANDROID_WORKER_READ_OUTPUT="$OUT/android-worker-read-results.json" \
    cargo test --locked --offline --test remote_android_model_interop -- --ignored
ZENPI_ANDROID_INTEROP_JAVA="$JAVA" ZENPI_ANDROID_INTEROP_CLASSPATH="${(j.:.)test_runtime}" \
    ZENPI_ANDROID_WORKER_TOOL_OUTPUT="$OUT/android-worker-tool-results.json" \
    ZENPI_HOME="$OUT/worker-tool-home" \
    cargo test --locked --offline --lib remote::worktree_host::tests::android_reads_production_multi_page_tool_loop -- --ignored --exact
shasum -a 256 "$OUT/main.jar" "$OUT/unit.jar" "$OUT/instrumented.jar"
ZENPI_ANDROID_INTEROP_JAVA="$JAVA" ZENPI_ANDROID_INTEROP_CLASSPATH="${(j.:.)test_runtime}" \
    ZENPI_HOME="$OUT/prompt-home" ZENPI_ANDROID_PROMPT_OUTPUT="$OUT/android-prompt-results.json" \
    cargo test --locked --offline --test remote_headless_commands android_generated_prompt_and_cancel_drive_real_owner_once -- --ignored --exact
ZENPI_ANDROID_INTEROP_JAVA="$JAVA" ZENPI_ANDROID_INTEROP_CLASSPATH="${(j.:.)test_runtime}" \
    ZENPI_HOME="$OUT/worker-cancel-home" ZENPI_ANDROID_WORKER_CANCEL_OUTPUT="$OUT/android-worker-cancel-results.json" \
    cargo test --locked --offline --lib remote::worktree_host::tests::android_cancel_frame_stops_one_running_worker_and_restores_receipt -- --ignored --exact

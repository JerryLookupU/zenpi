#!/usr/bin/env bash
# Compile the Android JVM unit-test classes with Gradle and expose them to the
# `#[ignore]`d Rust<->Kotlin interop tests (tests/remote_android_*interop.rs).
#
#   tools/build_android_interop_classes.sh          # build, print env exports
#   tools/build_android_interop_classes.sh --run    # build, then run the 8 suites
#
# On GitHub Actions the two ZENPI_ANDROID_INTEROP_* variables are also appended
# to $GITHUB_ENV so later steps can call `cargo test -- --ignored` directly.
set -euo pipefail

ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
run=0
case "${1:-}" in
    "") ;;
    --run) run=1 ;;
    *) echo "usage: $0 [--run]" >&2; exit 2 ;;
esac

if [[ -n "${JAVA_HOME:-}" ]]; then
    java="$JAVA_HOME/bin/java"
else
    java=$(command -v java)
fi
[[ -x "$java" ]] || { echo "java not found; set JAVA_HOME to a JDK 17" >&2; exit 1; }

(cd "$ROOT/android" && ./gradlew --console=plain :app:writeInteropClasspath)
classpath=$(cat "$ROOT/android/app/build/interop/classpath.txt")
[[ -n "$classpath" ]] || { echo "empty interop classpath" >&2; exit 1; }

export ZENPI_ANDROID_INTEROP_JAVA="$java"
export ZENPI_ANDROID_INTEROP_CLASSPATH="$classpath"
if [[ -n "${GITHUB_ENV:-}" ]]; then
    {
        echo "ZENPI_ANDROID_INTEROP_JAVA=$java"
        echo "ZENPI_ANDROID_INTEROP_CLASSPATH=$classpath"
    } >> "$GITHUB_ENV"
fi

if [[ $run -eq 0 ]]; then
    printf 'export ZENPI_ANDROID_INTEROP_JAVA=%q\n' "$java"
    printf 'export ZENPI_ANDROID_INTEROP_CLASSPATH=%q\n' "$classpath"
    exit 0
fi

home=$(mktemp -d)
trap 'rm -rf "$home"' EXIT
cd "$ROOT"
for suite in ack approval session terminal workspace worktree model ""; do
    test=remote_android${suite:+_$suite}_interop
    echo "== $test"
    mkdir -p "$home/$test"
    ZENPI_HOME="$home/$test" cargo test --locked --test "$test" -- --ignored --test-threads=1
done

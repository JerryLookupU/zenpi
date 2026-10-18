#!/usr/bin/env python3
"""Build fresh device-test APKs and bind them to the current source inventory."""
from __future__ import annotations

import argparse
import fcntl
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time
from generate_stage2_android_schema import generate as verify_schema

ROOT = Path(__file__).resolve().parents[1]
PROVENANCE = Path("android/app/build/stage2-device-build.json")
ATTEMPT = Path("android/app/build/stage2-device-build-attempt.json")
APKS = (
    Path("android/app/build/outputs/apk/debug/app-debug.apk"),
    Path("android/app/build/outputs/apk/androidTest/debug/app-debug-androidTest.apk"),
)
BUILD_ARGV = ("./gradlew", "--offline", "--no-daemon", "--no-build-cache", "--rerun-tasks",
              ":app:assembleDebug", ":app:assembleDebugAndroidTest")


def sha256(path: Path) -> str:
    if path.is_symlink() or not path.is_file():
        raise ValueError(f"Expected regular build input/artifact: {path}")
    with path.open("rb") as source:
        digest = hashlib.file_digest(source, "sha256").hexdigest()
    return digest


def inputs(root: Path) -> dict[str, str]:
    """Include source additions/deletions, excluding local build/SDK/signing state."""
    roots = (root / "android", root / "fuzz/remote_protocol")
    files = [root / "tools/stage2_android_build.py", root / "tools/generate_stage2_android_schema.py",
             root / "tools/validate_stage2_android_entry.py"]
    for directory in roots:
        if directory.is_symlink() or not directory.is_dir():
            raise ValueError(f"Missing regular build source directory: {directory}")
        def unreadable(error):
            raise ValueError("Unreadable Android build input directory") from error
        for base, directories, names in os.walk(directory, followlinks=False, onerror=unreadable):
            base = Path(base)
            directories[:] = sorted(name for name in directories
                if not (base == root / "android" and name in {"build", ".gradle", ".git", ".idea"})
                and not (base == root / "android/app" and name == "build"))
            for name in directories:
                if (base / name).is_symlink():
                    raise ValueError("Symlink in Android build inputs")
            for name in sorted(names):
                if base == root / "android" and (name == "local.properties" or name.endswith((".jks", ".keystore", ".log", ".iml"))):
                    continue
                files.append(base / name)
    return {path.relative_to(root).as_posix(): sha256(path) for path in sorted(files)}


def inventory_digest(inventory: dict[str, str]) -> str:
    return hashlib.sha256(json.dumps(inventory, sort_keys=True, separators=(",", ":")).encode()).hexdigest()


def failure_diagnosis(stdout: bytes, stderr: bytes) -> str:
    combined = stdout + b"\n" + stderr
    denied = any(marker in combined for marker in (b"Operation not permitted", b"Permission denied"))
    if denied and b"Unable to start the daemon process" in combined and (
            b"DaemonTcpServerConnector" in combined or b"TcpIncomingConnector.accept" in combined):
        return "Gradle daemon local TCP listener permission denied (not a Kotlin compiler/resource failure)"
    if denied and b"gradle-wrapper" in combined and b".zip.lck" in combined:
        return "Gradle wrapper cache lock permission denied"
    for marker in (b"Operation not permitted", b"Permission denied", b"JAVA_HOME", b"SDK location not found"):
        if marker in combined:
            return marker.decode()
    return ""


def record_attempt(root: Path, inventory: dict[str, str], started: float,
                   stdout: bytes, stderr: bytes, exit_code: int | None, timed_out: bool) -> Path:
    directory = root / ATTEMPT.parent
    logs = {}
    # Private local logs are diagnostic artifacts, never APK acceptance authority.
    for stream, content in (("stdout", stdout), ("stderr", stderr)):
        with tempfile.NamedTemporaryFile(dir=directory, prefix="stage2-gradle-", suffix=f"-{stream}.log", delete=False) as temporary:
            temporary_path = Path(temporary.name)
            temporary.write(content)
            temporary.flush()
            os.fsync(temporary.fileno())
        retained = temporary_path.with_suffix(".retained.log")
        try:
            os.replace(temporary_path, retained)
        finally:
            temporary_path.unlink(missing_ok=True)
        logs[stream] = {"path": retained.relative_to(root).as_posix(),
                        "sha256": hashlib.sha256(content).hexdigest(), "bytes": len(content)}
    attempt = {"schema_version": "stage2-device-build-attempt/v1", "passed": False,
               "argv": list(BUILD_ARGV), "cwd": "android", "started_at_unix": started,
               "finished_at_unix": time.time(), "exit_code": exit_code, "timed_out": timed_out,
               "inputs_sha256": inventory_digest(inventory), "logs": logs,
               "scope": "local Gradle diagnostics only; not APK provenance, device or release acceptance"}
    with tempfile.NamedTemporaryFile(mode="w", dir=directory, delete=False) as temporary:
        temporary_path = Path(temporary.name)
        json.dump(attempt, temporary, sort_keys=True, indent=2)
        temporary.write("\n")
        temporary.flush()
        os.fsync(temporary.fileno())
    try:
        os.replace(temporary_path, root / ATTEMPT)
    finally:
        temporary_path.unlink(missing_ok=True)
    return root / ATTEMPT


def require_current_build(root: Path = ROOT) -> dict:
    path = root / PROVENANCE
    if path.is_symlink() or not path.is_file():
        raise ValueError("Missing APK build provenance; run python3 tools/stage2_android_build.py")
    if path.stat().st_size > 1024 * 1024:
        raise ValueError("APK build provenance exceeds limit")
    receipt = json.loads(path.read_text())
    if (not isinstance(receipt, dict) or receipt.get("schema_version") != "stage2-device-build/v1"
            or receipt.get("passed") is not True or type(receipt.get("exit_code")) is not int or receipt.get("exit_code") != 0
            or receipt.get("argv") != list(BUILD_ARGV)):
        raise ValueError("APK build did not finish successfully with the required Gradle tasks")
    verify_schema(root, check=True)
    current = inputs(root)
    if receipt.get("inputs") != current or receipt.get("inputs_sha256") != inventory_digest(current):
        raise ValueError("APK source inventory is stale; rebuild before device validation")
    expected = {path.as_posix(): sha256(root / path) for path in APKS}
    if receipt.get("artifacts") != expected:
        raise ValueError("APK artifacts changed since the verified build")
    return {"path": PROVENANCE.as_posix(), "sha256": sha256(path),
            "inputs_sha256": inventory_digest(current), "artifacts": expected}


def build(root: Path = ROOT, timeout: int = 600) -> dict:
    output = root / PROVENANCE
    output.parent.mkdir(parents=True, exist_ok=True)
    with (output.parent / "stage2-device-build.lock").open("a") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        before = inputs(root)
        started = time.time()
        # Remove only this tool's previous authority before starting another build.
        output.unlink(missing_ok=True)
        verify_schema(root, check=True)
        try:
            result = subprocess.run(BUILD_ARGV, cwd=root / "android", capture_output=True, timeout=timeout)
        except subprocess.TimeoutExpired as error:
            attempt = record_attempt(root, before, started, error.stdout or b"", error.stderr or b"", None, True)
            error.add_note(f"Gradle diagnostics: {attempt}; no provenance published")
            raise
        attempt = record_attempt(root, before, started, result.stdout, result.stderr, result.returncode, False)
        if result.returncode:
            reason = failure_diagnosis(result.stdout, result.stderr)
            diagnosis = "; diagnostic: " + reason if reason else ""
            raise RuntimeError(f"Gradle APK build failed (exit {result.returncode}){diagnosis}; diagnostics: {attempt}; no provenance published")
        after = inputs(root)
        if before != after:
            raise RuntimeError(f"Build inputs changed during Gradle execution; diagnostics: {attempt}; no provenance published")
        verify_schema(root, check=True)
        artifacts = {path.as_posix(): sha256(root / path) for path in APKS}
        receipt = {"schema_version": "stage2-device-build/v1", "passed": True, "exit_code": 0,
                   "argv": list(BUILD_ARGV), "cwd": "android", "started_at_unix": started,
                   "finished_at_unix": time.time(), "inputs": after,
                   "inputs_sha256": inventory_digest(after), "artifacts": artifacts,
                   "stdout_sha256": hashlib.sha256(result.stdout).hexdigest(),
                   "stderr_sha256": hashlib.sha256(result.stderr).hexdigest(),
                   "scope": "fresh debug/device-test APK source attribution, not release reproducibility or device acceptance"}
        with tempfile.NamedTemporaryFile(mode="w", dir=output.parent, delete=False) as temporary:
            temporary_path = Path(temporary.name)
            json.dump(receipt, temporary, sort_keys=True, indent=2)
            temporary.write("\n")
            temporary.flush()
            os.fsync(temporary.fileno())
        try:
            os.replace(temporary_path, output)
        finally:
            temporary_path.unlink(missing_ok=True)
        return require_current_build(root)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--verify-only", action="store_true", help="Refuse stale/missing APKs without building or contacting adb")
    args = parser.parse_args()
    try:
        evidence = require_current_build() if args.verify_only else build()
    except (ValueError, RuntimeError, OSError, subprocess.TimeoutExpired) as error:
        for note in getattr(error, "__notes__", ()):
            print(note, file=sys.stderr)
        raise SystemExit(str(error)) from error
    print(json.dumps(evidence, indent=2))


if __name__ == "__main__":
    main()

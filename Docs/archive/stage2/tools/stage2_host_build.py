#!/usr/bin/env python3
"""Bind a Cargo-built host binary to the current Rust source inventory."""
import argparse
import fcntl
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import time

from stage2_android_build import ROOT, sha256, inventory_digest


def inputs(root):
    files = [root / name for name in ("Cargo.toml", "Cargo.lock", "tools/stage2_host_build.py", "tools/stage2_android_build.py")]
    for name in ("build.rs", "rust-toolchain", "rust-toolchain.toml"):
        path = root / name
        if path.exists() or path.is_symlink(): files.append(path)
    for name in ("src", "vendor", ".cargo"):
        directory = root / name
        if directory.is_symlink(): raise ValueError("Symlink in host source inventory")
        if not directory.exists():
            if name == "src": raise ValueError("Missing host sources")
            continue
        def unreadable(error): raise ValueError("Unreadable host source directory") from error
        for base, directories, names in os.walk(directory, followlinks=False, onerror=unreadable):
            base = Path(base)
            for child in directories:
                if (base / child).is_symlink(): raise ValueError("Symlink in host source inventory")
            files.extend(base / child for child in names)
    return {path.relative_to(root).as_posix(): sha256(path) for path in sorted(files)}


def paths(root, profile):
    if profile not in ("debug", "release"): raise ValueError("Unsupported host profile")
    return root / f"target/stage2-host-build-{profile}.json", root / f"target/{profile}/zenpi"


def argv(profile):
    return ["cargo", "build", "--offline", "--locked", "--bin", "zenpi", "--message-format=json"] + (["--release"] if profile == "release" else [])


def fresh_artifact(receipt, root, profile):
    directory = receipt.get("fresh_target")
    if not isinstance(directory, str): raise ValueError("Missing fresh host target")
    target = Path(directory)
    if target.parent != root / "target" or not target.name.startswith("stage2-fresh-host-") or target.is_symlink():
        raise ValueError("Invalid fresh host target")
    artifact = target / profile / "zenpi"
    if (target / profile).is_symlink(): raise ValueError("Symlink fresh host profile")
    if receipt.get("argv") != argv(profile) + ["--target-dir", str(target)]:
        raise ValueError("Invalid fresh host build command")
    if receipt.get("fresh_artifact_sha256") != sha256(artifact):
        raise ValueError("Fresh host artifact changed")
    return artifact


def require_current_host(binary, root=ROOT):
    binary = Path(binary)
    if not binary.is_absolute(): binary = root / binary
    if binary.is_symlink(): raise ValueError("Symlink host binary")
    selected = next((profile for profile in ("debug", "release") if binary.absolute() == paths(root, profile)[1].absolute()), None)
    if selected is None: raise ValueError("Host binary must use the attributed target/debug or target/release path")
    receipt_path, expected = paths(root, selected)
    if receipt_path.is_symlink() or not receipt_path.is_file(): raise ValueError("Missing host build provenance; run python3 tools/stage2_host_build.py")
    if receipt_path.stat().st_size > 2 * 1024 * 1024: raise ValueError("Host build receipt exceeds limit")
    receipt = json.loads(receipt_path.read_text())
    if not isinstance(receipt, dict) or receipt.get("schema_version") not in ("stage2-host-build/v1", "stage2-host-build/v2") \
        or receipt.get("passed") is not True or type(receipt.get("exit_code")) is not int or receipt.get("exit_code") != 0:
        raise ValueError("Host build did not succeed with the required Cargo command")
    if receipt["schema_version"] == "stage2-host-build/v2":
        fresh_artifact(receipt, root, selected)
        if receipt["fresh_artifact_sha256"] != sha256(expected): raise ValueError("Published host differs from fresh build")
    elif receipt.get("argv") != argv(selected):
        raise ValueError("Host build did not succeed with the required Cargo command")
    current = inputs(root)
    if receipt.get("inputs") != current or receipt.get("inputs_sha256") != inventory_digest(current):
        raise ValueError("Host source inventory is stale; rebuild before device validation")
    if receipt.get("artifact") != {"path": expected.relative_to(root).as_posix(), "sha256": sha256(expected)}:
        raise ValueError("Host binary changed since its attributed build")
    return {"path": receipt_path.relative_to(root).as_posix(), "sha256": sha256(receipt_path),
            "inputs_sha256": inventory_digest(current), "artifact": receipt["artifact"]}


def build(root=ROOT, profile="debug", timeout=600, fresh=False):
    root = Path(root).absolute()
    receipt_path, binary = paths(root, profile)
    receipt_path.parent.mkdir(parents=True, exist_ok=True)
    with (receipt_path.parent / "stage2-host-build.lock").open("a") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        before = inputs(root); started = time.time()
        receipt_path.unlink(missing_ok=True)
        environment = {name: value for name, value in os.environ.items()
                       if name in {"PATH", "HOME", "TMPDIR", "LANG", "LC_ALL", "LC_CTYPE"}}
        target = Path(tempfile.mkdtemp(prefix="stage2-fresh-host-", dir=root / "target")) if fresh else None
        command = argv(profile) + (["--target-dir", str(target)] if target is not None else [])
        built = target / profile / "zenpi" if target is not None else binary
        result = subprocess.run(command, cwd=root, env=environment, capture_output=True, timeout=timeout)
        if result.returncode: raise RuntimeError(f"Cargo host build failed (exit {result.returncode}); no provenance published")
        artifacts = []
        for line in result.stdout.splitlines():
            message = json.loads(line)
            if message.get("reason") == "compiler-artifact" and message.get("target", {}).get("name") == "zenpi" \
                and "bin" in message.get("target", {}).get("kind", []) and message.get("executable"):
                artifacts.append(Path(message["executable"]))
        if artifacts != [built.absolute()]: raise ValueError("Cargo did not report exactly the expected host executable")
        after = inputs(root)
        if before != after: raise ValueError("Host sources changed during build; no provenance published")
        built_hash = sha256(built)
        if fresh:
            if binary.is_symlink(): raise ValueError("Refusing to replace symlink host binary")
            binary.parent.mkdir(parents=True, exist_ok=True)
            with tempfile.NamedTemporaryFile(dir=binary.parent, delete=False) as temporary:
                publication = Path(temporary.name)
                with built.open("rb") as source: shutil.copyfileobj(source, temporary)
                temporary.flush(); os.fsync(temporary.fileno())
            try:
                os.chmod(publication, 0o755)
                if sha256(publication) != built_hash or inputs(root) != before:
                    raise ValueError("Fresh host publication changed; no provenance published")
                os.replace(publication, binary)
            finally: publication.unlink(missing_ok=True)
        receipt = {"schema_version": "stage2-host-build/v2" if fresh else "stage2-host-build/v1", "passed": True, "exit_code": 0,
            "argv": command, "cwd": str(root), "started_at_unix": started, "finished_at_unix": time.time(),
            "inputs": after, "inputs_sha256": inventory_digest(after),
            "artifact": {"path": binary.relative_to(root).as_posix(), "sha256": sha256(binary)},
            "environment_names": sorted(environment),
            "stdout_sha256": hashlib.sha256(result.stdout).hexdigest(),
            "stderr_sha256": hashlib.sha256(result.stderr).hexdigest(),
            "scope": "host source attribution only, not reproducible release or device/LAN acceptance"}
        if fresh:
            receipt.update(fresh_target=str(target), fresh_artifact_sha256=built_hash)
        with tempfile.NamedTemporaryFile(mode="w", dir=receipt_path.parent, delete=False) as temporary:
            path = Path(temporary.name); json.dump(receipt, temporary, sort_keys=True, indent=2); temporary.write("\n")
            temporary.flush(); os.fsync(temporary.fileno())
        try: os.replace(path, receipt_path)
        finally: path.unlink(missing_ok=True)
        return require_current_host(binary, root)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--profile", choices=("debug", "release"), default="debug")
    parser.add_argument("--verify-only", action="store_true")
    parser.add_argument("--fresh", action="store_true", help="Build in an empty retained target directory and atomically publish the exact executable")
    args = parser.parse_args()
    try:
        result = require_current_host(paths(ROOT, args.profile)[1]) if args.verify_only else build(profile=args.profile, fresh=args.fresh)
        print(json.dumps(result, indent=2))
    except (ValueError, RuntimeError, OSError, subprocess.TimeoutExpired) as error: raise SystemExit(str(error)) from error


if __name__ == "__main__": main()

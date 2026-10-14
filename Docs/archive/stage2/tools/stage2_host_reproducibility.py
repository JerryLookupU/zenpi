#!/usr/bin/env python3
"""Compare two fresh local locked release builds; not runtime/release acceptance."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time

from stage2_host_build import ROOT, inputs, inventory_digest, sha256


def environment():
    return {name: value for name, value in os.environ.items()
            if name in {"PATH", "HOME", "TMPDIR", "LANG", "LC_ALL", "LC_CTYPE"}}


def inventory(root):
    return inputs(root) | {"tools/stage2_host_reproducibility.py": sha256(root / "tools/stage2_host_reproducibility.py")}


def toolchain(root, env):
    result = {}
    for command in ("rustc", "cargo"):
        completed = subprocess.run([command, "-vV"], cwd=root, env=env, capture_output=True, timeout=30)
        if completed.returncode:
            raise ValueError("Unable to identify release toolchain")
        result[command] = completed.stdout.decode("utf-8", errors="strict")
    return result


def compare(root, receipt, runtime_parent, timeout=900, secondary_runtime_parent=None):
    root, receipt, runtime_parent = Path(root).absolute(), Path(receipt).absolute(), Path(runtime_parent).absolute()
    secondary_runtime_parent = Path(secondary_runtime_parent).absolute() if secondary_runtime_parent is not None else root / "target"
    if receipt.exists() or receipt.is_symlink():
        raise ValueError("Refusing to overwrite reproducibility evidence")
    if runtime_parent.is_symlink() or secondary_runtime_parent.is_symlink():
        raise ValueError("Refusing symlink release runtime parent")
    runtime_parent.mkdir(parents=True, exist_ok=True)
    secondary_runtime_parent.mkdir(parents=True, exist_ok=True)
    runtime = Path(tempfile.mkdtemp(prefix="stage2-release-compare-", dir=runtime_parent))
    started = time.time()
    env = environment()
    before = inventory(root)
    tools = toolchain(root, env)
    lanes = []
    failure = None
    for name in ("a", "b"):
        # Compare the temporary build location with the publication tree too;
        # two siblings alone previously missed the canonical Mach-O mismatch.
        if name == "a":
            target = runtime / name
            target.mkdir()
        else:
            target = Path(tempfile.mkdtemp(prefix="stage2-release-compare-", dir=secondary_runtime_parent)) / name
            target.mkdir()
        argv = ["cargo", "build", "--offline", "--locked", "--release", "--bin", "zenpi",
                "--message-format=json", "--target-dir", str(target)]
        lane = {"name": name, "target_directory": str(target), "argv": argv, "started_at_unix": time.time()}
        try:
            completed = subprocess.run(argv, cwd=root, env=env, capture_output=True, timeout=timeout)
            lane["exit_code"] = completed.returncode
            stdout, stderr = completed.stdout, completed.stderr
        except subprocess.TimeoutExpired as error:
            lane["exit_code"] = None
            stdout, stderr = error.stdout or b"", error.stderr or b""
            failure = "release_build_timeout"
        for stream, data in (("stdout", stdout), ("stderr", stderr)):
            path = runtime / f"{name}-{stream}.log"
            with path.open("xb") as output:
                os.chmod(path, 0o600)
                output.write(data)
            lane[stream] = {"path": str(path), "sha256": sha256(path)}
        lane["finished_at_unix"] = time.time()
        lanes.append(lane)
        if failure is not None:
            break
        if lane["exit_code"] != 0:
            failure = "release_build_failed"
            break
        artifact = target / "release/zenpi"
        reported = []
        try:
            for line in stdout.splitlines():
                message = json.loads(line)
                if message.get("reason") == "compiler-artifact" and message.get("target", {}).get("name") == "zenpi" \
                        and "bin" in message.get("target", {}).get("kind", []) and message.get("executable"):
                    reported.append(Path(message["executable"]))
            if reported != [artifact]:
                raise ValueError("Unexpected release executable")
            lane["artifact"] = {"path": str(artifact), "sha256": sha256(artifact)}
            if inventory(root) != before or toolchain(root, env) != tools:
                failure = "source_or_toolchain_changed"
                break
        except (ValueError, OSError):
            failure = "release_artifact_unverified"
            break
    if failure is None and lanes[0]["artifact"]["sha256"] != lanes[1]["artifact"]["sha256"]:
        failure = "release_artifacts_differ"
    result = {"schema_version": "stage2-host-reproducibility/v1", "passed": failure is None,
              "whole_item_accepted": False, "runtime_verified": False, "cross_host_verified": False,
              "scope": "two fresh independently located target directories, same local source/toolchain/environment; not supported-host runtime or product release",
              "runtime_parents": [str(runtime_parent), str(secondary_runtime_parent)],
              "cwd": str(root), "inputs": before, "inputs_sha256": inventory_digest(before),
              "toolchain": tools, "environment_names": sorted(env), "lanes": lanes, "failure": failure,
              "started_at_unix": started, "finished_at_unix": time.time()}
    receipt.parent.mkdir(parents=True, exist_ok=True)
    with receipt.open("x") as output:
        os.chmod(receipt, 0o600)
        json.dump(result, output, indent=2)
        output.write("\n")
    return result


def verify_receipt(root, receipt, binary=None):
    root, receipt = Path(root).absolute(), Path(receipt).absolute()
    if receipt.is_symlink() or not receipt.is_file() or receipt.stat().st_size > 2 * 1024 * 1024:
        raise ValueError("Invalid release comparison receipt")
    result = json.loads(receipt.read_text())
    if not isinstance(result, dict) or result.get("schema_version") != "stage2-host-reproducibility/v1" \
            or result.get("passed") is not True or result.get("failure") is not None \
            or any(result.get(field) is not False for field in ("whole_item_accepted", "runtime_verified", "cross_host_verified")) \
            or result.get("cwd") != str(root):
        raise ValueError("Release comparison unsuccessful or overclaims acceptance")
    env = environment()
    current = inventory(root)
    if result.get("inputs") != current or result.get("inputs_sha256") != inventory_digest(current) \
            or result.get("toolchain") != toolchain(root, env) or result.get("environment_names") != sorted(env):
        raise ValueError("Release comparison source/toolchain/environment is stale")
    parents, lanes = result.get("runtime_parents"), result.get("lanes")
    if not isinstance(parents, list) or len(parents) != 2 or any(not isinstance(parent, str) for parent in parents) \
            or not isinstance(lanes, list) or len(lanes) != 2:
        raise ValueError("Missing independent release comparison lanes")
    hashes, targets = [], []
    for name, parent, lane in zip(("a", "b"), parents, lanes):
        if not isinstance(lane, dict) or lane.get("name") != name or type(lane.get("exit_code")) is not int or lane["exit_code"] != 0:
            raise ValueError("Release comparison lane failed")
        target = Path(lane.get("target_directory", ""))
        parent = Path(parent)
        if not target.is_absolute() or not parent.is_absolute() or target.name != name \
                or target.parent.parent != parent or not target.parent.name.startswith("stage2-release-compare-") \
                or any(path.is_symlink() for path in (parent, target.parent, target, target / "release")):
            raise ValueError("Invalid release comparison target")
        expected = ["cargo", "build", "--offline", "--locked", "--release", "--bin", "zenpi",
                    "--message-format=json", "--target-dir", str(target)]
        if lane.get("argv") != expected:
            raise ValueError("Unexpected release comparison command")
        artifact = target / "release/zenpi"
        artifact_hash = sha256(artifact)
        if lane.get("artifact") != {"path": str(artifact), "sha256": artifact_hash}:
            raise ValueError("Release comparison artifact changed")
        for stream in ("stdout", "stderr"):
            evidence = lane.get(stream)
            if not isinstance(evidence, dict) or not isinstance(evidence.get("path"), str):
                raise ValueError("Missing release comparison log")
            path = Path(evidence["path"])
            if path != Path(lanes[0]["target_directory"]).parent / f"{name}-{stream}.log" \
                    or evidence.get("sha256") != sha256(path):
                raise ValueError("Release comparison log changed")
        hashes.append(artifact_hash)
        targets.append(target.resolve())
    if targets[0] == targets[1] or hashes[0] != hashes[1]:
        raise ValueError("Release comparison artifacts are not independent and equal")
    if binary is not None:
        from stage2_host_build import require_current_host
        provenance = require_current_host(binary, root)
        if provenance["artifact"]["path"] != "target/release/zenpi" or provenance["artifact"]["sha256"] != hashes[0]:
            raise ValueError("Published release differs from independent builds")
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--receipt", type=Path, required=True)
    parser.add_argument("--verify-only", action="store_true", help="Recheck retained evidence against current sources/toolchain")
    parser.add_argument("--binary", type=Path, help="With --verify-only, also require the attributed release to match both builds")
    parser.add_argument("--runtime-parent", type=Path, default=Path(tempfile.gettempdir()))
    parser.add_argument("--secondary-runtime-parent", type=Path,
                        help="Second fresh build parent (default: repository target directory)")
    args = parser.parse_args()
    if args.binary is not None and not args.verify_only:
        parser.error("--binary requires --verify-only")
    if args.verify_only:
        result = verify_receipt(ROOT, args.receipt, args.binary)
        print(json.dumps({"passed": True, "inputs_sha256": result["inputs_sha256"],
                          "published_release_checked": args.binary is not None}, indent=2))
        return
    result = compare(ROOT, args.receipt, args.runtime_parent, secondary_runtime_parent=args.secondary_runtime_parent)
    print(json.dumps({key: result[key] for key in ("passed", "failure", "inputs_sha256", "lanes")}, indent=2))
    if not result["passed"]:
        raise SystemExit("Release comparison failed; preserve the receipt and logs")


if __name__ == "__main__":
    main()

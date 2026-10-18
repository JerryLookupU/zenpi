#!/usr/bin/env python3
"""Measure idle cost of headless agents (ZE-300 / ZE-403 support).

Starts N `zenpi --mode headless` processes with no input (stdin held open),
waits for warm-up, then samples per-process CPU time and context switches over
a window. Requires a binary built with `--features dev-fixtures` (echo backend).
macOS: context switches come from `top -stats csw`; elsewhere only CPU is read.
"""

from __future__ import annotations

import argparse
import json
import os
import platform
import subprocess
import sys
import tempfile
import time


def cpu_seconds(pids: list[int]) -> dict[int, float]:
    out = subprocess.run(
        ["ps", "-o", "pid=,time=", "-p", ",".join(map(str, pids))],
        capture_output=True, text=True, check=False,
    ).stdout
    result: dict[int, float] = {}
    for line in out.splitlines():
        pid, clock = line.split()
        parts = [float(p) for p in clock.replace("-", ":").split(":")]
        seconds = 0.0
        for part in parts:
            seconds = seconds * 60 + part
        result[int(pid)] = seconds
    return result


def context_switches(pids: list[int]) -> dict[int, int]:
    if platform.system() != "Darwin":
        return {}
    args = ["top", "-l", "1", "-stats", "pid,csw"]
    for pid in pids:
        args += ["-pid", str(pid)]
    out = subprocess.run(args, capture_output=True, text=True, check=False).stdout
    result: dict[int, int] = {}
    for line in out.splitlines():
        fields = line.split()
        if len(fields) == 2 and fields[0].isdigit():
            result[int(fields[0])] = int(fields[1].rstrip("+"))
    return result


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", default="target/debug/zenpi")
    parser.add_argument("--agents", type=int, default=20)
    parser.add_argument("--warmup", type=float, default=3.0)
    parser.add_argument("--window", type=float, default=10.0)
    args = parser.parse_args()
    home = tempfile.mkdtemp(prefix="zenpi-idle-")
    env = {k: v for k, v in os.environ.items() if not k.startswith("ZENPI_")}
    env["ZENPI_HOME"] = home
    procs = [
        subprocess.Popen(
            [args.binary, "--mode", "headless", "--backend", "echo", "--no-session"],
            stdin=subprocess.PIPE, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
            env=env, cwd=home,
        )
        for _ in range(args.agents)
    ]
    try:
        time.sleep(args.warmup)
        pids = [p.pid for p in procs]
        cpu0, csw0 = cpu_seconds(pids), context_switches(pids)
        time.sleep(args.window)
        cpu1, csw1 = cpu_seconds(pids), context_switches(pids)
        cpu = {pid: cpu1.get(pid, 0) - cpu0.get(pid, 0) for pid in pids}
        csw = {pid: csw1.get(pid, 0) - csw0.get(pid, 0) for pid in pids if pid in csw0}
        report = {
            "agents": args.agents,
            "window_s": args.window,
            "total_cpu_s": round(sum(cpu.values()), 3),
            "cpu_core_fraction": round(sum(cpu.values()) / args.window, 4),
            "max_csw_per_s": round(max(csw.values()) / args.window, 1) if csw else None,
            "mean_csw_per_s": round(sum(csw.values()) / len(csw) / args.window, 1) if csw else None,
        }
        print(json.dumps(report))
    finally:
        for p in procs:
            p.stdin.close()
        for p in procs:
            try:
                p.wait(timeout=10)
            except subprocess.TimeoutExpired:
                p.kill()
    return 0


if __name__ == "__main__":
    sys.exit(main())

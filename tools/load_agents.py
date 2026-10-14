#!/usr/bin/env python3
"""G-LOAD(N): run N headless zenpi agents against one scripted provider.

ZE-403. One process per agent (`zenpi --mode headless --auto`, own session),
one asyncio provider speaking Chat Completions SSE with a fixed latency, and a
single `selectors` loop reading every agent's stdout (redis-ae style). Each
task is three provider rounds: `write_file`, `bash echo`, final answer.

Per-process numbers come from macOS `proc_pidinfo` or Linux `/proc` (no root
needed): RSS, CPU time, context switches, threads and open fds. Thresholds are the §2.1
budget of Docs/Zenpi_Execution_Blueprint_v2.md; the JSON report records each
check and the kernel limits that bounded the run.
"""

from __future__ import annotations

import argparse
import asyncio
import ctypes
import json
import os
import platform
import resource
import selectors
import shutil
import statistics
import subprocess
import sys
import tempfile
import threading
import time
from pathlib import Path

# ---------------------------------------------------------------- provider


def sse_frames(model: str, delta: dict, finish: str) -> bytes:
    chunk = lambda d, f: {"id": "chat-1", "model": model, "choices": [{"index": 0, "delta": d, "finish_reason": f}]}
    usage = {"choices": [], "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}}
    body = "".join(f"data: {json.dumps(frame)}\n\n" for frame in (chunk(delta, None), chunk({}, finish), usage))
    return (body + "data: [DONE]\n\n").encode()


PROBE_PROMPT = "probe: answer without tools"


def reply_for(request: dict) -> tuple[dict, str]:
    """Round = number of tool results already in the conversation."""
    if request.get("messages", [{}])[-1].get("content") == PROBE_PROMPT:
        return {"content": "probe done"}, "stop"
    rounds = sum(1 for message in request.get("messages", []) if message.get("role") == "tool")
    if rounds == 0:
        call = {"name": "write_file", "arguments": json.dumps({"path": "out.txt", "content": "load\n"})}
    elif rounds == 1:
        call = {"name": "run_command", "arguments": json.dumps({"command": "echo load-ok"})}
    else:
        return {"content": "task complete"}, "stop"
    call_id = f"call-{rounds}"
    return {"tool_calls": [{"index": 0, "id": call_id, "type": "function", "function": call}]}, "tool_calls"


class Provider:
    def __init__(self, latency: float, probe: float):
        self.latency = latency
        # A probe prompt is held this long so every agent provably waits on
        # the provider while the harness samples it.
        self.probe = probe
        self.requests = 0
        self.port = 0
        self.ready = threading.Event()
        self.loop: asyncio.AbstractEventLoop | None = None

    async def handle(self, reader: asyncio.StreamReader, writer: asyncio.StreamWriter) -> None:
        try:
            while True:
                head = await reader.readuntil(b"\r\n\r\n")
                length = 0
                for line in head.decode("latin-1").split("\r\n"):
                    name, _, value = line.partition(":")
                    if name.lower() == "content-length":
                        length = int(value.strip())
                request = json.loads(await reader.readexactly(length))
                self.requests += 1
                probe = any(m.get("content") == PROBE_PROMPT for m in request.get("messages", [])[-1:])
                await asyncio.sleep(self.probe if probe else self.latency)
                delta, finish = reply_for(request)
                body = sse_frames(request.get("model", "load"), delta, finish)
                writer.write(
                    b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\n"
                    + f"Content-Length: {len(body)}\r\n\r\n".encode()
                    + body
                )
                await writer.drain()
        except (asyncio.IncompleteReadError, ConnectionError):
            pass
        finally:
            writer.close()

    def run(self) -> None:
        async def main() -> None:
            server = await asyncio.start_server(self.handle, "127.0.0.1", 0, backlog=4096)
            self.port = server.sockets[0].getsockname()[1]
            self.loop = asyncio.get_running_loop()
            self.ready.set()
            async with server:
                await server.serve_forever()

        asyncio.run(main())


# ----------------------------------------------------------- process stats


class TaskInfo(ctypes.Structure):
    _fields_ = [
        ("virtual_size", ctypes.c_uint64),
        ("resident_size", ctypes.c_uint64),
        ("total_user", ctypes.c_uint64),
        ("total_system", ctypes.c_uint64),
        ("threads_user", ctypes.c_uint64),
        ("threads_system", ctypes.c_uint64),
        ("policy", ctypes.c_int32),
        ("faults", ctypes.c_int32),
        ("pageins", ctypes.c_int32),
        ("cow_faults", ctypes.c_int32),
        ("messages_sent", ctypes.c_int32),
        ("messages_received", ctypes.c_int32),
        ("syscalls_mach", ctypes.c_int32),
        ("syscalls_unix", ctypes.c_int32),
        ("csw", ctypes.c_int32),
        ("threadnum", ctypes.c_int32),
        ("numrunning", ctypes.c_int32),
        ("priority", ctypes.c_int32),
    ]


LIBPROC = ctypes.CDLL("/usr/lib/libproc.dylib") if platform.system() == "Darwin" else None
PROC_PIDLISTFDS, PROC_PIDTASKINFO = 1, 4
# total_user/total_system are Mach absolute time units.
TIMEBASE = 1.0
if LIBPROC is not None:
    class _Timebase(ctypes.Structure):
        _fields_ = [("numer", ctypes.c_uint32), ("denom", ctypes.c_uint32)]
    _tb = _Timebase()
    ctypes.CDLL("/usr/lib/libSystem.dylib").mach_timebase_info(ctypes.byref(_tb))
    TIMEBASE = _tb.numer / _tb.denom


CLOCK_TICKS = os.sysconf("SC_CLK_TCK") if hasattr(os, "sysconf") else 100


def linux_stats(pid: int) -> dict | None:
    """The same numbers from /proc on Linux (CI runners)."""
    try:
        stat = Path(f"/proc/{pid}/stat").read_text().rsplit(")", 1)[1].split()
        status = dict(
            line.split(":", 1) for line in Path(f"/proc/{pid}/status").read_text().splitlines() if ":" in line
        )
        fds = len(os.listdir(f"/proc/{pid}/fd"))
    except (OSError, IndexError, ValueError):
        return None
    switches = sum(int(status.get(key, "0").strip()) for key in ("voluntary_ctxt_switches", "nonvoluntary_ctxt_switches"))
    return {
        "rss": int(status.get("VmRSS", "0 kB").split()[0]) * 1024,
        # utime and stime are fields 14 and 15; `stat` starts at field 3.
        "cpu_s": (int(stat[11]) + int(stat[12])) / CLOCK_TICKS,
        "csw": switches,
        "threads": int(status.get("Threads", "0").strip()),
        "fds": fds,
    }


def proc_stats(pid: int) -> dict | None:
    if LIBPROC is None:
        return linux_stats(pid) if Path("/proc").is_dir() else None
    info = TaskInfo()
    if LIBPROC.proc_pidinfo(pid, PROC_PIDTASKINFO, 0, ctypes.byref(info), ctypes.sizeof(info)) <= 0:
        return None
    # A null buffer returns the fd table capacity; fill a buffer for the count.
    capacity = LIBPROC.proc_pidinfo(pid, PROC_PIDLISTFDS, 0, None, 0)
    buffer = ctypes.create_string_buffer(max(capacity, 8))
    fd_bytes = LIBPROC.proc_pidinfo(pid, PROC_PIDLISTFDS, 0, buffer, len(buffer))
    return {
        "rss": info.resident_size,
        "cpu_s": (info.total_user + info.total_system) * TIMEBASE / 1e9,
        "csw": info.csw,
        "threads": info.threadnum,
        "fds": max(fd_bytes // 8, 0),
    }


def sample(pids: list[int]) -> dict[int, dict]:
    return {pid: stats for pid in pids if (stats := proc_stats(pid))}


def pct(values: list[float], q: float) -> float:
    if not values:
        return 0.0
    ordered = sorted(values)
    return ordered[min(len(ordered) - 1, int(round(q * (len(ordered) - 1))))]


def window(pids: list[int], seconds: float) -> dict:
    """CPU and wakeups over `seconds` while the agents keep their state."""
    before, start = sample(pids), time.monotonic()
    time.sleep(seconds)
    after, elapsed = sample(pids), time.monotonic() - start
    live = [pid for pid in pids if pid in before and pid in after]
    cpu = sum(after[pid]["cpu_s"] - before[pid]["cpu_s"] for pid in live)
    wakeups = [(after[pid]["csw"] - before[pid]["csw"]) / elapsed for pid in live]
    return {
        "agents_sampled": len(live),
        "machine_cpu_cores": round(cpu / elapsed, 3),
        "wakeups_per_s_p99": round(pct(wakeups, 0.99), 2),
        "wakeups_per_s_max": round(max(wakeups, default=0), 2),
        "threads_max": max((after[pid]["threads"] for pid in live), default=0),
        "fds_max": max((after[pid]["fds"] for pid in live), default=0),
        "rss_mib_p99": round(pct([after[pid]["rss"] / 2**20 for pid in live], 0.99), 2),
        "rss_gib_total": round(sum(after[pid]["rss"] for pid in live) / 2**30, 3),
    }


# ------------------------------------------------------------------ agents


def raise_fd_limit(needed: int) -> dict:
    soft, hard = resource.getrlimit(resource.RLIMIT_NOFILE)
    if soft < needed:
        target = needed if hard == resource.RLIM_INFINITY else min(needed, hard)
        resource.setrlimit(resource.RLIMIT_NOFILE, (target, hard))
        soft = target
    if soft < needed:
        sys.exit(f"fd budget {needed} exceeds the hard limit {hard}; raise `ulimit -n`")
    return {"nofile_soft": soft, "nofile_hard": "unlimited" if hard == resource.RLIM_INFINITY else hard}


def sysctl(name: str) -> str:
    try:
        return subprocess.run(["sysctl", "-n", name], capture_output=True, text=True).stdout.strip()
    except OSError:
        return "unavailable"


class Agent:
    def __init__(self, index: int, binary: str, root: Path, home: Path):
        self.index = index
        self.cwd = root / f"agent-{index}"
        self.cwd.mkdir()
        self.session = self.cwd / "session.jsonl"
        env = {"PATH": os.environ.get("PATH", ""), "HOME": str(home), "ZENPI_HOME": str(home), "SHELL": "/bin/sh"}
        self.proc = subprocess.Popen(
            [binary, "--mode", "headless", "--auto", "--session", str(self.session)],
            cwd=self.cwd,
            env=env,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
        )
        os.set_blocking(self.proc.stdout.fileno(), False)
        self.buffer = b""
        self.sent_at = 0.0
        self.done_at = 0.0
        self.success: bool | None = None

    def prompt(self, message: str = "write a file then echo", request_id: str = "task") -> None:
        frame = {"schema_version": 2, "type": "prompt", "id": request_id, "message": message}
        self.sent_at = time.monotonic()
        self.success = None
        self.proc.stdin.write((json.dumps(frame) + "\n").encode())
        self.proc.stdin.flush()

    def readable(self) -> None:
        try:
            chunk = os.read(self.proc.stdout.fileno(), 65536)
        except BlockingIOError:
            return
        if not chunk:
            if self.success is None:
                self.success, self.done_at = False, time.monotonic()
            return
        self.buffer += chunk
        while b"\n" in self.buffer:
            line, self.buffer = self.buffer.split(b"\n", 1)
            try:
                value = json.loads(line)
            except ValueError:
                continue
            if value.get("type") == "response" and value.get("id") in ("task", "probe") and self.success is None:
                self.success, self.done_at = bool(value.get("success")), time.monotonic()


def journal_costs(agents: list[Agent]) -> dict:
    """fsyncs per tool call and WAL bytes per request, from the journals.

    The journal syncs every record except authorization/tool-execution audit
    events (ZE-304), so synced records = all records minus those."""
    deferred = {"authorization_decided", "tool_execution_started", "tool_execution_finished"}
    per_call, wal_per_request = [], []
    for agent in agents:
        try:
            records = [json.loads(line) for line in agent.session.read_text().splitlines() if line.strip()]
        except (OSError, ValueError):
            continue
        tool_calls = sum(1 for r in records if r.get("kind") == "turn" and r.get("turn", {}).get("role") == "tool")
        synced = sum(1 for r in records if not (r.get("kind") == "event" and r.get("event", {}).get("type") in deferred))
        if tool_calls:
            per_call.append(synced / tool_calls)
        wal = Path(str(agent.session) + ".reconnect")
        if wal.exists():
            wal_per_request.append(wal.stat().st_size)
    return {
        "journal_synced_records_per_tool_call_p99": round(pct(per_call, 0.99), 2),
        "wal_bytes_per_request_p99": int(pct(wal_per_request, 0.99)),
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--agents", type=int, default=200)
    parser.add_argument("--binary", default=str(Path("target/release/zenpi").resolve()))
    parser.add_argument("--latency-ms", type=int, default=200)
    parser.add_argument("--idle-window", type=float, default=10.0)
    parser.add_argument("--spawn-batch", type=int, default=100)
    parser.add_argument("--probe", type=float, default=3.0, help="seconds every agent waits on a held probe request while sampled")
    parser.add_argument(
        "--ramp",
        type=float,
        default=None,
        help="seconds over which prompts are sent (default N/200; 0 = lockstep)",
    )
    parser.add_argument("--report", default="")
    args = parser.parse_args()
    args.binary = str(Path(args.binary).resolve())
    if not Path(args.binary).exists():
        sys.exit(f"binary not found: {args.binary} (cargo build --release)")
    n = args.agents
    limits = raise_fd_limit(4 * n + 128)  # harness 3N+64 plus in-process provider N+64
    limits.update({name: sysctl(name) for name in ("kern.maxfiles", "kern.maxfilesperproc", "kern.maxproc", "kern.maxprocperuid")})

    ramp = n / 200 if args.ramp is None else args.ramp
    # Probe prompts are held past the ramp so the sample sees every agent waiting.
    provider = Provider(args.latency_ms / 1000, args.probe + ramp)
    threading.Thread(target=provider.run, daemon=True).start()
    provider.ready.wait()
    root = Path(tempfile.mkdtemp(prefix="zenpi-load-"))
    home = root / "home"
    home.mkdir()
    (home / "config.toml").write_text(
        f'backend="openai"\nmodel="gpt-4o-mini"\nbase_url="http://127.0.0.1:{provider.port}"\n'
        'wire_api="chat"\nrequires_openai_auth=false\nmax_retries=0\n'
    )
    report: dict = {"agents": n, "latency_ms": args.latency_ms, "cores": os.cpu_count(), "limits": limits}
    agents: list[Agent] = []
    try:
        started = time.monotonic()
        for index in range(n):
            agents.append(Agent(index, args.binary, root, home))
            if (index + 1) % args.spawn_batch == 0:
                time.sleep(0.05)
        report["spawn_s"] = round(time.monotonic() - started, 2)
        pids = [agent.proc.pid for agent in agents]
        time.sleep(3)
        report["idle"] = window(pids, args.idle_window)

        selector = selectors.DefaultSelector()
        for agent in agents:
            selector.register(agent.proc.stdout, selectors.EVENT_READ, agent)
        def ramped(send) -> None:
            """Send to every agent over the ramp while draining every stdout."""
            ramp_start = time.monotonic()
            for index, agent in enumerate(agents):
                due = ramp_start + ramp * index / max(n, 1)
                while time.monotonic() < due:
                    for key, _ in selector.select(timeout=max(0.0, due - time.monotonic())):
                        key.data.readable()
                send(agent)

        def drain(limit: float) -> None:
            deadline = time.monotonic() + limit
            while any(agent.success is None for agent in agents) and time.monotonic() < deadline:
                for key, _ in selector.select(timeout=1.0):
                    key.data.readable()

        report["ramp_s"] = ramp
        # 1. Waiting probe: every agent holds one provider request at once.
        ramped(lambda agent: agent.prompt(PROBE_PROMPT, "probe"))
        time.sleep(min(1.0, args.probe / 3))
        report["waiting_provider"] = window(pids, args.probe / 3)
        drain(args.probe + ramp + 120)
        report["probe_succeeded"] = sum(1 for agent in agents if agent.success)
        # 2. Tasks, spread over the ramp: concurrent agents, not lockstep.
        cpu_before = sample(pids)
        ramped(lambda agent: agent.prompt())
        drain(max(120, n * 0.2))
        cpu_after = sample(pids)
        task_cpu = sum(cpu_after[p]["cpu_s"] - cpu_before[p]["cpu_s"] for p in pids if p in cpu_before and p in cpu_after)
        succeeded = sum(1 for agent in agents if agent.success)
        durations = [agent.done_at - agent.sent_at for agent in agents if agent.success]
        rounds = 3
        overhead = [d / rounds - args.latency_ms / 1000 for d in durations]
        report["tasks"] = {
            "succeeded": succeeded,
            "success_rate": round(succeeded / n, 4),
            "task_s_p50": round(statistics.median(durations), 3) if durations else None,
            "task_s_p99": round(pct(durations, 0.99), 3),
            "round_overhead_ms_p99": round(pct(overhead, 0.99) * 1000, 1),
            "agent_cpu_ms_per_round": round(task_cpu / max(n * rounds, 1) * 1000, 2),
            "provider_requests": provider.requests,
            "files_written": sum(1 for agent in agents if (agent.cwd / "out.txt").exists()),
        }
        report["after"] = window(pids, 2.0)
        report["journal"] = journal_costs(agents)
    finally:
        for agent in agents:
            try:
                agent.proc.stdin.close()
            except OSError:
                pass
        for agent in agents:
            try:
                agent.proc.wait(timeout=20)
            except subprocess.TimeoutExpired:
                agent.proc.kill()
        shutil.rmtree(root, ignore_errors=True)

    idle, waiting, tasks = report["idle"], report["waiting_provider"], report["tasks"]
    checks = {
        "success_rate_100pct": tasks["success_rate"] == 1.0,
        "idle_rss_p99_le_8MiB": idle["rss_mib_p99"] <= 8,
        "idle_rss_total_le_24GiB": idle["rss_gib_total"] <= 24,
        "idle_cpu_le_1_core": idle["machine_cpu_cores"] <= 1,
        "waiting_cpu_le_1_core": waiting.get("machine_cpu_cores", 99) <= 1,
        "idle_wakeups_le_5_per_s": idle["wakeups_per_s_p99"] <= 5,
        "idle_threads_le_3": idle["threads_max"] <= 3,
        "active_threads_le_5": waiting.get("threads_max", 99) <= 5,
        "fds_le_16": max(idle["fds_max"], waiting.get("fds_max", 0)) <= 16,
        "round_overhead_p99_le_100ms": tasks["round_overhead_ms_p99"] <= 100,
    }
    report["checks"] = checks
    report["pass"] = all(checks.values())
    text = json.dumps(report, indent=2)
    print(text)
    if args.report:
        Path(args.report).write_text(text + "\n")
    return 0 if report["pass"] else 1


if __name__ == "__main__":
    sys.exit(main())

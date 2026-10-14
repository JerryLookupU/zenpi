#!/usr/bin/env python3
"""Run the Android pinned TLS client against the real Rust host.

The default uses adb reverse; ``--lan-ip`` uses the host's private interface
without forwarding. Neither mode proves physical camera scanning. Pairing
tokens remain in temporary process memory and are never saved in the receipt.
Existing emulators are not stopped by this harness.
"""
from __future__ import annotations

import argparse
import fcntl
import hashlib
import ipaddress
import json
import os
import selectors
import re
import shlex
import socket
import subprocess
import tempfile
import time
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import parse_qs, urlsplit
from stage2_android_build import require_current_build
from stage2_host_build import require_current_host
from stage2_instrumentation import require_test_results

ROOT = Path(__file__).resolve().parents[1]

INSTRUMENTATION_CLASSES = ("zenpi.HostPairingTest", "zenpi.PairingScreenTest")
EXPECTED_INSTRUMENTATION_TESTS = 9
RFC1918 = tuple(ipaddress.ip_network(value) for value in ("10.0.0.0/8", "172.16.0.0/12", "192.168.0.0/16"))
ULA = ipaddress.ip_network("fc00::/7")


def connect_emulator_wifi(adb):
    qemu = subprocess.run(adb+["shell","getprop","ro.kernel.qemu"],check=True,capture_output=True,text=True,timeout=10).stdout.strip()
    if qemu != "1": raise ValueError("Wi-Fi setup is emulator-only")
    command = ["cmd","wifi","connect-network","AndroidWifi","open"]
    result = subprocess.run(adb+["shell",*command],capture_output=True,text=True,timeout=10)
    if result.returncode == 0:
        setup = "shell"
    elif "SecurityException" not in result.stderr or "Uid 2000 does not have access" not in result.stderr:
        result.check_returncode()
    else:
        # Only the owned debug emulator's known association permission refusal.
        # App authority, route policy and TLS validation remain unchanged.
        subprocess.run(adb+["shell","su","0",*command],check=True,capture_output=True,text=True,timeout=10)
        setup = "owned_emulator_debug_su"
    # `connect-network` is asynchronous on API 31/34 images. Do not start the
    # host listener while Android still has only the emulator eth0 route: the
    # first socket admission would be reported as an application network loss.
    # The wlan0 route is the explicit readiness boundary for the owned image.
    deadline = time.monotonic() + 30
    while time.monotonic() < deadline:
        route = subprocess.run(adb+["shell", "ip", "route"], check=True,
            capture_output=True, text=True, timeout=10).stdout
        if re.search(r"(?m)^10\.0\.2\.0/24 dev wlan0(?:\s|$)", route):
            return setup
        time.sleep(0.5)
    raise RuntimeError("AndroidWifi association did not publish the wlan0 route")


def require_pair_tests(result):
    if type(result.returncode) is not int or result.returncode != 0:
        raise ValueError("Pairing instrumentation process failed")
    return require_test_results(result.stdout, INSTRUMENTATION_CLASSES, EXPECTED_INSTRUMENTATION_TESTS)


def private_lan_ip(value: str) -> str:
    """Accept a literal private/ULA address, never a hostname or wildcard."""
    try:
        address = ipaddress.ip_address(value)
    except ValueError as error:
        raise argparse.ArgumentTypeError("--lan-ip requires a literal private LAN address") from error
    if getattr(address, "scope_id", None) is not None:
        raise argparse.ArgumentTypeError("--lan-ip must not include an IPv6 interface scope")
    private = (address.version == 4 and any(address in network for network in RFC1918)) or (
        address.version == 6 and address in ULA
    )
    if (not private or address.is_unspecified or address.is_loopback
            or address.is_link_local or address.is_multicast):
        raise argparse.ArgumentTypeError("--lan-ip must be a private, non-loopback LAN address")
    # Store the canonical spelling so the advertised QR address comparison is
    # independent of user-entered IPv6 compression and case.
    return address.compressed


def socket_endpoint(host: str, port: int) -> str:
    return f"[{host}]:{port}" if ":" in host else f"{host}:{port}"


def require_no_reverse(adb):
    result = subprocess.run(adb + ["reverse", "--list"], capture_output=True, text=True, check=True, timeout=10)
    if result.stdout.strip():
        raise RuntimeError("Direct LAN validation requires an empty adb reverse table")


def serialized_device_validation(run):
    selector = argparse.ArgumentParser(add_help=False)
    selector.add_argument("--serial", required=True)
    selected, _ = selector.parse_known_args()
    locks = ROOT / ".ops/stage2_android2unix/validation-locks"
    locks.mkdir(parents=True, exist_ok=True)
    path = locks / (hashlib.sha256(selected.serial.encode()).hexdigest() + ".lock")
    with path.open("a") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        run()


def read_invite(process, timeout=15):
    """Read one bounded UTF-8 invite line without blocking either child pipe."""
    stream = process.stdout
    errors = process.stderr
    if stream is None or errors is None:
        raise RuntimeError("host stdout pipe unavailable")
    selector = selectors.DefaultSelector()
    selector.register(stream, selectors.EVENT_READ)
    selector.register(errors, selectors.EVENT_READ)
    deadline = time.monotonic() + timeout
    data = bytearray()
    try:
        while time.monotonic() < deadline:
            events = selector.select(max(0.0, deadline - time.monotonic()))
            if not events:
                break
            for key, _ in events:
                chunk = os.read(key.fileobj.fileno(), 4096)
                if not chunk:
                    selector.unregister(key.fileobj)
                    continue
                if key.fileobj is stream:
                    data.extend(chunk)
                    newline = data.find(b"\n")
                    if newline >= 0:
                        if newline > 4096:
                            raise RuntimeError("host invite line exceeds 4096 bytes")
                        line = bytes(data[:newline]).decode("utf-8", errors="strict").strip()
                        if not line.startswith("zenpi://pair?"):
                            raise RuntimeError("host emitted an invalid invite line")
                        return line
                    if len(data) > 4096:
                        raise RuntimeError("host invite line exceeds 4096 bytes")
                else:
                    # Drain stderr to keep a noisy child from blocking startup,
                    # but never retain diagnostics or credentials in memory.
                    # Deliberately discard diagnostics after draining them.
                    pass
            if not selector.get_map():
                break
    finally:
        selector.close()
    raise RuntimeError("host invitation deadline")


def seed_session(binary: Path, root: Path):
    """Produce a durable session through the production provider/headless path."""
    answer = "STAGE2_ANDROID_REAL_PROVIDER_TRANSCRIPT"
    requests = []

    class Handler(BaseHTTPRequestHandler):
        def log_message(self, *args):
            pass

        def do_POST(self):
            body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
            requests.append(body)
            self.send_response(200)
            self.send_header("Content-Type", "text/event-stream")
            self.end_headers()
            for event in (
                {"type": "response.created", "response": {"id": "stage2-fixture", "model": body["model"]}},
                {"type": "response.output_text.delta", "delta": answer},
                {"type": "response.completed", "response": {"id": "stage2-fixture", "status": "completed", "model": body["model"]}},
            ):
                self.wfile.write(("data: " + json.dumps(event) + "\n\n").encode())
                self.wfile.flush()

    server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        workspace = root / "workspace"
        config = root / "config"
        workspace.mkdir(); config.mkdir()
        session = root / "session.jsonl"
        (config / "config.toml").write_text(
            'backend="openai"\nmodel="gpt-4.1"\n'
            f'base_url="http://127.0.0.1:{server.server_port}/v1"\n'
            'wire_api="responses"\nrequires_openai_auth=false\nmax_retries=0\n')
        env = {k: v for k, v in os.environ.items()
               if k in {"PATH", "HOME", "TMPDIR", "LANG", "LC_ALL", "LC_CTYPE"}}
        env.update(ZENPI_HOME=str(config), NO_PROXY="127.0.0.1,localhost", no_proxy="127.0.0.1,localhost")
        argv = [str(binary), "--mode", "headless", "--session", str(session)]
        result = subprocess.run(argv, cwd=workspace, env=env,
            input=json.dumps({"schema_version": 2, "type": "prompt", "id": "seed", "text": "Reply with the fixture transcript"}) + "\n",
            capture_output=True, text=True, timeout=30)
        records = [json.loads(line) for line in result.stdout.splitlines()]
        if result.returncode != 0 or len(requests) != 1 or not any(
            record.get("id") == "seed" and record.get("type") == "response" and record.get("success")
            for record in records
        ) or answer not in session.read_text():
            raise RuntimeError("production provider session seed failed: " + result.stderr)
        project = hashlib.sha256(b"zenpi-project-workspace-v1\0" + str(workspace.resolve()).encode()).hexdigest()
        evidence = {"argv": argv, "cwd": str(workspace), "exit_code": result.returncode,
                    "stdout_sha256": hashlib.sha256(result.stdout.encode()).hexdigest(),
                    "stderr_sha256": hashlib.sha256(result.stderr.encode()).hexdigest(), "provider_requests": len(requests)}
        return workspace, session, env, project, answer, evidence
    finally:
        server.shutdown(); server.server_close(); thread.join(timeout=5)


class InstrumentationFailure(RuntimeError):
    def __init__(self, result, timed_out=False):
        super().__init__("instrumentation observation timed out" if timed_out else "instrumentation callback or reader failed")
        self.result = result
        self.timed_out = timed_out


def run_instrumentation(adb, remote, grant, revoke, update=None, tick=None, on_process_ready=None, on_workspace=None, on_background_update=None, on_subtab=None, on_directory=None, on_approval=None, on_worktree=None, timeout=90):
    process = subprocess.Popen(adb + ["shell", remote], stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    output, errors, grant_errors = [], [], []
    action_lock = threading.Lock()

    def read_output():
        for line in process.stdout:
            output.append(line)
            action = grant if "zenpi_phase=ungranted_projection_verified" in line else (
                revoke if "zenpi_phase=snapshot_rendered_revoke_requested" in line else (
                    update if "zenpi_phase=desktop_update_requested" in line else (
                        on_process_ready if "zenpi_phase=network_process_live_ready" in line else None)))
            if action:
                try:
                    with action_lock:
                        action()
                except Exception as error:
                    grant_errors.append(str(error))
            if on_workspace and "zenpi_phase=workspace_" in line:
                try:
                    with action_lock: on_workspace(line)
                except Exception as error: grant_errors.append(str(error))
            if on_background_update and "zenpi_phase=background_update_requested" in line:
                try:
                    with action_lock: on_background_update()
                except Exception as error: grant_errors.append(str(error))
            if on_subtab and "zenpi_phase=production_subtab_buttons_verified" in line:
                try:
                    with action_lock: on_subtab()
                except Exception as error: grant_errors.append(str(error))
            if on_directory and ("zenpi_phase=directory_tabs_verified" in line or "zenpi_phase=directory_order_verified" in line
                or "zenpi_phase=directory_selection_stress_" in line):
                try:
                    with action_lock: on_directory(line)
                except Exception as error: grant_errors.append(str(error))
            if on_approval and "zenpi_phase=approval_" in line:
                try:
                    with action_lock: on_approval(line)
                except Exception as error: grant_errors.append(str(error))
            if on_worktree and "zenpi_phase=worktree_" in line:
                try:
                    with action_lock: on_worktree(line)
                except Exception as error: grant_errors.append(str(error))

    stdout_thread = threading.Thread(target=read_output, daemon=True)
    def read_errors():
        for line in process.stderr:
            errors.append(line)
    stderr_thread = threading.Thread(target=read_errors, daemon=True)
    stdout_thread.start(); stderr_thread.start()
    timed_out = False
    observation_failed = False
    try:
        deadline = time.monotonic() + timeout
        while process.poll() is None:
            if time.monotonic() >= deadline:
                timed_out = True
                break
            if tick:
                with action_lock:
                    tick()
            time.sleep(.02)
    except Exception:
        observation_failed = True
    finally:
        if process.poll() is None:
            process.kill(); process.wait(timeout=5)
        stdout_thread.join(timeout=20); stderr_thread.join(timeout=5)
    result = subprocess.CompletedProcess(process.args, process.returncode, "".join(output), "".join(errors))
    if timed_out or observation_failed or stdout_thread.is_alive() or stderr_thread.is_alive() or grant_errors:
        # Preserve observed phases after terminal cleanup, but do not expose
        # arbitrary callback exception text (which may contain invitation data).
        raise InstrumentationFailure(result, timed_out)
    return result


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--serial", required=True)
    parser.add_argument("--binary", type=Path, default=ROOT / "target/debug/zenpi")
    parser.add_argument("--receipt", type=Path)
    parser.add_argument("--with-host-state", action="store_true")
    parser.add_argument(
        "--lan-ip",
        type=private_lan_ip,
        help="Use this host LAN literal instead of adb reverse (the device must reach it directly).",
    )
    args = parser.parse_args()
    build_provenance = require_current_build()
    host_provenance = require_current_host(args.binary)
    apk = ROOT / "android/app/build/outputs/apk/debug/app-debug.apk"
    test_apk = ROOT / "android/app/build/outputs/apk/androidTest/debug/app-debug-androidTest.apk"
    adb = ["adb", "-s", args.serial]
    boot_deadline = time.monotonic() + 90
    while True:
        boot = subprocess.run(adb + ["shell", "getprop", "sys.boot_completed"],
                              capture_output=True, text=True, timeout=10)
        if boot.returncode == 0 and boot.stdout.strip() == "1":
            break
        if time.monotonic() >= boot_deadline:
            raise SystemExit("selected Android device did not finish booting")
        time.sleep(2)
    for artifact in (args.binary, apk, test_apk):
        if not artifact.is_file():
            raise SystemExit(f"missing artifact: {artifact}")
    subprocess.run(adb + ["install", "-r", str(apk)], check=True, timeout=60)
    subprocess.run(adb + ["install", "-r", str(test_apk)], check=True, timeout=60)
    port = None
    reverse_created = False
    host = None
    started = time.time()
    with tempfile.TemporaryDirectory(prefix="zenpi-android-pair-") as temp:
        try:
            root = Path(temp)
            # Keep the production child deterministic and prevent ambient
            # provider credentials from entering a device-test process tree.
            env = {key: value for key, value in os.environ.items()
                   if key in {"PATH", "HOME", "TMPDIR", "LANG", "LC_ALL", "LC_CTYPE"}}
            env["ZENPI_HOME"] = str(root / "home")
            seed = None
            bind_host = args.lan_ip or "127.0.0.1"
            if args.lan_ip:
                require_no_reverse(adb)
                family = socket.AF_INET6 if ":" in args.lan_ip else socket.AF_INET
                with socket.socket(family, socket.SOCK_STREAM) as probe:
                    probe.bind((bind_host, 0))
            argv = [str(args.binary), "host", "pair", "--bind", socket_endpoint(bind_host, 0), "--plain",
                    "--duration-seconds", "120", "--identity", str(root / "identity.json")]
            if args.lan_ip:
                argv += ["--advertise", args.lan_ip]
            if args.with_host_state:
                workspace, session, env, project, answer, seed = seed_session(args.binary, root)
                argv += ["--workspace", str(workspace), "--session", str(session)]
            host = subprocess.Popen(argv, cwd=ROOT, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
            invite = read_invite(host)
            values = parse_qs(urlsplit(invite).query, strict_parsing=True)
            advertised = values["address"][0]
            port = int(advertised.rsplit(":", 1)[1])
            advertised_host = advertised.rsplit(":", 1)[0].strip("[]")
            if args.lan_ip and advertised_host != args.lan_ip:
                raise RuntimeError("host advertised a different LAN address")
            if not args.lan_ip:
                subprocess.run(adb + ["reverse", f"tcp:{port}", f"tcp:{port}"], check=True, timeout=15)
                reverse_created = True
            command = ["am", "instrument", "-w", "-r", "-e", "invite", invite]
            command += ["-e", "class", ",".join(INSTRUMENTATION_CLASSES)]
            grant = None
            revoke = None
            if args.with_host_state:
                command += ["-e", "expected_project", project, "-e", "expected_text", answer]

                def grant():
                    devices = json.loads((root / "identity.devices.json").read_text())["devices"]
                    if len(devices) != 1:
                        raise RuntimeError("expected exactly one fixture device")
                    subprocess.run([str(args.binary), "host", "pair", "--identity", str(root / "identity.json"),
                        "--grant-device-read", devices[0]["id"], "--project", project],
                        cwd=ROOT, env=env, check=True, capture_output=True, timeout=10)

                def revoke():
                    devices = json.loads((root / "identity.devices.json").read_text())["devices"]
                    if len(devices) != 1:
                        raise RuntimeError("expected exactly one fixture device")
                    subprocess.run([str(args.binary), "host", "pair", "--identity", str(root / "identity.json"),
                        "--revoke-device", devices[0]["id"]], cwd=ROOT, env=env,
                        check=True, capture_output=True, timeout=10)

            command += ["app.zenpi.android.test/androidx.test.runner.AndroidJUnitRunner"]
            result = run_instrumentation(adb, shlex.join(command), grant, revoke)
            # adb can return zero even when instrumentation assertions fail.
            try:
                completed = require_pair_tests(result)
            except ValueError:
                completed = []
            passed = bool(completed)
            if args.with_host_state:
                passed = passed and "zenpi_phase=host_state_verified" in result.stdout
            if args.lan_ip:
                # The direct route must remain direct for the entire run; do
                # not accept a late-created reverse tunnel as LAN evidence.
                require_no_reverse(adb)
            if host.poll() is not None:
                raise RuntimeError("host exited before validation finished")
            sdk = subprocess.run(adb + ["shell", "getprop", "ro.build.version.sdk"], capture_output=True, text=True, check=True).stdout.strip()
            screenshot = None
            if passed and args.with_host_state and args.receipt:
                screenshot_path = args.receipt.with_suffix(".png")
                screenshot_path.parent.mkdir(parents=True, exist_ok=True)
                pixels = subprocess.run(adb + ["exec-out", "run-as", "app.zenpi.android", "cat", "files/host-projection.png"],
                    capture_output=True, check=True, timeout=10).stdout
                if not pixels.startswith(b"\x89PNG\r\n\x1a\n"):
                    raise RuntimeError("snapshot screenshot is not PNG")
                screenshot_path.write_bytes(pixels)
                screenshot = {"path": str(screenshot_path), "sha256": hashlib.sha256(pixels).hexdigest()}
            receipt = {
                "scope": "production Rust host / Android pinned TLS + Compose states via direct LAN when --lan-ip is selected, otherwise adb reverse; authorized snapshot when explicitly selected",
                "camera_scanning_verified": False,
                "lan_reachability_verified": bool(args.lan_ip and passed),
                "transport_route": "direct_private_interface" if args.lan_ip else "adb_reverse",
                "adb_reverse_used": not bool(args.lan_ip),
                "lan_interface_tls_verified": bool(args.lan_ip and passed),
                "physical_lan_peer_verified": False,
                "device_registration_verified": passed, "session_sync_verified": False,
                "authorized_snapshot_verified": passed and args.with_host_state,
                "ungranted_project_hidden_verified": passed and args.with_host_state,
                "snapshot_reconnect_verified": passed and args.with_host_state,
                "snapshot_compose_render_verified": passed and args.with_host_state,
                "host_revocation_reconnect_refused_verified": passed and args.with_host_state,
                "live_events_verified": False,
                "snapshot_screenshot": screenshot,
                "host_argv": argv, "host_cwd": str(ROOT), "provider_seed": seed,
                "environment_names": sorted(k for k in (env or {}) if k.startswith("ZENPI_") or k.lower() == "no_proxy"),
                "requirement": json.loads((ROOT / "Docs/execution/stage2_android2unix_active_requirement.json").read_text()),
                "baseline_revision": subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT, capture_output=True, text=True, check=True).stdout.strip(),
                "integration_state": "uncommitted worktree, provisional evidence, no item acceptance",
                "related_items": ["S2-013", "S2-014", "S2-030", "S2-034", "S2-036", "S2-037"],
                "started_at_unix": started, "finished_at_unix": time.time(), "serial": args.serial,
                "android_api": sdk, "passed": passed, "instrumentation_exit_code": result.returncode,
                "instrumentation_classes": list(INSTRUMENTATION_CLASSES),
                "expected_instrumentation_tests": EXPECTED_INSTRUMENTATION_TESTS,
                "completed_instrumentation_tests": completed,
                "instrumentation_output": result.stdout.replace(invite, "<redacted-invite>"),
                "instrumentation_stdout_sha256": hashlib.sha256(result.stdout.encode()).hexdigest(),
                "instrumentation_stderr_sha256": hashlib.sha256(result.stderr.encode()).hexdigest(),
                "artifacts": {str(path.relative_to(ROOT)): hashlib.sha256(path.read_bytes()).hexdigest()
                              for path in (args.binary.resolve(), apk, test_apk)},
                "android_build_provenance": build_provenance,
                "host_build_provenance": host_provenance,
            }
            if require_current_build() != build_provenance:
                raise RuntimeError("Android source/build provenance changed during device validation")
            if require_current_host(args.binary) != host_provenance:
                raise RuntimeError("Host source/build provenance changed during device validation")
            print(result.stdout.replace(invite, "<redacted-invite>"))
            print(json.dumps(receipt, indent=2))
            if args.receipt:
                args.receipt.parent.mkdir(parents=True, exist_ok=True)
                args.receipt.write_text(json.dumps(receipt, indent=2) + "\n")
            if not passed:
                raise SystemExit("Android instrumentation failed")
        finally:
            if reverse_created:
                subprocess.run(adb + ["reverse", "--remove", f"tcp:{port}"], capture_output=True, timeout=15)
            if host is not None:
                host.terminate()
                try:
                    host.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    host.kill(); host.wait(timeout=5)


if __name__ == "__main__":
    serialized_device_validation(main)

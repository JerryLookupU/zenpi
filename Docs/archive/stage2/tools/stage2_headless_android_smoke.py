#!/usr/bin/env python3
"""Production non-TUI host and Android prompt/process recovery over pinned TLS.

Direct RFC1918 interface mode is optional. No physical camera/LAN claim is made.
Only fixture-owned Android process, listener and provider are stopped.
"""
from __future__ import annotations

import argparse
import base64
import hashlib
import json
import re
import shlex
import socket
import subprocess
import tempfile
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import parse_qs, urlsplit

from stage2_android_pair_smoke import ROOT, read_invite, seed_session, serialized_device_validation
from stage2_android_build import require_current_build
from stage2_host_build import require_current_host
from stage2_desktop_android_smoke import private_lan_ip, require_no_reverse, force_stop_live_android, prepare_emulator_host_network

ATTACHMENT_PATH = "documents/fixture note.txt"
ATTACHMENT_BYTES = b"STAGE2_HOST_ATTACHMENT_PAYLOAD\n"
ATTACHMENT_PROMPT = 'STAGE2_HEADLESS_ANDROID_FIRST @"' + ATTACHMENT_PATH + '"'
RESOURCE_NAME = "zz-stage2-review"
RESOURCE_ARGUMENTS = "STAGE2_HEADLESS_ANDROID_SECOND literal $1 $(touch never-created)"
RESOURCE_PROMPT = "/" + RESOURCE_NAME + " " + RESOURCE_ARGUMENTS
RESOURCE_TEMPLATE = "---\ndescription: Stage2 selected template\n---\nSTAGE2_HOST_TEMPLATE_BODY\n$ARGUMENTS\n"
RESOURCE_EXPANDED = "STAGE2_HOST_TEMPLATE_BODY\n" + RESOURCE_ARGUMENTS


def concurrent_read_evidence(output):
    return isinstance(output, str) and re.findall(
        r"(?m)^INSTRUMENTATION_STATUS: zenpi_phase=headless_concurrent_reads_verified$", output
    ) == ["INSTRUMENTATION_STATUS: zenpi_phase=headless_concurrent_reads_verified"] and re.findall(
        r"(?m)^INSTRUMENTATION_STATUS: headless_concurrent_read_count=(.*)$", output
    ) == ["3"]


def selection_evidence(requests, records, checkpoint, project, device):
    if len(requests) != 2 or any(not isinstance(request, dict) or request.get("model") != model or request.get("reasoning") != {"effort":effort}
        for request, model, effort in zip(requests, ("gpt-5.2", "gpt-5.2-2025-12-11"), ("high", "low"))):
        return False
    bindings = checkpoint.get("remote_workspace", {}).get("prompt_bindings", [])
    if not isinstance(bindings, list) or any(not isinstance(binding, dict) for binding in bindings):
        return False
    selected = [record for record in records if record.get("event", {}).get("type") == "model_selected"]
    if len(selected) != 2 or any(record.get("kind") != "event" for record in selected):
        return False
    revisions = []
    for record, operation, field, value, effort in zip(selected, ("model_select", "reasoning_select"),
        ("model", "reasoning_effort"), ("gpt-5.2-2025-12-11", "low"), ("high", "low")):
        event = record["event"]
        revision = record.get("seq")
        if type(revision) is not int or revision < 1 or revision > 18446744073709551615 \
            or event.get("model") != "gpt-5.2-2025-12-11" or event.get("reasoning_effort") != effort:
            return False
        revisions.append(revision)
        kind = "remote_" + ("model" if operation == "model_select" else "reasoning") + "_admission_v1"
        admissions = [record for record in records if record.get("event", {}).get("type") == kind]
        if len(admissions) != 2 or [item["event"].get("state") for item in admissions] != ["reserved", "succeeded"]:
            return False
        if any(item.get("kind") != "event" or type(item.get("seq")) is not int for item in admissions) \
            or [item["seq"] for item in admissions] != [revision - 1, revision + 1]:
            return False
        reserved, terminal = (item["event"] for item in admissions)
        result = terminal.get("result", {})
        if not isinstance(result, dict) or set(result) != {"request_id", "project_id", "session_id", field, "selection_revision"} \
            or result.get(field) != value or type(result.get("selection_revision")) is not int or result["selection_revision"] != revision \
            or result.get("project_id") != project or terminal.get("device_id") != device \
            or terminal.get("request_id") != result.get("request_id") or terminal.get("code") is not None:
            return False
        if any(reserved.get(key) != terminal.get(key) for key in ("request_id", "device_id", "fingerprint")) \
            or reserved.get("result") is not None or reserved.get("code") is not None \
            or not re.fullmatch(r"[0-9a-f]{64}", terminal.get("fingerprint", "")):
            return False
        matches = [binding for binding in bindings if binding.get("device_id") == device and binding.get("request_id") == terminal.get("request_id")]
        if len(matches) != 1 or matches[0] != {"device_id":device, "request_id":terminal["request_id"],
            "project_id":project, "session_id":result.get("session_id"), "fingerprint":terminal["fingerprint"], "operation":operation}:
            return False
        session_ids = {item.get("event", {}).get("result", {}).get("session_id") for item in records
            if item.get("event", {}).get("type") == "remote_prompt_admission_v1" and item["event"].get("state") == "accepted"}
        if session_ids != {result.get("session_id")} or not result.get("session_id"):
            return False
        positions = [records.index(item) for item in (admissions[0], record, admissions[1])]
        if positions != sorted(set(positions)):
            return False
    prompts = [item for item in records if item.get("event", {}).get("type") == "remote_prompt_admission_v1"
        and item["event"].get("state") == "accepted"]
    if len(prompts) != 2 or not records.index(prompts[0]) < records.index(selected[0]) < records.index(selected[1]) < records.index(prompts[1]):
        return False
    completed = [index for index,item in enumerate(records) if item.get("event",{}).get("type") == "remote_prompt_progress_v1"
        and item["event"].get("request_id") == prompts[0]["event"].get("request_id") and item["event"].get("state") == "succeeded"]
    if len(completed) != 1 or completed[0] >= records.index(selected[0]):
        return False
    return revisions[0] < revisions[1] and len({binding.get("request_id") for binding in bindings}) == len(bindings)


def resource_evidence(requests, records):
    if len(requests) != 2:
        return False
    users = [item for item in requests[1].get("input", []) if item.get("role") == "user"]
    if not users or users[-1].get("content") != RESOURCE_EXPANDED:
        return False
    if "STAGE2_HOST_TEMPLATE_BODY" in json.dumps(requests[0]) or RESOURCE_PROMPT in json.dumps(requests[1]):
        return False
    turns = [record.get("turn", {}) for record in records if record.get("kind") == "turn"]
    selected = [turn for turn in turns if turn.get("role") == "user" and turn.get("content") == RESOURCE_PROMPT]
    if len(selected) != 1:
        return False
    provenance = selected[0].get("metadata", {}).get("resource", {})
    if not isinstance(provenance, dict) or set(provenance) != {"kind", "generation", "name", "source", "source_hash"}:
        return False
    source = provenance.get("source")
    if provenance.get("kind") != "template" or provenance.get("name") != RESOURCE_NAME \
        or type(provenance.get("generation")) is not int or provenance["generation"] < 1 \
        or not isinstance(source, str) or not Path(source).is_absolute() \
        or Path(source).parts[-3:] != (".zenpi", "prompts", RESOURCE_NAME + ".md") \
        or provenance.get("source_hash") != hashlib.sha256(RESOURCE_TEMPLATE.encode()).hexdigest():
        return False
    if any(turn is not selected[0] and turn.get("metadata", {}).get("resource") for turn in turns):
        return False
    return "STAGE2_HOST_TEMPLATE_BODY" not in json.dumps(records) and "STAGE2_FILLER_BODY" not in json.dumps(requests)


def attachment_evidence(requests, records):
    if len(requests) != 2:
        return False
    users = [item for item in requests[0].get("input", []) if item.get("role") == "user"]
    if not users or not isinstance(users[-1].get("content"), list):
        return False
    expected = [{"type":"input_text", "text":ATTACHMENT_PROMPT},
        {"type":"input_file", "filename":"fixture note.txt", "file_data":"data:text/plain;base64," + base64.b64encode(ATTACHMENT_BYTES).decode()} ]
    if users[-1]["content"] != expected:
        return False
    second = [item for item in requests[1].get("input", []) if item.get("role") == "user"]
    if not second or any(part.get("type") in ("input_file", "input_image")
        for item in second for part in (item.get("content") if isinstance(item.get("content"), list) else [])):
        return False
    turns = [record.get("turn", {}) for record in records if record.get("kind") == "turn"]
    attached = [turn for turn in turns if turn.get("role") == "user" and turn.get("content") == ATTACHMENT_PROMPT]
    if len(attached) != 1:
        return False
    if any(turn is not attached[0] and turn.get("metadata", {}).get("attachments") for turn in turns):
        return False
    references = attached[0].get("metadata", {}).get("attachments")
    if references != [{"kind":"file", "mime_type":"text/plain", "path":ATTACHMENT_PATH,
        "url":None, "file_id":None, "size_bytes":len(ATTACHMENT_BYTES), "sha256":hashlib.sha256(ATTACHMENT_BYTES).hexdigest()}]:
        return False
    journal = json.dumps(records)
    return ATTACHMENT_BYTES.decode().strip() not in journal and base64.b64encode(ATTACHMENT_BYTES).decode() not in journal and "file_data" not in journal


class RunningStreamGate:
    """Provider completion is held until Android proves display and reattach."""
    phases = ("headless_running_stream_verified", "headless_background_writes_fenced_verified", "headless_background_reads_fenced_verified", "headless_running_stream_resumed_verified")

    def __init__(self):
        self.release = threading.Event()
        self.lock = threading.RLock()
        self.events = []
        self.failure = None

    def record(self, event):
        with self.lock:
            expected = ("provider_preview_flushed", *self.phases, "provider_completed_flushed")
            if len(self.events) >= len(expected) or event != expected[len(self.events)]:
                self.failure = "unexpected running-stream evidence order: " + event
                raise RuntimeError(self.failure)
            self.events.append(event)
            if event == self.phases[-1]: self.release.set()

    def evidence(self):
        with self.lock:
            return {"events":list(self.events), "failure":self.failure,
                "verified":self.failure is None and self.events == ["provider_preview_flushed", *self.phases, "provider_completed_flushed"]}


def prompt_evidence(requests, records, checkpoint, project, device, committed_id):
    if not attachment_evidence(requests, records) or not resource_evidence(requests, records):
        return False
    events = [record.get("event", {}) for record in records]
    admissions = [event for event in events if event.get("type") == "remote_prompt_admission_v1" and event.get("state") == "accepted"]
    bindings = checkpoint.get("remote_workspace", {}).get("prompt_bindings", [])
    if len(requests) != 2 or len(admissions) != 2 or len(bindings) != 4 \
        or not selection_evidence(requests, records, checkpoint, project, device):
        return False
    if len({event.get("request_id") for event in admissions}) != 2 or committed_id not in {event.get("request_id") for event in admissions}:
        return False
    session = admissions[0].get("result", {}).get("session_id")
    for event in admissions:
        result = event.get("result", {})
        if event.get("device_id") != device or result.get("project_id") != project or result.get("session_id") != session:
            return False
        matches = [value for value in bindings if value.get("request_id") == event.get("request_id") and value.get("device_id") == device]
        if len(matches) != 1 or matches[0].get("fingerprint") != event.get("fingerprint") or matches[0].get("project_id") != project or matches[0].get("session_id") != session:
            return False
        if not re.fullmatch(r"[0-9a-f]{64}", event.get("fingerprint", "")):
            return False
        progress = [value for value in events if value.get("type") == "remote_prompt_progress_v1" and value.get("request_id") == event.get("request_id")]
        if [value.get("state") for value in progress] != ["running", "succeeded"] or any(
            (value.get("device_id"), value.get("project_id"), value.get("session_id")) != (device, project, session) for value in progress):
            return False
    turns = [record.get("turn", {}) for record in records if record.get("kind") == "turn"]
    for index, marker in enumerate(("STAGE2_HEADLESS_ANDROID_FIRST", "STAGE2_HEADLESS_ANDROID_SECOND")):
        users = [value for value in requests[index].get("input", []) if value.get("role") == "user"]
        if marker not in json.dumps(users[-1:]):
            return False
        if len([turn for turn in turns if turn.get("role") == "assistant" and turn.get("content") == marker + "_DONE"]) != 1:
            return False
    return True


def ack_evidence(records, checkpoint, project, device):
    acknowledgements = checkpoint.get("remote_workspace", {}).get("event_acks", [])
    sessions = {record.get("event", {}).get("result", {}).get("session_id") for record in records
        if record.get("event", {}).get("type") == "remote_prompt_admission_v1"
        and record["event"].get("state") == "accepted"}
    if len(sessions) != 1 or None in sessions or len(acknowledgements) != 1:
        return False
    ack = acknowledgements[0]
    if not isinstance(ack, dict) or not isinstance(ack.get("cursor"), dict):
        return False
    turns = sum(record.get("kind") == "turn" for record in records)
    if type(ack["cursor"].get("next_sequence")) is not int or not turns:
        return False
    return ack == {"device_id":device, "project_id":project,
        "cursor":{"session_id":next(iter(sessions)), "next_sequence":turns}}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--serial", required=True)
    parser.add_argument("--binary", type=Path, default=ROOT / "target/debug/zenpi")
    parser.add_argument("--receipt", type=Path, required=True)
    route = parser.add_mutually_exclusive_group()
    route.add_argument("--lan-ip", type=private_lan_ip)
    route.add_argument("--emulator-host-network", action="store_true", help="Emulator 10.0.2.2 without forwarding; not physical LAN")
    parser.add_argument("--expected-api", type=int)
    args = parser.parse_args()
    build_provenance = require_current_build()
    host_provenance = require_current_host(args.binary)
    args.binary = args.binary.resolve()
    artifacts = (args.binary, ROOT / "android/app/build/outputs/apk/debug/app-debug.apk", ROOT / "android/app/build/outputs/apk/androidTest/debug/app-debug-androidTest.apk", Path(__file__).resolve())
    hashes = {str(path.relative_to(ROOT)): hashlib.sha256(path.read_bytes()).hexdigest() for path in artifacts}
    harness_inputs = {name:hashlib.sha256((ROOT/name).read_bytes()).hexdigest() for name in
        ("tools/stage2_headless_android_smoke.py", "tools/stage2_desktop_android_smoke.py", "tools/stage2_android_pair_smoke.py")}
    adb = ["adb", "-s", args.serial]
    deadline = time.monotonic() + 90
    while subprocess.run(adb + ["shell", "getprop", "sys.boot_completed"], capture_output=True, text=True, timeout=10).stdout.strip() != "1":
        if time.monotonic() > deadline: raise RuntimeError("Android boot deadline")
        time.sleep(2)
    android_api = int(subprocess.run(adb + ["shell", "getprop", "ro.build.version.sdk"], capture_output=True, text=True, check=True, timeout=10).stdout.strip())
    if args.expected_api is not None and android_api != args.expected_api:
        raise RuntimeError(f"Expected Android API {args.expected_api}, found {android_api}")
    bind_ip = "0.0.0.0" if args.emulator_host_network else args.lan_ip or "127.0.0.1"
    wifi_setup = prepare_emulator_host_network(adb) if args.emulator_host_network else None
    if args.lan_ip:
        require_no_reverse(adb)
        family = socket.AF_INET6 if ":" in bind_ip else socket.AF_INET
        with socket.socket(family, socket.SOCK_STREAM) as probe: probe.bind((bind_ip, 0))
    for artifact in artifacts[1:3]: subprocess.run(adb + ["install", "-r", str(artifact)], check=True, timeout=60)
    args.receipt.parent.mkdir(parents=True, exist_ok=True)
    started = time.time()
    passed = False
    outputs = []
    lines = []
    reader = None
    invite = None
    advertised_endpoint = None
    first_output_retained = False
    host = server = instrumentation = None
    port = None
    failure = None
    screenshot_evidence = None
    diff_screenshot_evidence = None
    stream_screenshot_evidence = None
    files_screenshot_evidence = None
    resources_screenshot_evidence = None
    selection_screenshot_evidence = None
    stream_gate = RunningStreamGate()
    process_evidence = None
    command_evidence = None
    with tempfile.TemporaryDirectory(prefix="zenpi-headless-android-") as raw:
        root = Path(raw)
        try:
            workspace, session, env, project, _, seed = seed_session(args.binary, root)
            env = {key: value for key, value in env.items()
                   if key in {"PATH", "HOME", "TMPDIR", "LANG", "LC_ALL", "LC_CTYPE", "ZENPI_HOME", "NO_PROXY", "no_proxy"}}
            subprocess.run(["git", "init", "-q", str(workspace)], check=True, capture_output=True, timeout=10)
            (workspace / "review.txt").write_text("HEADLESS_DIFF_BEFORE\n")
            subprocess.run(["git", "-C", str(workspace), "add", "review.txt"], check=True, capture_output=True, timeout=10)
            subprocess.run(["git", "-C", str(workspace), "-c", "user.email=fixture@example.invalid", "-c", "user.name=Fixture",
                "commit", "-qm", "diff fixture"], check=True, capture_output=True, timeout=10)
            (workspace / "review.txt").write_text("HEADLESS_DIFF_AFTER\n")
            diff_before = (workspace / "review.txt").read_bytes()
            (workspace / "documents").mkdir()
            (workspace / ATTACHMENT_PATH).write_bytes(ATTACHMENT_BYTES)
            for index in range(80):
                (workspace / "documents" / f"a-file-{index:03}.txt").write_text("fixture filler")
            prompts = workspace / ".zenpi/prompts"
            prompts.mkdir(parents=True)
            for index in range(40):
                (prompts / f"fixture-{index:02}.md").write_text("STAGE2_FILLER_BODY\n" if index % 2 == 0 else
                    "---\ndescription: ''\n---\nSTAGE2_FILLER_BODY\n")
            resource_file = prompts / (RESOURCE_NAME + ".md")
            resource_file.write_text(RESOURCE_TEMPLATE)
            skill = workspace / ".zenpi/skills/stage2-research"
            skill.mkdir(parents=True)
            (skill / "SKILL.md").write_text("---\nname: stage2-research\ndescription: Catalog skill fixture\n---\nSTAGE2_SKILL_BODY\n")
            requests = []
            class Handler(BaseHTTPRequestHandler):
                def log_message(self, *args): pass
                def do_POST(self):
                    body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
                    requests.append(body)
                    latest = json.dumps([value for value in body.get("input", []) if value.get("role") == "user"][-1:])
                    markers = [marker for marker in ("STAGE2_HEADLESS_ANDROID_FIRST", "STAGE2_HEADLESS_ANDROID_SECOND") if marker in latest]
                    if len(markers) != 1:
                        self.send_error(400); return
                    self.send_response(200); self.send_header("Content-Type", "text/event-stream"); self.end_headers()
                    def send(event):
                        self.wfile.write(("data: " + json.dumps(event) + "\n\n").encode()); self.wfile.flush()
                    send({"type":"response.output_text.delta", "delta":markers[0] + "_DONE"})
                    if markers[0] == "STAGE2_HEADLESS_ANDROID_FIRST":
                        # TextDone is previewable without adding a newline to the durable answer.
                        with stream_gate.lock:
                            send({"type":"response.output_text.done", "text":markers[0] + "_DONE"})
                            stream_gate.record("provider_preview_flushed")
                        if not stream_gate.release.wait(35):
                            return
                        if stream_gate.evidence()["failure"] is not None:
                            return
                    send({"type":"response.completed", "response":{"id":"headless", "status":"completed", "model":body["model"]}})
                    if markers[0] == "STAGE2_HEADLESS_ANDROID_FIRST": stream_gate.record("provider_completed_flushed")
            server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
            thread = threading.Thread(target=server.serve_forever, daemon=True); thread.start()
            (root / "config/config.toml").write_text('backend="openai"\nmodel="gpt-5.2"\nmodel_reasoning_effort="high"\n'
                f'base_url="http://127.0.0.1:{server.server_port}/v1"\nwire_api="responses"\nrequires_openai_auth=false\nmax_retries=0\n')
            identity = root / "identity.json"
            endpoint = f"[{bind_ip}]:0" if ":" in bind_ip else f"{bind_ip}:0"
            argv = [str(args.binary), "host", "pair", "--plain", "--workspace", str(workspace), "--session", str(session),
                "--bind", endpoint, "--duration-seconds", "90", "--identity", str(identity)]
            if args.emulator_host_network: argv += ["--advertise", "10.0.2.2"]
            host = subprocess.Popen(argv, cwd=workspace, env=env, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
            invite = read_invite(host)
            values = parse_qs(urlsplit(invite).query, strict_parsing=True)
            advertised_endpoint = values["address"][0]
            if args.emulator_host_network and values["address"][0].split(":",1)[0] != "10.0.2.2":
                raise RuntimeError("Unexpected emulator host alias")
            port = int(values["address"][0].rsplit(":", 1)[1])
            if not args.lan_ip and not args.emulator_host_network: subprocess.run(adb + ["reverse", f"tcp:{port}", f"tcp:{port}"], check=True, timeout=10)
            namespace = f"headless-{time.time_ns()}"
            command = ["am", "instrument", "-w", "-r", "-e", "class", "zenpi.HeadlessHostTest#androidComposerUsesHeadlessOwnerAndRetainsCommittedPromptUntilProcessDeath",
                "-e", "invite", invite, "-e", "expected_project", project, "-e", "process_namespace", namespace,
                "app.zenpi.android.test/androidx.test.runner.AndroidJUnitRunner"]
            instrumentation = subprocess.Popen(adb + ["shell", shlex.join(command)], stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
            lines = []; errors = []; state = {}; lock = threading.Lock()
            def observe():
                for line in instrumentation.stdout:
                    lines.append(line)
                    if "zenpi_phase=headless_local_grant_requested" in line:
                        try:
                            devices = json.loads(identity.with_suffix(".devices.json").read_text())["devices"]
                            if len(devices) != 1: raise RuntimeError("unexpected fixture device count")
                            subprocess.run([str(args.binary), "host", "pair", "--identity", str(identity), "--grant-device-control", devices[0]["id"], "--project", project], env=env, check=True, capture_output=True, timeout=10)
                        except Exception as error: errors.append(str(error))
                    phase = line.strip().split("zenpi_phase=", 1)[-1]
                    if phase in stream_gate.phases:
                        try: stream_gate.record(phase)
                        except Exception as error: errors.append(str(error))
                    if "network_process_pid=" in line:
                        with lock: state["pid"] = line.strip().split("network_process_pid=", 1)[1]
                    if "request_id=" in line:
                        with lock: state["request_id"] = line.strip().split("request_id=", 1)[1]
                    if "zenpi_phase=headless_prompt_committed_before_death" in line:
                        with lock: state["committed"] = True
            reader = threading.Thread(target=observe, daemon=True); reader.start()
            deadline = time.monotonic() + 70
            while True:
                with lock: committed = all(state.get(key) for key in ("pid", "request_id", "committed"))
                if committed: break
                if errors or instrumentation.poll() is not None or time.monotonic() > deadline:
                    raise RuntimeError("first Android phase failed: " + str(errors) + "\n" + "".join(lines))
                time.sleep(.05)
            previous_pid = force_stop_live_android(adb)
            if previous_pid != state["pid"]: raise RuntimeError("process identity mismatch")
            instrumentation.wait(timeout=10); reader.join(timeout=5)
            outputs.append("".join(lines).replace(invite, "<redacted-invite>"))
            first_output_retained = True
            if not concurrent_read_evidence("".join(lines)):
                raise RuntimeError("parallel authenticated Android reads were not proved")
            if not stream_gate.evidence()["verified"]:
                raise RuntimeError("running provider preview / foreground socket cancellation / reattach were not proved")
            if "zenpi_phase=headless_diff_read_verified" not in "".join(lines) or (workspace / "review.txt").read_bytes() != diff_before:
                raise RuntimeError("Android read-only diff not proved")
            if "zenpi_phase=headless_file_picker_reference_verified" not in "".join(lines) or (workspace / ATTACHMENT_PATH).read_bytes() != ATTACHMENT_BYTES:
                raise RuntimeError("Android host file picker not proved")
            if "zenpi_phase=headless_resource_pages_draft_verified" not in "".join(lines) or resource_file.read_text() != RESOURCE_TEMPLATE \
                or (workspace / "never-created").exists():
                raise RuntimeError("Android resource paging/draft/literal arguments not proved")
            if "zenpi_phase=headless_model_reasoning_selection_verified" not in "".join(lines):
                raise RuntimeError("Android model and reasoning selection not proved")
            command = ["am", "instrument", "-w", "-r", "-e", "class", "zenpi.HeadlessHostTest#newAndroidProcessResolvesOnlyTheSavedHeadlessPrompt",
                "-e", "process_namespace", namespace, "-e", "previous_pid", previous_pid, "-e", "request_id", state["request_id"],
                "app.zenpi.android.test/androidx.test.runner.AndroidJUnitRunner"]
            recovered = subprocess.run(adb + ["shell", shlex.join(command)], capture_output=True, text=True, timeout=35)
            outputs.append(recovered.stdout)
            if recovered.returncode or not re.search(r"(?m)^OK \(1 test\)$", recovered.stdout) or "headless_prompt_process_recovery_verified" not in recovered.stdout or "FAILURES" in recovered.stdout:
                raise RuntimeError("new Android process failed: " + recovered.stdout + recovered.stderr)
            if "headless_durable_cursor_ack_verified" not in recovered.stdout:
                raise RuntimeError("durable Android ACK not proved")
            process_evidence = {"killed_pid":previous_pid, "committed_request_id":state["request_id"],
                "pending_command_recovered_in_distinct_process":True, "recovery_exit_code":recovered.returncode}
            records = [json.loads(line) for line in session.read_text().splitlines()]
            checkpoint = json.loads(session.parent.joinpath("project-workspace.json").read_text())
            devices = json.loads(identity.with_suffix(".devices.json").read_text())["devices"]
            passed = prompt_evidence(requests, records, checkpoint, project, devices[0]["id"], state["request_id"])
            passed = passed and ack_evidence(records, checkpoint, project, devices[0]["id"])
            command_path = args.receipt.with_suffix(".commands.json")
            command_path.write_text(json.dumps({"requests":requests, "records":records, "checkpoint":checkpoint, "device_id":devices[0]["id"], "project_id":project, "committed_request_id":state["request_id"]}, indent=2) + "\n")
            command_evidence = {"path":str(command_path), "sha256":hashlib.sha256(command_path.read_bytes()).hexdigest()}
            screenshot = subprocess.run(adb + ["exec-out", "run-as", "app.zenpi.android", "cat", "files/headless-host.png"], capture_output=True, check=True, timeout=10).stdout
            if not screenshot.startswith(b"\x89PNG\r\n\x1a\n"): raise RuntimeError("missing PNG")
            args.receipt.with_suffix(".png").write_bytes(screenshot)
            screenshot_evidence = {"path":str(args.receipt.with_suffix(".png")), "sha256":hashlib.sha256(screenshot).hexdigest()}
            diff_screenshot = subprocess.run(adb + ["exec-out", "run-as", "app.zenpi.android", "cat", "files/headless-diff.png"], capture_output=True, check=True, timeout=10).stdout
            if not diff_screenshot.startswith(b"\x89PNG\r\n\x1a\n"): raise RuntimeError("missing diff PNG")
            diff_path = args.receipt.with_suffix(".diff.png")
            diff_path.write_bytes(diff_screenshot)
            diff_screenshot_evidence = {"path":str(diff_path), "sha256":hashlib.sha256(diff_screenshot).hexdigest()}
            stream_screenshot = subprocess.run(adb + ["exec-out", "run-as", "app.zenpi.android", "cat", "files/headless-stream.png"], capture_output=True, check=True, timeout=10).stdout
            if not stream_screenshot.startswith(b"\x89PNG\r\n\x1a\n"): raise RuntimeError("missing running-stream PNG")
            stream_path = args.receipt.with_suffix(".stream.png")
            stream_path.write_bytes(stream_screenshot)
            stream_screenshot_evidence = {"path":str(stream_path), "sha256":hashlib.sha256(stream_screenshot).hexdigest()}
            files_screenshot = subprocess.run(adb + ["exec-out", "run-as", "app.zenpi.android", "cat", "files/headless-files.png"], capture_output=True, check=True, timeout=10).stdout
            if not files_screenshot.startswith(b"\x89PNG\r\n\x1a\n"): raise RuntimeError("missing file picker PNG")
            files_path = args.receipt.with_suffix(".files.png")
            files_path.write_bytes(files_screenshot)
            files_screenshot_evidence = {"path":str(files_path), "sha256":hashlib.sha256(files_screenshot).hexdigest()}
            resources_screenshot = subprocess.run(adb + ["exec-out", "run-as", "app.zenpi.android", "cat", "files/headless-resources.png"], capture_output=True, check=True, timeout=10).stdout
            if not resources_screenshot.startswith(b"\x89PNG\r\n\x1a\n"): raise RuntimeError("missing resource page PNG")
            resources_path = args.receipt.with_suffix(".resources.png")
            resources_path.write_bytes(resources_screenshot)
            resources_screenshot_evidence = {"path":str(resources_path), "sha256":hashlib.sha256(resources_screenshot).hexdigest()}
            selection_screenshot = subprocess.run(adb + ["exec-out", "run-as", "app.zenpi.android", "cat", "files/headless-selection.png"], capture_output=True, check=True, timeout=10).stdout
            if not selection_screenshot.startswith(b"\x89PNG\r\n\x1a\n"): raise RuntimeError("missing selection PNG")
            selection_path = args.receipt.with_suffix(".selection.png")
            selection_path.write_bytes(selection_screenshot)
            selection_screenshot_evidence = {"path":str(selection_path), "sha256":hashlib.sha256(selection_screenshot).hexdigest()}
            if args.lan_ip or args.emulator_host_network: require_no_reverse(adb)
            if not passed: raise RuntimeError("independent journal/provider/prompt binding verifier failed")
            if not host.wait(timeout=95) == 0: raise RuntimeError("host expiry failed")
            family = socket.AF_INET6 if ":" in bind_ip else socket.AF_INET
            with socket.socket(family, socket.SOCK_STREAM) as probe:
                probe.settimeout(2)
                if probe.connect_ex(("127.0.0.1" if args.emulator_host_network else bind_ip, port)) == 0: raise RuntimeError("listener still reachable")
        except Exception as error:
            passed = False; failure = str(error)
        finally:
            # Unblock fixture threads on failure without manufacturing completion evidence.
            if not stream_gate.release.is_set():
                with stream_gate.lock: stream_gate.failure = stream_gate.failure or "fixture stopped before Android reattach proof"
                stream_gate.release.set()
            if instrumentation is not None and instrumentation.poll() is None:
                instrumentation.kill(); instrumentation.wait(timeout=5)
            if reader is not None:
                reader.join(timeout=5)
            if lines and not first_output_retained:
                outputs.append("".join(lines).replace(invite, "<redacted-invite>") if invite else "".join(lines))
            if failure and invite:
                failure = failure.replace(invite, "<redacted-invite>")
            if host is not None and host.poll() is None:
                host.terminate()
                try: host.wait(timeout=5)
                except subprocess.TimeoutExpired: host.kill(); host.wait(timeout=5)
            if server is not None: server.shutdown(); server.server_close(); thread.join(timeout=5)
            if port is not None and not args.lan_ip and not args.emulator_host_network: subprocess.run(adb + ["reverse", "--remove", f"tcp:{port}"], capture_output=True, timeout=10)
            current_hashes = {str(path.relative_to(ROOT)): hashlib.sha256(path.read_bytes()).hexdigest() for path in artifacts}
            if current_hashes != hashes: passed = False; failure = "artifacts changed during run"
            if harness_inputs != {name:hashlib.sha256((ROOT/name).read_bytes()).hexdigest() for name in harness_inputs}:
                passed = False; failure = "harness dependencies changed during run"
            try:
                if require_current_build() != build_provenance:
                    raise ValueError("Android source/build provenance changed during device validation")
                if require_current_host(args.binary) != host_provenance:
                    raise ValueError("Host source/build provenance changed during device validation")
            except (ValueError, OSError) as error:
                passed = False; failure = str(error)
            sdk = subprocess.run(adb + ["shell", "getprop", "ro.build.version.sdk"], capture_output=True, text=True, timeout=10).stdout.strip()
            receipt = {"scope":"production non-TUI Rust host / Android composer and saved prompt recovery after actual process death", "passed":passed,
                "direct_host_interface_verified":passed and bool(args.lan_ip), "adb_reverse_used":not bool(args.lan_ip) and not args.emulator_host_network,
                "transport_route":"emulator_host_alias" if args.emulator_host_network else "direct_private_interface" if args.lan_ip else "adb_reverse",
                "emulator_host_network_verified":passed and args.emulator_host_network,
                "wifi_association_setup":wifi_setup,
                "advertised_endpoint":advertised_endpoint,
                "harness_inputs":harness_inputs,
                "camera_scanning_verified":False, "physical_phone_lan_verified":False, "terminal_qr_decode_verified":False,
                "android_composer_provider_turn_verified":passed, "encrypted_pending_prompt_process_recovery_verified":passed,
                "concurrent_authenticated_android_reads_verified":passed,
                "concurrent_authenticated_android_read_count":3 if passed else None,
                "production_listener_connection_cap":4,
                "durable_android_cursor_ack_checkpoint_verified":passed,
                "running_provider_foreground_preview_and_reattach_verified":passed and stream_gate.evidence()["verified"],
                "background_android_writes_fenced_verified":passed and stream_gate.evidence()["verified"],
                "background_android_reads_fenced_verified":passed and stream_gate.evidence()["verified"],
                "android_host_diff_ui_read_only_verified":passed,
                "android_host_file_picker_provider_attachment_verified":passed,
                "android_host_resource_pages_template_provider_verified":passed,
                "android_model_reasoning_selection_provider_verified":passed,
                "exact_retry_without_provider_replay_verified":passed, "provider_requests":len(requests) if server else 0,
                "listener_expiry_closed_verified":passed, "artifacts":hashes, "android_build_provenance":build_provenance, "host_build_provenance":host_provenance, "android_api":sdk, "serial":args.serial,
                "expected_android_api":args.expected_api,
                "started_at_unix":started, "finished_at_unix":time.time(), "failure":failure,
                "command_evidence":command_evidence, "process_evidence":process_evidence, "screenshot":screenshot_evidence,
                "diff_screenshot":diff_screenshot_evidence,
                "files_screenshot":files_screenshot_evidence,
                "resources_screenshot":resources_screenshot_evidence,
                "selection_screenshot":selection_screenshot_evidence,
                "stream_screenshot":stream_screenshot_evidence, "running_stream_evidence":stream_gate.evidence(),
                "requirement":json.loads((ROOT / "Docs/execution/stage2_android2unix_active_requirement.json").read_text()),
                "integration_state":"uncommitted provisional evidence; no whole-item acceptance"}
            args.receipt.write_text(json.dumps(receipt, indent=2) + "\n")
            args.receipt.with_suffix(".instrumentation.txt").write_text("\n".join(outputs))
            print(json.dumps(receipt, indent=2))
    if not passed: raise SystemExit(failure)


if __name__ == "__main__":
    serialized_device_validation(main)

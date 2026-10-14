#!/usr/bin/env python3
"""Operator-assisted physical camera QR + direct LAN production desktop acceptance.

Requires a fresh dedicated test app installation. Never clears existing app data.
Keep this terminal visible; point the phone at its QR and manually enter its SAS.
"""
import argparse
import base64
import hashlib
import json
import os
import re
import shlex
import subprocess
import sys
import tempfile
import time
import threading
import uuid
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import urlsplit

from stage2_android_build import require_current_build
from stage2_host_build import require_current_host
from stage2_android_pair_smoke import ROOT, seed_session, run_instrumentation, serialized_device_validation, InstrumentationFailure
from stage2_desktop_android_smoke import Desktop, private_lan_ip, require_no_reverse, terminal_pixels, prompt_binding_evidence
from tui_user_shell_smoke import drain
from stage2_instrumentation import require_test_results
from stage2_host_smoke import validate_invite

CLASSES = ("zenpi.PhysicalQrHostTest",)
PHASES = ("physical_qr_camera_requested", "physical_qr_confirmation_visible", "physical_qr_invitation_retired_verified", "ungranted_projection_verified",
          "physical_qr_desktop_transcript_verified", "physical_qr_activity_recreation_resume_verified", "desktop_update_requested",
          "physical_qr_desktop_live_update_verified", "background_update_requested",
          "physical_qr_background_cache_fenced_verified", "physical_qr_foreground_replay_verified",
          "physical_qr_android_prompt_answer_verified", "physical_qr_self_revoke_verified", "physical_qr_local_cleanup_verified")
PHONE_PROMPT = "STAGE2_PHYSICAL_ANDROID_PROMPT"
DESKTOP_ANSWER = "STAGE2_PHYSICAL_DESKTOP_ANSWER"
LOCAL_PROMPT = "STAGE2_PHYSICAL_DESKTOP_LOCAL_PROMPT"
LOCAL_ANSWER = "STAGE2_PHYSICAL_DESKTOP_LOCAL_ANSWER"
BACKGROUND_PROMPT = "STAGE2_PHYSICAL_DESKTOP_BACKGROUND_PROMPT"
BACKGROUND_ANSWER = "STAGE2_PHYSICAL_DESKTOP_BACKGROUND_ANSWER"
HARNESS_INPUTS = ("tools/stage2_physical_qr_android_smoke.py", "tools/stage2_instrumentation.py",
    "tools/stage2_desktop_android_smoke.py", "tools/stage2_android_pair_smoke.py", "tools/decode_terminal_qr.swift",
    "tools/stage2_host_smoke.py", "tools/tui_project_workspace_smoke.py", "tools/tui_user_shell_smoke.py",
    "tools/stage2_android_build.py", "tools/stage2_host_build.py")


def digest(path):
    path = Path(path)
    if path.is_symlink() or not path.is_file(): raise ValueError("Evidence source must be a regular file")
    return hashlib.sha256(path.read_bytes()).hexdigest()


def require_new_evidence(path):
    for output in (path, path.with_suffix(".instrumentation.txt")):
        if output.exists() or output.is_symlink():
            raise ValueError("Refusing to overwrite physical QR evidence")


def public_command(address, fingerprint, project):
    return ["am", "instrument", "-w", "-r", "-e", "class", CLASSES[0], "-e", "physical_camera_run", "true",
        "-e", "expected_address", address, "-e", "expected_fingerprint", fingerprint, "-e", "expected_project", project,
        "app.zenpi.android.test/androidx.test.runner.AndroidJUnitRunner"]


def require_qr_invite(decoded, lan_ip, certificate, now=None):
    values = validate_invite(decoded, now=now)
    address = urlsplit("//" + values["address"])
    if private_lan_ip(address.hostname) != lan_ip:
        raise ValueError("QR LAN address mismatch")
    der = base64.b64decode(certificate + "=" * (-len(certificate) % 4), altchars=b"-_", validate=True)
    if hashlib.sha256(der).hexdigest() != values["fingerprint"]:
        raise ValueError("Production QR certificate pin mismatch")
    endpoint = {"host":lan_ip, "port":address.port, "fingerprint":values["fingerprint"]}
    return values, endpoint


def require_grant(records, device, project):
    if not isinstance(records, list) or len(records) not in (2, 3): raise ValueError("Physical registration/grant evidence missing")
    first, granted = records[:2]
    if any(not isinstance(record, dict) or record.get("id") != device or record.get("revoked") is not False
        or type(record.get("revision")) is not int for record in records): raise ValueError("Physical device grant identity mismatch")
    if first.get("projects") != [] or first.get("capabilities") != ["read"] or granted.get("projects") != [project] \
        or granted.get("capabilities") != ["read"] or granted["revision"] != first["revision"] + 1:
        raise ValueError("Physical Read grant mismatch")
    if len(records) == 3 and (records[2].get("projects") != [project] or records[2].get("capabilities") != ["read", "control"]
            or records[2]["revision"] != granted["revision"] + 1): raise ValueError("Physical Control grant mismatch")


def require_prompt_evidence(value, device, project):
    if not isinstance(value, dict) or not isinstance(value.get("session_id"), str) or not value["session_id"]:
        raise ValueError("Missing physical prompt owner")
    if value.get("provider_requests") != 3 or type(value["provider_requests"]) is not int \
            or value.get("provider_prompts") != [LOCAL_PROMPT, BACKGROUND_PROMPT, PHONE_PROMPT] \
            or value.get("provider_answers") != [LOCAL_ANSWER, BACKGROUND_ANSWER, DESKTOP_ANSWER] \
            or value.get("provider_prompt") != PHONE_PROMPT or value.get("provider_answer") != DESKTOP_ANSWER:
        raise ValueError("Physical provider roundtrip mismatch")
    if value.get("original_prefix_sha256") != value.get("preserved_prefix_sha256") \
            or not isinstance(value.get("original_prefix_sha256"), str) or not re.fullmatch(r"[0-9a-f]{64}", value["original_prefix_sha256"]):
        raise ValueError("Physical prompt rewrote original journal")
    turns = value.get("appended_turns")
    if not isinstance(turns, list) or len(turns) != 6 or any(not isinstance(turn, dict) for turn in turns) \
            or [turn.get("role") for turn in turns] != ["user", "assistant"] * 3 \
            or [turn.get("content") for turn in turns] != [LOCAL_PROMPT, LOCAL_ANSWER, BACKGROUND_PROMPT, BACKGROUND_ANSWER, PHONE_PROMPT, DESKTOP_ANSWER] \
            or any(not isinstance(turn.get("id"), str) or not turn["id"] for turn in turns) \
            or len({turn["id"] for turn in turns}) != 6:
        raise ValueError("Physical prompt durable turns mismatch")
    bindings, admissions = value.get("bindings"), value.get("admissions")
    if not isinstance(bindings, list) or not isinstance(admissions, list) \
            or any(not isinstance(record, dict) for record in bindings + admissions) \
            or not prompt_binding_evidence(bindings, admissions, device, project, value["session_id"], expected_count=1):
        raise ValueError("Physical prompt request binding mismatch")
    result = admissions[0].get("result")
    request = bindings[0].get("request_id")
    if not isinstance(request, str) or not re.fullmatch(r"[A-Za-z0-9_-]{1,128}", request) \
            or not isinstance(result, dict) or any(result.get(field) != expected for field, expected in (
                ("request_id", request), ("project_id", project), ("session_id", value["session_id"]))):
        raise ValueError("Physical prompt admission owner mismatch")
    progress = value.get("progress")
    if not isinstance(progress, list) or len(progress) != 2 or any(not isinstance(event, dict) for event in progress) \
            or [event.get("state") for event in progress] != ["running", "succeeded"] \
            or any(event.get("type") != "remote_prompt_progress_v1" or any(event.get(field) != expected
                for field, expected in (("device_id", device), ("request_id", request), ("project_id", project),
                    ("session_id", value["session_id"]))) for event in progress):
        raise ValueError("Physical prompt durable completion mismatch")


def provider_prompt(body):
    if not isinstance(body, dict) or not isinstance(body.get("input"), list):
        raise ValueError("Physical provider input missing")
    users = [item for item in body["input"] if isinstance(item, dict) and item.get("role") == "user"]
    content = users[-1].get("content") if users else None
    if isinstance(content, str): return content
    if isinstance(content, list) and content and all(isinstance(part, dict) and part.get("type") == "input_text"
            and isinstance(part.get("text"), str) for part in content):
        return "".join(part["text"] for part in content)
    raise ValueError("Physical provider user text missing")


def foreground_replay_evidence(output, session):
    value = {}
    for field in ("background_event_cursor", "recovered_event_cursor", "replay_session_id"):
        matches = re.findall(rf"(?m)^INSTRUMENTATION_STATUS: zenpi_{field}=([^\r\n]+)$", output)
        if len(matches) != 1: raise ValueError("Physical foreground replay observation missing or duplicated")
        value[field] = matches[0]
    if value["replay_session_id"] != session: raise ValueError("Physical foreground replay session mismatch")
    cursors = [value[field] for field in ("background_event_cursor", "recovered_event_cursor")]
    if any(not re.fullmatch(r"0|[1-9][0-9]{0,19}", cursor) or int(cursor) > 2**64 - 1 for cursor in cursors) \
            or int(cursors[1]) <= int(cursors[0]): raise ValueError("Physical foreground replay cursor did not advance")
    return value


def require_revoke_evidence(value, output, device, granted):
    requests = re.findall(r"(?m)^INSTRUMENTATION_STATUS: zenpi_physical_revoke_request_id=([^\r\n]+)$", output)
    if len(requests) != 1:
        raise ValueError("Physical revoke request observation missing or duplicated")
    try:
        if str(uuid.UUID(requests[0])) != requests[0]: raise ValueError
    except (ValueError, AttributeError):
        raise ValueError("Physical revoke request identity invalid") from None
    if not isinstance(value, dict) or value.get("id") != device or value.get("revoked") is not True \
            or type(value.get("revision")) is not int or value["revision"] != granted["revision"] + 1 \
            or value.get("revoke_request_id") != requests[0] \
            or value.get("projects") != granted["projects"] or value.get("capabilities") != granted["capabilities"]:
        raise ValueError("Physical durable self-revocation mismatch")


def validate_receipt(path):
    path = Path(path)
    if path.is_symlink() or path.stat().st_size > 8 * 1024 * 1024: raise ValueError("Physical receipt size/path limit")
    receipt = json.loads(path.read_text())
    if not isinstance(receipt, dict) or receipt.get("schema_version") != "stage2-physical-qr/v7" \
        or receipt.get("passed") is not True or receipt.get("failure") is not None \
        or receipt.get("whole_item_accepted") is not False or receipt.get("adb_reverse_used") is not False \
        or any(receipt.get(gate) is not True for gate in ("physical_camera_scan_verified", "physical_phone_lan_verified", "activity_recreation_resume_verified", "desktop_live_update_verified", "foreground_replay_verified", "android_prompt_roundtrip_verified", "self_revoke_verified", "local_cleanup_verified", "fixture_cleanup_verified")):
        raise ValueError("Physical receipt unsuccessful or overclaims acceptance")
    require_physical_device(receipt.get("serial"), receipt.get("emulator_property"), receipt.get("android_api"), receipt.get("expected_android_api"))
    endpoint = receipt.get("endpoint", {})
    ip = private_lan_ip(endpoint.get("host"))
    port = endpoint.get("port")
    fingerprint = endpoint.get("fingerprint"); project = receipt.get("project_id")
    if type(port) is not int or not 1 <= port <= 65535 or not isinstance(fingerprint, str) or not re.fullmatch(r"[0-9a-f]{64}", fingerprint) \
        or not isinstance(project, str) or not re.fullmatch(r"[0-9a-f]{64}", project): raise ValueError("Physical endpoint identity mismatch")
    address = f"[{ip}]:{port}" if ":" in ip else f"{ip}:{port}"
    if receipt.get("argv") != public_command(address, fingerprint, project): raise ValueError("Physical invocation has unexpected or private arguments")
    evidence = receipt.get("instrumentation", {})
    output_path = Path(evidence.get("path", ""))
    if output_path.is_symlink() or output_path.stat().st_size > 4 * 1024 * 1024 \
        or digest(output_path) != evidence.get("sha256"): raise ValueError("Physical output checksum/path mismatch")
    output = output_path.read_text()
    if "zenpi://pair?" in output: raise ValueError("Private invitation leaked into evidence")
    completed = require_result(subprocess.CompletedProcess([], receipt.get("instrumentation_exit_code"), output), receipt.get("phases"))
    if receipt.get("completed_tests") != completed: raise ValueError("Physical completed test identity mismatch")
    if receipt.get("artifacts") != {name:digest(ROOT / name) for name in HARNESS_INPUTS}: raise ValueError("Physical verifier/harness changed")
    for field in ("device_id", "journal_before_sha256", "journal_after_sha256"):
        if not isinstance(receipt.get(field), str) or not re.fullmatch(r"[0-9a-f]{64}", receipt[field]): raise ValueError("Missing physical journal/device evidence")
    if receipt["journal_before_sha256"] != receipt.get("read_only_journal_sha256"): raise ValueError("Physical Read changed journal")
    if receipt["journal_before_sha256"] == receipt["journal_after_sha256"]: raise ValueError("Physical prompt missing journal effect")
    require_grant(receipt.get("grant_records"), receipt["device_id"], project)
    if len(receipt["grant_records"]) != 3: raise ValueError("Physical Control grant missing")
    require_revoke_evidence(receipt.get("revoke_record"), output, receipt["device_id"], receipt["grant_records"][-1])
    require_prompt_evidence(receipt.get("prompt_evidence"), receipt["device_id"], project)
    if receipt.get("foreground_replay_evidence") != foreground_replay_evidence(output, receipt["prompt_evidence"]["session_id"]):
        raise ValueError("Physical foreground replay evidence mismatch")
    if receipt["prompt_evidence"]["original_prefix_sha256"] != receipt["journal_before_sha256"]: raise ValueError("Physical prompt journal binding mismatch")
    seed = receipt.get("seed", {})
    if type(seed.get("exit_code")) is not int or seed["exit_code"] != 0 \
            or type(seed.get("provider_requests")) is not int or seed["provider_requests"] != 1:
        raise ValueError("Physical provider seed unsuccessful")
    if receipt.get("android_build_provenance") != require_current_build(): raise ValueError("Current physical APK provenance mismatch")
    binary = receipt.get("host_build_provenance", {}).get("artifact", {}).get("path", "")
    if receipt["host_build_provenance"] != require_current_host(binary): raise ValueError("Current physical host provenance mismatch")
    return receipt


def require_physical_device(serial, qemu, api, expected_api):
    if not isinstance(serial, str) or not serial or serial.startswith("emulator-") or not isinstance(qemu, str) or qemu.strip() not in ("", "0") \
        or type(api) is not int or type(expected_api) is not int or api != expected_api or api < 31:
        raise ValueError("An explicitly selected physical API 31+ device is required")


def require_fresh_install(adb):
    for package in ("app.zenpi.android", "app.zenpi.android.test"):
        result = subprocess.run(adb + ["shell", "pm", "path", package], capture_output=True, text=True, timeout=10)
        if result.returncode not in (0, 1) or result.stdout.strip() or result.stderr.strip():
            raise ValueError("Dedicated physical device must have neither Zenpi package installed; existing installations are not replaced")


def require_result(result, observed):
    if type(result.returncode) is not int or result.returncode != 0 or observed != list(PHASES):
        raise ValueError("Physical camera workflow incomplete")
    completed = require_test_results(result.stdout, CLASSES, 1)
    phases = re.findall(r"(?m)^INSTRUMENTATION_STATUS: zenpi_phase=([^\r\n]+)$", result.stdout)
    if phases != list(PHASES): raise ValueError("Physical camera phase order mismatch")
    return completed


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--verify-receipt", type=Path)
    parser.add_argument("--serial")
    parser.add_argument("--expected-api", type=int)
    parser.add_argument("--lan-ip", type=private_lan_ip)
    parser.add_argument("--binary", type=Path, default=ROOT / "target/debug/zenpi")
    parser.add_argument("--receipt", type=Path)
    parser.add_argument("--physical-camera-run", action="store_true")
    args = parser.parse_args()
    if args.verify_receipt:
        validate_receipt(args.verify_receipt); print("Physical QR saved evidence verified"); return
    if not args.serial or args.expected_api is None or args.lan_ip is None or args.receipt is None or not args.physical_camera_run:
        parser.error("--serial, --expected-api, --lan-ip, --receipt and --physical-camera-run are required for a device run")
    if not sys.stdout.isatty(): raise ValueError("Visible terminal required for the production QR")
    size = os.get_terminal_size()
    if size.columns < 140 or size.lines < 100: raise ValueError("Enlarge the visible terminal to at least 140 columns by 100 rows")
    provenance = require_current_build(); host_provenance = require_current_host(args.binary)
    require_new_evidence(args.receipt)
    adb = ["adb", "-s", args.serial]
    def prop(name):
        return subprocess.run(adb + ["shell", "getprop", name], check=True, capture_output=True, text=True, timeout=10).stdout.strip()
    api = int(prop("ro.build.version.sdk")); qemu = prop("ro.kernel.qemu")
    require_physical_device(args.serial, qemu, api, args.expected_api)
    require_no_reverse(adb)
    require_fresh_install(adb)
    for path in provenance["artifacts"]:
        subprocess.run(adb + ["install", "-r", str(ROOT / path)], check=True, capture_output=True, timeout=60)
    started = time.time(); terminal = server = provider_thread = None; passed = False; failure = None; output = ""; completed = []; observed = []
    seed = None; device_id = None; command = []; before_hash = after_hash = None; secrets = []; endpoint = None; project = None; exit_code = None; grants = []
    read_hash = None; prompt_evidence = None; replay_evidence = None; revoke_record = None; provider_requests = []; provider_answers = []
    hashes = {name:digest(ROOT / name) for name in HARNESS_INPUTS}
    with tempfile.TemporaryDirectory(prefix="zenpi-physical-qr-") as temporary:
        root = Path(temporary)
        try:
            binary = args.binary.resolve()
            workspace, session, env, project, _, seed = seed_session(binary, root)
            class Handler(BaseHTTPRequestHandler):
                def log_message(self, *args): pass
                def do_POST(self):
                    length = int(self.headers.get("Content-Length", "0"))
                    if not 0 < length <= 1024 * 1024:
                        self.send_error(400); return
                    try:
                        body = json.loads(self.rfile.read(length))
                        prompt = provider_prompt(body)
                    except (ValueError, TypeError):
                        self.send_error(400); return
                    answers = {LOCAL_PROMPT:LOCAL_ANSWER, BACKGROUND_PROMPT:BACKGROUND_ANSWER, PHONE_PROMPT:DESKTOP_ANSWER}
                    if prompt not in answers:
                        self.send_error(400); return
                    answer = answers[prompt]
                    provider_requests.append(body)
                    provider_answers.append(answer)
                    self.send_response(200); self.send_header("Content-Type", "text/event-stream"); self.end_headers()
                    for event in ({"type":"response.output_text.delta", "delta":answer},
                            {"type":"response.completed", "response":{"id":"physical-roundtrip", "status":"completed", "model":body["model"]}}):
                        self.wfile.write(("data: " + json.dumps(event) + "\n\n").encode()); self.wfile.flush()
            server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
            provider_thread = threading.Thread(target=server.serve_forever, daemon=True); provider_thread.start()
            (root / "config/config.toml").write_text('backend="openai"\nmodel="gpt-4.1"\n'
                f'base_url="http://127.0.0.1:{server.server_port}/v1"\nwire_api="responses"\nrequires_openai_auth=false\nmax_retries=0\n')
            env["TERM"] = "xterm-256color"; identity = root / "identity.json"
            terminal = Desktop(binary, workspace, session, identity, env, args.lan_ip, 300)
            image = root / "terminal.pgm"; image.write_bytes(terminal_pixels(bytes(terminal.output)))
            decoded = subprocess.run(["swift", str(ROOT / "tools/decode_terminal_qr.swift"), str(image)],
                check=True, capture_output=True, text=True, timeout=60).stdout.strip()
            secrets = [decoded]
            certificate = json.loads(identity.read_text())["certificate"]
            values, endpoint = require_qr_invite(decoded, args.lan_ip, certificate)
            secrets.append(values["token"])
            address = values["address"]; fingerprint = values["fingerprint"]
            before_bytes = session.read_bytes()
            before_hash = hashlib.sha256(before_bytes).hexdigest()
            sys.stdout.buffer.write(bytes(terminal.output)); sys.stdout.buffer.flush()
            def grant():
                nonlocal device_id
                records = json.loads(identity.with_suffix(".devices.json").read_text())["devices"]
                if len(records) != 1: raise ValueError("Unexpected physical device registration")
                device_id = records[0]["id"]
                fields = ("id", "revoked", "revision", "capabilities", "projects")
                grants.append({field:records[0][field] for field in fields})
                subprocess.run([str(binary), "host", "pair", "--identity", str(identity), "--grant-device-read", device_id, "--project", project],
                    cwd=ROOT, env=env, check=True, capture_output=True, timeout=10)
                granted = json.loads(identity.with_suffix(".devices.json").read_text())["devices"]
                if len(granted) != 1: raise ValueError("Unexpected physical grant registry")
                grants.append({field:granted[0][field] for field in fields})
                require_grant(grants, device_id, project)
                terminal.write(b"\x1b"); terminal.expect(b"Prompt")
            def control():
                nonlocal read_hash
                read_hash = digest(session)
                if read_hash != before_hash: raise ValueError("Physical read changed journal")
                subprocess.run([str(binary), "host", "pair", "--identity", str(identity), "--grant-device-control", device_id, "--project", project],
                    cwd=ROOT, env=env, check=True, capture_output=True, timeout=10)
                records = json.loads(identity.with_suffix(".devices.json").read_text())["devices"]
                if len(records) != 1: raise ValueError("Unexpected physical Control registry")
                grants.append({field:records[0][field] for field in ("id", "revoked", "revision", "capabilities", "projects")})
                require_grant(grants, device_id, project)
                terminal.command(LOCAL_PROMPT, LOCAL_ANSWER.encode())
                terminal.wait(lambda: LOCAL_ANSWER in session.read_text())
            def background():
                terminal.command(BACKGROUND_PROMPT, BACKGROUND_ANSWER.encode())
                terminal.wait(lambda: BACKGROUND_ANSWER in session.read_text())
            # Only public endpoint/pin/project are supplied. No invite/token/SAS
            # reaches Android except through the camera and operator input.
            command = public_command(address, fingerprint, project)
            # The shared runner's workspace callback sees only workspace_*;
            # collect physical phases from its final output as exact identities.
            try:
                result = run_instrumentation(adb, shlex.join(command), grant, lambda: None, update=control,
                    on_background_update=background, tick=lambda: drain(terminal.fd, terminal.output, .01), timeout=360)
            except InstrumentationFailure as error:
                output = error.result.stdout
                exit_code = error.result.returncode
                observed = re.findall(r"(?m)^INSTRUMENTATION_STATUS: zenpi_phase=([^\r\n]+)$", output)
                raise
            output = result.stdout
            exit_code = result.returncode
            observed = re.findall(r"(?m)^INSTRUMENTATION_STATUS: zenpi_phase=([^\r\n]+)$", output)
            completed = require_result(result, observed)
            revoked = json.loads(identity.with_suffix(".devices.json").read_text())["devices"]
            if len(revoked) != 1: raise ValueError("Unexpected physical revoke registry")
            revoke_record = {field:revoked[0].get(field) for field in ("id", "revoked", "revision", "capabilities", "projects", "revoke_request_id")}
            require_revoke_evidence(revoke_record, output, device_id, grants[-1])
            require_no_reverse(adb)
            if device_id is None or read_hash != before_hash: raise ValueError("Physical read/registration incomplete")
            # Android rendering can precede the runner's durable terminal receipt.
            deadline = time.monotonic() + 15
            while True:
                after_bytes = session.read_bytes()
                records = [json.loads(line) for line in after_bytes.splitlines()]
                if any(record.get("event", {}).get("type") == "remote_prompt_progress_v1"
                        and record["event"].get("state") == "succeeded" for record in records): break
                if time.monotonic() >= deadline: raise ValueError("Physical prompt durable completion deadline")
                drain(terminal.fd, terminal.output, .05)
            after_hash = hashlib.sha256(after_bytes).hexdigest()
            if not after_bytes.startswith(before_bytes): raise ValueError("Physical prompt rewrote journal")
            appended = [json.loads(line) for line in after_bytes[len(before_bytes):].splitlines()]
            admissions = [record["event"] for record in appended if record.get("event", {}).get("type") == "remote_prompt_admission_v1"
                and record["event"].get("state") == "accepted"]
            checkpoint = json.loads((session.parent / "project-workspace.json").read_text())
            prompt_evidence = {"session_id":json.loads(before_bytes.splitlines()[0])["session_id"],
                "provider_requests":len(provider_requests), "provider_prompt":provider_prompt(provider_requests[-1]) if provider_requests else None,
                "provider_answer":DESKTOP_ANSWER,
                "provider_prompts":[provider_prompt(body) for body in provider_requests], "provider_answers":provider_answers,
                "original_prefix_sha256":before_hash, "preserved_prefix_sha256":hashlib.sha256(after_bytes[:len(before_bytes)]).hexdigest(),
                "appended_turns":[{field:record["turn"][field] for field in ("id", "role", "content")} for record in appended if record.get("kind") == "turn"],
                "admissions":admissions, "bindings":checkpoint["remote_workspace"]["prompt_bindings"],
                "progress":[record["event"] for record in appended if record.get("event", {}).get("type") == "remote_prompt_progress_v1"]}
            require_prompt_evidence(prompt_evidence, device_id, project)
            replay_evidence = foreground_replay_evidence(output, prompt_evidence["session_id"])
            terminal.expect(DESKTOP_ANSWER.encode(), timeout=15)
            if require_current_build() != provenance or require_current_host(args.binary) != host_provenance: raise ValueError("Build provenance changed")
            if hashes != {name:digest(ROOT / name) for name in hashes}: raise ValueError("Harness changed")
            passed = True
        except Exception as error:
            failure = type(error).__name__ + ": physical camera workflow incomplete"
        finally:
            try:
                subprocess.run(adb + ["shell", "am", "force-stop", "app.zenpi.android"], check=True, capture_output=True, timeout=10)
            except Exception:
                passed = False; failure = "Physical fixture cleanup incomplete"
            finally:
                if terminal is not None:
                    try: terminal.cleanup()
                    except Exception: passed = False; failure = "Desktop fixture cleanup incomplete"
                if server is not None:
                    try:
                        server.shutdown(); server.server_close(); provider_thread.join(timeout=5)
                        if provider_thread.is_alive(): raise ValueError("Provider thread still live")
                    except Exception: passed = False; failure = "Provider fixture cleanup incomplete"
    args.receipt.parent.mkdir(parents=True, exist_ok=True)
    for private in secrets: output = output.replace(private, "<redacted-invite>")
    path = args.receipt.with_suffix(".instrumentation.txt"); path.write_text(output)
    receipt = {"schema_version":"stage2-physical-qr/v7", "passed":passed, "failure":failure,
        "scope":"operator-assisted camera/manual SAS, direct LAN Activity recreation, desktop local/background updates, foreground replay, Android prompt and durable self-revocation",
        "whole_item_accepted":False, "physical_camera_scan_verified":passed, "physical_phone_lan_verified":passed,
        "local_cleanup_verified":passed, "fixture_cleanup_verified":passed, "endpoint":endpoint, "project_id":project, "instrumentation_exit_code":exit_code,
        "activity_recreation_resume_verified":passed, "desktop_live_update_verified":passed, "foreground_replay_verified":passed,
        "foreground_replay_evidence":replay_evidence,
        "android_prompt_roundtrip_verified":passed, "prompt_evidence":prompt_evidence, "read_only_journal_sha256":read_hash,
        "self_revoke_verified":passed, "revoke_record":revoke_record,
        "adb_reverse_used":False, "serial":args.serial, "android_api":api, "expected_android_api":args.expected_api, "emulator_property":qemu,
        "android_build_provenance":provenance, "host_build_provenance":host_provenance, "artifacts":hashes,
        "phases":observed, "completed_tests":completed, "device_id":device_id, "seed":seed,
        "grant_records":grants,
        "journal_before_sha256":before_hash, "journal_after_sha256":after_hash, "argv":command, "cwd":str(ROOT),
        "instrumentation":{"path":str(path), "sha256":hashlib.sha256(path.read_bytes()).hexdigest()},
        "started_at_unix":started, "finished_at_unix":time.time()}
    args.receipt.write_text(json.dumps(receipt, indent=2) + "\n")
    if passed:
        try: validate_receipt(args.receipt)
        except Exception:
            passed = False
            receipt.update(passed=False, failure="Final physical evidence verification failed", physical_camera_scan_verified=False,
                physical_phone_lan_verified=False, activity_recreation_resume_verified=False, desktop_live_update_verified=False, foreground_replay_verified=False, android_prompt_roundtrip_verified=False, self_revoke_verified=False, local_cleanup_verified=False, fixture_cleanup_verified=False)
            args.receipt.write_text(json.dumps(receipt, indent=2) + "\n")
    print("Physical QR validation passed" if passed else "Physical QR validation failed; inspect receipt")
    if not passed: raise SystemExit(1)


if __name__ == "__main__":
    try:
        if any(argument in ("-h", "--help", "--verify-receipt") for argument in sys.argv[1:]): main()
        else: serialized_device_validation(main)
    except (ValueError, OSError, KeyError, TypeError, subprocess.SubprocessError):
        raise SystemExit("Physical QR prerequisites or saved evidence verification failed; no acceptance published") from None

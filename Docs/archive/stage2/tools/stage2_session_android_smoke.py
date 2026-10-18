#!/usr/bin/env python3
"""Current APK session controls against the real desktop owner and terminal QR."""
import argparse
import base64
import hashlib
import json
from pathlib import Path
import re
import shlex
import subprocess
import tempfile
import time
from urllib.parse import parse_qs, urlsplit

from stage2_android_build import require_current_build
from stage2_host_build import require_current_host
from stage2_instrumentation import require_test_results
from stage2_android_pair_smoke import ROOT, seed_session, run_instrumentation, serialized_device_validation
from stage2_desktop_android_smoke import Desktop, private_lan_ip, require_no_reverse, terminal_pixels

PHASES = ("workspace_session_rename_verified", "workspace_session_new_verified", "workspace_session_resume_verified",
          "workspace_session_desktop_switch_requested", "workspace_session_desktop_switch_verified",
          "workspace_session_cancel_reattach_verified", "workspace_session_access_loss_requested",
          "workspace_session_access_loss_verified", "workspace_session_access_restore_verified")
INSTRUMENTATION_CLASSES = ("zenpi.SessionHostTest", "zenpi.SessionScopeScreenTest",
                           "zenpi.SessionNewScreenTest", "zenpi.SessionRenameScreenTest", "zenpi.SessionCatalogScreenTest")
EXPECTED_INSTRUMENTATION_TESTS = 9


def require_workflow_result(result, observed):
    try:
        require_test_results(result.stdout, INSTRUMENTATION_CLASSES, EXPECTED_INSTRUMENTATION_TESTS)
    except ValueError as error:
        raise RuntimeError("Session instrumentation test identities incomplete") from error
    phases = re.findall(r"(?m)^INSTRUMENTATION_STATUS: zenpi_phase=(workspace_session_[a-z_]+)$", result.stdout)
    if result.returncode or not re.search(rf"(?m)^OK \({EXPECTED_INSTRUMENTATION_TESTS} tests\)$", result.stdout) \
        or "FAILURES" in result.stdout or phases != list(PHASES) or observed != list(PHASES):
        raise RuntimeError("Session instrumentation did not prove all phases")
    if re.findall(r"(?m)^INSTRUMENTATION_STATUS: zenpi_session_search_ui=body_snippets_confirmation_offline_verified$", result.stdout) != [
            "INSTRUMENTATION_STATUS: zenpi_session_search_ui=body_snippets_confirmation_offline_verified"]:
        raise RuntimeError("Session search UI regression did not complete")
    marker = "INSTRUMENTATION_STATUS: zenpi_session_body_search=production_host_readonly_resume_verified"
    if re.findall(rf"(?m)^{re.escape(marker)}$", result.stdout) != [marker]:
        raise RuntimeError("Production session body search did not complete")
    for marker in ("INSTRUMENTATION_STATUS: zenpi_session_history_ui=paging_message_return_offline_verified",
                   "INSTRUMENTATION_STATUS: zenpi_session_history_host=production_revision_detail_readonly_verified"):
        if re.findall(rf"(?m)^{re.escape(marker)}$", result.stdout) != [marker]:
            raise RuntimeError("Session conversation history did not complete")


def session_evidence(old, new, checkpoint, project, device):
    """Require actual journal effects and distinct successful mapping receipts."""
    try:
        old_id = old[0]["session_id"]; new_id = new[0]["session_id"]
        if old_id == new_id or old[0]["kind"] != "session" or new[0]["kind"] != "session": return False
        if not any(record.get("turn", {}).get("content") == "STAGE2_ANDROID_REAL_PROVIDER_TRANSCRIPT" for record in old): return False
        if any(record.get("kind") == "turn" for record in new): return False
        events = [record.get("event", {}) for record in old]
        names = [event for event in events if event.get("type") == "session_renamed"]
        if names != [{"type": "session_renamed", "name": "Android development"}]: return False
        rename = [event for event in events if event.get("type") == "remote_session_name_admission_v1"]
        if len(rename) != 2 or [event["state"] for event in rename] != ["reserved", "succeeded"]: return False
        if any(event["device_id"] != device or event["request_id"] != rename[0]["request_id"]
               or event["fingerprint"] != rename[0]["fingerprint"] for event in rename): return False
        result = rename[1]["result"]
        if result["project_id"] != project or result["session_id"] != old_id or result["name"] != "Android development" \
            or result["request_id"] != rename[0]["request_id"] or type(result["journal_revision"]) is not int: return False
        if not re.fullmatch(r"[0-9a-f]{64}", rename[0]["fingerprint"]): return False
        transitions = [record.get("event", {}) for record in new if record.get("event", {}).get("type") == "session_new_transition"]
        if len(transitions) != 1 or transitions[0]["data"]["old_session_id"] != old_id \
            or transitions[0]["data"]["new_session_id"] != new_id or transitions[0]["data"]["route"] != "remote": return False
        remote = checkpoint["remote_workspace"]
        receipts = remote["receipts"]
        if len(receipts) != 2 or len({receipt["request_id"] for receipt in receipts}) != 2: return False
        revisions = []
        for receipt, before, after in zip(receipts, (old_id, new_id), (new_id, old_id)):
            value = receipt["result"]
            if receipt["device_id"] != device or value["request_id"] != receipt["request_id"] \
                or value["project_id"] != project or value["old_session_id"] != before or value["new_session_id"] != after \
                or value["active_project_id"] != project or value["required_owner"] != receipts[0]["result"]["required_owner"]: return False
            if not re.fullmatch(r"[0-9a-f]{64}", value["required_owner"]): return False
            if not re.fullmatch(r"[0-9a-f]{64}", receipt["fingerprint"]): return False
            if type(value["workspace_revision"]) is not int: return False
            revisions.append(value["workspace_revision"])
        if revisions[1] != revisions[0] + 1 or remote["revision"] != revisions[1]: return False
        if transitions[0]["request_id"] != receipts[0]["request_id"]: return False
        if transitions[0]["fingerprint"] != receipts[0]["fingerprint"]: return False
        bindings = remote["prompt_bindings"]
        if len(bindings) != 1 or bindings[0]["operation"] != "session_rename" \
            or any(bindings[0][key] != rename[0][key] for key in ("device_id", "request_id", "fingerprint")) \
            or bindings[0]["project_id"] != project or bindings[0]["session_id"] != old_id: return False
        return True
    except (KeyError, TypeError, IndexError):
        return False


def local_switch_evidence(original, replacement, before, after, project):
    try:
        old_id = original[0]["session_id"]; new_id = replacement[0]["session_id"]
        if original[0]["kind"] != "session" or replacement[0]["kind"] != "session" or old_id == new_id \
            or any(record.get("kind") == "turn" for record in replacement): return False
        transitions = [record["event"] for record in replacement if record.get("event", {}).get("type") == "session_new_transition"]
        if len(transitions) != 1: return False
        data = transitions[0]["data"]
        if data["route"] != "local" or data["old_session_id"] != old_id or data["new_session_id"] != new_id \
            or data["project_id"] != project: return False
        remote = after["remote_workspace"]; previous = before["remote_workspace"]
        if type(remote["revision"]) is not int or type(previous["revision"]) is not int \
            or remote["revision"] != previous["revision"] + 1: return False
        if {key:value for key,value in remote.items() if key != "revision"} \
            != {key:value for key,value in previous.items() if key != "revision"}: return False
        if {key:value for key,value in after.items() if key not in ("remote_workspace", "sessions")} \
            != {key:value for key,value in before.items() if key not in ("remote_workspace", "sessions")}: return False
        if {key:value for key,value in after["sessions"].items() if key != project} \
            != {key:value for key,value in before["sessions"].items() if key != project}: return False
        if after["sessions"][project] != data["new_session_path"] or before["sessions"][project] != data["old_session_path"]: return False
        if data["new_session_path"] == data["old_session_path"]: return False
        return True
    except (KeyError, TypeError, IndexError):
        return False


def access_change_evidence(records, project):
    try:
        if len(records) != 3: return False
        before, removed, restored = records
        if before["projects"] != [project] or removed["projects"] != ["session-access-hidden"] or restored["projects"] != [project]: return False
        if any(record["revoked"] is not False or record["capabilities"] != ["read", "control"] for record in records): return False
        if any(type(record["revision"]) is not int for record in records) \
            or [record["revision"] for record in records] != list(range(before["revision"], before["revision"] + 3)): return False
        return all({key:value for key,value in record.items() if key not in ("projects", "revision")} \
                   == {key:value for key,value in before.items() if key not in ("projects", "revision")} for record in records)
    except (KeyError, TypeError):
        return False


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--serial", required=True)
    parser.add_argument("--expected-api", required=True, type=int)
    parser.add_argument("--binary", type=Path, default=ROOT / "target/debug/zenpi")
    parser.add_argument("--lan-ip", type=private_lan_ip)
    parser.add_argument("--receipt", type=Path, required=True)
    args = parser.parse_args()
    provenance = require_current_build()
    host_provenance = require_current_host(args.binary)
    if any(args.receipt.with_suffix(suffix).exists() for suffix in (".json", ".png", ".auto-switch.png", ".commands.json", ".instrumentation.txt")):
        raise RuntimeError("Refusing to overwrite session evidence")
    binary = args.binary.resolve()
    artifacts = {str(path.relative_to(ROOT)): hashlib.sha256(path.read_bytes()).hexdigest()
                 for path in (binary, ROOT / "tools/stage2_session_android_smoke.py", ROOT / "tools/stage2_instrumentation.py")}
    adb = ["adb", "-s", args.serial]
    api = int(subprocess.run(adb + ["shell", "getprop", "ro.build.version.sdk"], check=True, capture_output=True, text=True, timeout=10).stdout.strip())
    if api != args.expected_api: raise RuntimeError("Android API mismatch")
    if args.lan_ip: require_no_reverse(adb)
    for path in provenance["artifacts"]: subprocess.run(adb + ["install", "-r", str(ROOT / path)], check=True, capture_output=True, timeout=60)
    started = time.time(); terminal = None; port = None; reverse_created = False; passed = False; failure = None
    outputs = ""; screen = auto_screen = evidence = None; command_evidence = []
    with tempfile.TemporaryDirectory(prefix="zenpi-session-device-") as temporary:
        root = Path(temporary)
        try:
            workspace, session, env, project, answer, seed = seed_session(binary, root)
            env["TERM"] = "xterm-256color"
            identity = root / "identity.json"
            terminal = Desktop(binary, workspace, session, identity, env, args.lan_ip or "127.0.0.1", 240)
            image = root / "terminal.pgm"; image.write_bytes(terminal_pixels(bytes(terminal.output)))
            decoded = subprocess.run(["swift", str(ROOT / "tools/decode_terminal_qr.swift"), str(image)], check=True, capture_output=True, text=True, timeout=60).stdout.strip()
            values = parse_qs(urlsplit(decoded).query, strict_parsing=True)
            endpoint = values["address"][0]; port = int(endpoint.rsplit(":", 1)[1])
            if endpoint.rsplit(":", 1)[0].strip("[]") != (args.lan_ip or "127.0.0.1"): raise RuntimeError("QR interface mismatch")
            cert = json.loads(identity.read_text())["certificate"]
            if hashlib.sha256(base64.urlsafe_b64decode(cert + "=" * (-len(cert) % 4))).hexdigest() != values["fingerprint"][0]: raise RuntimeError("QR fingerprint mismatch")
            if not args.lan_ip:
                subprocess.run(adb + ["reverse", f"tcp:{port}", f"tcp:{port}"], check=True, capture_output=True, timeout=10)
                reverse_created = True
            def grant():
                devices = json.loads(identity.with_suffix(".devices.json").read_text())["devices"]
                if len(devices) != 1: raise RuntimeError("Unexpected device count")
                subprocess.run([str(binary), "host", "pair", "--identity", str(identity), "--grant-device-control", devices[0]["id"], "--project", project], check=True, capture_output=True, timeout=10)
                terminal.write(b"\x1b"); terminal.expect(b"Prompt")
            observed = []
            prior_checkpoint = None
            prior_journal = None
            access_records = []
            access_checkpoint = None
            access_journal = None
            access_owner_hashes = None
            def device_record():
                devices = json.loads(identity.with_suffix(".devices.json").read_text())["devices"]
                if len(devices) != 1: raise RuntimeError("Unexpected device count")
                return devices[0]
            def change_access(target):
                record = device_record()
                subprocess.run([str(binary), "host", "pair", "--identity", str(identity), "--grant-device-control", record["id"], "--project", target],
                    cwd=ROOT, env=env, check=True, capture_output=True, timeout=10)
                access_records.append(device_record())
            def desktop_check(line):
                nonlocal prior_checkpoint, prior_journal, access_checkpoint, access_journal, access_owner_hashes
                phase = line.strip().split("zenpi_phase=", 1)[-1]
                if phase not in PHASES: return
                if len(observed) >= len(PHASES): raise RuntimeError("Repeated session phase")
                if phase != PHASES[len(observed)]: raise RuntimeError("Session phase order mismatch")
                if phase in (PHASES[0], PHASES[2]): terminal.expect(b"Android development")
                if phase == PHASES[3]:
                    checkpoint_path = session.parent / "project-workspace.json"
                    prior_checkpoint = json.loads(checkpoint_path.read_text())
                    prior_journal = session.read_bytes()
                    prior_session = prior_checkpoint["sessions"][project]
                    terminal.command("/new")
                    terminal.wait(lambda: json.loads(checkpoint_path.read_text())["sessions"][project] != prior_session)
                if phase == PHASES[6]:
                    access_records.append(device_record())
                    access_checkpoint = session.parent.joinpath("project-workspace.json").read_bytes()
                    current_path = Path(json.loads(access_checkpoint)["sessions"][project])
                    access_journal = current_path.read_bytes()
                    change_access("session-access-hidden")
                if phase == PHASES[7]: change_access(project)
                if phase == PHASES[8]:
                    checkpoint_path = session.parent / "project-workspace.json"
                    if checkpoint_path.read_bytes() != access_checkpoint \
                        or Path(json.loads(access_checkpoint)["sessions"][project]).read_bytes() != access_journal \
                        or not access_change_evidence(access_records, project): raise RuntimeError("Access change altered owner state or device identity")
                    access_owner_hashes = {"checkpoint_before_sha256":hashlib.sha256(access_checkpoint).hexdigest(),
                        "checkpoint_after_sha256":hashlib.sha256(checkpoint_path.read_bytes()).hexdigest(),
                        "journal_before_sha256":hashlib.sha256(access_journal).hexdigest(),
                        "journal_after_sha256":hashlib.sha256(Path(json.loads(access_checkpoint)["sessions"][project]).read_bytes()).hexdigest()}
                observed.append(phase)
            command = ["am", "instrument", "-w", "-r", "-e", "class", ",".join(INSTRUMENTATION_CLASSES),
                "-e", "invite", decoded, "-e", "expected_project", project,
                "-e", "process_namespace", f"session-{time.time_ns()}", "app.zenpi.android.test/androidx.test.runner.AndroidJUnitRunner"]
            command_evidence = [part.replace(decoded, "<redacted-invite>") for part in command]
            result = run_instrumentation(adb, shlex.join(command), grant, lambda: None, on_workspace=desktop_check, timeout=150)
            outputs = result.stdout.replace(decoded, "<redacted-invite>")
            require_workflow_result(result, observed)
            old = [json.loads(line) for line in session.read_text().splitlines()]
            if session.read_bytes() != prior_journal: raise RuntimeError("Local new mutated original journal")
            checkpoint = json.loads(session.parent.joinpath("project-workspace.json").read_text())
            replacement = checkpoint["remote_workspace"]["receipts"][0]["result"]["new_session_id"]
            candidates = [path for path in session.parent.glob("*.jsonl") if json.loads(path.read_text().splitlines()[0]).get("session_id") == replacement]
            if len(candidates) != 1: raise RuntimeError("Replacement journal missing or ambiguous")
            new = [json.loads(line) for line in candidates[0].read_text().splitlines()]
            device = json.loads(identity.with_suffix(".devices.json").read_text())["devices"][0]["id"]
            final_path = Path(checkpoint["sessions"][project])
            if final_path.parent.resolve() != session.parent.resolve() or final_path.is_symlink(): raise RuntimeError("Local replacement path escaped fixture")
            if final_path.resolve() in (session.resolve(), candidates[0].resolve()): raise RuntimeError("Desktop new reused historical session")
            local = [json.loads(line) for line in final_path.read_text().splitlines()]
            if prior_checkpoint is None or Path(prior_checkpoint["sessions"][project]).resolve() != session.resolve() \
                or not session_evidence(old, new, prior_checkpoint, project, device) \
                or not local_switch_evidence(old, local, prior_checkpoint, checkpoint, project): raise RuntimeError("Journal/mapping effects did not match Android controls")
            evidence = {"old": old, "new": new, "local": local, "checkpoint_before_local": prior_checkpoint,
                        "checkpoint": checkpoint, "project": project, "device": device, "seed": seed,
                        "access_records": access_records, "access_owner_hashes": access_owner_hashes}
            screen = subprocess.run(adb + ["exec-out", "run-as", "app.zenpi.android", "cat", "files/session-host.png"], check=True, capture_output=True, timeout=10).stdout
            if not screen.startswith(b"\x89PNG\r\n\x1a\n"): raise RuntimeError("Missing session screenshot")
            auto_screen = subprocess.run(adb + ["exec-out", "run-as", "app.zenpi.android", "cat", "files/session-auto-switch.png"], check=True, capture_output=True, timeout=10).stdout
            if not auto_screen.startswith(b"\x89PNG\r\n\x1a\n"): raise RuntimeError("Missing automatic session-switch screenshot")
            if require_current_build() != provenance: raise RuntimeError("Current APK provenance changed")
            if require_current_host(args.binary) != host_provenance: raise RuntimeError("Current host provenance changed")
            if args.lan_ip: require_no_reverse(adb)
            if artifacts != {str(path.relative_to(ROOT)): hashlib.sha256(path.read_bytes()).hexdigest()
                for path in (binary, ROOT / "tools/stage2_session_android_smoke.py", ROOT / "tools/stage2_instrumentation.py")}:
                raise RuntimeError("Host binary/harness changed during session workflow")
            passed = True
        except Exception as error:
            failure = type(error).__name__ + ": session workflow incomplete"
        finally:
            try:
                subprocess.run(adb + ["shell", "am", "force-stop", "app.zenpi.android"], check=True, capture_output=True, timeout=10)
            finally:
                try:
                    if terminal is not None: terminal.cleanup()
                finally:
                    if reverse_created: subprocess.run(adb + ["reverse", "--remove", f"tcp:{port}"], capture_output=True, timeout=10)
    args.receipt.parent.mkdir(parents=True, exist_ok=True)
    if screen is not None: args.receipt.with_suffix(".png").write_bytes(screen)
    if auto_screen is not None: args.receipt.with_suffix(".auto-switch.png").write_bytes(auto_screen)
    if evidence is not None: args.receipt.with_suffix(".commands.json").write_text(json.dumps(evidence, indent=2) + "\n")
    args.receipt.with_suffix(".instrumentation.txt").write_text(outputs)
    saved = {str(path): hashlib.sha256(path.read_bytes()).hexdigest() for path in
             (args.receipt.with_suffix(".png"), args.receipt.with_suffix(".auto-switch.png"), args.receipt.with_suffix(".commands.json"), args.receipt.with_suffix(".instrumentation.txt")) if path.exists()}
    receipt = {"passed": passed, "scope": "current Android buttons and production desktop canonical session controls",
        "whole_item_accepted": False, "physical_camera_verified": False, "physical_phone_lan_verified": False,
        "terminal_qr_decoded": passed, "direct_host_interface_verified": passed and bool(args.lan_ip), "adb_reverse_used": not bool(args.lan_ip),
        "desktop_local_new_android_automatic_snapshot_verified": passed,
        "android_cancel_retains_pairing_and_explicit_reattach_verified": passed,
        "android_project_access_loss_and_restoration_verified": passed,
        "android_api": api, "expected_android_api": args.expected_api, "android_build_provenance": provenance, "host_build_provenance": host_provenance,
        "artifacts": artifacts, "saved_evidence": saved, "argv": command_evidence, "cwd": str(ROOT),
        "failure": failure, "started_at_unix": started, "finished_at_unix": time.time()}
    args.receipt.write_text(json.dumps(receipt, indent=2) + "\n")
    print(json.dumps(receipt, indent=2))
    if not passed: raise SystemExit("Session workflow failed; inspect failed receipt")


if __name__ == "__main__":
    serialized_device_validation(main)

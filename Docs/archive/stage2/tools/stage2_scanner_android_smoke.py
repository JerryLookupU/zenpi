#!/usr/bin/env python3
"""Bounded production camera-permission denial and scanner result validation."""
import argparse
import hashlib
import json
import re
import subprocess
import time
from pathlib import Path
from stage2_android_pair_smoke import ROOT, serialized_device_validation
from stage2_android_build import require_current_build
from stage2_instrumentation import require_test_results

SCANNER_STATE_COUNT = 28
PERMISSION_CLASSES = ("zenpi.ScannerActivityTest",)
SCANNER_STATE_CLASSES = ("zenpi.ScannerResultTest", "zenpi.PairingScreenTest", "zenpi.DiffScreenTest", "zenpi.DeviceIdentityAdmissionTest", "zenpi.PairingActivityIntentTest")
SUITES = ((PERMISSION_CLASSES, 1), (SCANNER_STATE_CLASSES, SCANNER_STATE_COUNT))
COLD_RESTORE_PHASES = (
    "cold_restore_scanner_ready_verified", "cold_deep_link_confirmation_verified",
    "cold_deep_link_pending_fence_verified", "cold_deep_link_cancel_verified",
    "cold_invitation_validated_queue_verified", "cold_invitation_expired_queue_verified",
    "repair_confirmation_cancel_restores_host_verified", "invalid_replacement_qr_preserves_host_verified",
    "scanner_late_result_fence_verified",
    "scanner_live_read_pause_verified",
    "scanner_raw_payload_fence_verified",
    "registered_host_without_invite_verified",
    "external_pair_intent_single_owner_verified",
    "external_pair_intent_consumed_recreation_verified",
)


def require_instrumentation(result, count, permission=False):
    classes = PERMISSION_CLASSES if permission else SCANNER_STATE_CLASSES
    if type(result.returncode) is not int or result.returncode != 0 or type(count) is not int or count != (1 if permission else SCANNER_STATE_COUNT):
        raise ValueError("scanner instrumentation did not pass expected count")
    completed = require_test_results(result.stdout, classes, count)
    if permission:
        for phase in ("scanner_permission_denial_nonblank_verified", "production_pairing_qr_factory_verified"):
            marker = f"INSTRUMENTATION_STATUS: zenpi_phase={phase}"
            if re.findall(rf"(?m)^{re.escape(marker)}$", result.stdout) != [marker]:
                raise ValueError("production pairing scanner factory and nonblank permission denial were not proved")
    if not permission:
        marker = "INSTRUMENTATION_STATUS: zenpi_invite_monotonic_ui=expired_confirmation_never_revives_verified"
        if re.findall(rf"(?m)^{re.escape(marker)}$", result.stdout) != [marker]:
            raise ValueError("monotonic invitation confirmation expiry was not proved")
        marker = "INSTRUMENTATION_STATUS: zenpi_identity_admission=wiped_mismatched_key_foreground_fenced_verified"
        if re.findall(rf"(?m)^{re.escape(marker)}$", result.stdout) != [marker]:
            raise ValueError("local device identity admission fence was not proved")
        marker = "INSTRUMENTATION_STATUS: zenpi_sync_status=snapshot_tail_offline_status_verified"
        if re.findall(rf"(?m)^{re.escape(marker)}$", result.stdout) != [marker]:
            raise ValueError("synchronization recovery status was not proved")
        marker = "INSTRUMENTATION_STATUS: zenpi_phase=device_confirmation_scope_verified"
        if re.findall(rf"(?m)^{re.escape(marker)}$", result.stdout) != [marker]:
            raise ValueError("device confirmation scanner/host scope was not proved")
    return completed

def require_cold_restore(output):
    if any(len(re.findall(rf"(?m)^INSTRUMENTATION_STATUS: zenpi_phase={phase}$", output)) != 1 for phase in COLD_RESTORE_PHASES):
        raise ValueError("cold foreground scanner restore was not proved")


def validate_receipt(path):
    if path.stat().st_size > 16 * 1024 * 1024:
        raise ValueError("scanner receipt exceeds limit")
    receipt = json.loads(path.read_text())
    if not isinstance(receipt, dict):
        raise ValueError("invalid scanner receipt")
    if (receipt.get("schema_version") != "stage2-scanner-evidence/v2" or receipt.get("passed") is not True
            or receipt.get("whole_item_accepted") is not False or receipt.get("failure") is not None
            or any(receipt.get(gate) is not True for gate in ("actual_system_camera_permission_denial_verified",
                "scanner_cancel_invalid_pending_fences_verified", "cold_foreground_scanner_restore_verified", "diff_dialog_focus_cleared_verified",
                "external_pair_intent_single_owner_verified"))
            or any(receipt.get(gate) is not False for gate in ("physical_camera_scan_verified", "lan_communication_verified"))):
        raise ValueError("scanner receipt is unsuccessful or overclaims acceptance")
    if (type(receipt.get("android_api")) is not int or type(receipt.get("expected_android_api")) is not int
            or receipt["android_api"] < 31 or receipt["android_api"] != receipt["expected_android_api"]):
        raise ValueError("scanner API guard mismatch")
    for key in ("instrumentation", "screenshot"):
        evidence = receipt[key]
        source = Path(evidence["path"])
        if source.stat().st_size > (8 * 1024 * 1024 if key == "instrumentation" else 32 * 1024 * 1024):
            raise ValueError("scanner saved evidence exceeds limit")
        data = source.read_bytes()
        if hashlib.sha256(data).hexdigest() != evidence["sha256"]:
            raise ValueError("scanner saved evidence checksum mismatch")
        if key == "screenshot" and not data.startswith(b"\x89PNG\r\n\x1a\n"):
            raise ValueError("scanner screenshot is not PNG")
    output = Path(receipt["instrumentation"]["path"]).read_text()
    runs = receipt.get("instrumentation_runs")
    if not isinstance(runs, list) or len(runs) != len(SUITES):
        raise ValueError("missing scanner invocation identities")
    for index, (run, (classes, count)) in enumerate(zip(runs, SUITES)):
        if (not isinstance(run, dict) or run.get("classes") != list(classes) or type(run.get("expected_tests")) is not int
                or run["expected_tests"] != count or type(run.get("exit_code")) is not int or run["exit_code"] != 0
                or not isinstance(run.get("output"), str) or len(run["output"].encode()) > 4 * 1024 * 1024
                or run.get("stdout_sha256") != hashlib.sha256(run["output"].encode()).hexdigest()):
            raise ValueError("scanner invocation scope/checksum mismatch")
        completed = require_instrumentation(subprocess.CompletedProcess([], run["exit_code"], run["output"], ""), count, permission=index == 0)
        if run.get("completed_tests") != completed:
            raise ValueError("scanner completed identities mismatch")
        if index == 1: require_cold_restore(run["output"])
    if output != "\n".join(run["output"] for run in runs):
        raise ValueError("scanner saved invocation outputs mismatch")
    verifier = ROOT / "tools/stage2_instrumentation.py"
    if receipt.get("instrumentation_verifier") != {"path":"tools/stage2_instrumentation.py", "sha256":hashlib.sha256(verifier.read_bytes()).hexdigest()}:
        raise ValueError("scanner instrumentation verifier mismatch")
    return receipt


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--serial", required=True)
    parser.add_argument("--expected-api", required=True, type=int)
    parser.add_argument("--receipt", required=True, type=Path)
    args = parser.parse_args()
    build_provenance = require_current_build()
    for path in (args.receipt, args.receipt.with_suffix(".png"), args.receipt.with_suffix(".instrumentation.txt")):
        if path.exists(): raise RuntimeError("Refusing to overwrite existing scanner evidence")
    adb = ["adb", "-s", args.serial]
    api = int(subprocess.run(adb + ["shell", "getprop", "ro.build.version.sdk"], check=True, capture_output=True, text=True, timeout=10).stdout.strip())
    if api != args.expected_api or not args.serial.startswith("emulator-"):
        raise RuntimeError("This permission-mutating fixture requires the expected owned emulator API")
    artifacts = [ROOT / "android/app/build/outputs/apk/debug/app-debug.apk",
        ROOT / "android/app/build/outputs/apk/androidTest/debug/app-debug-androidTest.apk", Path(__file__).resolve()]
    hashes = {str(path.relative_to(ROOT)):hashlib.sha256(path.read_bytes()).hexdigest() for path in artifacts}
    verifier = ROOT / "tools/stage2_instrumentation.py"
    verifier_digest = hashlib.sha256(verifier.read_bytes()).hexdigest()
    for path in artifacts[:2]: subprocess.run(adb + ["install", "-r", str(path)], check=True, timeout=60)
    started = time.time()
    commands = []
    outputs = []
    instrumentation_runs = []
    passed = False
    failure = None
    screenshot = None
    try:
        # Only this emulator's app permission is changed, before any activity runs.
        for operation in (("revoke", "app.zenpi.android", "android.permission.CAMERA"),
                          ("clear-permission-flags", "app.zenpi.android", "android.permission.CAMERA", "user-set", "user-fixed")):
            command = adb + ["shell", "pm", *operation]
            commands.append(command)
            subprocess.run(command, check=True, capture_output=True, timeout=10)
        for classes, count in SUITES:
            command = adb + ["shell", "am", "instrument", "-w", "-r", "-e", "class", ",".join(classes),
                "app.zenpi.android.test/androidx.test.runner.AndroidJUnitRunner"]
            commands.append(command)
            result = subprocess.run(command, capture_output=True, text=True, timeout=120)
            outputs.append(result.stdout)
            completed = require_instrumentation(result, count, permission=classes == PERMISSION_CLASSES)
            instrumentation_runs.append({"classes":list(classes), "expected_tests":count, "exit_code":result.returncode,
                "output":result.stdout, "stdout_sha256":hashlib.sha256(result.stdout.encode()).hexdigest(), "completed_tests":completed})
            if count == SCANNER_STATE_COUNT: require_cold_restore(result.stdout)
        data = subprocess.run(adb + ["exec-out", "run-as", "app.zenpi.android", "cat", "files/scanner-permission-denied.png"],
            check=True, capture_output=True, timeout=10).stdout
        if not data.startswith(b"\x89PNG\r\n\x1a\n"): raise RuntimeError("missing permission denial screenshot")
        args.receipt.parent.mkdir(parents=True, exist_ok=True)
        path = args.receipt.with_suffix(".png")
        path.write_bytes(data)
        screenshot = {"path":str(path), "sha256":hashlib.sha256(data).hexdigest()}
        passed = True
    except subprocess.TimeoutExpired as error:
        outputs.append((error.stdout or b"").decode() if isinstance(error.stdout, bytes) else error.stdout or "")
        failure = "instrumentation observation timeout; fixture app stopped during terminal cleanup"
    except Exception as error:
        failure = str(error)
    finally:
        try:
            subprocess.run(adb + ["shell", "am", "force-stop", "app.zenpi.android"], check=True, capture_output=True, timeout=10)
        except (OSError, subprocess.SubprocessError):
            passed = False; failure = "scanner app cleanup did not complete; inspect the owned emulator"
        if hashes != {str(path.relative_to(ROOT)):hashlib.sha256(path.read_bytes()).hexdigest() for path in artifacts}:
            passed = False; failure = "artifacts changed during validation"
        if hashlib.sha256(verifier.read_bytes()).hexdigest() != verifier_digest:
            passed = False; failure = "instrumentation verifier changed during validation"
        try:
            if require_current_build() != build_provenance:
                raise ValueError("Android source/build provenance changed during device validation")
        except (ValueError, OSError) as error:
            passed = False; failure = str(error)
        args.receipt.parent.mkdir(parents=True, exist_ok=True)
        output_path = args.receipt.with_suffix(".instrumentation.txt")
        output_path.write_text("\n".join(outputs))
        receipt = {"schema_version":"stage2-scanner-evidence/v2", "passed":passed, "android_api":api, "expected_android_api":args.expected_api,
            "scope":"production scanner camera permission denial plus result/state, single-owner intent and dialog UI tests",
            "actual_system_camera_permission_denial_verified":passed,
            "scanner_cancel_invalid_pending_fences_verified":passed,
            "cold_foreground_scanner_restore_verified":passed,
            "diff_dialog_focus_cleared_verified":passed,
            "external_pair_intent_single_owner_verified":passed,
            "physical_camera_scan_verified":False, "lan_communication_verified":False,
            "whole_item_accepted":False, "started_at_unix":started, "finished_at_unix":time.time(),
            "failure":failure, "argv":commands, "cwd":str(ROOT), "artifacts":hashes, "android_build_provenance":build_provenance, "screenshot":screenshot,
            "instrumentation_runs":instrumentation_runs,
            "instrumentation_verifier":{"path":"tools/stage2_instrumentation.py", "sha256":verifier_digest},
            "instrumentation":{"path":str(output_path), "sha256":hashlib.sha256(output_path.read_bytes()).hexdigest()},
            "requirement":json.loads((ROOT / "Docs/execution/stage2_android2unix_active_requirement.json").read_text())}
        args.receipt.write_text(json.dumps(receipt, indent=2) + "\n")
        if passed:
            try:
                validate_receipt(args.receipt)
            except (ValueError, OSError, KeyError, TypeError):
                passed = False; failure = "scanner receipt failed final saved-evidence validation"
                receipt.update(passed=False, failure=failure)
                for gate in ("actual_system_camera_permission_denial_verified", "scanner_cancel_invalid_pending_fences_verified",
                             "cold_foreground_scanner_restore_verified", "diff_dialog_focus_cleared_verified", "external_pair_intent_single_owner_verified"):
                    receipt[gate] = False
                args.receipt.write_text(json.dumps(receipt, indent=2) + "\n")
        print(json.dumps(receipt, indent=2))
    if not passed: raise SystemExit(failure)


if __name__ == "__main__":
    serialized_device_validation(main)

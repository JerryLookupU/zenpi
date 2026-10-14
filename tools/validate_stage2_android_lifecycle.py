#!/usr/bin/env python3
"""Validate a same-artifact API 31/34/35 emulator lifecycle evidence matrix."""
import argparse
import hashlib
import json
import re
from pathlib import Path
from stage2_headless_android_smoke import RunningStreamGate, ack_evidence, prompt_evidence
from stage2_desktop_android_smoke import instrumentation_suite, diff_page_workflow_evidence, process_recovery_evidence
from stage2_instrumentation import require_test_results

APIS = (31, 34, 35)
# The explicit desktop suite also covers file/terminal/task UI boundaries.
DESKTOP_INSTRUMENTATION_CLASSES, DESKTOP_INSTRUMENTATION_TEST_COUNT = instrumentation_suite(True, True)
DESKTOP_GATES = (
    "passed", "terminal_qr_decoded_verified", "lan_interface_tls_verified",
    "controlled_emulator_doze_foreground_resume_verified", "foreground_auto_resume_verified",
    "default_network_loss_resume_verified", "workmanager_read_resume_verified",
    "stopped_network_observation_fenced_verified",
    "actual_process_death_network_resume_verified", "pending_write_process_recovery_verified",
    "actual_process_death_revoke_receipt_verified", "independent_process_cache_restore_verified",
    "android_worktree_diff_ui_verified", "android_worktree_diff_pages_verified", "listener_closed_with_desktop",
    "android_desktop_terminal_open_input_verified",
    "android_desktop_terminal_exit_reopen_verified",
    "local_reset_all_owned_jobs_joined_verified",
    "local_reset_retired_jobs_joined_verified",
    "first_pair_cancel_saved_host_restored_verified",
    "background_pair_saved_host_restored_verified", "network_pair_saved_host_restored_verified",
    "explicit_identity_preconnect_fences_verified",
    "resume_storage_wait_cancellation_fenced_verified",
    "foreground_storage_wait_cancellation_fenced_verified", "foreground_storage_final_admission_fenced_verified",
    "revocation_identity_removal_cancellation_fenced_verified",
    "device_confirmation_scope_verified",
)
HEADLESS_GATES = (
    "passed", "direct_host_interface_verified", "android_composer_provider_turn_verified",
    "encrypted_pending_prompt_process_recovery_verified", "concurrent_authenticated_android_reads_verified",
    "durable_android_cursor_ack_checkpoint_verified", "android_host_diff_ui_read_only_verified",
    "running_provider_foreground_preview_and_reattach_verified", "background_android_writes_fenced_verified",
    "background_android_reads_fenced_verified",
    "android_host_file_picker_provider_attachment_verified",
    "android_host_resource_pages_template_provider_verified",
    "android_model_reasoning_selection_provider_verified",
    "exact_retry_without_provider_replay_verified", "listener_expiry_closed_verified",
)
ARTIFACTS = (
    "target/debug/zenpi", "android/app/build/outputs/apk/debug/app-debug.apk",
    "android/app/build/outputs/apk/androidTest/debug/app-debug-androidTest.apk",
)


def validate_matrix(desktop, headless, transport="direct_private_interface"):
    if transport not in ("direct_private_interface", "emulator_host_alias"):
        raise ValueError("unsupported matrix transport")
    if len(desktop) != 3 or len(headless) != 3:
        raise ValueError("exactly three desktop and three headless receipts required")
    seen = {"desktop":set(), "headless":set()}
    artifacts = None
    for kind, receipts, gates in (("desktop", desktop, DESKTOP_GATES), ("headless", headless, HEADLESS_GATES)):
        for receipt in receipts:
            api = receipt.get("android_api")
            if str(api) not in {str(value) for value in APIS} or type(api) not in (int, str):
                raise ValueError("unsupported API")
            api = int(api)
            if api in seen[kind] or type(receipt.get("expected_android_api")) is not int or receipt["expected_android_api"] != api:
                raise ValueError("duplicate API or missing explicit runtime API guard")
            seen[kind].add(api)
            route_gate = "lan_interface_tls_verified" if kind == "desktop" else "direct_host_interface_verified"
            required = gates if transport == "direct_private_interface" else tuple(gate for gate in gates if gate != route_gate)
            if any(receipt.get(gate) is not True for gate in required):
                raise ValueError(f"missing {kind} behavioral gate for API {api}")
            if receipt.get("adb_reverse_used") is not False:
                raise ValueError("direct-interface evidence cannot use adb reverse")
            if transport == "emulator_host_alias":
                endpoint = receipt.get("advertised_endpoint", "")
                match = re.fullmatch(r"10\.0\.2\.2:([1-9][0-9]{0,4})", endpoint) if isinstance(endpoint, str) else None
                if receipt.get("transport_route") != transport or receipt.get("emulator_host_network_verified") is not True \
                    or receipt.get(route_gate) is not False or not match or int(match[1]) > 65535 \
                    or receipt.get("wifi_association_setup") not in ("shell", "owned_emulator_debug_su") \
                    or not re.fullmatch(r"emulator-[0-9]+", str(receipt.get("serial", ""))):
                    raise ValueError("missing explicit emulator host-alias evidence")
                physical = "physical_lan_peer_verified" if kind == "desktop" else "physical_phone_lan_verified"
                if receipt.get(physical) is not False or receipt.get("camera_scanning_verified") is not False:
                    raise ValueError("emulator host alias cannot claim physical LAN/camera")
            if kind == "desktop":
                if receipt.get("instrumentation_classes") != list(DESKTOP_INSTRUMENTATION_CLASSES) \
                    or type(receipt.get("expected_instrumentation_tests")) is not int \
                    or receipt["expected_instrumentation_tests"] != DESKTOP_INSTRUMENTATION_TEST_COUNT:
                    raise ValueError("incomplete desktop instrumentation suite identity")
                if receipt.get("transport_route") != transport or receipt.get("desktop_provider_requests") != 17:
                    raise ValueError("incomplete desktop provider/route evidence")
                outputs = receipt.get("instrumentation_outputs", [])
                if len(outputs) != 4 or any(not isinstance(output, str) or "FAILURES" in output for output in outputs) \
                    or re.search(rf"(?m)^OK \({DESKTOP_INSTRUMENTATION_TEST_COUNT} tests?\)$", outputs[0]) is None \
                    or "doze_foreground_resume_verified" not in outputs[0]:
                    raise ValueError("missing complete desktop instrumentation evidence")
                output_hashes = ("stdout_sha256", "process_death_stdout_sha256", "network_process_stdout_sha256", "revoke_process_stdout_sha256")
                completed = require_test_results(outputs[0], DESKTOP_INSTRUMENTATION_CLASSES, DESKTOP_INSTRUMENTATION_TEST_COUNT)
                for phase in ("stopped_network_observation_fenced_verified", "local_reset_all_owned_jobs_joined_verified", "local_reset_retired_jobs_joined_verified", "first_pair_cancel_saved_host_restored_verified",
                        "background_pair_saved_host_restored_verified", "network_pair_saved_host_restored_verified",
                        "explicit_identity_preconnect_fences_verified", "resume_storage_wait_cancellation_fenced_verified",
                        "foreground_storage_wait_cancellation_fenced_verified", "foreground_storage_final_admission_fenced_verified",
                        "revocation_identity_removal_cancellation_fenced_verified", "device_confirmation_scope_verified"):
                    marker = "INSTRUMENTATION_STATUS: zenpi_phase=" + phase
                    if re.findall(rf"(?m)^{re.escape(marker)}$", outputs[0]) != [marker]:
                        raise ValueError("Missing local reset/pairing recovery evidence")
                if not diff_page_workflow_evidence(receipt.get("diff_page_workflow_phases"), receipt.get("diff_patch_sha256"), receipt.get("diff_total_bytes"), outputs[0]):
                    raise ValueError("Missing bounded diff page content/change evidence")
                if receipt.get("completed_instrumentation_tests") != completed:
                    raise ValueError("Completed desktop test identities mismatch")
                if receipt.get("process_death_tool_metadata_rehydration_verified") is not True \
                    or receipt.get("process_death_cursor_replay_verified") is not False \
                    or not process_recovery_evidence(outputs[2], True):
                    raise ValueError("Missing authenticated tool-metadata process recovery")
                if any(hashlib.sha256(output.encode()).hexdigest() != receipt.get(key)
                       for output, key in zip(outputs, output_hashes)):
                    raise ValueError("desktop output checksum mismatch")
                if not all(re.fullmatch(r"[1-9][0-9]*", str(receipt.get(key, ""))) for key in ("killed_live_android_pid", "killed_live_revoke_android_pid")) \
                    or receipt["killed_live_android_pid"] == receipt["killed_live_revoke_android_pid"]:
                    raise ValueError("distinct live killed process identities required")
                killed, revoked = str(receipt["killed_live_android_pid"]), str(receipt["killed_live_revoke_android_pid"])
                markers = (
                    ("network_process_live_ready", f"network_process_pid={killed}", "Process crashed."),
                    ("pending_write_process_recovery_verified", f"network_process_pid={revoked}", "Process crashed."),
                    ("revoke_receipt_process_recovery_verified", "android_self_revoke_verified", "host_state_verified", "OK (1 test)"),
                )
                if type(receipt.get("instrumentation_exit_code")) is not int or receipt["instrumentation_exit_code"] != 0 or any(
                    marker not in output for output, required in zip(outputs[1:], markers) for marker in required
                ):
                    raise ValueError("desktop killed/recovered process output mismatch")
                recovered = re.findall(r"recovered_revoke_process_pid=([1-9][0-9]*)", outputs[3])
                if len(recovered) != 1 or recovered[0] in (killed, revoked):
                    raise ValueError("distinct recovered revoke process required")
            else:
                stream = receipt.get("running_stream_evidence", {})
                if stream != {"events":["provider_preview_flushed", *RunningStreamGate.phases, "provider_completed_flushed"],
                              "failure":None, "verified":True}:
                    raise ValueError("missing ordered running-stream/background-fence evidence")
                if receipt.get("provider_requests") != 2:
                    raise ValueError("headless exact-retry provider count")
                process = receipt.get("process_evidence", {})
                if process.get("pending_command_recovered_in_distinct_process") is not True or process.get("recovery_exit_code") != 0 \
                    or not re.fullmatch(r"[1-9][0-9]*", str(process.get("killed_pid", ""))):
                    raise ValueError("headless actual process-death evidence required")
            hashes = {key:receipt.get("artifacts", {}).get(key) for key in ARTIFACTS}
            if any(not isinstance(value, str) or len(value) != 64 or any(c not in "0123456789abcdef" for c in value) for value in hashes.values()):
                raise ValueError("missing artifact identity")
            if artifacts is None:
                artifacts = hashes
            elif hashes != artifacts:
                raise ValueError("matrix must use identical Rust/app/test APK artifacts")
    return {"apis":list(APIS), "artifacts":artifacts, "emulator_lifecycle_matrix_verified":True,
        "transport_route":transport, "direct_host_interface_matrix_verified":transport == "direct_private_interface",
        "emulator_host_network_matrix_verified":transport == "emulator_host_alias",
        "physical_phone_lan_verified":False, "camera_scanning_verified":False,
        "whole_item_accepted":False}


def validate_saved_headless(receipt):
    evidence = receipt["command_evidence"]
    command = Path(evidence["path"])
    if hashlib.sha256(command.read_bytes()).hexdigest() != evidence["sha256"]:
        raise ValueError("headless saved command checksum mismatch")
    saved = json.loads(command.read_text())
    if saved["committed_request_id"] != receipt["process_evidence"]["committed_request_id"]:
        raise ValueError("headless saved request identity mismatch")
    if not prompt_evidence(saved["requests"], saved["records"], saved["checkpoint"], saved["project_id"], saved["device_id"], saved["committed_request_id"]) \
        or not ack_evidence(saved["records"], saved["checkpoint"], saved["project_id"], saved["device_id"]):
        raise ValueError("headless provider/journal/ACK evidence mismatch")


def validate_saved_assets(receipt):
    assets = []
    if receipt.get("running_provider_foreground_preview_and_reattach_verified") is True and not isinstance(receipt.get("stream_screenshot"), dict):
        raise ValueError("running-stream screenshot required")
    if receipt.get("android_host_file_picker_provider_attachment_verified") is True and not isinstance(receipt.get("files_screenshot"), dict):
        raise ValueError("file picker screenshot required")
    if receipt.get("android_host_resource_pages_template_provider_verified") is True and not isinstance(receipt.get("resources_screenshot"), dict):
        raise ValueError("resource page screenshot required")
    if receipt.get("android_model_reasoning_selection_provider_verified") is True and not isinstance(receipt.get("selection_screenshot"), dict):
        raise ValueError("model selection screenshot required")
    for key in ("screenshot", "diff_screenshot", "stream_screenshot", "files_screenshot", "resources_screenshot", "selection_screenshot"):
        if key not in receipt:
            continue
        evidence = receipt[key]
        path = Path(evidence["path"])
        data = path.read_bytes()
        if not data.startswith(b"\x89PNG\r\n\x1a\n") or hashlib.sha256(data).hexdigest() != evidence["sha256"]:
            raise ValueError("saved screenshot identity mismatch")
        assets.append({"path":str(path), "sha256":evidence["sha256"]})
    return assets


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--desktop", type=Path, nargs=3, required=True)
    parser.add_argument("--headless", type=Path, nargs=3, required=True)
    parser.add_argument("--receipt", type=Path, help="Write the verified matrix receipt; never overwrites existing evidence")
    parser.add_argument("--transport", choices=("direct_private_interface", "emulator_host_alias"), default="direct_private_interface")
    args = parser.parse_args()
    receipts = [json.loads(path.read_text()) for path in args.desktop + args.headless]
    result = validate_matrix(receipts[:3], receipts[3:], args.transport)
    assets = []
    for receipt in receipts:
        assets.extend(validate_saved_assets(receipt))
    for path in args.headless:
        validate_saved_headless(json.loads(path.read_text()))
    result["receipts"] = [{"path":str(path), "sha256":hashlib.sha256(path.read_bytes()).hexdigest()} for path in args.desktop + args.headless]
    result["saved_assets"] = assets
    result["validator_sha256"] = hashlib.sha256(Path(__file__).read_bytes()).hexdigest()
    if args.receipt:
        with args.receipt.open("x") as output:
            output.write(json.dumps(result, indent=2) + "\n")
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()

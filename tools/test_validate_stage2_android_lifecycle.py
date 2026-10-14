import copy
import hashlib
import json
import tempfile
import unittest
from pathlib import Path
import test_stage2_headless_android_smoke as headless_tests
from stage2_headless_android_smoke import RunningStreamGate
from test_stage2_instrumentation import output_for
from stage2_instrumentation import expected_tests
from stage2_desktop_android_smoke import DIFF_PAGE_PHASES
from validate_stage2_android_lifecycle import APIS, ARTIFACTS, DESKTOP_GATES, HEADLESS_GATES, DESKTOP_INSTRUMENTATION_CLASSES, DESKTOP_INSTRUMENTATION_TEST_COUNT, validate_matrix, validate_saved_headless, validate_saved_assets


class MatrixTests(unittest.TestCase):
    def fixture(self):
        desktop, headless = [], []
        for api in APIS:
            shared = {"android_api":api, "expected_android_api":api, "adb_reverse_used":False,
                "artifacts":{key:"a" * 64 for key in ARTIFACTS}}
            diff_output = "\n".join([f"INSTRUMENTATION_STATUS: zenpi_phase={phase}" for phase in DIFF_PAGE_PHASES]
                + ["INSTRUMENTATION_STATUS: zenpi_diff_patch_sha256=" + "b" * 64, "INSTRUMENTATION_STATUS: zenpi_diff_total_bytes=100000"])
            outputs = [output_for(DESKTOP_INSTRUMENTATION_CLASSES) + "doze_foreground_resume_verified\n"
                + "INSTRUMENTATION_STATUS: zenpi_phase=stopped_network_observation_fenced_verified\n"
                + "INSTRUMENTATION_STATUS: zenpi_phase=local_reset_all_owned_jobs_joined_verified\n"
                + "INSTRUMENTATION_STATUS: zenpi_phase=local_reset_retired_jobs_joined_verified\n"
                + "INSTRUMENTATION_STATUS: zenpi_phase=first_pair_cancel_saved_host_restored_verified\n"
                + "INSTRUMENTATION_STATUS: zenpi_phase=background_pair_saved_host_restored_verified\n"
                + "INSTRUMENTATION_STATUS: zenpi_phase=network_pair_saved_host_restored_verified\n"
                + "INSTRUMENTATION_STATUS: zenpi_phase=explicit_identity_preconnect_fences_verified\n"
                + "INSTRUMENTATION_STATUS: zenpi_phase=resume_storage_wait_cancellation_fenced_verified\n"
                + "INSTRUMENTATION_STATUS: zenpi_phase=foreground_storage_wait_cancellation_fenced_verified\n"
                + "INSTRUMENTATION_STATUS: zenpi_phase=foreground_storage_final_admission_fenced_verified\n"
                + "INSTRUMENTATION_STATUS: zenpi_phase=revocation_identity_removal_cancellation_fenced_verified\n"
                + "INSTRUMENTATION_STATUS: zenpi_phase=device_confirmation_scope_verified\n" + diff_output,
                "network_process_live_ready network_process_pid=123 Process crashed.",
                "pending_write_process_recovery_verified network_process_pid=456 Process crashed.\n"
                "INSTRUMENTATION_STATUS: zenpi_phase=network_process_tool_metadata_rehydrated_verified\n"
                "INSTRUMENTATION_STATUS: network_process_seed_cursor=28\n"
                "INSTRUMENTATION_STATUS: network_process_recovered_cursor=30\n",
                "recovered_revoke_process_pid=789 revoke_receipt_process_recovery_verified android_self_revoke_verified host_state_verified OK (1 test)"]
            desktop.append(shared | {gate:True for gate in DESKTOP_GATES} | {"transport_route":"direct_private_interface",
                "desktop_provider_requests":17, "instrumentation_outputs":outputs,
                "diff_page_workflow_phases":list(DIFF_PAGE_PHASES), "diff_patch_sha256":"b" * 64, "diff_total_bytes":100000,
                "instrumentation_classes":list(DESKTOP_INSTRUMENTATION_CLASSES),
                "expected_instrumentation_tests":DESKTOP_INSTRUMENTATION_TEST_COUNT,
                "completed_instrumentation_tests":sorted(f"{name}#{method}" for name, method in expected_tests(DESKTOP_INSTRUMENTATION_CLASSES)),
                "killed_live_android_pid":"123", "killed_live_revoke_android_pid":"456",
                "instrumentation_exit_code":0,
                "process_death_tool_metadata_rehydration_verified":True, "process_death_cursor_replay_verified":False,
                **{key:hashlib.sha256(output.encode()).hexdigest() for key, output in zip(
                    ("stdout_sha256", "process_death_stdout_sha256", "network_process_stdout_sha256", "revoke_process_stdout_sha256"), outputs)}})
            headless.append(shared | {gate:True for gate in HEADLESS_GATES} | {"provider_requests":2,
                "running_stream_evidence":{"events":["provider_preview_flushed", *RunningStreamGate.phases, "provider_completed_flushed"],
                    "failure":None, "verified":True},
                "process_evidence":{"killed_pid":"123", "pending_command_recovered_in_distinct_process":True, "recovery_exit_code":0}})
        return desktop, headless

    def test_full_matrix_passes(self):
        self.assertEqual(validate_matrix(*self.fixture())["apis"], list(APIS))

    def alias_fixture(self):
        value = self.fixture()
        for index, receipts in enumerate(value):
            for receipt in receipts:
                receipt.update(transport_route="emulator_host_alias", emulator_host_network_verified=True,
                    advertised_endpoint="10.0.2.2:1234", wifi_association_setup="shell", serial="emulator-5584",
                    camera_scanning_verified=False)
                receipt["lan_interface_tls_verified" if index == 0 else "direct_host_interface_verified"] = False
                receipt["physical_lan_peer_verified" if index == 0 else "physical_phone_lan_verified"] = False
        return value

    def test_explicit_alias_matrix_retains_all_workflows_without_physical_credit(self):
        result = validate_matrix(*self.alias_fixture(), transport="emulator_host_alias")
        self.assertTrue(result["emulator_host_network_matrix_verified"])
        self.assertFalse(result["direct_host_interface_matrix_verified"])
        self.assertFalse(result["physical_phone_lan_verified"])
        with self.assertRaises(ValueError): validate_matrix(*self.alias_fixture())
        with self.assertRaises(ValueError): validate_matrix(*self.fixture(), transport="emulator_host_alias")

    def test_alias_refuses_forged_routes_reverse_and_physical_claims(self):
        for index in (0,1):
            route_gate = "lan_interface_tls_verified" if index == 0 else "direct_host_interface_verified"
            physical = "physical_lan_peer_verified" if index == 0 else "physical_phone_lan_verified"
            for field, wrong in (("advertised_endpoint","192.168.50.14:1234"), ("advertised_endpoint","10.0.2.2:65536"),
                ("advertised_endpoint","10.0.2.2:01234"), ("emulator_host_network_verified",False), (route_gate,True),
                (physical,True), ("camera_scanning_verified",True), ("adb_reverse_used",True), ("serial","physical"),
                ("wifi_association_setup",None), ("transport_route","adb_reverse")):
                value = self.alias_fixture(); value[index][0][field] = wrong
                with self.subTest(index=index,field=field,wrong=wrong), self.assertRaises(ValueError):
                    validate_matrix(*value, transport="emulator_host_alias")
        value = self.alias_fixture(); value[0][0]["android_worktree_diff_pages_verified"] = False
        with self.assertRaises(ValueError): validate_matrix(*value, transport="emulator_host_alias")

    def test_process_rehydration_requires_exact_branch_and_advancing_cursor(self):
        for transport in ("direct_private_interface", "emulator_host_alias"):
            for wrong in ("", "INSTRUMENTATION_STATUS: network_process_seed_cursor=28\n",
                "INSTRUMENTATION_STATUS: zenpi_phase=network_process_cursor_replay_verified\n"
                "INSTRUMENTATION_STATUS: network_process_seed_cursor=28\nINSTRUMENTATION_STATUS: network_process_recovered_cursor=30\n"):
                value = self.fixture() if transport == "direct_private_interface" else self.alias_fixture()
                value[0][0]["instrumentation_outputs"][2] = wrong
                value[0][0]["network_process_stdout_sha256"] = hashlib.sha256(wrong.encode()).hexdigest()
                with self.assertRaises(ValueError): validate_matrix(*value, transport=transport)

    def test_local_reset_gate_requires_single_actual_join_phase_not_just_boolean(self):
        marker = "INSTRUMENTATION_STATUS: zenpi_phase=local_reset_all_owned_jobs_joined_verified"
        for replacement in ("", marker + "\n" + marker, "local_reset_all_owned_jobs_joined_verified",
                "INSTRUMENTATION_STATUS: zenpi_phase=local_reset_read_jobs_joined_verified"):
            value = self.fixture()
            output = value[0][0]["instrumentation_outputs"][0].replace(marker, replacement)
            value[0][0]["instrumentation_outputs"][0] = output
            value[0][0]["stdout_sha256"] = hashlib.sha256(output.encode()).hexdigest()
            with self.assertRaises(ValueError): validate_matrix(*value)

    def test_old_reset_receipt_boolean_cannot_replace_all_owned_jobs_gate(self):
        value = self.fixture()
        for receipt in value[0]:
            receipt.pop("local_reset_all_owned_jobs_joined_verified")
            receipt["local_reset_read_jobs_joined_verified"] = True
        with self.assertRaises(ValueError): validate_matrix(*value)

    def test_retired_job_gate_requires_one_exact_phase(self):
        marker = "INSTRUMENTATION_STATUS: zenpi_phase=local_reset_retired_jobs_joined_verified"
        for replacement in ("", marker + "\n" + marker, "local_reset_retired_jobs_joined_verified"):
            value = self.fixture()
            output = value[0][0]["instrumentation_outputs"][0].replace(marker, replacement)
            value[0][0]["instrumentation_outputs"][0] = output
            value[0][0]["stdout_sha256"] = hashlib.sha256(output.encode()).hexdigest()
            with self.assertRaises(ValueError): validate_matrix(*value)

    def test_first_pair_cancel_requires_exact_reconciled_host_phase(self):
        marker = "INSTRUMENTATION_STATUS: zenpi_phase=first_pair_cancel_saved_host_restored_verified"
        for replacement in ("", marker + "\n" + marker, "first_pair_cancel_saved_host_restored_verified"):
            value = self.fixture()
            output = value[0][0]["instrumentation_outputs"][0].replace(marker, replacement)
            value[0][0]["instrumentation_outputs"][0] = output
            value[0][0]["stdout_sha256"] = hashlib.sha256(output.encode()).hexdigest()
            with self.assertRaises(ValueError): validate_matrix(*value)

    def test_background_storage_cancellation_boolean_is_required(self):
        value = self.fixture()
        value[0][0].pop("resume_storage_wait_cancellation_fenced_verified")
        with self.assertRaises(ValueError): validate_matrix(*value)

    def test_foreground_storage_wait_and_final_admission_gates_are_required(self):
        for gate in ("foreground_storage_wait_cancellation_fenced_verified", "foreground_storage_final_admission_fenced_verified",
                     "revocation_identity_removal_cancellation_fenced_verified", "device_confirmation_scope_verified"):
            value = self.fixture()
            value[0][0].pop(gate)
            with self.subTest(gate=gate), self.assertRaises(ValueError): validate_matrix(*value)

    def test_pairing_background_and_network_recovery_require_actual_phase(self):
        for phase in ("stopped_network_observation_fenced_verified", "background_pair_saved_host_restored_verified", "network_pair_saved_host_restored_verified",
                "explicit_identity_preconnect_fences_verified", "resume_storage_wait_cancellation_fenced_verified",
                "foreground_storage_wait_cancellation_fenced_verified", "foreground_storage_final_admission_fenced_verified",
                "revocation_identity_removal_cancellation_fenced_verified", "device_confirmation_scope_verified"):
            marker = "INSTRUMENTATION_STATUS: zenpi_phase=" + phase
            for replacement in ("", marker + "\n" + marker, phase):
                value = self.fixture()
                output = value[0][0]["instrumentation_outputs"][0].replace(marker, replacement)
                value[0][0]["instrumentation_outputs"][0] = output
                value[0][0]["stdout_sha256"] = hashlib.sha256(output.encode()).hexdigest()
                with self.assertRaises(ValueError): validate_matrix(*value)

    def test_diff_pages_require_content_digest_order_and_changed_document_refusal(self):
        for field, wrong in (("diff_patch_sha256", "c" * 64), ("diff_total_bytes", 65536),
                             ("diff_page_workflow_phases", list(reversed(DIFF_PAGE_PHASES)))):
            value = self.fixture(); value[0][0][field] = wrong
            with self.assertRaises(ValueError): validate_matrix(*value)
        value = self.fixture()
        value[0][0]["instrumentation_outputs"][0] = value[0][0]["instrumentation_outputs"][0].replace(DIFF_PAGE_PHASES[-1], "missing")
        value[0][0]["stdout_sha256"] = hashlib.sha256(value[0][0]["instrumentation_outputs"][0].encode()).hexdigest()
        with self.assertRaises(ValueError): validate_matrix(*value)

    def test_count_and_checksum_without_named_completions_never_prove_suite(self):
        value = self.fixture()
        value[0][0]["instrumentation_outputs"][0] = f"OK ({DESKTOP_INSTRUMENTATION_TEST_COUNT} tests)\ndoze_foreground_resume_verified"
        value[0][0]["stdout_sha256"] = hashlib.sha256(value[0][0]["instrumentation_outputs"][0].encode()).hexdigest()
        with self.assertRaises(ValueError): validate_matrix(*value)
        value = self.fixture(); value[0][0]["completed_instrumentation_tests"] = []
        with self.assertRaises(ValueError): validate_matrix(*value)

    def test_old_reduced_suite_or_wrong_suite_identity_never_passes(self):
        for field, wrong in (("instrumentation_classes", []),
            ("instrumentation_classes", list(reversed(DESKTOP_INSTRUMENTATION_CLASSES))),
            ("expected_instrumentation_tests", 44), ("expected_instrumentation_tests", "59")):
            value = self.fixture(); value[0][0][field] = wrong
            with self.assertRaises(ValueError): validate_matrix(*value)
        value = self.fixture()
        value[0][0]["instrumentation_outputs"][0] = "OK (44 tests)\ndoze_foreground_resume_verified"
        value[0][0]["stdout_sha256"] = hashlib.sha256(value[0][0]["instrumentation_outputs"][0].encode()).hexdigest()
        with self.assertRaises(ValueError): validate_matrix(*value)

    def test_running_stream_boolean_without_ordered_background_fence_is_not_evidence(self):
        for wrong in ({}, {"verified":True}, {"events":[], "failure":None, "verified":True}):
            value = self.fixture(); value[1][0]["running_stream_evidence"] = wrong
            with self.assertRaises(ValueError): validate_matrix(*value)
        value = self.fixture(); value[1][0]["running_stream_evidence"]["events"].remove("headless_background_writes_fenced_verified")
        with self.assertRaises(ValueError): validate_matrix(*value)
        value = self.fixture(); value[1][0]["running_stream_evidence"]["events"].reverse()
        with self.assertRaises(ValueError): validate_matrix(*value)

    def test_missing_or_false_gate_fails(self):
        for index, gates in enumerate((DESKTOP_GATES, HEADLESS_GATES)):
            for gate in gates:
                value = self.fixture(); value[index][0][gate] = False
                with self.assertRaises(ValueError): validate_matrix(*value)

    def test_old_write_only_background_proof_does_not_prove_read_fencing(self):
        value = self.fixture()
        value[1][0]["running_stream_evidence"]["events"].remove("headless_background_reads_fenced_verified")
        with self.assertRaises(ValueError): validate_matrix(*value)

    def test_wrong_api_duplicate_or_no_guard_fails(self):
        for field, wrong in (("android_api", 36), ("expected_android_api", None), ("android_api", True)):
            value = self.fixture(); value[0][0][field] = wrong
            with self.assertRaises(ValueError): validate_matrix(*value)
        value = self.fixture(); value[1][1] = copy.deepcopy(value[1][0])
        with self.assertRaises(ValueError): validate_matrix(*value)

    def test_mixed_artifacts_reverse_or_provider_replay_fails(self):
        for field, wrong in (("adb_reverse_used", True), ("provider_requests", 3)):
            value = self.fixture(); value[1][0][field] = wrong
            with self.assertRaises(ValueError): validate_matrix(*value)
        value = self.fixture(); value[0][1]["artifacts"][ARTIFACTS[0]] = "b" * 64
        with self.assertRaises(ValueError): validate_matrix(*value)

    def test_missing_count_tampered_output_or_wrong_route_fails(self):
        value = self.fixture(); value[0].pop()
        with self.assertRaises(ValueError): validate_matrix(*value)
        for field, wrong in (("transport_route", "adb_reverse"), ("desktop_provider_requests", 16), ("stdout_sha256", "b" * 64),
            ("instrumentation_outputs", [])):
            value = self.fixture(); value[0][0][field] = wrong
            with self.assertRaises(ValueError): validate_matrix(*value)

    def test_failed_output_or_invented_process_identity_fails(self):
        value = self.fixture(); value[0][0]["instrumentation_outputs"][1] = "FAILURES"
        with self.assertRaises(ValueError): validate_matrix(*value)
        value = self.fixture(); value[0][0]["killed_live_revoke_android_pid"] = "123"
        with self.assertRaises(ValueError): validate_matrix(*value)
        for field, wrong in (("killed_pid", "none"), ("pending_command_recovered_in_distinct_process", False), ("recovery_exit_code", 1)):
            value = self.fixture(); value[1][0]["process_evidence"][field] = wrong
            with self.assertRaises(ValueError): validate_matrix(*value)

    def test_recovery_output_must_match_distinct_killed_processes(self):
        for index, output in ((1, "Process crashed."), (2, "pending_write_process_recovery_verified network_process_pid=999 Process crashed."),
                              (3, "recovered_revoke_process_pid=123 revoke_receipt_process_recovery_verified android_self_revoke_verified host_state_verified OK (1 test)")):
            value = self.fixture(); value[0][0]["instrumentation_outputs"][index] = output
            key = ("stdout_sha256", "process_death_stdout_sha256", "network_process_stdout_sha256", "revoke_process_stdout_sha256")[index]
            value[0][0][key] = hashlib.sha256(output.encode()).hexdigest()
            with self.assertRaises(ValueError): validate_matrix(*value)

    def test_saved_command_checksum_identity_and_ack_are_verified(self):
        requests, _, _ = headless_tests.EvidenceTests().fixture()
        records, checkpoint = headless_tests.AckEvidenceTests().fixture()
        saved = {"requests":requests, "records":records, "checkpoint":checkpoint,
            "project_id":"project", "device_id":"device", "committed_request_id":"prompt-1"}
        with tempfile.TemporaryDirectory() as directory:
            command = Path(directory) / "commands.json"
            command.write_text(json.dumps(saved))
            receipt = {"command_evidence":{"path":str(command), "sha256":hashlib.sha256(command.read_bytes()).hexdigest()},
                "process_evidence":{"committed_request_id":"prompt-1"}}
            validate_saved_headless(receipt)
            receipt["process_evidence"]["committed_request_id"] = "foreign"
            with self.assertRaises(ValueError): validate_saved_headless(receipt)
            receipt["process_evidence"]["committed_request_id"] = "prompt-1"
            saved["checkpoint"]["remote_workspace"]["event_acks"][0]["cursor"]["next_sequence"] = 1
            command.write_text(json.dumps(saved))
            with self.assertRaises(ValueError): validate_saved_headless(receipt)
            receipt["command_evidence"]["sha256"] = hashlib.sha256(command.read_bytes()).hexdigest()
            with self.assertRaises(ValueError): validate_saved_headless(receipt)

    def test_saved_screenshot_hash_and_png_signature_are_verified(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "screenshot.png"
            path.write_bytes(b"\x89PNG\r\n\x1a\nfixture")
            receipt = {"screenshot":{"path":str(path), "sha256":hashlib.sha256(path.read_bytes()).hexdigest()}}
            self.assertEqual(len(validate_saved_assets(receipt)), 1)
            path.write_bytes(b"tampered")
            with self.assertRaises(ValueError): validate_saved_assets(receipt)
            receipt["screenshot"]["sha256"] = hashlib.sha256(path.read_bytes()).hexdigest()
            with self.assertRaises(ValueError): validate_saved_assets(receipt)

    def test_stream_screenshot_is_checked_not_ignored(self):
        with self.assertRaises(ValueError):
            validate_saved_assets({"running_provider_foreground_preview_and_reattach_verified":True})
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "stream.png"
            path.write_bytes(b"\x89PNG\r\n\x1a\nfixture")
            receipt = {"stream_screenshot":{"path":str(path), "sha256":hashlib.sha256(path.read_bytes()).hexdigest()}}
            self.assertEqual(len(validate_saved_assets(receipt)), 1)
            path.write_bytes(b"tampered")
            with self.assertRaises(ValueError): validate_saved_assets(receipt)

    def test_file_picker_screenshot_and_saved_attachment_bytes_are_required(self):
        with self.assertRaises(ValueError): validate_saved_assets({"android_host_file_picker_provider_attachment_verified":True})
        with tempfile.TemporaryDirectory() as directory:
            image = Path(directory) / "files.png"
            image.write_bytes(b"\x89PNG\r\n\x1a\nfixture")
            receipt = {"android_host_file_picker_provider_attachment_verified":True,
                "files_screenshot":{"path":str(image), "sha256":hashlib.sha256(image.read_bytes()).hexdigest()}}
            self.assertEqual(len(validate_saved_assets(receipt)), 1)
            image.write_bytes(b"tampered")
            with self.assertRaises(ValueError): validate_saved_assets(receipt)
            requests, _, _ = headless_tests.EvidenceTests().fixture()
            records, checkpoint = headless_tests.AckEvidenceTests().fixture()
            requests[0]["input"][-1]["content"][1]["file_data"] = "data:text/plain;base64,"
            command = Path(directory) / "commands.json"
            command.write_text(json.dumps({"requests":requests, "records":records, "checkpoint":checkpoint,
                "project_id":"project", "device_id":"device", "committed_request_id":"prompt-1"}))
            claimed = {"command_evidence":{"path":str(command), "sha256":hashlib.sha256(command.read_bytes()).hexdigest()},
                "process_evidence":{"committed_request_id":"prompt-1"}}
            with self.assertRaises(ValueError): validate_saved_headless(claimed)

    def test_resource_page_screenshot_and_saved_template_expansion_are_required(self):
        with self.assertRaises(ValueError): validate_saved_assets({"android_host_resource_pages_template_provider_verified":True})
        with tempfile.TemporaryDirectory() as directory:
            image = Path(directory) / "resources.png"
            image.write_bytes(b"\x89PNG\r\n\x1a\nfixture")
            receipt = {"android_host_resource_pages_template_provider_verified":True,
                "resources_screenshot":{"path":str(image), "sha256":hashlib.sha256(image.read_bytes()).hexdigest()}}
            self.assertEqual(len(validate_saved_assets(receipt)), 1)
            image.write_bytes(b"tampered")
            with self.assertRaises(ValueError): validate_saved_assets(receipt)
            requests, _, _ = headless_tests.EvidenceTests().fixture()
            records, checkpoint = headless_tests.AckEvidenceTests().fixture()
            requests[1]["input"][-1]["content"] = "STAGE2_HEADLESS_ANDROID_SECOND"
            command = Path(directory) / "commands.json"
            command.write_text(json.dumps({"requests":requests, "records":records, "checkpoint":checkpoint,
                "project_id":"project", "device_id":"device", "committed_request_id":"prompt-1"}))
            claimed = {"command_evidence":{"path":str(command), "sha256":hashlib.sha256(command.read_bytes()).hexdigest()},
                "process_evidence":{"committed_request_id":"prompt-1"}}
            with self.assertRaises(ValueError): validate_saved_headless(claimed)

    def test_model_selection_screenshot_and_actual_provider_configuration_are_required(self):
        with self.assertRaises(ValueError):validate_saved_assets({"android_model_reasoning_selection_provider_verified":True})
        with tempfile.TemporaryDirectory() as directory:
            path=Path(directory)/"selection.png";path.write_bytes(b"\x89PNG\r\n\x1a\nfixture")
            receipt={"android_model_reasoning_selection_provider_verified":True,"selection_screenshot":{"path":str(path),"sha256":hashlib.sha256(path.read_bytes()).hexdigest()}}
            self.assertEqual(len(validate_saved_assets(receipt)),1)
            path.write_bytes(b"tampered")
            with self.assertRaises(ValueError):validate_saved_assets(receipt)
            requests,_,_=headless_tests.EvidenceTests().fixture();records,checkpoint=headless_tests.AckEvidenceTests().fixture()
            requests[1]["reasoning"]={"effort":"high"}
            command=Path(directory)/"commands.json";command.write_text(json.dumps({"requests":requests,"records":records,"checkpoint":checkpoint,
                "project_id":"project","device_id":"device","committed_request_id":"prompt-1"}))
            claimed={"command_evidence":{"path":str(command),"sha256":hashlib.sha256(command.read_bytes()).hexdigest()},"process_evidence":{"committed_request_id":"prompt-1"}}
            with self.assertRaises(ValueError):validate_saved_headless(claimed)


if __name__ == "__main__": unittest.main()

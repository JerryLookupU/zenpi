import subprocess
import hashlib
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch
from stage2_scanner_android_smoke import COLD_RESTORE_PHASES, SCANNER_STATE_COUNT, PERMISSION_CLASSES, SCANNER_STATE_CLASSES, SUITES, ROOT, require_cold_restore, require_instrumentation, validate_receipt
from test_stage2_instrumentation import output_for


class ScannerEvidenceTests(unittest.TestCase):
    def result(self, output, code=0):
        return subprocess.CompletedProcess([], code, output, "")

    def output(self, permission=False):
        classes = PERMISSION_CLASSES if permission else SCANNER_STATE_CLASSES
        phases = ("scanner_permission_denial_nonblank_verified", "production_pairing_qr_factory_verified") if permission else COLD_RESTORE_PHASES
        marker = "" if permission else "INSTRUMENTATION_STATUS: zenpi_invite_monotonic_ui=expired_confirmation_never_revives_verified\n"
        if not permission: marker += "INSTRUMENTATION_STATUS: zenpi_identity_admission=wiped_mismatched_key_foreground_fenced_verified\n"
        if not permission: marker += "INSTRUMENTATION_STATUS: zenpi_sync_status=snapshot_tail_offline_status_verified\n"
        if not permission: marker += "INSTRUMENTATION_STATUS: zenpi_phase=device_confirmation_scope_verified\n"
        return output_for(classes) + "\n".join(f"INSTRUMENTATION_STATUS: zenpi_phase={phase}" for phase in phases) + "\n" + marker

    def test_exact_counts_and_capture_marker_pass(self):
        self.assertEqual(len(require_instrumentation(self.result(self.output(True)), 1, permission=True)), 1)
        self.assertEqual(len(require_instrumentation(self.result(self.output()), SCANNER_STATE_COUNT)), SCANNER_STATE_COUNT)

    def test_wrong_prefix_count_failures_and_exit_fail(self):
        for result in (self.result("OK (210 tests)"), self.result("OK (21 tests)\nFAILURES"),
                       self.result("OK (21 tests)", 1), self.result("INSTRUMENTATION_CODE: -1")):
            with self.assertRaises(ValueError): require_instrumentation(result, 21)

    def test_old_success_without_nonblank_evidence_is_refused(self):
        with self.assertRaises(ValueError): require_instrumentation(self.result("OK (1 test)"), 1, permission=True)

    def test_saved_receipt_requires_checksums_counts_and_new_marker(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "receipt.json"
            output = Path(directory) / "instrumentation.txt"
            png = Path(directory) / "screenshot.png"
            png.write_bytes(b"\x89PNG\r\n\x1a\nfixture")
            runs = []
            for index, (classes, count) in enumerate(SUITES):
                text = self.output(index == 0)
                runs.append({"classes":list(classes), "expected_tests":count, "exit_code":0, "output":text,
                    "stdout_sha256":hashlib.sha256(text.encode()).hexdigest(),
                    "completed_tests":require_instrumentation(self.result(text), count, permission=index == 0)})
            output.write_text("\n".join(run["output"] for run in runs))
            receipt = {"schema_version":"stage2-scanner-evidence/v2", "passed":True, "failure":None,
                "whole_item_accepted":False, "android_api":35, "expected_android_api":35,
                "actual_system_camera_permission_denial_verified":True, "scanner_cancel_invalid_pending_fences_verified":True,
                "cold_foreground_scanner_restore_verified":True, "diff_dialog_focus_cleared_verified":True,
                "external_pair_intent_single_owner_verified":True,
                "physical_camera_scan_verified":False, "lan_communication_verified":False,
                "instrumentation_runs":runs,
                "instrumentation_verifier":{"path":"tools/stage2_instrumentation.py", "sha256":hashlib.sha256((ROOT / "tools/stage2_instrumentation.py").read_bytes()).hexdigest()},
                "screenshot":{"path":str(png), "sha256":hashlib.sha256(png.read_bytes()).hexdigest()},
                "instrumentation":{"path":str(output), "sha256":hashlib.sha256(output.read_bytes()).hexdigest()}}
            path.write_text(json.dumps(receipt)); validate_receipt(path)
            import copy
            for change in (
                lambda value: value.pop("instrumentation_runs"),
                lambda value: value.update(schema_version="legacy"),
                lambda value: value.update(physical_camera_scan_verified=True),
                lambda value: value.update(lan_communication_verified=True),
                lambda value: value.update(expected_android_api=True),
                lambda value: value.update(failure="failed"),
                lambda value: value.pop("external_pair_intent_single_owner_verified"),
                lambda value: value["instrumentation_runs"][0].update(completed_tests=[]),
                lambda value: value["instrumentation_runs"][1].update(classes=list(reversed(SCANNER_STATE_CLASSES))),
                lambda value: value["instrumentation_runs"][1].update(exit_code=True),
                lambda value: value["instrumentation_runs"][1].update(stdout_sha256="b" * 64),
                lambda value: value["instrumentation_verifier"].update(sha256="b" * 64)):
                invalid = copy.deepcopy(receipt); change(invalid); path.write_text(json.dumps(invalid))
                with self.assertRaises(ValueError): validate_receipt(path)
            path.write_text(json.dumps(receipt))
            invalid = copy.deepcopy(receipt)
            invalid["instrumentation_runs"][1]["output"] = invalid["instrumentation_runs"][1]["output"].replace("class=zenpi.ScannerResultTest", "class=zenpi.ForeignTest")
            invalid["instrumentation_runs"][1]["stdout_sha256"] = hashlib.sha256(invalid["instrumentation_runs"][1]["output"].encode()).hexdigest()
            output.write_text("\n".join(run["output"] for run in invalid["instrumentation_runs"]))
            invalid["instrumentation"]["sha256"] = hashlib.sha256(output.read_bytes()).hexdigest()
            path.write_text(json.dumps(invalid))
            with self.assertRaises(ValueError): validate_receipt(path)
            path.write_text(json.dumps(receipt))
            output.write_text("OK (1 test)\nOK (13 tests)")
            with self.assertRaises(ValueError): validate_receipt(path)
            receipt["instrumentation"]["sha256"] = hashlib.sha256(output.read_bytes()).hexdigest()
            path.write_text(json.dumps(receipt))
            with self.assertRaises(ValueError): validate_receipt(path)

    def test_every_cold_restore_phase_is_required(self):
        output = "\n".join(f"INSTRUMENTATION_STATUS: zenpi_phase={phase}" for phase in COLD_RESTORE_PHASES)
        require_cold_restore(output)
        for phase in COLD_RESTORE_PHASES:
            with self.assertRaises(ValueError): require_cold_restore(output.replace(f"zenpi_phase={phase}", ""))
            with self.assertRaises(ValueError): require_cold_restore(output + f"\nINSTRUMENTATION_STATUS: zenpi_phase={phase}")

    def test_old_state_suite_count_cannot_prove_new_restore_coverage(self):
        with self.assertRaises(ValueError): require_instrumentation(self.result("OK (13 tests)"), SCANNER_STATE_COUNT)
        old = self.output().replace("INSTRUMENTATION_STATUS: zenpi_phase=scanner_live_read_pause_verified\n", "")
        with self.assertRaises(ValueError): require_cold_restore(old)

    def test_raw_payload_marker_is_required_even_with_all_prior_scanner_checks(self):
        current = self.output()
        marker = "INSTRUMENTATION_STATUS: zenpi_phase=scanner_raw_payload_fence_verified\n"
        self.assertIn(marker, current)
        with self.assertRaises(ValueError): require_cold_restore(current.replace(marker, ""))
        with self.assertRaises(ValueError): require_cold_restore(current + marker)

    def test_validated_queue_and_expiry_markers_are_required_and_unique(self):
        current = self.output()
        for phase in ("cold_invitation_validated_queue_verified", "cold_invitation_expired_queue_verified"):
            marker = f"INSTRUMENTATION_STATUS: zenpi_phase={phase}\n"
            self.assertIn(marker, current)
            with self.assertRaises(ValueError): require_cold_restore(current.replace(marker, ""))
            with self.assertRaises(ValueError): require_cold_restore(current + marker)

    def test_registered_host_without_invite_marker_is_required_and_unique(self):
        current = self.output()
        marker = "INSTRUMENTATION_STATUS: zenpi_phase=registered_host_without_invite_verified\n"
        self.assertIn(marker, current)
        with self.assertRaises(ValueError): require_cold_restore(current.replace(marker, ""))
        with self.assertRaises(ValueError): require_cold_restore(current + marker)

    def test_consumed_intent_recreation_marker_is_required_and_unique(self):
        current = self.output()
        marker = "INSTRUMENTATION_STATUS: zenpi_phase=external_pair_intent_consumed_recreation_verified\n"
        self.assertIn(marker, current)
        with self.assertRaises(ValueError): require_cold_restore(current.replace(marker, ""))
        with self.assertRaises(ValueError): require_cold_restore(current + marker)

    def test_summary_only_foreign_skipped_and_duplicate_tests_are_not_scanner_acceptance(self):
        output = self.output()
        for invalid in (f"OK ({SCANNER_STATE_COUNT} tests)\n", output.replace("class=zenpi.ScannerResultTest", "class=zenpi.ForeignTest"),
                        output.replace("INSTRUMENTATION_STATUS_CODE: 0", "INSTRUMENTATION_STATUS_CODE: -3", 1),
                        output + output, output.replace("current=2", "current=1")):
            with self.assertRaises(ValueError): require_instrumentation(self.result(invalid), SCANNER_STATE_COUNT)

    def test_permission_marker_must_be_exact_and_unique(self):
        output = self.output(True)
        for invalid in (output.replace("INSTRUMENTATION_STATUS: zenpi_phase=", "ignored zenpi_phase="),
                        output + "INSTRUMENTATION_STATUS: zenpi_phase=scanner_permission_denial_nonblank_verified\n"):
            with self.assertRaises(ValueError): require_instrumentation(self.result(invalid), 1, permission=True)

    def test_current_scanner_factory_marker_is_required_and_unique(self):
        output = self.output(True)
        marker = "INSTRUMENTATION_STATUS: zenpi_phase=production_pairing_qr_factory_verified\n"
        for invalid in (output.replace(marker, ""), output + marker,
                        output.replace("production_pairing_qr_factory_verified", "default_reader_only")):
            with self.assertRaises(ValueError): require_instrumentation(self.result(invalid), 1, permission=True)

    def test_device_confirmation_scope_requires_exact_unique_phase(self):
        output = self.output()
        marker = "INSTRUMENTATION_STATUS: zenpi_phase=device_confirmation_scope_verified\n"
        for invalid in (output.replace(marker, ""), output + marker,
                        output.replace(marker, "device_confirmation_scope_verified\n")):
            with self.assertRaises(ValueError): require_instrumentation(self.result(invalid), SCANNER_STATE_COUNT)

    def test_boolean_exit_status_cannot_masquerade_as_success(self):
        with self.assertRaises(ValueError): require_instrumentation(self.result(self.output(True), False), 1, permission=True)
        with self.assertRaises(ValueError): require_instrumentation(self.result(self.output(True)), True, permission=True)

    def test_production_orchestration_revalidates_saved_invocations_and_records_cleanup_failure(self):
        import stage2_scanner_android_smoke as scanner
        for fail_cleanup in (False, True):
            with self.subTest(fail_cleanup=fail_cleanup), tempfile.TemporaryDirectory() as directory:
                receipt = Path(directory) / "scanner.json"
                def run(argv, **kwargs):
                    if "getprop" in argv: return self.result("35\n")
                    if "force-stop" in argv and fail_cleanup:
                        raise subprocess.CalledProcessError(1, argv)
                    if "instrument" in argv:
                        return self.result(self.output(argv[argv.index("class") + 1] == ",".join(PERMISSION_CLASSES)))
                    if "exec-out" in argv:
                        return subprocess.CompletedProcess(argv, 0, b"\x89PNG\r\n\x1a\nfixture", b"")
                    return self.result("")
                with patch("sys.argv", ["scanner", "--serial", "emulator-fixture", "--expected-api", "35", "--receipt", str(receipt)]), \
                     patch.object(scanner, "require_current_build", return_value={"fixture":True}), \
                     patch.object(scanner.subprocess, "run", side_effect=run), patch("builtins.print"):
                    if fail_cleanup:
                        with self.assertRaises(SystemExit): scanner.main()
                    else:
                        scanner.main(); validate_receipt(receipt)
                saved = json.loads(receipt.read_text())
                self.assertEqual(saved["passed"], not fail_cleanup)
                self.assertEqual(len(saved["instrumentation_runs"]), 2)
                if fail_cleanup:
                    self.assertIn("cleanup", saved["failure"])
                    with self.assertRaises(ValueError): validate_receipt(receipt)


    def test_missing_duplicate_or_old_monotonic_ui_marker_cannot_prove_current_confirmation(self):
        marker = "INSTRUMENTATION_STATUS: zenpi_invite_monotonic_ui=expired_confirmation_never_revives_verified\n"
        output = self.output()
        for invalid in (output.replace(marker, ""), output + marker,
                        output.replace("expired_confirmation_never_revives_verified", "wall_clock_only_verified")):
            with self.assertRaises(ValueError): require_instrumentation(self.result(invalid), SCANNER_STATE_COUNT)

    def test_historical_scanner_count_receipts_do_not_prove_current_named_suite(self):
        for name in ("scanner-api35-20261002.json", "scanner-api35-resultfenced-20261002.json"):
            with self.subTest(receipt=name), self.assertRaises(ValueError):
                validate_receipt(ROOT / "Docs/quality/stage2_android2unix" / name)

    def test_missing_duplicate_or_foreign_identity_fence_marker_is_refused(self):
        marker = "INSTRUMENTATION_STATUS: zenpi_identity_admission=wiped_mismatched_key_foreground_fenced_verified\n"
        output = self.output()
        for invalid in (output.replace(marker, ""), output + marker,
                        output.replace("wiped_mismatched_key_foreground_fenced_verified", "transport_error_only")):
            with self.assertRaises(ValueError): require_instrumentation(self.result(invalid), SCANNER_STATE_COUNT)

    def test_missing_duplicate_or_old_sync_status_marker_is_refused(self):
        marker = "INSTRUMENTATION_STATUS: zenpi_sync_status=snapshot_tail_offline_status_verified\n"
        output = self.output()
        for invalid in (output.replace(marker, ""), output + marker,
                        output.replace("snapshot_tail_offline_status_verified", "connected_only")):
            with self.assertRaises(ValueError): require_instrumentation(self.result(invalid), SCANNER_STATE_COUNT)


if __name__ == "__main__": unittest.main()

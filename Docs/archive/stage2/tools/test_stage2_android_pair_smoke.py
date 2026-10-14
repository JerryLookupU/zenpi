import argparse
import unittest
import sys
import subprocess
from unittest.mock import patch

from stage2_android_pair_smoke import private_lan_ip, socket_endpoint, require_no_reverse, run_instrumentation, InstrumentationFailure
from stage2_android_pair_smoke import require_pair_tests, INSTRUMENTATION_CLASSES, EXPECTED_INSTRUMENTATION_TESTS
from test_stage2_instrumentation import output_for


class PairingSuiteTests(unittest.TestCase):
    def test_current_named_nine_test_suite_is_required(self):
        result = subprocess.CompletedProcess([], 0, output_for(INSTRUMENTATION_CLASSES))
        self.assertEqual(len(require_pair_tests(result)), EXPECTED_INSTRUMENTATION_TESTS)
        self.assertEqual(EXPECTED_INSTRUMENTATION_TESTS, 9)

    def test_old_count_summary_only_foreign_test_and_bad_status_are_refused(self):
        output = output_for(INSTRUMENTATION_CLASSES)
        for value in ("OK (9 tests)\n", output.replace("OK (9 tests)", "OK (8 tests)"),
                      output.replace("class=zenpi.HostPairingTest", "class=zenpi.ForeignTest"),
                      output.replace("INSTRUMENTATION_STATUS_CODE: 0", "INSTRUMENTATION_STATUS_CODE: -2", 1)):
            with self.subTest(output=value), self.assertRaises(ValueError):
                require_pair_tests(subprocess.CompletedProcess([], 0, value))
        for status in (False, 0.0, 1):
            with self.subTest(status=status), self.assertRaises(ValueError):
                require_pair_tests(subprocess.CompletedProcess([], status, output))


class InstrumentationObservationTests(unittest.TestCase):
    def child(self, source):
        return subprocess.Popen([sys.executable, "-u", "-c", source], stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)

    def test_timeout_preserves_partial_output_after_owned_process_is_reaped(self):
        process = self.child("import time; print('INSTRUMENTATION_STATUS: zenpi_phase=physical_qr_camera_requested', flush=True); time.sleep(30)")
        with patch("stage2_android_pair_smoke.subprocess.Popen", return_value=process):
            with self.assertRaises(InstrumentationFailure) as failure:
                run_instrumentation(["fixture"], "public command", lambda: None, lambda: None, timeout=.2)
        self.assertTrue(failure.exception.timed_out)
        self.assertIn("physical_qr_camera_requested", failure.exception.result.stdout)
        self.assertIsNotNone(process.poll())
        process.stdout.close(); process.stderr.close()

    def test_callback_failure_keeps_output_without_echoing_private_exception(self):
        process = self.child("print('INSTRUMENTATION_STATUS: zenpi_phase=desktop_update_requested'); print('completed fixture')")
        def refused(): raise ValueError("zenpi://pair?token=PRIVATE")
        with patch("stage2_android_pair_smoke.subprocess.Popen", return_value=process):
            with self.assertRaises(InstrumentationFailure) as failure:
                run_instrumentation(["fixture"], "public command", lambda: None, lambda: None, update=refused)
        self.assertFalse(failure.exception.timed_out)
        self.assertIn("completed fixture", failure.exception.result.stdout)
        self.assertNotIn("PRIVATE", str(failure.exception))
        self.assertEqual(0, process.poll())
        process.stdout.close(); process.stderr.close()

    def test_success_returns_observed_stdout_stderr_and_exit_code(self):
        process = self.child("import sys; print('observed'); print('diagnostic', file=sys.stderr)")
        with patch("stage2_android_pair_smoke.subprocess.Popen", return_value=process):
            result = run_instrumentation(["fixture"], "public command", lambda: None, lambda: None)
        self.assertEqual(0, result.returncode); self.assertEqual("observed\n", result.stdout); self.assertEqual("diagnostic\n", result.stderr)
        process.stdout.close(); process.stderr.close()

    def test_tick_failure_is_terminal_and_keeps_the_completed_observation(self):
        process = self.child("import time; print('partial fixture', flush=True); time.sleep(30)")
        def refused(): raise ValueError("PRIVATE tick diagnostic")
        with patch("stage2_android_pair_smoke.subprocess.Popen", return_value=process):
            with self.assertRaises(InstrumentationFailure) as failure:
                run_instrumentation(["fixture"], "public command", lambda: None, lambda: None, tick=refused)
        self.assertIsNotNone(process.poll()); self.assertFalse(failure.exception.timed_out)
        self.assertNotIn("PRIVATE", str(failure.exception))
        process.stdout.close(); process.stderr.close()


class DirectLanArgumentTests(unittest.TestCase):
    def test_only_private_literal_addresses_are_accepted(self):
        for value in ("10.0.2.2", "172.16.1.4", "192.168.50.14", "FD00::0012"):
            expected = "fd00::12" if value.startswith("FD") else value
            self.assertEqual(private_lan_ip(value), expected)
        for value in ("127.0.0.1", "0.0.0.0", "::1", "169.254.1.1", "224.0.0.1",
                      "192.0.2.14", "2001:db8::14", "fe80::12", "fd00::1%eth0", "example.invalid", "192.168.50.14:7777"):
            with self.subTest(value=value), self.assertRaises(argparse.ArgumentTypeError):
                private_lan_ip(value)

    def test_ipv4_and_ipv6_bind_endpoints_are_unambiguous(self):
        self.assertEqual(socket_endpoint("192.168.50.14", 0), "192.168.50.14:0")
        self.assertEqual(socket_endpoint("fd00::12", 0), "[fd00::12]:0")

    def test_direct_route_never_clears_existing_reverse_mapping(self):
        with patch("stage2_android_pair_smoke.subprocess.run", return_value=type("Result", (), {"stdout": "tcp:1234 tcp:1234\n"})()):
            with self.assertRaises(RuntimeError):
                require_no_reverse(["adb", "-s", "fixture"])

    def test_invalid_direct_address_is_rejected_before_adb_or_host_side_effects(self):
        parser = argparse.ArgumentParser()
        parser.add_argument("--lan-ip", type=private_lan_ip)
        for value in ("0.0.0.0", "127.0.0.1", "example.invalid"):
            with self.subTest(value=value), self.assertRaises(SystemExit):
                parser.parse_args(["--lan-ip", value])

    def test_receipt_route_fields_distinguish_direct_lan_from_reverse(self):
        self.assertEqual(socket_endpoint("192.168.50.14", 7777), "192.168.50.14:7777")


if __name__ == "__main__":
    unittest.main()

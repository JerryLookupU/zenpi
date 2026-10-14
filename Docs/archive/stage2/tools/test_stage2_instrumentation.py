import unittest
from stage2_instrumentation import expected_tests, require_test_results


def output_for(classes):
    tests = sorted(expected_tests(classes))
    lines = []
    for index, (name, method) in enumerate(tests, 1):
        for code in (1, 0):
            lines.extend((f"INSTRUMENTATION_STATUS: class={name}", f"INSTRUMENTATION_STATUS: test={method}",
                          f"INSTRUMENTATION_STATUS: current={index}", f"INSTRUMENTATION_STATUS: numtests={len(tests)}",
                          "INSTRUMENTATION_STATUS: stream=.", f"INSTRUMENTATION_STATUS_CODE: {code}"))
        lines.extend(("INSTRUMENTATION_STATUS: zenpi_phase=fixture", "INSTRUMENTATION_STATUS_CODE: 0"))
    return "\n".join(lines) + f"\nOK ({len(tests)} tests)\n"


class InstrumentationTests(unittest.TestCase):
    classes = ("zenpi.TerminalControlScreenTest", "zenpi.TerminalHistoryScreenTest")
    def test_full_named_start_success_chain_passes_with_phase_status(self):
        self.assertEqual(require_test_results(output_for(self.classes), self.classes, 2),
                         sorted(f"{name}#{method}" for name, method in expected_tests(self.classes)))

    def test_missing_duplicate_foreign_skipped_failed_or_unpaired_tests_fail(self):
        output = output_for(self.classes)
        for changed in (
            "OK (2 tests)\n", output.replace("INSTRUMENTATION_STATUS_CODE: 1", "INSTRUMENTATION_STATUS_CODE: 0", 1),
            output.replace("INSTRUMENTATION_STATUS_CODE: 0", "INSTRUMENTATION_STATUS_CODE: -3", 1),
            output.replace("INSTRUMENTATION_STATUS_CODE: 0", "INSTRUMENTATION_STATUS_CODE: -2", 1),
            output.replace("class=zenpi.TerminalControlScreenTest", "class=zenpi.ForeignTest"),
            output.replace("current=2", "current=1"), output.replace("numtests=2", "numtests=3"),
            output.replace("INSTRUMENTATION_STATUS: test=", "INSTRUMENTATION_STATUS: missing=", 1),
            output + "OK (2 tests)\n", output + "FAILURES",
            output.replace("INSTRUMENTATION_STATUS: class=", "INSTRUMENTATION_STATUS: class=duplicate\nINSTRUMENTATION_STATUS: class=", 1)):
            with self.subTest(changed=changed), self.assertRaises(ValueError): require_test_results(changed, self.classes, 2)

    def test_duplicate_completion_and_unfinished_tail_cannot_hide_behind_summary(self):
        output = output_for(self.classes)
        first, second = sorted(expected_tests(self.classes))
        duplicate = output.replace(second[0], first[0]).replace(second[1], first[1])
        unfinished = output.rsplit("INSTRUMENTATION_STATUS_CODE: 0", 2)[0] + "\nOK (2 tests)\n"
        for changed in (duplicate, unfinished):
            with self.assertRaises(ValueError): require_test_results(changed, self.classes, 2)

    def test_existing_runner_wire_format_parses_without_claiming_current_device_acceptance(self):
        from pathlib import Path
        root = Path(__file__).resolve().parents[1]
        historical = (root / "Docs/quality/stage2_android2unix/scanner-api35-20261002.instrumentation.txt").read_text()
        first_invocation = historical.split("INSTRUMENTATION_CODE: -1", 1)[0]
        identities = require_test_results(first_invocation, ("zenpi.ScannerActivityTest",), 1)
        self.assertEqual(identities, ["zenpi.ScannerActivityTest#actualCameraPermissionDenialReturnsToProductionPairingScreen"])

    def test_wrong_count_class_shape_and_oversize_output_refuse(self):
        with self.assertRaises(ValueError): require_test_results(output_for(self.classes), self.classes, 1)
        for classes in ((), ("../outside",), (self.classes[0], self.classes[0])):
            with self.assertRaises(ValueError): expected_tests(classes)
        with self.assertRaises(ValueError): require_test_results("x" * (4 * 1024 * 1024 + 1), self.classes, 2)


if __name__ == "__main__": unittest.main()

#!/usr/bin/env python3
import copy
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
import sys
from unittest.mock import patch

from stage2_android_build import APKS, ATTEMPT, BUILD_ARGV, PROVENANCE, build, failure_diagnosis, inputs, inventory_digest, require_current_build
from generate_stage2_android_schema import CONTRACT, OUTPUT, ROOT as SCHEMA_ROOT, render


class BuildProvenanceTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        for path, value in {
            "tools/stage2_android_build.py": "fixture builder",
            "tools/generate_stage2_android_schema.py": "fixture schema generator",
            "tools/validate_stage2_android_entry.py": "fixture entry validator",
            "android/app/src/main/java/App.kt": "production source",
            "android/app/src/androidTest/java/Device.kt": "instrumentation source",
            "android/app/src/main/AndroidManifest.xml": "manifest",
            "android/app/build.gradle.kts": "dependencies",
            "android/gradle/wrapper/gradle-wrapper.jar": "wrapper",
            "fuzz/remote_protocol/fixture.json": "{}",
            **{path.as_posix(): "apk fixture" for path in APKS},
        }.items():
            target = self.root / path
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_text(value)
        contract = (SCHEMA_ROOT / CONTRACT).read_bytes()
        (self.root / CONTRACT).write_bytes(contract)
        output = self.root / OUTPUT
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_bytes(render(contract))

    def receipt(self):
        inventory = inputs(self.root)
        return {"schema_version": "stage2-device-build/v1", "passed": True, "exit_code": 0,
                "argv": list(BUILD_ARGV), "inputs": inventory, "inputs_sha256": inventory_digest(inventory),
                "artifacts": {path.as_posix(): hashlib.sha256((self.root / path).read_bytes()).hexdigest() for path in APKS}}

    def write_receipt(self, receipt=None):
        (self.root / PROVENANCE).write_text(json.dumps(receipt or self.receipt()))

    def test_current_build_binds_source_and_both_apks(self):
        self.write_receipt()
        evidence = require_current_build(self.root)
        self.assertEqual(set(evidence["artifacts"]), {path.as_posix() for path in APKS})
        self.assertEqual(evidence["inputs_sha256"], inventory_digest(inputs(self.root)))

    def test_missing_failed_wrong_tasks_or_modified_inventory_refused(self):
        with self.assertRaises(ValueError): require_current_build(self.root)
        receipt = self.receipt()
        for field, value in (("passed", False), ("exit_code", 1), ("argv", ["./gradlew", "help"]),
                             ("schema_version", "future"), ("inputs_sha256", "0" * 64), ("inputs", {})):
            changed = copy.deepcopy(receipt); changed[field] = value
            self.write_receipt(changed)
            with self.subTest(field=field), self.assertRaises(ValueError): require_current_build(self.root)

    def test_non_integer_exit_status_cannot_authorize_device_apks(self):
        receipt = self.receipt()
        for status in (False, True, 0.0, "0", None):
            self.write_receipt(receipt | {"exit_code": status})
            with self.subTest(status=status), self.assertRaises(ValueError):
                require_current_build(self.root)

    def test_added_changed_deleted_sources_and_artifacts_refused(self):
        changes = (
            lambda: (self.root / "tools/generate_stage2_android_schema.py").write_text("changed generator"),
            lambda: (self.root / "tools/validate_stage2_android_entry.py").write_text("changed entry policy"),
            lambda: (self.root / "android/app/src/main/java/App.kt").write_text("new source"),
            lambda: (self.root / "android/app/src/main/java/Added.kt").write_text("added"),
            lambda: (self.root / "android/app/src/androidTest/java/Device.kt").unlink(),
            lambda: (self.root / APKS[0]).write_text("different apk"),
            lambda: (self.root / APKS[1]).unlink(),
        )
        for change in changes:
            self.write_receipt()
            change()
            with self.assertRaises(ValueError): require_current_build(self.root)
            # Recreate artifact so the next fixture receipt can be produced.
            if not (self.root / APKS[1]).exists(): (self.root / APKS[1]).write_text("apk fixture")

    def test_generated_local_sdk_and_signing_files_are_not_inventory(self):
        self.write_receipt()
        for name in ("android/local.properties", "android/operator.jks", "android/app/build/temporary.txt"):
            (self.root / name).write_text("not source")
        require_current_build(self.root)

    def test_source_and_artifact_symlinks_refused(self):
        source = self.root / "android/app/src/main/java/App.kt"
        target = self.root / "other.kt"; target.write_text("external")
        source.unlink(); source.symlink_to(target)
        with self.assertRaises(ValueError): inputs(self.root)
        source.unlink(); source.write_text("production source")
        self.write_receipt()
        (self.root / APKS[0]).unlink(); (self.root / APKS[0]).symlink_to(target)
        with self.assertRaises(ValueError): require_current_build(self.root)

    def test_source_packages_named_build_and_resource_logs_are_not_skipped(self):
        for relative in ("android/app/src/main/java/build/Compile.kt", "android/app/src/main/assets/sample.log"):
            target = self.root / relative; target.parent.mkdir(parents=True, exist_ok=True)
            target.write_text("source resource")
            self.write_receipt()
            target.write_text("changed resource")
            with self.assertRaises(ValueError): require_current_build(self.root)

    def test_invalid_receipt_type_is_explicit_refusal(self):
        (self.root / PROVENANCE).write_text("[]")
        with self.assertRaises(ValueError): require_current_build(self.root)

    def test_successful_build_uses_real_task_argv_and_publishes_after_completion(self):
        def run(argv, **kwargs):
            self.assertEqual(tuple(argv), BUILD_ARGV)
            self.assertFalse((self.root / PROVENANCE).exists())
            self.assertEqual(kwargs["cwd"], self.root / "android")
            return subprocess.CompletedProcess(argv, 0, b"build output", b"")
        self.write_receipt()
        with patch("stage2_android_build.subprocess.run", side_effect=run): build(self.root)
        require_current_build(self.root)

    def test_failure_timeout_and_mid_build_source_change_publish_no_authority(self):
        def change(argv, **kwargs):
            (self.root / "android/app/src/main/java/App.kt").write_text("during build")
            return subprocess.CompletedProcess(argv, 0, b"", b"")
        for result in (subprocess.CompletedProcess(BUILD_ARGV, 1, b"", b"failed"),
                       subprocess.TimeoutExpired(BUILD_ARGV, 1), change):
            self.write_receipt()
            options = {"side_effect": result} if isinstance(result, Exception) or callable(result) else {"return_value": result}
            with patch("stage2_android_build.subprocess.run", **options), self.assertRaises((RuntimeError, subprocess.TimeoutExpired)):
                build(self.root)
            self.assertFalse((self.root / PROVENANCE).exists())

    def test_failed_build_retains_private_hashed_logs_without_acceptance(self):
        self.write_receipt()
        result = subprocess.CompletedProcess(BUILD_ARGV, 1, b"compile diagnostic\n", b"Operation not permitted\n")
        with patch("stage2_android_build.subprocess.run", return_value=result), self.assertRaisesRegex(RuntimeError, "diagnostics:"):
            build(self.root)
        attempt = json.loads((self.root / ATTEMPT).read_text())
        self.assertFalse(attempt["passed"])
        self.assertEqual(attempt["exit_code"], 1)
        self.assertFalse(attempt["timed_out"])
        self.assertEqual(attempt["argv"], list(BUILD_ARGV))
        self.assertEqual(attempt["inputs_sha256"], inventory_digest(inputs(self.root)))
        for stream, expected in (("stdout", result.stdout), ("stderr", result.stderr)):
            log = attempt["logs"][stream]
            path = self.root / log["path"]
            self.assertEqual(path.read_bytes(), expected)
            self.assertEqual(log["sha256"], hashlib.sha256(expected).hexdigest())
            self.assertEqual(log["bytes"], len(expected))
            self.assertEqual(path.stat().st_mode & 0o777, 0o600)
        self.assertEqual((self.root / ATTEMPT).stat().st_mode & 0o777, 0o600)
        with self.assertRaises(ValueError): require_current_build(self.root)

    def test_daemon_listener_refusal_is_diagnosed_without_retry_or_provenance(self):
        self.write_receipt()
        result = subprocess.CompletedProcess(BUILD_ARGV, 1, b"Unable to start the daemon process\n",
            b"java.net.SocketException: Operation not permitted\nDaemonTcpServerConnector.start\n")
        with patch("stage2_android_build.subprocess.run", return_value=result) as run, \
                self.assertRaisesRegex(RuntimeError, "local TCP listener permission denied"):
            build(self.root)
        run.assert_called_once()
        self.assertFalse((self.root / PROVENANCE).exists())
        with self.assertRaises(ValueError): require_current_build(self.root)

    def test_wrapper_lock_refusal_is_distinct_from_daemon_listener(self):
        message = b"gradle-wrapper/dists/gradle-8-bin/fixture/gradle-8-bin.zip.lck (Permission denied)"
        self.assertEqual(failure_diagnosis(b"", message), "Gradle wrapper cache lock permission denied")
        self.assertEqual(failure_diagnosis(message, b""), "Gradle wrapper cache lock permission denied")

    def test_unrelated_failures_are_not_misclassified_as_listener_refusals(self):
        for stdout, stderr, expected in (
            (b"", b"Operation not permitted", "Operation not permitted"),
            (b"Unable to start the daemon process", b"invalid JVM option", ""),
            (b"DaemonTcpServerConnector", b"compilation error", ""),
            (b"SDK location not found", b"", "SDK location not found"),
            (b"", b"JAVA_HOME is not set", "JAVA_HOME"),
            (b"", b"Kotlin compilation failed", ""),
        ):
            with self.subTest(stderr=stderr):
                self.assertEqual(failure_diagnosis(stdout, stderr), expected)

    def test_timeout_retains_partial_output_and_distinguishes_unknown_exit(self):
        timeout = subprocess.TimeoutExpired(BUILD_ARGV, 1, output=b"partial output", stderr=b"partial error")
        with patch("stage2_android_build.subprocess.run", side_effect=timeout), self.assertRaises(subprocess.TimeoutExpired) as failure:
            build(self.root)
        attempt = json.loads((self.root / ATTEMPT).read_text())
        self.assertTrue(attempt["timed_out"])
        self.assertIsNone(attempt["exit_code"])
        self.assertIn("diagnostics:", failure.exception.__notes__[0])
        self.assertEqual((self.root / attempt["logs"]["stdout"]["path"]).read_bytes(), b"partial output")
        self.assertEqual((self.root / attempt["logs"]["stderr"]["path"]).read_bytes(), b"partial error")
        with self.assertRaises(ValueError): require_current_build(self.root)

    def test_success_attempt_is_not_provenance_and_does_not_overwrite_prior_logs(self):
        failed = subprocess.CompletedProcess(BUILD_ARGV, 1, b"first failure", b"")
        with patch("stage2_android_build.subprocess.run", return_value=failed), self.assertRaises(RuntimeError): build(self.root)
        first = json.loads((self.root / ATTEMPT).read_text())
        success = subprocess.CompletedProcess(BUILD_ARGV, 0, b"successful output", b"")
        with patch("stage2_android_build.subprocess.run", return_value=success): build(self.root)
        latest = json.loads((self.root / ATTEMPT).read_text())
        self.assertFalse(latest["passed"])
        self.assertEqual(latest["exit_code"], 0)
        self.assertNotEqual(first["logs"]["stdout"]["path"], latest["logs"]["stdout"]["path"])
        self.assertEqual((self.root / first["logs"]["stdout"]["path"]).read_bytes(), b"first failure")
        require_current_build(self.root)

    def test_every_device_harness_refuses_missing_provenance_before_adb(self):
        import stage2_android_pair_smoke as pair
        import stage2_desktop_android_smoke as desktop
        import stage2_headless_android_smoke as headless
        import stage2_scanner_android_smoke as scanner
        for module, arguments in ((pair, ["--serial", "fixture"]),
            (desktop, ["--serial", "fixture", "--receipt", "unused.json"]),
            (headless, ["--serial", "fixture", "--receipt", "unused.json"]),
            (scanner, ["--serial", "emulator-fixture", "--expected-api", "35", "--receipt", "unused.json"])):
            with self.subTest(module=module.__name__), patch.object(sys, "argv", [module.__name__, *arguments]), \
                patch.object(module, "require_current_build", side_effect=ValueError("stale build")), \
                patch.object(module.subprocess, "run") as run, patch.object(module.subprocess, "Popen") as popen:
                with self.assertRaisesRegex(ValueError, "stale build"): module.main()
                run.assert_not_called(); popen.assert_not_called()

    def test_stale_generated_source_refused_before_gradle_and_old_authority_removed(self):
        self.write_receipt()
        (self.root / OUTPUT).write_text("stale generated enums")
        with patch("stage2_android_build.subprocess.run") as run, self.assertRaisesRegex(ValueError, "schema is stale"):
            build(self.root)
        run.assert_not_called()
        self.assertFalse((self.root / PROVENANCE).exists())
        self.assertFalse((self.root / ATTEMPT).exists())
        self.assertEqual("stale generated enums", (self.root / OUTPUT).read_text())

    def test_matching_inventory_cannot_promote_stale_schema_to_current_apk(self):
        (self.root / OUTPUT).write_text("stale generated enums")
        self.write_receipt()
        with self.assertRaisesRegex(ValueError, "schema is stale"):
            require_current_build(self.root)
        self.assertEqual("stale generated enums", (self.root / OUTPUT).read_text())

    def test_invalid_contract_refused_before_gradle_without_regeneration(self):
        self.write_receipt()
        generated = (self.root / OUTPUT).read_bytes()
        (self.root / CONTRACT).write_text('{"schema_version":true}')
        with patch("stage2_android_build.subprocess.run") as run, self.assertRaises(ValueError):
            build(self.root)
        run.assert_not_called()
        self.assertFalse((self.root / PROVENANCE).exists())
        self.assertEqual(generated, (self.root / OUTPUT).read_bytes())


if __name__ == "__main__":
    unittest.main()

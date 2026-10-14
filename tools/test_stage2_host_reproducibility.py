#!/usr/bin/env python3
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch
import stage2_host_reproducibility as checker


class ReproducibilityTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.receipt = self.root / "result.json"
        for name in ("Cargo.toml", "Cargo.lock", "src/main.rs", "tools/stage2_host_build.py",
                     "tools/stage2_android_build.py", "tools/stage2_host_reproducibility.py"):
            path = self.root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text("fixture")
        self.targets = []

    def run_build(self, argv, **kwargs):
        self.assertNotIn("RUSTFLAGS", kwargs["env"])
        if argv[-1] == "-vV":
            return subprocess.CompletedProcess(argv, 0, (argv[0] + " fixture").encode(), b"")
        target = Path(argv[-1])
        self.assertFalse((target / "release").exists())
        self.targets.append(target)
        artifact = target / "release/zenpi"
        artifact.parent.mkdir()
        artifact.write_bytes(b"equal release")
        message = {"reason": "compiler-artifact", "target": {"name": "zenpi", "kind": ["bin"]}, "executable": str(artifact)}
        return subprocess.CompletedProcess(argv, 0, json.dumps(message).encode(), b"")

    def compare(self, run=None):
        with patch.object(checker.subprocess, "run", side_effect=run or self.run_build):
            return checker.compare(self.root, self.receipt, self.root / "runtime")

    def test_equal_fresh_builds_bind_source_toolchain_and_private_receipt(self):
        result = self.compare()
        self.assertTrue(result["passed"])
        self.assertEqual(2, len(set(self.targets)))
        self.assertTrue(self.targets[0].is_relative_to(self.root / "runtime"))
        self.assertTrue(self.targets[1].is_relative_to(self.root / "target"))
        self.assertEqual([str(self.root / "runtime"), str(self.root / "target")], result["runtime_parents"])
        self.assertEqual([str(target) for target in self.targets], [lane["target_directory"] for lane in result["lanes"]])
        self.assertFalse(result["runtime_verified"])
        self.assertFalse(result["whole_item_accepted"])
        self.assertEqual(0o600, self.receipt.stat().st_mode & 0o777)
        self.assertIn("tools/stage2_host_reproducibility.py", result["inputs"])

    def verify(self, binary=None):
        with patch.object(checker.subprocess, "run", side_effect=self.run_build):
            return checker.verify_receipt(self.root, self.receipt, binary)

    def test_saved_receipt_rechecks_sources_toolchain_logs_artifacts_and_claim_scope(self):
        result = self.compare()
        self.assertEqual(result, self.verify())
        original = self.receipt.read_text()
        for field, value in (("passed", False), ("runtime_verified", True), ("cross_host_verified", True),
                             ("whole_item_accepted", True), ("cwd", "/foreign"), ("inputs", {}), ("toolchain", {})):
            changed = json.loads(original); changed[field] = value
            self.receipt.write_text(json.dumps(changed))
            with self.assertRaises(ValueError): self.verify()
        self.receipt.write_text(original)
        for path in (self.targets[0] / "release/zenpi", Path(result["lanes"][1]["stderr"]["path"])):
            data = path.read_bytes(); path.write_bytes(b"tampered")
            with self.assertRaises(ValueError): self.verify()
            path.write_bytes(data)
        (self.root / "src/main.rs").write_text("new source")
        with self.assertRaises(ValueError): self.verify()

    def test_saved_receipt_requires_exact_independent_lane_paths_and_commands(self):
        result = self.compare()
        for change in (lambda value: value["lanes"][1].update(value["lanes"][0]),
                       lambda value: value["lanes"][1].update(exit_code=True),
                       lambda value: value["lanes"][1]["argv"].append("--other"),
                       lambda value: value["lanes"][1].update(target_directory="relative")):
            value = json.loads(json.dumps(result)); change(value)
            self.receipt.write_text(json.dumps(value))
            with self.assertRaises(ValueError): self.verify()

    def test_saved_receipt_optionally_binds_current_published_release(self):
        self.compare()
        provenance = {"artifact":{"path":"target/release/zenpi", "sha256":checker.sha256(self.targets[0] / "release/zenpi")}}
        with patch("stage2_host_build.require_current_host", return_value=provenance):
            self.verify(self.root / "target/release/zenpi")
        for path, digest in (("target/debug/zenpi", provenance["artifact"]["sha256"]), ("target/release/zenpi", "0" * 64)):
            with patch("stage2_host_build.require_current_host", return_value={"artifact":{"path":path, "sha256":digest}}):
                with self.assertRaises(ValueError): self.verify(self.root / "target/release/zenpi")

    def test_saved_receipt_and_artifact_symlinks_are_refused(self):
        self.compare()
        retained = self.root / "retained.json"
        self.receipt.rename(retained)
        self.receipt.symlink_to(retained)
        with self.assertRaises(ValueError): self.verify()
        self.receipt.unlink(); retained.rename(self.receipt)
        artifact = self.targets[0] / "release/zenpi"
        retained = self.root / "binary"
        artifact.rename(retained); artifact.symlink_to(retained)
        with self.assertRaises(ValueError): self.verify()

    def test_publication_tree_difference_fails_default_cross_location_gate(self):
        def run(argv, **kwargs):
            completed = self.run_build(argv, **kwargs)
            if "--target-dir" in argv and self.targets[-1].is_relative_to(self.root / "target"):
                (self.targets[-1] / "release/zenpi").write_bytes(b"publication UUID differs")
            return completed
        result = self.compare(run)
        self.assertEqual("release_artifacts_differ", result["failure"])
        self.assertFalse(result["passed"])
        self.assertEqual(2, len(result["lanes"]))

    def test_explicit_secondary_parent_is_bound_and_symlink_is_refused(self):
        second = self.root / "alternate"
        with patch.object(checker.subprocess, "run", side_effect=self.run_build):
            result = checker.compare(self.root, self.receipt, self.root / "runtime", secondary_runtime_parent=second)
        self.assertTrue(result["passed"])
        self.assertTrue(self.targets[1].is_relative_to(second))
        self.assertEqual(str(second), result["runtime_parents"][1])
        link = self.root / "linked"
        link.symlink_to(second, target_is_directory=True)
        with patch.object(checker.subprocess, "run") as run, self.assertRaises(ValueError):
            checker.compare(self.root, self.root / "symlink.json", self.root / "runtime", secondary_runtime_parent=link)
        run.assert_not_called()

    def test_different_artifacts_fail_without_promoting_runtime(self):
        def run(argv, **kwargs):
            completed = self.run_build(argv, **kwargs)
            if "--target-dir" in argv and len(self.targets) == 2:
                (self.targets[-1] / "release/zenpi").write_bytes(b"different")
            return completed
        result = self.compare(run)
        self.assertEqual("release_artifacts_differ", result["failure"])
        self.assertFalse(result["passed"])

    def test_failed_or_timed_out_build_never_retries_or_runs_second_lane(self):
        for timeout in (False, True):
            receipt = self.root / f"failed-{timeout}.json"
            def run(argv, **kwargs):
                if "--target-dir" not in argv:
                    return self.run_build(argv, **kwargs)
                self.targets.append(Path(argv[-1]))
                if timeout:
                    raise subprocess.TimeoutExpired(argv, 1, output=b"partial", stderr=b"timeout")
                return subprocess.CompletedProcess(argv, 1, b"", b"failed")
            self.targets = []
            with patch.object(checker.subprocess, "run", side_effect=run):
                result = checker.compare(self.root, receipt, self.root / "runtime")
            self.assertFalse(result["passed"])
            self.assertEqual(1, len(self.targets))

    def test_source_drift_or_wrong_artifact_stops_before_second_lane(self):
        for mode in ("drift", "artifact"):
            self.receipt = self.root / f"{mode}.json"
            self.targets = []
            def run(argv, **kwargs):
                completed = self.run_build(argv, **kwargs)
                if "--target-dir" in argv:
                    if mode == "drift":
                        (self.root / "src/main.rs").write_text("changed")
                    else:
                        message = json.loads(completed.stdout)
                        message["executable"] = str(self.root / "unexpected")
                        completed.stdout = json.dumps(message).encode()
                return completed
            result = self.compare(run)
            self.assertFalse(result["passed"])
            self.assertEqual(1, len(self.targets))

    def test_existing_receipt_refused_before_build(self):
        self.receipt.write_text("existing")
        with patch.object(checker.subprocess, "run") as run, self.assertRaises(ValueError):
            checker.compare(self.root, self.receipt, self.root / "runtime")
        run.assert_not_called()
        self.assertEqual("existing", self.receipt.read_text())

    def test_toolchain_drift_fails_after_first_build(self):
        def run(argv, **kwargs):
            completed = self.run_build(argv, **kwargs)
            if argv[-1] == "-vV" and self.targets:
                completed.stdout += b" changed"
            return completed
        result = self.compare(run)
        self.assertEqual("source_or_toolchain_changed", result["failure"])
        self.assertEqual(1, len(self.targets))

    def test_symlink_binary_or_malformed_cargo_report_cannot_pass(self):
        for mode in ("symlink", "malformed"):
            self.receipt = self.root / f"{mode}.json"
            self.targets = []
            def run(argv, **kwargs):
                completed = self.run_build(argv, **kwargs)
                if "--target-dir" in argv:
                    if mode == "symlink":
                        artifact = self.targets[-1] / "release/zenpi"
                        artifact.unlink()
                        artifact.symlink_to(self.root / "Cargo.toml")
                    else:
                        completed.stdout = b"not json"
                return completed
            result = self.compare(run)
            self.assertEqual("release_artifact_unverified", result["failure"])
            self.assertEqual(1, len(self.targets))


if __name__ == "__main__":
    unittest.main()

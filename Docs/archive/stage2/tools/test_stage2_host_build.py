#!/usr/bin/env python3
import copy
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
import sys
from unittest.mock import patch
import stage2_host_build as builder


class HostBuildTests(unittest.TestCase):
    def fresh_run(self, argv, **kwargs):
        target = Path(argv[-1]); self.assertFalse((target / "release").exists())
        binary = target / "release/zenpi"; binary.parent.mkdir(); binary.write_bytes(b"fresh release")
        return subprocess.CompletedProcess(argv, 0, json.dumps({"reason":"compiler-artifact",
            "target":{"name":"zenpi","kind":["bin"]}, "executable":str(binary)}).encode(), b"")

    def test_fresh_build_publishes_exact_artifact_with_verifiable_receipt(self):
        with patch.object(builder.subprocess, "run", side_effect=self.fresh_run):
            evidence = builder.build(self.root, "release", fresh=True)
        receipt, binary = builder.paths(self.root, "release")
        self.assertEqual(b"fresh release", binary.read_bytes())
        self.assertEqual(0o755, binary.stat().st_mode & 0o777)
        recorded = json.loads(receipt.read_text())
        self.assertEqual("stage2-host-build/v2", recorded["schema_version"])
        self.assertEqual(evidence, builder.require_current_host(binary, self.root))
        (Path(recorded["fresh_target"]) / "release/zenpi").write_bytes(b"tampered")
        with self.assertRaises(ValueError): builder.require_current_host(binary, self.root)

    def test_fresh_failure_or_source_drift_preserves_previous_binary_without_authority(self):
        receipt, binary = builder.paths(self.root, "release")
        binary.parent.mkdir(); binary.write_bytes(b"previous")
        def drift(argv, **kwargs):
            result = self.fresh_run(argv, **kwargs)
            (self.root / "src/main.rs").write_text("drift")
            return result
        for runner in (lambda *args, **kwargs: subprocess.CompletedProcess([], 1, b"", b"failed"), drift):
            with patch.object(builder.subprocess, "run", side_effect=runner), self.assertRaises((RuntimeError, ValueError)):
                builder.build(self.root, "release", fresh=True)
            self.assertEqual(b"previous", binary.read_bytes()); self.assertFalse(receipt.exists())

    def test_fresh_publication_refuses_symlink_and_changed_canonical_artifact(self):
        receipt, binary = builder.paths(self.root, "release")
        binary.parent.mkdir(); binary.symlink_to(self.root / "Cargo.toml")
        with patch.object(builder.subprocess, "run", side_effect=self.fresh_run), self.assertRaises(ValueError):
            builder.build(self.root, "release", fresh=True)
        self.assertTrue(binary.is_symlink()); self.assertFalse(receipt.exists())
        binary.unlink()
        with patch.object(builder.subprocess, "run", side_effect=self.fresh_run): builder.build(self.root, "release", fresh=True)
        recorded = json.loads(receipt.read_text())
        for field, value in (("fresh_target", str(self.root / "external")), ("argv", builder.argv("release"))):
            receipt.write_text(json.dumps(recorded | {field: value}))
            with self.assertRaises(ValueError): builder.require_current_host(binary, self.root)
        receipt.write_text(json.dumps(recorded)); binary.write_bytes(b"changed")
        with self.assertRaises(ValueError): builder.require_current_host(binary, self.root)

    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(); self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        for name in ("Cargo.toml", "Cargo.lock", "src/main.rs", "vendor/local/lib.rs",
                     ".cargo/config.toml", "tools/stage2_host_build.py", "tools/stage2_android_build.py", "target/debug/zenpi"):
            path = self.root / name; path.parent.mkdir(parents=True, exist_ok=True); path.write_text("fixture")
        self.receipt, self.binary = builder.paths(self.root, "debug")

    def publish(self):
        inventory = builder.inputs(self.root)
        receipt = {"schema_version":"stage2-host-build/v1", "passed":True, "exit_code":0, "argv":builder.argv("debug"),
            "inputs":inventory, "inputs_sha256":builder.inventory_digest(inventory),
            "artifact":{"path":"target/debug/zenpi", "sha256":builder.sha256(self.binary)}}
        self.receipt.write_text(json.dumps(receipt)); return receipt

    def test_current_inventory_includes_manifest_lock_vendor_and_cargo_config(self):
        self.publish(); evidence = builder.require_current_host(self.binary, self.root)
        self.assertEqual(evidence["artifact"]["path"], "target/debug/zenpi")
        for name in ("Cargo.toml", "Cargo.lock", "vendor/local/lib.rs", ".cargo/config.toml"):
            self.assertIn(name, builder.inputs(self.root))

    def test_build_script_addition_change_and_removal_invalidate_host_authority(self):
        self.publish()
        script = self.root / "build.rs"
        script.write_text("initial build policy")
        with self.assertRaises(ValueError): builder.require_current_host(self.binary, self.root)
        self.publish()
        self.assertIn("build.rs", builder.inputs(self.root))
        script.write_text("changed linker policy")
        with self.assertRaises(ValueError): builder.require_current_host(self.binary, self.root)
        self.publish()
        script.unlink()
        with self.assertRaises(ValueError): builder.require_current_host(self.binary, self.root)

    def test_source_addition_deletion_and_binary_changes_are_refused(self):
        for name in ("src/main.rs", "Cargo.lock", "vendor/local/lib.rs", ".cargo/config.toml", "target/debug/zenpi"):
            self.publish(); (self.root / name).write_text("changed")
            with self.assertRaises(ValueError): builder.require_current_host(self.binary, self.root)
        self.publish(); (self.root / "src/new.rs").write_text("added")
        with self.assertRaises(ValueError): builder.require_current_host(self.binary, self.root)
        self.publish(); (self.root / "src/new.rs").unlink()
        with self.assertRaises(ValueError): builder.require_current_host(self.binary, self.root)

    def test_missing_failed_wrong_command_or_corrupt_receipts_are_refused(self):
        with self.assertRaises(ValueError): builder.require_current_host(self.binary, self.root)
        receipt = self.publish()
        for key, value in (("passed",False),("exit_code",1),("argv",["cargo","check"]),("inputs",{}),("schema_version","future")):
            changed = copy.deepcopy(receipt); changed[key] = value; self.receipt.write_text(json.dumps(changed))
            with self.assertRaises(ValueError): builder.require_current_host(self.binary, self.root)
        self.receipt.write_text("[]")
        with self.assertRaises(ValueError): builder.require_current_host(self.binary, self.root)

    def test_non_integer_exit_status_cannot_authorize_host_binary(self):
        receipt = self.publish()
        for status in (False, True, 0.0, "0", None):
            self.receipt.write_text(json.dumps(receipt | {"exit_code": status}))
            with self.subTest(status=status), self.assertRaises(ValueError):
                builder.require_current_host(self.binary, self.root)

    def test_symlinks_and_unattributed_binary_path_are_refused(self):
        self.publish(); alias = self.root / "alias"; alias.symlink_to(self.binary)
        with self.assertRaises(ValueError): builder.require_current_host(alias, self.root)
        path = self.root / "src/link.rs"; path.symlink_to(self.root / "Cargo.toml")
        with self.assertRaises(ValueError): builder.inputs(self.root)
        path.unlink(); directory = self.root / "src/link"; directory.symlink_to(self.root / "vendor", target_is_directory=True)
        with self.assertRaises(ValueError): builder.inputs(self.root)

    def test_failed_build_invalidates_prior_receipt_and_success_binds_reported_executable(self):
        self.publish()
        with patch.object(builder.subprocess, "run", return_value=subprocess.CompletedProcess([],1,b"",b"failure")):
            with self.assertRaises(RuntimeError): builder.build(self.root)
        self.assertFalse(self.receipt.exists())
        output = json.dumps({"reason":"compiler-artifact","target":{"name":"zenpi","kind":["bin"]},"executable":str(self.binary)}).encode()+b"\n"
        with patch.object(builder.subprocess, "run", return_value=subprocess.CompletedProcess([],0,output,b"")) as run:
            evidence = builder.build(self.root)
            self.assertEqual(run.call_args.kwargs["cwd"], self.root)
            self.assertNotIn("RUSTFLAGS",run.call_args.kwargs["env"])
        self.assertEqual(evidence,builder.require_current_host(self.binary,self.root))

    def test_missing_wrong_or_duplicate_cargo_artifact_does_not_publish(self):
        message = {"reason":"compiler-artifact","target":{"name":"zenpi","kind":["bin"]},"executable":str(self.binary)}
        for messages in ([],[message,message],[message | {"executable":str(self.root/"other")} ]):
            output = b"\n".join(json.dumps(value).encode() for value in messages)
            with patch.object(builder.subprocess,"run",return_value=subprocess.CompletedProcess([],0,output,b"")):
                with self.assertRaises(ValueError): builder.build(self.root)
            self.assertFalse(self.receipt.exists())

    def test_concurrent_source_edit_never_publishes(self):
        def run(*args,**kwargs):
            (self.root/"src/main.rs").write_text("concurrent edit")
            output = json.dumps({"reason":"compiler-artifact","target":{"name":"zenpi","kind":["bin"]},"executable":str(self.binary)}).encode()
            return subprocess.CompletedProcess([],0,output,b"")
        with patch.object(builder.subprocess,"run",side_effect=run):
            with self.assertRaises(ValueError): builder.build(self.root)
        self.assertFalse(self.receipt.exists())

    def test_host_device_harnesses_refuse_stale_host_before_adb_or_child_process(self):
        import stage2_android_pair_smoke
        import stage2_desktop_android_smoke
        import stage2_headless_android_smoke
        import stage2_session_android_smoke
        for harness in (stage2_android_pair_smoke,stage2_desktop_android_smoke,stage2_headless_android_smoke,stage2_session_android_smoke):
            arguments = ["fixture","--serial","fixture","--receipt","unused.json"]
            if harness is stage2_session_android_smoke: arguments += ["--expected-api","35"]
            with self.subTest(harness=harness.__name__), patch.object(sys,"argv",arguments), \
                patch.object(harness,"require_current_build",return_value={}), \
                patch.object(harness,"require_current_host",side_effect=ValueError("stale host")), \
                patch.object(harness.subprocess,"run") as run, patch.object(harness.subprocess,"Popen") as popen:
                with self.assertRaisesRegex(ValueError,"stale host"): harness.main()
                run.assert_not_called(); popen.assert_not_called()


if __name__ == "__main__": unittest.main()

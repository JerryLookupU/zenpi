import hashlib
import json
import os
import stat
import tempfile
import time
import unittest
from unittest import mock
from pathlib import Path

from stage2_host_smoke import run, validate_invite
import stage2_host_build as builder


TOKEN = "AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8"


class HostInviteTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)
        for name in ("Cargo.toml", "Cargo.lock", "src/main.rs", "tools/stage2_host_build.py", "tools/stage2_android_build.py"):
            path = self.root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text("fixture")
        self.root_patch = mock.patch("stage2_host_smoke.ROOT", self.root)
        self.root_patch.start()

    def tearDown(self):
        self.root_patch.stop()
        self.tmp.cleanup()

    def _script(self, body: str) -> Path:
        path = self.root / "target/release/zenpi"
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text("#!/usr/bin/env python3\n" + body)
        path.chmod(path.stat().st_mode | stat.S_IXUSR)
        inventory = builder.inputs(self.root)
        receipt, _ = builder.paths(self.root, "release")
        receipt.write_text(json.dumps({"schema_version": "stage2-host-build/v1", "passed": True,
            "exit_code": 0, "argv": builder.argv("release"), "inputs": inventory,
            "inputs_sha256": builder.inventory_digest(inventory),
            "artifact": {"path": "target/release/zenpi", "sha256": builder.sha256(path)}}))
        return path

    def _invite(self, expiry="1700000100"):
        return ("zenpi://pair?v=1&address=192.168.50.14%3A7777&fingerprint="
                + "a" * 64 + "&token=" + TOKEN + "&expires=" + expiry)

    def test_exact_invite_fields_are_validated(self):
        self.assertEqual(validate_invite(self._invite(), now=1700000000)["v"], "1")

    def test_endpoint_rules_match_mapped_scoped_and_broadcast_host_refusals(self):
        for address in ("0.0.0.0:7777", "224.0.0.1:7777", "169.254.1.2:7777", "255.255.255.255:7777",
                        "[::]:7777", "[ff02::1]:7777", "[fe80::1]:7777", "[fe80::1%25en0]:7777",
                        "[fd00::1%25en0]:7777", "[::ffff:0.0.0.0]:7777", "[::ffff:224.0.0.1]:7777",
                        "[::ffff:169.254.1.2]:7777", "[::ffff:255.255.255.255]:7777"):
            with self.subTest(address=address), self.assertRaises(ValueError):
                validate_invite(self._invite().replace("192.168.50.14%3A7777", address), now=1700000000)
        for address in ("127.0.0.1:1", "[::1]:65535", "[fd00::14]:7777", "[::ffff:192.168.50.14]:7777"):
            self.assertEqual(validate_invite(self._invite().replace("192.168.50.14%3A7777", address), now=1700000000)["address"], address)

    def test_uri_controls_and_oversize_are_refused_before_urlsplit_normalization(self):
        for value in ("\n" + self._invite(), self._invite() + "\t", self._invite().replace("zenpi", "ze\nnpi"),
                      self._invite() + "\x7f", self._invite().replace(TOKEN, "A" * 2048)):
            with self.assertRaises(ValueError): validate_invite(value, now=1700000000)

    def test_token_and_expiry_are_strict(self):
        valid = self._invite()
        for suffix in ("=", "+"):
            with self.assertRaises(ValueError):
                validate_invite(valid.replace(TOKEN, TOKEN[:-1] + suffix), now=1700000000)
        with self.assertRaises(ValueError):
            validate_invite(valid.replace("1700000100", "１７０００００１００"), now=1700000000)
        with self.assertRaises(ValueError):
            validate_invite(valid, now=1700000101)
        with self.assertRaises(ValueError):
            validate_invite(valid.replace("1700000100", "1700000401"), now=1700000000)

    def test_invalid_shape_is_rejected(self):
        values = ("", "zenpi://other?v=1", "zenpi://pair?v=1&extra=x",
                  "zenpi://pair?v=1&address=x&fingerprint=" + "g" * 64,
                  "zenpi://pair?v=1&address=not-a-host%3A7777&fingerprint=" + "a" * 64
                  + "&token=" + "A" * 43 + "&expires=1700000100",
                  self._invite().replace("192.168.50.14%3A7777", "192.168.50.14%3A7777%2Fpath"))
        for value in values:
            with self.subTest(value=value), self.assertRaises(ValueError):
                validate_invite(value, now=1700000000)

    def test_success_receipt_redacts_one_use_token(self):
        invite = self._invite(str(int(time.time()) + 100))
        script = self._script(f"print({invite!r}, flush=True)\n")
        receipt = Path(self.tmp.name) / "receipt.json"
        evidence = run(script, "127.0.0.1:0", None, receipt)
        encoded = receipt.read_text()
        self.assertTrue(evidence["passed"])
        self.assertNotIn(invite, encoded)
        saved = json.loads(encoded)
        self.assertEqual(saved["invite"]["token_sha256"], hashlib.sha256(TOKEN.encode()).hexdigest())
        self.assertNotIn("token", saved["invite"])
        self.assertTrue(saved["host_build_provenance_current"])
        self.assertEqual(saved["binary_sha256"], saved["host_build_provenance"]["artifact"]["sha256"])

    def test_missing_or_stale_provenance_refuses_before_launch_or_receipt_write(self):
        for change in ("missing", "source", "binary", "receipt"):
            script = self._script("print('must not run')\n")
            receipt, _ = builder.paths(self.root, "release")
            if change == "missing": receipt.unlink()
            elif change == "source": (self.root / "src/main.rs").write_text("edited")
            elif change == "binary": script.write_text("edited binary")
            else: receipt.write_text("{}")
            output = self.root / "smoke.json"
            output.write_text("prior evidence")
            with self.subTest(change=change), mock.patch("stage2_host_smoke.subprocess.Popen") as launch:
                with self.assertRaises(ValueError): run(script, "127.0.0.1:0", None, output)
                launch.assert_not_called()
                self.assertEqual(output.read_text(), "prior evidence")

    def test_symlink_or_unattributed_path_cannot_hide_binary_identity(self):
        script = self._script("print('must not run')\n")
        alias = self.root / "alias"
        alias.symlink_to(script)
        with mock.patch("stage2_host_smoke.subprocess.Popen") as launch:
            with self.assertRaises(ValueError): run(alias, "127.0.0.1:0", None, None)
            launch.assert_not_called()

    def test_runtime_source_binary_or_receipt_drift_cannot_pass(self):
        invite = self._invite(str(int(time.time()) + 100))
        for change in ("source", "binary", "receipt", "receipt_reformatted", "binary_deleted", "receipt_deleted"):
            target = self.root / {"source": "src/main.rs", "binary": "target/release/zenpi",
                                  "receipt": "target/stage2-host-build-release.json",
                                  "receipt_reformatted": "target/stage2-host-build-release.json",
                                  "binary_deleted": "target/release/zenpi",
                                  "receipt_deleted": "target/stage2-host-build-release.json"}[change]
            mutation = "path.unlink()" if change.endswith("deleted") else (
                "path.write_text(path.read_text() + ' ')" if change == "receipt_reformatted"
                else "path.write_text('changed during smoke')")
            script = self._script(f"from pathlib import Path\nprint({invite!r}, flush=True)\npath = Path({str(target)!r})\n{mutation}\n")
            before = builder.require_current_host(script, self.root)
            output = self.root / "changed.json"
            evidence = run(script, "127.0.0.1:0", None, output)
            self.assertEqual(evidence["exit_code"], 0)
            self.assertFalse(evidence["passed"])
            self.assertFalse(evidence["host_build_provenance_current"])
            self.assertEqual(evidence["host_build_provenance"], before)
            self.assertEqual(evidence["binary_sha256"], before["artifact"]["sha256"])
            self.assertEqual(evidence["error"], "Host build provenance changed during smoke")
            self.assertNotIn(TOKEN, output.read_text())

    def test_clean_exit_is_required(self):
        invite = self._invite(str(int(time.time()) + 100))
        script = self._script(f"import os, signal\nprint({invite!r}, flush=True)\nos.kill(os.getpid(), signal.SIGTERM)\n")
        evidence = run(script, "127.0.0.1:0", None, None)
        self.assertFalse(evidence["passed"])
        self.assertEqual(evidence["exit_code"], -15)

    def test_output_eof_before_process_exit_does_not_kill_or_accept_failed_host(self):
        invite = self._invite(str(int(time.time()) + 100))
        for code in (0, 7):
            with self.subTest(code=code):
                script = self._script(f"import os, time\nprint({invite!r}, flush=True)\n"
                    f"os.close(1)\nos.close(2)\ntime.sleep(0.1)\nos._exit({code})\n")
                evidence = run(script, "127.0.0.1:0", None, None)
                self.assertEqual(evidence["exit_code"], code)
                self.assertEqual(evidence["passed"], code == 0)
                self.assertFalse(evidence["timed_out"])

    def test_failure_receipt_redacts_stderr(self):
        script = self._script("import sys\nprint('secret-token', file=sys.stderr, flush=True)\n")
        evidence = run(script, "127.0.0.1:0", None, None)
        self.assertFalse(evidence["passed"])
        self.assertNotIn("secret-token", json.dumps(evidence))

    def test_host_child_does_not_inherit_unrelated_provider_credentials(self):
        script = self._script("import os, sys\nprint(os.environ.get('ANTHROPIC_API_KEY', ''), file=sys.stderr, flush=True)\n")
        with mock.patch.dict(os.environ, {"ANTHROPIC_API_KEY": "fixture-secret"}):
            evidence = run(script, "127.0.0.1:0", None, None)
        self.assertFalse(evidence["passed"])
        self.assertNotIn("fixture-secret", json.dumps(evidence))


if __name__ == "__main__":
    unittest.main()

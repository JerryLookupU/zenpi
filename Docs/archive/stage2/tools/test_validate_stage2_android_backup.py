import shutil
import tempfile
import unittest
from pathlib import Path
import xml.etree.ElementTree as ET

from validate_stage2_android_backup import ANDROID, validate


class BackupPolicyTest(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        source = Path(__file__).resolve().parents[1] / "android/app/src/main"
        self.main = self.root / "android/app/src/main"
        self.main.mkdir(parents=True)
        shutil.copyfile(source / "AndroidManifest.xml", self.main / "AndroidManifest.xml")
        shutil.copytree(source / "res/xml", self.main / "res/xml")
        destination = self.main / "java/zenpi/security"
        destination.mkdir(parents=True)
        for name in ("PairedHostStore.kt", "LocalResetFiles.kt"):
            shutil.copyfile(source / "java/zenpi/security" / name, destination / name)

    def mutate(self, path, change):
        file = self.main / path
        tree = ET.parse(file)
        change(tree.getroot())
        tree.write(file)

    def test_repository_policy_excludes_all_domains_on_both_transports(self):
        self.assertTrue(validate(self.root)["ok"])

    def test_manifest_cannot_enable_backup_or_redirect_policy(self):
        for key, value in [("allowBackup", "true"), ("fullBackupContent", "false"),
                           ("dataExtractionRules", "@xml/other"), ("backupAgent", ".CustomBackup")]:
            with self.subTest(key=key):
                original = (self.main / "AndroidManifest.xml").read_bytes()
                self.mutate("AndroidManifest.xml", lambda root: root.find("application").set(ANDROID + key, value))
                with self.assertRaises(ValueError):
                    validate(self.root)
                (self.main / "AndroidManifest.xml").write_bytes(original)

    def test_missing_partial_duplicate_unknown_and_included_domains_fail(self):
        for path in ["res/xml/backup_rules.xml", "res/xml/data_extraction_rules.xml"]:
            for change in [lambda branch: branch.remove(branch[0]),
                           lambda branch: branch[0].set("path", "paired-host.v1"),
                           lambda branch: branch[0].set("domain", branch[1].get("domain")),
                           lambda branch: branch[0].set("domain", "unknown"),
                           lambda branch: ET.SubElement(branch, "include", {"domain": "root", "path": "."})]:
                with self.subTest(path=path, change=change):
                    file = self.main / path
                    original = file.read_bytes()
                    self.mutate(path, lambda root: change(root if root.tag == "full-backup-content" else root[0]))
                    with self.assertRaises(ValueError):
                        validate(self.root)
                    file.write_bytes(original)

    def test_device_transfer_cannot_be_omitted_or_substituted(self):
        self.mutate("res/xml/data_extraction_rules.xml", lambda root: root.remove(root.find("device-transfer")))
        with self.assertRaises(ValueError):
            validate(self.root)

    def test_paired_metadata_must_stay_encrypted_and_no_backup(self):
        destination = self.main / "java/zenpi/security"
        self.assertTrue(validate(self.root)["ok"])
        original = (destination / "PairedHostStore.kt").read_text()
        (destination / "PairedHostStore.kt").write_text(original.replace("context.noBackupFilesDir", "context.filesDir"))
        with self.assertRaises(ValueError):
            validate(self.root)


if __name__ == "__main__":
    unittest.main()

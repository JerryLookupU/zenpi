#!/usr/bin/env python3
"""Validate source backup policy; not a merged APK or device-transfer receipt."""

import argparse
import json
from pathlib import Path
import xml.etree.ElementTree as ET


ANDROID = "{http://schemas.android.com/apk/res/android}"
DOMAINS = frozenset({"root", "file", "database", "sharedpref", "external",
                     "device_root", "device_file", "device_database", "device_sharedpref"})


def exclusions(node):
    if node.attrib or len(node) != len(DOMAINS):
        raise ValueError("Backup branch must exclude every data domain")
    domains = set()
    for child in node:
        if child.tag != "exclude" or set(child.attrib) != {"domain", "path"} or child.attrib["path"] != ".":
            raise ValueError("Unexpected backup inclusion or partial exclusion")
        domain = child.attrib["domain"]
        if domain not in DOMAINS or domain in domains or len(child):
            raise ValueError("Unknown or duplicate backup domain")
        domains.add(domain)


def validate(root):
    main = Path(root) / "android/app/src/main"
    manifest = ET.parse(main / "AndroidManifest.xml").getroot()
    apps = manifest.findall("application")
    if len(apps) != 1:
        raise ValueError("Exactly one application is required")
    application = apps[0]
    expected = {"allowBackup": "false", "fullBackupContent": "@xml/backup_rules",
                "dataExtractionRules": "@xml/data_extraction_rules"}
    if any(application.get(ANDROID + key) != value for key, value in expected.items()) or application.get(ANDROID + "backupAgent"):
        raise ValueError("Application backup policy missing or overridden")
    legacy = ET.parse(main / "res/xml/backup_rules.xml").getroot()
    if legacy.tag != "full-backup-content":
        raise ValueError("Invalid legacy backup root")
    exclusions(legacy)
    extraction = ET.parse(main / "res/xml/data_extraction_rules.xml").getroot()
    if extraction.tag != "data-extraction-rules" or extraction.attrib or len(extraction) != 2:
        raise ValueError("Invalid extraction policy")
    branches = {child.tag for child in extraction}
    if branches != {"cloud-backup", "device-transfer"}:
        raise ValueError("Cloud and device-transfer policies required")
    for child in extraction:
        exclusions(child)
    # Paired metadata is intentionally kept in noBackupFilesDir and protected
    # by Android Keystore. Reject a source regression that moves it into a
    # transferable location or weakens the local-reset contract.
    store_path = main / "java/zenpi/security/PairedHostStore.kt"
    reset_path = main / "java/zenpi/security/LocalResetFiles.kt"
    if not store_path.is_file() or not reset_path.is_file():
        raise ValueError("paired host storage sources are required for backup-policy validation")
    store = store_path.read_text()
    required_store_fragments = (
        'AtomicFile(File(context.noBackupFilesDir,',
        'KeyStore.getInstance("AndroidKeyStore")',
        'Cipher.getInstance("AES/GCM/NoPadding")',
        'private val reset = LocalResetFiles(file.baseFile)',
    )
    if any(fragment not in store for fragment in required_store_fragments):
        raise ValueError("paired host metadata must remain encrypted in noBackupFilesDir")
    reset = reset_path.read_text()
    if any(fragment not in reset for fragment in (".reset", "requireAvailable", "finish")):
        raise ValueError("local reset marker contract missing")
    return {"ok": True, "scope": "source_backup_policy", "domains": len(DOMAINS),
            "cloud_backup": "excluded", "device_transfer": "excluded", "legacy_backup": "excluded"}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    args = parser.parse_args()
    try:
        result = validate(args.root)
    except (ValueError, OSError, ET.ParseError) as error:
        print(json.dumps({"ok": False, "error": str(error)}))
        return 1
    print(json.dumps(result, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

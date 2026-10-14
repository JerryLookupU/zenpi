#!/usr/bin/env python3
"""Read-only signed APK inspection; not source attribution, install or release acceptance."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import time
import xml.etree.ElementTree as ET

from stage2_android_build import sha256
from validate_stage2_android_backup import exclusions, DOMAINS
from validate_stage2_android_entry import require_single_owner

ANDROID = "{http://schemas.android.com/apk/res/android}"


def certificate(output, expected):
    digests = re.findall(r"(?m)^Signer #[0-9]+ certificate SHA-256 digest: ([0-9a-fA-F]{64})$", output)
    counts = re.findall(r"(?m)^Number of signers: ([0-9]+)$", output)
    if counts != ["1"] or len(digests) != 1 or digests[0].lower() != expected:
        raise ValueError("APK signer does not match the single expected release certificate")
    return digests[0].lower()


def manifest(text):
    root = ET.fromstring(text)
    if root.tag != "manifest" or root.get("package") != "app.zenpi.android": raise ValueError("Release package mismatch")
    if any(root.get(ANDROID + name) is not None for name in ("sharedUserId", "sharedUserMaxSdkVersion")) or root.findall("instrumentation"):
        raise ValueError("Shared UID or instrumentation is not allowed in the release package")
    version = root.get(ANDROID + "versionCode", "")
    if not re.fullmatch(r"[1-9][0-9]*", version) or int(version) > 2100000000: raise ValueError("Invalid release version code")
    sdk = root.findall("uses-sdk")
    if len(sdk) != 1 or sdk[0].get(ANDROID + "minSdkVersion") != "31" or sdk[0].get(ANDROID + "targetSdkVersion") != "35":
        raise ValueError("Release API range mismatch")
    apps = root.findall("application")
    if len(apps) != 1: raise ValueError("Exactly one release application required")
    app = apps[0]
    for name in ("debuggable", "testOnly"):
        if app.get(ANDROID + name, "false") != "false": raise ValueError("Debug/test-only release APK refused")
    if app.get(ANDROID + "allowBackup") != "false" or app.get(ANDROID + "usesCleartextTraffic") != "false":
        raise ValueError("Release backup/cleartext policy mismatch")
    if any(not app.get(ANDROID + name, "").startswith(("@ref/", "@xml/")) for name in ("fullBackupContent", "dataExtractionRules")):
        raise ValueError("Release extraction/backup rule references missing")
    if app.get(ANDROID + "backupAgent") or app.get(ANDROID + "networkSecurityConfig"):
        raise ValueError("Unexpected release backup/network override")
    permissions = [node.get(ANDROID + "name") for node in root if node.tag.startswith("uses-permission")]
    allowed = {"android.permission.INTERNET", "android.permission.ACCESS_NETWORK_STATE", "android.permission.CAMERA",
               "android.permission.WAKE_LOCK", "android.permission.RECEIVE_BOOT_COMPLETED",
               "app.zenpi.android.DYNAMIC_RECEIVER_NOT_EXPORTED_PERMISSION"}
    if not {"android.permission.INTERNET", "android.permission.ACCESS_NETWORK_STATE", "android.permission.CAMERA"}.issubset(permissions) \
        or len(permissions) != len(set(permissions)) or any(name not in allowed for name in permissions): raise ValueError("Release permission set mismatch")
    dynamic_permission = "app.zenpi.android.DYNAMIC_RECEIVER_NOT_EXPORTED_PERMISSION"
    declarations = root.findall("permission")
    if root.findall("permission-tree") or root.findall("permission-group") or len(declarations) > 1:
        raise ValueError("Unexpected release permission declaration")
    if declarations:
        declaration = declarations[0]
        if declaration.attrib != {ANDROID + "name": dynamic_permission, ANDROID + "protectionLevel": declaration.get(ANDROID + "protectionLevel")} \
            or declaration.get(ANDROID + "protectionLevel") not in ("signature", "0x2") or len(declaration):
            raise ValueError("Release receiver permission must be signature-only")
    if (dynamic_permission in permissions) != bool(declarations):
        raise ValueError("Release receiver permission declaration/use mismatch")
    if any(node.get(ANDROID + "name") == "androidx.work.impl.foreground.SystemForegroundService" for node in app.findall("service")):
        raise ValueError("Unexpected foreground service")
    activities = [node for node in app.findall("activity") if node.get(ANDROID + "name") == "zenpi.MainActivity"]
    if len(activities) != 1 or activities[0].get(ANDROID + "exported") != "true": raise ValueError("Release entry activity mismatch")
    require_single_owner(app, "zenpi.MainActivity")
    filters = activities[0].findall("intent-filter")
    launcher = any({node.get(ANDROID + "name") for node in item.findall("action")} == {"android.intent.action.MAIN"}
                   and {node.get(ANDROID + "name") for node in item.findall("category")} == {"android.intent.category.LAUNCHER"} for item in filters)
    deep = [item for item in filters if any(node.get(ANDROID + "name") == "android.intent.action.VIEW" for node in item.findall("action"))]
    if not launcher or len(deep) != 1 or len(deep[0].findall("data")) != 1 \
        or {node.get(ANDROID+"name") for node in deep[0].findall("action")} != {"android.intent.action.VIEW"} \
        or {node.get(ANDROID+"name") for node in deep[0].findall("category")} != {"android.intent.category.DEFAULT", "android.intent.category.BROWSABLE"} \
        or deep[0].find("data").attrib != {ANDROID+"scheme":"zenpi", ANDROID+"host":"pair"}:
        raise ValueError("Release launcher or QR deep-link route mismatch")
    protected = {"androidx.work.impl.background.systemjob.SystemJobService":("service", "android.permission.BIND_JOB_SERVICE"),
        "androidx.work.impl.diagnostics.DiagnosticsReceiver":("receiver", "android.permission.DUMP"),
        "androidx.profileinstaller.ProfileInstallReceiver":("receiver", "android.permission.DUMP")}
    component_names = set()
    for node in app:
        if node.tag not in ("activity", "activity-alias", "service", "receiver", "provider"): continue
        name = node.get(ANDROID+"name")
        if not name or name in component_names: raise ValueError("Missing or duplicate release component identity")
        component_names.add(name)
        exported = node.get(ANDROID+"exported")
        if exported == "false" or node is activities[0]: continue
        if exported is None and not node.findall("intent-filter") and node.tag != "provider": continue
        if exported != "true" or protected.get(name) != (node.tag, node.get(ANDROID+"permission")):
            raise ValueError("Unexpected exported release component")
    return {"package":"app.zenpi.android", "version_code":int(version), "min_sdk":31, "target_sdk":35,
            "debuggable":False, "test_only":False, "permissions":sorted(permissions)}


def backup_references(text, table):
    app = ET.fromstring(text).find("application")
    entries = re.findall(r"(?m)^    resource (0x[0-9a-f]{8}) xml/([A-Za-z0-9_]+)$", table)
    names = ("backup_rules", "data_extraction_rules")
    for attribute, name in zip(("fullBackupContent", "dataExtractionRules"), names):
        matches = [identity for identity, resource in entries if resource == name]
        if len(matches) != 1 or sum(identity == matches[0] for identity, _ in entries) != 1:
            raise ValueError("Missing or ambiguous packaged backup resource")
        if app.get(ANDROID + attribute) not in ("@xml/" + name, "@ref/" + matches[0]):
            raise ValueError("Manifest does not reference the inspected backup resource")
    return names


def backup_xml(text, extraction=False):
    root = ET.fromstring(text)
    if not extraction:
        if root.tag != "full-backup-content": raise ValueError("Invalid packaged backup root")
        exclusions(root)
    else:
        if root.tag != "data-extraction-rules" or root.attrib or len(root) != 2 \
                or {child.tag for child in root} != {"cloud-backup", "device-transfer"}:
            raise ValueError("Invalid packaged extraction policy")
        for child in root: exclusions(child)


def packaged_backup(apk, sdk, manifest_text, run):
    analyzer = str(sdk/"cmdline-tools/latest/bin/apkanalyzer")
    table = run([str(sdk/"build-tools/35.0.0/aapt2"), "dump", "resources", "--no-values", str(apk)])
    names = backup_references(manifest_text, table)
    configs = run([analyzer, "resources", "configs", "--package", "app.zenpi.android", "--type", "xml", str(apk)])
    # This release format has only unqualified policy XML. Refuse variants
    # rather than inspecting the default while Android selects an override.
    if configs.splitlines() != ["default"]: raise ValueError("Unexpected packaged XML configuration")
    policies = {}
    for index, name in enumerate(names):
        path = run([analyzer, "resources", "value", "--package", "app.zenpi.android", "--config", "default",
                    "--type", "xml", "--name", name, str(apk)]).strip()
        if path != f"res/xml/{name}.xml": raise ValueError("Unexpected packaged backup XML path")
        text = run([analyzer, "resources", "xml", "--file", path, str(apk)])
        backup_xml(text, extraction=index == 1)
        policies[name] = {"path":path, "decoded_xml_sha256":hashlib.sha256(text.encode()).hexdigest()}
    return {"domains":len(DOMAINS), "resources":policies,
            "legacy_backup":"excluded", "cloud_backup":"excluded", "device_transfer":"excluded"}


def inspect(apk, expected, sdk, receipt):
    if not re.fullmatch(r"[0-9a-f]{64}", expected): raise ValueError("Expected signer must be lowercase SHA-256")
    if receipt.exists() or receipt.is_symlink(): raise ValueError("Refusing to overwrite release inspection evidence")
    before = sha256(apk)
    commands = [
        [str(sdk/"build-tools/35.0.0/apksigner"), "verify", "--verbose", "--print-certs", "--min-sdk-version", "31", "--max-sdk-version", "35", str(apk)],
        [str(sdk/"build-tools/35.0.0/zipalign"), "-c", "-P", "16", "4", str(apk)],
        [str(sdk/"cmdline-tools/latest/bin/apkanalyzer"), "manifest", "print", str(apk)],
    ]
    evidence = []; started = time.time(); failure = None; summary = None
    try:
        def run(command):
            result = subprocess.run(command, capture_output=True, timeout=60)
            evidence.append({"argv":command, "exit_code":result.returncode,
                "stdout_sha256":hashlib.sha256(result.stdout).hexdigest(), "stderr_sha256":hashlib.sha256(result.stderr).hexdigest()})
            if result.returncode: raise ValueError("APK inspection tool refused artifact")
            if len(result.stdout) > 4 * 1024 * 1024 or len(result.stderr) > 1024 * 1024:
                raise ValueError("APK inspection output exceeds limit")
            return result.stdout.decode("utf-8", errors="strict")
        outputs = [run(command) for command in commands]
        signer = certificate(outputs[0], expected)
        summary = manifest(outputs[2]) | {"signer_sha256":signer, "zip_alignment_verified":True}
        summary["backup_policy"] = packaged_backup(apk, sdk, outputs[2], run)
        if sha256(apk) != before: raise ValueError("APK changed during inspection")
    except (ValueError, OSError, ET.ParseError, subprocess.TimeoutExpired) as error:
        failure = type(error).__name__ + ": release artifact inspection incomplete"
    result = {"schema_version":"stage2-release-inspection/v2", "passed":failure is None,
        "whole_item_accepted":False, "source_attribution_verified":False, "install_verified":False,
        "upgrade_verified":False, "reproducible_build_verified":False, "backup_rule_contents_verified":failure is None,
        "scope":"single signed APK certificate/alignment/merged-manifest and packaged backup XML inspection only; not runtime backup/transfer behavior",
        "apk":{"path":str(apk),"sha256":before}, "expected_signer_sha256":expected,
        "summary":summary, "commands":evidence, "failure":failure,
        "started_at_unix":started,"finished_at_unix":time.time()}
    receipt.parent.mkdir(parents=True,exist_ok=True)
    with receipt.open("x") as output: json.dump(result,output,indent=2); output.write("\n")
    return result


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--apk",type=Path,required=True)
    parser.add_argument("--expected-signer-sha256",required=True)
    parser.add_argument("--sdk",type=Path,default=Path(os.environ.get("ANDROID_HOME",str(Path.home()/"Library/Android/sdk"))))
    parser.add_argument("--receipt",type=Path,required=True)
    args=parser.parse_args()
    try: result=inspect(args.apk,args.expected_signer_sha256,args.sdk,args.receipt)
    except (ValueError,OSError) as error: raise SystemExit(str(error)) from error
    print(json.dumps(result,indent=2))
    if not result["passed"]: raise SystemExit("Release artifact refused; inspect the failed receipt")


if __name__ == "__main__": main()

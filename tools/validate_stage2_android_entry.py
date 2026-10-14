#!/usr/bin/env python3
"""Validate single-owner pairing entry configuration, not Activity runtime behavior."""
import json
from pathlib import Path
import xml.etree.ElementTree as ET

ANDROID = "{http://schemas.android.com/apk/res/android}"


def require_single_owner(application, name):
    activities = [node for node in application.findall("activity") if node.get(ANDROID + "name") == name]
    if len(activities) != 1:
        raise ValueError("Exactly one pairing activity is required")
    activity = activities[0]
    if activity.get(ANDROID + "launchMode") not in ("singleTask", "2"):
        raise ValueError("Pairing activity must reuse its task owner")
    if activity.get(ANDROID + "documentLaunchMode") not in ("never", "3"):
        raise ValueError("Pairing activity must refuse document task duplication")
    expected = {"allowTaskReparenting": "false",
                "multiprocess": "false", "noHistory": "false"}
    if any(activity.get(ANDROID + key, value) != value for key, value in expected.items()) \
            or any(activity.get(ANDROID + key) is not None for key in ("process", "taskAffinity")) \
            or application.get(ANDROID + "allowTaskReparenting", "false") != "false":
        raise ValueError("Pairing activity task ownership may not be overridden")
    return activity


def require_private_scanner(application):
    activities = [node for node in application.findall("activity")
                  if node.get(ANDROID + "name") == ".PairingCaptureActivity"]
    if len(activities) != 1 or activities[0].get(ANDROID + "exported") != "false" \
            or activities[0].findall("intent-filter") \
            or any(activities[0].get(ANDROID + key) is not None for key in ("process", "taskAffinity")):
        raise ValueError("Exactly one private in-process pairing scanner is required")


def validate(root):
    manifest = ET.parse(Path(root) / "android/app/src/main/AndroidManifest.xml").getroot()
    apps = manifest.findall("application")
    if len(apps) != 1:
        raise ValueError("Exactly one application is required")
    require_single_owner(apps[0], ".MainActivity")
    require_private_scanner(apps[0])
    return {"ok": True, "scope": "source_pairing_entry_only", "launch_mode": "singleTask",
            "activity_runtime_verified": False}


if __name__ == "__main__":
    try:
        print(json.dumps(validate(Path(__file__).resolve().parents[1]), indent=2))
    except (ValueError, OSError, ET.ParseError) as error:
        raise SystemExit(str(error)) from error

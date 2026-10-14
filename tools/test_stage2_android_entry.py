import copy
from pathlib import Path
import unittest
import xml.etree.ElementTree as ET
from validate_stage2_android_entry import ANDROID, require_private_scanner, require_single_owner, validate


class PairingEntryTests(unittest.TestCase):
    def test_scanner_is_private_single_and_in_process(self):
        source = ET.fromstring('<application xmlns:android="http://schemas.android.com/apk/res/android">'
            '<activity android:name=".PairingCaptureActivity" android:exported="false"/></application>')
        require_private_scanner(source)
        for key, value in [("exported", None), ("exported", "true"), ("process", ":scan"), ("taskAffinity", "")]:
            application = copy.deepcopy(source)
            if value is None: application[0].attrib.pop(ANDROID + key)
            else: application[0].set(ANDROID + key, value)
            with self.subTest(key=key, value=value), self.assertRaises(ValueError):
                require_private_scanner(application)
        for kind in ("missing", "duplicate", "filter"):
            application = copy.deepcopy(source)
            if kind == "missing": application.remove(application[0])
            elif kind == "duplicate": application.append(copy.deepcopy(application[0]))
            else: ET.SubElement(application[0], "intent-filter")
            with self.subTest(kind=kind), self.assertRaises(ValueError): require_private_scanner(application)

    def test_current_source_requires_single_owner_without_claiming_runtime(self):
        evidence = validate(Path(__file__).resolve().parents[1])
        self.assertTrue(evidence["ok"])
        self.assertFalse(evidence["activity_runtime_verified"])

    def test_missing_duplicate_wrong_launch_or_task_overrides_are_refused(self):
        source = ET.fromstring('<application xmlns:android="http://schemas.android.com/apk/res/android">'
            '<activity android:name=".MainActivity" android:launchMode="singleTask" android:documentLaunchMode="never"/></application>')
        require_single_owner(source, ".MainActivity")
        for key, value in [("launchMode", None), ("launchMode", "standard"), ("launchMode", "singleTop"),
                           ("launchMode", "singleInstance"), ("documentLaunchMode", None), ("documentLaunchMode", "none"), ("documentLaunchMode", "always"),
                           ("allowTaskReparenting", "true"), ("multiprocess", "true"),
                           ("noHistory", "true"), ("taskAffinity", ""), ("process", ":parallel")]:
            application = copy.deepcopy(source)
            activity = application.find("activity")
            if value is None: activity.attrib.pop(ANDROID + key)
            else: activity.set(ANDROID + key, value)
            with self.subTest(key=key, value=value), self.assertRaises(ValueError):
                require_single_owner(application, ".MainActivity")
        application = copy.deepcopy(source)
        application.append(copy.deepcopy(application[0]))
        with self.assertRaises(ValueError): require_single_owner(application, ".MainActivity")
        with self.assertRaises(ValueError): require_single_owner(ET.Element("application"), ".MainActivity")
        application = copy.deepcopy(source)
        application.set(ANDROID + "allowTaskReparenting", "true")
        with self.assertRaises(ValueError): require_single_owner(application, ".MainActivity")


if __name__ == "__main__": unittest.main()

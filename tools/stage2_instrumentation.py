"""Verify named JUnit start/success pairs, not just the runner's summary count."""
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[1]


def expected_tests(classes, root=ROOT):
    if not classes or len(classes) != len(set(classes)):
        raise ValueError("Invalid instrumentation class set")
    identities = set()
    for name in classes:
        if not re.fullmatch(r"zenpi\.[A-Za-z][A-Za-z0-9]*", name):
            raise ValueError("Invalid instrumentation class name")
        source = (root / "android/app/src/androidTest/java" / (name.replace(".", "/") + ".kt")).read_text()
        methods = re.findall(r"@Test\s+fun\s+([A-Za-z][A-Za-z0-9_]*)\s*\(", source)
        # Refuse new annotation/method styles until discovery supports them.
        if not methods or len(methods) != len(set(methods)) or len(methods) != len(re.findall(r"(?m)^\s*@Test\b", source)):
            raise ValueError("Unsupported instrumentation source methods")
        identities.update((name, method) for method in methods)
    return identities


def require_test_results(output, classes, count):
    expected = expected_tests(classes)
    if len(expected) != count or len(output.encode()) > 4 * 1024 * 1024:
        raise ValueError("Instrumentation suite count or output limit mismatch")
    if re.findall(r"(?m)^OK \(([0-9]+) tests?\)$", output) != [str(count)] or "FAILURES" in output:
        raise ValueError("Instrumentation summary mismatch")
    fields = {}
    active = None
    completed = set()
    for line in output.splitlines():
        match = re.fullmatch(r"INSTRUMENTATION_STATUS: (class|test|current|numtests)=(.*)", line)
        if match:
            key, value = match.groups()
            if key in fields:
                raise ValueError("Duplicate instrumentation status field")
            fields[key] = value
        elif line.startswith("INSTRUMENTATION_STATUS_CODE:"):
            code = line.removeprefix("INSTRUMENTATION_STATUS_CODE:").strip()
            if fields:
                if set(fields) != {"class", "test", "current", "numtests"}:
                    raise ValueError("Incomplete instrumentation identity")
                identity = fields["class"], fields["test"]
                if identity not in expected or fields["numtests"] != str(count) or fields["current"] != str(len(completed) + 1):
                    raise ValueError("Unexpected instrumentation test identity or index")
                if code == "1" and active is None and identity not in completed:
                    active = identity
                elif code == "0" and active == identity and identity not in completed:
                    completed.add(identity)
                    active = None
                else:
                    raise ValueError("Instrumentation test skipped, failed or unpaired")
            elif code != "0":
                raise ValueError("Unscoped instrumentation status")
            fields = {}
    if fields or active is not None or completed != expected:
        raise ValueError("Missing completed instrumentation tests")
    return sorted(f"{name}#{method}" for name, method in completed)

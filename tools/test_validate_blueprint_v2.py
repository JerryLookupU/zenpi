#!/usr/bin/env python3
"""Negative and positive cases for tools/validate_blueprint_v2.py."""

from __future__ import annotations

import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from validate_blueprint_v2 import BlueprintError, validate  # noqa: E402


HEADER = """```yaml
blueprint_version: 9.9.9
authoritative: true
stable_id_pattern: '^ZE-[0-9]{3}$'
layers: A0, A1, A2, B1, B2
per_item_code_loc_cap: 5000
```
"""


def line(item_id: str, state: str = " ", layer: str = "A1", depends: str = "—", loc: int = 10) -> str:
    return (
        f"- [{state}] **{item_id}** — title；layer `{layer}` | Depends: {depends} | "
        f"Owned paths: `src/x.rs` | Validators: test | Estimated LOC: {loc}"
    )


def doc(*lines: str, header: str = HEADER) -> str:
    return header + "\n" + "\n".join(lines) + "\n"


class ValidateBlueprintV2(unittest.TestCase):
    def assert_rejects(self, text: str, fragment: str) -> None:
        with self.assertRaises(BlueprintError) as ctx:
            validate(text)
        self.assertIn(fragment, str(ctx.exception))

    def test_valid_document(self) -> None:
        summary = validate(doc(line("ZE-001", "x", "A0"), line("ZE-100", "_", depends="ZE-001"), line("ZE-101")))
        self.assertEqual((summary["items"], summary["accepted"], summary["self_tested"]), (3, 1, 1))

    def test_repository_blueprint_is_valid(self) -> None:
        root = Path(__file__).resolve().parent.parent
        validate((root / "Docs/Zenpi_Execution_Blueprint_v2.md").read_text(encoding="utf-8"))

    def test_rejects_duplicate_id(self) -> None:
        self.assert_rejects(doc(line("ZE-100"), line("ZE-100")), "duplicate id")

    def test_rejects_unknown_dependency(self) -> None:
        self.assert_rejects(doc(line("ZE-100", depends="ZE-999")), "unknown dependency")

    def test_rejects_cycle(self) -> None:
        self.assert_rejects(doc(line("ZE-100", depends="ZE-101"), line("ZE-101", depends="ZE-100")), "cycle")

    def test_rejects_loc_at_cap(self) -> None:
        self.assert_rejects(doc(line("ZE-100", loc=5000)), "Estimated LOC")

    def test_rejects_bad_state(self) -> None:
        self.assert_rejects(doc(line("ZE-100").replace("- [ ]", "- [v]")), "malformed")

    def test_rejects_missing_field(self) -> None:
        self.assert_rejects(doc(line("ZE-100").replace("| Validators: test ", "")), "Validators")

    def test_rejects_closed_item_with_open_dependency(self) -> None:
        self.assert_rejects(doc(line("ZE-100"), line("ZE-101", "x", depends="ZE-100")), "dependency ZE-100 is open")

    def test_rejects_layer_gate_violation(self) -> None:
        self.assert_rejects(doc(line("ZE-100", layer="A1"), line("ZE-200", "x", "A2")), "still has open items")

    def test_rejects_dependency_on_higher_layer(self) -> None:
        self.assert_rejects(doc(line("ZE-100", layer="A1", depends="ZE-200"), line("ZE-200", layer="B2")), "higher-layer")

    def test_layer_gate_is_per_block(self) -> None:
        validate(doc(line("ZE-500", layer="B1"), line("ZE-200", "x", "A2")))

    def test_cross_block_dependency_on_same_level_is_allowed(self) -> None:
        validate(doc(line("ZE-100", layer="A1"), line("ZE-500", layer="B1", depends="ZE-100")))

    def test_rejects_non_authoritative(self) -> None:
        self.assert_rejects(doc(line("ZE-100"), header=HEADER.replace("authoritative: true", "authoritative: false")), "authoritative")

    def test_rejects_id_outside_pattern(self) -> None:
        self.assert_rejects(doc(line("V2-100")), "stable_id_pattern")


if __name__ == "__main__":
    unittest.main()

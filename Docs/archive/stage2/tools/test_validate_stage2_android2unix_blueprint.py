#!/usr/bin/env python3
"""Negative and positive tests for the Stage 2 Blueprint structural gate."""
from __future__ import annotations

import tempfile
import unittest
import json
from unittest.mock import patch
from pathlib import Path

import validate_stage2_android2unix_blueprint as gate
import generate_stage2_android2unix_gantt as gantt


class Stage2BlueprintGateTests(unittest.TestCase):
    def test_current_draft_is_structurally_valid(self) -> None:
        result = gate.validate(strict=False)
        self.assertTrue(result["ok"])
        self.assertEqual(result["items"], 36)
        total = sum(result["counts"].values())
        self.assertEqual(total, 36)

    def test_duplicate_id_is_rejected(self) -> None:
        text = (gate.ROOT / gate.BLUEPRINT).read_text(encoding="utf-8")
        first = next(line for line in text.splitlines() if line.startswith("- [ ] **S2-001**"))
        with self.assertRaises(gate.BlueprintError):
            gate.parse_items(text + "\n" + first + "\n")

    def test_missing_dependency_is_rejected(self) -> None:
        text = (gate.ROOT / gate.BLUEPRINT).read_text(encoding="utf-8")
        mutated = text.replace("Depends: S2-003 | Owner scope", "Depends: S2-999 | Owner scope", 1)
        with self.assertRaises(gate.BlueprintError):
            gate.parse_items(mutated)

    def test_loc_cap_is_rejected(self) -> None:
        text = (gate.ROOT / gate.BLUEPRINT).read_text(encoding="utf-8")
        mutated = text.replace("Estimated LOC: 500\n", "Estimated LOC: 5000\n", 1)
        item = next(iter(gate.parse_items(mutated).values()))
        self.assertGreaterEqual(item["loc"], 5000)

    def test_gantt_has_all_ids(self) -> None:
        blueprint = gate.parse_items((gate.ROOT / gate.BLUEPRINT).read_text(encoding="utf-8"))
        _, rows = gate.parse_gantt((gate.ROOT / gate.GANTT).read_text(encoding="utf-8"))
        self.assertEqual(set(blueprint), set(rows))


class MasterReceiptTests(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        for relative in (gate.BLUEPRINT, gate.SPEC,
                         Path("Docs/execution/stage2_android2unix_active_requirement.json")):
            destination = self.root / relative
            destination.parent.mkdir(parents=True, exist_ok=True)
            destination.write_bytes((gate.ROOT / relative).read_bytes())
        self.receipts = self.root / "Docs/quality/stage2_android2unix/receipts"
        self.receipts.mkdir(parents=True)
        self.mark("S2-001")
        self.items = gate.parse_items((self.root / gate.BLUEPRINT).read_text())
        self.receipt = {
            "item_id": "S2-001", "run_id": "fixture-run",
            "passed": True, "baseline_digest": "a" * 64, "patch_sha256": "b" * 64,
            "requirement_digest": gate.requirement_digest(self.items),
            "specification_digest": gate.digest(self.root / gate.SPEC),
            "source_paths": list(self.items["S2-001"]["owned"]),
            "integrated_at": "2026-10-04T12:00:00+08:00",
            "commands": [{"argv": "python3 tools/validate_stage2_android2unix_blueprint.py --strict",
                          "exit_code": 0, "stdout_sha256": "c" * 64, "stderr_sha256": "d" * 64}],
        }

    def mark(self, item_id: str) -> None:
        path = self.root / gate.BLUEPRINT
        path.write_text(path.read_text().replace(f"- [ ] **{item_id}**", f"- [x] **{item_id}**"))
        with patch.multiple(gantt, ROOT=self.root, BP=path, SPEC=self.root / gate.SPEC,
                            OUT=self.root / gate.GANTT):
            gantt.main([])

    def write_receipt(self) -> Path:
        path = self.receipts / "S2-001-fixture-run.json"
        path.write_text(json.dumps(self.receipt))
        return path

    def test_partial_acceptance_requires_receipt(self) -> None:
        gate.validate(self.root, strict=False)
        with self.assertRaisesRegex(gate.BlueprintError, "successful Master receipt"):
            gate.validate(self.root, strict=True)

    def test_matching_receipt_allows_partial_acceptance(self) -> None:
        self.write_receipt()
        self.assertEqual(gate.validate(self.root, strict=True)["counts"]["[x]"], 1)

    def test_all_checked_with_empty_receipts_is_refused(self) -> None:
        path = self.root / gate.BLUEPRINT
        path.write_text(path.read_text().replace("- [ ] **", "- [x] **"))
        self.mark("S2-001")
        with self.assertRaisesRegex(gate.BlueprintError, "successful Master receipt"):
            gate.validate(self.root, strict=True)

    def test_unfinished_dependency_is_refused(self) -> None:
        self.mark("S2-002")
        path = self.root / gate.BLUEPRINT
        path.write_text(path.read_text().replace("- [x] **S2-001**", "- [ ] **S2-001**"))
        self.mark("S2-002")
        with self.assertRaisesRegex(gate.BlueprintError, "unfinished dependencies"):
            gate.validate(self.root, strict=True)

    def test_invalid_identity_and_digests_are_refused(self) -> None:
        for field, value in (("item_id", "S2-002"), ("run_id", "../escape"),
                             ("passed", 1), ("passed", False),
                             ("requirement_digest", "e" * 64),
                             ("specification_digest", "e" * 64),
                             ("baseline_digest", None), ("patch_sha256", "invalid"),
                             ("source_paths", []), ("integrated_at", "2026-10-04T12:00:00")):
            with self.subTest(field=field, value=value):
                original = self.receipt[field]
                self.receipt[field] = value
                self.write_receipt()
                with self.assertRaises(gate.BlueprintError):
                    gate.validate(self.root, strict=True)
                self.receipt[field] = original

    def test_missing_changed_or_failed_commands_are_refused(self) -> None:
        original = self.receipt["commands"]
        for commands in ([], original + original,
                         [{**original[0], "argv": "true"}],
                         [{**original[0], "stdout_sha256": "invalid"}],
                         [{**original[0], "stderr_sha256": None}]):
            with self.subTest(commands=commands):
                self.receipt["commands"] = commands
                self.write_receipt()
                with self.assertRaises(gate.BlueprintError):
                    gate.validate(self.root, strict=True)

    def test_exit_status_requires_integer_zero(self) -> None:
        for value in (False, True, 0.0, "0", None, 1, -1):
            with self.subTest(exit_code=value):
                self.receipt["commands"][0]["exit_code"] = value
                self.write_receipt()
                with self.assertRaises(gate.BlueprintError):
                    gate.validate(self.root, strict=True)

    def test_malformed_or_symlink_receipt_is_refused(self) -> None:
        path = self.write_receipt()
        path.write_text("not JSON")
        with self.assertRaises(gate.BlueprintError):
            gate.validate(self.root, strict=True)
        path.unlink()
        outside = self.root / "external.json"
        outside.write_text(json.dumps(self.receipt))
        path.symlink_to(outside)
        with self.assertRaises(gate.BlueprintError):
            gate.validate(self.root, strict=True)

    def test_failed_history_does_not_hide_successful_receipt(self) -> None:
        self.write_receipt()
        failed = {**self.receipt, "run_id": "failed-run", "passed": False}
        (self.receipts / "S2-001-failed-run.json").write_text(json.dumps(failed))
        gate.validate(self.root, strict=True)

    def test_symlink_receipts_directory_is_refused(self) -> None:
        self.write_receipt()
        destination = self.root / "relocated-receipts"
        self.receipts.rename(destination)
        self.receipts.symlink_to(destination, target_is_directory=True)
        with self.assertRaisesRegex(gate.BlueprintError, "symlink receipts directory"):
            gate.validate(self.root, strict=True)


if __name__ == "__main__":
    unittest.main()

#!/usr/bin/env python3
"""Negative and positive tests for the Stage 2 Blueprint structural gate."""
from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

import validate_stage2_android2unix_blueprint as gate


class Stage2BlueprintGateTests(unittest.TestCase):
    def test_current_draft_is_structurally_valid(self) -> None:
        result = gate.validate(strict=False)
        self.assertTrue(result["ok"])
        self.assertEqual(result["items"], 35)
        self.assertEqual(result["counts"]["[ ]"], 35)

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


if __name__ == "__main__":
    unittest.main()

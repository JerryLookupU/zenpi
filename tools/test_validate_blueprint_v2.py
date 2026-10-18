#!/usr/bin/env python3
"""Negative and positive cases for tools/validate_blueprint_v2.py."""

from __future__ import annotations

import json
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from validate_blueprint_v2 import (  # noqa: E402
    BlueprintError,
    canonical_digest,
    find_overlaps,
    parse_items,
    validate,
)


HEADER = """```yaml
blueprint_version: 9.9.9
authoritative: true
stable_id_pattern: '^ZE-[0-9]{3}$'
layers: A0, A1, A2, B1, B2
per_item_code_loc_cap: 5000
```
"""


def line(
    item_id: str,
    state: str = " ",
    layer: str = "A1",
    depends: str = "—",
    loc: int = 10,
    owned: str = "`src/x.rs`",
) -> str:
    return (
        f"- [{state}] **{item_id}** — title；layer `{layer}` | Depends: {depends} | "
        f"Owned paths: {owned} | Validators: test | Estimated LOC: {loc}"
    )


def write_handoff(directory: Path, item_id: str, /, **overrides: object) -> dict:
    """Write a digest-valid handoff, then apply `overrides` after digesting."""
    record = {
        "schema_version": 1,
        "item_id": item_id,
        "claim_id": f"{item_id}@blueprint-9.9.9",
        "from": "worker",
        "to": "master",
        "summary": "s",
        "changed_files": ["src/x.rs"],
        "diff_stat": "1 file changed",
        "commands_run": ["test"],
        "validation_result": {"test": "pass"},
        "notes": "",
        "created_at": "2026-10-14T00:00:00Z",
    }
    record["digest"] = canonical_digest(record)
    record.update(overrides)
    (directory / f"{item_id}.json").write_text(json.dumps(record), encoding="utf-8")
    return record


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
        text = (root / "Docs/Zenpi_Execution_Blueprint_v2.md").read_text(encoding="utf-8")
        summary = validate(text, root / "Docs/quality/ze")
        self.assertGreater(summary["handoffs_checked"], 0)

    def test_valid_handoff_passes(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            write_handoff(Path(tmp), "ZE-100")
            summary = validate(doc(line("ZE-100", "_")), Path(tmp))
            self.assertEqual(summary["handoffs_checked"], 1)

    def test_missing_handoff_fails(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            with self.assertRaises(BlueprintError) as ctx:
                validate(doc(line("ZE-100", "_")), Path(tmp))
            self.assertIn("missing handoff", str(ctx.exception))

    def test_tampered_digest_fails(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            write_handoff(Path(tmp), "ZE-100", summary="edited after digest")
            with self.assertRaises(BlueprintError) as ctx:
                validate(doc(line("ZE-100", "x")), Path(tmp))
            self.assertIn("digest mismatch", str(ctx.exception))

    def test_handoff_requires_commands_and_matching_id(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            record = write_handoff(Path(tmp), "ZE-100", commands_run=[])
            record["digest"] = canonical_digest(record)
            (Path(tmp) / "ZE-100.json").write_text(json.dumps(record), encoding="utf-8")
            with self.assertRaises(BlueprintError) as ctx:
                validate(doc(line("ZE-100", "_")), Path(tmp))
            self.assertIn("commands_run", str(ctx.exception))
            write_handoff(Path(tmp), "ZE-100", item_id="ZE-101")
            with self.assertRaises(BlueprintError) as ctx:
                validate(doc(line("ZE-100", "_")), Path(tmp))
            self.assertIn("item_id", str(ctx.exception))

    def test_a0_accepted_needs_no_handoff(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            summary = validate(doc(line("ZE-001", "x", "A0"), line("ZE-100")), Path(tmp))
            self.assertEqual(summary["handoffs_checked"], 0)

    def test_overlap_report_finds_shared_path_and_ignores_dependent_pairs(self) -> None:
        items = parse_items(
            doc(
                line("ZE-100", owned="`src/tui.rs`, `src/a.rs`"),
                line("ZE-101", owned="`src/tui.rs`"),
                line("ZE-102", depends="ZE-103", owned="`src/a.rs`"),
                line("ZE-103", depends="ZE-100", owned="`src/b.rs`"),
                line("ZE-104", owned="`android/`"),
                line("ZE-105", owned="`android/app/Main.kt`"),
                line("ZE-106", "x", owned="`src/tui.rs`"),
            )
        )
        pairs = {(o["a"], o["b"]): o["paths"] for o in find_overlaps(items)}
        self.assertEqual(pairs[("ZE-100", "ZE-101")], ["src/tui.rs"])
        self.assertEqual(pairs[("ZE-104", "ZE-105")], ["android/app/Main.kt"])
        self.assertNotIn(("ZE-100", "ZE-102"), pairs)  # transitive ZE-102 -> ZE-103 -> ZE-100
        self.assertFalse(any("ZE-106" in pair for pair in pairs))  # accepted items are not open
        self.assertEqual(len(pairs), 2)

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

#!/usr/bin/env python3
"""Validate the authoritative v2 execution Blueprint (b3ehive checklist form).

Checks the claims the execution controller relies on: header metadata, one
checklist line per stable ID, legal checkbox states, required fields, a
closed and acyclic dependency DAG, dependencies never pointing at a higher
layer, the strict layer gate, `[x]` items having only `[x]` dependencies, and
the strict per-item LOC cap.  Standard library only.

Handoff evidence (§4): every `[_]` or `[x]` item outside the delivered
level-0 baseline must have `Docs/quality/ze/<ID>.json` carrying `item_id`,
`changed_files`, a non-empty `commands_run`, a `validation_result` object and
a `digest` equal to the sha256 of the canonical JSON of every other field.

`--report-overlaps` lists pairs of open items (`[ ]`/`[_]`) that share an
owned path without a transitive `Depends` path between them.  Overlaps are a
report for the Master's merge order, never an error.

Layers are `<block letter><level>` (e.g. `A2`, `B3`).  The layer gate applies
within one block; dependencies may cross blocks but never point at a higher
level.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from pathlib import Path
from typing import Any


DEFAULT_PATH = Path("Docs/Zenpi_Execution_Blueprint_v2.md")
EVIDENCE_DIR = Path("Docs/quality/ze")
ITEM_RE = re.compile(r"^- \[(?P<state>[ _x])\] \*\*(?P<id>[A-Z0-9]+-[0-9]{3})\*\* — (?P<body>.*)$")
LOOSE_ITEM_RE = re.compile(r"^\s*[-*] \[.\] \*\*[A-Z0-9]+-[0-9]{3}\*\*")
LAYER_RE = re.compile(r"layer `([A-Z][0-9]+)`")
DEPENDS_RE = re.compile(r"\| Depends: ([^|]*)")
LOC_RE = re.compile(r"\| Estimated LOC: ([0-9]+)\s*$")
OWNED_RE = re.compile(r"\| Owned paths: (.*?) \| Validators:")
REQUIRED_FIELDS = ("| Depends:", "| Owned paths:", "| Validators:", "| Estimated LOC:")


class BlueprintError(Exception):
    """A deterministic validation failure."""


def parse_header(text: str) -> dict[str, str]:
    match = re.search(r"```yaml\s*\n(.*?)\n```", text, re.DOTALL)
    if not match:
        raise BlueprintError("missing fenced yaml header")
    values: dict[str, str] = {}
    for raw in match.group(1).splitlines():
        raw = raw.strip()
        if not raw or raw.startswith("#"):
            continue
        key, sep, value = raw.partition(":")
        if not sep or not re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", key):
            raise BlueprintError(f"invalid yaml header line: {raw!r}")
        if key in values:
            raise BlueprintError(f"duplicate yaml key: {key}")
        values[key] = value.strip().strip("'\"")
    return values


def parse_items(text: str) -> list[dict[str, Any]]:
    items: list[dict[str, Any]] = []
    for lineno, line in enumerate(text.splitlines(), 1):
        match = ITEM_RE.match(line)
        if not match:
            if LOOSE_ITEM_RE.match(line):
                raise BlueprintError(f"line {lineno}: malformed checklist line")
            continue
        body = match.group("body")
        for field in REQUIRED_FIELDS:
            if field not in body:
                raise BlueprintError(f"{match.group('id')}: missing field {field.strip('| :')}")
        layer = LAYER_RE.search(body)
        depends = DEPENDS_RE.search(body)
        loc = LOC_RE.search(body)
        if not layer:
            raise BlueprintError(f"{match.group('id')}: missing layer")
        if not loc:
            raise BlueprintError(f"{match.group('id')}: Estimated LOC must be the final integer field")
        owned = OWNED_RE.search(body)
        deps_raw = depends.group(1).strip() if depends else ""
        deps = [] if deps_raw in {"", "—", "-"} else [d.strip() for d in deps_raw.split(",")]
        items.append(
            {
                "id": match.group("id"),
                "state": match.group("state"),
                "layer": layer.group(1),
                "depends": deps,
                "loc": int(loc.group(1)),
                "owned": re.findall(r"`([^`]+)`", owned.group(1)) if owned else [],
                "title": body.split("；layer", 1)[0].strip(),
                "line": lineno,
            }
        )
    return items


def canonical_digest(record: dict[str, Any]) -> str:
    """sha256 of the canonical JSON of every field except `digest`."""
    payload = {key: value for key, value in record.items() if key != "digest"}
    canonical = json.dumps(payload, sort_keys=True, separators=(",", ":"), ensure_ascii=False)
    return hashlib.sha256(canonical.encode("utf-8")).hexdigest()


def load_handoff(evidence_dir: Path, item_id: str) -> dict[str, Any] | None:
    path = evidence_dir / f"{item_id}.json"
    if not path.is_file():
        return None
    try:
        record = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise BlueprintError(f"{item_id}: handoff {path} unreadable: {error}") from error
    if not isinstance(record, dict):
        raise BlueprintError(f"{item_id}: handoff {path} is not a JSON object")
    return record


def check_handoff(record: dict[str, Any] | None, item_id: str, evidence_dir: Path) -> None:
    where = evidence_dir / f"{item_id}.json"
    if record is None:
        raise BlueprintError(f"{item_id}: missing handoff {where}")
    if record.get("item_id") != item_id:
        raise BlueprintError(f"{item_id}: handoff item_id is {record.get('item_id')!r}")
    if not isinstance(record.get("changed_files"), list):
        raise BlueprintError(f"{item_id}: handoff changed_files must be a list")
    commands = record.get("commands_run")
    if not isinstance(commands, list) or not commands:
        raise BlueprintError(f"{item_id}: handoff commands_run must be a non-empty list")
    if not isinstance(record.get("validation_result"), dict):
        raise BlueprintError(f"{item_id}: handoff validation_result must be an object")
    if record.get("digest") != canonical_digest(record):
        raise BlueprintError(f"{item_id}: handoff digest mismatch in {where}")


def paths_overlap(a: str, b: str) -> bool:
    if a == b:
        return True
    return (a.endswith("/") and b.startswith(a)) or (b.endswith("/") and a.startswith(b))


def find_overlaps(items: list[dict[str, Any]]) -> list[dict[str, Any]]:
    """Open item pairs sharing an owned path with no transitive Depends path."""
    by_id = {item["id"]: item for item in items}
    ancestors: dict[str, set[str]] = {}

    def closure(node: str) -> set[str]:
        if node not in ancestors:
            found: set[str] = set()
            for dep in by_id[node]["depends"]:
                if dep in by_id:
                    found.add(dep)
                    found |= closure(dep)
            ancestors[node] = found
        return ancestors[node]

    open_items = [item for item in items if item["state"] != "x"]
    overlaps = []
    for index, left in enumerate(open_items):
        for right in open_items[index + 1 :]:
            if right["id"] in closure(left["id"]) or left["id"] in closure(right["id"]):
                continue
            shared = sorted(
                {max(a, b, key=len) for a in left["owned"] for b in right["owned"] if paths_overlap(a, b)}
            )
            if shared:
                overlaps.append({"a": left["id"], "b": right["id"], "paths": shared})
    return overlaps


def validate(text: str, evidence_dir: Path | None = None) -> dict[str, Any]:
    """Validate `text`; when `evidence_dir` is given, also verify handoffs."""
    header = parse_header(text)
    for key in ("blueprint_version", "authoritative", "stable_id_pattern", "layers", "per_item_code_loc_cap"):
        if key not in header:
            raise BlueprintError(f"missing header key: {key}")
    if header["authoritative"] != "true":
        raise BlueprintError("authoritative must be true")
    id_pattern = re.compile(header["stable_id_pattern"])
    layers = [layer.strip() for layer in header["layers"].split(",")]
    if any(not re.fullmatch(r"[A-Z][0-9]+", layer) for layer in layers):
        raise BlueprintError("layers must look like A0, A1, B1")
    rank = {layer: int(layer[1:]) for layer in layers}
    cap = int(header["per_item_code_loc_cap"])

    items = parse_items(text)
    if not items:
        raise BlueprintError("no checklist items")
    by_id: dict[str, dict[str, Any]] = {}
    for item in items:
        if not id_pattern.fullmatch(item["id"]):
            raise BlueprintError(f"{item['id']}: does not match stable_id_pattern")
        if item["id"] in by_id:
            raise BlueprintError(f"{item['id']}: duplicate id")
        if item["layer"] not in rank:
            raise BlueprintError(f"{item['id']}: unknown layer {item['layer']}")
        if item["loc"] >= cap:
            raise BlueprintError(f"{item['id']}: Estimated LOC {item['loc']} >= {cap}")
        by_id[item["id"]] = item

    for item in items:
        for dep in item["depends"]:
            if dep not in by_id:
                raise BlueprintError(f"{item['id']}: unknown dependency {dep}")
            if dep == item["id"]:
                raise BlueprintError(f"{item['id']}: depends on itself")
            if rank[by_id[dep]["layer"]] > rank[item["layer"]]:
                raise BlueprintError(f"{item['id']}: depends on higher-layer item {dep}")
            if item["state"] == "x" and by_id[dep]["state"] != "x":
                raise BlueprintError(f"{item['id']}: [x] while dependency {dep} is open")

    visiting: set[str] = set()
    done: set[str] = set()

    def visit(node: str, path: list[str]) -> None:
        if node in done:
            return
        if node in visiting:
            raise BlueprintError("dependency cycle: " + " -> ".join(path + [node]))
        visiting.add(node)
        for dep in by_id[node]["depends"]:
            visit(dep, path + [node])
        visiting.discard(node)
        done.add(node)

    for item_id in by_id:
        visit(item_id, [])

    for block in sorted({layer[0] for layer in layers}):
        block_items = [i for i in items if i["layer"][0] == block]
        open_layers = [i["layer"] for i in block_items if i["state"] != "x"]
        if not open_layers:
            continue
        lowest = min(open_layers, key=lambda layer: rank[layer])
        for item in block_items:
            if item["state"] == "x" and rank[item["layer"]] > rank[lowest]:
                raise BlueprintError(f"{item['id']}: [x] in {item['layer']} while {lowest} still has open items")

    handoffs = 0
    if evidence_dir is not None:
        for item in items:
            if item["state"] == " " or rank[item["layer"]] == 0:
                continue
            check_handoff(load_handoff(evidence_dir, item["id"]), item["id"], evidence_dir)
            handoffs += 1

    counts = {state: sum(1 for i in items if i["state"] == state) for state in (" ", "_", "x")}
    return {
        "version": header["blueprint_version"],
        "items": len(items),
        "open": counts[" "],
        "self_tested": counts["_"],
        "accepted": counts["x"],
        "max_loc": max(i["loc"] for i in items),
        "cap": cap,
        "handoffs_checked": handoffs,
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("path", nargs="?", type=Path, default=DEFAULT_PATH)
    parser.add_argument("--json", action="store_true", help="print the summary as JSON")
    parser.add_argument(
        "--evidence-dir",
        type=Path,
        default=None,
        help="handoff directory (default: quality/ze next to the blueprint, i.e. Docs/quality/ze)",
    )
    parser.add_argument(
        "--report-overlaps",
        action="store_true",
        help="list open items sharing an owned path without a Depends path (report only)",
    )
    args = parser.parse_args(argv)
    evidence_dir = args.evidence_dir or args.path.parent / "quality" / "ze"
    try:
        text = args.path.read_text(encoding="utf-8")
        summary = validate(text, evidence_dir)
        overlaps = find_overlaps(parse_items(text))
    except (BlueprintError, OSError) as error:
        print(f"Blueprint v2 invalid: {error}", file=sys.stderr)
        return 1
    summary["overlaps"] = len(overlaps)
    if args.report_overlaps:
        summary["overlap_pairs"] = overlaps
    if args.json:
        print(json.dumps(summary, sort_keys=True, ensure_ascii=False))
        return 0
    print(
        f"Blueprint v2 valid ({summary['version']}): {summary['items']} items, "
        f"[ ] {summary['open']} / [_] {summary['self_tested']} / [x] {summary['accepted']}, "
        f"max LOC {summary['max_loc']} < {summary['cap']}, "
        f"{summary['handoffs_checked']} handoffs verified, {len(overlaps)} owned-path overlaps"
    )
    if args.report_overlaps:
        for pair in overlaps:
            print(f"  overlap {pair['a']} <-> {pair['b']}: {', '.join(pair['paths'])}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

#!/usr/bin/env python3
"""Validate the authoritative v2 execution Blueprint (b3ehive checklist form).

Checks the claims the execution controller relies on: header metadata, one
checklist line per stable ID, legal checkbox states, required fields, a
closed and acyclic dependency DAG, dependencies never pointing at a higher
layer, the strict layer gate, `[x]` items having only `[x]` dependencies, and
the strict per-item LOC cap.  Standard library only.

Layers are `<block letter><level>` (e.g. `A2`, `B3`).  The layer gate applies
within one block; dependencies may cross blocks but never point at a higher
level.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path
from typing import Any


DEFAULT_PATH = Path("Docs/Zenpi_Execution_Blueprint_v2.md")
ITEM_RE = re.compile(r"^- \[(?P<state>[ _x])\] \*\*(?P<id>[A-Z0-9]+-[0-9]{3})\*\* — (?P<body>.*)$")
LOOSE_ITEM_RE = re.compile(r"^\s*[-*] \[.\] \*\*[A-Z0-9]+-[0-9]{3}\*\*")
LAYER_RE = re.compile(r"layer `([A-Z][0-9]+)`")
DEPENDS_RE = re.compile(r"\| Depends: ([^|]*)")
LOC_RE = re.compile(r"\| Estimated LOC: ([0-9]+)\s*$")
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
        deps_raw = depends.group(1).strip() if depends else ""
        deps = [] if deps_raw in {"", "—", "-"} else [d.strip() for d in deps_raw.split(",")]
        items.append(
            {
                "id": match.group("id"),
                "state": match.group("state"),
                "layer": layer.group(1),
                "depends": deps,
                "loc": int(loc.group(1)),
                "title": body.split("；layer", 1)[0].strip(),
                "line": lineno,
            }
        )
    return items


def validate(text: str) -> dict[str, Any]:
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

    counts = {state: sum(1 for i in items if i["state"] == state) for state in (" ", "_", "x")}
    return {
        "version": header["blueprint_version"],
        "items": len(items),
        "open": counts[" "],
        "self_tested": counts["_"],
        "accepted": counts["x"],
        "max_loc": max(i["loc"] for i in items),
        "cap": cap,
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("path", nargs="?", type=Path, default=DEFAULT_PATH)
    parser.add_argument("--json", action="store_true", help="print the summary as JSON")
    args = parser.parse_args(argv)
    try:
        summary = validate(args.path.read_text(encoding="utf-8"))
    except (BlueprintError, OSError) as error:
        print(f"Blueprint v2 invalid: {error}", file=sys.stderr)
        return 1
    if args.json:
        print(json.dumps(summary, sort_keys=True))
    else:
        print(
            f"Blueprint v2 valid ({summary['version']}): {summary['items']} items, "
            f"[ ] {summary['open']} / [_] {summary['self_tested']} / [x] {summary['accepted']}, "
            f"max LOC {summary['max_loc']} < {summary['cap']}"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

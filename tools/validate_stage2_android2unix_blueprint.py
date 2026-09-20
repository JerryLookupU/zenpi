#!/usr/bin/env python3
"""Portable structural gate for the Stage 2 Android/Unix execution Blueprint.

This checker validates the document and its generated Gantt without launching
workers.  ``--strict`` additionally requires the final receipts/selector
surfaces used by an execution controller; an unfinished draft is valid without
those runtime artifacts.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
BLUEPRINT = Path("Docs/stage2_android2unix_blueprint.md")
SPEC = Path("Docs/stage2_android2unix_spec.md")
GANTT = Path("Docs/stage2_android2unix_gantt.md")
ID_RE = re.compile(r"^S2-[0-9]{3}$")
STATES = {" ", "_", "x"}
HEX = re.compile(r"^[0-9a-f]{64}$")


class BlueprintError(Exception):
    pass


def yaml_scalar(value: str) -> Any:
    value = value.strip()
    if value in {"true", "false"}:
        return value == "true"
    if re.fullmatch(r"-?\d+", value):
        return int(value)
    if value.startswith("[") and value.endswith("]"):
        body = value[1:-1].strip()
        return [] if not body else [yaml_scalar(part) for part in body.split(",")]
    if (value.startswith("'") and value.endswith("'")) or (value.startswith('"') and value.endswith('"')):
        return value[1:-1]
    return value


def header(text: str) -> dict[str, Any]:
    blocks = re.findall(r"```yaml\s*\n(.*?)\n```", text, re.S)
    if not blocks:
        raise BlueprintError("missing fenced yaml header")
    result: dict[str, Any] = {}
    for raw in blocks[0].splitlines():
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        key, sep, value = line.partition(":")
        if not sep or not re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", key):
            raise BlueprintError(f"invalid header line: {raw!r}")
        if key in result:
            raise BlueprintError(f"duplicate header key: {key}")
        result[key] = yaml_scalar(value)
    return result


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def rel_safe(value: str) -> bool:
    path = Path(value)
    return bool(value) and not path.is_absolute() and "\\" not in value and ".." not in path.parts and "\x00" not in value and not any(ch in value for ch in "*?[]")


def parse_items(text: str) -> dict[str, dict[str, Any]]:
    items: dict[str, dict[str, Any]] = {}
    pattern = re.compile(r"^- \[([ _x])\] \*\*(S2-[0-9]{3})\*\* — (.*?)；layer `([^`]+)` \| (.*)$")
    for number, line in enumerate(text.splitlines(), 1):
        if line.startswith("- [") and "**S2-" in line:
            match = pattern.match(line)
            if not match:
                raise BlueprintError(f"line {number}: malformed checklist row")
            mark, item_id, title, layer, fields = match.groups()
            if mark not in STATES or not ID_RE.fullmatch(item_id):
                raise BlueprintError(f"line {number}: invalid state or ID")
            if item_id in items:
                raise BlueprintError(f"duplicate checklist ID {item_id}")
            values: dict[str, str] = {}
            for cell in fields.split(" | "):
                key, sep, value = cell.partition(": ")
                if not sep or key in values:
                    raise BlueprintError(f"line {number}: malformed or duplicate field in {item_id}")
                values[key] = value
            required = {"Depends", "Owner scope", "Owned paths", "Validators", "Rollback", "Estimate", "Estimated LOC"}
            missing = required - values.keys()
            if missing:
                raise BlueprintError(f"{item_id}: missing fields {sorted(missing)}")
            try:
                loc = int(values["Estimated LOC"])
            except ValueError as exc:
                raise BlueprintError(f"{item_id}: Estimated LOC is not an integer") from exc
            depends = () if values["Depends"] in {"—", "-", "none"} else tuple(x.strip() for x in values["Depends"].split(",") if x.strip())
            owned = tuple(x.strip().strip("`") for x in values["Owned paths"].split(",") if x.strip())
            if not owned or any(not rel_safe(path) for path in owned):
                raise BlueprintError(f"{item_id}: unsafe or empty owned path")
            items[item_id] = dict(id=item_id, mark=mark, title=title, layer=layer, depends=depends, owner=values["Owner scope"], owned=owned, loc=loc, line=number, fields=values)
    if not items:
        raise BlueprintError("no checklist rows")
    for item in items.values():
        for dep in item["depends"]:
            if dep not in items:
                raise BlueprintError(f"{item['id']}: dependency {dep} is missing")
    visiting: set[str] = set()
    visited: set[str] = set()
    def visit(item_id: str) -> None:
        if item_id in visiting:
            raise BlueprintError(f"dependency cycle at {item_id}")
        if item_id in visited:
            return
        visiting.add(item_id)
        for dep in items[item_id]["depends"]:
            visit(dep)
        visiting.remove(item_id)
        visited.add(item_id)
    for item_id in items:
        visit(item_id)
    return items


def check_headers(bp: dict[str, Any], sp: dict[str, Any]) -> None:
    required = {
        "schema_version": "execution-blueprint/stage2", "authoritative": True,
        "stable_id_pattern": "^S2-[0-9]{3}$", "status_values": "[ ]|[_]|[x]",
        "per_item_code_loc_cap": 5000, "worker_transport": "tmux_codex_tui",
        "app_server_workers": "forbidden", "nested_agents": "forbidden",
        "worker_lifecycle": "bounded", "execution_spec": str(SPEC), "gantt_projection": str(GANTT),
    }
    for key, value in required.items():
        if bp.get(key) != value:
            raise BlueprintError(f"Blueprint header {key} must be {value!r}, got {bp.get(key)!r}")
    required_spec = {
        "schema_version": "execution-spec/stage2", "authoritative_blueprint": str(BLUEPRINT),
        "gantt_projection": str(GANTT), "gantt_naming": "exact-prefix-Blueprint-to-Gantt",
        "worker_transport": "tmux_codex_tui", "app_server_workers": "forbidden",
        "nested_agents": "forbidden", "worker_lifecycle": "bounded", "per_item_code_loc_cap": 5000,
    }
    for key, value in required_spec.items():
        if sp.get(key) != value:
            raise BlueprintError(f"spec header {key} must be {value!r}, got {sp.get(key)!r}")


def parse_gantt(text: str) -> tuple[dict[str, Any], dict[str, tuple[str, tuple[str, ...]]]]:
    gh = header(text)
    marker = re.search(r"^## Monitoring index\s*$", text, re.M)
    if not marker:
        raise BlueprintError("Gantt has no Monitoring index")
    section = text[marker.end():]
    rows: dict[str, tuple[str, tuple[str, ...]]] = {}
    seen_header = False
    for line in section.splitlines():
        if not line.startswith("|"):
            continue
        cells = [cell.strip() for cell in line.strip().strip("|").split("|")]
        if cells and cells[0].lower() == "id":
            required = {"id", "state", "depends on", "claim/owner", "startup/live", "handoff/integration/repair", "scheduling note"}
            if not required.issubset({cell.lower() for cell in cells}):
                raise BlueprintError("Gantt monitoring header lacks runtime columns")
            seen_header = True
            continue
        if len(cells) < 7 or not ID_RE.fullmatch(cells[0]):
            continue
        item_id = cells[0]
        if item_id in rows:
            raise BlueprintError(f"Gantt duplicate ID {item_id}")
        deps = () if cells[2] in {"—", "-", "none"} else tuple(x for x in re.split(r"[, ]+", cells[2]) if x)
        if any(not cells[i] for i in (1, 3, 4, 5, 6)):
            raise BlueprintError(f"Gantt {item_id} has an empty runtime field")
        rows[item_id] = (cells[1], deps)
    if not seen_header:
        raise BlueprintError("Gantt monitoring header missing")
    if not re.search(r"```mermaid\s*\n.*^\s*gantt\s*$.*?```", text, re.M | re.S | re.I):
        raise BlueprintError("Gantt has no renderable Mermaid gantt")
    # Mutable marks are forbidden in the projection body.  The generated YAML
    # and monitoring state names are intentionally words, not checklist marks.
    if re.search(r"(?<!`)(?:\[[ _xX]\])(?!`)", text):
        raise BlueprintError("Gantt contains mutable checklist mark")
    return gh, rows


def validate(root: Path = ROOT, strict: bool = False) -> dict[str, Any]:
    root = root.resolve()
    bp_path, sp_path, gantt_path = (root / p for p in (BLUEPRINT, SPEC, GANTT))
    for path in (bp_path, sp_path, gantt_path):
        if not path.is_file():
            raise BlueprintError(f"missing required file: {path.relative_to(root)}")
    bp_text, sp_text, gantt_text = (path.read_text(encoding="utf-8") for path in (bp_path, sp_path, gantt_path))
    bp, sp = header(bp_text), header(sp_text)
    check_headers(bp, sp)
    items = parse_items(bp_text)
    for item in items.values():
        if item["loc"] < 0 or item["loc"] >= 5000:
            raise BlueprintError(f"{item['id']}: Estimated LOC must be 0..4999")
    gh, grows = parse_gantt(gantt_text)
    if gh.get("schema_version") != "execution-gantt/stage2" or gh.get("source_path") != str(BLUEPRINT) or gh.get("spec_path") != str(SPEC) or gh.get("projection_authority") is not False:
        raise BlueprintError("invalid Gantt authority header")
    if gh.get("source_sha256") != digest(bp_path) or gh.get("spec_sha256") != digest(sp_path):
        raise BlueprintError("stale Gantt source/spec digest")
    if set(grows) != set(items):
        raise BlueprintError(f"Gantt item mismatch: missing={sorted(set(items)-set(grows))}, extra={sorted(set(grows)-set(items))}")
    state_map = {" ": "unclaimed", "_": "self_tested", "x": "master_accepted"}
    for item_id, item in items.items():
        state, deps = grows[item_id]
        if state != state_map[item["mark"]] or set(deps) != set(item["depends"]):
            raise BlueprintError(f"Gantt state/dependency mismatch for {item_id}")
    selector = root / "Docs/execution/stage2_android2unix_active_requirement.json"
    if strict:
        if not selector.is_file():
            raise BlueprintError("strict validation requires active requirement selector")
        value = json.loads(selector.read_text(encoding="utf-8"))
        if value.get("schema_version") != "stage2-selector/v1" or value.get("active") is not True:
            raise BlueprintError("invalid Stage 2 selector")
        if value.get("blueprint") != str(BLUEPRINT) or value.get("specification") != str(SPEC):
            raise BlueprintError("selector points at wrong Stage 2 files")
        receipts = root / "Docs/quality/stage2_android2unix/receipts"
        unfinished = [item_id for item_id, item in items.items() if item["mark"] != "x"]
        if not unfinished and not receipts.is_dir():
            raise BlueprintError("strict completion requires receipts")
    counts = {"[ ]": sum(item["mark"] == " " for item in items.values()), "[_]": sum(item["mark"] == "_" for item in items.values()), "[x]": sum(item["mark"] == "x" for item in items.values())}
    return {"ok": True, "blueprint": str(BLUEPRINT), "specification": str(SPEC), "gantt": str(GANTT), "items": len(items), "counts": counts, "strict": strict}


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--strict", action="store_true")
    args = parser.parse_args(argv)
    try:
        print(json.dumps(validate(strict=args.strict), ensure_ascii=False, indent=2))
        return 0
    except (BlueprintError, OSError, UnicodeError, json.JSONDecodeError) as exc:
        print(f"stage2 blueprint invalid: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
